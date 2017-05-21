use futures::sync::oneshot;
use futures::future;
use futures::Future;
use hyper;
use hyper::{Response, Request, Uri};
use hyper::client::Client;
use hyper::server::Service;
use hyper_tls::HttpsConnector;
use mktemp::Temp;
use std::io::prelude::*;
use std::marker::PhantomData;
use std::path::Path;
use std::process::Command;
use tokio_core::reactor::Handle;
use tokio_process::CommandExt;
use tera::Tera;

use http::*;
use papers::DocumentSpec;
use error::{Error, ErrorKind};
use config::Config;

pub trait FromHandle {
    fn build(handle: &Handle) -> Self;
}

impl FromHandle for Client<HttpsConnector> {
    fn build(handle: &Handle) -> Self {
        Client::configure()
            .connector(https_connector(handle))
            .build(handle)
    }
}

fn extract_filename_from_uri(uri: &Uri) -> Option<String> {
    match uri.path().split('/').last() {
        Some(name) => {
            if !name.is_empty() {
                Some(name.to_string())
            } else {
                None
            }
        }
        None => None,
    }
}

pub trait Renderer {
    fn new(config: &'static Config, handle: &Handle) -> Self;
    fn preview(&self,
               d: DocumentSpec,
               sender: oneshot::Sender<Result<String, Error>>)
               -> Box<Future<Item = (), Error = ()>>;
    fn render(&self, d: DocumentSpec) -> Box<Future<Item = (), Error = ()>>;
}

#[derive(Clone, Debug)]
pub struct ConcreteRenderer<S>
    where S: Service<Request=Request, Response=Response, Error=hyper::Error> + FromHandle + 'static
{
    config: &'static Config,
    handle: Handle,
    _client: PhantomData<S>,
}

impl<S> ConcreteRenderer<S>
    where S: Service<Request=Request, Response=Response, Error=hyper::Error> + FromHandle + 'static
{
    fn get_template(&self, template_url: &Uri) -> Box<Future<
        Item=hyper::client::Response,
        Error=Error>
    > {
        S::build(&self.handle).get_follow_redirect(template_url)
    }
}

impl<S> Renderer for ConcreteRenderer<S>
    where S: Service<Request = Request, Response = Response, Error = hyper::Error> + FromHandle
{
    fn new(config: &'static Config, handle: &Handle) -> Self {
        ConcreteRenderer {
            config,
            handle: handle.clone(),
            _client: PhantomData,
        }
    }

    fn preview(&self,
               document_spec: DocumentSpec,
               sender: oneshot::Sender<Result<String, Error>>)
               -> Box<Future<Item = (), Error = ()>> {
        let DocumentSpec {
            variables,
            template_url,
            ..
        } = document_spec;
        let response = self.get_template(&template_url.0);
        let max_asset_size = self.config.max_asset_size;
        let bytes = response.and_then(move |res| res.get_body_bytes_limit(max_asset_size));
        let template_string =
            bytes.and_then(|bytes| ::std::string::String::from_utf8(bytes).map_err(Error::from));
        let rendered =
            template_string.and_then(move |template_string| {
                                         Tera::one_off(&template_string, &variables, false)
                                             .map_err(Error::from)
                                     });
        let work = rendered
            .then(|rendered| sender.send(rendered))
            .map(|_| ())
            .map_err(|_| ());
        Box::new(work)
    }

    // Since `mktemp::Temp` implements Drop by deleting the directory, we don't need to worry about
    // leaving files or directories behind. On the flipside, we must ensure it is not dropped before
    // the last returned future that needs the directory finishes.
    fn render(&self, document_spec: DocumentSpec) -> Box<Future<Item = (), Error = ()>> {
        let dir = Temp::new_dir();

        if let Err(err) = dir {
            error!(self.config.logger, err);
            return Box::new(future::err(()));
        }

        let dir = dir.unwrap();

        let temp_dir_path = dir.to_path_buf();
        let mut template_path = temp_dir_path.clone();
        template_path.push(Path::new(&document_spec.output_filename.replace("pdf", "tex")));
        let max_asset_size = self.config.max_asset_size;
        let handle = self.handle.clone();
        let logger = self.config.logger.clone();

        debug!(logger,
               "Trying to generate PDF with document spec: {:?}",
               document_spec);

        let DocumentSpec {
            assets_urls,
            callback_url,
            output_filename,
            template_url,
            variables,
            ..
        } = document_spec;

        let res = self.get_template(&template_url.0);

        debug!(logger, "Starting Renderer worker");

        // First download the template and populate it
        let bytes = res.and_then(move |res| res.get_body_bytes_limit(max_asset_size));

        let template_string = {
            let logger = logger.clone();
            bytes.and_then(move |bytes| {
                               debug!(logger, "Successfully downloaded the template");
                               String::from_utf8(bytes).map_err(Error::from)
                           })
        };

        let rendered_template =
            template_string.and_then(move |template_string| {
                                         Tera::one_off(&template_string, &variables, false)
                                             .map_err(Error::from)
                                     });

        let written_template_path = {
            let logger = logger.clone();
            let template_path = template_path.clone();
            rendered_template.and_then(move |latex_string| {
                debug!(logger, "Writing template to {:?}", &template_path);
                let mut file = ::std::fs::File::create(&template_path).unwrap();
                file.write_all(latex_string.as_bytes())
                    .expect("could not write latex file");
                debug!(logger,
                       "Template successfully written to {:?}",
                       &template_path);
                Ok(template_path)
            })
        };

        // Download the assets and save them in the temporary directory
        let files_written = {
            let logger = logger.clone();
            let temp_dir_path = temp_dir_path.clone();
            let handle = handle.clone();
            let max_asset_size = max_asset_size;
            written_template_path.and_then(move |_| {
                debug!(logger, "Downloading assets {:?}", assets_urls);
                let futures = assets_urls
                    .into_iter()
                    .map(move |uri| {
                        let logger = logger.clone();
                        let mut path = temp_dir_path.clone();

                        let response = Client::configure()
                            .connector(https_connector(&handle))
                            .build(&handle)
                            .get_follow_redirect(&uri.0);

                        let body = response.and_then(move |res| {
                                                         let filename = res.filename();
                                                         res.get_body_bytes_limit(max_asset_size)
                                                             .map(|bytes| (bytes, filename))
                                                     });
                        body.and_then(move |(bytes, filename)| {
                            let filename =
                                filename.or_else(|| extract_filename_from_uri(&uri.0));
                            match filename {
                                Some(filename) => {
                                    path.push(filename);
                                    debug!(logger, "Writing asset {:?} as {:?}", uri, path);
                                    ::std::fs::File::create(&path)
                                        .and_then(|mut file| file.write_all(&bytes))
                                        .map_err(Error::from)
                                }
                                _ => Ok(()),
                            }
                        })
                    });
                future::join_all(futures)
            })
        };

        // Then run latex
        let latex_out = {
            let handle = handle.clone();
            let template_path = template_path.clone();
            let temp_dir_path = temp_dir_path.clone();
            files_written.and_then(move |_| {
                Command::new("xelatex")
                    .current_dir(&temp_dir_path)
                    .arg("-interaction=nonstopmode")
                    .arg("-file-line-error")
                    .arg("-shell-restricted")
                    .arg(template_path)
                    .output_async(&handle)
                    .map_err(Error::from)
            })
        };

        let output_path = {
            let logger = logger.clone();
            let temp_dir_path = temp_dir_path.clone();
            latex_out
                .and_then(move |output| {
                    let stdout = String::from_utf8(output.stdout).unwrap();
                    if output.status.success() {
                        debug!(logger, "{}", stdout);
                        Ok(())
                    } else {
                        Err(ErrorKind::LatexFailed(stdout).into())
                    }
                })
                .map(move |_| {
                         // Construct the path to the generated PDF
                         let mut path = temp_dir_path;
                         path.push(Path::new(&output_filename));
                         path
                     })
        };

        // Then post a multipart request from the generated PDF
        let callback_response = {
            let logger = logger.clone();
            let handle = handle.clone();
            let callback_url = callback_url.clone();
            output_path
                .and_then(move |pdf_path| {
                              debug!(logger, "Reading the pdf from {:?}", pdf_path);
                              debug!(logger, "Sending generated PDF to {}", callback_url.0);
                              multipart_request_with_file(Request::new(hyper::Method::Post,
                                                                       callback_url.0),
                                                          pdf_path)
                          })
                .and_then(move |req| {
                    // Avoid dir being dropped early
                    let _dir = dir;

                    Client::configure()
                        .connector(https_connector(&handle))
                        .build(&handle)
                        .request(req)
                        .map_err(Error::from)
                })
        };

        let response_bytes = {
            let logger = logger.clone();
            let max_asset_size = max_asset_size;
            callback_response.and_then(move |response| {
                info!(logger,
                      "Callback response: {}",
                      response.status().canonical_reason().unwrap_or("unknown"));

                response.get_body_bytes_limit(max_asset_size)
            })
        };

        let res = {
            let logger = logger.clone();
            response_bytes.and_then(move |bytes| {
                debug!(logger,
                       "Callback response body: {:?}",
                       ::std::str::from_utf8(&bytes).unwrap_or("<binary content>"));
                future::ok(())
            })
        };

        // Report errors to the callback url
        let handle_errors = {
            let logger = logger.clone();
            res.or_else(move |error| {
                            error!(logger, format!("{}", error));
                            let req = Request::new(hyper::Method::Post, callback_url.0);
                            Client::new(&handle)
                                .request(multipart_request_with_error(req, &error).unwrap())
                                .map(|_| ())
                        })
        };

        Box::new(handle_errors.map_err(|_| ()))
    }
}

/// A renderer that should never be called. This is meant for testing.
pub struct NilRenderer;

impl Renderer for NilRenderer {
    fn new(_: &'static Config, _: &Handle) -> Self {
        unimplemented!();
    }

    fn preview(&self,
               _: DocumentSpec,
               _: oneshot::Sender<Result<String, Error>>)
               -> Box<Future<Item = (), Error = ()>> {
        unimplemented!();
    }

    fn render(&self, _: DocumentSpec) -> Box<Future<Item = (), Error = ()>> {
        unimplemented!();
    }
}

/// A renderer that does nothing. Meant for testing.
pub struct NoopRenderer;

impl Renderer for NoopRenderer {
    fn new(_: &'static Config, _: &Handle) -> Self {
        NoopRenderer
    }

    fn preview(&self,
               _: DocumentSpec,
               _: oneshot::Sender<Result<String, Error>>)
               -> Box<Future<Item = (), Error = ()>> {
        Box::new(future::ok(()))
    }

    fn render(&self, _: DocumentSpec) -> Box<Future<Item = (), Error = ()>> {
        Box::new(future::ok(()))
    }
}


#[cfg(test)]
mod tests {
    use hyper::Uri;
    use super::extract_filename_from_uri;

    #[test]
    fn test_extract_filename_from_uri_works() {
        let assert_extracted = |input: &'static str, expected_output: Option<&'static str>| {
            let uri = input.parse::<Uri>().unwrap();
            assert_eq!(extract_filename_from_uri(&uri),
                       expected_output.map(|o| o.to_string()));
        };

        assert_extracted("/logo.png", Some("logo.png"));
        assert_extracted("/assets/", None);
        assert_extracted("/assets/icon", Some("icon"));
        assert_extracted("/", None);
        assert_extracted("http://www.store2be.com", None);
    }
}

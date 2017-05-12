use futures::future;
use futures::Future;
use hyper;
use hyper::Uri;
use hyper::client::{Client, Request};
use mktemp::Temp;
use slog;
use std::io::prelude::*;
use std::path::Path;
use std::process::Command;
use tokio_core::reactor::{Handle, Remote};
use tokio_process::CommandExt;
use tera::Tera;

use http::*;
use papers::DocumentSpec;
use error::{Error, ErrorKind};

pub struct Renderer {
    dir: Temp,
    document_spec: DocumentSpec,
    handle: Handle,
    template_path: ::std::path::PathBuf,
    logger: slog::Logger,
}

fn extract_filename_from_uri(uri: &Uri) -> Option<String> {
    match uri.path().split('/').last() {
        Some(name) => {
            if !name.is_empty() {
                Some(name.to_string())
            } else {
                None
            }
        },
        None => None,
    }
}

// Since `mktemp::Temp` implements Drop by deleting the directory, we don't need to worry about
// leaving files or directories behind. On the flipside, we must ensure it is not dropped before
// the last returned future that needs the directory finishes.
impl Renderer {

    pub fn new(remote: Remote, document_spec: DocumentSpec, logger: slog::Logger) -> Result<Renderer, Error> {
        let dir = Temp::new_dir()?;
        let mut template_path = dir.to_path_buf();
        template_path.push(Path::new(&document_spec.output_filename.replace("pdf", "tex")));
        Ok(Renderer {
            document_spec,
            handle: remote.handle().unwrap(),
            template_path,
            dir,
            logger,
        })
    }

    fn get_template(&self) -> Box<Future<Item=hyper::client::Response, Error=Error>> {
        Client::configure()
            .connector(https_connector(&self.handle))
            .build(&self.handle)
            .get_follow_redirect(&self.document_spec.template_url.0)
    }

    pub fn preview(self) -> Box<Future<Item=String, Error=Error>> {
        let response = self.get_template();
        let Renderer { document_spec, ..  } = self;
        let DocumentSpec { variables, ..  } = document_spec;
        let bytes = response.and_then(|res| res.get_body_bytes());
        let template_string = bytes.and_then(|bytes| ::std::string::String::from_utf8(bytes).map_err(Error::from));
        let rendered = template_string.and_then(move |template_string| {
            Tera::one_off(&template_string, &variables, false).map_err(Error::from)
        });
        Box::new(rendered)
    }

    pub fn execute(self) -> Box<Future<Item=(), Error=()>> {
        let res = self.get_template();

        let Renderer {
            handle,
            document_spec,
            template_path,
            dir,
            logger,
        } = self;

        debug!(logger, "Trying to generate PDF with document spec: {:?}", document_spec);

        let DocumentSpec {
            assets_urls,
            callback_url,
            output_filename,
            variables,
            ..
        } = document_spec;

        debug!(logger, "Starting Renderer worker");

        let temp_dir_path = dir.to_path_buf();

        // First download the template and populate it
        let bytes = res.and_then(|res| res.get_body_bytes());

        let template_string = {
            let logger = logger.clone();
            bytes.and_then(move |bytes| {
                debug!(logger, "Successfully downloaded the template");
                String::from_utf8(bytes)
                    .map_err(Error::from)
            })
        };

        let rendered_template = template_string.and_then(move |template_string| {
            Tera::one_off(&template_string, &variables, false)
                .map_err(Error::from)
        });

        let written_template_path = {
            let logger = logger.clone();
            let template_path = template_path.clone();
            rendered_template.and_then(move |latex_string| {
                debug!(logger, "Writing template to {:?}", &template_path);
                let mut file = ::std::fs::File::create(&template_path).unwrap();
                file.write_all(latex_string.as_bytes()).expect("could not write latex file");
                debug!(logger, "Template successfully written to {:?}", &template_path);
                Ok(template_path)
            })
        };

        // Download the assets and save them in the temporary directory
        let files_written = {
            let logger = logger.clone();
            let temp_dir_path = temp_dir_path.clone();
            let handle = handle.clone();
            written_template_path.and_then(move |_| {
                debug!(logger, "Downloading assets {:?}", assets_urls);
                let futures = assets_urls.into_iter().map(move |uri| {
                    let logger = logger.clone();
                    let mut path = temp_dir_path.clone();

                    let response = Client::configure()
                        .connector(https_connector(&handle))
                        .build(&handle)
                        .get_follow_redirect(&uri.0);

                    let body = response.and_then(|res| {
                            let filename = res.filename();
                            res.get_body_bytes().map(|bytes| (bytes, filename))
                    });

                    body.and_then(move |(bytes, filename)| {
                        let filename = filename.or_else(|| extract_filename_from_uri(&uri.0));
                        match filename {
                            Some(filename) => {
                                path.push(filename);
                                debug!(logger, "Writing asset {:?} as {:?}", uri, path);
                                ::std::fs::File::create(&path)
                                    .and_then(|mut file| file.write_all(&bytes))
                                    .map_err(Error::from)
                            },
                            _ => Ok(())
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
            latex_out.and_then(move |output| {
                let stdout = String::from_utf8(output.stdout).unwrap();
                if output.status.success() {
                    debug!(logger, "{}", stdout);
                    Ok(())
                } else {
                    Err(ErrorKind::LatexFailed(stdout).into())
                }
            }).map(move |_| {
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
            output_path.and_then(move |pdf_path| {
                debug!(logger, "Reading the pdf from {:?}", pdf_path);
                debug!(logger, "Sending generated PDF to {}", callback_url.0);
                multipart_request_with_file(Request::new(hyper::Method::Post, callback_url.0), pdf_path)
            }).and_then(move |req| {
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
            callback_response.and_then(move |response| {
                info!(
                    logger,
                    "Callback response: {}",
                    response.status().canonical_reason().unwrap_or("unknown"));

                response.get_body_bytes()
            })
        };

        let res = {
            let logger = logger.clone();
            response_bytes.and_then(move |bytes| {
                debug!(
                    logger,
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

#[cfg(test)]
mod tests {
    use hyper::Uri;
    use super::extract_filename_from_uri;

    #[test]
    fn test_extract_filename_from_uri_works() {
        let assert_extracted = |input: &'static str, expected_output: Option<&'static str>| {
            let uri = input.parse::<Uri>().unwrap();
            assert_eq!(extract_filename_from_uri(&uri), expected_output.map(|o| o.to_string()));
        };

        assert_extracted("/logo.png", Some("logo.png"));
        assert_extracted("/assets/", None);
        assert_extracted("/assets/icon", Some("icon"));
        assert_extracted("/", None);
        assert_extracted("http://www.store2be.com", None);
    }
}

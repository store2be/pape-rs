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

pub struct Workspace {
    dir: Temp,
    document_spec: DocumentSpec,
    handle: Handle,
    template_path: ::std::path::PathBuf,
    logger: slog::Logger,
}

struct WorkspaceContext {
    handle: Handle,
    logger: slog::Logger,
    temp_dir_path: ::std::path::PathBuf,
}

/// We ignore file names that end with a slash for now, and always determine the file name from the Uri
/// TODO: investigate Content-Disposition: attachment
fn extract_file_name_from_uri(uri: &Uri) -> Option<String> {
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

/// Since `mktemp::Temp` implements Drop by deleting the directory, we don't need to worry about
/// leaving files or directories behind. On the flipside, we must ensure it is not dropped before
/// the last returned future that needs the directory finishes.
impl Workspace {

    pub fn new(remote: Remote, document_spec: DocumentSpec, logger: slog::Logger) -> Result<Workspace, Error> {
        let dir = Temp::new_dir()?;
        let mut template_path = dir.to_path_buf();
        template_path.push(Path::new("out.tex"));
        Ok(Workspace {
            document_spec: document_spec,
            handle: remote.handle().unwrap(),
            template_path: template_path,
            dir: dir,
            logger: logger,
        })
    }

    pub fn preview(self) -> Box<Future<Item=String, Error=Error>> {
        let Workspace {
            handle,
            document_spec,
            ..
        } = self;

        let DocumentSpec {
            template_url,
            variables,
            ..
        } = document_spec;

        // Download the template and populate it
        let work = Client::configure()
            .connector(https_connector(&handle.clone()))
            .build(&handle.clone())
            .get_follow_redirect(&template_url.0)
            .and_then(|res| res.get_body_bytes())
            .and_then(|bytes| {
                ::std::string::String::from_utf8(bytes).map_err(Error::from)
            }).and_then(move |template_string| {
                Tera::one_off(&template_string, &variables, false).map_err(Error::from)
            });

        Box::new(work)
    }

    pub fn execute(self) -> Box<Future<Item=(), Error=()>> {
        let Workspace {
            handle,
            document_spec,
            template_path,
            dir,
            logger,
        } = self;

        let DocumentSpec {
            assets_urls,
            callback_url,
            template_url,
            variables,
        } = document_spec;

        debug!(logger, "Starting Workspace worker");

        let context = WorkspaceContext {
            handle: handle,
            logger: logger,
            temp_dir_path: dir.to_path_buf(),
        };

        let error_logger = context.logger.clone();
        let error_path_handle = context.handle.clone();
        let error_path_callback_url = callback_url.0.clone();

        // First download the template and populate it
        let work = Client::configure()
            .connector(https_connector(&context.handle.clone()))
            .build(&context.handle.clone())
            .get_follow_redirect(&template_url.0)
            .and_then(|res| res.get_body_bytes())
            .and_then(|bytes| {
                debug!(context.logger, "Successfully downloaded the template");
                ::std::string::String::from_utf8(bytes)
                    .map_err(Error::from)
                    .map(|template_string| (context, template_string))
            }).and_then(move |(context, template_string)| {
                Tera::one_off(&template_string, &variables, false)
                    .map_err(Error::from)
                    .map(|latex_string| (context, latex_string))
            }).and_then(|(context, latex_string)| {
                debug!(context.logger, "Writing template to {:?}", template_path.clone());
                let mut file = ::std::fs::File::create(template_path.clone()).unwrap();
                file.write_all(latex_string.as_bytes()).expect("could not write latex file");
                debug!(context.logger, "Template successfully written to {:?}", template_path.clone());
                Ok((context, template_path))
            })

        // Then download the assets and save them in the temporary directory
                .and_then(move |(context, template_path)| {
                    let inner_handle = context.handle.clone();
                    let inner_temp_dir_path = context.temp_dir_path.clone();
                    debug!(context.logger, "Downloading assets {:?}", assets_urls);
                    let futures = assets_urls.into_iter().map(move |uri| {
                        let mut path = inner_temp_dir_path.clone();
                        Client::configure()
                            .connector((https_connector(&inner_handle.clone())))
                            .build(&inner_handle.clone())
                            .get_follow_redirect(&uri.0)
                            .map(move |res| (res, uri.0))
                            .and_then(move |(res, uri)| {
                                let file_name = res.file_name().take();
                                res.get_body_bytes().map(|bytes| (bytes, file_name, uri))
                            }).and_then(move |(bytes, file_name, uri)| {
                                let file_name = file_name.or(extract_file_name_from_uri(&uri));
                                if let Some(file_name) = file_name {
                                    path.push(file_name);
                                    ::std::fs::File::create(&path)
                                        .and_then(|mut file| file.write_all(&bytes))
                                        .map(|_| ())
                                        .map_err(Error::from)
                                } else {
                                    Ok(())
                                }
                            })
                    });
                    future::join_all(futures).map(|result| (context, template_path, result))
                })

        // Then run latex
                .and_then(move |(context, template_path, _)| {
                    let inner_handle = context.handle.clone();
                    Command::new("xelatex")
                        .arg(&format!("-output-directory={}", &context.temp_dir_path.to_str().unwrap()))
                        .arg(template_path.clone())
                        .status_async(&inner_handle)
                        .map(|exit_status| (context, exit_status))
                        .map_err(Error::from)
                }).and_then(|(context, exit_status)| {
                    if exit_status.success() {
                        Ok(context)
                    } else {
                        Err(ErrorKind::LatexFailed.into())
                    }
                })

        // Then construct the path to the generated PDF
                .map(move |context| {
                    let mut path = context.temp_dir_path.clone();
                    path.push(Path::new("out.pdf"));
                    (context, path)
                })

        // Then get a multipart request from the generated PDF
                .and_then(move |(context, pdf_path)| {
                    debug!(context.logger, "Reading the pdf from {:?}", pdf_path);
                    multipart_request_with_file(
                        Request::new(hyper::Method::Post, callback_url.0),
                        pdf_path
                    ).map(|r| (context, r))
                })
        // Finally, post the PDF to the callback URL
                .and_then(move |(context, request)| {
                    // Avoid dir being dropped early
                    let _dir = dir;

                    Client::configure()
                        .connector(https_connector(&context.handle.clone()))
                        .build(&context.handle.clone())
                        .request(request)
                        .map(|_| ())
                        .map_err(Error::from)
        // Report errors to the callback url
                }).or_else(move |error| {
                    error!(error_logger, format!("{}", error));
                    let req = Request::new(hyper::Method::Post, error_path_callback_url);
                    Client::new(&error_path_handle)
                        .request(multipart_request_with_error(req, error).unwrap())
                        .map(|_| ())
                });

        Box::new(work.map_err(|_| ()))
    }
}

#[cfg(test)]
mod tests {
    use hyper::Uri;
    use super::extract_file_name_from_uri;

    #[test]
    fn test_extract_file_name_from_uri_works() {
        let assert_extracted = |input: &'static str, expected_output: Option<&'static str>| {
            let uri = input.parse::<Uri>().unwrap();
            assert_eq!(extract_file_name_from_uri(&uri), expected_output.map(|o| o.to_string()));
        };

        assert_extracted("/logo.png", Some("logo.png"));
        assert_extracted("/assets/", None);
        assert_extracted("/assets/icon", Some("icon"));
        assert_extracted("/", None);
        assert_extracted("http://www.store2be.com", None);
    }
}

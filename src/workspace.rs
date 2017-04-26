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
        let work = Client::new(&handle.clone())
            .get_follow_redirect(template_url.0)
            .and_then(|res| res.get_body_bytes())
            .and_then(|bytes| {
                future::result(::std::string::String::from_utf8(bytes)).map_err(Error::from)
            }).and_then(move |template_string| {
                Tera::one_off(&template_string, &variables, false).map_err(Error::from)
            });

        Box::new(work)
    }

    pub fn execute(self) -> Box<Future<Item=(), Error=Error>> {
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

        let out_tex_path = dir.to_path_buf();
        let out_pdf_path = dir.to_path_buf();

        let context = WorkspaceContext {
            handle: handle,
            logger: logger,
        };

        // First download the template and populate it
        let work = Client::new(&context.handle.clone())
            .get_follow_redirect(template_url.0)
            .and_then(|res| res.get_body_bytes())
            .and_then(|bytes| {
                debug!(context.logger, "Successfully downloaded the template");
                future::result(::std::string::String::from_utf8(bytes))
                    .map_err(Error::from)
                    .map(|template_string| (context, template_string))
            }).and_then(move |(context, template_string)| {
                Tera::one_off(&template_string, &variables, false)
                    .map_err(Error::from)
                    .map(|latex_string| (context, latex_string))
            }).and_then(|(context, latex_string)| {
                debug!(context.logger, "Writing template to {:?}", template_path.clone());
                let mut file = ::std::fs::File::create(template_path.clone()).unwrap();
                file.write_all(latex_string.as_bytes()).unwrap();
                debug!(context.logger, "Template successfully written to {:?}", template_path.clone());
                future::ok((context, template_path))
            })

        // Then download the assets and save them in the temporary directory
                .and_then(move |(context, template_path)| {
                    let named = assets_urls.into_iter().filter_map(|url| {
                        let inner = url.0;
                        extract_file_name_from_uri(&inner).map(|file_name| (file_name, inner))
                    });

                    let inner_handle = context.handle.clone();

                    let download_named = move |(name, url)| {
                        let mut path = out_tex_path.clone();
                        path.push(name);

                        Client::new(&inner_handle)
                            .get_follow_redirect(url)
                            .and_then(|res| res.get_body_bytes())
                            .map(move |bytes| ::std::fs::File::create(&path).unwrap().write_all(&bytes))
                            .map_err(Error::from)
                    };
                    debug!(context.logger, "Downloading assets {:?}", named);
                    future::join_all(named.map(download_named))
                        .map(|result| (context, template_path, result))
                })

        // Then run latex
                .and_then(move |(context, template_path, _)| {
                    let inner_handle = context.handle.clone();
                    let temp_dir_path = template_path.parent().unwrap().to_str().unwrap();
                    Command::new("xelatex")
                        .arg(&format!("-output-directory={}", temp_dir_path))
                        .arg(template_path.clone())
                        .status_async(&inner_handle)
                        .map(|exit_status| (context, exit_status))
                        .map_err(Error::from)
                }).and_then(|(context, exit_status)| {
                    if exit_status.success() {
                        future::ok(context)
                    } else {
                        future::err(ErrorKind::LatexFailed.into())
                    }
                })

        // Then construct the path to the generated PDF
                .and_then(move |context| {
                    let mut path = out_pdf_path;
                    path.push(Path::new("out.pdf"));
                    future::ok((context, path))
                })

        // Then get a multipart request from the generated PDF
                .and_then(move |(context, pdf_path)| {
                    debug!(context.logger, "Reading the pdf from {:?}", pdf_path);
                    future::result(
                        multipart_request_with_file(
                            Request::new(hyper::Method::Post, callback_url.0),
                            pdf_path
                        ).map(|r| (context, r))
                    )
                })
        // Finally, post the PDF to the callback URL
                .and_then(move |(context, request)| {
                    // Avoid dir being dropped early
                    let _dir = dir;

                    Client::new(&context.handle).request(request)
                        .map(|_| ())
                        .map_err(Error::from)
                });

        Box::new(work)
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

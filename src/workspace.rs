use futures::future;
use futures::Future;
use hyper::Uri;
use hyper::client::{Client, Request};
use hyper::Method::Post;
use mktemp::Temp;
use std::io::prelude::*;
use std::path::Path;
use std::process::Command;
use tokio_core::reactor::{Handle, Remote};
use tokio_process::CommandExt;
use tera::Tera;

use http_client;
use papers::DocumentSpec;
use error::Error;

pub struct Workspace {
    dir: Temp,
    document_spec: DocumentSpec,
    handle: Handle,
    template_path: ::std::path::PathBuf,
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

/// Since mktemp::Temp implements Drop by deleting the directory, we don't need to worry about
/// leaving files or directories behind.
impl Workspace {

    pub fn new(remote: Remote, document_spec: DocumentSpec) -> Result<Workspace, Error> {
        let dir = Temp::new_dir()?;
        let mut template_path = dir.to_path_buf();
        template_path.push(Path::new("out.tex"));
        Ok(Workspace {
            document_spec,
            handle: remote.handle().unwrap(),
            template_path,
            dir,
        })
    }

    pub fn preview(self) -> Box<Future<Item=String, Error=Error>> {
        let Workspace {
            handle,
            document_spec,
            template_path: _,
            dir: _,
        } = self;

        let DocumentSpec {
            assets_urls: _,
            callback_url: _,
            template_url,
            variables,
        } = document_spec;

        // Download the template and populate it
        let work = http_client::download_file(&handle.clone(), template_url.0)
            .and_then(|bytes| {
                future::result(::std::string::String::from_utf8(bytes))
                    .map_err(Error::from)
            }).and_then(move |template_string| {
                Tera::one_off(&template_string, &variables, false)
                    .map_err(Error::from)
            });

        Box::new(work)
    }

    pub fn execute(self) -> Box<Future<Item=Vec<u8>, Error=Error>> {
        let Workspace {
            handle,
            document_spec,
            template_path,
            dir,
        } = self;

        let DocumentSpec {
            assets_urls,
            callback_url,
            template_url,
            variables,
        } = document_spec;

        let out_tex_path = dir.to_path_buf();
        let out_pdf_path = dir.to_path_buf();

        // First download the template and populate it
        let work = http_client::download_file(&handle.clone(), template_url.0)
                .and_then(|bytes| {
                    future::result(::std::string::String::from_utf8(bytes))
                        .map_err(Error::from)
                        .map(|template_string| (handle, template_string))
                }).and_then(move |(handle, template_string)| {
                    Tera::one_off(&template_string, &variables, false)
                        .map_err(Error::from)
                        .map(|latex_string| (handle, latex_string))
                }).and_then(|(handle, latex_string)| {
                    let mut file = ::std::fs::File::open(template_path.clone()).unwrap();
                    file.write_all(latex_string.as_bytes()).unwrap();
                    future::ok((handle, template_path))
                })

        // Then download the assets and save them in the temporary directory
                .and_then(move |(handle, template_path)| {
                    let named = assets_urls.into_iter().filter_map(|url| {
                        let inner = url.0;
                        extract_file_name_from_uri(&inner).map(|file_name| (file_name, inner))
                    });

                    let inner_handle = handle.clone();

                    let download_named = move |(name, url)| {
                        let mut path = out_tex_path.clone();
                        path.push(name);

                        http_client::download_file(&inner_handle, url)
                            .map(move |bytes| ::std::fs::File::open(&path).unwrap().write_all(&bytes))
                            .map_err(Error::from)
                    };
                    future::join_all(named.map(download_named))
                        .map(|result| (handle, template_path, result))
                })

        // Then run latex
                .and_then(move |(handle, template_path, _)| {
                    let inner_handle = handle.clone();
                    Command::new("pdflatex")
                        .arg(template_path.clone().to_str().unwrap())
                        .status_async(&inner_handle)
                        .map(|exit_status| (exit_status, handle))
                        .map_err(Error::from)
                }).and_then(|(exit_status, handle)| {
                    if exit_status.success() {
                        future::ok(handle)
                    } else {
                        future::err(Error::LatexFailed)
                    }
                })

        // Then construct the path to the generated PDF
                .and_then(move |handle| {
                    let mut path = out_pdf_path;
                    path.push(Path::new("out.pdf"));
                    future::ok((handle, path))
                })

        // Finally, post it to the callback URL
                .and_then(move |(handle, pdf_path)| {
                    let pdf_file = ::std::fs::File::open(pdf_path).unwrap();
                    let pdf_bytes: Vec<u8> = pdf_file.bytes().collect::<Result<Vec<u8>, _>>().unwrap();

                    let mut request = Request::new(Post, callback_url.0);
                    request.set_body(pdf_bytes);

                    let client = Client::new(&handle);
                    client.request(request);
                    future::ok(())
                }).map(|_| Vec::new());

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

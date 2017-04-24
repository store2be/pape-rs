use futures::future;
use futures::Stream;
use futures::future::{LoopFn, BoxFuture, Future, empty};
use hyper;
use hyper::{Uri, StatusCode};
use hyper::client::{Client, Response};
use mktemp::Temp;
use std::default::Default;
use std::io;
use std::io::prelude::*;
use std::collections::HashMap;
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

/// We ignore file names that end with a slash for now, and always determine the file name from the
/// Uri
/// TODO: investigate Content-Disposition: attachment
fn extract_file_name_from_uri(uri: &Uri) -> Option<String> {
    uri.path().split('/').last().map(|name| name.to_string())
}

/// Since mktemp::Temp implements Drop by deleting the directory, we don't need to worry about
/// leaving files or directories behind.
impl Workspace {

    pub fn new(remote: Remote, document_spec: DocumentSpec) -> Result<Workspace, io::Error> {
        let dir = Temp::new_dir()?;
        let mut template_path = dir.to_path_buf();
        template_path.push(Path::new("template.tex"));
        Ok(Workspace {
            document_spec,
            handle: remote.handle().unwrap(),
            template_path,
            dir,
        })
    }

    pub fn execute(self) {
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

        let final_handle = handle.clone();

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
                        let mut path = dir.to_path_buf();
                        path.push(name);

                        http_client::download_file(&inner_handle, url)
                            .map(move |bytes| ::std::fs::File::open(&path).unwrap().write_all(&bytes))
                            .map_err(Error::from)
                    };
                    future::join_all(named.map(download_named))
                        .map(|result| (handle, template_path, result))
                })

        // Then run latex
                .and_then(|(handle, template_path, _)| {
                    Command::new("pdflatex")
                        .arg(template_path.clone().to_str().unwrap())
                        .status_async(&handle.clone())
                        .map_err(Error::from)
                        .and_then(|exit_status| {
                            if exit_status.success() {
                                Ok(())
                            } else {
                                Err(Error::LatexFailed)
                            }
                        })
                })
                .map_err(|_| ());

        final_handle.spawn(work);
    }
}

use futures::future;
use futures::future::{BoxFuture, Future, empty};
use hyper;
use hyper::client::{Response};
use mktemp::Temp;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::collections::HashMap;
use tokio_core::reactor::Remote;
use papers::DocumentSpec;

pub struct Workspace {
    dir: Temp,
    document_spec: DocumentSpec,
    remote: Remote,
}

/// Since mktemp::Temp implements Drop by deleting the directory, we don't need to worry about
/// leaving files or directories behind.
impl Workspace {
    pub fn new(remote: Remote, document_spec: DocumentSpec) -> Result<Workspace, io::Error> {
        Ok(Workspace {
            dir: Temp::new_dir()?,
            document_spec,
            remote,
        })
    }

    pub fn execute(self) {
        self.remote.spawn(|_| empty())
    }

    fn download_files() -> BoxFuture<(Workspace, Vec<String>), io::Error> {
        unimplemented!()
    }

    fn generate_latex(files: Vec<String>) -> BoxFuture<(), io::Error> {
        unimplemented!()
    }

    fn run_latex<T: Read>() -> BoxFuture<BufReader<T>, io::Error> {
        // tokio_process spawn, check exit code, and then open the file, return an async reader to
        // that file
        unimplemented!()
    }

    fn post_generated_pdf() -> BoxFuture<Response, hyper::Error> {
        unimplemented!()
    }
}

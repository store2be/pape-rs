use futures::future::Future;
use mktemp::Temp;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::collections::HashMap;
use tokio::core::Handle;

pub struct Workspace {
    dir: Temp,
    document_spec: DocumentGenerationRequest,
    handle: Handle,
}

/// Since mktemp::Temp implements Drop by deleting the directory, we don't need to worry about
/// leaving files or directories behind.
impl Workspace {
    pub fn new(handle: Handle, spec: DocumentSpec) -> Result<Workspace, io::Error> {
        Ok(Workspace {
            dir: Temp::new_dir()?,
            document_spec,
        })
    }

    pub fn execute(self) {
        self.handle.spawn()
    }

    fn download_files() -> BoxFuture<(Workspace, Vec<File>), io::Error> {
    }

    fn generate_latex(files: Vec<File>) -> BoxFuture<Pdf, io::Error> {
    }

    // fn populate(multipart: Multipart<Vec<u8>>) -> Result<(), io::Error> {
        // get the template from the payload

        // get the variables from the payload

        // render the template with the variables

        // save the rendered template to the workspace

        // save the other files to the template

        // unimplemented!()
    // }

    fn run_latex<T: Read>() -> future::BoxFuture<BufReader<T>, io::Error> {
        // tokio_process spawn, check exit code, and then open the file, return an async reader to
        // that file
        unimplemented!()
    }
}

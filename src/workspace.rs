use futures::future;
use mktemp::Temp;
use std::io;
use multipart::server::Multipart;
use std::io::buffered::BufReader;

struct WorkspaceContents {
    template: String,
    assets: Vec<Vec<u8>>,
    variables: HashMap<String, String>,
}

pub struct Workspace {
    dir: Temp,
}

/// Since mktemp::Temp implements Drop by deleting the directory, we don't need to worry about
/// leaving files or directories behind.
impl Workspace {
    pub fn new() -> Result<Workspace, io::Error> {
        Ok(Workspace {
            dir: Temp::new_dir()?,
        })
    }

    fn populate(multipart: Multipart) -> Result<(), io::Error> {
        // get the template from the payload

        // get the variables from the payload

        // render the template with the variables

        // save the rendered template to the workspace

        // save the other files to the template

        unimplemented!()
    }

    pub fn run_latex() -> future::AndThen<BufReader, io::Error> {
        // tokio_process spawn, check exit code, and then open the file, return an async reader to
        // that file
        unimplemented!()
    }
}

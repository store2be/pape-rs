use futures::future;
use mktemp::Temp;
use std::io;
use multipart::server::{Multipart, MultipartFile};
use std::io::prelude::*;
use std::io::BufReader;
use std::collections::HashMap;

struct DocumentGenerationRequest {
    template_url: Url,
    assets_urls: Vec<Url>,
    callback_url: Url,
    variables: Hashmap<String, String>,
}

struct Filename(String);
struct FileBytes(Vec<u8>);

struct WorkspaceContents {
    template: String,
    assets: Vec<(Filename, FileBytes)>,
    variables: HashMap<String, String>,
}

impl WorkspaceContents {
    fn from_multipart<T: Read>(mut multipart: Multipart<T>) -> Result<Self, io::Error> {
        use multipart::server::MultipartData::*;

        let mut contents = WorkspaceContents {
            template: String::new(),
            assets: Vec::new(),
            variables: HashMap::new(),
        };

        multipart.foreach_entry(|entry| {
            match (entry.name.as_str(), entry.data) {
                ("template.tex", File(mut file)) => {
                    file.read_to_string(&mut contents.template);
                },
                // ("endpoint", Text(endpoint_url)) => {
                //     unimplemented!()
                // },
                (field_name, File(mut file)) => {
                    let mut buf: Vec<u8> = Vec::new();
                    file.read_to_end(&mut buf);
                    let filename = file.filename.unwrap_or(field_name.to_string());
                    contents.assets.push((Filename(filename), FileBytes(buf)))
                },
                (variable_name, Text(text)) => {
                    contents.variables.insert(variable_name.to_owned(), text.text);
                },
            };
        });

        Ok(contents)
    }
}

pub struct Workspace {
    dir: Temp,
}

/// Since mktemp::Temp implements Drop by deleting the directory, we don't need to worry about
/// leaving files or directories behind.
impl Workspace {
    pub fn new<T: Read>(multipart: Multipart<T>) -> Result<Workspace, io::Error> {
        Ok(Workspace {
            dir: Temp::new_dir()?,
        })
    }

    // fn populate(multipart: Multipart<Vec<u8>>) -> Result<(), io::Error> {
        // get the template from the payload

        // get the variables from the payload

        // render the template with the variables

        // save the rendered template to the workspace

        // save the other files to the template

        // unimplemented!()
    // }

    pub fn run_latex<T: Read>() -> future::BoxFuture<BufReader<T>, io::Error> {
        // tokio_process spawn, check exit code, and then open the file, return an async reader to
        // that file
        unimplemented!()
    }
}

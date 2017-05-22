///! This binary aims to make it simple to test a template locally: it serves the assets and the
///! template from the local directory, and receives the PDF from the callback endpoint.

extern crate futures;
#[macro_use]
extern crate lazy_static;
extern crate multipart;
extern crate papers;
extern crate tokio_core;
#[macro_use]
extern crate serde_json as json;

use futures::Future;
use std::fs::File;
use std::io::prelude::*;

use papers::prelude::*;
use papers::renderer::*;

fn main() {
    let core = tokio_core::reactor::Core::new().unwrap();

    lazy_static! {
        static ref CONFIG: Config = Config::from_env();
    }

    let variables: json::Value = if let Ok(file) = File::open("variables.json") {
        let bytes: Vec<u8> = file.bytes().collect::<Result<Vec<u8>, _>>().unwrap();
        json::from_slice(&bytes).expect("variables.json is not valid JSON")
    } else {
        json!({})
    };

    let document_spec = DocumentSpec {
        assets_urls: vec![],
        callback_url: PapersUri("unreachable".parse().unwrap()),
        output_filename: "unreachable".to_string(),
        template_url: PapersUri("unreachable".parse().unwrap()),
        variables: variables,
    };

    LocalRenderer::new(&CONFIG, &core.handle())
        .render(document_spec)
        .wait()
        .unwrap()
}

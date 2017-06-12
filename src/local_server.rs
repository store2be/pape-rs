///! This binary aims to make it simple to test a template locally: it serves the assets and the
///! template from the local directory, and receives the PDF from the callback endpoint.

extern crate futures;
extern crate multipart;
extern crate papers;
extern crate tokio_core;
extern crate tera;
#[macro_use]
extern crate serde_json as json;

use std::fs::File;
use std::io::prelude::*;

use papers::prelude::*;

fn render(document_spec: DocumentSpec) {
    let DocumentSpec { variables, .. } = document_spec;
    let template_string = ::std::fs::File::open("template.tex")
        .expect("could not open template.tex")
        .bytes()
        .collect::<Result<Vec<u8>, _>>()
        .unwrap();
    let template_string = String::from_utf8(template_string).unwrap();
    let rendered_template = tera::Tera::one_off(&template_string, &variables, false)
        .expect("failed to render the template");
    let mut rendered_template_file = ::std::fs::File::create("rendered.tex")
        .expect("could not create rendered.tex");
    rendered_template_file
        .write_all(rendered_template.as_bytes())
        .unwrap();
    let output = ::std::process::Command::new("xelatex")
        .arg("-interaction=nonstopmode")
        .arg("-file-line-error")
        .arg("-shell-restricted")
        .arg("rendered.tex")
        .output()
        .expect("latex error")
        .stdout;
    println!("{}", String::from_utf8(output).unwrap());
}

fn main() {
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

    render(document_spec)
}

///! This binary aims to make it simple to test a template locally: it serves the assets and the
///! template from the local directory, and receives the PDF from the callback endpoint.

extern crate futures;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate mime;
extern crate multipart;
extern crate papers;
extern crate hyper;
extern crate tokio_core;
#[macro_use]
extern crate serde_json as json;

use futures::{future, Future};
use hyper::{Request, Response};
use hyper::server::{self, Service};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use papers::http::*;
use papers::prelude::*;

/// This is a simple service that fills two roles:
///
/// - serves the local directory
/// - it receives the generated PDF on the /callback endpoint
struct LocalServer;

impl FromHandle for LocalServer {
    fn build(_: &tokio_core::reactor::Handle) -> Self {
        LocalServer
    }
}

impl server::Service for LocalServer {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = server::Response, Error = hyper::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let path = req.path().to_string();
        match path.as_str() {
            "/callback" => {
                println!("Callback endpoint called");
                let headers = req.headers().clone();
                println!("Headers: {:?}", headers);
                Box::new(req.get_body_bytes()
                             .from_err()
                             .map(|bytes| {
                    println!("{} bytes received", bytes.len());
                    let mut multipart =
                        multipart::server::Multipart::from_request(MultipartRequest(headers,
                                                                                    bytes))
                                .expect("could not parse multipart");
                    {
                        let mut entry = multipart
                            .read_entry()
                            .expect("could not parse next field")
                            .expect("next field is empty");
                        if entry.name != "file" {
                            panic!("{:?} {:?}", entry.name, entry.data.as_text().unwrap())
                        } else {
                            let file = entry.data.as_file().unwrap();
                            let filename = file.filename.clone().unwrap();
                            let mut out = File::create(Path::new(&filename)).unwrap();
                            let bytes = file.bytes().collect::<Result<Vec<u8>, _>>().unwrap();
                            out.write_all(&bytes).unwrap()
                        }
                    }
                    multipart.save().with_dir(".");
                }).map(|_| {
                    server::Response::new()
                }).or_else(|err: Error| {
                    println!("Error on callback endpoint: {}", err);
                    future::ok(server::Response::new())
                }))
            },
            path => {
                let file_path = path.trim_left_matches('/');
                let file = File::open(Path::new(file_path))
                    .expect(&format!("couln't read {}", path));
                let bytes: Vec<u8> = file.bytes().collect::<Result<Vec<u8>, _>>().unwrap();
                Box::new(future::ok(server::Response::new().with_body(bytes)))
            }
        }
    }
}

fn main() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    lazy_static! {
        static ref CONFIG: Config = Config::from_env();
    }
    let papers: Papers<ConcreteRenderer<LocalServer>> = Papers::new(core.remote(), &CONFIG);

    let variables: json::Value = if let Ok(file) = File::open("variables.json") {
        let bytes: Vec<u8> = file.bytes().collect::<Result<Vec<u8>, _>>().unwrap();
        json::from_slice(&bytes).expect("variables.json is not valid JSON")
    } else {
        json!({})
    };

    let assets: Vec<PapersUri> = ::std::fs::read_dir(::std::path::Path::new("."))
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_name().to_str().unwrap() != "template.tex")
        .map(|entry| {
                 let filename = entry.file_name();
                 let filename = filename.to_str().unwrap();
                 PapersUri(format!("http://127.0.0.1:8733/{}", filename)
                               .parse()
                               .unwrap())
             })
        .collect();

    let document_spec = DocumentSpec {
        assets_urls: assets,
        callback_url: PapersUri("http://127.0.0.1:8733/callback".parse().unwrap()),
        output_filename: "out.pdf".to_string(),
        template_url: PapersUri("http://127.0.0.1:8733/template.tex".parse().unwrap()),
        variables: variables,
    };

    let req = Request::new(
            hyper::Method::Post,
            "http://127.0.0.1:8019/submit".parse().unwrap())
        .with_body( json::to_string(&document_spec).unwrap().into() )
        .with_header(hyper::header::ContentType(mime!(Application/Json)));
    core.run(papers.call(req)).unwrap();
}

///! This binary aims to make it simple to test a template locally: it serves the assets and the
///! template from the local directory, and receives the PDF from the callback endpoint.

extern crate futures;
#[macro_use]
extern crate mime;
extern crate papers;
extern crate hyper;
extern crate tokio_core;
extern crate tokio_service;
extern crate serde_json as json;

use papers::http::*;

use futures::future;
use futures::{Future, Sink, Stream};
use futures::sync::mpsc;
use hyper::server;
use hyper::client;
use papers::papers::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use tokio_service::Service;

/// This is a simple service that fills two roles:
///
/// - serves the local directory
/// - it receives the generated PDF on the /callback endpoint
struct LocalServer {
    sender: mpsc::Sender<()>,
}

impl LocalServer {
    pub fn new(sender: mpsc::Sender<()>) -> LocalServer {
        LocalServer {
            sender: sender,
        }
    }
}

impl Service for LocalServer {
    type Request = server::Request;
    type Response = server::Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=server::Response, Error=hyper::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let path = req.path().to_string();
        let sender = self.sender.clone();
        match path.as_str() {
            "/callback" => {
                Box::new(req.body().fold(Vec::new(), |mut acc, chunk| {
                    acc.extend_from_slice(&chunk);
                    future::ok::<_, hyper::Error>(acc)
                }).map(|bytes| {
                    let mut file = File::create(Path::new("out.pdf")).expect("can't create out.pdf");
                    file.write_all(&bytes).expect("could not write out.pdf");
                }).and_then(move |_| {
                    sender.send(()).map_err(|_| hyper::Error::Incomplete)
                }).map(|_| {
                    server::Response::new()
                }))
            },
            path => {
                let file_path = path.trim_left_matches('/');
                let file = File::open(Path::new(file_path)).expect(&format!("couln't read {}", path));
                let bytes: Vec<u8> = file.bytes().collect::<Result<Vec<u8>, _>>().unwrap();
                Box::new(future::ok(server::Response::new().with_body(bytes)))
            }
        }
    }
}

fn main() {
    let (sender, receiver) = mpsc::channel::<()>(5);

    std::thread::spawn(|| {
        papers::server::Server::new().with_port(8019).start();
    });

    std::thread::spawn(move || {
        hyper::server::Http::new()
            .bind(&"127.0.0.1:8733".parse().unwrap(), move || Ok(LocalServer::new(sender.clone())))
            .expect("could not bind to 127.0.0.1:8733")
            .run()
            .unwrap();
    });

    let variables: HashMap<String, String> = if let Ok(file) = File::open("variables.json") {
        let bytes: Vec<u8> = file.bytes().collect::<Result<Vec<u8>, _>>().unwrap();
        json::from_slice(&bytes).expect("variables.json is invalid json")
    } else {
        HashMap::new()
    };

    let document_spec = DocumentSpec {
        assets_urls: Vec::new(),
        callback_url: PapersUri("http://127.0.0.1:8733/callback".parse().unwrap()),
        template_url: PapersUri("http://127.0.0.1:8733/template.tex".parse().unwrap()),
        variables: variables,
    };

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let client = hyper::Client::new(&core.handle());
    let req = client::Request::new(
            hyper::Method::Post,
            "http://127.0.0.1:8019/submit".parse().unwrap()
        ).with_body( json::to_string(&document_spec).unwrap())
        .with_header(hyper::header::ContentType(mime!(Application/Json)));
    let work = client.request(req).then(|_| receiver.into_future());
    core.run(work).unwrap();
}

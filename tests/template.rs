extern crate futures;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate mime;
extern crate hyper;
extern crate tokio_core;
extern crate papers;
extern crate serde_json as json;

use futures::future;
use futures::Future;
use hyper::client::Request;
use hyper::server::{self, Service};
use hyper::header::ContentType;
use papers::prelude::*;
use papers::http::*;

static TEMPLATE: &'static str = r"
\documentclass{article}

\begin{document}
hello, {{who}}
\end{document}
";

static EXPECTED_TEMPLATE_RESULT: &'static str = r"
\documentclass{article}

\begin{document}
hello, world
\end{document}
";

struct MockServer;

impl FromHandle for MockServer {
    fn build(_: &tokio_core::reactor::Handle) -> Self {
        MockServer
    }
}

impl Service for MockServer {
    type Request = server::Request;
    type Response = server::Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = server::Response, Error = hyper::Error>>;

    fn call(&self, _: Self::Request) -> Self::Future {
        Box::new(future::ok(hyper::server::Response::new().with_body(TEMPLATE)))
    }
}

#[test]
fn test_simple_template_preview() {
    let document_spec = r#"{
        "template_url": "http://127.0.0.1:8732/test",
        "callback_url": "/",
        "variables": {
            "who": "world"
        }
    }"#;

    let request = Request::new(
        hyper::Method::Post,
        "http://127.0.0.1:8019/preview".parse().unwrap())
        .with_body(document_spec.into())
        .with_header(ContentType(mime!(Application/Json)));
    let core = tokio_core::reactor::Core::new().unwrap();

    lazy_static! {
        static ref CONFIG: Config = Config::from_env();
    }

    let papers: Papers<ConcreteRenderer<MockServer>> = Papers::new(core.remote(), &CONFIG);
    let response = papers.call(request).wait().unwrap();
    let status = response.status();
    let body: Vec<u8> = response.get_body_bytes().wait().unwrap();
    assert_eq!(status, hyper::StatusCode::Ok);
    assert_eq!(::std::str::from_utf8(&body).unwrap(),
               EXPECTED_TEMPLATE_RESULT);
}

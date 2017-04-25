extern crate futures;
#[macro_use]
extern crate mime;
extern crate hyper;
extern crate tokio_core;
extern crate tokio_service;
extern crate papers;
extern crate serde_json as json;

use futures::future;
use futures::{Future, Stream};
use hyper::client::{Client, Request};
use tokio_service::Service;
use hyper::server;
use hyper::header::ContentType;

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

impl Service for MockServer {
    type Request = server::Request;
    type Response = server::Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=server::Response, Error=hyper::Error>>;

    fn call(&self, _: Self::Request) -> Self::Future {
        Box::new(future::ok(hyper::server::Response::new().with_body(TEMPLATE)))
    }
}

#[test]
fn test_simple_template_preview() {
    std::thread::spawn(|| {
        papers::server::Server::new().with_port(8019).start();
    });

    std::thread::spawn(|| {
        hyper::server::Http::new()
            .bind(&"127.0.0.1:8732".parse().unwrap(), || Ok(MockServer))
            .unwrap()
            .run()
            .unwrap();
    });

    std::thread::sleep(std::time::Duration::from_millis(20));

    let mut core = tokio_core::reactor::Core::new().unwrap();

    let handle = core.handle();
    let test_client = Client::new(&handle.clone());

    let document_spec = r#"{
        "template_url": "http://127.0.0.1:8732/test",
        "callback_url": "/",
        "variables": {
            "who": "world"
        }
    }"#;

    let request = {
        let mut req = Request::new(
            hyper::Method::Post,
            "http://127.0.0.1:8019/preview".parse().unwrap());
        req.set_body(document_spec);
        {
            let mut headers = req.headers_mut();
            headers.set(ContentType(mime!(Application/Json)));
        }
        req
    };
    let test = test_client.request(request)
        .and_then(|res| {
            let status = res.status();
            res.body().fold(Vec::new(), |mut acc, chunk| {
                acc.extend_from_slice(&chunk);
                future::ok::<_, hyper::Error>(acc)
            }).map(move |body| (status, body))
        });

    let (status, body) = core.run(test).unwrap();
    assert_eq!(status, hyper::StatusCode::Ok);
    assert_eq!(::std::str::from_utf8(&body).unwrap(), EXPECTED_TEMPLATE_RESULT);
}

extern crate futures;
#[macro_use]
extern crate mime;
extern crate hyper;
extern crate slog;
extern crate tokio_core;
extern crate papers;
extern crate serde_json as json;

use futures::future;
use futures::{Future, Stream, Sink};
use hyper::client::{Client, Request};
use hyper::server;
use hyper::header::ContentType;
use futures::sync::mpsc;

use papers::http::*;

static TEMPLATE: &'static str = r"
\documentclass{article}

\begin{document}
hello, {{who}}

\end{document}
";

struct MockServer {
    sender: mpsc::Sender<server::Request>,
}

impl MockServer {
    pub fn new(sender: mpsc::Sender<server::Request>) -> MockServer {
        MockServer { sender: sender }
    }
}

impl server::Service for MockServer {
    type Request = server::Request;
    type Response = server::Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = server::Response, Error = hyper::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let res = match req.path() {
            "/assets/logo.png" => server::Response::new().with_body(b"54321" as &[u8]),
            "/template" => {
                std::thread::sleep(std::time::Duration::from_millis(20));
                server::Response::new().with_body(TEMPLATE)
            },
            "/callback" => server::Response::new(),
            _ => server::Response::new().with_status(hyper::StatusCode::NotFound),

        };
        Box::new(self.sender
                     .clone()
                     .send(req)
                     .map(|_| res)
                     .map_err(|_| hyper::Error::Incomplete))
    }
}

#[test]
fn test_end_to_end() {
    let (sender, receiver) = mpsc::channel(30);

    std::thread::spawn(|| { papers::server::Server::new().with_port(8019).start(); });

    std::thread::spawn(move || {
                           hyper::server::Http::new()
                               .bind(&"127.0.0.1:8733".parse().unwrap(),
                                     move || Ok(MockServer::new(sender.clone())))
                               .unwrap()
                               .run()
                               .unwrap();
                       });

    std::thread::sleep(std::time::Duration::from_millis(20));

    let mut core = tokio_core::reactor::Core::new().unwrap();

    let handle = core.handle();
    let test_client = Client::new(&handle.clone());

    let document_spec = r#"{
        "assets_urls": ["http://127.0.0.1:8733/assets/logo.png", "http://127.0.0.1/8733/dead-end/"],
        "template_url": "http://127.0.0.1:8733/template",
        "callback_url": "http://127.0.0.1:8733/callback",
        "variables": {
            "who": "peter"
        }
    }"#;

    let request: Request<hyper::Body> =
        Request::new(hyper::Method::Post,
                     "http://127.0.0.1:8019/submit".parse().unwrap())
                .with_body(document_spec.into())
                .with_header(ContentType(mime!(Application / Json)));

    let test = test_client
        .request(request)
        .and_then(|res| {
            let status = res.status();
            res.body()
                .fold(Vec::new(), |mut acc, chunk| {
                    acc.extend_from_slice(&chunk);
                    future::ok::<_, hyper::Error>(acc)
                })
                .map(move |body| (status, body))
        });

    let expected_requests: Vec<Result<(&'static str, hyper::Method), ()>> =
        vec![("/template", hyper::Method::Get),
             ("/assets/logo.png", hyper::Method::Get),
             ("/callback", hyper::Method::Post)]
                .into_iter()
                .map(Ok)
                .collect();

    let expectations = receiver
        .take(expected_requests.len() as u64)
        .zip(futures::stream::iter(expected_requests))
        .for_each(|(request, schema)| {
                      assert_eq!(request.path(), schema.0);
                      assert_eq!(request.method(), &schema.1);
                      future::ok(())
                  });

    // Request + expectations
    let tests = test.map_err(|_| ())
        .and_then(|res| expectations.map(|_| res));

    let (status, body) = core.run(tests).unwrap();
    assert_eq!(status, hyper::StatusCode::Ok);
    assert_eq!(::std::str::from_utf8(&body).unwrap(), "");
}

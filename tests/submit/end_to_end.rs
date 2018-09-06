extern crate futures;
extern crate hyper;
extern crate mime;
extern crate papers;
extern crate serde_json as json;
extern crate slog;
extern crate tokio_core;

use futures::future;
use futures::sync::mpsc;
use futures::{Future, Sink, Stream};
use hyper::client::{Client, Request};
use hyper::header::ContentType;
use hyper::server;
use toolbox;

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
                ::std::thread::sleep(::std::time::Duration::from_millis(20));
                server::Response::new().with_body(TEMPLATE)
            }
            "/callback" => {
                // TODO: It would be nice if we could write this to check that the service reported
                // a successful PDF generation, but this is for later, since we consume the request
                // later in the process.

                // let bytes = req.get_body_bytes().wait().expect("could not read response");
                // let summary =
                // json::from_slice::<Summary>(&bytes).expect("response was not valid");
                // if let Summary::Error(err) = summary {
                //     panic!("Error reported to callback endpoint: {}", err);
                // }
                server::Response::new()
            }
            _ => server::Response::new().with_status(hyper::StatusCode::NotFound),
        };
        Box::new(
            self.sender
                .clone()
                .send(req)
                .map(|_| res)
                .map_err(|_| hyper::Error::Incomplete),
        )
    }
}

pub fn test_end_to_end() {
    let (sender, receiver) = mpsc::channel(30);

    let mock_port = toolbox::random_port();
    let papers_port = toolbox::random_port();

    let _join_papers = ::std::thread::spawn(move || {
        papers::server::Server::new()
            .with_port(i32::from(papers_port))
            .start()
            .unwrap();
    });

    let _join_mock = ::std::thread::spawn(move || {
        hyper::server::Http::new()
            .bind(
                &format!("127.0.0.1:{}", mock_port).parse().unwrap(),
                move || Ok(MockServer::new(sender.clone())),
            )
            .expect("could not bind")
            .run()
            .expect("could not run");
    });

    ::std::thread::sleep(::std::time::Duration::from_millis(400));

    let mut core = tokio_core::reactor::Core::new().expect("could not create reactor");

    let handle = core.handle();
    let test_client = Client::new(&handle.clone());

    // Some URLs in this test have whitespace left in on purpose to test parsing
    let document_spec = format!(
        r#"{{
        "assets_urls": ["  http://127.0.0.1:{port}/assets/logo.png       "],
        "template_url": "     http://127.0.0.1:{port}/template  ",
        "callback_url": " http://127.0.0.1:{port}/callback ",
        "variables": {{
            "who": "peter"
        }}
    }}"#,
        port = mock_port
    );

    let request: Request<hyper::Body> = Request::new(
        hyper::Method::Post,
        format!("http://127.0.0.1:{}/submit", papers_port)
            .parse()
            .unwrap(),
    ).with_body(document_spec.into())
        .with_header(ContentType(mime::APPLICATION_JSON));

    let test = test_client.request(request).and_then(|res| {
        let status = res.status();
        res.body()
            .fold(Vec::new(), |mut acc, chunk| {
                acc.extend_from_slice(&chunk);
                future::ok::<_, hyper::Error>(acc)
            })
            .map(move |body| (status, body))
    });

    let expected_requests: Vec<Result<(&'static str, hyper::Method), ()>> = vec![
        ("/template", hyper::Method::Get),
        ("/assets/logo.png", hyper::Method::Get),
        ("/callback", hyper::Method::Post),
    ].into_iter()
        .map(Ok)
        .collect();

    let expectations = receiver
        .take(expected_requests.len() as u64)
        .zip(futures::stream::iter_ok(expected_requests))
        .for_each(|(request, schema)| {
            let schema = schema.unwrap();
            assert_eq!(request.path(), schema.0);
            assert_eq!(request.method(), &schema.1);
            future::ok(())
        });

    // Request + expectations
    let tests = test
        .map_err(|err| println!("Test error: {}", err))
        .and_then(|res| expectations.map(|_| res));

    let (status, body) = core.run(tests).expect("tests failed");
    assert_eq!(status, hyper::StatusCode::Ok);
    assert_eq!(
        ::std::str::from_utf8(&body).expect("body is not valid utf8"),
        ""
    );
}

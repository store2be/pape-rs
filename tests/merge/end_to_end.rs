use futures;
use futures::future::*;
use futures::sync::mpsc;
use futures::{Future, Sink, Stream};
use futures_cpupool::CpuPool;
use hyper;
use hyper::client::Client;
use hyper::header::ContentType;
use hyper::server;
use hyper::{Request, Response};
use json;
use mime;
use std::path::*;
use tokio_core;

use papers;
use papers::http::*;
use papers::prelude::*;
use toolbox;

type Message = &'static str;

struct MockServer {
    sender: mpsc::Sender<Message>,
}

impl MockServer {
    pub fn new(sender: mpsc::Sender<Message>) -> MockServer {
        MockServer { sender }
    }
}

impl server::Service for MockServer {
    type Request = server::Request;
    type Response = server::Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = server::Response, Error = hyper::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let path = req.path().to_owned();
        match &*path {
            "/logo.png" => {
                println!("logo.png endpoint was called");
                let res = Response::new().with_file_unsafe(Path::new("tests/assets/logo.png"));
                Box::new(ok(res))
            }
            "/doc.pdf" => {
                println!("doc.pdf endpoint was called");
                let res = Response::new().with_file_unsafe(Path::new("tests/assets/doc.pdf"));
                Box::new(ok(res))
            }
            "/callback" => {
                let sender = self.sender.clone();
                let res = req
                    .get_body_bytes()
                    .and_then(|bytes| {
                        ok(json::from_slice::<Summary>(&bytes).expect("could not read summary"))
                    })
                    .map(|summary| {
                        if let Summary::Error { error: err, .. } = summary {
                            panic!("Error reported to callback endpoint: {}", err);
                        }
                        summary
                    })
                    .and_then(move |_| {
                        sender
                            .send("callback ok")
                            .map_err(|_| ErrorKind::InternalServerError.into())
                    })
                    .and_then(|_| {
                        println!("Sending back response from callback endpoint");
                        ok(server::Response::new())
                    })
                    .map_err(|_| hyper::Error::Incomplete);
                Box::new(res)
            }
            other => panic!("Unexpected request to {}", other),
        }
    }
}

pub fn test_end_to_end() {
    let pool = CpuPool::new(3);
    let (sender, receiver) = mpsc::channel(30);

    let mock_port = toolbox::random_port();
    let papers_port = toolbox::random_port();

    let mut join_papers = pool.spawn_fn(move || {
        papers::server::Server::new()
            .with_port(i32::from(papers_port))
            .start()
    });

    let mut join_mock = pool.spawn_fn(move || {
        println!("Starting mock server on port {}", mock_port);
        hyper::server::Http::new()
            .bind(
                &format!("127.0.0.1:{}", mock_port).parse().unwrap(),
                move || Ok(MockServer::new(sender.clone())),
            )
            .expect("could not bind")
            .run()
    });

    ::std::thread::sleep(::std::time::Duration::from_millis(400));

    let mut core = tokio_core::reactor::Core::new().expect("could not create reactor");

    let handle = core.handle();
    let test_client = Client::new(&handle.clone());

    let merge_spec = format!(
        r#"{{
        "assets_urls": ["http://127.0.0.1:{port}/logo.png", "http://127.0.0.1:{port}/doc.pdf"],
        "callback_url": "http://127.0.0.1:{port}/callback"
    }}"#,
        port = mock_port
    );

    let request: Request<hyper::Body> = Request::new(
        hyper::Method::Post,
        format!("http://127.0.0.1:{}/merge", papers_port)
            .parse()
            .unwrap(),
    ).with_body(merge_spec.into())
        .with_header(ContentType(mime::APPLICATION_JSON));

    let test = test_client.request(request);

    let expected_messages: Vec<Result<Message, ()>> =
        vec!["callback ok"].into_iter().map(Ok).collect();

    let expectations = receiver
        .take(expected_messages.len() as u64)
        .zip(futures::stream::iter_ok(expected_messages))
        .for_each(|(message, expected)| {
            assert_eq!(Ok(message), expected);
            ok(())
        });

    // Request + expectations
    let tests = test.map_err(|err| panic!("Test error: {}", err))
        .and_then(|res| expectations.map(|_| res))
        // Crash if any of the servers panicked
        .then(move |res| {
            if let Err(e) = join_mock.poll() {
                panic!("Mock server panicked: {}", e)
            }

            if let Err(e) = join_papers.poll() {
                panic!("Papers server panicked: {}", e)
            }

            res
        });

    let res = core.run(tests).expect("tests failed");
    assert_eq!(res.status(), hyper::StatusCode::Ok);
}

pub fn test_rejection() {
    let pool = CpuPool::new(3);
    let (_sender, receiver) = mpsc::channel(30);

    let papers_port = toolbox::random_port();

    let mut join_papers = pool.spawn_fn(move || {
        papers::server::Server::new()
            .with_port(i32::from(papers_port))
            .start()
    });

    ::std::thread::sleep(::std::time::Duration::from_millis(400));

    let mut core = tokio_core::reactor::Core::new().expect("could not create reactor");

    let handle = core.handle();
    let test_client = Client::new(&handle.clone());

    let merge_spec = format!(
        r#"{{
        "assets_urls": [],
        "callback_url": "http://127.0.0.1:8888/callback"
    }}"#,
    );

    let request: Request<hyper::Body> = Request::new(
        hyper::Method::Post,
        format!("http://127.0.0.1:{}/merge", papers_port)
            .parse()
            .unwrap(),
    ).with_body(merge_spec.into())
        .with_header(ContentType(mime::APPLICATION_JSON));

    let test = test_client.request(request);

    let expected_messages: Vec<Result<Message, ()>> = vec![].into_iter().map(Ok).collect();

    let expectations = receiver
        .take(expected_messages.len() as u64)
        .zip(futures::stream::iter_ok(expected_messages))
        .for_each(|(message, expected)| {
            assert_eq!(Ok(message), expected);
            ok(())
        });

    // Request + expectations
    let tests = test.map_err(|err| panic!("Test error: {}", err))
        .and_then(|res| expectations.map(|_| res))
        // Crash if any of the servers panicked
        .then(move |res| {
            if let Err(e) = join_papers.poll() {
                panic!("Papers server panicked: {}", e)
            }

            res
        });

    let res = core.run(tests).expect("tests failed");
    println!("{:?}", res);
    assert_eq!(res.status(), hyper::StatusCode::UnprocessableEntity);
}

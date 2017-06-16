extern crate futures;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate mime;
extern crate hyper;
extern crate slog;
extern crate tokio_core;
extern crate papers;
extern crate serde_json as json;

use futures::future;
use futures::{Future, Stream};
use hyper::client::{Client, Request};
use hyper::header::ContentType;

use papers::http::*;
use papers::prelude::*;

#[test]
fn test_assets_exceed_max_length() {
    lazy_static! {
        static ref CONFIG: Config = Config::from_env()
            .with_max_assets_per_document(1);
    }

    std::thread::spawn(|| {
        papers::server::Server::new()
            .with_port(8049)
            .with_config(&CONFIG)
            .start();
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

    let request: Request<hyper::Body> = Request::new(
        hyper::Method::Post,
        "http://127.0.0.1:8049/submit".parse().unwrap(),
    ).with_body(document_spec.into())
        .with_header(ContentType(mime!(Application / Json)));

    let test = test_client.request(request).and_then(|res| {
        let status = res.status();
        res.body()
            .fold(Vec::new(), |mut acc, chunk| {
                acc.extend_from_slice(&chunk);
                future::ok::<_, hyper::Error>(acc)
            })
            .map(move |body| (status, body))
    });

    // Request
    let tests = test.map_err(|_| ());

    let (status, _) = core.run(tests).unwrap();
    assert_eq!(status, hyper::StatusCode::UnprocessableEntity);
}

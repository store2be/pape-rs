extern crate futures;
extern crate hyper;
#[macro_use]
extern crate lazy_static;
extern crate mime;
extern crate papers;
extern crate serde_json as json;
extern crate slog;
extern crate tokio_core;

use futures::Future;
use hyper::client::{Client, Request};
use hyper::header::ContentType;

use papers::http::*;
use papers::prelude::*;

#[test]
fn test_assets_exceed_max_length() {
    lazy_static! {
        static ref CONFIG: Config = Config::from_env().with_max_assets_per_document(1);
    }

    std::thread::spawn(|| {
        papers::server::Server::new()
            .with_port(8049)
            .with_config(&CONFIG)
            .start()
            .unwrap();
    });

    std::thread::sleep(std::time::Duration::from_millis(500));

    let mut core = tokio_core::reactor::Core::new().expect("could not start event loop");

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
        "http://127.0.0.1:8049/submit".parse().expect("Wrong uri"),
    ).with_body(document_spec.into())
        .with_header(ContentType(mime::APPLICATION_JSON));

    let test = test_client
        .request(request)
        .and_then(|res| Ok(res.status()));

    // Request
    let tests = test.map_err(|_| ());

    let status = core.run(tests).expect("test failed");
    assert_eq!(status, hyper::StatusCode::UnprocessableEntity);
}

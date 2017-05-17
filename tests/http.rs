extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate papers;

use futures::Future;
use hyper::client::Client;

#[test]
fn test_health_check() {
    let _handle = std::thread::spawn(|| { papers::server::Server::new().start(); });

    std::thread::sleep(std::time::Duration::from_millis(20));

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let test_client = Client::new(&handle.clone());

    let test = test_client
        .get("http://127.0.0.1:8008/healthz".parse().unwrap())
        .map(|response| response.status());

    let status = core.run(test).unwrap();

    assert_eq!(status, hyper::StatusCode::Ok);
}

#[test]
fn test_404() {
    std::thread::spawn(|| { papers::server::Server::new().with_port(8018).start(); });

    std::thread::sleep(std::time::Duration::from_millis(20));

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let test_client = Client::new(&handle.clone());

    let test = test_client
        .get("http://127.0.0.1:8018/dead-end".parse().unwrap())
        .map(|response| response.status());

    let status = core.run(test).unwrap();
    assert_eq!(status, hyper::StatusCode::NotFound);
}

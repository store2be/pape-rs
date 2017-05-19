extern crate futures;
extern crate hyper;
#[macro_use]
extern crate lazy_static;
extern crate tokio_core;
extern crate papers;

use hyper::Request;
use hyper::server::Service;
use papers::papers::Papers;
use papers::config::Config;
use papers::renderer::NilRenderer;
use futures::Future;
fn config() -> &'static Config {
    lazy_static! {
        static ref CONFIG: Config = Config::from_env();
    }
    &CONFIG
}

#[test]
fn test_health_check() {
    let core = tokio_core::reactor::Core::new().unwrap();
    let service: Papers<NilRenderer> = Papers::new(core.remote(), &config());
    let request = Request::new(hyper::Method::Get, "http://127.0.0.1:8018/healthz".parse().unwrap());
    let response = service.call(request).wait().unwrap();
    assert_eq!(response.status(), hyper::StatusCode::Ok);
}

#[test]
fn test_404() {
    let core = tokio_core::reactor::Core::new().unwrap();
    let service: Papers<NilRenderer> = Papers::new(core.remote(), &config());
    let request = Request::new(hyper::Method::Get, "http://127.0.0.1:8018/dead-end".parse().unwrap());
    let response = service.call(request).wait().unwrap();
    assert_eq!(response.status(), hyper::StatusCode::NotFound);
}

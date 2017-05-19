extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate papers;

#[macro_use]
extern crate lazy_static;

use futures::Future;
use hyper::client::Request;
use hyper::server::Service;
use hyper::Method::Post;
use hyper::header::{Authorization, Bearer};
use papers::config::Config;
use papers::papers::Papers;
use papers::renderer::NoopRenderer;
use papers::http::RequestExt;

fn config_with_auth() -> &'static Config {
    lazy_static! {
        static ref CONFIG_WITH_AUTH: Config = Config::from_env().with_auth("secret-string".to_string());
    }

    &CONFIG_WITH_AUTH
}

fn config_empty_auth() -> &'static Config {
    lazy_static! {
        static ref CONFIG_EMPTY_AUTH: Config = Config::from_env().with_auth("".to_string());
    }

    &CONFIG_EMPTY_AUTH
}



#[test]
fn test_submit_ignore_auth_when_bearer_not_set() {
    let core = tokio_core::reactor::Core::new().unwrap();
    let request = Request::new(Post, "http://127.0.0.1:38018/submit".parse().unwrap());
    let service: Papers<NoopRenderer> = Papers::new(core.remote(), config_empty_auth());
    let response = service.call(request).wait().unwrap();
    // 422 error code here because there is no POST body
    assert_eq!(response.status(), hyper::StatusCode::UnprocessableEntity);
}

#[test]
fn test_submit_fails_when_auth_is_expected_but_missing() {
    let core = tokio_core::reactor::Core::new().unwrap();
    let request = Request::new(Post, "http://127.0.0.1:38019/submit".parse().unwrap());
    let service: Papers<NoopRenderer> = Papers::new(core.remote(), config_with_auth());
    let response = service.call(request).wait().unwrap();
    assert_eq!(response.status(), hyper::StatusCode::Forbidden);
}

#[test]
fn test_submit_fails_if_auth_header_does_not_match_env_var() {
    let core = tokio_core::reactor::Core::new().unwrap();
    let request = Request::new(Post, "http://127.0.0.1:38021/submit".parse().unwrap())
        .with_header(Authorization(Bearer { token: "other-string".to_string() }));
    let service: Papers<NoopRenderer> = Papers::new(core.remote(), config_with_auth());
    let response = service.call(request).wait().unwrap();
    assert_eq!(response.status(), hyper::StatusCode::Forbidden);
}

#[test]
fn test_submit_succeeds_if_auth_header_matches_env_var() {
    let core = tokio_core::reactor::Core::new().unwrap();
    let request = Request::new(Post, "http://127.0.0.1:38020/submit".parse().unwrap())
        .with_header(Authorization(Bearer { token: "secret-string".to_string() }));
    let service: Papers<NoopRenderer> = Papers::new(core.remote(), config_with_auth());
    let response = service.call(request).wait().unwrap();
    assert_eq!(response.status(), hyper::StatusCode::UnprocessableEntity);
}

#[test]
fn test_preview_fails_if_auth_header_does_not_match_env_var() {
    let core = tokio_core::reactor::Core::new().unwrap();
    let request = Request::new(Post, "http://127.0.0.1:38022/preview".parse().unwrap())
        .with_header(Authorization(Bearer { token: "other-string".to_string() }));
    let service: Papers<NoopRenderer> = Papers::new(core.remote(), config_with_auth());
    let response = service.call(request).wait().unwrap();
    assert_eq!(response.status(), hyper::StatusCode::Forbidden);
}

#[test]
fn test_preview_succeeds_if_auth_header_matches_env_var() {
    let core = tokio_core::reactor::Core::new().unwrap();
    let request = Request::new(Post, "http://127.0.0.1:38023/preview".parse().unwrap())
        .with_header(Authorization(Bearer { token: "secret-string".to_string() }));
    let service: Papers<NoopRenderer> = Papers::new(core.remote(), config_with_auth());
    let response = service.call(request).wait().unwrap();
    assert_eq!(response.status(), hyper::StatusCode::UnprocessableEntity);
}

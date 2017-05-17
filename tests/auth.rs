extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate papers;

use futures::Future;
use hyper::client::{Client, Request};
use hyper::Method::Post;
use hyper::header::{Authorization, Bearer};
use papers::http::RequestExt;

#[test]
fn test_submit_ignore_auth_when_bearer_not_set() {
    std::thread::spawn(|| {
                           papers::server::Server::new()
                               .with_port(38018)
                               .with_auth("".to_string())
                               .start();
                       });

    std::thread::sleep(std::time::Duration::from_millis(20));

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let test_client = Client::new(&handle.clone());

    let request = Request::new(Post, "http://127.0.0.1:38018/submit".parse().unwrap());
    let test = test_client
        .request(request)
        .map(|response| response.status());

    let status = core.run(test).unwrap();
    // 422 error code here because there is no POST body
    assert_eq!(status, hyper::StatusCode::UnprocessableEntity);
}

#[test]
fn test_submit_fails_when_auth_is_expected_but_missing() {
    std::thread::spawn(|| {
                           papers::server::Server::new()
                               .with_port(38019)
                               .with_auth("secret-string".to_string())
                               .start();
                       });

    std::thread::sleep(std::time::Duration::from_millis(20));

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let test_client = Client::new(&handle.clone());

    let request = Request::new(Post, "http://127.0.0.1:38019/submit".parse().unwrap());
    let test = test_client
        .request(request)
        .map(|response| response.status());

    let status = core.run(test).unwrap();
    assert_eq!(status, hyper::StatusCode::Forbidden);
}

#[test]
fn test_submit_fails_if_auth_header_does_not_match_env_var() {
    std::thread::spawn(|| {
                           papers::server::Server::new()
                               .with_port(38021)
                               .with_auth("secret-string".to_string())
                               .start();
                       });

    std::thread::sleep(std::time::Duration::from_millis(20));

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let test_client = Client::new(&handle.clone());

    let request = Request::new(Post, "http://127.0.0.1:38021/submit".parse().unwrap())
        .with_header(Authorization(Bearer { token: "other-string".to_string() }));
    let test = test_client
        .request(request)
        .map(|response| response.status());

    let status = core.run(test).unwrap();
    assert_eq!(status, hyper::StatusCode::Forbidden);
}
#[test]

fn test_submit_succeeds_if_auth_header_matches_env_var() {
    std::thread::spawn(|| {
                           papers::server::Server::new()
                               .with_port(38020)
                               .with_auth("secret-string".to_string())
                               .start();
                       });

    std::thread::sleep(std::time::Duration::from_millis(20));

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let test_client = Client::new(&handle.clone());

    let request = Request::new(Post, "http://127.0.0.1:38020/submit".parse().unwrap())
        .with_header(Authorization(Bearer { token: "secret-string".to_string() }));
    let test = test_client
        .request(request)
        .map(|response| response.status());

    let status = core.run(test).unwrap();
    assert_eq!(status, hyper::StatusCode::UnprocessableEntity);
}

#[test]
fn test_preview_fails_if_auth_header_does_not_match_env_var() {
    std::thread::spawn(|| {
                           papers::server::Server::new()
                               .with_port(38022)
                               .with_auth("secret-string".to_string())
                               .start();
                       });

    std::thread::sleep(std::time::Duration::from_millis(20));

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let test_client = Client::new(&handle.clone());

    let request = Request::new(Post, "http://127.0.0.1:38022/preview".parse().unwrap())
        .with_header(Authorization(Bearer { token: "other-string".to_string() }));
    let test = test_client
        .request(request)
        .map(|response| response.status());

    let status = core.run(test).unwrap();
    assert_eq!(status, hyper::StatusCode::Forbidden);
}

#[test]
fn test_preview_succeeds_if_auth_header_matches_env_var() {
    std::thread::spawn(|| {
                           papers::server::Server::new()
                               .with_port(38023)
                               .with_auth("secret-string".to_string())
                               .start();
                       });

    std::thread::sleep(std::time::Duration::from_millis(20));

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let test_client = Client::new(&handle.clone());

    let request = Request::new(Post, "http://127.0.0.1:38023/preview".parse().unwrap())
        .with_header(Authorization(Bearer { token: "secret-string".to_string() }));
    let test = test_client
        .request(request)
        .map(|response| response.status());

    let status = core.run(test).unwrap();
    assert_eq!(status, hyper::StatusCode::UnprocessableEntity);
}

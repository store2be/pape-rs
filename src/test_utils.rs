use futures::future::*;
use hyper;
use hyper::server::Service;
use hyper::{Request, Response};
use papers::FromHandle;
use tokio_core::reactor::Handle;

/// A service that should never be called. This is meant for testing.
#[derive(Debug, Clone)]
pub struct NilService;

impl FromHandle for NilService {
    fn build(_: &Handle) -> NilService {
        NilService
    }
}

impl Service for NilService {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Response, Error = hyper::Error>>;

    fn call(&self, _: Self::Request) -> Self::Future {
        unimplemented!();
    }
}

/// A service that does nothing. Meant for testing.
#[derive(Debug, Clone)]
pub struct NoopService;

impl Service for NoopService {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Response, Error = hyper::Error>>;

    fn call(&self, _: Self::Request) -> Self::Future {
        Box::new(ok(Response::new()))
    }
}

impl FromHandle for NoopService {
    fn build(_: &Handle) -> NoopService {
        NoopService
    }
}

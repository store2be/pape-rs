use error::*;
use futures::future;
use futures::{Future, Stream};
use hyper;
use std::path::PathBuf;
use hyper::header::{Header, ContentType};
use hyper::client::{Client, Request, Response};
use multipart::client::lazy;
use hyper::header::{Location};
use hyper::{Uri, StatusCode};
use std::io::prelude::*;

pub trait ResponseExt {
    fn get_body_bytes(self) -> Box<Future<Item=Vec<u8>, Error=Error>>;
}

impl ResponseExt for Response {
    fn get_body_bytes(self) -> Box<Future<Item=Vec<u8>, Error=Error>> {
        Box::new(self.body().map_err(Error::from).fold(Vec::new(), |mut acc, chunk| {
            acc.extend_from_slice(&chunk);
            future::ok::<_, Error>(acc)
        }))
    }
}

pub trait RequestExt {
    fn with_header<T: Header>(self, header: T) -> Self;

    fn with_body<T: Into<hyper::Body>>(self, body: T) -> Self;
}

impl RequestExt for Request<hyper::Body> {
    fn with_header<T: Header>(mut self, header: T) -> Self {
        {
            let mut h = self.headers_mut();
            h.set(header);
        }
        self
    }

    fn with_body<T: Into<hyper::Body>>(self, body: T) -> Self {
        let mut req = self;
        req.set_body(body.into());
        req
    }
}

pub trait ClientExt {
    fn get_follow_redirect(self, uri: Uri) -> Box<Future<Item=Response, Error=Error>>;
}

impl ClientExt for Client<hyper::client::HttpConnector> {
    fn get_follow_redirect(self, uri: Uri) -> Box<Future<Item=Response, Error=Error>> {
        Box::new(future::loop_fn(uri, move |uri| {
            self.get(uri)
                .map_err(Error::from)
                .and_then(|res| {
                    match determine_get_result(res) {
                        Ok(GetResult::Redirect(redirect_uri)) => {
                            Ok(future::Loop::Continue(redirect_uri))
                        },
                        Ok(GetResult::Ok(res)) => Ok(future::Loop::Break(res)),
                        Err(err) => Err(err),
                    }
                })
        }))
    }
}

enum GetResult {
    Ok(Response),
    Redirect(Uri),
}

fn determine_get_result(res: Response) -> Result<GetResult> {
    match res.status() {
        StatusCode::TemporaryRedirect | StatusCode::PermanentRedirect => {
            match res.headers().get::<Location>() {
                Some(location) => Ok(GetResult::Redirect(location.parse()?)),
                None => Err(ErrorKind::UnprocessableEntity.into()),
            }
        },
        StatusCode::Ok => Ok(GetResult::Ok(res)),
        _ => Err(ErrorKind::UnprocessableEntity.into()),
    }
}

pub fn multipart_request_with_file(request: Request, path: PathBuf) -> Result<Request> {
    let mut fields = lazy::Multipart::new().add_file("file", path).prepare().unwrap();
    let mut bytes: Vec<u8> = Vec::new();
    fields.read_to_end(&mut bytes)?;
    let req = request
        .with_body(bytes)
        .with_header(ContentType(mime!(Multipart/FormData; Boundary=(fields.boundary()))));
    Ok(req)
}

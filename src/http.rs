use error::*;
use futures::future;
use futures::{Future, Stream};
use mime;
use multipart;
use hyper;
use std::path::PathBuf;
use hyper::server;
use hyper::header::{Header, ContentType};
use hyper::client::{Client, Request, Response};
use multipart::client::lazy;
use hyper::header::{Location};
use hyper::{Uri, StatusCode};
use std::io::prelude::*;

pub trait ServerRequestExt {
    fn get_body_bytes(self) -> Box<Future<Item=Vec<u8>, Error=Error>>;
    fn has_content_type(&self, mime: mime::Mime) -> bool;
}

impl ServerRequestExt for server::Request {
    fn get_body_bytes(self) -> Box<Future<Item=Vec<u8>, Error=Error>> {
        Box::new(self.body().map_err(Error::from).fold(Vec::new(), |mut acc, chunk| {
            acc.extend_from_slice(&chunk);
            future::ok::<_, Error>(acc)
        }))
    }

    fn has_content_type(&self, mime: mime::Mime) -> bool {
        use mime::Mime;

        let content_type = self.headers().get::<ContentType>().cloned();
        let Mime(top_level, sub_level, _) = mime;

        if let Some(ContentType(Mime(found_top_level, found_sub_level, _))) = content_type {
            found_top_level == top_level && found_sub_level == sub_level
        } else {
            false
        }

    }
}

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

pub fn multipart_request_with_file(request: Request, path: PathBuf) -> ::std::result::Result<Request, Error> {
    let mut fields = lazy::Multipart::new()
        .add_file("file", path)
        .prepare()
        .map_err(|_| "Failed to prepare multipart body")?;
    let mut bytes: Vec<u8> = Vec::new();
    fields.read_to_end(&mut bytes)?;
    Ok(
        request
        .with_body(bytes)
        .with_header(ContentType(mime!(Multipart/FormData; Boundary=(fields.boundary()))))
    )
}

pub fn multipart_request_with_error(request: Request, error: Error) -> Result<Request> {
    let mut fields = lazy::Multipart::new()
        .add_text("error", format!("{}", error))
        .prepare()
        .map_err(|_| "Failed to prepare multipart body")?;
    let mut bytes: Vec<u8> = Vec::new();
    fields.read_to_end(&mut bytes)?;
    Ok(
        request
        .with_body(bytes)
        .with_header(ContentType(mime!(Multipart/FormData; Boundary=(fields.boundary()))))
    )
}

#[derive(Debug)]
pub struct MultipartRequest(pub hyper::Headers, pub Vec<u8>);

impl multipart::server::HttpRequest for MultipartRequest {
    type Body = ::std::io::Cursor<Vec<u8>>;

    fn multipart_boundary(&self) -> Option<&str> {
        let content_type = self.0.get::<ContentType>();
        match content_type {
            Some(&ContentType(mime::Mime(mime::TopLevel::Multipart, mime::SubLevel::FormData, ref params))) => {
                // param is (attr, value)
                params.iter().find(|param| {
                    param.0.as_str() == "boundary"
                }).map(|param| param.1.as_str())
            },
            _ => None
        }
    }

    fn body(self) -> Self::Body {
        ::std::io::Cursor::new(self.1)
    }
}

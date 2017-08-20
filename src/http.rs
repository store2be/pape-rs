use error::*;
use futures::future;
use futures::{Future, Stream};
use mime;
use hyper;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use hyper::server::{self, Service};
use hyper::header::{ContentDisposition, ContentType, DispositionParam, Header, TransferEncoding};
use hyper::{Request, Response};
use hyper::header::Location;
use hyper::{StatusCode, Uri};
use tokio_core::reactor::Handle;

pub fn https_connector(handle: &Handle) -> HttpsConnector<HttpConnector> {
    HttpsConnector::new(4, handle).expect("Could not create an https connector")
}

pub trait ServerResponseExt {
    fn with_body<T: Into<hyper::Body>>(self, body: T) -> Self;
}

impl ServerResponseExt for server::Response {
    fn with_body<T: Into<hyper::Body>>(self, body: T) -> Self {
        let mut res = self;
        res.set_body(body.into());
        res
    }
}

pub trait ServerRequestExt {
    fn get_body_bytes(self) -> Box<Future<Item = Vec<u8>, Error = Error>>;
    fn has_content_type(&self, mime: mime::Mime) -> bool;
}

impl ServerRequestExt for server::Request {
    fn get_body_bytes(self) -> Box<Future<Item = Vec<u8>, Error = Error>> {
        Box::new(self.body().map_err(Error::from).fold(
            Vec::new(),
            |mut acc, chunk| {
                acc.extend_from_slice(&chunk);
                future::ok::<_, Error>(acc)
            },
        ))
    }

    fn has_content_type(&self, mime: mime::Mime) -> bool {
        let content_type = self.headers().get::<ContentType>().cloned();
        if let Some(content_type) = content_type {
            content_type.type_() == mime.type_() && content_type.subtype() == mime.subtype()
        } else {
            false
        }
    }
}

pub trait ResponseExt {
    fn filename(&self) -> Option<String>;
    fn get_body_bytes(self) -> Box<Future<Item = Vec<u8>, Error = Error>>;
    /// Try to populate a vector with the contents of the response body, but stop after `limit`
    /// bytes with an error.
    fn get_body_bytes_with_limit(self, limit: u32) -> Box<Future<Item = Vec<u8>, Error = Error>>;
}

impl ResponseExt for Response {
    fn get_body_bytes_with_limit(self, limit: u32) -> Box<Future<Item = Vec<u8>, Error = Error>> {
        Box::new(self.body().from_err().fold(
            Vec::<u8>::new(),
            move |mut acc, chunk| {

                if (acc.len() + chunk.len()) > limit as usize {
                    return future::err(ErrorKind::UnprocessableEntity.into());
                }

                acc.extend_from_slice(&chunk);
                future::ok::<_, Error>(acc)
            },
        ))
    }

    fn filename(&self) -> Option<String> {
        match self.headers().get::<ContentDisposition>() {
            Some(&ContentDisposition {
                parameters: ref params,
                ..
            }) => params
                .iter()
                .find(|param| match **param {
                    DispositionParam::Filename(_, _, _) => true,
                    _ => false,
                })
                .and_then(|param| {
                    if let DispositionParam::Filename(_, _, ref bytes) = *param {
                        String::from_utf8(bytes.to_owned()).ok()
                    } else {
                        None
                    }
                }),
            _ => None,
        }
    }

    fn get_body_bytes(self) -> Box<Future<Item = Vec<u8>, Error = Error>> {
        Box::new(self.body().map_err(Error::from).fold(
            Vec::new(),
            |mut acc, chunk| {
                acc.extend_from_slice(&chunk);
                future::ok::<_, Error>(acc)
            },
        ))
    }
}

pub trait RequestExt {
    fn with_header<T: Header>(self, header: T) -> Self;

    fn with_body(self, body: hyper::Body) -> Self;
}

impl RequestExt for Request {
    fn with_header<T: Header>(mut self, header: T) -> Self {
        {
            let mut h = self.headers_mut();
            h.set(header);
        }
        self
    }

    fn with_body(self, body: hyper::Body) -> Self {
        let mut req = self;
        req.set_body(body);
        req.with_header(TransferEncoding(vec![hyper::header::Encoding::Identity]))
    }
}

pub trait ClientExt {
    fn get_follow_redirect(self, uri: &Uri) -> Box<Future<Item = Response, Error = Error>>;
}

impl<S> ClientExt for S
where
    S: Service<Request = Request, Response = Response, Error = hyper::Error> + 'static,
{
    fn get_follow_redirect(self, uri: &Uri) -> Box<Future<Item = Response, Error = Error>> {
        Box::new(future::loop_fn(uri.clone(), move |uri| {
            let request = Request::new(hyper::Method::Get, uri);
            self.call(request).map_err(Error::from).and_then(
                |res| match determine_get_result(res) {
                    Ok(GetResult::Redirect(redirect_uri)) => {
                        Ok(future::Loop::Continue(redirect_uri))
                    }
                    Ok(GetResult::Ok(res)) => Ok(future::Loop::Break(res)),
                    Err(err) => Err(err),
                },
            )
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
                None => Err("Redirect without Location header".into()),
            }
        }
        _ => Ok(GetResult::Ok(res)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper;
    use hyper::server::Service;
    use futures::future;

    #[derive(Debug, Clone)]
    struct MockServer {
        response_to_logo_png: hyper::header::ContentDisposition,
    }

    impl MockServer {
        fn respond_to_logo_png_with(
            content_disposition: hyper::header::ContentDisposition,
        ) -> MockServer {
            MockServer {
                response_to_logo_png: content_disposition,
            }
        }
    }

    impl Service for MockServer {
        type Request = server::Request;
        type Response = server::Response;
        type Error = hyper::Error;
        type Future = Box<Future<Item = server::Response, Error = hyper::Error>>;

        fn call(&self, req: Self::Request) -> Self::Future {
            let res = match req.path() {
                "/assets/logo.png" => server::Response::new()
                    .with_body(b"54321" as &[u8])
                    .with_header(self.response_to_logo_png.clone()),
                _ => server::Response::new().with_status(hyper::StatusCode::NotFound),

            };
            Box::new(future::ok(res))
        }
    }


    #[test]
    fn test_filename_prefers_content_disposition() {
        let response_header = hyper::header::ContentDisposition {
            disposition: hyper::header::DispositionType::Attachment,
            parameters: vec![
                hyper::header::DispositionParam::Filename(
                    hyper::header::Charset::Ext("UTF-8".to_string()),
                    None,
                    b"this_should_be_the_filename.png".to_vec(),
                ),
            ],
        };
        let server = MockServer::respond_to_logo_png_with(response_header);

        let request: hyper::client::Request<hyper::Body> = Request::new(
            hyper::Method::Get,
            "http://127.0.0.1:8738/assets/logo.png".parse().unwrap(),
        );

        let response = server.call(request).wait().unwrap();
        assert_eq!(
            response.filename(),
            Some("this_should_be_the_filename.png".to_string())
        )
    }

    #[test]
    fn test_filename_works_with_content_disposition_inline() {
        let response_header = hyper::header::ContentDisposition {
            disposition: hyper::header::DispositionType::Inline,
            parameters: vec![
                hyper::header::DispositionParam::Filename(
                    hyper::header::Charset::Ext("UTF-8".to_string()),
                    None,
                    b"this_should_be_the_filename.png".to_vec(),
                ),
            ],
        };

        let server = MockServer::respond_to_logo_png_with(response_header);

        let request: hyper::client::Request<hyper::Body> = Request::new(
            hyper::Method::Get,
            "http://127.0.0.1:8738/assets/logo.png".parse().unwrap(),
        );

        let response = server.call(request).wait().unwrap();
        assert_eq!(
            response.filename(),
            Some("this_should_be_the_filename.png".to_string())
        )
    }

    // S3 returns Content-Disposition without disposition (just filename)
    #[test]
    fn test_content_disposition_works_without_disposition() {
        let server = MockServer::respond_to_logo_png_with(hyper::header::ContentDisposition {
            disposition: hyper::header::DispositionType::Ext("".to_string()),
            parameters: vec![
                hyper::header::DispositionParam::Filename(
                    hyper::header::Charset::Ext("UTF-8".to_string()),
                    None,
                    b"this_should_be_the_filename.png".to_vec(),
                ),
            ],
        });

        let request: hyper::client::Request<hyper::Body> = Request::new(
            hyper::Method::Get,
            "http://127.0.0.1:8740/assets/logo.png".parse().unwrap(),
        );

        let response = server.call(request).wait().unwrap();
        assert_eq!(
            response.filename(),
            Some("this_should_be_the_filename.png".to_string())
        )
    }

    struct MockFileServer;

    impl Service for MockFileServer {
        type Request = server::Request;
        type Response = server::Response;
        type Error = hyper::Error;
        type Future = Box<Future<Item = server::Response, Error = hyper::Error>>;

        fn call(&self, _: Self::Request) -> Self::Future {
            let mut response_body: Vec<u8> = Vec::with_capacity(3000);

            for n in 0..3000 {
                response_body.push((n / 250) as u8);
            }

            let res = server::Response::new().with_body(response_body);
            Box::new(future::ok(res))
        }
    }

    #[test]
    fn test_get_body_bytes_with_limit_is_enforced() {
        let request = Request::new(hyper::Method::Get, "/".parse().unwrap());
        let response = MockFileServer.call(request).wait().unwrap();
        let result = response.get_body_bytes_with_limit(2000).wait();
        match result {
            Err(Error(ErrorKind::UnprocessableEntity, _)) => (),
            other => panic!("Wrong result to get_body_bytes_max_size: {:?}", other),
        }
    }

    #[test]
    fn test_get_body_bytes_with_limit_is_not_excessively_zealous() {
        let request = Request::new(hyper::Method::Get, "/".parse().unwrap());
        let response = MockFileServer.call(request).wait().unwrap();
        let result = response.get_body_bytes_with_limit(3000).wait();
        match result {
            Ok(_) => (),
            other => panic!("Wrong result to get_body_bytes_max_size: {:?}", other),
        }
    }
}

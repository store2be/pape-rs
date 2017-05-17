mod document_spec;

use futures::future::{Future, ok, err, result};
use hyper;
use hyper::{Get, Post, Head, StatusCode};
use hyper::server::{Request, Response, Service, NewService};
use hyper::header::{Authorization, Bearer};
use serde_json;
use slog;
use tokio_core::reactor::Remote;

use http::*;
use error::{Error, ErrorKind};
pub use self::document_spec::{DocumentSpec, PapersUri};
use renderer::Renderer;

pub fn log_request(logger: &slog::Logger, req: &Request) {
    info!(
        logger,
        "{} {} IP={:?}",
        req.method(),
        req.path(),
        req.remote_addr().unwrap(),
    );
}

pub struct Papers {
    auth: String,
    remote: Remote,
    logger: slog::Logger,
    max_assets_per_document: u8,
}

impl Papers {
    pub fn new(remote: Remote, logger: slog::Logger, auth: String, max_assets_per_document: u8) -> Papers {
        Papers {
            auth,
            remote,
            logger,
            max_assets_per_document,
        }
    }

    // Check Authorization header if `PAPERS_BEARER` env var is set
    fn check_auth_header(&self, req: &Request) -> Result<(), Error> {
        let headers = req.headers().clone();
        let authorization = headers.get::<Authorization<Bearer>>();
        match authorization {
            Some(header_bearer) => {
                if self.auth != "" && header_bearer.token != self.auth {
                    return Err(Error::from_kind(ErrorKind::Forbidden));
                }
            }
            None => {
                if self.auth != "" {
                    return Err(Error::from_kind(ErrorKind::Forbidden));
                }
            }
        }
        Ok(())
    }

    fn submit(&self, req: Request) -> Box<Future<Item = Response, Error = Error>> {
        log_request(&self.logger, &req);
        debug!(self.logger, "{:#?}", req);

        if let Err(error) = self.check_auth_header(&req) {
            return Box::new(err(error));
        }

        if !req.has_content_type(mime!(Application / Json)) {
            return Box::new(err(ErrorKind::UnprocessableEntity.into()));
        }

        let handle = self.remote.handle().unwrap().clone();

        let response = req.get_body_bytes();

        let document_spec = response.and_then(|body| {
            result(serde_json::from_slice::<DocumentSpec>(body.as_slice())
                       .map_err(|err| Error::with_chain(err, ErrorKind::UnprocessableEntity)))
        });

        let max_assets_per_document = self.max_assets_per_document;
        let document_spec = document_spec.and_then(move |spec| {
            if spec.assets_urls.len() > max_assets_per_document as usize {
                return err(ErrorKind::UnprocessableEntity.into());
            }
            ok(spec)
        });

        let renderer = {
            let logger = self.logger.clone();
            let handle = handle.clone();
            document_spec
                .and_then(|document_spec| result(Renderer::new(handle, document_spec, logger)))
        };

        let response = {
            let handle = handle.clone();
            renderer
                .and_then(move |renderer| {
                              handle.spawn(renderer.execute());
                              ok(Response::new().with_status(StatusCode::Ok))
                          })
        };

        Box::new(response)
    }

    fn preview(&self, req: Request) -> Box<Future<Item = Response, Error = Error>> {
        log_request(&self.logger, &req);
        debug!(self.logger, "{:#?}", req);

        if let Err(error) = self.check_auth_header(&req) {
            return Box::new(err(error));
        }

        if !req.has_content_type(mime!(Application / Json)) {
            return Box::new(err(ErrorKind::UnprocessableEntity.into()));
        }

        let handle = self.remote.handle().unwrap();
        let logger = self.logger.clone();

        let response = req.get_body_bytes();
        let document_spec = response.and_then(|body| {
            result(serde_json::from_slice::<DocumentSpec>(body.as_slice())
                       .map_err(|_| ErrorKind::UnprocessableEntity.into()))
        });
        let renderer =
            document_spec.and_then(|document_spec| {
                                       result(Renderer::new(handle, document_spec, logger))
                                           .map_err(|err| {
                    Error::with_chain(err, ErrorKind::InternalServerError)
                })
                                   });

        let preview = renderer.and_then(|renderer| renderer.preview());

        let response = preview.and_then(|populated_template| {
                                            ok(Response::new()
                                                   .with_status(StatusCode::Ok)
                                                   .with_body(populated_template))
                                        });

        Box::new(response)

    }

    fn health_check(&self, req: Request) -> Box<Future<Item = Response, Error = Error>> {
        log_request(&self.logger, &req);
        Box::new(ok(Response::new().with_status(StatusCode::Ok)))
    }
}

impl Service for Papers {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Response, Error = hyper::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let response = match (req.method(), req.path()) {
                (&Get, "/healthz") |
                (&Head, "/healthz") => self.health_check(req),
                (&Post, "/preview") => self.preview(req),
                (&Post, "/submit") => self.submit(req),
                _ => {
                    log_request(&self.logger, &req);
                    Box::new(ok(Response::new().with_status(StatusCode::NotFound)))
                }
            }
            .then(|handler_result| match handler_result {
                      Ok(response) => ok(response),
                      Err(err) => ok(err.into_response()),
                  });

        Box::new(response)
    }
}

impl NewService for Papers {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Instance = Papers;

    fn new_service(&self) -> Result<Self::Instance, ::std::io::Error> {
        Ok(Papers {
               auth: self.auth.clone(),
               remote: self.remote.clone(),
               logger: self.logger.clone(),
               max_assets_per_document: self.max_assets_per_document,
           })
    }
}

mod document_spec;

use futures::future::{Future, ok, err, result};
use futures::sync::oneshot;
use hyper;
use hyper::{Get, Post, Head, StatusCode};
use hyper::server::{Request, Response, Service, NewService};
use hyper::header::{Authorization, Bearer};
use serde_json;
use slog;
use std::marker::PhantomData;
use tokio_core::reactor::Remote;

use http::*;
use error::{Error, ErrorKind};
pub use self::document_spec::{DocumentSpec, PapersUri};
use renderer::Renderer;
use config::Config;

pub fn log_request(logger: &slog::Logger, req: &Request) {
    info!(
        logger,
        "{} {} IP={:?}",
        req.method(),
        req.path(),
        req.remote_addr(),
    );
}

pub struct Papers<R>
    where R: Renderer
{
    remote: Remote,
    config: &'static Config,
    _renderer: PhantomData<R>,
}

impl<R: Renderer> Papers<R> {
    pub fn new(remote: Remote, config: &'static Config) -> Papers<R> {
        Papers {
            remote,
            config,
            _renderer: PhantomData,
        }
    }

    // Check Authorization header if `PAPERS_BEARER` env var is set
    fn check_auth_header(&self, req: &Request) -> Result<(), Error> {
        let headers = req.headers().clone();
        let authorization = headers.get::<Authorization<Bearer>>();
        match authorization {
            Some(header_bearer) => {
                if self.config.auth != "" && header_bearer.token != self.config.auth {
                    return Err(Error::from_kind(ErrorKind::Forbidden));
                }
            }
            None => {
                if self.config.auth != "" {
                    return Err(Error::from_kind(ErrorKind::Forbidden));
                }
            }
        }
        Ok(())
    }

    fn submit(&self, req: Request) -> Box<Future<Item = Response, Error = Error>> {
        debug!(self.config.logger, "{:#?}", req);

        if let Err(error) = self.check_auth_header(&req) {
            return Box::new(err(error));
        }

        if !req.has_content_type(mime!(Application / Json)) {
            return Box::new(err(ErrorKind::UnprocessableEntity.into()));
        }

        let body = req.get_body_bytes();

        let document_spec = body.and_then(|body| {
            result(serde_json::from_slice::<DocumentSpec>(body.as_slice())
                       .map_err(|err| Error::with_chain(err, ErrorKind::UnprocessableEntity)))
        });

        let logger = self.config.logger.clone();
        let max_assets_per_document = self.config.max_assets_per_document;
        let document_spec = document_spec.and_then(move |spec| {
            if spec.assets_urls.len() > max_assets_per_document as usize {
                error!(
                    logger,
                    "Assets URLs length exceeds the maximum ({}).\
                    To change it set PAPERS_MAX_ASSETS_PER_DOCUMENT",
                    max_assets_per_document,
                );
                return err(ErrorKind::UnprocessableEntity.into());
            }
            ok(spec)
        });

        let response = {
            let config = self.config;
            let remote = self.remote.clone();
            document_spec.and_then(move |document_spec| {
                                       remote.spawn(move |handle| {
                                                        R::new(config, handle).render(document_spec)
                                                    });
                                       ok(Response::new().with_status(StatusCode::Ok))
                                   })
        };

        Box::new(response)
    }

    fn preview(&self, req: Request) -> Box<Future<Item = Response, Error = Error>> {
        debug!(self.config.logger, "{:#?}", req);

        if let Err(error) = self.check_auth_header(&req) {
            return Box::new(err(error));
        }

        if !req.has_content_type(mime!(Application / Json)) {
            return Box::new(err(ErrorKind::UnprocessableEntity.into()));
        }

        let body = req.get_body_bytes();
        let document_spec = body.and_then(|body| {
            result(serde_json::from_slice::<DocumentSpec>(body.as_slice())
                       .map_err(|_| ErrorKind::UnprocessableEntity.into()))
        });

        let preview = {
            let remote = self.remote.clone();
            let config = self.config;
            let (sender, receiver) = oneshot::channel();
            document_spec
                .and_then(move |document_spec| {
                              remote.spawn(move |handle| {
                                               R::new(config, handle).preview(document_spec, sender)
                                           });
                              ok(())
                          })
                .and_then(move |_| receiver.map_err(|err| panic!(err)))
                .flatten()
        };

        let response = preview.and_then(|populated_template| {
                                            ok(Response::new()
                                                   .with_status(StatusCode::Ok)
                                                   .with_body(populated_template))
                                        });

        Box::new(response)

    }

    fn health_check(&self, _: Request) -> Box<Future<Item = Response, Error = Error>> {
        Box::new(ok(Response::new().with_status(StatusCode::Ok)))
    }
}

impl<R: Renderer> Service for Papers<R> {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Response, Error = hyper::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        log_request(&self.config.logger, &req);
        let response = match (req.method(), req.path()) {
                (&Get, "/healthz") |
                (&Head, "/healthz") => self.health_check(req),
                (&Post, "/preview") => self.preview(req),
                (&Post, "/submit") => self.submit(req),
                _ => Box::new(ok(Response::new().with_status(StatusCode::NotFound))),
            }
            .then(|handler_result| match handler_result {
                      Ok(response) => ok(response),
                      Err(err) => ok(err.into_response()),
                  });

        Box::new(response)
    }
}

impl<R: Renderer> NewService for Papers<R> {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Instance = Papers<R>;

    fn new_service(&self) -> Result<Self::Instance, ::std::io::Error> {
        Ok(Papers {
               remote: self.remote.clone(),
               config: self.config,
               _renderer: PhantomData,
           })
    }
}

mod document_spec;

use futures::future::{Future, ok, err, result};
use hyper;
use hyper::{Get, Post, Head, StatusCode};
use hyper::server::{Request, Response};
use serde_json;
use slog;
use tokio_service::{NewService, Service};
use tokio_core::reactor::Remote;

use http::*;
use error::{Error, ErrorKind};
pub use self::document_spec::{DocumentSpec, PapersUri};
use workspace::Workspace;

pub struct Papers {
    remote: Remote,
    logger: slog::Logger,
}

impl Papers {
    pub fn new(remote: Remote, logger: slog::Logger) -> Papers {
        Papers {
            remote: remote,
            logger: logger,
        }
    }

    fn submit(&self, req: Request) -> Box<Future<Item=Response, Error=Error>> {
        info!(
            self.logger,
            "Received a submit request ({}) from {:?}",
            req.method(),
            req.remote_addr().unwrap(),
        );
        debug!(self.logger, "Full request: {:#?}", req);

        if !req.has_content_type(mime!(Application/Json)) {
            return Box::new(err(ErrorKind::UnprocessableEntity.into()));
        }

        let remote = self.remote.clone();
        let handle = self.remote.handle().unwrap().clone();
        let logger = self.logger.clone();

        let response = req.get_body_bytes()

        // Parse the body into a DocumentSpec
        .and_then(|body| {
            result(
                serde_json::from_slice::<DocumentSpec>(body.as_slice())
                    .map_err(|err| Error::with_chain(err, ErrorKind::UnprocessableEntity))
            )
        })

        // Handle the parsed request
        .map_err(|_| ErrorKind::InternalServerError.into())
        .and_then(|document_spec| {
            result(Workspace::new(remote, document_spec, logger))
        }).and_then(move |workspace| {
            handle.spawn(workspace.execute());
            ok(Response::new().with_status(StatusCode::Ok))
        }).map_err(|_| ErrorKind::InternalServerError.into());

        Box::new(response)
    }

    fn preview(&self, req: Request) -> Box<Future<Item=Response, Error=Error>> {
        info!(
            self.logger,
            "Received a preview request ({}) from: {:?}",
            req.method(),
            req.remote_addr().unwrap(),
        );
        debug!(self.logger, "Full request: {:#?}", req);

        if !req.has_content_type(mime!(Application/Json)) {
            return Box::new(err(ErrorKind::UnprocessableEntity.into()));
        }

        let remote = self.remote.clone();
        let logger = self.logger.clone();

        let response = req.get_body_bytes()

        // Parse the body into a DocumentSpec
        .and_then(|body| {
            result(
                serde_json::from_slice::<DocumentSpec>(body.as_slice())
                    .map_err(|_| ErrorKind::UnprocessableEntity.into())
            )
        })

        // Handle the parsed request
        .and_then(|document_spec| {
            result(Workspace::new(remote, document_spec, logger))
                .map_err(|err| Error::with_chain(err, ErrorKind::InternalServerError))
        })
        .and_then(|workspace| {
            workspace.preview()
        }).and_then(|populated_template| {
            ok(Response::new()
                .with_status(StatusCode::Ok)
                .with_body(populated_template))
        });

        Box::new(response)

    }

    fn health_check(&self, req: Request) -> Box<Future<Item=Response, Error=Error>> {
        info!(
            self.logger,
            "Received a health check request ({}) from {:?}",
            req.method(),
            req.remote_addr().unwrap(),
        );
        Box::new(ok(Response::new().with_status(StatusCode::Ok)))
    }
}

impl Service for Papers {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Response, Error=hyper::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let response = match (req.method(), req.path()) {
            (&Get, "/healthz") | (&Head, "/healthz") => self.health_check(req),
            (&Post, "/preview") => self.preview(req),
            (&Post, "/submit") => self.submit(req),
            _ => {
                info!(
                    self.logger,
                    "Received a {} request to a non-existing endpoint \"{}\" from {:?}",
                    req.method(),
                    req.path(),
                    req.remote_addr().unwrap(),
                );
                Box::new(ok(Response::new().with_status(StatusCode::NotFound)))
            }
        }.then(|handler_result| {
            match handler_result {
                Ok(response) => ok(response),
                Err(err) => ok(err.into_response()),
            }
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
            remote: self.remote.clone(),
            logger: self.logger.clone(),
        })
    }
}

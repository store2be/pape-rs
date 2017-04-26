mod document_spec;

use futures::future::{Future, ok, err, result};
use futures::Stream;
use hyper;
use hyper::{Get, Post, Head, StatusCode};
use hyper::header::ContentType;
use hyper::mime::{Mime, TopLevel, SubLevel};
use hyper::server::{Request, Response};
use serde_json;
use tokio_service::{NewService, Service};
use tokio_core::reactor::Remote;

use error::{Error, ErrorKind};
pub use self::document_spec::DocumentSpec;
use workspace::Workspace;

pub struct Papers {
    remote: Remote,
}

impl Papers {
    pub fn new(remote: Remote) -> Papers {
        Papers {
            remote: remote,
        }
    }

    fn submit(&self, req: Request) -> Box<Future<Item=Response, Error=Error>> {
        let content_type = req.headers().get::<ContentType>().cloned();

        // Return an error if the content type is not application/json
        match content_type {
            Some(ContentType(Mime(TopLevel::Application, SubLevel::Json, _))) => (),
            _ => return Box::new(err(ErrorKind::UnprocessableEntity.into())),
        };

        let remote = self.remote.clone();
        let handle = self.remote.handle().unwrap().clone();

        let response = req.body()
            // Ignore hyper errors (i.e. io error, invalid utf-8, etc.) for now
            .map_err(|_| ErrorKind::UnprocessableEntity.into())
            .fold(Vec::new(), |mut acc, chunk| {
            // Receive all the body chunks into a vector
            acc.extend_from_slice(&chunk);
            ok::<_, Error>(acc)
        })


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
            result(Workspace::new(remote, document_spec))
        }).and_then(move |workspace| {
            handle.spawn(workspace.execute().map(|_| ()).map_err(|err| panic!("{}", err)));
            ok(Response::new().with_status(StatusCode::Ok))
        }).map_err(|_| ErrorKind::InternalServerError.into());

        Box::new(response)
    }

    fn preview(&self, req: Request) -> Box<Future<Item=Response, Error=Error>> {
        let content_type = {
            req.headers().get::<ContentType>().cloned()
        };

        // Return an error if the content type is not application/json
        match content_type {
            Some(ContentType(Mime(TopLevel::Application, SubLevel::Json, _))) => (),
            _ => return Box::new(err(ErrorKind::UnprocessableEntity.into())),
        };

        let remote = self.remote.clone();

        let response = req.body()
            // Ignore hyper errors (i.e. io error, invalid utf-8, etc.) for now
            .map_err(|err| Error::with_chain(err, ErrorKind::UnprocessableEntity))
            .fold(Vec::new(), |mut acc, chunk| {
            // Receive all the body chunks into a vector
            acc.extend_from_slice(&chunk);
            ok::<_, Error>(acc)
        })


        // Parse the body into a DocumentSpec
        .and_then(|body| {
            result(
                serde_json::from_slice::<DocumentSpec>(body.as_slice())
                    .map_err(|_| ErrorKind::UnprocessableEntity.into())
            )
        })

        // Handle the parsed request
        .and_then(|document_spec| {
            result(Workspace::new(remote, document_spec))
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

    fn health_check(&self, _: Request) -> Box<Future<Item=Response, Error=Error>> {
        ok(Response::new().with_status(StatusCode::Ok)).boxed()
    }
}

impl Service for Papers {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Response, Error=hyper::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        // debug!("called with uri {:?}, and method {:?}", req.path(), req.method());
        let response = match (req.method(), req.path()) {
            (&Get, "/healthz") | (&Head, "/healthz") => self.health_check(req),
            (&Post, "/preview") => self.preview(req),
            (&Post, "/submit") => self.submit(req),
            _ => ok(Response::new().with_status(StatusCode::NotFound)).boxed(),
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
        })
    }
}

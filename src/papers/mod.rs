mod document_spec;

use futures::future::{BoxFuture, Future, ok, err, result};
use futures::Stream;
use hyper;
use hyper::{Get, Post, Head, StatusCode};
use hyper::header::ContentType;
use hyper::mime::{Mime, TopLevel, SubLevel};
use hyper::server::{Request, Response};
use serde_json;
use tokio_service::{NewService, Service};
use tokio_core::reactor::Remote;

use error::Error;
pub use self::document_spec::DocumentSpec;
use workspace::Workspace;

pub struct Papers {
    remote: Remote,
}

impl Papers {
    pub fn new(remote: Remote) -> Papers {
        Papers {
            remote,
        }
    }

    fn submit(&self, req: Request) -> BoxFuture<Response, Error> {
        let content_type = {
            req.headers().get::<ContentType>().map(|c| c.clone())
        };

        // Return an error if the content type is not json/form-data
        match content_type {
            Some(ContentType(Mime(TopLevel::Application, SubLevel::Json, _))) => (),
            _ => return err(Error::UnprocessableEntity).boxed(),
        };

        let remote = self.remote.clone();

        req.body()
            // Ignore hyper errors (i.e. io error, invalid utf-8, etc.) for now
            .map_err(|_| Error::UnprocessableEntity)
            .fold(Vec::new(), |mut acc, chunk| {
            // Receive all the body chunks into a vector
            acc.extend_from_slice(&chunk);
            ok::<_, Error>(acc)
        })


        // Parse the body into a DocumentSpec
        .and_then(|body| {
            result(
                serde_json::from_slice::<DocumentSpec>(body.as_slice())
                    .map_err(|_| Error::UnprocessableEntity)
            )
        })

        // Handle the parsed request
        .and_then(|document_spec| {
            ok(match Workspace::new(remote, document_spec) {
                Ok(workspace) => {
                    workspace.execute();
                    Response::new().with_status(StatusCode::Ok)
                },
                Err(_) => {
                    Response::new().with_status(StatusCode::InternalServerError)
                },
            })
        }).boxed()
    }

    fn health_check(&self, _: Request) -> BoxFuture<Response, Error> {
        let response = Response::new().with_status(StatusCode::Ok);
        ok(response).boxed()
    }
}

impl Service for Papers {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Response, Error=hyper::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        // debug!("called with uri {:?}, and method {:?}", req.path(), req.method());
        match (req.method(), req.path()) {
            (&Get, "/healthz") => self.health_check(req),
            (&Head, "/healthz") => self.health_check(req),
            (&Post, "/submit") => self.submit(req),
            _ => ok(Response::new().with_status(StatusCode::NotFound)).boxed(),
        }.then(|handler_result| {
            match handler_result {
                Ok(response) => ok(response),
                Err(err) => ok(err.into_response()),
            }
        }).boxed()
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

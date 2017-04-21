use futures::Stream;
use futures::future::{BoxFuture, Future, ok, err, result, FutureResult};
use hyper;
use hyper::{Get, Post, StatusCode};
use hyper::header::ContentType;
use hyper::mime::{Mime, TopLevel, SubLevel, Attr, Value};
use hyper::server::{Request, Response, handle};
use tokio_service::Service;
use std::thread;
use std::path::PathBuf;
use workspace::Workspace;

pub struct Papers;

#[derive(Serialize, Deserialize)]
struct DocumentSpec {
    assets_urls: Option<Vec<Url>>,
    callback_url: Url,
    template_url: Url,
    variables: Option<Hashmap<String, String>>,
}

impl Papers {
    fn submit(&self, req: Request) -> Box<Future<Item=Response, Error=hyper::Error>> {
        let content_type = {
            req.headers().get::<ContentType>().map(|c| c.clone())
        };

        // Return an error if the content type is not json/form-data
        match content_type {
            Some(ContentType(Mime(TopLevel::Application, SubLevel::Json, _))) => (),
            _ => return ok(Response::new().with_status(StatusCode::BadRequest)),
        };

        // Receive all the body chunks into a vector
        let body = req.body().fold(Vec::new(), |mut acc, chunk| {
            acc.extend_from_slice(&chunk);
            ok::<_, hyper::Error>(acc)
        });

        let document_spec: DocumentSpec = serde_json::from_bytes(&body);

        ok(match document_spec {
            Ok(spec) => {
                match Workspace::new(handle(), spec) {
                    Ok(workspace) => {
                        workspace.execute();
                        Response::new().with_status(StatusCode::Ok)
                    },
                    Err(_) => {
                        Response::new().with_status(StatusCode::InternalServerError)
                    },
                }
            },
            Err(err) => {
                Response::new().with_status(StatusCode::UnprocessableEntity)
            },
        }

        match document_spec {
            Ok(spec) => {
                result(Workspace::new(spec)).and_then(|workspace| {
                    workspace.download_files()
                }).and_then(|(workspace, files)| {
                    workspace.generate_latex(files)
                }).and_then(|pdf| {
                    Response::new()
                        .with_status
                    .map(|_| Response::new().with_status(StatusCode::Ok)).boxed()

                })
            },
            Err(err) => err(err),
        }
    }

    fn health_check(&self, _: Request) -> BoxFuture<Response, hyper::Error> {
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
        match (req.method(), req.path()) {
            (&Get, "/healthz") => self.health_check(req),
            (&Post, "/submit") => self.submit(req),
            _ => ok(Response::new().with_status(StatusCode::NotFound)).boxed(),
        }
    }
}

use futures::Stream;
use futures::future::{BoxFuture, Future, ok, err, result, FutureResult};
use hyper;
use hyper::{Get, Post, StatusCode};
use hyper::header::ContentType;
use hyper::mime::{Mime, TopLevel, SubLevel, Attr, Value};
use hyper::server::{Request, Response};
use tokio_service::Service;
use multipart::server::Multipart;
use multipart::server::save::{Entries, SaveResult};
use std::thread;

pub struct PdfRenderer;

impl PdfRenderer {
    fn render_pdf(&self, req: Request) -> Box<Future<Item=Response, Error=hyper::Error>> {
        let headers = req.headers();

        // Return an error if the content type is not multipart/form-data
        let boundary = result(match headers.get::<ContentType>() {
            Some(&ContentType(Mime(TopLevel::Multipart, SubLevel::FormData, params))) =>
                params.iter().find(|param| {
                    match param.0 { Attr::Boundary => true, _ => false }
                }).map(|param| param.1),
            _ => None,
        }.ok_or(hyper::Error::Header));

        // Receive all the body chunks into a vector
        let body = req.body().fold(Vec::new(), |acc, chunk| {
            acc.extend_from_slice(&chunk);
            ok(acc)
        });

        // Parse the body and save the files in a temporary directory
        let multipart = boundary.join(body).map(|(boundary, body): (Value, Vec<u8>)| {
            let multipart = Multipart::with_body(body.as_slice(), boundary.as_str());
            multipart.save().temp()
        });

        // Extract the path of the temporary directory
        let workspace = multipart.and_then(|save_result: SaveResult<Entries, _>| match save_result {
            SaveResult::Full(entries) => Ok(entries),
            _ => Err(hyper::Error::Timeout)
        }).map(|entries| entries.save_dir.into_path());

        // Spawn the command in a separate thread, wait for it to finish, return the resulting pdf
        let command_output = workspace.and_then(|workspace: PathBuf| {
            // tokio_process spawn, check exit code, return an async reader to that file
        });

        // create a response
        // then forward the pdf into the response body
        // send it to the documents endpoint from the request

        // return the response
        // response.boxed()
        unimplemented!()
    }

    fn health_check(&self, _: Request) -> BoxFuture<Response, hyper::Error> {
        let response = Response::new().with_status(StatusCode::Ok);
        ok(response).boxed()
    }
}

impl Service for PdfRenderer {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Response, Error=hyper::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        match (req.method(), req.path()) {
            (&Get, "/health") => self.health_check(req),
            (&Post, "/render-pdf") => self.render_pdf(req),
            _ => ok(Response::new().with_status(StatusCode::NotFound)).boxed(),
        }
    }
}

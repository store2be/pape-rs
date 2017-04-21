use hyper::server::Response;
use hyper::{StatusCode};

pub enum Error {
    UnprocessableEntity,
}

impl Error {
    pub fn into_response(self) -> Response {
        match self {
            Error::UnprocessableEntity => Response::new().with_status(StatusCode::UnprocessableEntity),
        }
    }
}

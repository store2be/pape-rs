use hyper;
use hyper::server::Response;
use hyper::{StatusCode};
use tera;

pub enum Error {
    Hyper(hyper::Error),
    Tera(tera::Error),
    UnprocessableEntity,
    UriError(hyper::error::UriError),
}

impl Error {
    pub fn into_response(self) -> Response {
        match self {
            Error::UnprocessableEntity => Response::new().with_status(StatusCode::UnprocessableEntity),
            Error::Tera(_) | Error::UriError(_) | Error::Hyper(_) => {
                Response::new().with_status(StatusCode::InternalServerError)
            },
        }
    }
}

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Error {
        Error::Hyper(err)
    }
}

impl From<hyper::error::UriError> for Error {
    fn from(err: hyper::error::UriError) -> Error {
        Error::UriError(err)
    }
}

impl From<::std::string::FromUtf8Error> for Error {
    fn from(err: ::std::string::FromUtf8Error) -> Error {
        Error::UnprocessableEntity
    }
}

impl From<tera::Error> for Error {
    fn from(err: tera::Error) -> Error {
        Error::Tera(err)
    }
}


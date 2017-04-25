use hyper;
use hyper::server::Response;
use hyper::{StatusCode};
use tera;

pub enum Error {
    Hyper(hyper::Error),
    Tera(tera::Error),
    InternalServerError,
    UriError(hyper::error::UriError),
    UnprocessableEntity,
    LatexFailed,
}

impl Error {
    pub fn into_response(self) -> Response {
        match self {
            Error::UnprocessableEntity => Response::new().with_status(StatusCode::UnprocessableEntity),
            Error::InternalServerError => Response::new().with_status(StatusCode::InternalServerError),
            Error::Tera(_) | Error::UriError(_) | Error::Hyper(_) => {
                Response::new().with_status(StatusCode::InternalServerError)
            },
            _ => unreachable!(),
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
    fn from(_: ::std::string::FromUtf8Error) -> Error {
        Error::UnprocessableEntity
    }
}

impl From<tera::Error> for Error {
    fn from(err: tera::Error) -> Error {
        Error::Tera(err)
    }
}

impl From<::std::io::Error> for Error {
    fn from(_: ::std::io::Error) -> Error {
        Error::UnprocessableEntity
    }
}

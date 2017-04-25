use hyper;
use hyper::server::Response;
use hyper::{StatusCode};
use tera;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
    }


    foreign_links {
        Hyper(hyper::Error);
        Io(::std::io::Error);
        Tera(tera::Error);
        UriError(hyper::error::UriError);
        FromUtf8Error(::std::string::FromUtf8Error);
    }

    errors {
        LatexFailed {
            description("The latex command failed")
            display("The latex command failed")
        }
        InternalServerError {
            description("Internal server error")
            display("Internal server error")
        }
        UnprocessableEntity {
            description("Unprocessable entity")
            display("Unprocessable entity")
        }
    }
}

impl Error {
    pub fn into_response(self) -> Response {
        match self {
            Error(ErrorKind::UnprocessableEntity, _) => Response::new().with_status(StatusCode::UnprocessableEntity),
            Error(ErrorKind::InternalServerError, _) => Response::new().with_status(StatusCode::InternalServerError),
            Error(ErrorKind::Tera(_), _)
            | Error(ErrorKind::UriError(_), _)
            | Error(ErrorKind::Hyper(_), _) => {
                Response::new().with_status(StatusCode::InternalServerError)
            },
            _ => unreachable!(),
        }
    }
}

use hyper;
use hyper::server::Response;
use hyper::StatusCode;
use multipart;
use tera;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
    }


    foreign_links {
        FromUtf8Error(::std::string::FromUtf8Error);
        Hyper(hyper::Error);
        Io(::std::io::Error);
        MultipartError(multipart::Error);
        Tera(tera::Error);
        UriError(hyper::error::UriError);
    }

    errors {
        Forbidden {
            description("Forbidden")
            display("Forbidden")
        }
        LatexFailed(output: String) {
            description("The latex command failed")
            display("The latex command failed with the following output:\n{}", output)
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
            Error(ErrorKind::UnprocessableEntity, _) => {
                Response::new().with_status(StatusCode::UnprocessableEntity)
            }
            Error(ErrorKind::Forbidden, _) => Response::new().with_status(StatusCode::Forbidden),
            Error(ErrorKind::InternalServerError, _) |
            Error(ErrorKind::Tera(_), _) |
            Error(ErrorKind::UriError(_), _) |
            Error(ErrorKind::Hyper(_), _) => {
                Response::new().with_status(StatusCode::InternalServerError)
            }
            _ => unreachable!(),
        }
    }
}

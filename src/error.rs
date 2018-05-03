use hyper;
use hyper::server::Response;
use hyper::StatusCode;
use s3;
use serde_json;
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
        S3Get(s3::GetObjectError);
        S3Put(s3::PutObjectError);
        Json(serde_json::Error);
        UriError(hyper::error::UriError);
        FromUtf8Error(::std::string::FromUtf8Error);
    }

    errors {
        Forbidden {
            description("Forbidden")
            display("Forbidden.")
        }
        InternalServerError {
            description("Internal server error")
            display("Internal server error.")
        }
        LatexFailed(output: String) {
            description("The latex command failed")
            display("The latex command failed with the following output:\n{}", output)
        }
        MergeFailed(output: String) {
            description("A document merge failed")
            display("The provided documents could not be merged, output:\n{}", output)
        }
        UnprocessableEntity(reason: String) {
            description("Unprocessable entity")
            display("Unprocessable entity:\n{}", reason)
        }
    }
}

impl Error {
    pub fn into_response(self) -> Response {
        match self {
            Error(ErrorKind::UnprocessableEntity(_), _) => {
                Response::new().with_status(StatusCode::UnprocessableEntity)
            }
            Error(ErrorKind::Forbidden, _) => Response::new().with_status(StatusCode::Forbidden),
            Error(ErrorKind::InternalServerError, _)
            | Error(ErrorKind::Tera(_), _)
            | Error(ErrorKind::UriError(_), _)
            | Error(ErrorKind::Hyper(_), _) => {
                Response::new().with_status(StatusCode::InternalServerError)
            }
            _ => unreachable!(),
        }
    }
}

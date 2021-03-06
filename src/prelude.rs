//! Common imports used throughout the App. We should strive to keep this minimal.

pub(crate) use crate::config::Config;
pub(crate) use crate::papers::{DocumentSpec, MergeSpec, PapersUri};
pub(crate) use crate::utils::http::ReqwestResponseExt as _;
use failure::Fail;
pub use failure::{format_err, ResultExt as _};
pub use std::sync::Arc;

pub type Response = http::Response<hyper::Body>;

/// An error that can bubble up into our endpoints, and be translated into an error response.
#[derive(Debug, Fail)]
pub enum EndpointError {
    #[fail(display = "Forbidden (403)")]
    Forbidden {
        #[fail(cause)]
        cause: failure::Error,
    },
    #[fail(display = "Internal Server Error (500)")]
    InternalServerError {
        #[fail(cause)]
        cause: failure::Error,
    },
    #[fail(display = "Unprocessable Entity (422)")]
    UnprocessableEntity {
        #[fail(cause)]
        cause: failure::Error,
    },
}

impl From<serde_json::error::Error> for EndpointError {
    fn from(err: serde_json::error::Error) -> Self {
        EndpointError::UnprocessableEntity { cause: err.into() }
    }
}

impl From<std::io::Error> for EndpointError {
    fn from(err: std::io::Error) -> Self {
        EndpointError::InternalServerError { cause: err.into() }
    }
}

impl From<failure::Error> for EndpointError {
    fn from(err: failure::Error) -> Self {
        EndpointError::InternalServerError { cause: err }
    }
}

/// Create an empty response with the provided body. Status code is 200 by default, and there is no
/// Content-Type.
pub(crate) fn empty_response() -> Response {
    http::Response::new(hyper::Body::empty())
}

/// Create a JSON response with the provided body. Status code is 200 by default.
pub(crate) fn json_response<T: serde::Serialize>(body: &T) -> Result<Response, failure::Error> {
    let body = serde_json::to_vec(body)?;
    let mut response = http::Response::new(body.into());
    let mut headers = hyperx::Headers::with_capacity(1);
    headers.set(hyperx::header::ContentType::json());
    *response.headers_mut() = headers.into();
    Ok(response)
}

impl EndpointError {
    pub(crate) fn into_rejection(self) -> warp::Rejection {
        warp::reject::custom(self.compat())
    }

    pub(crate) fn to_response(&self) -> Response {
        use serde_json::json;

        match self {
            EndpointError::Forbidden { cause } => {
                let body = json!({
                    "message": display_error(&cause),
                });
                let mut response = json_response(&body).expect("serialization error");
                *response.status_mut() = http::StatusCode::FORBIDDEN;
                response
            }
            EndpointError::InternalServerError { .. } => {
                let mut response = empty_response();
                *response.status_mut() = http::StatusCode::INTERNAL_SERVER_ERROR;
                response
            }
            EndpointError::UnprocessableEntity { cause } => {
                let body = json!({
                    "message": display_error(&cause),
                });
                let mut response = json_response(&body).expect("serialization error");
                *response.status_mut() = http::StatusCode::UNPROCESSABLE_ENTITY;
                response
            }
        }
    }
}

/// Display an error and its causes in a readable way. This is meant for user-facing errors, use
/// the [`Debug`](std::fmt::Debug) impl for logging/sentry..
pub(crate) fn display_error(error: &failure::Error) -> String {
    use std::fmt::Write;

    let mut msg = format!("{}\n", error);

    for cause in error.iter_causes() {
        writeln!(msg, "Caused by: {}", cause).expect("formatting error");
    }

    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use failure::format_err;

    #[test]
    fn display_error_works() {
        let err = format_err!("something went wrong");

        assert_eq!(display_error(&err), "something went wrong\n");
    }

    const ERROR_WITH_CAUSES: &str = "Unprocessable Entity (422)
Caused by: error doing things
Caused by: aw, snap
";

    #[test]
    fn display_error_shows_causes() {
        let complex_err = EndpointError::UnprocessableEntity {
            cause: format_err!("aw, snap").context("error doing things").into(),
        };

        assert_eq!(display_error(&complex_err.into()), ERROR_WITH_CAUSES);
    }
}

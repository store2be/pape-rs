use crate::prelude::*;
use warp::{
    filters::{header::header, BoxedFilter},
    Filter,
};

/// Reject with a 403.
fn reject_forbidden() -> warp::Rejection {
    EndpointError::Forbidden {
        cause: format_err!("Missing or invalid Authorization header"),
    }
    .into_rejection()
}

/// Extract the token from a "Bearer <token>" formatted authorization header.
fn extract_bearer(header: &str) -> Option<&str> {
    let bearer_re = regex::Regex::new(r"^[Bb]earer (.*)$").unwrap();
    bearer_re
        .captures(header)
        .and_then(|captures| captures.get(1))
        .map(|capture| capture.as_str())
}

/// A filter that rejects unauthorized requests based on `secret`.
pub(crate) fn auth_filter(secret: String) -> BoxedFilter<()> {
    header("Authorization")
        .and_then(move |auth_header: String| {
            let token: Option<&str> = extract_bearer(&auth_header);

            match token {
                Some(token) if token == secret => Ok(()),
                _ => Err(reject_forbidden()),
            }
        })
        .untuple_one()
        .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_token_extraction() {
        assert_eq!("my-secret", extract_bearer("Bearer my-secret").unwrap());
        assert_eq!("my-secret", extract_bearer("bearer my-secret").unwrap());
    }
}

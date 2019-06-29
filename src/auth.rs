use crate::prelude::*;
use futures::FutureExt;
use tide::middleware::{Middleware, Next};

/// Check the Authorization header against a secret.
pub struct AuthMiddleware {
    secret: String,
    bearer_re: regex::Regex,
}

/// Create a 403 response.
fn forbidden() -> Response {
    let mut response = empty_response();
    *response.status_mut() = http::StatusCode::FORBIDDEN;
    response
}

impl AuthMiddleware {
    pub fn new(secret: String) -> AuthMiddleware {
        AuthMiddleware {
            secret,
            bearer_re: regex::Regex::new(r"^[Bb]earer (.*)$").unwrap(),
        }
    }

    async fn async_handle<'a>(&'a self, context: Context, next: Next<'a, AppState>) -> Response {
        let auth_header = context.headers().get(http::header::AUTHORIZATION);
        let token: Option<&str> = auth_header
            .and_then(|header| header.to_str().ok())
            .and_then(|header| self.extract_bearer(header));

        match token {
            Some(token) if token == self.secret => next.run(context).await,
            _ => forbidden(),
        }
    }

    fn extract_bearer<'a>(&self, value: &'a str) -> Option<&'a str> {
        self.bearer_re
            .captures(value)
            .and_then(|captures| captures.get(1))
            .map(|capture| capture.as_str())
    }
}

impl Middleware<AppState> for AuthMiddleware {
    fn handle<'a>(
        &'a self,
        context: Context,
        next: Next<'a, AppState>,
    ) -> futures::future::BoxFuture<'a, Response> {
        self.async_handle(context, next).boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_token_extraction() {
        let middleware = AuthMiddleware::new("irrelevant".to_owned());
        assert_eq!(
            "my-secret",
            middleware.extract_bearer("Bearer my-secret").unwrap()
        );
        assert_eq!(
            "my-secret",
            middleware.extract_bearer("bearer my-secret").unwrap()
        );
    }
}

use crate::auth::AuthMiddleware;
use crate::endpoints;
use crate::prelude::*;

/// Create a [tide::App](tide::App) based on the provided configuration.
pub fn app(config: Arc<Config>) -> tide::App<AppState> {
    let auth_middleware = config.auth.clone().map(AuthMiddleware::new);
    let app_state = AppState::new(config);
    let mut app = tide::App::new(app_state);

    // Authentication if enabled.
    if let Some(middleware) = auth_middleware {
        app.middleware(middleware);
    }

    app.at("/healthz").head(endpoints::health_check);
    app.at("/healthz").get(endpoints::health_check);
    app.at("/merge").post(endpoints::merge);
    app.at("/submit").post(endpoints::submit);
    app.at("/preview").post(endpoints::preview);

    app
}

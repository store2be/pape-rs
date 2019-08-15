use crate::auth::auth_filter;
use crate::endpoints;
use crate::prelude::*;
use futures::{FutureExt, TryFutureExt};
use warp::{
    filters::{
        body::json,
        method::{get2, head, post2},
        path::{end, path},
        BoxedFilter,
    },
    Filter,
};

fn config_filter(config: Arc<Config>) -> impl Fn() -> BoxedFilter<(Arc<Config>,)> {
    move || {
        let config = Arc::clone(&config);
        warp::any().map(move || config.clone()).boxed()
    }
}

/// Create a [warp BoxedFilter](warp::filters::BoxedFilter) based on the provided configuration.
pub fn app(config: Arc<Config>) -> BoxedFilter<(impl warp::Reply,)> {
    // Authentication if enabled.
    let auth_filter = config.auth.clone().map(auth_filter);
    let with_config = config_filter(config);

    let base = if let Some(filter) = auth_filter {
        warp::any().and(filter).boxed()
    } else {
        warp::any().boxed()
    };

    // GET /healthz
    // HEAD /healthz
    let healthz = path("healthz")
        .and(end())
        .and(head().or(get2()).unify())
        .map(|| "OK");

    // POST /merge
    let merge = path("merge")
        .and(end())
        .and(post2())
        .and(json())
        .and(with_config())
        .and_then(|merge_spec, config| {
            endpoints::merge(merge_spec, config)
                .map_err(EndpointError::into_rejection)
                .boxed()
                .compat()
        });

    // POST /submit
    let submit = path("submit")
        .and(end())
        .and(post2())
        .and(json())
        .and(with_config())
        .and_then(|document_spec, config| {
            endpoints::submit(document_spec, config)
                .map_err(EndpointError::into_rejection)
                .boxed()
                .compat()
        });

    // POST /preview
    let preview = path("preview")
        .and(end())
        .and(post2())
        .and(json())
        .and(with_config())
        .and_then(|document_spec, config| {
            endpoints::preview(document_spec, config)
                .map_err(EndpointError::into_rejection)
                .boxed()
                .compat()
        });

    let routes = healthz.or(merge).or(submit).or(preview);

    base.and(routes).recover(recover).boxed()
}

fn recover(rejection: warp::Rejection) -> Result<Response, warp::Rejection> {
    use failure::Compat;

    if let Some(endpoint_err) = rejection.find_cause::<Compat<EndpointError>>() {
        return Ok(endpoint_err.get_ref().to_response());
    }

    Err(rejection)
}

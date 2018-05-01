use futures::future;
use futures::Future;
use hyper;
use hyper::{Request, Response, Uri};
use hyper::server::Service;
use serde_json;
use slog::Logger;
use error_chain::ChainedError;
use mime;
extern crate sentry;

use http::*;
use papers::Summary;
use error::Error;
use config::Config;

/// This reports to the provided callback url with the presigned URL of the generated PDF and the
/// location of the debugging output. It returns the response from the callback url as a future.
pub fn report_success<S>(
    config: &'static Config,
    logger: &Logger,
    client: S,
    callback_url: Uri,
    s3_prefix: String,
    presigned_url: String,
) -> Box<Future<Item = (), Error = Error>>
where
    S: Service<Request = Request, Response = Response, Error = hyper::Error> + 'static + Clone,
{
    let outcome = Summary::File {
        file: presigned_url,
        s3_folder: s3_prefix,
    };

    debug!(logger, "Summary sent to callback: {:?}", outcome);

    let callback_response = future::result(serde_json::to_vec(&outcome))
        .map_err(|err| {
            Error::with_chain(err, "Error encoding the rendering outcome")
        })
        .and_then(move |body| {
            let req = Request::new(hyper::Method::Post, callback_url)
                .with_body(body.into())
                .with_header(hyper::header::ContentType(mime::APPLICATION_JSON));

            client.call(req).map_err(|err| {
                Error::with_chain(err, "Error posting to callback URL")
            })
        });

    let response_bytes = {
        let logger = logger.clone();
        let max_asset_size = config.max_asset_size;
        callback_response.and_then(move |response| {
            info!(
                logger,
                "Callback response: {}",
                response.status().canonical_reason().unwrap_or("unknown")
            );

            response.get_body_bytes_with_limit(max_asset_size)
        })
    };

    Box::new({
        let logger = logger.clone();
        response_bytes.and_then(move |bytes| {
            debug!(
                logger,
                "Callback response body: {:?}",
                ::std::str::from_utf8(&bytes).unwrap_or("<binary content>")
            );
            future::ok(())
        })
    })
}

/// When an error occurs during the generation process, it is reported with this function. It calls
/// the `callback_url` from the document spec, posting a `Summary` object with the error and the
/// key where the debug output can be found.
pub fn report_failure<S>(
    logger: &Logger,
    client: S,
    error: &Error,
    s3_prefix: String,
    callback_url: Uri,
) -> Box<Future<Item = (), Error = Error>>
where
    S: Service<Request = Request, Response = Response, Error = hyper::Error> + 'static,
{
    error!(logger, "Reporting error: {}", error.display_chain());
    sentry::capture_message(&error.display_chain().to_string(), sentry::Level::Error);

    let outcome = Summary::Error {
        error: format!("{}", error.display_chain()),
        s3_folder: s3_prefix,
    };
    debug!(logger, "Summary sent to callback: {:?}", outcome);
    let res = future::result(serde_json::to_vec(&outcome))
        .map_err(Error::from)
        .and_then(move |body| {
            let req = Request::new(hyper::Method::Post, callback_url)
                .with_body(body.into())
                .with_header(hyper::header::ContentType(mime::APPLICATION_JSON));
            client.call(req).map_err(Error::from)
        })
        .map(|_| ());
    Box::new(res)
}

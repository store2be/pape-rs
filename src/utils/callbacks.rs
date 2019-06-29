use crate::papers::Summary;
use crate::prelude::*;
use futures::compat::*;
use reqwest::r#async::Client;
use sentry;
use slog::{debug, error, info, Logger};

/// This reports to the provided callback url with the presigned URL of the generated PDF and the
/// location of the debugging output. It returns the response from the callback url as a future.
pub async fn report_success<'a>(
    logger: Logger,
    callback_url: &'a str,
    s3_prefix: String,
    presigned_url: String,
) -> Result<(), failure::Error> {
    let client = Client::new();
    let outcome = Summary::File {
        file: presigned_url,
        s3_folder: s3_prefix,
    };

    debug!(logger, "Summary sent to callback: {:?}", outcome);

    let callback_response = client
        .post(&callback_url.to_string())
        .json(&outcome)
        .send()
        .compat()
        .await
        .context("Error posting to callback URL")?;

    info!(
        logger,
        "Callback response: {}.",
        callback_response
            .status()
            .canonical_reason()
            .unwrap_or("unknown")
    );

    debug!(
        logger,
        "Callback response body: {:?}.",
        callback_response.body(),
    );
    Ok(())
}

/// When an error occurs during the generation process, it is reported with this function. It calls
/// the `callback_url` from the document spec, posting a `Summary` object with the error and the
/// key where the debug output can be found.
pub async fn report_failure<'a>(
    logger: Logger,
    error: failure::Error,
    s3_prefix: String,
    callback_url: &'a str,
) -> Result<(), failure::Error> {
    let client = Client::new();

    // For the logs and sentry, we want the most detailed (but less readable) version of the
    // error, so we use the [`Debug`](std::fmt::Debug) implementation.
    error!(logger, "Error to be reported to the callback URL: {:?}.", &error);
    sentry::capture_message(&format!("{:?}", &error), sentry::Level::Error);

    // For the callback, we want the user-facing version of the error.
    let err_msg = display_error(&error);

    let outcome = Summary::Error {
        backtrace: error.backtrace().to_string(),
        error: err_msg,
        s3_folder: s3_prefix,
    };

    debug!(logger, "Summary sent to callback: {:?}.", outcome);

    client
        .post(callback_url)
        .json(&outcome)
        .send()
        .compat()
        .await?;

    Ok(())
}

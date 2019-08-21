use crate::prelude::*;
use crate::utils::http::{client_response_body_to_file, extract_filename_from_uri};
use futures::compat::*;
use slog::{debug, Logger};

/// A wrapper around a temporary directory where we download and manipulate files.
pub struct Workspace {
    /// The HTTP client for the workspace.
    client: reqwest::r#async::Client,
    /// The app config.
    config: Arc<Config>,
    /// The temporary directory for the task.
    ///
    /// Since [`mktemp::Temp`](mktemp::Temp) implements [`Drop`](Drop) by deleting the
    /// directory, we don't need to worry about leaving files or directories behind.
    temp_dir: mktemp::Temp,
    /// The local logger for the task. This may contain context for more useful logging. The
    /// Workspace will additionally log to a file in the temporary directory.
    logger: Logger,
    /// The directory we will upload to inside the destination S3 bucket.
    s3_dir_name: String,
}

impl Workspace {
    /// Construct a `Workspace`. The `base_logger` will be used as a base to construct the file +
    /// stderr logger used in the `Workspace`.
    pub fn new(base_logger: Logger, config: Arc<Config>) -> Result<Self, failure::Error> {
        let temp_dir = mktemp::Temp::new_dir().context("Could not create a temporary directory")?;
        let logger = crate::utils::logging::file_logger(base_logger, &temp_dir.to_path_buf());
        Ok(Workspace {
            config: config.clone(),
            client: reqwest::r#async::Client::new(),
            logger,
            temp_dir,
            s3_dir_name: crate::utils::s3::s3_dir_name(),
        })
    }

    /// Log in the context of the workspace. The workspace logger should always be used when
    /// working in the context of a workspace, so the logs in the right sink, and with the
    /// right context.
    pub fn logger(&self) -> Logger {
        self.logger.clone()
    }

    /// The path to the workspace's temporary directory.
    pub fn temp_dir_path(&self) -> &std::path::Path {
        self.temp_dir.as_ref()
    }

    /// The name of the file will be deduced from the `Content-Disposition` header of the
    /// response or the last segment of `url`.
    pub async fn download_file<'a>(
        &'a self,
        url: &'a hyper::Uri,
    ) -> Result<std::path::PathBuf, failure::Error> {
        self.download_file_impl(url, None).await
    }

    /// Download the file with the `url` and prefix its name with `prefix`.
    pub async fn download_file_with_prefix<'a>(
        &'a self,
        url: &'a hyper::Uri,
        prefix: String,
    ) -> Result<std::path::PathBuf, failure::Error> {
        self.download_file_impl(url, Some(prefix)).await
    }

    /// Shared implementation for `download_file` and `download_file_with_prefix`.
    async fn download_file_impl<'a>(
        &'a self,
        uri: &'a hyper::Uri,
        prefix: Option<String>,
    ) -> Result<std::path::PathBuf, failure::Error> {
        let url = uri.to_string();

        let response = self.client.get(&url).send().compat().await?;

        let filename: String = response
            .filename()
            .or_else(|| extract_filename_from_uri(uri).map(|s| s.to_owned()))
            .ok_or_else(|| format_err!("Could not produce filename for {}", uri))?;

        let filename = if let Some(prefix) = prefix {
            format!("{}-{}", prefix, filename)
        } else {
            filename
        };

        let dest_path = self.temp_dir_path().join(filename);

        debug!(self.logger, "Writing file {:?} as {:?}.", &uri, &dest_path);

        client_response_body_to_file(response, dest_path.clone(), self.config.max_asset_size)
            .await
            .context("Error downloading asset")?;

        Ok(dest_path)
    }

    pub async fn report_success<'a>(
        &'a self,
        presigned_url: String,
        callback_url: &'a str,
    ) -> Result<(), failure::Error> {
        crate::utils::callbacks::report_success(
            self.logger(),
            callback_url,
            self.s3_dir_name.clone(),
            presigned_url,
        )
        .await
    }

    /// Report errors to the callback URL.
    pub async fn report_failure(
        &self,
        error: failure::Error,
        callback_url: String,
    ) -> Result<(), failure::Error> {
        crate::utils::callbacks::report_failure(
            self.logger(),
            error,
            self.s3_dir_name.to_owned(),
            &callback_url,
        )
        .await
    }

    /// Returns a presigned URL to the uploaded file.
    pub async fn upload_to_s3(
        &self,
        file_path: std::path::PathBuf,
    ) -> Result<String, failure::Error> {
        let filename = file_path.file_name().ok_or_else(|| {
            format_err!("missing filename in \"{}\"", file_path.to_string_lossy())
        })?;
        let key = format!("{}/{}", &self.s3_dir_name, filename.to_string_lossy());
        crate::utils::s3::upload_document(&self.config, self.logger(), file_path, key).await
    }

    /// Upload the whole workspace directory to the S3 directory as `workspace.tar`.
    pub async fn upload_workspace(&self) -> Result<(), failure::Error> {
        let workspace_tar_key = format!("{}/{}", &self.s3_dir_name, "workspace.tar");

        crate::utils::s3::upload_workspace(
            &self.config,
            self.logger(),
            self.temp_dir_path(),
            workspace_tar_key,
        )
        .await
    }
}

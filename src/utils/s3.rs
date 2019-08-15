use chrono::Utc;
use futures::compat::*;
use rusoto_s3::S3;
use slog::{debug, Logger};
use std::default::Default;
use std::path::PathBuf;
use tokio::{fs::File, io::AsyncWrite};

use crate::prelude::*;

/// Generate a unique s3 bucket directory name. This currently returns a simple timestamp.
pub fn s3_dir_name() -> String {
    format!("{}", Utc::now())
}

/// Posts the file to the given key in S3 with the default S3 configuration.
pub async fn post_to_s3(
    config: &Config,
    path: PathBuf,
    key: String,
) -> Result<(), failure::Error> {
    use futures::compat::*;

    let client = rusoto_s3::S3Client::new(config.s3.region.clone());

    debug!(config.logger, "Uploading {:?} to {:?}.", path, key);
    let file = File::open(path).compat().await?;
    let bytes = Vec::new();
    let (_, bytes) = tokio::io::read_to_end(file, bytes).compat().await?;

    let request = rusoto_s3::PutObjectRequest {
        body: Some(bytes.into()),
        bucket: config.s3.bucket.clone(),
        key,
        ..Default::default()
    };

    client
        .put_object(request)
        .compat()
        .await
        .context("Error during S3 upload")?;

    Ok(())
}

/// Gets a presigned url for the specified key in the bucket specified in the configuration.
///
/// This does not perform any request.
pub fn get_presigned_url(config: &Config, key: String) -> String {
    use rusoto_s3::util::*;
    use rusoto_s3::GetObjectRequest;

    let request = GetObjectRequest {
        bucket: config.s3.bucket.clone(),
        key,
        response_expires: Some(format!("{}", config.s3.expiration_time)),
        ..Default::default()
    };

    let options = rusoto_s3::util::PreSignedRequestOption {
        expires_in: std::time::Duration::from_secs(3600 * 24),
    };

    request.get_presigned_url(&config.s3.region, &config.s3.credentials, &options)
}

/// This function is responsible for uploading a tar file with the contents from the workspace (the
/// temporary directory where we generated the PDF) to S3 under the given key.
pub async fn upload_workspace<'a>(
    config: &'a Config,
    logger: Logger,
    workspace: &'a std::path::Path,
    key: String,
) -> Result<(), failure::Error> {
    debug!(logger, "Tarring {:?}.", workspace);
    let mut tarred_workspace: Vec<u8> = Vec::new();

    let dir_name: PathBuf = workspace
        .components()
        .last()
        .unwrap()
        .as_os_str()
        .into();
    debug!(logger, "Tarring {:?} as {:?}.", &workspace, &dir_name);

    {
        let mut tarrer = tar::Builder::new(&mut tarred_workspace);
        tarrer.append_dir_all(&dir_name, &workspace)?;
        debug!(logger, "Tar was successful.");
        tarrer.finish()?;
    }

    // Write the tarred workspace to disk
    let mut tar_file_path = workspace.to_path_buf();
    tar_file_path.push("workspace.tar");

    let mut output_file = File::create(tar_file_path.clone()).compat().await?;

    futures01::future::poll_fn(|| output_file.poll_write(&tarred_workspace))
        .compat()
        .await?;

    // Upload the tarred workspace to S3
    post_to_s3(config, tar_file_path, key).await
}

/// Takes the path to a generated pdf and a key, returns the presigned url to the uploaded document
pub async fn upload_document(
    config: &Config,
    logger: Logger,
    local_path: PathBuf,
    key: String,
) -> Result<String, failure::Error> {
    debug!(logger, "Uploading to {:?} / {:?}.", config.s3.bucket, key);
    post_to_s3(config, local_path, key.clone()).await?;
    Ok(get_presigned_url(config, key))
}

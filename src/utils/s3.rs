use tar;
use futures_cpupool::CpuPool;
use futures::Future;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use chrono::Utc;
use rusoto;
use s3;
use s3::S3;
use slog::Logger;
use std::default::Default;
use std::fs::File;

use error::Error;
use config::Config;

pub fn s3_dir_name() -> String {
    format!("{}", Utc::now())
}

/// Posts the file to the given key in S3 with the default S3 configuration.
pub fn post_to_s3(config: &'static Config, path: &Path, key: String) -> Result<(), Error> {
    let client = s3::S3Client::new(
        rusoto::request::default_tls_client().expect("could not create TLS client"),
        config,
        config.s3.region,
    );
    debug!(config.logger, "Uploading {:?} to {:?}", path, key);
    let mut body: Vec<u8> = Vec::new();
    let mut file = File::open(path)?;
    file.read_to_end(&mut body)?;
    let request = s3::PutObjectRequest {
        body: Some(body),
        bucket: config.s3.bucket.clone(),
        key,
        ..Default::default()
    };

    client
        .put_object(&request)
        .map(|_| ())
        .map_err(|err| Error::with_chain(err, "Error during S3 upload"))
}

/// Gets a presigned url for the specified key in the bucket specified in the configuration.
///
/// This does not perform any request.
pub fn get_presigned_url(config: &'static Config, key: String) -> Result<String, Error> {
    let client = s3::S3Client::new(
        rusoto::request::default_tls_client().expect("could not create TLS client"),
        config,
        config.s3.region,
    );
    let request = s3::GetObjectRequest {
        bucket: config.s3.bucket.clone(),
        key,
        response_expires: Some(format!("{}", config.s3.expiration_time)),
        ..Default::default()
    };
    client.presigned_url(&request).map_err(|err| {
        Error::with_chain(err, "Could not generate presigned url")
    })
}

/// This function is responsible for uploading a tar file with the contents from the workspace (the
/// temporary directory where we generated the PDF) to S3 under the given key.
pub fn upload_workspace(
    config: &'static Config,
    logger: Logger,
    workspace: PathBuf,
    key: String,
) -> Result<(), Error> {
    debug!(logger, "Tarring {:?}", workspace);
    let mut tarred_workspace: Vec<u8> = Vec::new();
    {
        let dir_name: PathBuf = workspace
            .clone()
            .components()
            .last()
            .unwrap()
            .as_os_str()
            .into();
        debug!(logger, "tar {:?} as {:?}", &workspace, &dir_name);
        let mut tarrer = tar::Builder::new(&mut tarred_workspace);
        tarrer.append_dir_all(&dir_name, &workspace)?;
        debug!(logger, "Tar was successful");
        tarrer.finish()?;
    }

    // Write the tarred workspace to disk
    let mut tar_file_path = workspace.to_path_buf();
    tar_file_path.push("workspace.tar");
    let mut output_file = File::create(&tar_file_path)?;
    output_file.write_all(&tarred_workspace)?;

    // Upload the tarred workspace to S3
    post_to_s3(config, &tar_file_path, key)
}

/// Takes the path to a generated pdf and a key, returns the presigned url to the uploaded document
pub fn upload_document(
    config: &'static Config,
    logger: Logger,
    pool: CpuPool,
    local_path: PathBuf,
    key: String,
) -> Box<Future<Item = String, Error = Error>> {
    Box::new(pool.spawn_fn(move || {
        debug!(logger, "Uploading to {:?} / {:?}", config.s3.bucket, key);
        post_to_s3(config, &local_path, key.clone())?;
        get_presigned_url(config, key)
    }))
}

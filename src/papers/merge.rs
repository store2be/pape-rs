use mktemp::Temp;

use futures_cpupool::CpuPool;
use hyper::client::*;
use futures::*;
use std::process::Command;
use tokio_process::CommandExt;
use tokio_core::reactor::Handle;
use config::Config;
use error::{Error, ErrorKind};
use papers::MergeSpec;
use http::*;
use std::io::prelude::*;
use slog::Logger;
use std::path::*;
use utils::callbacks::*;
use utils::s3::*;

pub fn merge_documents(config: &'static Config, handle: &Handle, spec: MergeSpec) -> Box<Future<Item = (), Error = ()>> {
    let pool = CpuPool::new(3);
    let temp_dir = Temp::new_dir().expect("Could not create a temporary directory");
    let max_asset_size = config.max_asset_size.clone();
    let logger = config.logger.clone();
    let s3_prefix = s3_dir_name();
    debug!(logger, "Downloading PDFs for mergin: {:?}", &spec.assets_urls);

    let client = Client::configure()
        .connector(https_connector(handle))
        .build(handle);

    let assets_downloads = {
        let logger = logger.clone();
        let temp_dir = temp_dir.to_path_buf();
        let client = client.clone();
        spec.assets_urls
            .into_iter()
            .enumerate()
            .map(move |(index, uri)| {
                let mut path = temp_dir.to_path_buf();
                let logger = logger.clone();
                client.clone().get_follow_redirect(&uri.0)
                    .and_then(move |res| {
                        let filename = res.filename();
                        res.get_body_bytes_with_limit(max_asset_size)
                            .map(|bytes| (bytes, filename))
                    })
                .and_then(move |(bytes, filename)| {
                    let filename = filename
                        .or_else(|| extract_filename_from_uri(&uri.0))
                        .unwrap_or_else(|| format!("{}.pdf", index));
                    path.push(filename);
                    debug!(logger, "Writing file {:?} as {:?}", &uri, &path);
                    ::std::fs::File::create(&path)
                        .and_then(|mut file| file.write_all(&bytes))
                        .map(|_| path)
                        .map_err(|e| Error::with_chain(e, "Error writing PDF"))
                })
            })
    };

    let paths = future::join_all(assets_downloads);


    // Convert non-PDF files to PDF with imagemagick
    let converted_paths = {
        let handle = handle.clone();
        let logger = logger.clone();
        paths.and_then(move |paths| {
            let futures = paths.into_iter().map(move |path| {
                let logger = logger.clone();
                match path.extension() {
                    Some(extension) if extension == "pdf" => Box::new(future::ok(path.clone())),
                    None => Box::new(future::ok(path.clone())),
                    Some(_) => image_to_pdf(logger, &handle, &path.clone()),
                }
            });
            future::join_all(futures)
        })
    };

    let merged = {
        let output_filename = spec.output_filename.clone();
        let handle = handle.clone();
        let temp_dir = temp_dir.to_path_buf();
        converted_paths.and_then(move |paths| {
            Command::new("pdfunite")
                .current_dir(temp_dir)
                .args(paths)
                .arg(output_filename)
                .output_async(&handle)
                .map_err(|err| Error::with_chain(err, "Error merging PDFs"))
        })
    };

    let unwrapped = {
        let mut output_path = temp_dir.to_path_buf();
        output_path.push(&spec.output_filename);
        let logger = logger.clone();
        merged.and_then(move |output| {
            let stdout = String::from_utf8(output.stdout).expect("Output was not valid utf8");
            if output.status.success() {
                debug!(logger, "{}", stdout);
                Ok(output_path)
            } else {
                Err(ErrorKind::MergeFailed(stdout).into())
            }
        })
    };

    let presigned_url = {
        let logger = logger.clone();
        let s3_prefix = s3_prefix.clone();
        let pool = pool.clone();
        unwrapped.and_then(move |output_path| {
            let filename = output_path.file_name().unwrap().to_string_lossy().into_owned();
            let key = format!("{}/{}", &s3_prefix, filename);
            pool.spawn_fn(move || {
                debug!(
                    logger,
                    "Uploading the merged PDF as {:?} / {:?}",
                    config.s3.bucket,
                    key
                );
                post_to_s3(config, &output_path, key.clone())?;
                get_presigned_url(config, key)
            })
        })
    };

    let callback_response = {
        let callback_url = spec.callback_url.clone();
        let client = client.clone();
        let s3_prefix = s3_prefix.clone();
        let logger = logger.clone();
        presigned_url.and_then(move |presigned_url| {
            report_success(
                config,
                logger,
                client,
                callback_url.0,
                s3_prefix,
                presigned_url
            )
        })
    };

    // Report errors to the callback url
    let handle_errors = {
        let callback_url = spec.callback_url.clone();
        let logger = logger.clone();
        let client = client.clone();
        let s3_prefix = s3_prefix.clone();
        callback_response.or_else(move |error| {
            report_failure(logger, client, error, s3_prefix, callback_url.0)
        })
    };

    let tarred_workspace_uploaded = {
        let config = config.clone();
        let key = format!("{}/{}", &s3_prefix, "workspace.tar");
        let temp_dir_path = temp_dir.to_path_buf();
        let logger = logger.clone();
        handle_errors
            .then(move |_| {
                pool.spawn_fn(move || upload_workspace(config, logger, temp_dir_path, key))
            })
        .map_err(move |_| { let _hold = temp_dir; })
    };


    Box::new(tarred_workspace_uploaded.map(|_| ()).map_err(|_| ()))
}

/// Convert an image to an A4 pdf using imagemagick's `convert` command.
///
/// Sample command:
///
/// `convert sc.png -resize 1190x1684 -gravity center -background white -extent 1190x1684 sc.pdf`
fn image_to_pdf(logger: Logger, handle: &Handle, original_file_path: &Path) -> Box<Future<Item=PathBuf,Error=Error>> {
    let stem = original_file_path.file_stem().expect("Invalid path"); // "/tmp/something.jpeg" -> "something"
    let final_path = original_file_path.with_file_name(format!("{}.pdf", stem.to_string_lossy()));
    let work = Command::new("convert")
        .current_dir(&original_file_path.parent().expect("Invalid path"))
        .arg(original_file_path)
        .arg("-resize")
        .arg("595x842")
        .arg("-gravity")
        .arg("center")
        .arg("-background")
        .arg("white")
        .arg("-extent")
        .arg("595x842")
        .arg("-page")
        .arg("A4")
        .arg(&final_path)
        .output_async(handle)
        .map_err(|err| Error::with_chain(err, "Error while converting image to pdf"))
        .and_then(move |output| {
            let stdout = String::from_utf8(output.stdout).unwrap();
            if output.status.success() {
                debug!(logger, "{}", stdout);
                Ok(final_path)
            } else {
                Err(ErrorKind::MergeFailed(stdout).into())
            }
        });

    Box::new(work)
}

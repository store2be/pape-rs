use crate::papers::{MergeSpec, Workspace};
use crate::prelude::*;
use futures::compat::*;
use futures::{StreamExt, TryFutureExt};
use slog::{debug, error, Logger};
use std::path::*;
use std::process::Command;
use tokio_process::CommandExt;

pub struct Merger {
    /// The blueprint for the merged document.
    merge_spec: MergeSpec,
    /// The path of the merged file.
    output_path: PathBuf,
    /// See the docs for [`Workspace`](crate::papers::Workspace).
    workspace: Workspace,
}

impl Merger {
    pub fn new(config: Arc<Config>, merge_spec: MergeSpec) -> Result<Self, failure::Error> {
        let logger = config.logger.clone();
        let workspace = Workspace::new(logger, config)?;

        let output_path = workspace.temp_dir_path().join(&merge_spec.output_filename);

        Ok(Merger {
            merge_spec,
            output_path,
            workspace,
        })
    }

    /// This function does the whole merging process from a
    /// [`MergeSpec`](crate::papers::MergeSpec).
    ///
    /// It:
    ///
    /// - Downloads the documents to merge
    /// - Converts those that are not PDFs to PDF
    /// - Merges the PDFs
    /// - Uploads the result to S3
    /// - Reports to the `callback_url` from the `MergeSpec` with the error or presigned url of
    /// the generated document.
    /// - Uploads the debugging output to S3 as a tar file.
    ///
    /// This method takes ownership because it is meant to be used to create futures to be
    /// spawned in the background.
    pub async fn merge_documents(self) -> Result<(), ()> {
        self.merge_documents_inner()
            .or_else(|err| self.report_failure(err))
            .await
            .ok();

        self.workspace
            .upload_workspace()
            .await
            .map_err(|err| {
                error!(
                    self.workspace.logger(),
                    "Error uploading workspace.tar: {:?}.", err
                )
            })
            .ok();

        Ok(())
    }

    async fn merge_documents_inner(&self) -> Result<(), failure::Error> {
        // Download
        let asset_paths = self
            .download_assets()
            .await
            .context("Error downloading assets.")?;

        // Convert
        let converted_paths = self
            .convert_assets_to_pdf(asset_paths)
            .await
            .context("Error converting asset file to PDF.")?;

        // Merge
        self.merge_pdf(converted_paths)
            .await
            .context("Error merging the PDFs.")?;

        // Upload the merged PDF
        let presigned_url = self
            .workspace
            .upload_to_s3(self.output_path.to_owned())
            .await?;

        // Report success
        let callback_url = self.merge_spec.callback_url();

        self.workspace
            .report_success(presigned_url, &callback_url)
            .await
    }

    /// Convert non-PDF files to PDF with imagemagick, and returns the path of the converted
    /// PDFs.
    async fn convert_assets_to_pdf(
        &self,
        asset_paths: Vec<PathBuf>,
    ) -> Result<Vec<PathBuf>, failure::Error> {
        let mut futures = futures::stream::FuturesUnordered::new();

        for path in asset_paths.into_iter() {
            let logger = self.workspace.logger().clone();
            let to_pdf = async move |path: PathBuf| -> Result<PathBuf, failure::Error> {
                match path.extension() {
                    Some(extension) if extension == "pdf" => return Ok(path),
                    None => Ok(path),
                    Some(_) => image_to_pdf(logger, path.clone()).await,
                }
            };
            futures.push(to_pdf(path));
        }

        let converted_paths: Vec<Result<_, _>> = futures.collect().await;
        converted_paths.into_iter().collect()
    }

    /// Download the assets to merge, and returns the paths to the downloaded files, preserving
    /// the order of the assets.
    ///
    /// It is NOT guaranteed that the files will have the same filenames as in the provided
    /// URLs.
    async fn download_assets(&self) -> Result<Vec<PathBuf>, failure::Error> {
        debug!(
            self.workspace.logger(),
            "Downloading PDFs for merging: {:?}.",
            &self.merge_spec.asset_urls().collect::<Vec<_>>()
        );

        let mut asset_downloads = Vec::new();

        for uri in self.merge_spec.asset_urls() {
            asset_downloads.push(
                self.workspace
                    .download_file_with_prefix(uri, uuid::Uuid::new_v4().to_string()),
            );
        }

        let paths: Vec<Result<PathBuf, _>> = futures::future::join_all(asset_downloads).await;
        paths.into_iter().collect()
    }

    async fn report_failure(&self, error: failure::Error) -> Result<(), ()> {
        error!(
            self.workspace.logger(),
            "Error merging documents: {:?}.", error
        );
        let callback_url = self.merge_spec.callback_url();
        match self.workspace.report_failure(error, callback_url).await {
            Ok(()) => (),
            Err(err) => error!(self.workspace.logger(), "Documents merge failed: {:?}.", err),
        }

        Ok(())
    }

    async fn merge_pdf(&self, converted_paths: Vec<PathBuf>) -> Result<(), failure::Error> {
        let output = Command::new("pdfunite")
            .current_dir(&self.workspace.temp_dir_path())
            .args(converted_paths)
            .arg(&self.merge_spec.output_filename)
            .output_async()
            .compat()
            .await
            .context("Error merging PDFs")?;

        let stdout_and_err = crate::utils::process::whole_output(&output).expect("output is utf8");

        if output.status.success() {
            debug!(
                self.workspace.logger(),
                "pdfunite output: {}.", stdout_and_err
            );
        } else {
            return Err(format_err!(
                "Merge failed. pdfunite output:\n{}",
                stdout_and_err
            ));
        };

        Ok(())
    }
}

/// Convert an image to an A4 pdf using imagemagick's `convert` command.
///
/// Sample command:
///
/// `convert sc.png -resize 1190x1684 -gravity center -background white -extent 1190x1684 sc.pdf`
async fn image_to_pdf(
    logger: Logger,
    original_file_path: PathBuf,
) -> Result<PathBuf, failure::Error> {
    // "/tmp/something.jpeg" -> "something"
    let stem = original_file_path.file_stem().expect("Invalid path");
    let final_path = original_file_path.with_file_name(format!("{}.pdf", stem.to_string_lossy()));
    let output = Command::new("convert")
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
        .arg("-density")
        .arg("72")
        .arg("-page")
        .arg("A4")
        .arg(&final_path)
        .output_async()
        .compat()
        .await
        .context("Error while converting image to pdf")?;

    let stdout_and_err =
        crate::utils::process::whole_output(&output).context("convert output was not utf8")?;

    if output.status.success() {
        debug!(logger, "ImageMagick output {}", stdout_and_err);
        Ok(final_path)
    } else {
        Err(format_err!("Merge failed. Output:\n{}", stdout_and_err))
    }
}

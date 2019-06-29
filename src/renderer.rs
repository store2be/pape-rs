use crate::papers::{DocumentSpec, Workspace};
use crate::prelude::*;
use futures::{compat::*, StreamExt};
use slog::{debug, error};
use std::process::Command;
use tokio_fs::File;
use tokio_io::AsyncWrite;
use tokio_process::CommandExt;

/// The name of the downloaded template inside our Tera instance.
const TEMPLATE_NAME: &'static str = "template";

pub struct Renderer {
    /// The manifest for the document to render.
    document_spec: DocumentSpec,
    /// The path to the rendered document.
    output_path: std::path::PathBuf,
    /// The path to the downloaded template.
    template_path: std::path::PathBuf,
    /// The templating engine.
    tera: tera::Tera,
    /// See the docs for [`Workspace`](crate::papers::Workspace).
    workspace: Workspace,
}

impl Renderer {
    pub fn new(config: Arc<Config>, document_spec: DocumentSpec) -> Result<Self, failure::Error> {
        let workspace = Workspace::new(config.logger.clone(), config)?;

        let output_path = workspace
            .temp_dir_path()
            .join(&document_spec.output_filename);

        let template_path = workspace
            .temp_dir_path()
            .join(document_spec.output_filename.replace("pdf", "tex"));

        Ok(Renderer {
            tera: crate::utils::templating::make_tera(),
            workspace,
            document_spec,
            output_path,
            template_path,
        })
    }

    pub async fn preview(&mut self) -> Result<String, failure::Error> {
        self.download_and_register_template().await?;

        self.tera
            .render(TEMPLATE_NAME, &self.document_spec.variables())
            .map_err(|err| format_err!("Rendering error: {}", err))
    }

    /// This function does the whole generation process from a
    /// [`DocumentSpec`](crate::papers::DocumentSpec).
    ///
    /// This method takes ownership because it is meant to be used to create futures to be
    /// spawned in the background.
    pub async fn render(mut self) -> Result<(), ()> {
        debug!(
            self.workspace.logger(),
            "Generating PDF with document spec: {:?}.", self.document_spec
        );

        match self.render_inner().await {
            // it worked, move on
            Ok(()) => (),
            // it failed -> report it
            Err(err) => {
                self.report_failure(err).await.ok();
            }
        }

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

    async fn render_inner(&mut self) -> Result<(), failure::Error> {
        // First download the template and populate it
        self.download_and_register_template().await?;
        self.render_template().await?;

        // Download the assets and save them in the temporary directory
        self.download_assets().await?;

        // Then run latex
        self.run_latex().await?;

        // Upload the resulting PDF and construct a presigned URL to it
        let presigned_url = self
            .workspace
            .upload_to_s3(self.output_path.to_owned())
            .await?;

        // Report to the callback URL
        let callback_url = self.document_spec.callback_url();
        let presigned_url = self
            .workspace
            .report_success(presigned_url, &callback_url)
            .await?;

        Ok(presigned_url)
    }

    fn template_path(&self) -> &std::path::Path {
        &self.template_path
    }

    /// Downloads all assets from the document spec in the workspace in parallel. It fails if any of
    /// those cannot be downloaded.
    async fn download_assets(&self) -> Result<Vec<std::path::PathBuf>, failure::Error> {
        debug!(
            &self.workspace.logger(),
            "Downloading assets: {:?}.",
            self.document_spec.asset_urls().collect::<Vec<_>>()
        );

        let mut futures = futures::stream::FuturesUnordered::new();

        for uri in self.document_spec.asset_urls() {
            let uri: hyper::Uri = uri.to_owned();
            // We need a closure to take ownership of `uri`.
            let download = async move || self.workspace.download_file(&uri).await;
            futures.push(download());
        }

        let futures: Vec<Result<_, _>> = futures.collect().await;
        futures.into_iter().collect()
    }

    /// Download and register the template in the Renderer's Tera instance.
    async fn download_and_register_template(&mut self) -> Result<(), failure::Error> {
        let file_path = self
            .workspace
            .download_file(&self.document_spec.template_url.0)
            .await?;

        self.tera
            .add_template_file(&file_path, Some("template"))
            .map_err(|err| format_err!("failed to add template: {:?}", err))?;

        debug!(
            self.workspace.logger(),
            "Successfully downloaded the template."
        );

        Ok(())
    }

    async fn render_template(&self) -> Result<(), failure::Error> {
        let rendered_template = self
            .tera
            .render(TEMPLATE_NAME, &self.document_spec.variables())
            .map_err(|err| format_err!("Rendering error: {}.", err))?;

        debug!(
            self.workspace.logger(),
            "Writing template to {:?}.",
            &self.template_path()
        );

        let mut file = File::create(self.template_path().to_owned())
            .compat()
            .await?;

        futures01::future::poll_fn(|| file.poll_write(rendered_template.as_bytes()))
            .compat()
            .await
            .context("Could not write latex file.")?;

        debug!(
            self.workspace.logger(),
            "Template successfully written to {:?}.",
            &self.template_path()
        );

        Ok(())
    }

    async fn run_latex(&self) -> Result<(), failure::Error> {
        debug!(
            &self.workspace.logger(),
            "Value of template_path: {:?}.",
            self.template_path()
        );
        debug!(
            &self.workspace.logger(),
            "Rendered template exists: {:?}.",
            self.template_path().exists()
        );

        debug!(&self.workspace.logger(), "Spawning latex.");
        let latex_out = Command::new("xelatex")
            .current_dir(&self.workspace.temp_dir_path())
            .arg("-interaction=nonstopmode")
            .arg("-file-line-error")
            .arg("-shell-restricted")
            .arg(self.template_path())
            .output_async()
            .compat()
            .await
            .context("Error generating PDF")?;

        let stdout = String::from_utf8(latex_out.stdout)?;

        if !latex_out.status.success() {
            return Err(format_err!("LaTeX failed. Stdout:\n{}", stdout));
        }

        debug!(&self.workspace.logger(), "LaTeX succeeded. Stdout:\n{}", stdout);

        Ok(())
    }

    /// Report failure and move on.
    async fn report_failure(&self, error: failure::Error) -> Result<(), ()> {
        error!(
            self.workspace.logger(),
            "Error rendering document: {:?}.", error,
        );

        let callback_url = self.document_spec.callback_url();

        match self.workspace.report_failure(error, callback_url).await {
            Ok(()) => (),
            Err(err) => error!(
                self.workspace.logger(),
                "Error reporting failure to callback_url: {:?}.", err
            ),
        }

        Ok(())
    }
}

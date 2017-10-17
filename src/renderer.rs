use futures::sync::oneshot;
use futures::future;
use futures::Future;
use futures_cpupool::CpuPool;
use hyper;
use hyper::{Request, Response, Uri};
use hyper::server::Service;
use mktemp::Temp;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio_core::reactor::Handle;
use tokio_process::CommandExt;
use tera::Tera;
use slog::Logger;
use utils::logging::file_logger;

use latex::escape_latex;
use http::*;
use papers::{DocumentSpec, PapersUri};
use error::{Error, ErrorKind};
use config::Config;
use utils::s3::*;
use utils::callbacks::*;

struct Context {
    assets_urls: Vec<PapersUri>,
    callback_url: PapersUri,
    config: &'static Config,
    logger: Logger,
    output_filename: String,
    pool: CpuPool,
    s3_prefix: String,
    tmp_dir: PathBuf,
}

#[derive(Debug)]
pub struct Renderer<S>
where
    S: Service<Request = Request, Response = Response, Error = hyper::Error> + Clone + 'static,
{
    config: &'static Config,
    handle: Handle,
    client: S,
}

impl<S> Renderer<S>
where
    S: Service<Request = Request, Response = Response, Error = hyper::Error> + Clone + 'static,
{
    fn get_template(
        &self,
        template_url: &Uri,
    ) -> Box<Future<Item = hyper::client::Response, Error = Error>> {
        self.client.clone().get_follow_redirect(template_url)
    }

    pub fn new(config: &'static Config, handle: &Handle, client: S) -> Self {
        Renderer {
            client,
            config,
            handle: handle.clone(),
        }
    }

    pub fn preview(
        &self,
        document_spec: DocumentSpec,
        sender: oneshot::Sender<Result<String, Error>>,
    ) -> Box<Future<Item = (), Error = ()>> {
        let DocumentSpec {
            variables,
            template_url,
            no_escape_latex,
            ..
        } = document_spec;
        let variables = if no_escape_latex { variables } else { escape_latex(variables) };
        let response = self.get_template(&template_url.0);
        let max_asset_size = self.config.max_asset_size;
        let bytes = response.and_then(move |res| res.get_body_bytes_with_limit(max_asset_size));
        let template_string = bytes.and_then(|bytes| {
            ::std::string::String::from_utf8(bytes).map_err(Error::from)
        });
        let rendered = template_string.and_then(move |template_string| {
            Tera::one_off(&template_string, &variables, false).map_err(Error::from)
        });
        let work = rendered
            .then(|rendered| sender.send(rendered))
            .map(|_| ())
            .map_err(|_| ());
        Box::new(work)
    }

    // Since `mktemp::Temp` implements Drop by deleting the directory, we don't need to worry about
    // leaving files or directories behind. On the flipside, we must ensure it is not dropped before
    // the last returned future that needs the directory finishes.
    pub fn render(&self, document_spec: DocumentSpec) -> Box<Future<Item = (), Error = ()>> {
        let dir = Temp::new_dir();
        let pool = CpuPool::new(3);
        let s3_prefix = s3_dir_name();

        if let Err(err) = dir {
            error!(self.config.logger, "{}", err);
            return Box::new(future::err(()));
        }

        let dir = dir.unwrap();

        let temp_dir_path = dir.to_path_buf();
        let logger = file_logger(self.config.logger.clone(), &temp_dir_path);

        let mut template_path = temp_dir_path.clone();
        template_path.push(Path::new(
            &document_spec.output_filename.replace("pdf", "tex"),
        ));
        let max_asset_size = self.config.max_asset_size;

        debug!(
            logger,
            "Trying to generate PDF with document spec: {:?}",
            document_spec
        );

        let DocumentSpec {
            assets_urls,
            callback_url,
            output_filename,
            template_url,
            variables,
            no_escape_latex,
            ..
        } = document_spec;
        let variables = if no_escape_latex { variables } else { escape_latex(variables) };

        let context = Context {
            assets_urls,
            callback_url: callback_url.clone(),
            config: self.config,
            logger: logger.clone(),
            tmp_dir: temp_dir_path.to_path_buf(),
            s3_prefix: s3_prefix.clone(),
            output_filename: output_filename.clone(),
            pool: pool.clone(),
        };

        let res = self.get_template(&template_url.0);
        let client = self.client.clone();

        debug!(context.logger, "Starting Renderer worker");

        // First download the template and populate it
        let bytes = res.and_then(move |res| res.get_body_bytes_with_limit(max_asset_size));

        let template_string = bytes.and_then(move |bytes| {
            debug!(context.logger, "Successfully downloaded the template");
            String::from_utf8(bytes)
                .map(|s| (context, s))
                .map_err(Error::from)
        });

        let rendered_template = template_string.and_then(move |(context, template_string)| {
            Tera::one_off(&template_string, &variables, false)
                .map(|rendered| (context, rendered))
                .map_err(Error::from)
        });

        let written_template_path = rendered_template.and_then(move |(context, latex_string)| {
            debug!(context.logger, "Writing template to {:?}", &template_path);
            let mut file = ::std::fs::File::create(&template_path)?;
            file.write_all(latex_string.as_bytes())
                .expect("could not write latex file");
            debug!(
                context.logger,
                "Template successfully written to {:?}",
                &template_path
                );
            Ok((context, template_path))
        });

        let download_client = client.clone();
        // Download the assets and save them in the temporary directory
        let files_written = written_template_path
            .and_then(move |(context, template_path)| {
                download_assets(context, download_client)
                    .map(|(context, _)| (context, template_path))
            });

        // Then run latex
        let latex_out = {
            let handle = self.handle.clone();
            files_written.and_then(move |(context, template_path)| {
                debug!(context.logger, "Spawning latex");
                debug!(context.logger, "template_path {:?}", template_path);
                debug!(context.logger, "tmp_dir {:?}", context.tmp_dir);
                debug!(
                    context.logger,
                    "Rendered template exists: {:?}",
                    template_path.exists()
                );
                Command::new("xelatex")
                    .current_dir(&context.tmp_dir)
                    .arg("-interaction=nonstopmode")
                    .arg("-file-line-error")
                    .arg("-shell-restricted")
                    .arg(template_path)
                    .output_async(&handle)
                    .map(|out| (context, out))
                    .map_err(|err| Error::with_chain(err, "Error generating PDF"))
            })
        };

        let output_path = latex_out
            .and_then(move |(context, output)| {
                let stdout = String::from_utf8(output.stdout).unwrap();
                if output.status.success() {
                    debug!(context.logger, "{}", stdout);
                    Ok(context)
                } else {
                    Err(ErrorKind::LatexFailed(stdout).into())
                }
            })
        .map(move |context| {
            // Construct the path to the generated PDF
            let mut path = context.tmp_dir.to_path_buf();
            path.push(Path::new(&context.output_filename));
            (context, path)
        });

        let s3_upload = output_path.and_then(move |(context, path)| {
            let key = format!("{}/{}", &context.s3_prefix, &context.output_filename);
            upload_document(context.config, context.logger.clone(), context.pool.clone(), path, key)
                .map(|presigned_url| (context, presigned_url))
        });

        let callback_client = client.clone();
        let callback_response = s3_upload.and_then(move |(context, presigned_url)| {
            report_success(
                context.config,
                context.logger,
                callback_client,
                context.callback_url.0,
                context.s3_prefix,
                presigned_url)
        });

        // Report errors to the callback url
        let handle_errors = {
            let logger = logger.clone();
            let client = self.client.clone();
            let s3_prefix = s3_prefix.clone();
            callback_response.or_else(move |error| {
                report_failure(logger, client, error, s3_prefix, callback_url.0)
            })
        };

        let tarred_workspace_uploaded = {
            let config = self.config.clone();
            let key = format!("{}/{}", &s3_prefix, "workspace.tar");
            handle_errors
                .then(move |_| {
                    pool.spawn_fn(move || upload_workspace(config, logger, temp_dir_path, key))
                })
                .map_err(move |_| { let _hold = dir; })
        };


        Box::new(tarred_workspace_uploaded)
    }
}

/// Downloads all assets from the document spec in the workspace in parallel. It fails if any of
/// those cannot be downloaded.
fn download_assets<S>(
    context: Context,
    client: S,
) -> Box<Future<Item = (Context, Vec<()>), Error = Error>>
where
    S: Service<Request = Request, Response = Response, Error = hyper::Error> + 'static + Clone,
{
    let max_asset_size = context.config.max_asset_size.clone();
    let assets_urls = context.assets_urls.clone();
    let tmp_dir = context.tmp_dir.to_path_buf();
    let logger = context.logger.clone();

    debug!(context.logger, "Downloading assets {:?}", context.assets_urls);
    let futures = assets_urls.into_iter().map(move |uri| {
        let logger = logger.clone();
        let mut path = tmp_dir.to_path_buf();
        let client = client.clone();

        let response = client.get_follow_redirect(&uri.0);

        let body = response.and_then(move |res| {
            let filename = res.filename();
            res.get_body_bytes_with_limit(max_asset_size)
                .map(|bytes| (bytes, filename))
        });

        body.and_then(move |(bytes, filename)| {
            let filename = filename.or_else(|| extract_filename_from_uri(&uri.0));
            match filename {
                Some(filename) => {
                    path.push(filename);
                    debug!(logger, "Writing asset {:?} as {:?}", uri, path);
                    ::std::fs::File::create(&path)
                        .and_then(|mut file| file.write_all(&bytes))
                        .map_err(|e| Error::with_chain(e, "Error writing asset"))
                }
                _ => Ok(()),
            }
        })
    });

    Box::new(future::join_all(futures).map(|futs| (context, futs)))
}

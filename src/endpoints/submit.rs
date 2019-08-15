use crate::papers::DocumentSpec;
use crate::prelude::*;
use crate::renderer::Renderer;
use futures::{FutureExt, TryFutureExt};

pub(crate) async fn submit(document_spec: DocumentSpec, config: Arc<Config>) -> Result<Response, EndpointError> {
    document_spec.validate(&config)?;

    tokio::executor::spawn(
        Renderer::new(config, document_spec)?
            .render()
            .boxed()
            .compat(),
    );

    Ok(empty_response())
}

use crate::papers::DocumentSpec;
use crate::prelude::*;
use crate::renderer::Renderer;
use futures::{FutureExt, TryFutureExt};

pub(crate) async fn submit(mut context: Context) -> Result<Response, EndpointError> {
    let document_spec: DocumentSpec = body_json(&mut context).await?;

    document_spec.validate(&context.config())?;

    tokio::executor::spawn(
        Renderer::new(context.config(), document_spec)?
            .render()
            .boxed()
            .compat(),
    );

    Ok(empty_response())
}

use crate::prelude::*;
use crate::renderer::Renderer;

pub(crate) async fn preview(mut context: Context) -> Result<Response, EndpointError> {
    let document_spec: DocumentSpec = body_json(&mut context).await?;

    document_spec.validate(&context.config())?;

    let mut renderer = Renderer::new(context.config(), document_spec)?;
    let populated_template = renderer.preview().await?;

    Ok(http::Response::new(populated_template.into()))
}

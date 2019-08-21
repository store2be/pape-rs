use crate::prelude::*;
use crate::papers::Renderer;

pub(crate) async fn preview(document_spec: DocumentSpec, config: Arc<Config>) -> Result<Response, EndpointError> {
    document_spec.validate(&config)?;

    let mut renderer = Renderer::new(config, document_spec)?;
    let populated_template = renderer.preview().await?;

    Ok(http::Response::new(populated_template.into()))
}

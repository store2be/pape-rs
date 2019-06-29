use crate::prelude::*;

pub(crate) async fn health_check(_: Context) -> Result<Response, EndpointError> {
    Ok(empty_response())
}

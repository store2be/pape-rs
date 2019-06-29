use crate::papers::Merger;
use crate::prelude::*;
use futures::{FutureExt, TryFutureExt};

pub(crate) async fn merge(mut context: Context) -> Result<Response, EndpointError> {
    let merge_spec: MergeSpec = body_json(&mut context).await?;

    merge_spec.validate()?;

    tokio::executor::spawn(
        Merger::new(context.config(), merge_spec)?
            .merge_documents()
            .boxed()
            .compat(),
    );

    Ok(empty_response())
}

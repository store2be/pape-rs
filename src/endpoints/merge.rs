use crate::papers::Merger;
use crate::prelude::*;
use futures::{FutureExt, TryFutureExt};

pub(crate) async fn merge(merge_spec: MergeSpec, config: Arc<Config>) -> Result<Response, EndpointError> {
    merge_spec.validate()?;

    tokio::executor::spawn(
        Merger::new(config, merge_spec)?
            .merge_documents()
            .boxed()
            .compat(),
    );

    Ok(empty_response())
}

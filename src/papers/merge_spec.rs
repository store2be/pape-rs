use crate::papers::uri::PapersUri;
use crate::prelude::*;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct MergeSpec {
    #[serde(default = "default_assets")]
    assets_urls: Vec<PapersUri>,
    callback_url: PapersUri,
    #[serde(default = "default_output_filename")]
    pub output_filename: String,
}

fn default_assets() -> Vec<PapersUri> {
    Vec::new()
}

fn default_output_filename() -> String {
    format!("out_{}.pdf", Utc::now().to_rfc3339())
}

impl MergeSpec {
    pub fn asset_urls(&self) -> impl std::iter::Iterator<Item = &hyper::Uri> {
        self.assets_urls.iter().map(|uri| &uri.0)
    }

    pub fn callback_url(&self) -> String {
        self.callback_url.0.to_string()
    }

    /// Validate that the specification is consistent, and that it can be expected to succeed.
    ///
    /// The error is intended for consumption by the client of the service.
    pub fn validate(&self) -> Result<(), EndpointError> {
        // Trying to merge 0 documents will not succeed
        if self.assets_urls.is_empty() {
            return Err(self.assets_count_error());
        }

        Ok(())
    }

    fn assets_count_error(&self) -> EndpointError {
        EndpointError::UnprocessableEntity {
            cause: format_err!("Cannot merge with an empty asset_urls array."),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, json};

    #[test]
    fn it_validates_uris() {
        let json = r#"{
            "callback_url": "?/",
            "asset_urls": ["example.com/pdf"]
        }"#;
        assert!(from_str::<MergeSpec>(&json).is_err());
    }

    #[test]
    fn merge_spec_validate_empty_asset_urls() {
        let wrong_spec_json = json!({
            "assets_urls": [],
            "callback_url": "https://example.com/callback",
        });

        let serialized: MergeSpec = serde_json::from_value(wrong_spec_json).unwrap();

        if let Err(EndpointError::UnprocessableEntity { cause: msg }) = serialized.validate() {
            assert_eq!(
                msg.to_string(),
                "Cannot merge with an empty asset_urls array."
            );
            return;
        }

        panic!("did not validate that asset_urls is not empty");
    }
}

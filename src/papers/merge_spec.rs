use chrono::Utc;
use error::{Error, ErrorKind};
use papers::uri::PapersUri;

#[derive(Deserialize, Serialize, Debug)]
pub struct MergeSpec {
    #[serde(default = "default_assets")]
    pub assets_urls: Vec<PapersUri>,
    pub callback_url: PapersUri,
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
    /// Validate that the specification is consistent, and that it can be expected to succeed.
    ///
    /// The error is intended for consumption by the client of the service.
    pub fn validate(&self) -> Result<(), Error> {
        // Trying to merge 0 documents will not succeed
        if self.assets_urls.is_empty() {
            return Err(MergeSpec::validation_error());
        }

        Ok(())
    }

    fn validation_error() -> Error {
        Error::from_kind(ErrorKind::UnprocessableEntity(
            "Cannot merge with an empty asset_urls array.".to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::from_str;

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
        use error::{Error, ErrorKind};
        use serde_json;

        let wrong_spec_json = json!({
            "assets_urls": [],
            "callback_url": "https://example.com/callback",
        });

        let serialized: MergeSpec = serde_json::from_value(wrong_spec_json).unwrap();

        if let Err(Error(ErrorKind::UnprocessableEntity(msg), _)) = serialized.validate() {
            assert_eq!(msg, "Cannot merge with an empty asset_urls array.");
            return;
        }

        panic!("did not validate that asset_urls is not empty");
    }
}

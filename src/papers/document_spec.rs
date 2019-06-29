use crate::latex::escape_tex;
use crate::papers::uri::PapersUri;
use crate::prelude::*;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use slog::error;

/// See https://serde.rs/custom-date-format.html for the custom deserialization.
/// An alternative design would be making a newtype containing an Uri and implementing Deserialize
/// for that.
#[derive(Deserialize, Serialize, Debug)]
pub struct DocumentSpec {
    #[serde(default = "default_assets")]
    pub assets_urls: Vec<PapersUri>,
    pub callback_url: PapersUri,
    #[serde(default = "default_output_filename")]
    pub output_filename: String,
    pub template_url: PapersUri,
    #[serde(default = "default_value")]
    pub variables: serde_json::Value,
    #[serde(default = "return_false")]
    pub no_escape_tex: bool,
}

impl DocumentSpec {
    pub fn validate(&self, config: &Config) -> Result<(), EndpointError> {
        if self.assets_urls.len() > config.max_assets_per_document as usize {
            error!(
                &config.logger,
                "Assets URLs length exceeds the maximum ({}).\
                 To change it set PAPERS_MAX_ASSETS_PER_DOCUMENT.",
                &config.max_assets_per_document,
            );
            return Err(EndpointError::UnprocessableEntity {
                cause: format_err!(
                    "Asset URLs length exceeds the maximum ({}).",
                    &config.max_assets_per_document
                ),
            });
        }

        Ok(())
    }

    pub fn variables(&self) -> serde_json::Value {
        if self.no_escape_tex {
            self.variables.clone()
        } else {
            escape_tex(self.variables.clone())
        }
    }

    pub fn callback_url(&self) -> String {
        self.callback_url.0.to_string()
    }

    pub fn asset_urls(&self) -> impl std::iter::Iterator<Item = &hyper::Uri> {
        self.assets_urls.iter().map(|uri| &uri.0)
    }
}

fn return_false() -> bool {
    false
}

fn default_assets() -> Vec<PapersUri> {
    Vec::new()
}

fn default_output_filename() -> String {
    format!("out_{}.pdf", Utc::now().to_rfc3339())
}

fn default_value() -> serde_json::Value {
    json!({})
}

#[cfg(test)]
mod tests {
    use super::DocumentSpec;
    use serde_json::{from_str, json};

    #[test]
    fn it_validates_uris() {
        let json = r#"{
            "callback_url": "?/",
            "template_url": "bac"
        }"#;
        assert!(from_str::<DocumentSpec>(&json).is_err());
    }

    #[test]
    fn it_works_without_assets() {
        let json = r#"{
            "callback_url": "abc",
            "template_url": "def"
        }"#;
        let spec = from_str::<DocumentSpec>(&json).unwrap();
        assert_eq!(spec.variables, json!({}));
        assert_eq!(spec.assets_urls.len(), 0);
    }

    #[test]
    fn it_parses_uris() {
        let json = r#"{
            "callback_url": "abc",
            "template_url": " http://127.0.0.1/template  "
        }"#;
        let spec = from_str::<DocumentSpec>(&json).unwrap();
        assert_eq!(spec.variables, json!({}));
        assert_eq!(
            format!("{}", spec.template_url.0),
            "http://127.0.0.1/template"
        );
    }
}

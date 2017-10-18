use chrono::Utc;
use papers::uri::PapersUri;

#[derive(Deserialize, Serialize, Debug)]
pub struct MergeSpec {
    #[serde(default = "default_assets")] pub assets_urls: Vec<PapersUri>,
    pub callback_url: PapersUri,
    #[serde(default = "default_output_filename")] pub output_filename: String,
}

fn default_assets() -> Vec<PapersUri> {
    Vec::new()
}

fn default_output_filename() -> String {
    format!("out_{}.pdf", Utc::now().to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::MergeSpec;
    use serde_json::from_str;

    #[test]
    fn it_validates_uris() {
        let json = r#"{
            "callback_url": "?/",
            "asset_urls": ["example.com/pdf"]
        }"#;
        assert!(from_str::<MergeSpec>(&json).is_err());
    }
}

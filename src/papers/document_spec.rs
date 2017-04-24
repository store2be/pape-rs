use hyper::Uri;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct PapersUri(#[serde(with = "uri_deserializer")] pub Uri);

/// See https://serde.rs/custom-date-format.html for the custom deserialization.
/// An alternative design would be making a newtype containing an Uri and implementing Deserialize
/// for that.
#[derive(Deserialize, Debug)]
pub struct DocumentSpec {
    #[serde(default = "default_assets")]
    pub assets_urls: Vec<PapersUri>,
    pub callback_url: PapersUri,
    pub template_url: PapersUri,
    #[serde(default = "default_hashmap")]
    pub variables: HashMap<String, String>,
}

mod uri_deserializer {
    use hyper::Uri;
    use serde::{self, Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Uri, D::Error>
        where D: Deserializer<'de> {
        let uri_string = String::deserialize(deserializer)?;
        uri_string.parse().map_err(serde::de::Error::custom)
    }
}


fn default_hashmap() -> HashMap<String, String> { HashMap::new() }
fn default_assets() -> Vec<PapersUri> { Vec::new() }

#[cfg(test)]
mod tests {
    use super::DocumentSpec;
    use serde_json::from_str;

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
        assert!(spec.variables.is_empty());
        assert_eq!(spec.assets_urls.len(), 0);
    }
}

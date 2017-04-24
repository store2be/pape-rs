use hyper::Uri;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct PapersUri(#[serde(with = "uri_deserializer")] pub Uri);

/// See https://serde.rs/custom-date-format.html for the custom deserialization.
/// An alternative design would be making a newtype containing an Uri and implementing Deserialize
/// for that.
#[derive(Deserialize, Debug)]
pub struct DocumentSpec {
    pub assets_urls: Option<Vec<PapersUri>>,
    pub callback_url: PapersUri,
    pub template_url: PapersUri,
    pub variables: Option<HashMap<String, String>>,
}

mod uri_deserializer {
    use hyper::Uri;
    use serde::{self, Deserialize, Serializer, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Uri, D::Error>
        where D: Deserializer<'de> {
        let uri_string = String::deserialize(deserializer)?;
        uri_string.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    // extern crate serde_test;

    // #[test]
    // fn it_tests
}

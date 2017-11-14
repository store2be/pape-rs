use hyper::Uri;

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct PapersUri(#[serde(with = "uri_deserializer")] pub Uri);

mod uri_deserializer {
    use hyper::Uri;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(uri: &Uri, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", uri))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Uri, D::Error>
    where
        D: Deserializer<'de>,
    {
        let uri_string = String::deserialize(deserializer)?;
        uri_string.trim().parse().map_err(serde::de::Error::custom)
    }
}

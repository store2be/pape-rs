#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Summary {
    File(String),
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::Summary;
    use serde_json::to_string;

    #[test]
    fn it_serializes_errors_as_expected() {
        let summary = Summary::Error("meow".to_string());
        assert_eq!(&to_string(&summary).unwrap(), "{\"error\":\"meow\"}");
    }

    #[test]
    fn it_serializes_success_as_expected() {
        let summary = Summary::File("https://example.com/the_file.pdf".to_string());
        assert_eq!(&to_string(&summary).unwrap(), "{\"file\":\"https://example.com/the_file.pdf\"}");
    }
}

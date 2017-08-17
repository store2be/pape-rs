#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase", untagged)]
pub enum Summary {
    File { file: String, s3_folder: String },
    Error { error: String, s3_folder: String },
}

#[cfg(test)]
mod tests {
    use super::Summary;
    use serde_json::to_string;

    #[test]
    fn it_serializes_errors_as_expected() {
        let summary = Summary::Error {
            error: "meow".to_string(),
            s3_folder: "/the/bucket/the/key".to_string(),
        };
        assert_eq!(
            &to_string(&summary).unwrap(),
            "{\"error\":\"meow\",\"s3_folder\":\"/the/bucket/the/key\"}"
        );
    }

    #[test]
    fn it_serializes_success_as_expected() {
        let summary = Summary::File {
            file: "https://example.com/the_file.pdf".to_string(),
            s3_folder: "/my/bucket/my/key".to_string(),
        };
        assert_eq!(
            &to_string(&summary).unwrap(),
            "{\"file\":\"https://example.com/the_file.pdf\",\"s3_folder\":\"/my/bucket/my/key\"}"
        );
    }
}

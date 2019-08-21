use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase", untagged)]
pub enum Summary {
    File {
        file: String,
        s3_folder: String,
    },
    Error {
        error: String,
        backtrace: String,
        s3_folder: String,
    },
}

#[cfg(test)]
mod tests {
    use super::Summary;

    #[test]
    fn it_serializes_errors_as_expected() {
        let summary = Summary::Error {
            backtrace: "".to_owned(),
            error: "meow".to_owned(),
            s3_folder: "/the/bucket/the/key".to_owned(),
        };
        assert_eq!(
            &serde_json::to_string(&summary).unwrap(),
            "{\"error\":\"meow\",\"backtrace\":\"\",\"s3_folder\":\"/the/bucket/the/key\"}"
        );
    }

    #[test]
    fn it_serializes_success_as_expected() {
        let summary = Summary::File {
            file: "https://example.com/the_file.pdf".to_owned(),
            s3_folder: "/my/bucket/my/key".to_owned(),
        };
        assert_eq!(
            &serde_json::to_string(&summary).unwrap(),
            "{\"file\":\"https://example.com/the_file.pdf\",\"s3_folder\":\"/my/bucket/my/key\"}"
        );
    }
}

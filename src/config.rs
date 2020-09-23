use crate::human_size::Bytes;
use rusoto_core::region::Region;
use slog::{o, warn, Logger};
use sloggers::types::Severity;
use sloggers::Build;
use std::str::FromStr;

const MAX_ASSET_SIZE_DEFAULT: u32 = 10_000_000;
const MAX_ASSETS_PER_DOCUMENT_DEFAULT: u32 = 20;

fn max_assets_per_document(logger: &Logger) -> u32 {
    match std::env::var("PAPERS_MAX_ASSETS_PER_DOCUMENT").map(|max| max.parse()) {
        Ok(Ok(max)) => max,
        Ok(Err(_)) => {
            warn!(
                logger,
                "Unable to parse PAPERS_MAX_ASSETS_PER_DOCUMENT environment variable"
            );
            MAX_ASSETS_PER_DOCUMENT_DEFAULT
        }
        _ => MAX_ASSETS_PER_DOCUMENT_DEFAULT,
    }
}

fn aws_region() -> rusoto_core::region::Region {
    std::env::var("PAPERS_LOCALSTACK_ENDPOINT")
        .and_then(|ep| {
            Ok(rusoto_core::region::Region::Custom {
                endpoint: ep,
                name: "localstack".to_owned(),
            })
        })
        .or_else::<rusoto_core::region::Region, _>(|_| {
            Ok(std::env::var("PAPERS_AWS_REGION")
                .expect("The PAPERS_AWS_REGION environment variable was not provided")
                .parse::<rusoto_core::region::Region>()
                .expect("The provided AWS region is not valid"))
        })
        .expect("Something went wrong with processing the AWS region")
}

/// Relies on the PAPERS_LOG_LEVEL env variable.
pub fn build_logger() -> Logger {
    let minimum_level = if let Ok("debug") = std::env::var("PAPERS_LOG_LEVEL")
        .as_ref()
        .map(String::as_str)
    {
        Severity::Debug
    } else {
        Severity::Info
    };

    let drain = sloggers::terminal::TerminalLoggerBuilder::new()
        .level(minimum_level)
        .build()
        .expect("Could not build a terminal logger");
    slog::Logger::root(drain, o!("version" => env!("CARGO_PKG_VERSION")))
}

/// Configuration for the S3 integration.
#[derive(Debug)]
pub struct S3Config {
    /// The bucket name.
    pub bucket: String,
    /// The AWS region of the bucket.
    pub region: Region,
    /// The expiration time of presigned URLs in seconds.
    pub expiration_time: u32,
    /// The AWS credentials.
    pub credentials: rusoto_credential::AwsCredentials,
    /// The AWS credentials provider.
    credentials_provider: rusoto_credential::EnvironmentProvider,
}

impl S3Config {
    pub(crate) fn client(&self) -> rusoto_s3::S3Client {
        rusoto_s3::S3Client::new_with(
            rusoto_core::request::HttpClient::new().unwrap(),
            self.credentials_provider.clone(),
            self.region.clone(),
        )
    }
}

/// Please refer to the README for more details about configuration
#[derive(Debug)]
pub struct Config {
    /// A long secret that is used in the Authorization header to authenticate a request against
    /// papers
    pub auth: Option<String>,
    /// Limits the number of assets allowed for a given DocumentSpec
    pub max_assets_per_document: u32,
    /// Limits the size of the assets downloaded by the service, including templates
    pub max_asset_size: u32,
    /// The root logger for the application
    pub logger: Logger,
    /// The S3 configuration
    pub s3: S3Config,
}

impl Config {
    /// Create a default `Config` for testing purposes.
    pub fn for_tests() -> Config {
        Config {
            auth: None,
            logger: build_logger(),
            max_asset_size: MAX_ASSET_SIZE_DEFAULT,
            max_assets_per_document: MAX_ASSETS_PER_DOCUMENT_DEFAULT,
            s3: S3Config {
                bucket: "walrus".into(),
                credentials: rusoto_credential::AwsCredentials::new("a", "b", None, None),
                credentials_provider: rusoto_credential::EnvironmentProvider::default(),
                expiration_time: 3600,
                region: rusoto_core::region::Region::Custom {
                    endpoint: "http://s3.localhost".into(),
                    name: "local_s3".into(),
                },
            },
        }
    }

    /// The normal way to construct a `Config`, reading from environment variables.
    pub fn from_env() -> Config {
        use futures01::Future;
        use rusoto_credential::ProvideAwsCredentials;

        let max_asset_size = std::env::var("PAPERS_MAX_ASSET_SIZE")
            .map_err(|_| ())
            .and_then(|s| Bytes::from_str(&s))
            .map(|bytes| bytes.0)
            .unwrap_or(MAX_ASSET_SIZE_DEFAULT);

        let auth = std::env::var("PAPERS_BEARER").ok();

        let logger = build_logger();
        let max_assets_per_document = max_assets_per_document(&logger);

        let expiration_time: u32 = std::env::var("PAPERS_S3_EXPIRATION_TIME")
            .unwrap_or_else(|_| "86400".to_string()) // one day
            .parse()
            .expect("PAPERS_S3_EXPIRATION_TIME should be a duration in seconds");

        let credentials_provider = rusoto_credential::EnvironmentProvider::with_prefix("PAPERS");

        let credentials = credentials_provider
            .credentials()
            .wait()
            .expect("error reading AWS credentials from environment");

        let s3 = S3Config {
            bucket: std::env::var("PAPERS_S3_BUCKET")
                .expect("The PAPERS_S3_BUCKET environment variable was not provided"),
            credentials,
            credentials_provider,
            region: aws_region(),
            expiration_time,
        };

        Config {
            auth,
            logger,
            max_asset_size,
            max_assets_per_document,
            s3,
        }
    }

    /// Return a new `Config` with the specified auth secret.
    pub fn with_auth(self, auth: String) -> Config {
        Config {
            auth: Some(auth),
            ..self
        }
    }

    /// Set `max_assets_per_documents` and return `self`.
    pub fn with_max_assets_per_document(self, max_assets_per_document: u32) -> Config {
        Config {
            max_assets_per_document,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_aws_region_localstack() {
        std::env::set_var("PAPERS_LOCALSTACK_ENDPOINT", "my-endpoint");

        assert_eq!(
            aws_region(),
            rusoto_core::region::Region::Custom {
                endpoint: "my-endpoint".to_owned(),
                name: "localstack".to_owned()
            }
        )
    }

    #[test]
    fn test_aws_region_env() {
        std::env::set_var("PAPERS_AWS_REGION", "us-east-1");

        assert_eq!(aws_region(), rusoto_core::region::Region::UsEast1,)
    }
}

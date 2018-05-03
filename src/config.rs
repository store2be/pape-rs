use chrono::{DateTime, Duration, Utc};
use dotenv::dotenv;
use human_size::Bytes;
use rusoto::credential::{AwsCredentials, CredentialsError, ProvideAwsCredentials};
use rusoto::region::Region;
use slog::{self, Logger};
use sloggers::types::Severity;
use sloggers::{self, Build};
use std::str::FromStr;

fn max_assets_per_document(logger: &Logger) -> u8 {
    let default = 20;
    match ::std::env::var("PAPERS_MAX_ASSETS_PER_DOCUMENT").map(|max| max.parse()) {
        Ok(Ok(max)) => max,
        Ok(Err(_)) => {
            warn!(
                logger,
                "Unable to parse PAPERS_MAX_ASSETS_PER_DOCUMENT environment variable"
            );
            default
        }
        _ => default,
    }
}

pub fn is_debug_active() -> bool {
    match ::std::env::var("PAPERS_LOG_LEVEL") {
        Ok(ref level) if level.contains("debug") => true,
        _ => false,
    }
}

#[derive(Debug)]
pub struct S3Config {
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    pub region: Region,
    pub expiration_time: u32,
}

/// Please refer to the README for more details about configuration
#[derive(Debug)]
pub struct Config {
    /// A long secret that is used in the Authorization header to authenticate a request against
    /// papers
    pub auth: String,
    /// Limits the number of assets allowed for a given DocumentSpec
    pub max_assets_per_document: u8,
    /// Limits the size of the assets downloaded by the service, including templates
    pub max_asset_size: u32,
    /// The root logger for the application
    pub logger: Logger,
    /// The S3 configuration
    pub s3: S3Config,
}

impl Config {
    pub fn from_env() -> Config {
        dotenv().ok();

        let max_asset_size = ::std::env::var("PAPERS_MAX_ASSET_SIZE")
            .map_err(|_| ())
            .and_then(|s| Bytes::from_str(&s))
            .map(|bytes| bytes.0)
            .unwrap_or(10_000_000);

        let auth = ::std::env::var("PAPERS_BEARER").unwrap_or_else(|_| "".to_string());

        let minimum_level = if is_debug_active() {
            Severity::Debug
        } else {
            Severity::Info
        };
        let drain = sloggers::terminal::TerminalLoggerBuilder::new()
            .level(minimum_level)
            .build()
            .expect("Could not build a terminal logger");
        let logger = slog::Logger::root(drain, o!());

        let max_assets_per_document = max_assets_per_document(&logger);

        let aws_region_string = ::std::env::var("PAPERS_AWS_REGION")
            .expect("The PAPERS_AWS_REGION environment variable was not provided");

        let expiration_time: u32 = ::std::env::var("PAPERS_S3_EXPIRATION_TIME")
            .unwrap_or_else(|_| "86400".to_string()) // one day
            .parse()
            .expect("PAPERS_S3_EXPIRATION_TIME should be a duration in seconds");

        let s3 = S3Config {
            bucket: ::std::env::var("PAPERS_S3_BUCKET")
                .expect("The PAPERS_S3_BUCKET environment variable was not provided"),
            access_key: ::std::env::var("PAPERS_AWS_ACCESS_KEY")
                .expect("The PAPERS_AWS_ACCESS_KEY environment variable was not provided"),
            secret_key: ::std::env::var("PAPERS_AWS_SECRET_KEY")
                .expect("The PAPERS_AWS_SECRET_KEY environment variable was not provided"),
            region: aws_region_string
                .parse()
                .expect("The provided AWS region is not valid"),
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

    pub fn with_auth(self, auth: String) -> Config {
        Config { auth, ..self }
    }

    pub fn with_max_assets_per_document(self, max_assets_per_document: u8) -> Config {
        Config {
            max_assets_per_document,
            ..self
        }
    }
}

impl<'a> ProvideAwsCredentials for &'a Config {
    fn credentials(&self) -> Result<AwsCredentials, CredentialsError> {
        Ok(AwsCredentials::new(
            self.s3.access_key.clone(),
            self.s3.secret_key.clone(),
            None,
            DateTime::<Utc>::checked_add_signed(Utc::now(), Duration::days(1)).unwrap(),
        ))
    }
}

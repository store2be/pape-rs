use human_size::Bytes;
use std::str::FromStr;
use slog::{self, Logger, Filter, DrainExt, Level};
use slog_term;

fn max_assets_per_document(logger: &slog::Logger) -> u8 {
    let default = 20;
    match ::std::env::var("PAPERS_MAX_ASSETS_PER_DOCUMENT").map(|max| max.parse()) {
        Ok(Ok(max)) => max,
        Ok(Err(_)) => {
            warn!(logger,
                  "Unable to parse PAPERS_MAX_ASSETS_PER_DOCUMENT environmental variable");
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
pub struct Config {
    /// TODO: document this here
    pub auth: String,
    /// Limits the number of assets allowed for a given DocumentSpec
    pub max_assets_per_document: u8,
    /// Limits the size of the assets downloaded by the service, including templates
    pub max_asset_size: u32,
    /// The root logger for the application
    pub logger: Logger,
}

impl Config {
    pub fn from_env() -> Config {
        let max_asset_size = ::std::env::var("PAPERS_MAX_ASSET_SIZE")
            .map_err(|_| ())
            .and_then(|s| Bytes::from_str(&s))
            .map(|bytes| bytes.0)
            .unwrap_or(10_000_000);

        let auth = ::std::env::var("PAPERS_BEARER").unwrap_or_else(|_| "".to_string());

        let minimum_level = if is_debug_active() {
            Level::Debug
        } else {
            Level::Info
        };
        let drain = slog_term::streamer().full().build().fuse();
        let drain = Filter::new(drain,
                                move |record| record.level().is_at_least(minimum_level));
        let logger = slog::Logger::root(drain, o!());

        let max_assets_per_document = max_assets_per_document(&logger);

        Config {
            auth,
            max_assets_per_document,
            max_asset_size,
            logger,
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

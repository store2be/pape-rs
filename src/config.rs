use human_size::Bytes;
use std::str::FromStr;

pub struct Config {
    /// TODO: document this here
    pub auth: String,
    /// Limits the size of the assets downloaded by the service, including templates
    pub max_asset_size: u32,
}

impl Config {
    pub fn from_env() -> Config {
        let max_asset_size = ::std::env::var("PAPERS_MAX_ASSET_SIZE")
            .map_err(|_| ())
            .and_then(|s| Bytes::from_str(&s))
            .map(|bytes| bytes.0)
            .unwrap_or(10_000_000);

        let auth = ::std::env::var("PAPERS_BEARER").unwrap_or_else(|_| "".to_string());

        Config {
            auth,
            max_asset_size,
        }
    }

    pub fn with_auth(self, auth: String) -> Config {
        Config {
            auth,
            ..self
        }
    }
}

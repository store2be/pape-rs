// Temporarily disabled because of warnings in error_chain
// #![deny(warnings)]

// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

extern crate dotenv;
extern crate chrono;
#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
extern crate hyper_tls;
#[macro_use]
extern crate lazy_static;
extern crate mktemp;
extern crate mime;
#[cfg(test)]
#[macro_use]
extern crate quickcheck;
extern crate regex;
extern crate rusoto_core as rusoto;
extern crate rusoto_s3 as s3;
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog;
extern crate sloggers;
extern crate tar;
extern crate tera;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_process;

pub mod config;
pub mod error;
pub mod http;
mod human_size;
mod latex;
pub mod papers;
pub mod renderer;
mod utils;
pub mod server;
pub mod test_utils;

pub mod prelude {
    pub use config::Config;
    pub use error::{Error, ErrorKind};
    pub use papers::{DocumentSpec, FromHandle, Papers, PapersUri, Summary};
    pub use renderer::Renderer;
    pub use server::Server;
}

#![deny(warnings)]

extern crate chrono;
#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;
#[macro_use]
extern crate lazy_static;
extern crate mktemp;
extern crate mime_multipart as multipart;
#[cfg(test)]
#[macro_use]
extern crate quickcheck;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate tera;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_process;

pub mod config;
pub mod error;
pub mod http;
mod human_size;
pub mod papers;
pub mod renderer;
pub mod server;
pub mod test_utils;

pub mod prelude {
    pub use config::Config;
    pub use error::{Error, ErrorKind};
    pub use papers::{FromHandle, DocumentSpec, Papers, PapersUri};
    pub use renderer::Renderer;
    pub use server::Server;
}

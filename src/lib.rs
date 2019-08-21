//! # papers
//!
//! These docs are for papers as a library. This is not a supported use case for now, we expose
//! exports to make integration testing easier internally. This may become more fleshed out over
//! time.
//!
//! For documentation on the service itself, which is meant to be used as an executable, please
//! refer to the [README](https://github.com/store2be/pape-rs).

#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![deny(missing_docs)]

mod app;
mod auth;
/// Service configuration.
pub mod config;
mod endpoints;
mod human_size;
/// Latex-related utilities.
mod latex;
/// Papers local.
pub mod local_server;
/// Core logic for the asynchronous jobs.
pub mod papers;
/// Prelude.
mod prelude;
mod renderer;
/// Utility modules.
pub mod utils;

pub use app::app;
pub use config::Config;

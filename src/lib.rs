#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate hyper;
extern crate mktemp;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate tera;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_service;
extern crate tokio_process;

mod error;
mod http_client;
mod papers;
mod workspace;
pub mod server;

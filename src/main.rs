#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate hyper;
#[macro_use]
extern crate log;
extern crate mktemp;
extern crate tera;
extern crate tokio_service;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate slog;
extern crate slog_envlogger;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_process;

mod server;
mod error;
mod http_client;
mod papers;
mod workspace;


fn main() {
    let _logger = slog_envlogger::init().unwrap();
    let port = ::std::env::var("PAPERS_PORT").unwrap_or("8080".to_string());
    server::Server::new()
        .with_port(port.parse().unwrap())
        .start()
}

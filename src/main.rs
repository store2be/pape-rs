#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate hyper;
extern crate mktemp;
extern crate tera;
extern crate tokio_service;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_process;

mod server;
mod error;
mod http_client;
mod papers;
mod workspace;


fn main() {
    let port = ::std::env::var("PAPERS_PORT").unwrap_or("8080".to_string());
    server::Server::new()
        .with_port(port.parse().unwrap())
        .start()
}

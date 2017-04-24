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
mod papers;
mod workspace;


fn main() {
    server::Server::new().start()
}

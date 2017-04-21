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

mod error;
mod papers;
mod template;
mod workspace;

use papers::Papers;

use futures::Future;
use hyper::server::Http;

fn main() {
    let mut core = tokio_core::reactor::Core::new().unwrap();;
    let papers_service = Papers::new(core.remote());
    let socket_addr = "0.0.0.0:80".parse().unwrap();
    println!("Starting server on http://{}", socket_addr);
    let tcp_stream = tokio_core::net::TcpStream::connect(&socket_addr, &core.handle())
        .wait()
        .unwrap();
    Http::new()
        .bind_connection(&core.handle(), tcp_stream, socket_addr, papers_service);
    core.run::<futures::future::Empty<(), ()>>(futures::future::empty()).unwrap()
}

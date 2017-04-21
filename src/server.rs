extern crate futures;
extern crate hyper;
extern crate mktemp;
extern crate tera;
extern crate tokio_service;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

mod papers;
mod template;
mod workspace;

use papers::Papers;

use hyper::server::{Http, Server};
use std::net::SocketAddr;
use std::str::FromStr;

fn main() {
    Http::new().bind(&"0.0.0.0:80".parse().unwrap(), || Ok(Papers))
        .unwrap()
        .run();
}

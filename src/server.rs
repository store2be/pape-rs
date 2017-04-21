extern crate futures;
extern crate hyper;
extern crate mktemp;
extern crate multipart;
extern crate tera;
extern crate tokio_service;

mod pdf_renderer;
mod template;
mod workspace;

use pdf_renderer::PdfRenderer;

use hyper::server::{Http, Server};
use std::net::SocketAddr;
use std::str::FromStr;

fn main() {
    Http::new().bind(&"0.0.0.0:80".parse().unwrap(), || Ok(PdfRenderer))
        .unwrap()
        .run();
}

// Temporarily disabled because of warnings in error_chain
// #![deny(warnings)]

extern crate papers;

fn main() {
    let port = ::std::env::var("PAPERS_PORT").unwrap_or_else(|_| "8080".to_string());
    papers::server::Server::new()
        .with_port(port.parse().unwrap())
        .start()
}

extern crate slog_envlogger;

extern crate papers;

fn main() {
    let _logger = slog_envlogger::init().unwrap();
    let port = ::std::env::var("PAPERS_PORT").unwrap_or_else(|_| "8080".to_string());
    server::Server::new()
        .with_port(port.parse().unwrap())
        .start()
}

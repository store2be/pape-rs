use papers::Papers;

use futures::future;
use futures::{Future, Stream};
use hyper::server::{Http, NewService};
use slog;
use slog::{Filter, DrainExt, Level};
use slog_term;
use tokio_core;

pub fn is_debug_active() -> bool {
    match ::std::env::var("PAPERS_LOG_LEVEL") {
        Ok(ref level) if level.contains("debug") => true,
        _ => false,
    }
}

fn max_assets_per_document(logger: &slog::Logger) -> u8 {
    let default = 20;
    match ::std::env::var("PAPERS_MAX_ASSETS_PER_DOCUMENT").map(|max| max.parse()) {
        Ok(Ok(max)) => max,
        Ok(Err(_)) => {
            warn!(logger, "Unable to parse PAPERS_MAX_ASSETS_PER_DOCUMENT environmental variable");
            default
        },
        _ => default,
    }
}

pub struct Server {
    auth: String,
    port: i32,
    logger: slog::Logger,
    max_assets_per_document: u8,
}

impl Server {
    pub fn new() -> Server {
        let minimum_level = if is_debug_active() {
            Level::Debug
        } else {
            Level::Info
        };
        let drain = slog_term::streamer().full().build().fuse();
        let drain = Filter::new(drain,
                                move |record| record.level().is_at_least(minimum_level));
        let logger = slog::Logger::root(drain, o!());
        let bearer = ::std::env::var("PAPERS_BEARER").unwrap_or_else(|_| "".to_string());
        let max_assets_per_document = max_assets_per_document(&logger);
        Server {
            auth: bearer,
            port: 8008,
            logger: logger,
            max_assets_per_document,
        }
    }

    pub fn with_auth(self, auth: String) -> Server {
        Server { auth, ..self }
    }

    pub fn with_port(self, port: i32) -> Server {
        Server { port, ..self }
    }

    pub fn start(self) {
        let mut core = tokio_core::reactor::Core::new().unwrap();
        let papers_service = Papers::new(core.remote(), self.logger.new(o!()), self.auth);
        let socket_addr = format!("0.0.0.0:{:?}", self.port).parse().unwrap();
        let handle = core.handle();
        let listener = tokio_core::net::TcpListener::bind(&socket_addr, &core.handle()).unwrap();
        let work = listener
            .incoming()
            .for_each(|(tcp_stream, socket_addr)| {
                          Http::new().bind_connection(&handle,
                                                      tcp_stream,
                                                      socket_addr,
                                                      papers_service.new_service().unwrap());
                          future::ok(())
                      });
        core.run(future::ok(info!(self.logger.new(o!("version" => env!("CARGO_PKG_VERSION"))),
                                  "Server started on http://{}",
                                  socket_addr))
                         .and_then(|_| work))
            .unwrap()
    }
}

impl ::std::default::Default for Server {
    fn default() -> Server {
        Server::new()
    }
}

use papers::Papers;

use futures::future;
use futures::{Future, Stream};
use hyper::server::Http;
use slog;
use slog::{Filter, DrainExt, Level};
use slog_term;
use tokio_service::NewService;
use tokio_core;


pub struct Server {
    port: i32,
    logger: slog::Logger,
}

impl Server {
    pub fn new() -> Server {
        let minimum_level = match ::std::env::var("PAPERS_LOG_LEVEL") {
            Ok(ref level) if level.contains("debug") => Level::Debug,
            _ => Level::Info,
        };
        let drain = slog_term::streamer().full().build().fuse();
        let drain = Filter::new(drain, move |record| record.level().is_at_least(minimum_level));
        let logger = slog::Logger::root(drain, o!("version" => env!("CARGO_PKG_VERSION")));
        Server {
            port: 8008,
            logger: logger,
        }
    }

    pub fn with_port(self, port: i32) -> Server {
        Server {
            port: port,
            ..self
        }
    }

    pub fn start(self) {
        let mut core = tokio_core::reactor::Core::new().unwrap();;
        let papers_service = Papers::new(core.remote(), self.logger.new(o!()));
        let socket_addr = format!("0.0.0.0:{:?}", self.port).parse().unwrap();
        let handle = core.handle();
        let listener = tokio_core::net::TcpListener::bind(&socket_addr, &core.handle()).unwrap();
        let work = listener.incoming().for_each(|(tcp_stream, socket_addr)| {
            Http::new().bind_connection(&handle, tcp_stream, socket_addr, papers_service.new_service().unwrap());
            future::ok(())
        });
        core.run(future::ok(info!(self.logger, "Server started on http://{}", socket_addr)).and_then(|_| work)).unwrap()
    }
}

impl ::std::default::Default for Server {
    fn default() -> Server {
        Server::new()
    }
}

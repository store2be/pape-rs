use papers::Papers;

use futures::future;
use futures::Stream;
use hyper::Client;
use hyper::server::{Http, NewService};
use hyper_tls::HttpsConnector;
use tokio_core;
use config::Config;

pub struct Server {
    port: i32,
    config: &'static Config,
}

impl Server {
    pub fn new() -> Server {
        lazy_static! {
            static ref CONFIG: Config = Config::from_env();
        }

        Server {
            port: 8008,
            config: &CONFIG,
        }
    }

    pub fn with_config(self, config: &'static Config) -> Server {
        Server { config, ..self }
    }

    pub fn with_port(self, port: i32) -> Server {
        Server { port, ..self }
    }

    pub fn start(self) {
        let mut core = tokio_core::reactor::Core::new().unwrap();
        let papers_service: Papers<Client<HttpsConnector>> = Papers::new(core.remote(),
                                                                         self.config);
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

        info!(self.config
                  .logger
                  .new(o!("version" => env!("CARGO_PKG_VERSION"))),
              "Server started on http://{}",
              socket_addr);

        core.run(work).unwrap()
    }
}

impl ::std::default::Default for Server {
    fn default() -> Server {
        Server::new()
    }
}

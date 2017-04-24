use papers::Papers;

use futures::future;
use futures::Stream;
use hyper::server::Http;
use tokio_service::NewService;
use tokio_core;

pub struct Server {
    port: i32,
}

impl Server {
    pub fn new() -> Server {
        Server {
            port: 8008,
        }
    }

    pub fn with_port(self, port: i32) -> Server {
        Server {
            port: port,
        }
    }

    pub fn start(self) {
        let mut core = tokio_core::reactor::Core::new().unwrap();;
        let papers_service = Papers::new(core.remote());
        let socket_addr = format!("0.0.0.0:{:?}", self.port).parse().unwrap();
        let handle = core.handle();
        println!("Starting server on http://{}", socket_addr);
        let listener = tokio_core::net::TcpListener::bind(&socket_addr, &core.handle()).unwrap();
        let work = listener.incoming().for_each(|(tcp_stream, socket_addr)| {
            Http::new().bind_connection(&handle, tcp_stream, socket_addr, papers_service.new_service().unwrap());
            future::ok(())
        });
        core.run(work).unwrap()
    }
}

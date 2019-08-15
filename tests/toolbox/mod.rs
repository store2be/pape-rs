#![allow(dead_code)]

//! Private utilities for testing Papers.

use futures::channel::mpsc::Sender;
use rand::distributions::{Range, Sample};
use rand::thread_rng;
use std::sync::Arc;
use warp::{filters::BoxedFilter, Filter};

const PRIVATE_PORTS_MIN: u16 = 49_152;
const PRIVATE_PORTS_MAX: u16 = 65_535;

fn random_port() -> u16 {
    Range::new(PRIVATE_PORTS_MIN, PRIVATE_PORTS_MAX).sample(&mut thread_rng())
}

pub struct FilesServer {
    temp_dir: mktemp::Temp,
    port: u16,
    receiver: futures::channel::mpsc::Receiver<(http::method::Method, String)>,
    _thread: std::thread::JoinHandle<()>,
}

pub struct CallbackServer {
    _thread: std::thread::JoinHandle<()>,
    receiver: futures::channel::mpsc::Receiver<(http::method::Method, String)>,
    port: u16,
}

struct PapersServer {
    config: Arc<papers::Config>,
    port: u16,
    _thread: std::thread::JoinHandle<()>,
}

pub struct TestSetup {
    papers_server: PapersServer,
    files_server: Option<FilesServer>,
    callback_server: Option<CallbackServer>,
    client: reqwest::Client,
}

impl TestSetup {
    /// Start with the default test setup config (papers server only).
    pub fn start_default() -> Self {
        Self::start(std::default::Default::default())
    }

    /// Spawns all the services we want to test.
    pub fn start(config: TestSetupConfig) -> Self {
        let TestSetupConfig {
            config,
            serve_files,
            enable_callback_server,
        } = config;

        let papers_port = random_port();
        let config = Arc::new(config.unwrap_or_else(|| papers::Config::for_tests()));
        let config_handle = config.clone();

        let papers_thread_handle = std::thread::spawn(move || {
            warp::serve(papers::app(config_handle)).run(([0, 0, 0, 0], papers_port))
        });

        let files_server = if serve_files {
            let temp_dir = mktemp::Temp::new_dir().unwrap();
            let port = random_port();
            let dir = temp_dir.to_path_buf();
            let (sender, receiver) = futures::channel::mpsc::channel(20);
            let handle = std::thread::spawn(move || {
                let report = reporter(sender);
                let dir = dir.to_str().expect("dir is utf-8");
                let fs = warp::filters::fs::dir(dir.to_owned());
                warp::serve(report().and(fs)).run(([0, 0, 0, 0], port));
            });
            Some(FilesServer {
                temp_dir,
                port,
                receiver,
                _thread: handle,
            })
        } else {
            None
        };

        let callback_server = if enable_callback_server {
            let port = random_port();
            let (sender, receiver) = futures::channel::mpsc::channel(20);
            let handle = std::thread::spawn(move || {
                let report = reporter(sender);
                warp::serve(report().map(|| "OK")).run(([0, 0, 0, 0], port));
            });
            Some(CallbackServer {
                _thread: handle,
                receiver,
                port,
            })
        } else {
            None
        };

        TestSetup {
            client: reqwest::Client::new(),
            callback_server,
            papers_server: PapersServer {
                _thread: papers_thread_handle,
                config,
                port: papers_port,
            },
            files_server,
        }
    }

    /// The path of the temporary directory
    ///
    /// Will panic if the test setup was not configured to serve files (with
    /// [TestSetupConfig.serve_files](TestSetupConfig.serve_files)).
    pub fn files_dir(&self) -> std::path::PathBuf {
        self.files_server
            .as_ref()
            .expect("files server is not set up")
            .temp_dir
            .to_path_buf()
    }

    fn files_server_port(&self) -> u16 {
        self.files_server
            .as_ref()
            .expect("files server is not set up")
            .port
    }

    pub fn files_server_url(&self, path: &str) -> String {
        format!("http://localhost:{}/{}", self.files_server_port(), path)
    }

    fn papers_port(&self) -> u16 {
        self.papers_server.port
    }

    /// Build a URL to the papers server local to this [`TestSetup`](TestSetup).
    pub fn papers_url(&self, path: &str) -> String {
        format!("http://localhost:{}/{}", self.papers_port(), path)
    }

    /// Build a URL to the callback server local to this [`TestSetup`](TestSetup).
    pub fn callback_server_url(&self, path: &str) -> String {
        format!(
            "http://localhost:{}/{}",
            self.callback_server
                .as_ref()
                .expect("callback server is not set up")
                .port,
            path
        )
    }

    pub fn client(&self) -> reqwest::Client {
        self.client.clone()
    }

    pub fn callback_requests(&mut self) -> Vec<(http::Method, String)> {
        let mut msgs = Vec::new();

        while let Some(msg) = self
            .callback_server
            .as_mut()
            .unwrap()
            .receiver
            .try_next()
            .ok()
            .and_then(|o| o)
        {
            msgs.push(msg);
        }

        msgs
    }

    pub fn files_requests(&mut self) -> Vec<(http::Method, String)> {
        let mut msgs = Vec::new();

        while let Some(msg) = self
            .files_server
            .as_mut()
            .unwrap()
            .receiver
            .try_next()
            .ok()
            .and_then(|o| o)
        {
            msgs.push(msg);
        }

        msgs
    }
}

/// By default, you will get: a papers server listening to a random port (see `TestServer.port()`).
///
/// You can also optionally get a client server and a file server to test collaboration with
/// external services.
pub struct TestSetupConfig {
    config: Option<papers::Config>,
    serve_files: bool,
    enable_callback_server: bool,
}

impl TestSetupConfig {
    pub fn default() -> TestSetupConfig {
        TestSetupConfig {
            config: None,
            serve_files: false,
            enable_callback_server: false,
        }
    }

    pub fn set_config(&mut self, config: papers::Config) {
        self.config = Some(config);
    }

    pub fn serve_files(&mut self) {
        self.serve_files = true;
    }

    pub fn enable_callback_server(&mut self) {
        self.enable_callback_server = true;
    }
}

impl std::default::Default for TestSetupConfig {
    fn default() -> TestSetupConfig {
        Self::default()
    }
}

fn reporter(sender: Sender<(http::Method, String)>) -> impl Fn() -> BoxedFilter<()> {
    let sender = Arc::new(std::sync::Mutex::new(sender));
    move || {
        let sender = sender.clone();
        warp::filters::method::method()
            .and(warp::filters::path::full())
            .map(
                move |method: http::Method, full_path: warp::filters::path::FullPath| {
                    sender
                        .clone()
                        .lock()
                        .expect("acquiring sender lock")
                        .try_send((method.to_owned(), full_path.as_str().to_owned()))
                        .expect("sending request in test context");
                },
            )
            .untuple_one()
            .boxed()
    }
}

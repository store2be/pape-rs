#![deny(warnings)]

use dotenv::dotenv;
use std::sync::Arc;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
enum Command {
    #[structopt(name = "server", help = "Start the papers HTTP server")]
    Server,
    #[structopt(name = "local", help = "Produce PDF locally")]
    Local,
    #[structopt(name = "version", help = "Prints the current version of Papers")]
    Version,
    #[structopt(name = "help")]
    Help,
}

#[derive(StructOpt, Debug)]
#[structopt(
    name = "papers",
    about = "A Latex template to PDF generation web service written in Rust."
)]
struct Cli {
    #[structopt(subcommand)]
    command: Option<Command>,
}

fn main() -> Result<(), failure::Error> {
    dotenv().ok();
    pretty_env_logger::init();

    let port = std::env::var("PAPERS_PORT").unwrap_or_else(|_| "8080".to_string());
    let port: std::net::SocketAddr = ([0, 0, 0, 0], port.parse()?).into();

    let sentry_dsn = std::env::var("SENTRY_DSN").unwrap_or_else(|_| "".to_string());
    let _guard = sentry::init(sentry_dsn);
    sentry::integrations::panic::register_panic_handler();

    let opts = Cli::from_args();
    match opts.command {
        Some(Command::Server) | None => {
            warp::serve(papers::app(Arc::new(papers::Config::from_env()))).run(port)
        }
        Some(Command::Local) => papers::local_server::render_locally(),
        Some(Command::Version) => println!(env!("CARGO_PKG_VERSION")),
        Some(Command::Help) => Cli::clap().print_help().unwrap(),
    }

    Ok(())
}

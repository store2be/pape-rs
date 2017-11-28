#![deny(warnings)]

extern crate papers;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
enum Command {
    #[structopt(name = "server", help = "Start the papers HTTP server")]
    Server,
    #[structopt(name = "local", help = "Produce PDF locally")]
    Local,
    #[structopt(name = "help")]
    Help,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "papers", about = "A Latex template to PDF generation web service written in Rust.")]
struct Cli {
    #[structopt(subcommand)]
    command: Option<Command>,
}

fn main() {
    let port = ::std::env::var("PAPERS_PORT").unwrap_or_else(|_| "8080".to_string());
    let opts = Cli::from_args();
    match opts.command {
        Some(Command::Server) | None => {
            papers::server::Server::new()
                .with_port(port.parse().unwrap())
                .start()
                .unwrap()
        }
        Some(Command::Local) => papers::local_server::render_locally(),
        Some(Command::Help) => Cli::clap().print_help().unwrap(),
    }
}

extern crate futures;
extern crate hyper;
extern crate mktemp;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate tera;
extern crate tokio_core;
extern crate tokio_service;

mod error;
mod template;
mod papers;
mod workspace;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}

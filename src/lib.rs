extern crate futures;
extern crate hyper;
extern crate mktemp;
extern crate multipart;
extern crate tera;
extern crate tokio_service;

mod template;
mod pdf_renderer;
mod workspace;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}

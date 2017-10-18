extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
extern crate mime;
extern crate papers;
extern crate rand;
extern crate serde_json as json;
extern crate slog;
extern crate tokio_core;

mod merge;
mod toolbox;

#[test]
fn test_merge() {
    merge::end_to_end::test_end_to_end();
}

extern crate futures;
extern crate mime;
extern crate futures_cpupool;
extern crate hyper;
extern crate slog;
extern crate tokio_core;
extern crate papers;
extern crate rand;
extern crate serde_json as json;

mod merge;
mod toolbox;

#[test]
fn test_merge() {
    merge::end_to_end::test_end_to_end();
}

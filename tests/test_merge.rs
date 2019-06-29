#![feature(async_await)]

mod merge;
mod toolbox;

#[test]
fn test_merge() {
    merge::end_to_end::test_end_to_end();
}

#[test]
fn test_merge_rejection() {
    merge::end_to_end::test_rejection();
}

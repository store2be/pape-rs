#![feature(async_await)]

mod toolbox;

use papers::Config;
use serde_json::json;
use toolbox::*;

#[test]
fn test_assets_exceed_max_length() {
    let config = Config::for_tests().with_max_assets_per_document(1);

    let mut test_setup_config = TestSetupConfig::default();
    test_setup_config.set_config(config);
    let test_setup = TestSetup::start(test_setup_config);

    let document_spec = json!({
        "assets_urls": ["http://127.0.0.1:8733/assets/logo.png", "http://127.0.0.1/8733/dead-end/"],
        "template_url": "http://127.0.0.1:8733/template",
        "callback_url": "http://127.0.0.1:8733/callback",
        "variables": {
            "who": "peter"
        }
    });

    let url = test_setup.papers_url("submit");

    let response = test_setup
        .client()
        .post(&url)
        .json(&document_spec)
        .send()
        .unwrap();

    assert_eq!(response.status(), http::StatusCode::UNPROCESSABLE_ENTITY);
}

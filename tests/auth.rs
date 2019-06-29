#![feature(async_await)]

mod toolbox;

use papers::Config;
use serde_json::json;
use toolbox::*;

fn config_with_auth() -> Config {
    Config::for_tests().with_auth("secret-string".to_string())
}

fn config_empty_auth() -> Config {
    let config = Config::for_tests();
    assert!(config.auth.is_none());
    config
}

#[test]
fn test_submit_ignore_auth_when_not_configured() {
    let mut test_setup_config = TestSetupConfig::default();
    test_setup_config.set_config(config_empty_auth());
    let test_setup = TestSetup::start(test_setup_config);

    let response = test_setup
        .client()
        .post(&test_setup.papers_url("submit"))
        .json(&json!({}))
        .send()
        .unwrap();

    // 422 error code here because the posted DocumentSpec is invalid
    assert_eq!(response.status(), 422);
}

#[test]
fn test_submit_fails_when_auth_is_expected_but_missing() {
    let mut test_setup_config = TestSetupConfig::default();
    test_setup_config.set_config(config_with_auth());
    let test_setup = TestSetup::start(test_setup_config);

    let response = test_setup
        .client()
        .post(&test_setup.papers_url("submit"))
        .json(&json!({}))
        .send()
        .unwrap();

    assert_eq!(response.status(), 403);
}

#[test]
fn test_submit_fails_if_auth_header_does_not_match_env_var() {
    let mut test_setup_config = TestSetupConfig::default();
    test_setup_config.set_config(config_with_auth());
    let test_setup = TestSetup::start(test_setup_config);

    let response = test_setup
        .client()
        .post(&test_setup.papers_url("submit"))
        .header("Authorization", "Bearer other-string")
        .json(&json!({}))
        .send()
        .unwrap();

    assert_eq!(response.status(), 403);
}

#[test]
fn test_submit_succeeds_if_auth_header_matches_env_var() {
    let mut test_setup_config = TestSetupConfig::default();
    test_setup_config.set_config(config_with_auth());
    let test_setup = TestSetup::start(test_setup_config);

    let response = test_setup
        .client()
        .post(&test_setup.papers_url("submit"))
        .header("Authorization", "Bearer secret-string")
        .json(&json!({}))
        .send()
        .unwrap();

    assert_eq!(response.status(), 422);
}

#[test]
fn test_preview_fails_if_auth_header_does_not_match_env_var() {
    let mut test_setup_config = TestSetupConfig::default();
    test_setup_config.set_config(config_with_auth());
    let test_setup = TestSetup::start(test_setup_config);

    let response = test_setup
        .client()
        .post(&test_setup.papers_url("submit"))
        .header("Authorization", "Bearer other-string")
        .json(&json!({}))
        .send()
        .unwrap();

    assert_eq!(response.status(), 403);
}

#[test]
fn test_preview_succeeds_if_auth_header_matches_env_var() {
    let mut test_setup_config = TestSetupConfig::default();
    test_setup_config.set_config(config_with_auth());
    let test_setup = TestSetup::start(test_setup_config);

    let response = test_setup
        .client()
        .post(&test_setup.papers_url("submit"))
        .header("Authorization", "Bearer secret-string")
        .json(&json!({}))
        .send()
        .unwrap();

    assert_eq!(response.status(), 422);
}

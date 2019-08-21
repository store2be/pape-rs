mod toolbox;

use toolbox::*;

#[test]
fn test_health_check() {
    let test_setup = TestSetup::start_default();

    let healthz_url = test_setup.papers_url("healthz");
    let response = test_setup.client().get(&healthz_url).send().unwrap();

    assert_eq!(response.status(), 200);

    let response = test_setup.client().head(&healthz_url).send().unwrap();

    assert_eq!(response.status(), 200);
}

#[test]
fn test_404() {
    let test_setup = TestSetup::start_default();

    let response = test_setup
        .client()
        .get(&test_setup.papers_url("dead-end"))
        .send()
        .unwrap();

    assert_eq!(response.status(), 404);
}

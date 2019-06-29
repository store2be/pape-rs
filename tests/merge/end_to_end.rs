use crate::toolbox::*;

pub fn test_end_to_end() {
    let mut test_config = TestSetupConfig::default();
    test_config.serve_files();
    test_config.enable_callback_server();
    let mut test_setup = TestSetup::start(test_config);

    for file_name in &["logo.png", "doc.pdf"] {
        let origin = std::path::Path::new("tests/assets").join(file_name);
        let destination = test_setup.files_dir().join(file_name);
        std::fs::copy(origin, destination).unwrap();
    }

    let merge_spec = serde_json::json!({
        "assets_urls": vec![
            test_setup.files_server_url("logo.png"),
            test_setup.files_server_url("doc.pdf"),
        ],
        "callback_url": test_setup.callback_server_url("done"),
    });

    let response = test_setup
        .client()
        .post(&test_setup.papers_url("merge"))
        .json(&merge_spec)
        .send()
        .unwrap();

    assert_eq!(response.status(), 200);

    // Leave one second to the background job to finish.
    std::thread::sleep(std::time::Duration::from_secs(1));

    assert_eq!(
        test_setup.callback_requests(),
        vec![(http::Method::POST, "/done".to_owned())],
    );

    // Converting to a set because we don't care about ordering, since files are downloaded in
    // parallel.
    let files_requests: std::collections::HashSet<_> =
        test_setup.files_requests().into_iter().collect();
    let expected: std::collections::HashSet<_> = vec![
        (http::Method::GET, "/logo.png".to_owned()),
        (http::Method::GET, "/doc.pdf".to_owned()),
    ]
    .into_iter()
    .collect();
    assert_eq!(files_requests, expected);
}

pub fn test_rejection() {
    let mut test_config = TestSetupConfig::default();
    test_config.serve_files();
    test_config.enable_callback_server();
    let test_setup = TestSetup::start(test_config);

    for file_name in &["logo.png", "doc.pdf"] {
        let origin = std::path::Path::new("tests/assets").join(file_name);
        let destination = test_setup.files_dir().join(file_name);
        std::fs::copy(origin, destination).unwrap();
    }

    let merge_spec = serde_json::json!({
        "assets_urls": Vec::<String>::new(),
        "callback_url": test_setup.callback_server_url("done"),
    });

    let response = test_setup
        .client()
        .post(&test_setup.papers_url("merge"))
        .json(&merge_spec)
        .send()
        .unwrap();

    assert_eq!(response.status(), 422);
}

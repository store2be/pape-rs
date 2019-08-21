use crate::toolbox::*;
use serde_json::json;

const TEMPLATE: &'static str = r"
\documentclass{article}

\begin{document}
hello, {{who}}

\end{document}
";

pub fn test_end_to_end() {
    use std::io::Write;

    let mut test_config = TestSetupConfig::default();

    test_config.serve_files();
    test_config.enable_callback_server();

    let mut test_setup = TestSetup::start(test_config);

    std::fs::copy(
        std::path::Path::new("tests/assets").join("logo.png"),
        test_setup.files_dir().join("logo.png"),
    )
    .unwrap();

    let mut template_file = std::fs::File::create(test_setup.files_dir().join("template")).unwrap();
    write!(template_file, "{}", TEMPLATE).unwrap();

    // URLs in this test have whitespace left in on purpose to test parsing
    let document_spec = json!({
        "assets_urls": [
            format!(" {}      ", test_setup.files_server_url("logo.png")),
        ],
        "template_url": format!("     {}  ", test_setup.files_server_url("template")),
        "callback_url": format!(" {} ", test_setup.callback_server_url("callback")),
        "variables": {
            "who": "peter"
        }
    });

    let response = test_setup
        .client()
        .post(&test_setup.papers_url("submit"))
        .json(&document_spec)
        .send()
        .unwrap();

    assert_eq!(response.status(), 200);

    // Leave one second to the background job to finish.
    std::thread::sleep(std::time::Duration::from_secs(1));

    let expected_callback_request = &[(http::Method::POST, "/callback".to_owned())];
    assert_eq!(test_setup.callback_requests(), expected_callback_request);

    let expected_files_requests = vec![
        (http::Method::GET, "/template".to_owned()),
        (http::Method::GET, "/logo.png".to_owned()),
    ];
    assert_eq!(test_setup.files_requests(), expected_files_requests);
}

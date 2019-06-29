#![feature(async_await)]

mod toolbox;

use serde_json::json;
use toolbox::*;

static TEMPLATE: &'static str = r"
\documentclass{article}

\begin{document}
hello, {{who}}
\end{document}
";

static EXPECTED_TEMPLATE_RESULT: &'static str = r"
\documentclass{article}

\begin{document}
hello, world
\end{document}
";

#[test]
fn test_simple_template_preview() {
    use std::io::prelude::*;

    let mut test_setup_config = TestSetupConfig::default();
    test_setup_config.serve_files();
    let test_setup = TestSetup::start(test_setup_config);

    let template_file_path = test_setup.files_dir().join("template.tex.tera");
    let mut template_file = std::fs::File::create(template_file_path).unwrap();
    write!(template_file, "{}", TEMPLATE).unwrap();

    let document_spec = json!({
        "template_url": test_setup.files_server_url("template.tex.tera"),
        "callback_url": "/",
        "variables": {
            "who": "world"
        }
    });

    let mut response = test_setup
        .client()
        .post(&test_setup.papers_url("preview"))
        .json(&document_spec)
        .send()
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().unwrap(), EXPECTED_TEMPLATE_RESULT);
}

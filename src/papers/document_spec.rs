use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct DocumentSpec {
    assets_urls: Option<Vec<String>>,
    callback_url: String,
    template_url: String,
    variables: Option<HashMap<String, String>>,
}

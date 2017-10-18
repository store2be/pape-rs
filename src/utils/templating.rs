use latex::escape_tex;
use serde_json::Value;
use std::collections::HashMap;
use tera::{Error, Tera};

fn escape_tex_filter(json: Value, _: HashMap<String, Value>) -> Result<Value, Error> {
    Ok(escape_tex(json))
}

pub fn make_tera() -> Tera {
    let mut tera = Tera::default();
    tera.register_filter("escape_tex", escape_tex_filter);
    tera
}

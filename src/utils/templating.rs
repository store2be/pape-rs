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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_tera_surfaces_working_escape_filter() {

        static TEMPLATE: &'static str = r"
        \documentclass{article}

        \begin{document}
        {{escape_me | escape_tex}}
        {{do_not_escape}}
        \end{document}
        ";

        static EXPECTED_TEMPLATE_RESULT: &'static str = r"
        \documentclass{article}

        \begin{document}
        Brothers \& Sisters 100\% 0.50\$ Ernst is numero \#1 rust\_convention \{or not\}
        % you shall not compile %
        \end{document}
        ";

        let mut tera = make_tera();
        let variables = json!({
            "escape_me": "Brothers & Sisters 100% 0.50$ Ernst is numero #1 rust_convention {or not}",
            "do_not_escape": "% you shall not compile %",
        });
        tera.add_raw_template("template", TEMPLATE)
            .expect("failed to add raw template");
        let rendered_template = tera.render("template", &variables)
            .expect("failed to render the template");
        assert_eq!(rendered_template, EXPECTED_TEMPLATE_RESULT);
    }
}


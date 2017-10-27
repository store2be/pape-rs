use latex::{escape_tex, unescape_tex};
use serde_json::Value;
use std::collections::HashMap;
use tera::{Error, Tera};

fn escape_tex_filter(json: Value, _: HashMap<String, Value>) -> Result<Value, Error> {
    Ok(escape_tex(json))
}

fn unescape_tex_filter(json: Value, _: HashMap<String, Value>) -> Result<Value, Error> {
    Ok(unescape_tex(json))
}

pub fn make_tera() -> Tera {
    let mut tera = Tera::default();
    tera.register_filter("escape_tex", escape_tex_filter);
    tera.register_filter("unescape_tex", unescape_tex_filter);
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
        Bros \& Siss 100\% 0.50\$ Ernst is nr \#1 \{or not\}
        % you shall not compile %
        \end{document}
        ";

        let mut tera = make_tera();
        let variables = json!({
            "escape_me": "Bros & Siss 100% 0.50$ Ernst is nr #1 {or not}",
            "do_not_escape": "% you shall not compile %",
        });
        tera.add_raw_template("template", TEMPLATE)
            .expect("failed to add raw template");
        let rendered_template = tera.render("template", &variables)
            .expect("failed to render the template");
        assert_eq!(rendered_template, EXPECTED_TEMPLATE_RESULT);
    }

    #[test]
    fn make_tera_surfaces_working_unescape_filter() {
        static TEMPLATE: &'static str = r"
        \documentclass{article}

        \begin{document}
        {{unescape_me | unescape_tex}}
        \end{document}
        ";

        static EXPECTED_TEMPLATE_RESULT: &'static str = r"
        \documentclass{article}

        \begin{document}
        Bros & Siss 100% 0.50$ Ernst is nr #1 {or not}
        \end{document}
        ";

        let mut tera = make_tera();
        let variables = json!({
            "unescape_me": "Bros \\& Siss 100\\% 0.50\\$ Ernst is nr \\#1 \\{or not\\}",
        });
        tera.add_raw_template("template", TEMPLATE)
            .expect("failed to add raw template");
        let rendered_template = tera.render("template", &variables)
            .expect("failed to render the template");
        assert_eq!(rendered_template, EXPECTED_TEMPLATE_RESULT);
    }
}

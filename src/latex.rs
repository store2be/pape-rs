use serde_json::Value;
use regex::Regex;

pub fn escape_tex_string(string: &str) -> String {
    lazy_static! {
        static ref LATEX_SPECIAL_CHARACTER: Regex = Regex::new(r"([&%$#_{}])").unwrap();
    }
    LATEX_SPECIAL_CHARACTER.replace_all(string, "\\$1").to_string()
}

pub fn escape_tex(json: Value) -> Value {
    match json {
        Value::String(s) => {
            Value::String(escape_tex_string(&s))
        }
        Value::Object(obj) => {
            Value::Object(obj.into_iter().map(|(k, v)| (k, escape_tex(v))).collect())
        }
        Value::Array(values) => Value::Array(values.into_iter().map(escape_tex).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_tex_does_not_alter_basic_json_strings() {
        let original = json!("abc");
        let expected = json!("abc");
        assert_eq!(escape_tex(original), expected);
    }

    #[test]
    fn escape_tex_does_not_alter_basic_json_numbers() {
        let original = json!(3);
        let expected = json!(3);
        assert_eq!(escape_tex(original), expected);
    }

    #[test]
    fn escape_tex_does_not_alter_basic_json_arrays() {
        let original = json!(["a", 33, null]);
        let expected = json!(["a", 33, null]);
        assert_eq!(escape_tex(original), expected);
    }

    #[test]
    fn escape_tex_escapes_latex_special_characters() {
        let original = json!(
            r##"## Ye$sterday,
            I ate 30% discounted M&M's 90% of the time, & it was {{ _sweet_ }} as hell
            ##"##
        );
        let expected = json!(
            r##"\#\# Ye\$sterday,
            I ate 30\% discounted M\&M's 90\% of the time, \& it was \{\{ \_sweet\_ \}\} as hell
            \#\#"##
        );
        assert_eq!(escape_tex(original), expected);
    }

    #[test]
    fn escape_tex_escapes_strings_in_arrays() {
        let original = json!(["M&Ms", "Ben & Jerry's"]);
        let expected = json!(["M\\&Ms", "Ben \\& Jerry's"]);
        assert_eq!(escape_tex(original), expected);
    }

    #[test]
    fn escape_tex_escapes_recursively_into_the_json_structure() {
        let original = json!({
            "deadEnd": null,
            "names": ["Jack & John", "Santa & Claus"],
            "discounts": {
                "alot": "70%",
                "commented": "33% # or thereabouts",
                "multiplicator": 3.32
            },
        });
        let expected = json!({
            "deadEnd": null,
            "names": ["Jack \\& John", "Santa \\& Claus"],
            "discounts": {
                "alot": "70\\%",
                "commented": "33\\% \\# or thereabouts",
                "multiplicator": 3.32
            },
        });
        assert_eq!(escape_tex(original), expected);
    }
}

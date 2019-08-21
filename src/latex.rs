use regex::Regex;
use serde_json::Value;

pub fn unescape_tex_string(string: &str) -> String {
    let re = Regex::new(r"\\([&%$#_{}])").unwrap();
    re.replace_all(string, "$1").to_string()
}

pub fn escape_tex_string(string: &str) -> String {
    let re = Regex::new(r"([&%$#_{}])").unwrap();
    re.replace_all(string, "\\$1").to_string()
}

pub fn unescape_tex(json: Value) -> Value {
    transform_strings(json, &unescape_tex_string)
}

pub fn escape_tex(json: Value) -> Value {
    transform_strings(json, &escape_tex_string)
}

fn transform_strings(json: Value, callback: &dyn Fn(&str) -> String) -> Value {
    match json {
        Value::String(s) => Value::String(callback(&s)),
        Value::Object(obj) => Value::Object(
            obj.into_iter()
                .map(|(k, v)| (k, transform_strings(v, callback)))
                .collect(),
        ),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|v| transform_strings(v, callback))
                .collect(),
        ),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::*;
    use serde_json::json;

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

    quickcheck! {
        fn escape_tex_and_unescape_tex_roundtrip(input: String) -> bool {
            input == unescape_tex_string(&escape_tex_string(&input))
        }
    }
}

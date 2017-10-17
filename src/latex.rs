use serde_json::Value;
use regex::Regex;

pub fn escape_latex(json: Value) -> Value {
    lazy_static! {
        static ref LATEX_SPECIAL_CHARACTER: Regex = Regex::new(r"([&%$#_{}])").unwrap();
    }
    match json {
        Value::String(s) => Value::String(
            LATEX_SPECIAL_CHARACTER
                .replace_all(&s, "\\$1")
                .to_string()
        ),
        Value::Object(obj) => {
            Value::Object(obj.into_iter().map(|(k, v)| (k, escape_latex(v))).collect())
        },
        Value::Array(values) => Value::Array(values.into_iter().map(escape_latex).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_latex_does_not_alter_basic_json_strings() {
        let original = json!("abc");
        let expected = json!("abc");
        assert_eq!(escape_latex(original), expected);
    }

    #[test]
    fn escape_latex_does_not_alter_basic_json_numbers() {
        let original = json!(3);
        let expected = json!(3);
        assert_eq!(escape_latex(original), expected);
    }

    #[test]
    fn escape_latex_does_not_alter_basic_json_arrays() {
        let original = json!(["a", 33, null]);
        let expected = json!(["a", 33, null]);
        assert_eq!(escape_latex(original), expected);
    }

    #[test]
    fn escape_latex_escapes_latex_special_characters() {
        let original = json!(
            r##"## Ye$sterday,
            I ate 30% discounted M&M's 90% of the time, & it was {{ _sweet_ }} as hell ##"##
        );
        let expected = json!(
            r##"\#\# Ye\$sterday,
            I ate 30\% discounted M\&M's 90\% of the time, \& it was \{\{ \_sweet\_ \}\} as hell \#\#"##
        );
        assert_eq!(escape_latex(original), expected);
    }

    #[test]
    fn escape_latex_escapes_strings_in_arrays() {
        let original = json!(["M&Ms", "Ben & Jerry's"]);
        let expected = json!(["M\\&Ms", "Ben \\& Jerry's"]);
        assert_eq!(escape_latex(original), expected);
    }

    #[test]
    fn escape_latex_escapes_recursively_into_the_json_structure() {
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
        assert_eq!(escape_latex(original), expected);
    }
}

//! `JSON.parse` for the converter's body detection.

use crate::model::Json;

/// `JSON.parse`; `None` when the text is not JSON.
pub fn parse(s: &str) -> Option<Json> {
    serde_json::from_str::<Json>(s).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::js_stringify;

    #[test]
    fn parses_and_prints_like_js() {
        let v = parse(r#"{"b":1.0,"2":[1e21,-0],"1":"x"}"#).unwrap();
        assert_eq!(js_stringify(&v), r#"{"1":"x","2":[1e+21,0],"b":1}"#);
        assert!(parse("{'a':1}").is_none());
    }
}

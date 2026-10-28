//! `when:` conditions (`utils/condition-evaluator.ts`).

use crate::interpolate::Store;
use crate::jsval::{self, JsVal};
use crate::model::{ConditionExpression, Json, WhenCondition};
use crate::store::{get_value_by_path, value_to_string};
use regex::Regex;
use std::sync::LazyLock;

/// A condition expression with JS-typed `right` (a parsed `Infinity` or a
/// string shorthand number cannot live in a JSON value).
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub left: String,
    pub operator: String,
    /// `None` is `undefined`.
    pub right: Option<JsVal>,
    pub case_sensitive: bool,
}

impl From<&ConditionExpression> for Expr {
    fn from(c: &ConditionExpression) -> Self {
        Expr {
            left: c.left.clone(),
            operator: c.operator.clone(),
            right: c.right.as_ref().map(JsVal::from_json),
            case_sensitive: c.case_sensitive.unwrap_or(false),
        }
    }
}

/// TS `{ passed, description }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprResult {
    pub passed: bool,
    pub description: String,
}

/// TS `ConditionResult { shouldRun, reason }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionOutcome {
    pub run: bool,
    pub reason: Option<String>,
}

// `\s` and `.` as JavaScript defines them.
const WS: &str = r"[ \t\n\r\x0B\x0C\x{a0}\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}\x{feff}]";
const DOT: &str = r"[^\n\r\x{2028}\x{2029}]";

static UNARY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"(?i)^({DOT}+?){WS}+(exists|not-exists)$")).expect("unary regex")
});
static BINARY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)^({DOT}+?){WS}*(==|!=|>=|<=|>|<|contains|matches){WS}*({DOT}+)$"
    ))
    .expect("binary regex")
});

/// TS `parseStringCondition`: `"store.status == 200"`, `"store.x exists"`.
pub fn parse_string_condition(condition: &str) -> Option<Expr> {
    let trimmed = jsval::trim(condition);
    if let Some(c) = UNARY.captures(trimmed) {
        return Some(Expr {
            left: jsval::trim(&c[1]).to_string(),
            operator: c[2].to_lowercase(),
            right: None,
            case_sensitive: false,
        });
    }
    let c = BINARY.captures(trimmed)?;
    Some(Expr {
        left: jsval::trim(&c[1]).to_string(),
        operator: c[2].to_lowercase(),
        right: Some(parse_right_value(jsval::trim(&c[3]))),
        case_sensitive: false,
    })
}

fn parse_right_value(value: &str) -> JsVal {
    match value {
        "true" => return JsVal::Bool(true),
        "false" => return JsVal::Bool(false),
        _ => {}
    }
    let num = jsval::str_to_number(value);
    if !num.is_nan() && !value.is_empty() {
        return JsVal::Num(num);
    }
    let quoted = (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''));
    if quoted {
        // JS slice(1, -1); a lone quote char yields ""
        let inner: Vec<char> = value.chars().collect();
        return JsVal::Str(if inner.len() >= 2 {
            inner[1..inner.len() - 1].iter().collect()
        } else {
            String::new()
        });
    }
    JsVal::Str(value.to_string())
}

/// TS `getStoreValue`: store text is parsed back into JSON/number/boolean.
fn store_value(path: &str, store: &Store) -> JsVal {
    let normalized = path.strip_prefix("store.").unwrap_or(path);
    if let Some(value) = store.get(normalized) {
        if value.starts_with('{') || value.starts_with('[') {
            return match serde_json::from_str::<Json>(value) {
                Ok(j) => JsVal::from_json(&j),
                Err(_) => JsVal::Str(value.clone()),
            };
        }
        let num = jsval::str_to_number(value);
        if !num.is_nan() && !value.is_empty() {
            return JsVal::Num(num);
        }
        return match value.as_str() {
            "true" => JsVal::Bool(true),
            "false" => JsVal::Bool(false),
            _ => JsVal::Str(value.clone()),
        };
    }
    let mut parts = normalized.split('.');
    let root = parts.next().unwrap_or_default();
    let Some(root_value) = store.get(root) else {
        return JsVal::Undefined;
    };
    let rest: Vec<&str> = parts.collect();
    match serde_json::from_str::<Json>(root_value) {
        Ok(parsed) => get_value_by_path(&parsed, &rest.join("."))
            .map_or(JsVal::Undefined, |v| JsVal::from_json(&v)),
        Err(_) => JsVal::Undefined,
    }
}

fn format_value(v: &JsVal) -> String {
    match v {
        JsVal::Str(s) => format!("\"{s}\""),
        other => other.to_js_string(),
    }
}

/// `valueToString` over a JS value (undefined/null -> "").
fn val_to_string(v: &JsVal) -> String {
    match v {
        JsVal::Undefined | JsVal::Null => String::new(),
        JsVal::Str(s) => s.clone(),
        JsVal::Num(n) => jsval::num_to_string(*n),
        JsVal::Bool(b) => b.to_string(),
        JsVal::Obj(j) => value_to_string(Some(j)),
    }
}

/// TS `evaluateExpression`.
pub fn evaluate_expression(expr: &Expr, store: &Store) -> ExprResult {
    let left = store_value(&expr.left, store);
    let cs = expr.case_sensitive;
    let right = expr.right.as_ref();
    let description = format!(
        "{} {}{}",
        expr.left,
        expr.operator,
        right.map_or(String::new(), |r| format!(" {}", format_value(r)))
    );
    let undef = JsVal::Undefined;
    let r = right.unwrap_or(&undef);
    let present = !matches!(&left, JsVal::Undefined | JsVal::Null)
        && !matches!(&left, JsVal::Str(s) if s.is_empty());
    let done = |passed: bool| ExprResult {
        passed,
        description: description.clone(),
    };
    let eq = || match (&left, r) {
        (JsVal::Str(a), JsVal::Str(b)) => {
            if cs {
                a == b
            } else {
                a.to_lowercase() == b.to_lowercase()
            }
        }
        _ => left.loose_eq(r),
    };

    match expr.operator.as_str() {
        "exists" => done(present),
        "not-exists" => done(!present),
        "==" => done(eq()),
        "!=" => done(!eq()),
        ">" => done(left.to_number() > r.to_number()),
        "<" => done(left.to_number() < r.to_number()),
        ">=" => done(left.to_number() >= r.to_number()),
        "<=" => done(left.to_number() <= r.to_number()),
        "contains" => {
            let l = val_to_string(&left);
            let needle = if matches!(r, JsVal::Undefined | JsVal::Null) {
                String::new()
            } else {
                r.to_js_string()
            };
            done(if cs {
                l.contains(&needle)
            } else {
                l.to_lowercase().contains(&needle.to_lowercase())
            })
        }
        "matches" => {
            let l = val_to_string(&left);
            let pattern = if matches!(r, JsVal::Undefined | JsVal::Null) {
                String::new()
            } else {
                r.to_js_string()
            };
            match jsval::regex_test(&pattern, &l, !cs) {
                Some(passed) => done(passed),
                None => ExprResult {
                    passed: false,
                    description: format!("{description} (invalid regex)"),
                },
            }
        }
        other => ExprResult {
            passed: false,
            description: format!("unknown operator: {other}"),
        },
    }
}

fn single(expr: &Expr, store: &Store) -> ConditionOutcome {
    let r = evaluate_expression(expr, store);
    ConditionOutcome {
        run: r.passed,
        reason: (!r.passed).then(|| format!("condition not met: {}", r.description)),
    }
}

/// TS `evaluateCondition`: shorthand string, `all`, `any`, or inline
/// `left`/`operator`/`right`; nothing usable means run.
pub fn evaluate(when: &WhenCondition, store: &Store) -> ConditionOutcome {
    let obj = match when {
        WhenCondition::Expr(s) => {
            let Some(expr) = parse_string_condition(s) else {
                return ConditionOutcome {
                    run: false,
                    reason: Some(format!("invalid condition syntax: \"{s}\"")),
                };
            };
            return single(&expr, store);
        }
        WhenCondition::Object(o) => o,
    };
    if let Some(all) = obj.all.as_ref().filter(|a| !a.is_empty()) {
        for e in all {
            let r = evaluate_expression(&Expr::from(e), store);
            if !r.passed {
                return ConditionOutcome {
                    run: false,
                    reason: Some(format!("condition not met: {}", r.description)),
                };
            }
        }
        return ConditionOutcome {
            run: true,
            reason: None,
        };
    }
    if let Some(any) = obj.any.as_ref().filter(|a| !a.is_empty()) {
        let mut descriptions = Vec::new();
        for e in any {
            let r = evaluate_expression(&Expr::from(e), store);
            if r.passed {
                return ConditionOutcome {
                    run: true,
                    reason: None,
                };
            }
            descriptions.push(r.description);
        }
        return ConditionOutcome {
            run: false,
            reason: Some(format!("no conditions met: {}", descriptions.join(", "))),
        };
    }
    match (&obj.left, &obj.operator) {
        (Some(left), Some(op)) if !left.is_empty() && !op.is_empty() => single(
            &Expr {
                left: left.clone(),
                operator: op.clone(),
                right: obj.right.as_ref().map(JsVal::from_json),
                case_sensitive: obj.case_sensitive.unwrap_or(false),
            },
            store,
        ),
        _ => ConditionOutcome {
            run: true,
            reason: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ConditionExpression, WhenObject};
    use serde_json::json;

    fn ctx(pairs: &[(&str, &str)]) -> Store {
        pairs
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect()
    }
    fn p(s: &str) -> Option<Expr> {
        parse_string_condition(s)
    }
    fn expr(left: &str, op: &str, right: Option<JsVal>) -> Expr {
        Expr {
            left: left.into(),
            operator: op.into(),
            right,
            case_sensitive: false,
        }
    }
    fn n(v: f64) -> Option<JsVal> {
        Some(JsVal::Num(v))
    }
    fn st(v: &str) -> Option<JsVal> {
        Some(JsVal::Str(v.into()))
    }

    #[test]
    fn parse_binary_operators() {
        assert_eq!(
            p("store.status == 200"),
            Some(expr("store.status", "==", n(200.0)))
        );
        assert_eq!(
            p("store.type != admin"),
            Some(expr("store.type", "!=", st("admin")))
        );
        assert_eq!(
            p("store.count > 10"),
            Some(expr("store.count", ">", n(10.0)))
        );
        assert_eq!(
            p("store.price < 100.5"),
            Some(expr("store.price", "<", n(100.5)))
        );
        assert_eq!(
            p("store.version >= 2"),
            Some(expr("store.version", ">=", n(2.0)))
        );
        assert_eq!(p("store.age <= 65"), Some(expr("store.age", "<=", n(65.0))));
        assert_eq!(
            p("store.message contains error"),
            Some(expr("store.message", "contains", st("error")))
        );
        assert_eq!(
            p("store.email matches ^[a-z]+@"),
            Some(expr("store.email", "matches", st("^[a-z]+@")))
        );
    }
    #[test]
    fn parse_unary() {
        assert_eq!(
            p("store.userId exists"),
            Some(expr("store.userId", "exists", None))
        );
        assert_eq!(
            p("store.error not-exists"),
            Some(expr("store.error", "not-exists", None))
        );
        assert_eq!(
            p("store.error NOT-EXISTS"),
            Some(expr("store.error", "not-exists", None))
        );
        assert_eq!(
            p("  store.x   Exists  "),
            Some(expr("store.x", "exists", None))
        );
    }
    #[test]
    fn parse_right_values() {
        assert_eq!(
            p("store.enabled == true").unwrap().right,
            Some(JsVal::Bool(true))
        );
        assert_eq!(
            p("store.disabled == false").unwrap().right,
            Some(JsVal::Bool(false))
        );
        assert_eq!(
            p("store.name == \"John Doe\"").unwrap().right,
            st("John Doe")
        );
        assert_eq!(p("store.name == 'John Doe'").unwrap().right, st("John Doe"));
        // quoted number stays a string; unbalanced quote stays literal
        assert_eq!(p("store.a == \"200\"").unwrap().right, st("200"));
        assert_eq!(p("store.a == \"x").unwrap().right, st("\"x"));
        assert_eq!(p("store.a == \"").unwrap().right, st(""));
        // case-sensitive booleans: True is a string
        assert_eq!(p("store.a == True").unwrap().right, st("True"));
        assert_eq!(p("store.a == 0x10").unwrap().right, n(16.0));
        assert_eq!(p("store.a == -5").unwrap().right, n(-5.0));
    }
    #[test]
    fn parse_whitespace_and_case() {
        assert_eq!(
            p("store.status==200"),
            Some(expr("store.status", "==", n(200.0)))
        );
        assert_eq!(
            p("  store.status   ==   200  "),
            Some(expr("store.status", "==", n(200.0)))
        );
        assert_eq!(
            p("store.m CONTAINS x"),
            Some(expr("store.m", "contains", st("x")))
        );
        // lazy left: first operator wins
        assert_eq!(p("a == b == c"), Some(expr("a", "==", st("b == c"))));
        // longer operators first via alternation order
        assert_eq!(p("a >= 1"), Some(expr("a", ">=", n(1.0))));
    }
    #[test]
    fn parse_invalid() {
        assert_eq!(p("invalid"), None);
        assert_eq!(p(""), None);
        assert_eq!(p("store.x"), None);
        assert_eq!(p("== 5"), None);
    }

    fn c1() -> Store {
        ctx(&[
            ("status", "200"),
            ("userId", "123"),
            ("name", "John"),
            ("empty", ""),
            ("enabled", "true"),
            ("count", "42"),
            ("body", r#"{"type":"user","nested":{"id":1}}"#),
        ])
    }
    fn ev(e: Expr) -> bool {
        evaluate_expression(&e, &c1()).passed
    }
    fn evcs(e: Expr) -> bool {
        evaluate_expression(
            &Expr {
                case_sensitive: true,
                ..e
            },
            &c1(),
        )
        .passed
    }

    #[test]
    fn exists_ops() {
        assert!(ev(expr("store.userId", "exists", None)));
        assert!(!ev(expr("store.empty", "exists", None)));
        assert!(!ev(expr("store.missing", "exists", None)));
        assert!(ev(expr("store.missing", "not-exists", None)));
        assert!(ev(expr("store.empty", "not-exists", None)));
        assert!(!ev(expr("store.userId", "not-exists", None)));
        // nested JSON navigation
        assert!(ev(expr("store.body.type", "exists", None)));
        assert!(ev(expr("store.body.nested.id", "exists", None)));
        assert!(!ev(expr("store.body.nope", "exists", None)));
    }
    #[test]
    fn eq_ops() {
        assert!(ev(expr("store.status", "==", n(200.0))));
        assert!(ev(expr("store.name", "==", st("john"))));
        assert!(!evcs(expr("store.name", "==", st("john"))));
        assert!(ev(expr("store.enabled", "==", Some(JsVal::Bool(true)))));
        assert!(ev(expr("store.status", "!=", n(404.0))));
        assert!(!ev(expr("store.status", "!=", n(200.0))));
        // JS loose equality: number store value vs numeric string literal
        assert!(ev(expr("store.status", "==", st("200"))));
        // missing store value is undefined: == null is true, == 0 is false
        assert!(ev(expr("store.missing", "==", Some(JsVal::Null))));
        assert!(!ev(expr("store.missing", "==", n(0.0))));
        // "" parses to the string "" (not a number): == 0 is true under loose eq
        assert!(ev(expr("store.empty", "==", n(0.0))));
        // a true store value equals 1 loosely
        assert!(ev(expr("store.enabled", "==", n(1.0))));
        // right missing: undefined == undefined
        assert!(ev(expr("store.missing", "==", None)));
    }
    #[test]
    fn numeric_compare_ops() {
        assert!(ev(expr("store.count", ">", n(40.0))));
        assert!(ev(expr("store.count", "<", n(50.0))));
        assert!(ev(expr("store.count", ">=", n(42.0))));
        assert!(ev(expr("store.count", "<=", n(42.0))));
        assert!(!ev(expr("store.count", ">", n(42.0))));
        // NaN comparisons are false
        assert!(!ev(expr("store.missing", ">", n(0.0))));
        assert!(!ev(expr("store.name", "<", n(100.0))));
        // string right is coerced
        assert!(ev(expr("store.count", ">", st("40"))));
    }
    #[test]
    fn contains_op() {
        assert!(ev(expr("store.name", "contains", st("OH"))));
        assert!(!evcs(expr("store.name", "contains", st("OH"))));
        assert!(!ev(expr("store.name", "contains", st("xyz"))));
        // objects stringify as JSON
        assert!(ev(expr("store.body", "contains", st("\"type\":\"user\""))));
        // missing is "", right None is ""
        assert!(ev(expr("store.missing", "contains", None)));
    }
    #[test]
    fn matches_op() {
        assert!(ev(expr("store.name", "matches", st("^[A-Z]"))));
        assert!(ev(expr("store.name", "matches", st("^john$"))));
        assert!(!evcs(expr("store.name", "matches", st("^john$"))));
        assert!(!ev(expr("store.name", "matches", st("["))));
        let r = evaluate_expression(&expr("store.name", "matches", st("[")), &c1());
        assert_eq!(r.description, "store.name matches \"[\" (invalid regex)");
        // lookahead allowed (JS regex)
        assert!(ev(expr("store.name", "matches", st("^(?!admin)"))));
        // numeric right is stringified
        assert!(ev(expr("store.status", "matches", n(200.0))));
    }
    #[test]
    fn matches_caps_large_input() {
        let big = Store::from_iter([("body".to_string(), "a".repeat(150_000))]);
        let t = std::time::Instant::now();
        let r = evaluate_expression(&expr("store.body", "matches", st("^a+$")), &big);
        assert!(r.passed);
        assert!(t.elapsed().as_secs() < 5);
    }
    #[test]
    fn unknown_operator_and_descriptions() {
        let r = evaluate_expression(&expr("store.a", "~=", n(1.0)), &c1());
        assert!(!r.passed);
        assert_eq!(r.description, "unknown operator: ~=");
        let r = evaluate_expression(&expr("store.status", "==", n(404.0)), &c1());
        assert_eq!(r.description, "store.status == 404");
        let r = evaluate_expression(&expr("store.name", "==", st("x")), &c1());
        assert_eq!(r.description, "store.name == \"x\"");
        let r = evaluate_expression(&expr("store.name", "==", Some(JsVal::Null)), &c1());
        assert_eq!(r.description, "store.name == null");
        let r = evaluate_expression(&expr("store.name", "exists", None), &c1());
        assert_eq!(r.description, "store.name exists");
    }
    #[test]
    fn store_value_coercion() {
        let s = ctx(&[
            ("n", " "),
            ("t", "false"),
            ("j", "[1,2]"),
            ("bad", "{oops"),
            ("f", "1.50"),
        ]);
        let e =
            |l: &str, op: &str, r: Option<JsVal>| evaluate_expression(&expr(l, op, r), &s).passed;
        // whitespace-only string is Number(" ") === 0
        assert!(e("n", "==", n(0.0)));
        assert!(e("store.t", "==", Some(JsVal::Bool(false))));
        assert!(e("store.j", "contains", st("1,2")));
        assert!(e("store.j.1", "==", n(2.0)));
        assert!(e("store.bad", "==", st("{oops")));
        assert!(e("store.f", "==", n(1.5)));
        assert!(e("store.f", "matches", st("^1.5$")));
    }

    fn wc(c: &Store, w: WhenCondition) -> ConditionOutcome {
        evaluate(&w, c)
    }
    fn c2() -> Store {
        ctx(&[
            ("status", "200"),
            ("userId", "123"),
            ("type", "admin"),
            ("enabled", "true"),
        ])
    }
    fn ce(l: &str, op: &str, r: Option<serde_json::Value>) -> ConditionExpression {
        ConditionExpression {
            left: l.into(),
            operator: op.into(),
            right: r,
            case_sensitive: None,
        }
    }

    #[test]
    fn string_shorthand() {
        let r = wc(&c2(), WhenCondition::Expr("store.status == 200".into()));
        assert_eq!(
            r,
            ConditionOutcome {
                run: true,
                reason: None
            }
        );
        let r = wc(&c2(), WhenCondition::Expr("store.status == 404".into()));
        assert!(!r.run);
        assert_eq!(r.reason.unwrap(), "condition not met: store.status == 404");
        let r = wc(&c2(), WhenCondition::Expr("invalid syntax".into()));
        assert!(!r.run);
        assert_eq!(
            r.reason.unwrap(),
            "invalid condition syntax: \"invalid syntax\""
        );
        // the unquoted shorthand compares text case-insensitively
        assert!(wc(&c2(), WhenCondition::Expr("store.type == ADMIN".into())).run);
    }
    #[test]
    fn single_condition_object() {
        let w = WhenObject {
            left: Some("store.userId".into()),
            operator: Some("exists".into()),
            ..Default::default()
        };
        assert!(wc(&c2(), WhenCondition::Object(w)).run);
        let w = WhenObject {
            left: Some("store.missing".into()),
            operator: Some("exists".into()),
            ..Default::default()
        };
        let r = wc(&c2(), WhenCondition::Object(w));
        assert!(!r.run);
        assert_eq!(r.reason.unwrap(), "condition not met: store.missing exists");
        let w = WhenObject {
            left: Some("store.type".into()),
            operator: Some("==".into()),
            right: Some(json!("ADMIN")),
            case_sensitive: Some(true),
            ..Default::default()
        };
        assert!(!wc(&c2(), WhenCondition::Object(w)).run);
    }
    #[test]
    fn all_conditions() {
        let w = WhenObject {
            all: Some(vec![
                ce("store.status", "==", Some(json!(200))),
                ce("store.userId", "exists", None),
                ce("store.type", "==", Some(json!("admin"))),
            ]),
            ..Default::default()
        };
        assert!(wc(&c2(), WhenCondition::Object(w)).run);
        let w = WhenObject {
            all: Some(vec![
                ce("store.status", "==", Some(json!(200))),
                ce("store.status", "==", Some(json!(404))),
                ce("store.userId", "exists", None),
            ]),
            ..Default::default()
        };
        let r = wc(&c2(), WhenCondition::Object(w));
        assert!(!r.run);
        assert_eq!(r.reason.unwrap(), "condition not met: store.status == 404");
    }
    #[test]
    fn any_conditions() {
        let w = WhenObject {
            any: Some(vec![
                ce("store.status", "==", Some(json!(404))),
                ce("store.status", "==", Some(json!(200))),
                ce("store.status", "==", Some(json!(500))),
            ]),
            ..Default::default()
        };
        assert!(wc(&c2(), WhenCondition::Object(w)).run);
        let w = WhenObject {
            any: Some(vec![
                ce("store.status", "==", Some(json!(404))),
                ce("store.status", "==", Some(json!(500))),
            ]),
            ..Default::default()
        };
        let r = wc(&c2(), WhenCondition::Object(w));
        assert!(!r.run);
        assert_eq!(
            r.reason.unwrap(),
            "no conditions met: store.status == 404, store.status == 500"
        );
    }
    #[test]
    fn edge_cases_default_to_run() {
        assert!(wc(&c2(), WhenCondition::Object(WhenObject::default())).run);
        assert!(
            wc(
                &c2(),
                WhenCondition::Object(WhenObject {
                    all: Some(vec![]),
                    ..Default::default()
                })
            )
            .run
        );
        assert!(
            wc(
                &c2(),
                WhenCondition::Object(WhenObject {
                    any: Some(vec![]),
                    ..Default::default()
                })
            )
            .run
        );
        // left without operator (or empty strings) is not a condition
        let w = WhenObject {
            left: Some("store.x".into()),
            ..Default::default()
        };
        assert!(wc(&c2(), WhenCondition::Object(w)).run);
        let w = WhenObject {
            left: Some(String::new()),
            operator: Some("exists".into()),
            ..Default::default()
        };
        assert!(wc(&c2(), WhenCondition::Object(w)).run);
        // `all` wins over `any`
        let w = WhenObject {
            all: Some(vec![ce("store.status", "==", Some(json!(404)))]),
            any: Some(vec![ce("store.status", "==", Some(json!(200)))]),
            ..Default::default()
        };
        assert!(!wc(&c2(), WhenCondition::Object(w)).run);
    }
    #[test]
    fn nested_json_in_store() {
        let c = ctx(&[("body", r#"{"user":{"id":123,"role":"admin"}}"#)]);
        assert!(
            wc(
                &c,
                WhenCondition::Expr("store.body.user.role == admin".into())
            )
            .run
        );
        assert!(wc(&c, WhenCondition::Expr("store.body.user.id == 123".into())).run);
        assert!(
            !wc(
                &c,
                WhenCondition::Expr("store.body.user.id.x exists".into())
            )
            .run
        );
    }
}

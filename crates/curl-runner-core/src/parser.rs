//! YAML loading and defaults merging (`parser/yaml.ts` + `processYamlFile` in `cli.ts`).

use crate::config::DefaultsLayers;
use crate::model::{GlobalConfig, Json, OutputConfig, RequestConfig, RetryConfig, YamlFile};
use indexmap::IndexMap;
use serde_yaml_ng::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid YAML: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),
    /// Typed deserialization failed; `field` is the path (e.g. `request.timeout`).
    #[error("invalid value at `{field}`: {message}")]
    Invalid { field: String, message: String },
}

/// A loaded file: global config, merged variables and requests with defaults applied.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParsedFile {
    pub global: GlobalConfig,
    /// `global.variables` overlaid with `collection.variables` (for interpolation).
    pub variables: IndexMap<String, String>,
    /// Order: `request`, `requests`, `collection.requests`.
    pub requests: Vec<RequestConfig>,
}

/// A YAML value like `serde_yaml_ng::Value`, except that an integer outside
/// the `u64`/`i64` range becomes an `f64` (what JS reads it as) instead of a
/// parse error. Tags are dropped, as `yaml_to_json` does anyway.
struct Lax(Value);

impl<'de> serde::Deserialize<'de> for Lax {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::Deserialize as _;
        use serde::de::{EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor};
        use serde_yaml_ng::{Mapping, Number};

        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Lax;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("any YAML value")
            }
            fn visit_bool<E>(self, v: bool) -> Result<Lax, E> {
                Ok(Lax(Value::Bool(v)))
            }
            fn visit_i64<E>(self, v: i64) -> Result<Lax, E> {
                Ok(Lax(Value::Number(v.into())))
            }
            fn visit_u64<E>(self, v: u64) -> Result<Lax, E> {
                Ok(Lax(Value::Number(v.into())))
            }
            fn visit_i128<E>(self, v: i128) -> Result<Lax, E> {
                Ok(Lax(Value::Number(Number::from(v as f64))))
            }
            fn visit_u128<E>(self, v: u128) -> Result<Lax, E> {
                Ok(Lax(Value::Number(Number::from(v as f64))))
            }
            fn visit_f64<E>(self, v: f64) -> Result<Lax, E> {
                Ok(Lax(Value::Number(v.into())))
            }
            fn visit_str<E>(self, v: &str) -> Result<Lax, E> {
                Ok(Lax(Value::String(v.to_string())))
            }
            fn visit_string<E>(self, v: String) -> Result<Lax, E> {
                Ok(Lax(Value::String(v)))
            }
            fn visit_unit<E>(self) -> Result<Lax, E> {
                Ok(Lax(Value::Null))
            }
            fn visit_none<E>(self) -> Result<Lax, E> {
                Ok(Lax(Value::Null))
            }
            fn visit_some<D2: serde::Deserializer<'de>>(self, d: D2) -> Result<Lax, D2::Error> {
                Lax::deserialize(d)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Lax, A::Error> {
                let mut out = Vec::new();
                while let Some(Lax(v)) = seq.next_element()? {
                    out.push(v);
                }
                Ok(Lax(Value::Sequence(out)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Lax, A::Error> {
                let mut out = Mapping::new();
                // Duplicate keys: the last value wins (in the first key's
                // position), as with `Bun.YAML.parse` / JS object assignment.
                while let Some((Lax(k), Lax(v))) = map.next_entry()? {
                    out.insert(k, v);
                }
                Ok(Lax(Value::Mapping(out)))
            }
            fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<Lax, A::Error> {
                let (_tag, variant): (serde::de::IgnoredAny, _) = data.variant()?;
                variant.newtype_variant::<Lax>()
            }
        }
        d.deserialize_any(V)
    }
}

/// JS `String(key)` for a YAML mapping key.
fn key_string(k: &Value) -> String {
    match k {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => match n.as_f64() {
            Some(f) if n.is_f64() => crate::jsval::num_to_string(f),
            _ => n.to_string(),
        },
        Value::Tagged(t) => key_string(&t.value),
        // `String([a, b])` is `a,b`; a mapping is `[object Object]`.
        other => crate::jsval::JsVal::from_json(&yaml_to_json(other.clone())).to_js_string(),
    }
}

/// A JS number as JSON. Integral values print as `Number.prototype.toString`
/// does (`1234567890123456789` is `1234567890123456800`): stored as an
/// `i64`/`u64` with those digits when they fit, so every serializer prints
/// them, else as an `f64` (JS-aware printers use `num_to_string`). NaN and
/// ±Infinity have no JSON form: `null`, as `JSON.stringify` prints them.
pub(crate) fn js_number_json(f: f64) -> Json {
    if !f.is_finite() {
        return Json::Null;
    }
    if f.fract() == 0.0 && f.abs() < 1e21 {
        let text = crate::jsval::num_to_string(f);
        if let Ok(i) = text.parse::<i64>() {
            return Json::from(i);
        }
        if let Ok(u) = text.parse::<u64>() {
            return Json::from(u);
        }
    }
    serde_json::Number::from_f64(f).map_or(Json::Null, Json::Number)
}

fn yaml_to_json(v: Value) -> Json {
    match v {
        Value::Null => Json::Null,
        Value::Bool(b) => Json::Bool(b),
        Value::Number(n) => n.as_f64().map_or(Json::Null, js_number_json),
        Value::String(s) => Json::String(s),
        Value::Sequence(seq) => Json::Array(seq.into_iter().map(yaml_to_json).collect()),
        Value::Mapping(m) => Json::Object(
            m.into_iter()
                .map(|(k, v)| (key_string(&k), yaml_to_json(v)))
                .collect(),
        ),
        Value::Tagged(t) => yaml_to_json(t.value),
    }
}

/// Stage 1: YAML text to untyped JSON. Merge keys are applied, non-string
/// mapping keys are stringified like JS (`200` -> `"200"`), empty doc -> `{}`.
pub fn parse_raw(content: &str) -> Result<Json, ParseError> {
    match parse_raw_opt(content)? {
        Json::Null => Ok(Json::Object(Default::default())),
        other => Ok(other),
    }
}

/// [`parse_raw`] that keeps an empty (or `null`) document as `Json::Null`.
pub fn parse_raw_opt(content: &str) -> Result<Json, ParseError> {
    match load_docs(content)?.as_slice() {
        [] => Ok(Json::Null),
        [doc] => Ok(doc.clone()),
        _ => Err(serde_error(content)),
    }
}

/// `Bun.YAML.parse`: like [`parse_raw_opt`], but a multi-document stream
/// yields an array of its documents instead of an error.
pub fn parse_raw_stream(content: &str) -> Result<Json, ParseError> {
    let mut docs = load_docs(content)?;
    Ok(match docs.len() {
        0 => Json::Null,
        1 => docs.pop().unwrap_or(Json::Null),
        _ => Json::Array(docs),
    })
}

/// Every document, with Bun's scalar resolution and merge keys.
fn load_docs(content: &str) -> Result<Vec<Json>, ParseError> {
    use crate::yaml_load::LoadError;
    let docs = crate::yaml_load::load(content).map_err(|e| match e {
        LoadError::Syntax => serde_error(content),
        LoadError::Limit(msg) => {
            ParseError::Yaml(<serde_yaml_ng::Error as serde::de::Error>::custom(msg))
        }
    })?;
    Ok(docs.into_iter().map(yaml_to_json).collect())
}

/// The error `serde_yaml_ng` (same libyaml parser) reports for `content`.
fn serde_error(content: &str) -> ParseError {
    match serde_yaml_ng::from_str::<Lax>(content) {
        Err(e) => ParseError::Yaml(e),
        Ok(_) => ParseError::Yaml(<serde_yaml_ng::Error as serde::de::Error>::custom(
            "invalid YAML",
        )),
    }
}

/// Stage 1 from a file. Variable interpolation (Task 4) runs on this value.
pub fn load_raw(path: &Path) -> Result<Json, ParseError> {
    let content = std::fs::read_to_string(path).map_err(|source| ParseError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    parse_raw(&content)
}

/// Typed `global` block of a raw document only (other sections are ignored).
/// Missing or non-object `global` yields the empty config.
pub fn global_from_raw(raw: &Json) -> Result<GlobalConfig, ParseError> {
    match raw.get("global") {
        Some(g) if g.is_object() => {
            serde_path_to_error::deserialize(g.clone()).map_err(|e| ParseError::Invalid {
                field: format!("global.{}", e.path()),
                message: e.inner().to_string(),
            })
        }
        _ => Ok(GlobalConfig::default()),
    }
}

/// Stage 2: typed deserialization of (possibly interpolated) raw JSON, then
/// defaults/variables merging and `sourceFile` stamping. Numeric fields accept
/// numeric strings, so `timeout: "5000"` works after interpolation; a
/// non-numeric string yields `ParseError::Invalid` naming the field.
pub fn from_raw(raw: Json, source: &Path) -> Result<ParsedFile, ParseError> {
    from_raw_with_defaults(raw, source, &DefaultsLayers::default())
}

/// Like [`from_raw`], with externally supplied default layers. Per request,
/// lowest to highest: `layers.base` < file `global.defaults` <
/// `collection.defaults` < `layers.overrides` (CLI/env) < the request itself.
/// Shallow spread, except `retry`, which merges field-wise across layers.
/// `sourceOutputConfig` is the file's `global.output` overlaid by
/// `layers.output_overrides`. Build `layers` with [`crate::config::defaults_layers`].
pub fn from_raw_with_defaults(
    raw: Json,
    source: &Path,
    layers: &DefaultsLayers,
) -> Result<ParsedFile, ParseError> {
    // Validate the whole document first so errors name the full path
    // (`requests[0].timeout`); the typed value itself is not needed.
    let _: YamlFile =
        serde_path_to_error::deserialize(raw.clone()).map_err(|e| ParseError::Invalid {
            field: e.path().to_string(),
            message: e.inner().to_string(),
        })?;
    let RawParsedFile {
        global,
        variables,
        requests,
    } = prepare_raw(&raw, source, layers, None)?;
    let requests = requests
        .into_iter()
        .enumerate()
        .map(|(i, r)| {
            serde_path_to_error::deserialize(r).map_err(|e| ParseError::Invalid {
                field: format!("request[{i}].{}", e.path()),
                message: e.inner().to_string(),
            })
        })
        .collect::<Result<_, _>>()?;
    Ok(ParsedFile {
        global,
        variables,
        requests,
    })
}

/// A loaded file whose requests stay untyped: each is the merged raw JSON
/// object (defaults applied, file variables interpolated, `sourceFile` and
/// `sourceOutputConfig` stamped). Typed at execution time, after the
/// response-store interpolation (see `interpolate::interpolate_request_json`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RawParsedFile {
    pub global: GlobalConfig,
    /// `global.variables` overlaid with `collection.variables`.
    pub variables: IndexMap<String, String>,
    /// Order: `request`, `requests`, `collection.requests`.
    pub requests: Vec<Json>,
}

/// The parts of `collection` needed before the requests are typed.
#[derive(serde::Deserialize, Default)]
struct CollectionHead {
    #[serde(
        default,
        deserialize_with = "crate::model::serde_util::lenient_opt_map"
    )]
    variables: Option<IndexMap<String, String>>,
    #[serde(default)]
    defaults: Option<RequestConfig>,
}

fn typed_part<T: serde::de::DeserializeOwned + Default>(
    v: Option<&Json>,
    prefix: &str,
) -> Result<T, ParseError> {
    match v {
        Some(v) if v.is_object() => {
            serde_path_to_error::deserialize(v.clone()).map_err(|e| ParseError::Invalid {
                field: format!("{prefix}.{}", e.path()),
                message: e.inner().to_string(),
            })
        }
        _ => Ok(T::default()),
    }
}

/// The run pipeline's loader (TS `processYamlFile` + `prepareRequest`):
/// `global` and the collection's `variables`/`defaults` are typed; each raw
/// request object is interpolated with the file's variables (when `env` is
/// given; `None` skips interpolation) and then merged over the default layers
/// with [`merge_request_json`]. Layer order as in [`from_raw_with_defaults`].
/// Requests stay JSON, so a field holding `${store.x}` survives until
/// execution; a request that cannot be typed fails at execution, not here.
pub fn prepare_raw(
    raw: &Json,
    source: &Path,
    layers: &DefaultsLayers,
    env: Option<crate::interpolate::EnvLookup>,
) -> Result<RawParsedFile, ParseError> {
    let global = global_from_raw(raw)?;
    let collection = raw.get("collection");
    let head: CollectionHead = typed_part(collection, "collection")?;

    let mut variables = global.variables.clone().unwrap_or_default();
    if let Some(v) = &head.variables {
        variables.extend(v.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    // `global.http2` is the lowest-priority source of `defaults.http2`.
    let global_http2 = RequestConfig {
        http2: global.http2,
        ..Default::default()
    };
    let file_defaults = spread_defaults(
        &spread_defaults(
            &spread_defaults(&layers.base, &global_http2),
            &global.defaults.clone().unwrap_or_default(),
        ),
        &head.defaults.unwrap_or_default(),
    );
    let defaults = to_json(&spread_defaults(&file_defaults, &layers.overrides));
    // cli.ts stamps each request with the file's `global.output` (`{}` when
    // absent); CLI/env output settings win over it.
    let file_output = spread_output(
        &global.output.clone().unwrap_or_default(),
        &layers.output_overrides,
    );
    let stamp_output = serde_json::to_value(&file_output).unwrap_or_default();
    let source_file = Json::String(source.to_string_lossy().into_owned());

    let mut items: Vec<&Json> = Vec::new();
    if let Some(r) = raw.get("request").filter(|r| truthy(r)) {
        items.push(r);
    }
    for (key, list) in [
        ("requests", raw.get("requests")),
        (
            "collection.requests",
            collection.and_then(|c| c.get("requests")),
        ),
    ] {
        match list {
            None | Some(Json::Null) => {}
            Some(Json::Array(a)) => items.extend(a),
            Some(other) => {
                return Err(ParseError::Invalid {
                    field: key.into(),
                    message: format!("expected a sequence, got {other}"),
                });
            }
        }
    }

    let ctx = env.map(|env| crate::interpolate::InterpCtx::new(&variables, None, env));
    let requests = items
        .into_iter()
        .map(|r| {
            let r = match &ctx {
                Some(ctx) => crate::interpolate::interpolate_json(r, ctx),
                None => r.clone(),
            };
            let mut m = merge_request_json(&defaults, &r);
            if let Json::Object(o) = &mut m {
                // `--http2` is an explicit command-line choice: it beats the
                // request's own `http2`.
                if let Some(h) = layers.overrides.http2 {
                    o.insert("http2".into(), Json::Bool(h));
                }
                o.insert("sourceFile".into(), source_file.clone());
                o.insert("sourceOutputConfig".into(), stamp_output.clone());
            }
            m
        })
        .collect();

    Ok(RawParsedFile {
        global,
        variables,
        requests,
    })
}

/// JS truthiness of a JSON value.
fn truthy(v: &Json) -> bool {
    match v {
        Json::Null => false,
        Json::Bool(b) => *b,
        Json::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Json::String(s) => !s.is_empty(),
        _ => true,
    }
}

/// A typed request as merge input: `None` fields are omitted, and an empty
/// `url` (the "absent" placeholder) is dropped so it never overrides.
fn to_json(r: &RequestConfig) -> Json {
    let mut v = serde_json::to_value(r).unwrap_or_else(|_| Json::Object(Default::default()));
    if let Json::Object(o) = &mut v
        && o.get("url").is_some_and(|u| u.as_str() == Some(""))
    {
        o.remove("url");
    }
    v
}

/// `{ ...base, ...over }` for one map-valued field (null/absent sides count as `{}`).
fn spread_obj(base: Option<&Json>, over: Option<&Json>) -> Json {
    let mut m = match base {
        Some(Json::Object(b)) => b.clone(),
        _ => Default::default(),
    };
    match over {
        Some(Json::Object(o)) => m.extend(o.iter().map(|(k, v)| (k.clone(), v.clone()))),
        // Not a map: keep it so typing reports the field.
        Some(other) if !other.is_null() => return other.clone(),
        _ => {}
    }
    Json::Object(m)
}

/// Port of `YamlParser.mergeConfigs` on raw JSON, the single implementation
/// of the request merge rules: `{...base, ...override}` with `headers`,
/// `params`, `variables` merged one level deep (always present) and `retry`
/// merged field-wise when either side has one. An explicit `null` in `over`
/// replaces the base value, like a JS spread.
pub fn merge_request_json(base: &Json, over: &Json) -> Json {
    let (Some(b), Some(o)) = (base.as_object(), over.as_object()) else {
        return over.clone();
    };
    let mut out = b.clone();
    out.extend(o.iter().map(|(k, v)| (k.clone(), v.clone())));
    for key in ["headers", "params", "variables"] {
        out.insert(key.into(), spread_obj(b.get(key), o.get(key)));
    }
    let (br, or) = (
        b.get("retry").filter(|v| truthy(v)),
        o.get("retry").filter(|v| truthy(v)),
    );
    if br.is_some() || or.is_some() {
        out.insert("retry".into(), spread_obj(br, or));
    } else {
        out.remove("retry");
    }
    Json::Object(out)
}

/// Port of `YamlParser.mergeConfigs` for typed values (delegates to
/// [`merge_request_json`]).
pub fn merge_configs(base: &RequestConfig, over: &RequestConfig) -> RequestConfig {
    serde_json::from_value(merge_request_json(&to_json(base), &to_json(over)))
        .expect("merging two valid requests yields a valid request")
}

/// Shallow `{...base, ...over}` for `defaults` blocks, with `retry` merged field-wise.
pub(crate) fn spread_defaults(base: &RequestConfig, over: &RequestConfig) -> RequestConfig {
    let mut d = overlay(base, over);
    d.retry = match (&base.retry, &over.retry) {
        (None, None) => None,
        (b, o) => {
            let (b, o) = (b.clone().unwrap_or_default(), o.clone().unwrap_or_default());
            Some(RetryConfig {
                count: o.count.or(b.count),
                delay: o.delay.or(b.delay),
                backoff: o.backoff.or(b.backoff),
                retryable_statuses: o.retryable_statuses.or(b.retryable_statuses),
            })
        }
    };
    d
}

/// Field-wise `{...base, ...over}` for an output block.
pub(crate) fn spread_output(base: &OutputConfig, over: &OutputConfig) -> OutputConfig {
    crate::config::spread!(
        OutputConfig,
        base,
        over,
        [
            verbose,
            quiet,
            show_headers,
            show_body,
            show_metrics,
            format,
            pretty_level,
            save_to_file,
            dry_run
        ]
    )
}

/// Shallow `{...base, ...over}`: a field set in `over` replaces `base`'s.
fn overlay(base: &RequestConfig, over: &RequestConfig) -> RequestConfig {
    macro_rules! pick {
        ($($f:ident),*) => { RequestConfig {
            url: if over.url.is_empty() { base.url.clone() } else { over.url.clone() },
            $($f: over.$f.clone().or_else(|| base.$f.clone()),)*
        } };
    }
    pick!(
        name,
        method,
        headers,
        params,
        source_file,
        body,
        form_data,
        timeout,
        follow_redirects,
        max_redirects,
        auth,
        proxy,
        insecure,
        ssl,
        output,
        http2,
        retry,
        variables,
        store,
        when,
        expect,
        snapshot,
        diff,
        source_output_config
    )
}

/// Parse YAML text and build (no interpolation). `source_file` is stamped on requests.
pub fn load_str(content: &str, source_file: &str) -> Result<ParsedFile, ParseError> {
    from_raw(parse_raw(content)?, Path::new(source_file))
}

/// `load_raw` + `from_raw`, no interpolation.
pub fn load_file(path: &Path) -> Result<ParsedFile, ParseError> {
    from_raw(load_raw(path)?, path)
}

/// `load_raw` + `from_raw_with_defaults`, no interpolation.
pub fn load_file_with_defaults(
    path: &Path,
    layers: &DefaultsLayers,
) -> Result<ParsedFile, ParseError> {
    from_raw_with_defaults(load_raw(path)?, path, layers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use indexmap::IndexMap;

    /// Integers beyond 2^53 are JS numbers: they go on the wire with the
    /// digits `Number.prototype.toString` prints (what `JSON.stringify` of
    /// the `Bun.YAML.parse` result sends), never in float notation.
    #[test]
    fn big_integers_serialize_like_js() {
        let y = "request:\n  url: u\n  body:\n    id: 1234567890123456789\n    a: 9007199254740993\n    n: -1234567890123456789\n    b: 1e21\n    c: 100000000000000000000\n    d: 18446744073709551615\n    e: 0xFFFFFFFFFFFFFFFF\n";
        let r = &load_str(y, "f").unwrap().requests[0];
        let want = r#"{"id":1234567890123456800,"a":9007199254740992,"n":-1234567890123456800,"b":1e+21,"c":100000000000000000000,"d":18446744073709552000,"e":18446744073709552000}"#;
        assert_eq!(crate::transport::easy::body_payload(r).unwrap(), want);
        let raw = parse_raw(y).unwrap();
        assert_eq!(crate::snapshot::js_stringify(&raw["request"]["body"]), want);
        assert_eq!(
            serde_json::to_string(&raw["request"]["body"]["id"]).unwrap(),
            "1234567890123456800"
        );
        assert_eq!(
            serde_json::to_string(&raw["request"]["body"]["n"]).unwrap(),
            "-1234567890123456800"
        );
    }

    /// Deep input is a clean parse error (no stack overflow), and the deepest
    /// accepted value converts and prints on a test thread's small stack.
    #[test]
    fn deep_nesting_errors_cleanly() {
        let n = 100_000;
        let e = parse_raw_stream(&format!("{}{}", "[".repeat(n), "]".repeat(n))).unwrap_err();
        assert!(
            e.to_string()
                .contains("recursion limit exceeded: more than 128 nested levels"),
            "{e}"
        );
        let d = crate::yaml_load::MAX_DEPTH;
        let ok =
            parse_raw_stream(&format!("v: {}{}", "[".repeat(d - 1), "]".repeat(d - 1))).unwrap();
        assert!(crate::snapshot::js_stringify(&ok).starts_with("{\"v\":[[["));
    }

    #[test]
    fn duplicate_keys_last_value_wins() {
        let v = parse_raw("a: 1\nb: 2\na: 3\nc: {x: 1, x: [2]}\n").unwrap();
        assert_eq!(v, serde_json::json!({"a": 3, "b": 2, "c": {"x": [2]}}));
        assert_eq!(
            v.as_object().unwrap().keys().collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
        let r = load_str(
            "request:\n  url: https://a.com\n  url: https://b.com\n",
            "f",
        )
        .unwrap();
        assert_eq!(r.requests[0].url, "https://b.com");
    }

    #[test]
    fn parse_raw_stream_like_bun() {
        assert_eq!(parse_raw_stream("").unwrap(), Json::Null);
        assert_eq!(parse_raw_stream("~").unwrap(), Json::Null);
        assert_eq!(
            parse_raw_stream("a: 1").unwrap(),
            serde_json::json!({"a": 1})
        );
        assert_eq!(
            parse_raw_stream("a: 1\n---\nb: 2\n").unwrap(),
            serde_json::json!([{"a": 1}, {"b": 2}])
        );
        assert!(parse_raw_stream("a: [1").is_err());
    }

    fn req(url: &str) -> RequestConfig {
        RequestConfig {
            url: url.into(),
            method: Some("GET".into()),
            ..Default::default()
        }
    }
    fn map(pairs: &[(&str, &str)]) -> Option<IndexMap<String, String>> {
        Some(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
    }

    #[test]
    fn merges_base_and_override_configs() {
        let base = RequestConfig {
            headers: map(&[("Content-Type", "application/json")]),
            ..Default::default()
        };
        let r = merge_configs(&base, &req("https://api.example.com"));
        assert_eq!(r.url, "https://api.example.com");
        assert_eq!(r.headers, map(&[("Content-Type", "application/json")]));
    }

    #[test]
    fn override_takes_precedence_for_same_keys() {
        let base = RequestConfig {
            headers: map(&[("X-Custom", "base")]),
            timeout: Some(5000),
            ..Default::default()
        };
        let over = RequestConfig {
            headers: map(&[("X-Custom", "override")]),
            ..req("u")
        };
        let r = merge_configs(&base, &over);
        assert_eq!(r.headers.unwrap()["X-Custom"], "override");
        assert_eq!(r.timeout, Some(5000));
    }

    #[test]
    fn merges_nested_objects() {
        let base = RequestConfig {
            params: map(&[("a", "1")]),
            variables: map(&[("x", "base")]),
            ..Default::default()
        };
        let over = RequestConfig {
            params: map(&[("b", "2")]),
            variables: map(&[("y", "override")]),
            ..req("u")
        };
        let r = merge_configs(&base, &over);
        assert_eq!(r.params, map(&[("a", "1"), ("b", "2")]));
        assert_eq!(r.variables, map(&[("x", "base"), ("y", "override")]));
    }

    fn retry(count: u64, delay: Option<u64>, st: Option<Vec<u16>>) -> Option<RetryConfig> {
        Some(RetryConfig {
            count: Some(count),
            delay,
            backoff: None,
            retryable_statuses: st,
        })
    }

    #[test]
    fn deep_merges_retry_config() {
        let base = RequestConfig {
            retry: retry(3, Some(1000), None),
            ..Default::default()
        };
        let over = RequestConfig {
            retry: retry(5, None, None),
            ..req("u")
        };
        assert_eq!(
            merge_configs(&base, &over).retry,
            retry(5, Some(1000), None)
        );
    }

    #[test]
    fn deep_merges_retry_with_retryable_statuses() {
        let base = RequestConfig {
            retry: retry(3, Some(1000), Some(vec![429, 500])),
            ..Default::default()
        };
        let over = RequestConfig {
            retry: retry(2, None, None),
            ..req("u")
        };
        assert_eq!(
            merge_configs(&base, &over).retry,
            retry(2, Some(1000), Some(vec![429, 500]))
        );
    }

    #[test]
    fn retry_only_in_base_or_override() {
        let with = RequestConfig {
            retry: retry(3, Some(500), None),
            ..Default::default()
        };
        assert_eq!(
            merge_configs(&with, &req("u")).retry,
            retry(3, Some(500), None)
        );
        assert_eq!(
            merge_configs(
                &RequestConfig::default(),
                &RequestConfig {
                    retry: retry(2, Some(100), None),
                    ..req("u")
                }
            )
            .retry,
            retry(2, Some(100), None)
        );
    }

    #[test]
    fn explicit_zero_count_wins_and_unset_is_absent() {
        let base = RequestConfig {
            retry: retry(3, Some(10), None),
            ..Default::default()
        };
        let over = RequestConfig {
            retry: retry(0, None, None),
            ..req("u")
        };
        assert_eq!(merge_configs(&base, &over).retry.unwrap().count, Some(0));
        let r = load_str("request: { url: u, retry: { delay: 5 } }", "f").unwrap();
        let rc = r.requests[0].retry.clone().unwrap();
        assert_eq!(rc.count, None);
        assert_eq!(serde_json::to_string(&rc).unwrap(), r#"{"delay":5}"#);
    }

    #[test]
    fn no_retry_in_either() {
        assert_eq!(
            merge_configs(&RequestConfig::default(), &req("u")).retry,
            None
        );
    }

    #[test]
    fn collection_defaults_variables_and_source_file() {
        let y = r#"
global:
  variables: { a: "1", n: 5 }
  defaults: { timeout: 1000, headers: { X-G: g } }
collection:
  name: c
  variables: { a: "2" }
  defaults: { timeout: 2000, headers: { X-C: c } }
  requests:
    - { name: r3, url: "http://x/3" }
request: { name: r1, url: "http://x/1", headers: { X-R: r } }
requests:
  - { name: r2, url: "http://x/2" }
"#;
        let p = load_str(y, "f.yaml").unwrap();
        assert_eq!(p.variables, map(&[("a", "2"), ("n", "5")]).unwrap());
        let names: Vec<_> = p.requests.iter().map(|r| r.name.clone().unwrap()).collect();
        assert_eq!(names, ["r1", "r2", "r3"]);
        assert!(
            p.requests
                .iter()
                .all(|r| r.source_file.as_deref() == Some("f.yaml"))
        );
        // collection defaults shallow-override global defaults (headers replaced, not merged)
        assert_eq!(p.requests[1].timeout, Some(2000));
        assert_eq!(p.requests[1].headers, map(&[("X-C", "c")]));
        assert_eq!(p.requests[0].headers, map(&[("X-C", "c"), ("X-R", "r")]));
        assert_eq!(p.requests[0].params, Some(IndexMap::new()));
    }

    #[test]
    fn unknown_keys_ignored_and_status_untagged() {
        let y =
            "request: { url: u, bogus: 1, expect: { status: [200, 201], extra: x } }\nfoo: bar\n";
        let p = load_str(y, "f").unwrap();
        assert_eq!(
            p.requests[0].expect.as_ref().unwrap().status,
            Some(StatusExpect::Many(vec![200, 201]))
        );
    }

    #[test]
    fn yaml_merge_keys_apply() {
        let y = "x-base: &b { timeout: 7, method: POST }\nrequests:\n  - <<: *b\n    url: u\n";
        let p = load_str(y, "f").unwrap();
        assert_eq!(p.requests[0].timeout, Some(7));
        assert_eq!(p.requests[0].method.as_deref(), Some("POST"));
    }

    #[test]
    fn untagged_unions_parse() {
        let y = r#"
request:
  url: u
  when: "store.status == 200"
  snapshot: true
  diff: { enabled: true }
  formData: { a: x, b: 3, c: true, d: { file: ./f.png, filename: g.png } }
"#;
        let r = &load_str(y, "f").unwrap().requests[0];
        assert!(matches!(r.when, Some(WhenCondition::Expr(_))));
        assert_eq!(r.snapshot, Some(SnapshotSetting::Bool(true)));
        assert!(matches!(r.diff, Some(DiffSetting::Config(_))));
        let fd = r.form_data.as_ref().unwrap();
        assert!(matches!(fd["a"], FormFieldValue::Text(_)));
        assert!(matches!(fd["b"], FormFieldValue::Number(_)));
        assert!(matches!(fd["c"], FormFieldValue::Bool(true)));
        assert!(matches!(fd["d"], FormFieldValue::File(_)));
    }

    #[test]
    fn yaml_1_1_scalars_resolve_like_bun() {
        let y = "request: { url: u, body: { a: yes, b: on, c: no, d: 010, e: 0o10, f: 'yes', g: 0b1 } }";
        let b = load_str(y, "f").unwrap().requests[0].body.clone().unwrap();
        assert_eq!(b["a"], true);
        assert_eq!(b["b"], true);
        assert_eq!(b["c"], false);
        assert_eq!(b["d"], 10); // leading zeros are decimal
        assert_eq!(b["e"], 8);
        assert_eq!(b["f"], "yes"); // quoted stays a string
        assert_eq!(b["g"], "0b1"); // no binary literals in Bun
        let r = load_str("request: { url: u, insecure: yes }", "f").unwrap();
        assert_eq!(r.requests[0].insecure, Some(true));
    }

    /// `tests/fixtures/bun-yaml/scalars.json`: `Bun.YAML.parse` results
    /// (as `JSON.stringify` prints them) for keyword, number, tag, key and
    /// quoting edge cases. Inputs Bun rejects must fail here too.
    #[test]
    fn bun_yaml_parity_table() {
        let rows: Vec<Json> =
            serde_json::from_str(include_str!("../tests/fixtures/bun-yaml/scalars.json")).unwrap();
        let mut bad = Vec::new();
        for row in &rows {
            let yaml = row["yaml"].as_str().unwrap();
            let got = parse_raw_stream(yaml).map(|v| crate::snapshot::js_stringify(&v));
            // Known, logged differences (CHANGES-FROM-TS.md): what Rust gives.
            let known = match yaml {
                // Bun ends the document at a bare `---` / `...` on an
                // unindented line (and errors on `...` at end of input).
                "v: ---" => Some(r#"{"v":"---"}"#),
                "v: ..." => Some(r#"{"v":"..."}"#),
                // Bun: "Multiline implicit key" for a flow-collection key.
                "? [a, b]\n: 1\n? {x: 1}\n: 2\n" => Some(r#"{"a,b":1,"[object Object]":2}"#),
                _ => None,
            };
            if let Some(rust) = known {
                if got.as_deref().ok() != Some(rust) {
                    bad.push(format!("{yaml:?}: known difference changed: rust {got:?}"));
                }
                continue;
            }
            let Some(want) = row["json"].as_str() else {
                // Bun rejects it: Rust must too.
                if got.is_ok() {
                    bad.push(format!("{yaml:?}: bun errors / rust {got:?}"));
                }
                continue;
            };
            if got.as_deref().ok() != Some(want) {
                bad.push(format!("{yaml:?}: bun {want} / rust {got:?}"));
            }
        }
        assert!(
            bad.is_empty(),
            "{} of {} differ:\n{}",
            bad.len(),
            rows.len(),
            bad.join("\n")
        );
    }

    #[test]
    fn invalid_yaml_and_missing_file_error() {
        assert!(matches!(load_str("a: [", "f"), Err(ParseError::Yaml(_))));
        assert!(matches!(
            load_file(Path::new("/nonexistent/x.yaml")),
            Err(ParseError::Io { .. })
        ));
    }

    #[test]
    fn numeric_fields_accept_strings_and_floats() {
        let y = r#"
request:
  url: u
  timeout: " 5000 "
  maxRedirects: 2.7
  retry: { count: "3", delay: 100.0, backoff: "1.5", retryableStatuses: ["429", 500] }
  expect: { status: "200" }
global: { maxConcurrency: "4", ci: { failOnPercentage: "12.5", failOn: 1 }, connectionPool: { keepaliveTime: "7" }, profile: { iterations: "10", warmup: 1 }, watch: { debounce: "300" } }
"#;
        let p = load_str(y, "f").unwrap();
        let r = &p.requests[0];
        assert_eq!(r.timeout, Some(5000));
        assert_eq!(r.max_redirects, Some(2)); // floats truncate
        let rc = r.retry.as_ref().unwrap();
        assert_eq!(
            (rc.count, rc.delay, rc.backoff),
            (Some(3), Some(100), Some(1.5))
        );
        assert_eq!(rc.retryable_statuses, Some(vec![429, 500]));
        assert_eq!(
            r.expect.as_ref().unwrap().status,
            Some(StatusExpect::One(200))
        );
        assert_eq!(p.global.max_concurrency, Some(4));
        assert_eq!(p.global.ci.as_ref().unwrap().fail_on_percentage, Some(12.5));
        assert_eq!(
            p.global.connection_pool.as_ref().unwrap().keepalive_time,
            Some(7)
        );
        assert_eq!(p.global.profile.as_ref().unwrap().iterations, 10);
        assert_eq!(p.global.watch.as_ref().unwrap().debounce, Some(300));
        let s = load_str(
            "request: { url: u, expect: { status: [\"200\", 201.0] } }",
            "f",
        )
        .unwrap();
        assert_eq!(
            s.requests[0].expect.as_ref().unwrap().status,
            Some(StatusExpect::Many(vec![200, 201]))
        );
    }

    #[test]
    fn unresolved_placeholder_errors_naming_field_then_works_after_substitution() {
        let dir = tempfile_dir();
        let path = dir.join("t.yaml");
        std::fs::write(&path, "request:\n  url: u\n  timeout: ${T}\n").unwrap();
        match load_file(&path) {
            Err(ParseError::Invalid { field, message }) => {
                assert_eq!(field, "request.timeout");
                assert!(message.contains("expected a number"), "{message}");
            }
            other => panic!("{other:?}"),
        }
        let mut raw = load_raw(&path).unwrap();
        raw["request"]["timeout"] = Json::String("5000".into());
        assert_eq!(
            from_raw(raw, &path).unwrap().requests[0].timeout,
            Some(5000)
        );
        std::fs::remove_dir_all(dir).ok();
    }

    fn tempfile_dir() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "cr-core-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn non_string_keys_are_stringified() {
        let y = "request: { url: u, body: { 200: ok, true: t, 1.5: f, 2.0: g }, expect: { body: { 404: nf } } }";
        let r = &load_str(y, "f").unwrap().requests[0];
        assert_eq!(
            r.body.as_ref().unwrap().to_string(),
            r#"{"200":"ok","true":"t","1.5":"f","2":"g"}"#
        );
        assert_eq!(
            r.expect.as_ref().unwrap().body.as_ref().unwrap()["404"],
            "nf"
        );
    }

    #[test]
    fn whole_number_floats_become_integers() {
        let raw = parse_raw("a: 2.0\nb: 1.5\nc: [3.0, -4.0]\nd: 1e3").unwrap();
        assert_eq!(raw.to_string(), r#"{"a":2,"b":1.5,"c":[3,-4],"d":1000}"#);
    }

    #[test]
    fn diff_entries_omit_absent_sides_but_keep_null() {
        let d = SnapshotDiff {
            path: "a".into(),
            expected: None,
            received: Some(Json::Null),
            diff_type: "added".into(),
        };
        assert_eq!(
            serde_json::to_string(&d).unwrap(),
            r#"{"path":"a","received":null,"type":"added"}"#
        );
        let back: SnapshotDiff = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(back, d);
        let r = ResponseDiff {
            path: "b".into(),
            baseline: Some(Json::from(1)),
            current: None,
            diff_type: "removed".into(),
        };
        assert_eq!(
            serde_json::to_string(&r).unwrap(),
            r#"{"path":"b","baseline":1,"type":"removed"}"#
        );
    }

    #[test]
    fn empty_file_is_empty() {
        let p = load_str("", "f").unwrap();
        assert!(p.requests.is_empty());
    }

    #[test]
    fn parses_every_example() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/cli/examples");
        let mut n = 0;
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "yaml") {
                load_file(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
                n += 1;
            }
        }
        assert!(n >= 40, "only {n} examples");
    }

    #[test]
    fn round_trip_key_order() {
        let mut r = ExecutionResult::new(req("http://x"), true);
        r.status = Some(200);
        r.body = Some(
            serde_json::from_str(r#"{"z":1,"a":{"y":[1,{"b":2,"a":1}],"b":1.5},"m":null}"#)
                .unwrap(),
        );
        let mut h = IndexMap::new();
        h.insert("z-h".to_string(), HeaderValue::One("1".into()));
        h.insert(
            "a-h".to_string(),
            HeaderValue::Many(vec!["x".into(), "y".into()]),
        );
        r.headers = Some(h);
        r.metrics = Some(Metrics {
            duration: 12.0,
            size: Some(3.5),
            ..Default::default()
        });
        let s = serde_json::to_string(&r).unwrap();
        assert_eq!(
            s,
            r#"{"request":{"url":"http://x","method":"GET"},"success":true,"status":200,"headers":{"z-h":"1","a-h":["x","y"]},"body":{"z":1,"a":{"y":[1,{"b":2,"a":1}],"b":1.5},"m":null},"metrics":{"duration":12,"size":3.5}}"#
        );
        let back: ExecutionResult = serde_json::from_str(&s).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn layers_order_and_retry_field_wise() {
        let yaml = "global:\n  defaults:\n    headers: { X-File: f }\n    timeout: 5000\n    retry: { count: 1, delay: 100 }\n  output:\n    verbose: true\n    prettyLevel: detailed\nrequests:\n  - url: http://a\n  - url: http://b\n    timeout: 1000\n";
        let layers = DefaultsLayers {
            base: RequestConfig {
                headers: map(&[("X-Global", "g")]),
                timeout: Some(1),
                method: Some("POST".into()),
                ..Default::default()
            },
            overrides: RequestConfig {
                timeout: Some(30000),
                retry: Some(RetryConfig {
                    count: Some(3),
                    ..Default::default()
                }),
                ..Default::default()
            },
            output_overrides: OutputConfig {
                pretty_level: Some("minimal".into()),
                ..Default::default()
            },
        };
        let f =
            from_raw_with_defaults(parse_raw(yaml).unwrap(), Path::new("a.yaml"), &layers).unwrap();
        let r = &f.requests[0];
        // file headers replace base headers (shallow spread); base method survives.
        assert_eq!(r.headers, map(&[("X-File", "f")]));
        assert_eq!(r.method.as_deref(), Some("POST"));
        // CLI/env layer beats file `global.defaults`.
        assert_eq!(r.timeout, Some(30000));
        // retry merges field-wise: CLI count 3, file delay 100.
        let retry = r.retry.as_ref().unwrap();
        assert_eq!((retry.count, retry.delay), (Some(3), Some(100)));
        // The request's own value beats everything.
        assert_eq!(f.requests[1].timeout, Some(1000));
        // CLI output beats file `global.output`; other file fields survive.
        let o = r.source_output_config.as_ref().unwrap();
        assert_eq!(o.pretty_level.as_deref(), Some("minimal"));
        assert_eq!(o.verbose, Some(true));
    }

    #[test]
    fn source_output_config_is_empty_object_without_global_output() {
        let f = load_str("request:\n  url: http://a\n", "a.yaml").unwrap();
        assert_eq!(f.requests[0].source_output_config, Some(Default::default()));
    }

    #[test]
    fn prepare_raw_keeps_store_placeholders_and_interpolates_file_variables() {
        let yaml = "global:\n  variables: { HOST: http://h }\n  defaults: { timeout: 5000, headers: { X-D: d } }\ncollection:\n  variables: { P: /p }\n  defaults: { method: POST }\n  requests:\n    - url: \"${HOST}${P}\"\n      timeout: \"${store.t}\"\n      headers: { X-E: \"${E}\" }\n";
        let env = |k: &str| (k == "E").then(|| "env".to_string());
        let f = prepare_raw(
            &parse_raw(yaml).unwrap(),
            Path::new("a.yaml"),
            &DefaultsLayers::default(),
            Some(&env),
        )
        .unwrap();
        let r = &f.requests[0];
        assert_eq!(r["url"], "http://h/p");
        assert_eq!(r["timeout"], "${store.t}");
        assert_eq!(r["method"], "POST");
        assert_eq!(r["headers"], serde_json::json!({"X-D": "d", "X-E": "env"}));
        assert_eq!(r["sourceFile"], "a.yaml");
        assert_eq!(r["sourceOutputConfig"], serde_json::json!({}));
    }

    #[test]
    fn prepare_raw_without_env_does_not_interpolate_and_keeps_bad_types() {
        let yaml = "requests:\n  - url: \"${X}\"\n    timeout: abc\n";
        let f = prepare_raw(
            &parse_raw(yaml).unwrap(),
            Path::new("a"),
            &DefaultsLayers::default(),
            None,
        )
        .unwrap();
        assert_eq!(f.requests[0]["url"], "${X}");
        assert_eq!(f.requests[0]["timeout"], "abc");
        let e = prepare_raw(
            &parse_raw("requests: 5\n").unwrap(),
            Path::new("a"),
            &DefaultsLayers::default(),
            None,
        )
        .unwrap_err();
        assert!(e.to_string().contains("requests"));
    }

    #[test]
    fn merge_request_json_null_replaces_and_retry_merges() {
        let base = serde_json::json!({"timeout": 5, "retry": {"count": 1, "delay": 9}, "url": "b"});
        let over = serde_json::json!({"timeout": null, "retry": {"count": 3}});
        let m = merge_request_json(&base, &over);
        assert_eq!(m["timeout"], Json::Null);
        assert_eq!(m["url"], "b");
        assert_eq!(m["retry"], serde_json::json!({"count": 3, "delay": 9}));
        assert_eq!(m["params"], serde_json::json!({}));
        let none = merge_request_json(&serde_json::json!({}), &serde_json::json!({"url": "u"}));
        assert!(none.get("retry").is_none());
    }
}

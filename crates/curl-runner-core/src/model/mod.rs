//! Serde data model mirroring `packages/cli/src/types/*.ts`.
//!
//! Conventions: snake_case fields with camelCase serde names, every `Option`
//! skipped when `None`, `IndexMap` for every TS `Record` (key order matters),
//! unknown keys ignored (no `deny_unknown_fields`). String unions are kept as
//! `String` because the TS runtime accepts any value.

pub mod diff;
pub mod execution;
pub mod global;
pub mod profile;
pub mod request;
pub mod snapshot;

pub use diff::*;
pub use execution::*;
pub use global::*;
pub use profile::*;
pub use request::*;
pub use snapshot::*;

/// Arbitrary JSON value (TS `JsonValue`).
pub type Json = serde_json::Value;

/// Serde helpers shared by the model.
pub(crate) mod serde_util {
    use super::Json;
    use indexmap::IndexMap;
    use serde::{Deserialize, Deserializer, Serializer};

    /// Render a YAML scalar the way JS `String(value)` would.
    fn scalar_to_string(v: Json) -> String {
        match v {
            Json::String(s) => s,
            Json::Number(n) => match n.as_f64() {
                Some(f) if n.is_f64() && f.fract() == 0.0 && f.abs() < 1e21 => {
                    format!("{}", f as i64)
                }
                _ => n.to_string(),
            },
            Json::Null => "null".to_string(),
            other => other.to_string(),
        }
    }

    /// `Option<Json>` that keeps an explicit `null` as `Some(Json::Null)`
    /// (plain `Option` would collapse it to `None`); absent stays `None`.
    pub fn keep_null<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Json>, D::Error> {
        Json::deserialize(d).map(Some)
    }

    /// Coerce a JSON value to a JS-style number. Accepts numbers and numeric
    /// strings (surrounding whitespace allowed), mirroring `Number(x)` on values
    /// that were interpolated into strings (`timeout: ${T}` -> `"5000"`).
    fn json_to_f64(v: &Json) -> Result<f64, String> {
        let f = match v {
            Json::Number(n) => n.as_f64(),
            Json::String(s) => s
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|f| f.is_finite() && !s.trim().is_empty()),
            _ => None,
        };
        f.ok_or_else(|| format!("expected a number, got {v}"))
    }

    /// Integer fields: whole numbers, numeric strings, and floats. Fractional
    /// values are truncated toward zero (TS passes these through JS arithmetic /
    /// curl args where the fraction is not meaningful); negatives are rejected.
    pub fn json_to_u64(v: Json) -> Result<u64, String> {
        if let Json::Number(n) = &v
            && let Some(u) = n.as_u64()
        {
            return Ok(u);
        }
        let f = json_to_f64(&v)?;
        if f < 0.0 {
            return Err(format!("expected a non-negative number, got {v}"));
        }
        Ok(f.trunc() as u64)
    }

    pub fn json_to_u16(v: Json) -> Result<u16, String> {
        let u = json_to_u64(v.clone())?;
        u16::try_from(u).map_err(|_| format!("number out of range: {v}"))
    }

    pub fn opt_u64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
        match Option::<Json>::deserialize(d)? {
            None => Ok(None),
            Some(v) => json_to_u64(v).map(Some).map_err(serde::de::Error::custom),
        }
    }

    pub fn u64_or_default<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        Ok(opt_u64(d)?.unwrap_or(0))
    }

    pub fn opt_f64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
        match Option::<Json>::deserialize(d)? {
            None => Ok(None),
            Some(v) => json_to_f64(&v).map(Some).map_err(serde::de::Error::custom),
        }
    }

    pub fn opt_vec_u16<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<u16>>, D::Error> {
        match Option::<Vec<Json>>::deserialize(d)? {
            None => Ok(None),
            Some(v) => v
                .into_iter()
                .map(json_to_u16)
                .collect::<Result<_, _>>()
                .map(Some)
                .map_err(serde::de::Error::custom),
        }
    }

    /// Optional string that tolerates numeric/bool YAML scalars (`version: 1.0`).
    pub fn lenient_opt_string<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
        Ok(Option::<Json>::deserialize(d)?.map(scalar_to_string))
    }

    /// `Record<string, string>` tolerating numeric/bool YAML scalars (`x: 5`).
    pub fn lenient_opt_map<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<IndexMap<String, String>>, D::Error> {
        let m = Option::<IndexMap<String, Json>>::deserialize(d)?;
        Ok(m.map(|m| {
            m.into_iter()
                .map(|(k, v)| (k, scalar_to_string(v)))
                .collect()
        }))
    }

    /// Serialize an f64 like `JSON.stringify`: whole numbers without `.0`.
    pub fn ser_f64<S: Serializer>(v: &f64, s: S) -> Result<S::Ok, S::Error> {
        if v.fract() == 0.0 && v.abs() < 9.007_199_254_740_992e15 {
            s.serialize_i64(*v as i64)
        } else {
            s.serialize_f64(*v)
        }
    }

    pub fn ser_opt_f64<S: Serializer>(v: &Option<f64>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(v) => ser_f64(v, s),
            None => s.serialize_none(),
        }
    }

    pub fn ser_vec_f64<S: Serializer>(v: &[f64], s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = s.serialize_seq(Some(v.len()))?;
        for x in v {
            seq.serialize_element(&JsNum(*x))?;
        }
        seq.end()
    }

    struct JsNum(f64);
    impl serde::Serialize for JsNum {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            ser_f64(&self.0, s)
        }
    }
}

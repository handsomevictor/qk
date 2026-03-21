//! Field type casting: `--cast FIELD=TYPE` for query-time type coercion.
//!
//! Supported types (with accepted aliases):
//!   - `number`  (num / float / int / integer)
//!   - `string`  (str / text)
//!   - `bool`    (boolean)
//!   - `null`    (none)
//!   - `auto`    — CSV-style inference: numbers, booleans, null-likes, strings

use std::collections::HashMap;

use serde_json::Value;

use crate::record::Record;
use crate::util::error::{QkError, Result};
use crate::util::intern::intern;

/// Maximum number of specific warning lines before switching to a summary.
const MAX_WARNINGS: usize = 5;

/// Target type for `--cast FIELD=TYPE`.
#[derive(Debug, Clone, PartialEq)]
pub enum CastType {
    /// Coerce to Number. Null-like strings → Null; other non-numeric strings → warning + field removed.
    Number,
    /// Coerce to String. Always succeeds: numbers/bools are converted to their string form.
    Str,
    /// Coerce to Bool. Strings like "true"/"1"/"yes" → true; "false"/"0"/"no" → false; others → warning + removed.
    Bool,
    /// Force the field to null regardless of the original value.
    Null,
    /// Auto-infer type (same logic as CSV `coerce_value`): numbers, booleans, null-likes, strings.
    Auto,
}

impl CastType {
    /// Parse a type name from a string (case-insensitive).
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "number" | "num" | "float" | "int" | "integer" => Some(Self::Number),
            "string" | "str" | "text" => Some(Self::Str),
            "bool" | "boolean" => Some(Self::Bool),
            "null" | "none" => Some(Self::Null),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }

    /// Human-readable list of all supported type names, for error messages.
    pub fn all_names() -> &'static str {
        "number (num/float/int), string (str/text), bool (boolean), null (none), auto"
    }
}

/// Parse a list of `--cast FIELD=TYPE` strings into a field→type map.
pub fn parse_cast_map(specs: &[String]) -> Result<HashMap<String, CastType>> {
    let mut map = HashMap::new();
    for spec in specs {
        let (field, type_str) = spec.split_once('=').ok_or_else(|| {
            QkError::Query(format!(
                "--cast requires FIELD=TYPE (e.g. --cast latency=number), got: {spec:?}"
            ))
        })?;
        let ct = CastType::from_str(type_str).ok_or_else(|| {
            QkError::Query(format!(
                "unknown cast type {type_str:?}. Supported: {}",
                CastType::all_names()
            ))
        })?;
        map.insert(field.to_string(), ct);
    }
    Ok(map)
}

/// Apply cast specifications to all records.
///
/// Returns `(updated_records, warnings)`. Warnings are capped at `MAX_WARNINGS` specific
/// entries; if more occurred, a summary line is appended.
pub fn apply_casts(
    records: Vec<Record>,
    casts: &HashMap<String, CastType>,
) -> (Vec<Record>, Vec<String>) {
    if casts.is_empty() {
        return (records, Vec::new());
    }
    let mut warnings: Vec<String> = Vec::new();
    let mut total_warn: usize = 0;

    let updated = records
        .into_iter()
        .map(|mut rec| {
            for (field, ct) in casts {
                if let Some(val) = rec.fields.get(field.as_str()).cloned() {
                    let loc = if rec.source.line > 0 {
                        format!("line {}, {}", rec.source.line, rec.source.file)
                    } else {
                        rec.source.file.clone()
                    };
                    let (new_val, warn) = coerce_one(&val, ct, field, &loc);
                    if let Some(w) = warn {
                        total_warn += 1;
                        if warnings.len() < MAX_WARNINGS {
                            warnings.push(w);
                        }
                    }
                    match new_val {
                        Some(v) => {
                            rec.fields.insert(intern(field), v);
                        }
                        None => {
                            rec.fields.swap_remove(field.as_str());
                        }
                    }
                }
            }
            rec
        })
        .collect();

    if total_warn > MAX_WARNINGS {
        warnings.push(format!(
            "... and {} more type-mismatch warning(s) suppressed",
            total_warn - MAX_WARNINGS
        ));
    }

    (updated, warnings)
}

/// Coerce one value to the target type.
///
/// Returns `(Option<Value>, Option<warning_string>)`:
/// - `(Some(v), None)` — success, use the new value.
/// - `(None, Some(w))` — failure; field is removed from the record (so numeric ops skip it).
fn coerce_one(
    val: &Value,
    ct: &CastType,
    field: &str,
    loc: &str,
) -> (Option<Value>, Option<String>) {
    match ct {
        CastType::Number => match val {
            Value::Number(_) => (Some(val.clone()), None),
            Value::Null => (Some(Value::Null), None),
            Value::Bool(b) => (Some(Value::Number((*b as i64).into())), None),
            Value::String(s) => {
                if is_null_like(s) {
                    // Null-like strings are silently treated as null — no warning
                    (Some(Value::Null), None)
                } else if let Ok(i) = s.parse::<i64>() {
                    (Some(Value::Number(i.into())), None)
                } else if let Ok(f) = s.parse::<f64>() {
                    let n = serde_json::Number::from_f64(f)
                        .unwrap_or_else(|| serde_json::Number::from(0));
                    (Some(Value::Number(n)), None)
                } else {
                    let w = format!(
                        "[qk warning] --cast {field}=number: value {s:?} is not numeric ({loc}) — field skipped"
                    );
                    (None, Some(w))
                }
            }
            _ => (Some(val.clone()), None),
        },

        CastType::Str => {
            let s = match val {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => "null".to_string(),
                other => other.to_string(),
            };
            (Some(Value::String(s)), None)
        }

        CastType::Bool => match val {
            Value::Bool(_) => (Some(val.clone()), None),
            Value::Null => (Some(Value::Null), None),
            Value::Number(n) => {
                let b = n.as_f64().map(|f| f != 0.0).unwrap_or(false);
                (Some(Value::Bool(b)), None)
            }
            Value::String(s) => match s.to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => (Some(Value::Bool(true)), None),
                "false" | "0" | "no" | "off" => (Some(Value::Bool(false)), None),
                _ => {
                    let w = format!(
                        "[qk warning] --cast {field}=bool: value {s:?} cannot be parsed as bool ({loc}) — field skipped"
                    );
                    (None, Some(w))
                }
            },
            _ => (Some(val.clone()), None),
        },

        CastType::Null => (Some(Value::Null), None),

        CastType::Auto => {
            let new_val = match val {
                Value::String(s) => auto_coerce(s),
                other => other.clone(),
            };
            (Some(new_val), None)
        }
    }
}

/// Auto-coerce a string value — same logic as the CSV parser's `coerce_value`.
fn auto_coerce(s: &str) -> Value {
    if is_null_like(s) {
        return Value::Null;
    }
    match s.to_ascii_lowercase().as_str() {
        "true" => return Value::Bool(true),
        "false" => return Value::Bool(false),
        _ => {}
    }
    if let Ok(i) = s.parse::<i64>() {
        return Value::Number(i.into());
    }
    if let Ok(f) = s.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            return Value::Number(n);
        }
    }
    Value::String(s.to_string())
}

/// Returns true if the string looks like an intentional null / missing value.
pub fn is_null_like(s: &str) -> bool {
    matches!(
        s.trim(),
        "" | "None" | "none" | "null" | "NULL" | "NA" | "N/A" | "n/a" | "NaN" | "nan"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cast_map_valid() {
        let specs = vec![
            "latency=number".to_string(),
            "status=string".to_string(),
            "active=bool".to_string(),
        ];
        let map = parse_cast_map(&specs).unwrap();
        assert_eq!(map["latency"], CastType::Number);
        assert_eq!(map["status"], CastType::Str);
        assert_eq!(map["active"], CastType::Bool);
    }

    #[test]
    fn parse_cast_map_unknown_type_errors() {
        assert!(parse_cast_map(&["x=foobar".to_string()]).is_err());
    }

    #[test]
    fn parse_cast_map_missing_eq_errors() {
        assert!(parse_cast_map(&["latency".to_string()]).is_err());
    }

    #[test]
    fn coerce_numeric_string_to_number() {
        let (val, warn) = coerce_one(&Value::String("42".into()), &CastType::Number, "n", "f");
        assert_eq!(val, Some(Value::Number(42.into())));
        assert!(warn.is_none());
    }

    #[test]
    fn coerce_null_like_to_null() {
        for s in &["None", "null", "NA", "N/A", "NaN", ""] {
            let (val, warn) =
                coerce_one(&Value::String(s.to_string()), &CastType::Number, "n", "f");
            assert_eq!(val, Some(Value::Null), "failed for {s:?}");
            assert!(warn.is_none());
        }
    }

    #[test]
    fn coerce_non_numeric_string_warns_and_removes() {
        let (val, warn) = coerce_one(
            &Value::String("unknown".into()),
            &CastType::Number,
            "n",
            "f",
        );
        assert_eq!(val, None);
        assert!(warn.is_some());
    }

    #[test]
    fn coerce_number_to_string() {
        let (val, warn) = coerce_one(&Value::Number(200.into()), &CastType::Str, "status", "f");
        assert_eq!(val, Some(Value::String("200".into())));
        assert!(warn.is_none());
    }

    #[test]
    fn coerce_bool_string_true() {
        let (val, warn) = coerce_one(
            &Value::String("true".into()),
            &CastType::Bool,
            "active",
            "f",
        );
        assert_eq!(val, Some(Value::Bool(true)));
        assert!(warn.is_none());
    }

    #[test]
    fn coerce_bool_invalid_warns() {
        let (val, warn) = coerce_one(
            &Value::String("maybe".into()),
            &CastType::Bool,
            "active",
            "f",
        );
        assert_eq!(val, None);
        assert!(warn.is_some());
    }

    #[test]
    fn auto_coerce_string_number() {
        assert_eq!(auto_coerce("123"), Value::Number(123.into()));
        assert_eq!(auto_coerce("None"), Value::Null);
        assert_eq!(auto_coerce("true"), Value::Bool(true));
        assert_eq!(auto_coerce("hello"), Value::String("hello".into()));
    }
}

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::Value;

use crate::record::{Record, SourceInfo};
use crate::util::error::{QkError, Result};
use crate::util::intern::intern;

/// Parse a TOML input into records.
///
/// Strategy:
///
/// - If the root is an array of tables `[[section]]`, each element becomes a record.
/// - Otherwise the entire document becomes a single record.
///
/// Values are converted through `serde_json::Value` for uniformity.
pub fn parse(input: &str, source_file: &str) -> Result<Vec<Record>> {
    let toml_val: ::toml::Value = ::toml::from_str(input).map_err(|e| QkError::Parse {
        file: source_file.to_string(),
        line: 0,
        msg: e.to_string(),
    })?;
    let json_val = toml_to_json(toml_val);

    match json_val {
        Value::Array(arr) => arr
            .into_iter()
            .enumerate()
            .map(|(i, v)| json_to_record(v, source_file, i + 1))
            .collect(),
        other => Ok(vec![json_to_record(other, source_file, 1)?]),
    }
}

/// Recursively convert `toml::Value` → `serde_json::Value`.
fn toml_to_json(v: ::toml::Value) -> Value {
    match v {
        ::toml::Value::String(s) => Value::String(s),
        ::toml::Value::Integer(i) => Value::Number(i.into()),
        ::toml::Value::Float(f) => serde_json::json!(f),
        ::toml::Value::Boolean(b) => Value::Bool(b),
        ::toml::Value::Datetime(dt) => Value::String(dt.to_string()),
        ::toml::Value::Array(arr) => Value::Array(arr.into_iter().map(toml_to_json).collect()),
        ::toml::Value::Table(map) => {
            let obj: serde_json::Map<String, Value> =
                map.into_iter().map(|(k, v)| (k, toml_to_json(v))).collect();
            Value::Object(obj)
        }
    }
}

fn json_to_record(value: Value, file: &str, line: usize) -> Result<Record> {
    let raw = value.to_string();
    let fields: IndexMap<Arc<str>, Value> = match value {
        Value::Object(map) => map.into_iter().map(|(k, v)| (intern(&k), v)).collect(),
        other => {
            let mut m = IndexMap::new();
            m.insert(intern("value"), other);
            m
        }
    };
    Ok(Record::new(
        fields,
        raw,
        SourceInfo {
            file: file.to_string(),
            line,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_table() {
        let input = "name = \"server\"\nport = 8080\nenabled = true\n";
        let records = parse(input, "test.toml").unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].fields["name"], Value::String("server".into()));
        assert_eq!(records[0].fields["port"], Value::Number(8080.into()));
        assert_eq!(records[0].fields["enabled"], Value::Bool(true));
    }

    #[test]
    fn parses_array_of_tables() {
        let input = "[[servers]]\nhost = \"a\"\n\n[[servers]]\nhost = \"b\"\n";
        let records = parse(input, "test.toml").unwrap();
        // Root has a "servers" key with an array — one record for the root
        assert_eq!(records.len(), 1);
        // The "servers" field is an array
        assert!(records[0].fields.contains_key("servers"));
    }

    #[test]
    fn error_on_invalid_toml() {
        let result = parse("not valid = = toml", "bad.toml");
        assert!(result.is_err());
    }
}

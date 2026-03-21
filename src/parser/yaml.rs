use std::sync::Arc;

use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use crate::record::{Record, SourceInfo};
use crate::util::error::{QkError, Result};
use crate::util::intern::intern;

/// Parse a YAML input into records.
///
/// Supports multi-document YAML (separated by `---`).
/// Each document is normalised through `serde_json::Value` so the query
/// engine never has to know about `serde_yaml` types.
pub fn parse(input: &str, source_file: &str) -> Result<Vec<Record>> {
    let mut records = Vec::new();
    for (doc_index, doc) in serde_yaml::Deserializer::from_str(input).enumerate() {
        let yaml_val = serde_yaml::Value::deserialize(doc).map_err(|e| QkError::Parse {
            file: source_file.to_string(),
            line: doc_index + 1,
            msg: e.to_string(),
        })?;
        let json_val = yaml_to_json(yaml_val);
        let line = doc_index + 1;
        match json_val {
            Value::Array(arr) => {
                for (i, item) in arr.into_iter().enumerate() {
                    records.push(json_to_record(item, source_file, line + i)?);
                }
            }
            obj => records.push(json_to_record(obj, source_file, line)?),
        }
    }
    Ok(records)
}

/// Recursively convert `serde_yaml::Value` → `serde_json::Value`.
fn yaml_to_json(v: serde_yaml::Value) -> Value {
    match v {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(b) => Value::Bool(b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Number(i.into())
            } else if let Some(f) = n.as_f64() {
                serde_json::json!(f)
            } else {
                Value::Null
            }
        }
        serde_yaml::Value::String(s) => Value::String(s),
        serde_yaml::Value::Sequence(arr) => {
            Value::Array(arr.into_iter().map(yaml_to_json).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let obj: serde_json::Map<String, Value> = map
                .into_iter()
                .map(|(k, v)| (yaml_key_to_string(k), yaml_to_json(v)))
                .collect();
            Value::Object(obj)
        }
        serde_yaml::Value::Tagged(tagged) => yaml_to_json(tagged.value),
    }
}

fn yaml_key_to_string(v: serde_yaml::Value) -> String {
    match v {
        serde_yaml::Value::String(s) => s,
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        _ => String::from("_key"),
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
    fn parses_single_mapping() {
        let yaml = "level: error\nservice: api\n";
        let records = parse(yaml, "test.yaml").unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].fields["level"], Value::String("error".into()));
        assert_eq!(records[0].fields["service"], Value::String("api".into()));
    }

    #[test]
    fn parses_sequence_of_mappings() {
        let yaml = "- level: error\n  service: api\n- level: info\n  service: web\n";
        let records = parse(yaml, "test.yaml").unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].fields["level"], Value::String("error".into()));
        assert_eq!(records[1].fields["level"], Value::String("info".into()));
    }

    #[test]
    fn parses_multi_document_yaml() {
        let yaml = "level: error\n---\nlevel: info\n";
        let records = parse(yaml, "test.yaml").unwrap();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn parses_numeric_fields() {
        let yaml = "status: 503\nlatency: 1200\n";
        let records = parse(yaml, "test.yaml").unwrap();
        assert_eq!(records[0].fields["status"], Value::Number(503.into()));
    }

    #[test]
    fn error_on_invalid_yaml() {
        let result = parse("{bad: yaml: :", "test.yaml");
        assert!(result.is_err());
    }
}

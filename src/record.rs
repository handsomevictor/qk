use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::Value;

/// Source location of a record within an input file.
#[derive(Debug, Clone, Default)]
pub struct SourceInfo {
    pub file: String,
    pub line: usize,
}

/// Unified intermediate representation of a parsed record.
///
/// All parsers produce `Vec<Record>`. The query engine only operates on this type,
/// never on format-specific structures.
///
/// Field keys use `Arc<str>` so that identical field names across many records
/// (e.g. "level", "msg", "ts") share a single heap allocation, reducing memory
/// usage significantly for large files.
#[derive(Debug, Clone)]
pub struct Record {
    /// Ordered key-value fields (preserves insertion order for table output).
    pub fields: IndexMap<Arc<str>, Value>,
    /// Original raw text of the record, if available.
    ///
    /// `None` for synthetic records produced by aggregation. Parsers populate
    /// this so that `--fmt raw` can replay the original bytes.
    pub raw: Option<String>,
    /// Where this record came from.
    pub source: SourceInfo,
}

impl Record {
    /// Create a new record.
    pub fn new(fields: IndexMap<Arc<str>, Value>, raw: Option<String>, source: SourceInfo) -> Self {
        Self {
            fields,
            raw,
            source,
        }
    }

    /// Get a field value, supporting dotted nested access (e.g. `"response.status"`).
    pub fn get(&self, key: &str) -> Option<&Value> {
        if let Some((head, tail)) = key.split_once('.') {
            match self.fields.get(head) {
                Some(Value::Object(map)) => nested_get(map, tail),
                _ => None,
            }
        } else {
            self.fields.get(key)
        }
    }
}

/// Recursively resolve a dotted key path within a JSON object map.
fn nested_get<'a>(map: &'a serde_json::Map<String, Value>, key: &str) -> Option<&'a Value> {
    if let Some((head, tail)) = key.split_once('.') {
        match map.get(head) {
            Some(Value::Object(inner)) => nested_get(inner, tail),
            _ => None,
        }
    } else {
        map.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::intern::intern;

    fn make_record(json: &str) -> Record {
        let v: Value = serde_json::from_str(json).unwrap();
        let fields = match v {
            Value::Object(m) => m.into_iter().map(|(k, v)| (intern(&k), v)).collect(),
            _ => IndexMap::new(),
        };
        Record::new(fields, Some(json.to_string()), SourceInfo::default())
    }

    #[test]
    fn get_top_level_field() {
        let r = make_record(r#"{"level":"error","msg":"timeout"}"#);
        assert_eq!(r.get("level"), Some(&Value::String("error".into())));
    }

    #[test]
    fn get_nested_field() {
        let r = make_record(r#"{"response":{"status":503}}"#);
        assert_eq!(r.get("response.status"), Some(&Value::Number(503.into())));
    }

    #[test]
    fn get_missing_field_returns_none() {
        let r = make_record(r#"{"a":1}"#);
        assert!(r.get("b").is_none());
    }
}

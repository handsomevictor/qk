use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::Value;

use crate::record::{Record, SourceInfo};
use crate::util::error::{QkError, Result};
use crate::util::intern::intern;

/// Parse NDJSON input: one JSON object per line, blank lines ignored.
///
/// Corrupt lines are skipped with a warning on stderr; they do not abort parsing.
pub fn parse(input: &str, source_file: &str) -> Result<Vec<Record>> {
    let mut records = Vec::new();
    for (i, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match parse_line(line, source_file, i + 1) {
            Ok(record) => records.push(record),
            Err(e) => eprintln!("[qk warning] {e}"),
        }
    }
    Ok(records)
}

/// Parse a single NDJSON line into a `Record`. Used by the streaming stdin reader.
pub fn parse_line(line: &str, file: &str, line_num: usize) -> Result<Record> {
    let value: Value = serde_json::from_str(line).map_err(|e| QkError::Parse {
        file: file.to_string(),
        line: line_num,
        msg: e.to_string(),
    })?;
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
        Some(line.to_string()),
        SourceInfo {
            file: file.to_string(),
            line: line_num,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_ndjson() {
        let input =
            "{\"level\":\"error\",\"service\":\"api\"}\n{\"level\":\"info\",\"service\":\"web\"}\n";
        let records = parse(input, "test.ndjson").unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].fields["level"], Value::String("error".into()));
        assert_eq!(records[1].fields["level"], Value::String("info".into()));
    }

    #[test]
    fn skips_blank_lines() {
        let input = "{\"a\":1}\n\n{\"a\":2}\n";
        let records = parse(input, "test.ndjson").unwrap();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn wraps_non_object_in_value_field() {
        let records = parse("42\n", "test.ndjson").unwrap();
        assert_eq!(records[0].fields["value"], Value::Number(42.into()));
    }

    #[test]
    fn corrupt_line_is_skipped_not_aborted() {
        // A single corrupt line must not abort the whole parse.
        // The good line should still be returned.
        let input = "not-json\n{\"ok\":true}\n";
        let records = parse(input, "test.ndjson").unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].fields["ok"], Value::Bool(true));
    }

    #[test]
    fn all_corrupt_returns_empty_vec() {
        // All-corrupt input should produce an empty vec, not an error.
        let result = parse("not-json\nalso-not-json\n", "test.ndjson");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn parse_line_error_includes_line_number() {
        // parse_line() itself still returns Err with a line-number hint.
        let result = parse_line("not-json", "test.ndjson", 5);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("line 5"), "expected line number in: {msg}");
    }
}

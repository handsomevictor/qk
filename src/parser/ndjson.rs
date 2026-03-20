use indexmap::IndexMap;
use serde_json::Value;

use crate::record::{Record, SourceInfo};
use crate::util::error::{QkError, Result};

/// Parse NDJSON input: one JSON object per line, blank lines ignored.
pub fn parse(input: &str, source_file: &str) -> Result<Vec<Record>> {
    let mut records = Vec::new();
    for (i, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let record = parse_line(line, source_file, i + 1)?;
        records.push(record);
    }
    Ok(records)
}

fn parse_line(line: &str, file: &str, line_num: usize) -> Result<Record> {
    let value: Value = serde_json::from_str(line).map_err(|e| QkError::Parse {
        file: file.to_string(),
        line: line_num,
        msg: e.to_string(),
    })?;
    let fields: IndexMap<String, Value> = match value {
        Value::Object(map) => map.into_iter().collect(),
        other => {
            let mut m = IndexMap::new();
            m.insert("value".to_string(), other);
            m
        }
    };
    Ok(Record::new(
        fields,
        line.to_string(),
        SourceInfo { file: file.to_string(), line: line_num },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_ndjson() {
        let input = "{\"level\":\"error\",\"service\":\"api\"}\n{\"level\":\"info\",\"service\":\"web\"}\n";
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
    fn error_includes_line_number() {
        let result = parse("not-json\n", "test.ndjson");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("line 1"), "expected line number in: {msg}");
    }
}

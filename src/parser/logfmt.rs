use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::Value;

use crate::record::{Record, SourceInfo};
use crate::util::error::{QkError, Result};
use crate::util::intern::intern;

/// Parse logfmt input: `key=value` pairs per line, quoted values supported.
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
    let mut fields: IndexMap<Arc<str>, Value> = IndexMap::new();
    let mut remaining = line;

    while !remaining.is_empty() {
        remaining = remaining.trim_start();
        if remaining.is_empty() {
            break;
        }

        let eq_pos = remaining.find('=').ok_or_else(|| QkError::Parse {
            file: file.to_string(),
            line: line_num,
            msg: format!("expected 'key=value', got '{remaining}'"),
        })?;

        let key = remaining[..eq_pos].trim();
        remaining = &remaining[eq_pos + 1..];

        let (value, rest) = parse_value(remaining);
        fields.insert(intern(key), Value::String(value));
        remaining = rest;
    }

    Ok(Record::new(
        fields,
        line.to_string(),
        SourceInfo {
            file: file.to_string(),
            line: line_num,
        },
    ))
}

/// Parse a logfmt value: either a double-quoted string or a bare token.
fn parse_value(s: &str) -> (String, &str) {
    if s.starts_with('"') {
        parse_quoted(s)
    } else {
        parse_bare(s)
    }
}

fn parse_quoted(s: &str) -> (String, &str) {
    let inner = &s[1..]; // skip opening "
    let mut value = String::new();
    let mut chars = inner.char_indices();
    let mut end = inner.len();

    while let Some((i, c)) = chars.next() {
        if c == '\\' {
            if let Some((_, escaped)) = chars.next() {
                value.push(escaped);
            }
        } else if c == '"' {
            end = i + 1;
            break;
        } else {
            value.push(c);
        }
    }
    (value, &inner[end..])
}

fn parse_bare(s: &str) -> (String, &str) {
    let end = s.find(|c: char| c.is_whitespace()).unwrap_or(s.len());
    (s[..end].to_string(), &s[end..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_logfmt() {
        let input = "level=error service=api";
        let records = parse(input, "test.log").unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].fields["level"], Value::String("error".into()));
        assert_eq!(records[0].fields["service"], Value::String("api".into()));
    }

    #[test]
    fn parses_quoted_values() {
        let input = r#"level=info msg="request timeout" latency=250"#;
        let records = parse(input, "test.log").unwrap();
        assert_eq!(
            records[0].fields["msg"],
            Value::String("request timeout".into())
        );
        assert_eq!(records[0].fields["latency"], Value::String("250".into()));
    }

    #[test]
    fn parses_multiple_lines() {
        let input = "level=info svc=a\nlevel=error svc=b\n";
        let records = parse(input, "test.log").unwrap();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn skips_blank_lines() {
        let input = "a=1\n\nb=2\n";
        let records = parse(input, "test.log").unwrap();
        assert_eq!(records.len(), 2);
    }
}

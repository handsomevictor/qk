use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::Value;

use crate::record::{Record, SourceInfo};
use crate::util::error::Result;
use crate::util::intern::intern;

/// Parse plain text: each non-empty line becomes a record with a single `line` field.
pub fn parse(input: &str, source_file: &str) -> Result<Vec<Record>> {
    let records = input
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.is_empty())
        .map(|(i, line)| {
            let mut fields: IndexMap<Arc<str>, Value> = IndexMap::new();
            fields.insert(intern("line"), Value::String(line.to_string()));
            Record::new(
                fields,
                Some(line.to_string()),
                SourceInfo {
                    file: source_file.to_string(),
                    line: i + 1,
                },
            )
        })
        .collect();
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_text() {
        let input = "hello world\nfoo bar\n";
        let records = parse(input, "test.txt").unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(
            records[0].fields["line"],
            Value::String("hello world".into())
        );
        assert_eq!(records[1].fields["line"], Value::String("foo bar".into()));
    }

    #[test]
    fn skips_blank_lines() {
        let input = "a\n\nb\n";
        let records = parse(input, "test.txt").unwrap();
        assert_eq!(records.len(), 2);
    }
}

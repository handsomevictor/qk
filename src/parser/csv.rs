use indexmap::IndexMap;
use serde_json::Value;

use crate::record::{Record, SourceInfo};
use crate::util::error::{QkError, Result};

/// Parse CSV input: first row is the header, subsequent rows are records.
pub fn parse(input: &str, source_file: &str, delimiter: u8) -> Result<Vec<Record>> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .from_reader(input.as_bytes());

    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| QkError::Parse {
            file: source_file.to_string(),
            line: 1,
            msg: e.to_string(),
        })?
        .iter()
        .map(|h| h.to_string())
        .collect();

    let mut records = Vec::new();
    for (i, result) in rdr.records().enumerate() {
        let row = result.map_err(|e| QkError::Parse {
            file: source_file.to_string(),
            line: i + 2,
            msg: e.to_string(),
        })?;
        records.push(build_record(&headers, &row, source_file, i + 2));
    }
    Ok(records)
}

fn build_record(
    headers: &[String],
    row: &csv::StringRecord,
    file: &str,
    line_num: usize,
) -> Record {
    let mut fields: IndexMap<String, Value> = IndexMap::new();
    for (i, cell) in row.iter().enumerate() {
        let key = headers.get(i).map(|s| s.as_str()).unwrap_or("_extra");
        fields.insert(key.to_string(), Value::String(cell.to_string()));
    }
    let raw = row.iter().collect::<Vec<_>>().join(",");
    Record::new(fields, raw, SourceInfo { file: file.to_string(), line: line_num })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_csv() {
        let input = "name,age,city\nalice,30,NYC\nbob,25,LA\n";
        let records = parse(input, "test.csv", b',').unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].fields["name"], Value::String("alice".into()));
        assert_eq!(records[0].fields["age"], Value::String("30".into()));
        assert_eq!(records[1].fields["name"], Value::String("bob".into()));
    }

    #[test]
    fn parses_tsv() {
        let input = "name\tage\nalice\t30\n";
        let records = parse(input, "test.tsv", b'\t').unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].fields["name"], Value::String("alice".into()));
    }

    #[test]
    fn error_on_bad_csv() {
        // Unterminated quote triggers a parse error
        let input = "name,age\n\"unclosed,30\n";
        let result = parse(input, "bad.csv", b',');
        assert!(result.is_err());
    }
}

use indexmap::IndexMap;
use serde_json::Value;

use crate::record::{Record, SourceInfo};
use crate::util::error::{QkError, Result};

/// Parse CSV/TSV input.
///
/// When `no_header` is false (default), the first row is treated as column names.
/// When `no_header` is true, all rows are data and columns are named `col1`, `col2`, ...
pub fn parse(input: &str, source_file: &str, delimiter: u8, no_header: bool) -> Result<Vec<Record>> {
    if no_header {
        parse_headerless(input, source_file, delimiter)
    } else {
        parse_with_header(input, source_file, delimiter)
    }
}

fn parse_with_header(input: &str, source_file: &str, delimiter: u8) -> Result<Vec<Record>> {
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

fn parse_headerless(input: &str, source_file: &str, delimiter: u8) -> Result<Vec<Record>> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .from_reader(input.as_bytes());

    // Collect all rows first to determine column count from the first row
    let mut all_rows: Vec<csv::StringRecord> = Vec::new();
    for (i, result) in rdr.records().enumerate() {
        let row = result.map_err(|e| QkError::Parse {
            file: source_file.to_string(),
            line: i + 1,
            msg: e.to_string(),
        })?;
        all_rows.push(row);
    }

    let col_count = all_rows.first().map(|r| r.len()).unwrap_or(0);
    let headers: Vec<String> = (1..=col_count).map(|n| format!("col{n}")).collect();

    let records = all_rows
        .iter()
        .enumerate()
        .map(|(i, row)| build_record(&headers, row, source_file, i + 1))
        .collect();
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
        fields.insert(key.to_string(), coerce_value(cell));
    }
    let raw = row.iter().collect::<Vec<_>>().join(",");
    Record::new(fields, raw, SourceInfo { file: file.to_string(), line: line_num })
}

/// Attempt to coerce a CSV cell string to the most specific JSON value type.
///
/// - Null-like: `""`, `"None"`, `"null"`, `"NULL"`, `"NA"`, `"N/A"`, `"NaN"` → `Value::Null`
/// - Integer: `"42"` → `Value::Number`
/// - Float: `"3.14"` → `Value::Number`
/// - Boolean: `"true"` / `"false"` (case-insensitive) → `Value::Bool`
/// - Anything else → `Value::String`
fn coerce_value(cell: &str) -> Value {
    match cell.trim() {
        "" | "None" | "none" | "null" | "NULL" | "NA" | "N/A" | "n/a" | "NaN" | "nan" => {
            Value::Null
        }
        s if s.eq_ignore_ascii_case("true") => Value::Bool(true),
        s if s.eq_ignore_ascii_case("false") => Value::Bool(false),
        s => {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_csv_with_header() {
        let input = "name,age,city\nalice,30,NYC\nbob,25,LA\n";
        let records = parse(input, "test.csv", b',', false).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].fields["name"], Value::String("alice".into()));
        assert_eq!(records[0].fields["age"], Value::Number(30.into()));
        assert_eq!(records[1].fields["name"], Value::String("bob".into()));
    }

    #[test]
    fn parses_csv_no_header() {
        let input = "alice,30,NYC\nbob,25,LA\n";
        let records = parse(input, "test.csv", b',', true).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].fields["col1"], Value::String("alice".into()));
        assert_eq!(records[0].fields["col2"], Value::Number(30.into()));
        assert_eq!(records[0].fields["col3"], Value::String("NYC".into()));
        assert_eq!(records[1].fields["col1"], Value::String("bob".into()));
    }

    #[test]
    fn parses_tsv() {
        let input = "name\tage\nalice\t30\n";
        let records = parse(input, "test.tsv", b'\t', false).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].fields["name"], Value::String("alice".into()));
        assert_eq!(records[0].fields["age"], Value::Number(30.into()));
    }

    #[test]
    fn coerces_null_like_values() {
        let input = "name,score\nalice,None\nbob,\ncarol,N/A\n";
        let records = parse(input, "test.csv", b',', false).unwrap();
        assert_eq!(records[0].fields["score"], Value::Null);
        assert_eq!(records[1].fields["score"], Value::Null);
        assert_eq!(records[2].fields["score"], Value::Null);
    }

    #[test]
    fn coerces_numeric_values() {
        let input = "x,y\n42,3.14\n-1,0\n";
        let records = parse(input, "test.csv", b',', false).unwrap();
        assert_eq!(records[0].fields["x"], Value::Number(42.into()));
        assert_eq!(records[1].fields["x"], Value::Number((-1_i64).into()));
    }

    #[test]
    fn error_on_bad_csv() {
        let input = "name,age\n\"unclosed,30\n";
        let result = parse(input, "bad.csv", b',', false);
        assert!(result.is_err());
    }
}

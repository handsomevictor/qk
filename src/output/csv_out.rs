use std::io::Write;

use serde_json::Value;

use crate::record::Record;
use crate::util::error::{QkError, Result};

/// Serialize records back to CSV.
///
/// The header row is the union of all field names across records.
pub fn write(records: &[Record], out: &mut impl Write) -> Result<()> {
    if records.is_empty() {
        return Ok(());
    }

    let headers = collect_headers(records);

    // Header row
    writeln!(out, "{}", headers.join(",")).map_err(io_err)?;

    // Data rows
    for rec in records {
        let row: Vec<String> = headers
            .iter()
            .map(|h| {
                let raw = rec.fields.get(h).map(value_to_csv).unwrap_or_default();
                csv_escape(&raw)
            })
            .collect();
        writeln!(out, "{}", row.join(",")).map_err(io_err)?;
    }
    Ok(())
}

fn collect_headers(records: &[Record]) -> Vec<String> {
    let mut seen = indexmap::IndexSet::new();
    for rec in records {
        for key in rec.fields.keys() {
            seen.insert(key.clone());
        }
    }
    seen.into_iter().collect()
}

fn value_to_csv(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Wrap a CSV cell in double quotes if it contains a comma, double quote, or newline.
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn io_err(e: std::io::Error) -> QkError {
    QkError::Io { path: "<stdout>".to_string(), source: e }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use crate::record::SourceInfo;

    fn make_records(jsons: &[&str]) -> Vec<Record> {
        jsons
            .iter()
            .map(|s| {
                let v: Value = serde_json::from_str(s).unwrap();
                let fields = match v {
                    Value::Object(m) => m.into_iter().collect(),
                    _ => IndexMap::new(),
                };
                Record::new(fields, s.to_string(), SourceInfo::default())
            })
            .collect()
    }

    #[test]
    fn produces_header_and_rows() {
        let records = make_records(&[r#"{"name":"alice","age":"30"}"#]);
        let mut buf = Vec::new();
        write(&records, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        // serde_json without preserve_order uses BTreeMap → alphabetical: age, name
        assert_eq!(lines[0], "age,name");
        assert_eq!(lines[1], "30,alice");
    }

    #[test]
    fn escapes_commas_in_values() {
        let records = make_records(&[r#"{"msg":"hello, world"}"#]);
        let mut buf = Vec::new();
        write(&records, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("\"hello, world\""));
    }

    #[test]
    fn escapes_double_quotes_in_values() {
        let records = make_records(&[r#"{"msg":"say \"hi\""}"#]);
        let mut buf = Vec::new();
        write(&records, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("\"\""));
    }

    #[test]
    fn empty_records_produces_no_output() {
        let mut buf = Vec::new();
        write(&[], &mut buf).unwrap();
        assert!(buf.is_empty());
    }
}

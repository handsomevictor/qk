use std::io::Write;
use std::sync::Arc;

use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use serde_json::Value;

use crate::record::Record;
use crate::util::error::{QkError, Result};

/// Maximum width of a single column, in characters.
const MAX_COL_WIDTH: usize = 60;

/// Render records as a formatted table.
///
/// Column names are the union of all field names across records, in the order
/// they first appear. Missing cells are rendered as empty strings.
pub fn write(records: &[Record], out: &mut impl Write, color: bool) -> Result<()> {
    if records.is_empty() {
        return Ok(());
    }

    let headers = collect_headers(records);
    let table = build_table(&headers, records, color);

    writeln!(out, "{table}").map_err(|e| QkError::Io {
        path: "<stdout>".to_string(),
        source: e,
    })
}

/// Collect the union of all field names, preserving first-seen order.
fn collect_headers(records: &[Record]) -> Vec<Arc<str>> {
    let mut seen = indexmap::IndexSet::new();
    for rec in records {
        for key in rec.fields.keys() {
            seen.insert(Arc::clone(key));
        }
    }
    seen.into_iter().collect()
}

fn build_table(headers: &[Arc<str>], records: &[Record], color: bool) -> Table {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);

    // Header row
    let header_cells: Vec<Cell> = headers
        .iter()
        .map(|h| {
            let cell = Cell::new(h.as_ref()).add_attribute(Attribute::Bold);
            if color {
                cell.fg(Color::Cyan)
            } else {
                cell
            }
        })
        .collect();
    table.set_header(header_cells);

    // Data rows
    for rec in records {
        let cells: Vec<Cell> = headers
            .iter()
            .map(|h| {
                let raw = rec.fields.get(h).map(value_display).unwrap_or_default();
                let truncated = truncate(&raw, MAX_COL_WIDTH);
                if color {
                    colorize_cell(Cell::new(truncated), rec.fields.get(h))
                } else {
                    Cell::new(truncated)
                }
            })
            .collect();
        table.add_row(cells);
    }

    table
}

fn value_display(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max - 1).collect();
        format!("{cut}…")
    }
}

fn colorize_cell(cell: Cell, value: Option<&Value>) -> Cell {
    match value {
        Some(Value::Number(_)) => cell.fg(Color::Blue),
        Some(Value::Bool(_)) => cell.fg(Color::Yellow),
        Some(Value::Null) | None => cell.fg(Color::DarkGrey),
        _ => cell,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::SourceInfo;
    use indexmap::IndexMap;

    fn make_records(jsons: &[&str]) -> Vec<Record> {
        use crate::util::intern::intern;
        jsons
            .iter()
            .map(|s| {
                let v: Value = serde_json::from_str(s).unwrap();
                let fields = match v {
                    Value::Object(m) => m.into_iter().map(|(k, v)| (intern(&k), v)).collect(),
                    _ => IndexMap::new(),
                };
                Record::new(fields, s.to_string(), SourceInfo::default())
            })
            .collect()
    }

    #[test]
    fn empty_records_produces_no_output() {
        let mut buf = Vec::new();
        write(&[], &mut buf, false).unwrap();
        assert!(buf.is_empty());
    }

    #[test]
    fn table_contains_header_and_values() {
        let records = make_records(&[r#"{"level":"error","service":"api"}"#]);
        let mut buf = Vec::new();
        write(&records, &mut buf, false).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("level"));
        assert!(output.contains("service"));
        assert!(output.contains("error"));
        assert!(output.contains("api"));
    }

    #[test]
    fn missing_fields_render_as_empty() {
        let records = make_records(&[
            r#"{"a":"1","b":"2"}"#,
            r#"{"a":"3"}"#, // missing "b"
        ]);
        let mut buf = Vec::new();
        write(&records, &mut buf, false).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("a"));
        assert!(output.contains("b"));
    }

    #[test]
    fn long_values_are_truncated() {
        let long = "x".repeat(MAX_COL_WIDTH + 10);
        let json = format!(r#"{{"field":"{}"}}"#, long);
        let records = make_records(&[&json]);
        let mut buf = Vec::new();
        write(&records, &mut buf, false).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains('…'));
    }

    #[test]
    fn collect_headers_union() {
        use crate::util::intern::intern;
        let records = make_records(&[r#"{"a":1,"b":2}"#, r#"{"a":3,"c":4}"#]);
        let headers = collect_headers(&records);
        assert!(headers.contains(&intern("a")));
        assert!(headers.contains(&intern("b")));
        assert!(headers.contains(&intern("c")));
    }
}

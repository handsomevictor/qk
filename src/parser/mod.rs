use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::Value;

use crate::detect::Format;
use crate::record::{Record, SourceInfo};
use crate::util::error::{QkError, Result};
use crate::util::intern::intern;

pub mod csv;
pub mod logfmt;
pub mod ndjson;
pub mod plaintext;
pub mod toml_fmt;
pub mod yaml;

/// Dispatch to the appropriate parser based on detected format.
///
/// `csv_sep` overrides the field delimiter when the format is (or is treated as)
/// CSV.  When `csv_sep` is `Some(b)`, the input is **always** parsed as
/// delimited text using byte `b`, regardless of the detected format — this
/// lets users handle semicolon- or pipe-separated files that the auto-detector
/// would otherwise classify as plain-text.
pub fn parse(
    input: &str,
    format: &Format,
    source_file: &str,
    no_header: bool,
    csv_sep: Option<u8>,
) -> Result<Vec<Record>> {
    // If the caller supplied an explicit separator, bypass format detection and
    // always parse as delimited text with that byte.
    if let Some(sep) = csv_sep {
        return csv::parse(input, source_file, sep, no_header);
    }

    match format {
        Format::Ndjson => ndjson::parse(input, source_file),
        Format::Json => parse_json_document(input, source_file),
        Format::Csv => csv::parse(input, source_file, b',', no_header),
        Format::Tsv => csv::parse(input, source_file, b'\t', no_header),
        Format::Logfmt => logfmt::parse(input, source_file),
        Format::Yaml => yaml::parse(input, source_file),
        Format::Toml => toml_fmt::parse(input, source_file),
        // Gzip should be decompressed before reaching this point
        Format::Gzip => Err(QkError::UnsupportedFormat(
            "gzip file must be decompressed before parsing".to_string(),
        )),
        Format::Plaintext => plaintext::parse(input, source_file),
    }
}

/// Parse a JSON document.
///
/// Handles three layouts:
/// - Single object: `{ … }`
/// - JSON array: `[ { … }, { … } ]`
/// - Concatenated JSON (multiple top-level objects, pretty-printed or compact):
///   ```text
///   { … }
///   { … }
///   ```
///   This is common when multiple pretty-printed API responses are appended to
///   the same file. `serde_json`'s streaming iterator handles all three cases
///   transparently.
fn parse_json_document(input: &str, source_file: &str) -> Result<Vec<Record>> {
    let stream = serde_json::Deserializer::from_str(input).into_iter::<Value>();
    let mut records: Vec<Record> = Vec::new();
    let mut record_num: usize = 0;

    for result in stream {
        let value = result.map_err(|e| QkError::Parse {
            file: source_file.to_string(),
            line: record_num + 1,
            msg: e.to_string(),
        })?;
        match value {
            Value::Array(arr) => {
                for v in arr {
                    record_num += 1;
                    records.push(object_to_record(v, source_file, record_num)?);
                }
            }
            other => {
                record_num += 1;
                records.push(object_to_record(other, source_file, record_num)?);
            }
        }
    }
    Ok(records)
}

fn object_to_record(value: Value, file: &str, line: usize) -> Result<Record> {
    let raw = value.to_string();
    match value {
        Value::Object(map) => {
            let fields: IndexMap<Arc<str>, Value> =
                map.into_iter().map(|(k, v)| (intern(&k), v)).collect();
            Ok(Record::new(
                fields,
                Some(raw),
                SourceInfo {
                    file: file.to_string(),
                    line,
                },
            ))
        }
        other => Err(QkError::Parse {
            file: file.to_string(),
            line,
            msg: format!("expected JSON object, got {other}"),
        }),
    }
}

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
pub fn parse(
    input: &str,
    format: &Format,
    source_file: &str,
    no_header: bool,
) -> Result<Vec<Record>> {
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

/// Parse a JSON document: either an array of objects or a single object.
fn parse_json_document(input: &str, source_file: &str) -> Result<Vec<Record>> {
    let value: Value = serde_json::from_str(input).map_err(|e| QkError::Parse {
        file: source_file.to_string(),
        line: 1,
        msg: e.to_string(),
    })?;

    match value {
        Value::Array(arr) => arr
            .into_iter()
            .enumerate()
            .map(|(i, v)| object_to_record(v, source_file, i + 1))
            .collect(),
        obj @ Value::Object(_) => Ok(vec![object_to_record(obj, source_file, 1)?]),
        _ => Ok(vec![]),
    }
}

fn object_to_record(value: Value, file: &str, line: usize) -> Result<Record> {
    let raw = value.to_string();
    match value {
        Value::Object(map) => {
            let fields: IndexMap<Arc<str>, Value> =
                map.into_iter().map(|(k, v)| (intern(&k), v)).collect();
            Ok(Record::new(
                fields,
                raw,
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

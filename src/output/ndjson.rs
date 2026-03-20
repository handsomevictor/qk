use std::io::Write;

use crate::record::Record;
use crate::util::error::{QkError, Result};

use super::color;

/// Write records as NDJSON (one JSON object per line) to the given writer.
///
/// When `color` is true, ANSI escape codes are applied for terminal rendering.
pub fn write(records: &[Record], out: &mut impl Write, use_color: bool) -> Result<()> {
    for rec in records {
        let line = if use_color {
            color::paint_record(&rec.fields)
        } else {
            serde_json::to_string(&rec.fields).map_err(|e| QkError::Parse {
                file: rec.source.file.clone(),
                line: rec.source.line,
                msg: e.to_string(),
            })?
        };
        writeln!(out, "{line}").map_err(|e| QkError::Io {
            path: "<stdout>".to_string(),
            source: e,
        })?;
    }
    Ok(())
}

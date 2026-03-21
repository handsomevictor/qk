use std::io::{self, Write};

use crate::cli::OutputFormat;
use crate::record::Record;
use crate::util::error::{QkError, Result};

pub mod color;
pub mod csv_out;
pub mod ndjson;
pub mod pretty;
pub mod table;

/// Render `records` in the requested format to stdout.
pub fn render(records: &[Record], fmt: &OutputFormat, use_color: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    match fmt {
        OutputFormat::Ndjson => ndjson::write(records, &mut out, use_color),
        OutputFormat::Pretty => pretty::write(records, &mut out, use_color),
        OutputFormat::Table => table::write(records, &mut out, use_color),
        OutputFormat::Csv => csv_out::write(records, &mut out),
        OutputFormat::Raw => write_raw(records, &mut out),
    }
}

/// Render a single record to the given writer. Used by the streaming stdin path.
///
/// Formats that require all records up-front (table, csv) fall back to NDJSON
/// in streaming mode and should not be called here — the caller must gate on
/// `is_streaming_compatible_format()` first.
pub fn render_one(
    rec: &Record,
    fmt: &OutputFormat,
    use_color: bool,
    out: &mut impl Write,
) -> Result<()> {
    match fmt {
        OutputFormat::Ndjson => ndjson::write(std::slice::from_ref(rec), out, use_color),
        OutputFormat::Pretty => pretty::write(std::slice::from_ref(rec), out, use_color),
        OutputFormat::Raw => write_raw(std::slice::from_ref(rec), out),
        // Table and CSV need all records — callers must not use streaming for these.
        OutputFormat::Table | OutputFormat::Csv => {
            ndjson::write(std::slice::from_ref(rec), out, use_color)
        }
    }
}

/// Returns true for output formats that can emit records one at a time.
pub fn is_streaming_compatible(fmt: &OutputFormat) -> bool {
    matches!(
        fmt,
        OutputFormat::Ndjson | OutputFormat::Pretty | OutputFormat::Raw
    )
}

fn write_raw(records: &[Record], out: &mut impl Write) -> Result<()> {
    for rec in records {
        let line = rec.raw.as_deref().unwrap_or("");
        writeln!(out, "{line}").map_err(|e| QkError::Io {
            path: "<stdout>".to_string(),
            source: e,
        })?;
    }
    Ok(())
}

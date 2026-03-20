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

fn write_raw(records: &[Record], out: &mut impl Write) -> Result<()> {
    for rec in records {
        writeln!(out, "{}", rec.raw).map_err(|e| QkError::Io {
            path: "<stdout>".to_string(),
            source: e,
        })?;
    }
    Ok(())
}

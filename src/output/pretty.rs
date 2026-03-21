use std::io::Write;

use owo_colors::{OwoColorize, Style};

use crate::record::Record;
use crate::util::error::{QkError, Result};

/// Write records as pretty-printed indented JSON, with a blank line between each.
///
/// This replaces the common pattern of piping through `jq .`.
pub fn write(records: &[Record], out: &mut impl Write, use_color: bool) -> Result<()> {
    for (i, rec) in records.iter().enumerate() {
        if i > 0 {
            writeln!(out).map_err(io_err)?;
        }
        let json = serde_json::to_string_pretty(&rec.fields)
            .map_err(|e| QkError::Query(format!("failed to serialise record: {e}")))?;
        if use_color {
            let coloured = colorise_pretty(&json);
            writeln!(out, "{coloured}").map_err(io_err)?;
        } else {
            writeln!(out, "{json}").map_err(io_err)?;
        }
    }
    Ok(())
}

/// Apply light ANSI colouring to an already-indented JSON string.
///
/// Coloring rules (same palette as `color::paint_record`):
/// - Object keys → bold cyan
/// - String values → green
/// - Number values → yellow
/// - Boolean values → magenta
/// - null → dim
fn colorise_pretty(json: &str) -> String {
    let mut out = String::with_capacity(json.len() + json.len() / 2);
    for line in json.lines() {
        out.push_str(&colour_line(line));
        out.push('\n');
    }
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

fn colour_line(line: &str) -> String {
    let indent_len = line.len() - line.trim_start().len();
    let indent = &line[..indent_len];

    if let Some(colon_pos) = find_key_colon(line) {
        // colon_pos is the index of `:` in the original line
        let key_raw = line[indent_len..colon_pos].trim_matches('"');
        let key_coloured = format!("\"{}\"", key_raw.style(Style::new().bold().cyan()));
        let value_raw = line[colon_pos + 1..].trim_start();
        let value_coloured = colour_value_str(value_raw);
        format!("{indent}{key_coloured}: {value_coloured}")
    } else {
        // Pure structural line ({, }, [, ]) or bare value inside array
        let trimmed = line.trim_start();
        let value_coloured = colour_value_str(trimmed);
        format!("{indent}{value_coloured}")
    }
}

/// Colour a JSON value token (possibly with trailing comma).
fn colour_value_str(s: &str) -> String {
    let (core, suffix) = if let Some(stripped) = s.strip_suffix(',') {
        (stripped, ",")
    } else {
        (s, "")
    };

    let coloured = if core.starts_with('"') && core.ends_with('"') {
        core.green().to_string()
    } else if core == "true" || core == "false" {
        core.magenta().to_string()
    } else if core == "null" {
        core.dimmed().to_string()
    } else if core.parse::<f64>().is_ok() {
        core.yellow().to_string()
    } else {
        // Structural tokens like `{`, `}`, `[`, `]`, or `{,` lines
        core.to_string()
    };
    format!("{coloured}{suffix}")
}

/// Find the byte-index of the `:` that separates a JSON key from its value.
/// Returns `None` for structural lines (`{`, `}`, etc.) and bare value lines.
fn find_key_colon(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('"') {
        return None;
    }
    let indent = line.len() - trimmed.len();
    let mut escaped = false;
    let mut chars = trimmed.char_indices();
    chars.next(); // skip opening `"`
    for (idx, ch) in chars {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            // idx is the closing quote; check for `:` after optional whitespace
            let after = trimmed[idx + 1..].trim_start();
            if after.starts_with(':') {
                // Return position of `:` in the original line
                let ws = trimmed[idx + 1..].len() - after.len();
                return Some(indent + idx + 1 + ws);
            }
            return None;
        }
    }
    None
}

fn io_err(e: std::io::Error) -> QkError {
    QkError::Io {
        path: "<stdout>".to_string(),
        source: e,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{Record, SourceInfo};
    use indexmap::IndexMap;
    use serde_json::Value;

    fn make_record(json: &str) -> Record {
        use crate::util::intern::intern;
        let v: Value = serde_json::from_str(json).unwrap();
        let fields = match v {
            Value::Object(m) => m.into_iter().map(|(k, v)| (intern(&k), v)).collect(),
            _ => IndexMap::new(),
        };
        Record::new(fields, Some(json.to_string()), SourceInfo::default())
    }

    #[test]
    fn pretty_output_is_valid_json() {
        let rec = make_record(r#"{"level":"error","msg":"oops"}"#);
        let mut buf = Vec::new();
        write(&[rec], &mut buf, false).unwrap();
        let s = String::from_utf8(buf).unwrap();
        serde_json::from_str::<Value>(s.trim()).unwrap();
    }

    #[test]
    fn pretty_output_is_indented() {
        let rec = make_record(r#"{"level":"error"}"#);
        let mut buf = Vec::new();
        write(&[rec], &mut buf, false).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains('\n'), "should be multi-line");
    }

    #[test]
    fn blank_line_between_records() {
        let r1 = make_record(r#"{"a":1}"#);
        let r2 = make_record(r#"{"b":2}"#);
        let mut buf = Vec::new();
        write(&[r1, r2], &mut buf, false).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\n\n"), "should have blank line between records");
    }

    #[test]
    fn color_mode_contains_ansi() {
        let rec = make_record(r#"{"level":"error"}"#);
        let mut buf = Vec::new();
        write(&[rec], &mut buf, true).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains('\x1b'), "should have ANSI codes in color mode");
    }
}

//! Colorized terminal rendering for NDJSON output.
//!
//! Color scheme:
//! - Keys:                bold cyan
//! - Strings:             green  (semantic overrides below)
//! - Numbers:             yellow (HTTP status codes get range-based color)
//! - Booleans:            bold magenta (true) / magenta (false)
//! - Null:                dim
//! - Structural punctuation: dim  ({, }, [, ], :, ,)
//!
//! Semantic overrides by field key:
//! - level/severity:  error→bold red · warn→bold yellow · info→bold green · debug→blue · trace→dim
//! - msg/message:     bright white  (most prominent — the human-readable content)
//! - ts/timestamp:    dim           (background noise, least prominent)
//! - error/exception: red           (error message strings)
//! - status (HTTP):   2xx→green · 3xx→cyan · 4xx→yellow · 5xx→bold red

use indexmap::IndexMap;
use owo_colors::{OwoColorize, Style};
use serde_json::Value;

/// Render a record's fields as a colorized single-line JSON string.
pub fn paint_record(fields: &IndexMap<String, Value>) -> String {
    let mut buf = String::with_capacity(256);
    buf.push_str(&dim("{"));
    for (i, (key, value)) in fields.iter().enumerate() {
        if i > 0 {
            buf.push_str(&dim(", "));
        }
        push_key(&mut buf, key);
        buf.push_str(&dim(": "));
        push_value(&mut buf, key, value);
    }
    buf.push_str(&dim("}"));
    buf
}

fn push_key(buf: &mut String, key: &str) {
    let js = json_str(key);
    buf.push_str(&format!("{}", js.style(Style::new().bold().cyan())));
}

fn push_value(buf: &mut String, key: &str, value: &Value) {
    match value {
        Value::String(s) => buf.push_str(&paint_string(key, s)),
        Value::Number(n) => buf.push_str(&paint_number(key, n)),
        Value::Bool(b) => buf.push_str(&paint_bool(*b)),
        Value::Null => buf.push_str(&format!("{}", "null".dimmed())),
        Value::Array(arr) => push_array(buf, arr),
        Value::Object(map) => push_object(buf, map),
    }
}

fn paint_string(key: &str, s: &str) -> String {
    let js = json_str(s);
    if is_level_key(key) {
        return paint_level_value(s, &js);
    }
    if is_message_key(key) {
        return format!("{}", js.style(Style::new().bright_white()));
    }
    if is_timestamp_key(key) {
        return format!("{}", js.dimmed());
    }
    if is_error_key(key) {
        return format!("{}", js.red());
    }
    format!("{}", js.green())
}

/// Apply color to a log level value. `raw` is the plain string; `js` is its JSON form.
pub fn paint_level_value(raw: &str, js: &str) -> String {
    match raw.to_ascii_lowercase().as_str() {
        "error" | "fatal" | "critical" | "crit" | "err" => {
            format!("{}", js.style(Style::new().bold().red()))
        }
        "warn" | "warning" => format!("{}", js.style(Style::new().bold().yellow())),
        "info" | "information" | "notice" => {
            format!("{}", js.style(Style::new().bold().green()))
        }
        "debug" | "dbg" => format!("{}", js.blue()),
        "trace" => format!("{}", js.dimmed()),
        _ => format!("{}", js.green()),
    }
}

fn paint_number(key: &str, n: &serde_json::Number) -> String {
    if is_status_key(key) {
        if let Some(code) = n.as_u64() {
            return match code {
                200..=299 => format!("{}", n.to_string().green()),
                300..=399 => format!("{}", n.to_string().cyan()),
                400..=499 => format!("{}", n.to_string().yellow()),
                500..=599 => format!("{}", n.to_string().style(Style::new().bold().red())),
                _ => format!("{}", n.to_string().yellow()),
            };
        }
    }
    format!("{}", n.to_string().yellow())
}

fn paint_bool(b: bool) -> String {
    if b {
        format!("{}", "true".style(Style::new().bold().magenta()))
    } else {
        format!("{}", "false".magenta())
    }
}

fn push_array(buf: &mut String, arr: &[Value]) {
    buf.push_str(&dim("["));
    for (i, v) in arr.iter().enumerate() {
        if i > 0 {
            buf.push_str(&dim(", "));
        }
        push_value(buf, "", v);
    }
    buf.push_str(&dim("]"));
}

fn push_object(buf: &mut String, map: &serde_json::Map<String, Value>) {
    buf.push_str(&dim("{"));
    for (i, (k, v)) in map.iter().enumerate() {
        if i > 0 {
            buf.push_str(&dim(", "));
        }
        buf.push_str(&format!("{}", json_str(k).cyan()));
        buf.push_str(&dim(": "));
        push_value(buf, k, v);
    }
    buf.push_str(&dim("}"));
}

fn dim(s: &str) -> String {
    format!("{}", s.dimmed())
}

/// Serialize a string to its JSON representation (with surrounding quotes and proper escaping).
fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| format!("\"{}\"", s))
}

fn is_level_key(key: &str) -> bool {
    matches!(key, "level" | "severity" | "lvl" | "log_level" | "loglevel")
}

fn is_message_key(key: &str) -> bool {
    matches!(key, "msg" | "message" | "body" | "text" | "description" | "log")
}

fn is_timestamp_key(key: &str) -> bool {
    matches!(
        key,
        "ts" | "time" | "timestamp" | "@timestamp" | "datetime"
            | "date" | "created_at" | "updated_at"
    )
}

fn is_error_key(key: &str) -> bool {
    matches!(
        key,
        "error" | "err" | "exception" | "stack_trace" | "stacktrace" | "cause"
    )
}

fn is_status_key(key: &str) -> bool {
    matches!(
        key,
        "status" | "status_code" | "http_status" | "code" | "response_code"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(json: &str) -> IndexMap<String, Value> {
        let v: Value = serde_json::from_str(json).unwrap();
        match v {
            Value::Object(m) => m.into_iter().collect(),
            _ => IndexMap::new(),
        }
    }

    #[test]
    fn paint_record_contains_field_names() {
        let out = paint_record(&fields(r#"{"level":"error","msg":"oops"}"#));
        assert!(out.contains("level"));
        assert!(out.contains("msg"));
    }

    #[test]
    fn paint_record_produces_ansi_codes() {
        let out = paint_record(&fields(r#"{"level":"info","msg":"ok"}"#));
        assert!(out.contains('\x1b'), "expected ANSI escape codes");
    }

    #[test]
    fn error_level_produces_red() {
        let out = paint_level_value("error", "\"error\"");
        // ANSI red = code 31
        assert!(out.contains("31"));
    }

    #[test]
    fn warn_level_produces_yellow() {
        let out = paint_level_value("warn", "\"warn\"");
        // ANSI yellow = code 33
        assert!(out.contains("33"));
    }

    #[test]
    fn info_level_produces_green() {
        let out = paint_level_value("info", "\"info\"");
        // ANSI green = code 32
        assert!(out.contains("32"));
    }

    #[test]
    fn number_produces_yellow() {
        let n = serde_json::Number::from(42_i64);
        let out = paint_number("latency", &n);
        // ANSI yellow = code 33
        assert!(out.contains("33"));
    }

    #[test]
    fn http_200_produces_green() {
        let n = serde_json::Number::from(200_i64);
        let out = paint_number("status", &n);
        assert!(out.contains("32"));
    }

    #[test]
    fn http_404_produces_yellow() {
        let n = serde_json::Number::from(404_i64);
        let out = paint_number("status", &n);
        assert!(out.contains("33"));
    }

    #[test]
    fn http_500_produces_red() {
        let n = serde_json::Number::from(500_i64);
        let out = paint_number("status", &n);
        assert!(out.contains("31"));
    }

    #[test]
    fn bool_true_produces_magenta() {
        let out = paint_bool(true);
        // ANSI magenta = code 35
        assert!(out.contains("35"));
    }

    #[test]
    fn json_str_escapes_special_chars() {
        let out = json_str("say \"hi\"");
        assert_eq!(out, r#""say \"hi\"""#);
    }

    #[test]
    fn timestamp_field_gets_dim() {
        let out = paint_string("ts", "2024-01-01");
        // ANSI dim = code 2
        assert!(out.contains('\x1b'));
        // should NOT be green (33), should be dim (2)
        // dim is ESC[2m
        assert!(out.contains("[2m") || out.contains("\x1b[2m"));
    }

    #[test]
    fn message_field_gets_bright_white() {
        let out = paint_string("msg", "connection timeout");
        // bright white = 97
        assert!(out.contains("97"));
    }
}

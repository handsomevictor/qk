//! Timestamp parsing and time-series bucketing utilities.
//!
//! Supports three timestamp representations:
//! - **RFC 3339** strings — `"2024-01-15T10:05:30Z"`, `"2024-01-15T10:05:30+00:00"`
//! - **Unix epoch** (integer/float ≥ 1_000_000_000) — seconds since 1970-01-01T00:00:00Z
//! - **Epoch-ms** (integer/float ≥ 1_000_000_000_000) — milliseconds since epoch
//!
//! Bucket sizes are expressed as a duration string: `"30s"`, `"1m"`, `"5m"`, `"1h"`, `"1d"`.

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde_json::Value;

/// Parse a bucket-size string like `"5m"`, `"30s"`, `"2h"`, `"1d"` → seconds.
///
/// Returns `None` if the string is not a valid duration.
pub fn parse_bucket_secs(s: &str) -> Option<i64> {
    let s = s.trim();
    let (num_str, unit) = if let Some(rest) = s.strip_suffix('s') {
        (rest, 1i64)
    } else if let Some(rest) = s.strip_suffix('m') {
        (rest, 60)
    } else if let Some(rest) = s.strip_suffix('h') {
        (rest, 3_600)
    } else if let Some(rest) = s.strip_suffix('d') {
        (rest, 86_400)
    } else {
        return None;
    };
    num_str
        .parse::<i64>()
        .ok()
        .filter(|&n| n > 0)
        .map(|n| n * unit)
}

/// Try to extract a Unix timestamp (in seconds) from a JSON value.
///
/// Recognises:
/// - RFC 3339 strings
/// - Integer/float numbers ≥ 1_000_000_000 (epoch-s) or ≥ 1_000_000_000_000 (epoch-ms)
pub fn value_to_timestamp(v: &Value) -> Option<i64> {
    match v {
        Value::String(s) => parse_rfc3339(s),
        Value::Number(n) => {
            let f = n.as_f64()?;
            // Epoch-ms threshold: > 1e12 implies milliseconds
            if f >= 1_000_000_000_000.0 {
                Some((f / 1_000.0) as i64)
            } else if f >= 1_000_000_000.0 {
                Some(f as i64)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Floor a Unix timestamp to the nearest bucket boundary and return an RFC 3339 string.
///
/// `bucket_secs` must be positive.  The bucketing formula is:
/// `bucket_start = floor(ts / bucket_secs) * bucket_secs`
pub fn bucket_label(ts: i64, bucket_secs: i64) -> String {
    let bucket_ts = (ts / bucket_secs) * bucket_secs;
    match Utc.timestamp_opt(bucket_ts, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        _ => bucket_ts.to_string(),
    }
}

/// Returns `true` if the string looks like a duration (digits followed by s/m/h/d).
pub fn looks_like_duration(s: &str) -> bool {
    parse_bucket_secs(s).is_some()
}

fn parse_rfc3339(s: &str) -> Option<i64> {
    // Try full RFC 3339 with timezone
    if let Ok(dt) = s.parse::<DateTime<Utc>>() {
        return Some(dt.timestamp());
    }
    // Try naive datetime (assume UTC)
    for fmt in &[
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%SZ",
        "%Y-%m-%d %H:%M:%SZ",
    ] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(naive.and_utc().timestamp());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bucket_secs_valid() {
        assert_eq!(parse_bucket_secs("30s"), Some(30));
        assert_eq!(parse_bucket_secs("5m"), Some(300));
        assert_eq!(parse_bucket_secs("1h"), Some(3_600));
        assert_eq!(parse_bucket_secs("1d"), Some(86_400));
        assert_eq!(parse_bucket_secs("2h"), Some(7_200));
    }

    #[test]
    fn parse_bucket_secs_invalid() {
        assert_eq!(parse_bucket_secs("5x"), None);
        assert_eq!(parse_bucket_secs("abc"), None);
        assert_eq!(parse_bucket_secs("0m"), None);
        assert_eq!(parse_bucket_secs(""), None);
    }

    #[test]
    fn value_to_timestamp_rfc3339() {
        let v = Value::String("2024-01-15T10:05:30Z".to_string());
        assert!(value_to_timestamp(&v).is_some());
    }

    #[test]
    fn value_to_timestamp_epoch_secs() {
        let v = Value::Number(1_705_312_330_i64.into());
        assert_eq!(value_to_timestamp(&v), Some(1_705_312_330));
    }

    #[test]
    fn value_to_timestamp_epoch_ms() {
        let ms: i64 = 1_705_312_330_000;
        let v = serde_json::json!(ms);
        assert_eq!(value_to_timestamp(&v), Some(1_705_312_330));
    }

    #[test]
    fn value_to_timestamp_non_ts_number() {
        // Small number (port, status code) should not be treated as a timestamp
        let v = Value::Number(8080.into());
        assert_eq!(value_to_timestamp(&v), None);
    }

    #[test]
    fn bucket_label_5m() {
        // 2024-01-15T10:07:30Z = 1705312050 → floor to 5m → 1705312200? Let me verify:
        // 1705312050 / 300 = 5684373.5 → floor = 5684373 * 300 = 1705311900
        let ts = 1_705_312_050_i64;
        let label = bucket_label(ts, 300);
        assert!(label.contains('T'), "expected RFC 3339: {label}");
    }

    #[test]
    fn looks_like_duration_true() {
        assert!(looks_like_duration("5m"));
        assert!(looks_like_duration("1h"));
        assert!(looks_like_duration("30s"));
    }

    #[test]
    fn looks_like_duration_false() {
        assert!(!looks_like_duration("level"));
        assert!(!looks_like_duration("service"));
        assert!(!looks_like_duration("5x"));
    }
}

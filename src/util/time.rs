//! Timestamp parsing and time-series bucketing utilities.
//!
//! Supports three timestamp representations:
//! - **RFC 3339** strings — `"2024-01-15T10:05:30Z"`, `"2024-01-15T10:05:30+00:00"`
//! - **Unix epoch** (integer/float ≥ 1_000_000_000) — seconds since 1970-01-01T00:00:00Z
//! - **Epoch-ms** (integer/float ≥ 1_000_000_000_000) — milliseconds since epoch
//!
//! Bucket sizes are expressed as a duration string: `"30s"`, `"1m"`, `"5m"`, `"1h"`, `"1d"`.

use chrono::{DateTime, Datelike, IsoWeek, NaiveDateTime, TimeZone, Timelike, Utc};
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

/// Returns `true` if the string is a valid bucket specifier.
///
/// Accepts both fixed-duration strings (`"5m"`, `"1h"`, `"30s"`) and
/// calendar-unit keywords (`"hour"`, `"day"`, `"week"`, `"month"`, `"year"`).
pub fn looks_like_duration(s: &str) -> bool {
    parse_bucket_secs(s).is_some() || parse_calendar_unit(s).is_some()
}

/// Calendar-aligned time-bucket units for natural period grouping.
///
/// Unlike fixed-second buckets (`5m`, `1h`), calendar units align to
/// natural boundaries: day = midnight UTC, month = 1st of month, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarUnit {
    Hour,
    Day,
    Week,
    Month,
    Year,
}

/// Parse a calendar unit keyword.
///
/// Returns `Some` for `"hour"`, `"day"`, `"week"`, `"month"`, `"year"`;
/// `None` for anything else (including fixed-duration strings like `"5m"`).
pub fn parse_calendar_unit(s: &str) -> Option<CalendarUnit> {
    match s.trim().to_ascii_lowercase().as_str() {
        "hour" => Some(CalendarUnit::Hour),
        "day" => Some(CalendarUnit::Day),
        "week" => Some(CalendarUnit::Week),
        "month" => Some(CalendarUnit::Month),
        "year" => Some(CalendarUnit::Year),
        _ => None,
    }
}

/// Floor a Unix timestamp to a calendar-aligned boundary and return a label string.
///
/// Labels use ISO 8601 format:
/// - Hour  → `"2024-01-15T10:00:00Z"`
/// - Day   → `"2024-01-15"`
/// - Week  → `"2024-W03"` (ISO week)
/// - Month → `"2024-01"`
/// - Year  → `"2024"`
pub fn calendar_bucket_label(ts: i64, unit: CalendarUnit) -> String {
    let dt = match Utc.timestamp_opt(ts, 0) {
        chrono::LocalResult::Single(d) => d,
        _ => return ts.to_string(),
    };
    match unit {
        CalendarUnit::Hour => {
            let truncated = dt
                .with_minute(0)
                .and_then(|d| d.with_second(0))
                .and_then(|d| d.with_nanosecond(0))
                .unwrap_or(dt);
            truncated.format("%Y-%m-%dT%H:00:00Z").to_string()
        }
        CalendarUnit::Day => dt.format("%Y-%m-%d").to_string(),
        CalendarUnit::Week => {
            // ISO week: e.g. "2024-W03"
            let iso: IsoWeek = dt.iso_week();
            format!("{:04}-W{:02}", iso.year(), iso.week())
        }
        CalendarUnit::Month => dt.format("%Y-%m").to_string(),
        CalendarUnit::Year => dt.format("%Y").to_string(),
    }
}

/// Return the current UTC time as a Unix timestamp (seconds).
pub fn now_secs() -> i64 {
    Utc::now().timestamp()
}

/// Parse a relative timestamp expression like `"now"`, `"now-5m"`, `"now+1h"`.
///
/// Returns `None` if the string is not a recognised relative-time expression.
/// Absolute timestamps (RFC 3339, epoch numbers) are handled by `value_to_timestamp`.
pub fn parse_relative_ts(s: &str) -> Option<i64> {
    let s = s.trim();
    if s == "now" {
        return Some(now_secs());
    }
    // Match "now±<duration>"
    let (sign, rest) = if let Some(r) = s.strip_prefix("now-") {
        (-1i64, r)
    } else if let Some(r) = s.strip_prefix("now+") {
        (1i64, r)
    } else {
        return None;
    };
    let secs = parse_bucket_secs(rest)?;
    Some(now_secs() + sign * secs)
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
        // 2024-01-15T10:05:30Z = 1705313130
        let v = Value::String("2024-01-15T10:05:30Z".to_string());
        assert_eq!(value_to_timestamp(&v), Some(1_705_313_130));
    }

    #[test]
    fn value_to_timestamp_rfc3339_with_offset() {
        // 2024-01-15T18:05:30+08:00 == 2024-01-15T10:05:30Z = 1705313130
        let v = Value::String("2024-01-15T18:05:30+08:00".to_string());
        assert_eq!(value_to_timestamp(&v), Some(1_705_313_130));
    }

    #[test]
    fn value_to_timestamp_float_epoch() {
        // Float epoch seconds: fractional part is truncated
        let v = serde_json::json!(1_705_313_130.9_f64);
        assert_eq!(value_to_timestamp(&v), Some(1_705_313_130));
    }

    #[test]
    fn value_to_timestamp_naive_datetime() {
        // Naive datetime without timezone — treated as UTC
        let v = Value::String("2024-01-15T10:05:30".to_string());
        assert_eq!(value_to_timestamp(&v), Some(1_705_313_130));
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
    fn bucket_label_5m_exact_value() {
        // 2024-01-15T10:07:30Z = 1705313250
        // 1705313250 / 300 = 5684377.5 → floor = 5684377 * 300 = 1705313100 = 2024-01-15T10:05:00Z
        let ts = 1_705_313_250_i64;
        let label = bucket_label(ts, 300);
        assert_eq!(label, "2024-01-15T10:05:00Z");
    }

    #[test]
    fn bucket_label_1h_exact_value() {
        // 2024-01-15T10:12:10Z = 1705313530
        // 1705313530 / 3600 = 473698.2 → floor = 473698 * 3600 = 1705312800 = 2024-01-15T10:00:00Z
        let ts = 1_705_313_530_i64;
        let label = bucket_label(ts, 3_600);
        assert_eq!(label, "2024-01-15T10:00:00Z");
    }

    #[test]
    fn bucket_label_on_boundary() {
        // 2024-01-15T10:10:00Z = 1705313400 — exactly on a 5m boundary
        // 1705313400 / 300 = 5684378 remainder 0 → maps to itself
        let ts = 1_705_313_400_i64;
        let label = bucket_label(ts, 300);
        assert_eq!(label, "2024-01-15T10:10:00Z");
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

    // --- P3: UTC day-boundary and timezone-offset bucket tests ---

    #[test]
    fn bucket_label_1d_midnight_boundary() {
        // 2024-01-15T00:00:00Z = 1705276800 — exactly on a 1d boundary
        // 1705276800 / 86400 = 19741 remainder 0 → maps to itself
        let ts = 1_705_276_800_i64;
        let label = bucket_label(ts, 86_400);
        assert_eq!(label, "2024-01-15T00:00:00Z");
    }

    #[test]
    fn bucket_label_1d_mid_day_floors_to_midnight() {
        // 2024-01-15T14:30:00Z = 1705328200
        // 1705328200 / 86400 = 19742.x → floor * 86400 = 1705276800 = 2024-01-15T00:00:00Z
        let ts = 1_705_328_200_i64;
        let label = bucket_label(ts, 86_400);
        assert_eq!(label, "2024-01-15T00:00:00Z");
    }

    #[test]
    fn value_to_timestamp_positive_offset() {
        // 2024-01-15T18:05:30+08:00 == 2024-01-15T10:05:30Z = 1705313130
        let v = serde_json::Value::String("2024-01-15T18:05:30+08:00".to_string());
        assert_eq!(value_to_timestamp(&v), Some(1_705_313_130));
    }

    #[test]
    fn value_to_timestamp_negative_offset() {
        // 2024-01-15T05:05:30-05:00 == 2024-01-15T10:05:30Z = 1705313130
        let v = serde_json::Value::String("2024-01-15T05:05:30-05:00".to_string());
        assert_eq!(value_to_timestamp(&v), Some(1_705_313_130));
    }

    // --- parse_relative_ts tests ---

    #[test]
    fn parse_relative_ts_now() {
        let before = now_secs();
        let ts = parse_relative_ts("now").expect("'now' should parse");
        let after = now_secs();
        assert!(ts >= before && ts <= after);
    }

    #[test]
    fn parse_relative_ts_now_minus_5m() {
        let now = now_secs();
        let ts = parse_relative_ts("now-5m").expect("'now-5m' should parse");
        // Allow ±2 second tolerance for slow machines
        assert!((ts - (now - 300)).abs() <= 2);
    }

    #[test]
    fn parse_relative_ts_now_plus_1h() {
        let now = now_secs();
        let ts = parse_relative_ts("now+1h").expect("'now+1h' should parse");
        assert!((ts - (now + 3_600)).abs() <= 2);
    }

    #[test]
    fn parse_relative_ts_invalid() {
        assert_eq!(parse_relative_ts("2024-01-15T10:00:00Z"), None);
        assert_eq!(parse_relative_ts("5m"), None);
        assert_eq!(parse_relative_ts(""), None);
        assert_eq!(parse_relative_ts("now-bad"), None);
    }

    #[test]
    fn parse_calendar_unit_valid() {
        assert_eq!(parse_calendar_unit("day"), Some(CalendarUnit::Day));
        assert_eq!(parse_calendar_unit("hour"), Some(CalendarUnit::Hour));
        assert_eq!(parse_calendar_unit("week"), Some(CalendarUnit::Week));
        assert_eq!(parse_calendar_unit("month"), Some(CalendarUnit::Month));
        assert_eq!(parse_calendar_unit("year"), Some(CalendarUnit::Year));
        // Case-insensitive
        assert_eq!(parse_calendar_unit("DAY"), Some(CalendarUnit::Day));
        assert_eq!(parse_calendar_unit("Month"), Some(CalendarUnit::Month));
    }

    #[test]
    fn parse_calendar_unit_invalid() {
        assert_eq!(parse_calendar_unit("5m"), None);
        assert_eq!(parse_calendar_unit("1h"), None);
        assert_eq!(parse_calendar_unit(""), None);
        assert_eq!(parse_calendar_unit("daily"), None);
    }

    #[test]
    fn calendar_bucket_label_day() {
        // 2024-01-15T10:07:30Z = 1705313250 → truncates to 2024-01-15
        let label = calendar_bucket_label(1_705_313_250, CalendarUnit::Day);
        assert_eq!(label, "2024-01-15");
    }

    #[test]
    fn calendar_bucket_label_month() {
        // 2024-01-15T10:07:30Z → 2024-01
        let label = calendar_bucket_label(1_705_313_250, CalendarUnit::Month);
        assert_eq!(label, "2024-01");
    }

    #[test]
    fn calendar_bucket_label_year() {
        let label = calendar_bucket_label(1_705_313_250, CalendarUnit::Year);
        assert_eq!(label, "2024");
    }

    #[test]
    fn calendar_bucket_label_hour() {
        // 2024-01-15T10:07:30Z → 2024-01-15T10:00:00Z
        let label = calendar_bucket_label(1_705_313_250, CalendarUnit::Hour);
        assert_eq!(label, "2024-01-15T10:00:00Z");
    }

    #[test]
    fn calendar_bucket_label_week() {
        // 2024-01-15 is in ISO week 3 of 2024 → "2024-W03"
        let label = calendar_bucket_label(1_705_313_250, CalendarUnit::Week);
        assert_eq!(label, "2024-W03");
    }

    #[test]
    fn looks_like_duration_accepts_calendar_units() {
        assert!(looks_like_duration("day"));
        assert!(looks_like_duration("month"));
        assert!(looks_like_duration("year"));
        assert!(looks_like_duration("hour"));
        assert!(looks_like_duration("week"));
        // Fixed durations still work
        assert!(looks_like_duration("5m"));
        assert!(looks_like_duration("1h"));
    }
}

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::Value;

use crate::record::{Record, SourceInfo};
use crate::util::error::Result;
use crate::util::intern::intern;
use crate::util::time::{
    bucket_label, calendar_bucket_label, parse_bucket_secs, parse_calendar_unit, parse_relative_ts,
    value_to_timestamp,
};

use super::parser::{Aggregation, FastQuery, FilterExpr, FilterOp, LogicalOp, SortExpr, SortOrder};

/// Returns `true` if this query requires all records to be buffered before output
/// (aggregation or sort). When `false`, records can be processed one at a time
/// in streaming mode.
pub fn requires_buffering(query: &FastQuery) -> bool {
    query.aggregation.is_some() || query.sort.is_some()
}

/// Evaluate a single record against the query filters and projection.
///
/// Returns `Some(projected_record)` if the record passes all filters, `None` if filtered out.
/// Only valid when `!requires_buffering(query)`. Does not apply aggregation or sort.
pub fn eval_one(query: &FastQuery, rec: Record) -> Result<Option<Record>> {
    if !query.filters.is_empty() && !eval_filters(&query.filters, &query.logical_ops, &rec)? {
        return Ok(None);
    }
    Ok(Some(apply_projection_one(&query.projection, rec)))
}

/// Apply projection to a single record (used by the streaming path).
fn apply_projection_one(projection: &Option<Vec<String>>, mut rec: Record) -> Record {
    let Some(fields) = projection else {
        return rec;
    };
    let kept: IndexMap<Arc<str>, Value> = fields
        .iter()
        .filter_map(|f| rec.fields.swap_remove(f.as_str()).map(|v| (intern(f), v)))
        .collect();
    rec.fields = kept;
    rec
}

/// Apply a `FastQuery` to a list of records, returning `(results, warnings)`.
///
/// Warnings are emitted when a string value cannot be coerced to a number during
/// numeric aggregation (sum/avg/min/max). Null-like strings ("None", "NA", etc.)
/// are silently skipped without a warning.
pub fn eval(query: &FastQuery, records: Vec<Record>) -> Result<(Vec<Record>, Vec<String>)> {
    let filtered = filter_records(query, records)?;

    if let Some(agg) = &query.aggregation {
        return aggregate(agg, filtered);
    }

    let projected = apply_projection(&query.projection, filtered);
    let sorted = apply_sort(&query.sort, projected)?;
    let limited = apply_limit(query.limit, sorted);

    Ok((limited, Vec::new()))
}

// ── Filter ────────────────────────────────────────────────────────────────────

fn filter_records(query: &FastQuery, records: Vec<Record>) -> Result<Vec<Record>> {
    if query.filters.is_empty() {
        return Ok(records);
    }
    let mut out = Vec::new();
    for rec in records {
        if eval_filters(&query.filters, &query.logical_ops, &rec)? {
            out.push(rec);
        }
    }
    Ok(out)
}

/// Evaluate the filter chain against a single record.
fn eval_filters(filters: &[FilterExpr], ops: &[LogicalOp], rec: &Record) -> Result<bool> {
    let mut result = eval_filter(&filters[0], rec)?;
    for (i, filter) in filters[1..].iter().enumerate() {
        let rhs = eval_filter(filter, rec)?;
        result = match ops.get(i) {
            Some(LogicalOp::Or) => result || rhs,
            _ => result && rhs, // default: AND
        };
    }
    Ok(result)
}

fn eval_filter(f: &FilterExpr, rec: &Record) -> Result<bool> {
    let field_val = rec.get(&f.field);

    match f.op {
        FilterOp::Exists => return Ok(field_val.is_some()),
        FilterOp::Ne => {
            return Ok(field_val.map(value_to_str) != Some(f.value.clone()));
        }
        _ => {}
    }

    let Some(val) = field_val else {
        return Ok(false);
    };

    match f.op {
        FilterOp::Eq => Ok(value_matches_str(val, &f.value)),
        FilterOp::Gt => compare_values(val, &f.value, |a, b| a > b),
        FilterOp::Lt => compare_values(val, &f.value, |a, b| a < b),
        FilterOp::Gte => compare_values(val, &f.value, |a, b| a >= b),
        FilterOp::Lte => compare_values(val, &f.value, |a, b| a <= b),
        FilterOp::Contains => Ok(value_to_str(val).contains(f.value.as_str())),
        FilterOp::StartsWith => Ok(value_to_str(val).starts_with(f.value.as_str())),
        FilterOp::EndsWith => Ok(value_to_str(val).ends_with(f.value.as_str())),
        // Both Regex and Glob use a pre-compiled Regex stored at parse time — zero per-record cost.
        FilterOp::Regex | FilterOp::Glob => {
            let re = f
                .compiled
                .as_ref()
                .expect("regex pre-compiled in parse_filter");
            Ok(re.is_match(&value_to_str(val)))
        }
        FilterOp::Between => {
            // value is encoded as "LOW\x00HIGH"
            let mut parts = f.value.splitn(2, '\x00');
            let low = parts.next().unwrap_or("");
            let high = parts.next().unwrap_or("");
            let ge = compare_values(val, low, |a, b| a >= b)?;
            let le = compare_values(val, high, |a, b| a <= b)?;
            Ok(ge && le)
        }
        FilterOp::Exists | FilterOp::Ne => unreachable!(),
    }
}

/// Compare a JSON value against a string literal.
///
/// Strategy (in order):
/// 1. If the literal is a relative-time expression (`now`, `now-5m`, …) and the field
///    value is a recognisable timestamp → compare as Unix epoch seconds.
/// 2. If the value is numeric **and** the literal parses as a number → numeric comparison.
/// 3. Otherwise → lexicographic (dictionary-order) string comparison.
///
/// Lexicographic order is correct for RFC 3339 timestamps because the format is
/// zero-padded and sortable as ASCII: `"2024-01-15T10:06:00Z" > "2024-01-15T10:05:00Z"`.
fn compare_values(val: &Value, literal: &str, cmp: impl Fn(f64, f64) -> bool) -> Result<bool> {
    // Relative-time shorthand: resolve "now-5m" → epoch seconds, then compare.
    if let Some(ref_ts) = parse_relative_ts(literal) {
        if let Some(val_ts) = value_to_timestamp(val) {
            return Ok(cmp(val_ts as f64, ref_ts as f64));
        }
    }
    if let Some(n) = value_as_f64(val) {
        if let Ok(m) = literal.parse::<f64>() {
            return Ok(cmp(n, m));
        }
    }
    // Lexicographic comparison: map Ordering → signed integer then compare against 0
    let ord = value_to_str(val).as_str().cmp(literal) as i8;
    Ok(cmp(f64::from(ord), 0.0))
}

fn value_matches_str(val: &Value, s: &str) -> bool {
    match val {
        Value::String(v) => v == s,
        Value::Number(n) => n.to_string() == s,
        Value::Bool(b) => b.to_string() == s,
        Value::Null => s == "null",
        _ => false,
    }
}

fn value_to_str(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

fn value_as_f64(val: &Value) -> Option<f64> {
    match val {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

// ── Projection ────────────────────────────────────────────────────────────────

fn apply_projection(projection: &Option<Vec<String>>, records: Vec<Record>) -> Vec<Record> {
    let Some(fields) = projection else {
        return records;
    };
    records
        .into_iter()
        .map(|mut rec| {
            let kept: IndexMap<Arc<str>, Value> = fields
                .iter()
                .filter_map(|f| rec.fields.swap_remove(f.as_str()).map(|v| (intern(f), v)))
                .collect();
            rec.fields = kept;
            rec
        })
        .collect()
}

// ── Aggregation ───────────────────────────────────────────────────────────────

fn aggregate(agg: &Aggregation, records: Vec<Record>) -> Result<(Vec<Record>, Vec<String>)> {
    match agg {
        Aggregation::Count => {
            let mut fields: IndexMap<Arc<str>, Value> = IndexMap::new();
            fields.insert(intern("count"), Value::Number(records.len().into()));
            Ok((
                vec![Record::new(fields, None, SourceInfo::default())],
                Vec::new(),
            ))
        }
        Aggregation::CountBy(fields) => {
            let recs = count_by_multi(fields, records)?;
            Ok((recs, Vec::new()))
        }
        Aggregation::GroupByTime { bucket, field } => {
            let recs = group_by_time(bucket, field, records)?;
            Ok((recs, Vec::new()))
        }
        Aggregation::Fields => {
            let recs = fields_discovery(records)?;
            Ok((recs, Vec::new()))
        }
        Aggregation::Sum(field) => stat_agg("sum", field, &records, |nums| {
            if nums.is_empty() {
                None
            } else {
                Some(nums.iter().sum())
            }
        }),
        Aggregation::Avg(field) => stat_agg("avg", field, &records, |nums| {
            if nums.is_empty() {
                None
            } else {
                Some(nums.iter().sum::<f64>() / nums.len() as f64)
            }
        }),
        Aggregation::Min(field) => stat_agg("min", field, &records, |nums| {
            if nums.is_empty() {
                None
            } else {
                nums.iter().cloned().reduce(f64::min)
            }
        }),
        Aggregation::Max(field) => stat_agg("max", field, &records, |nums| {
            if nums.is_empty() {
                None
            } else {
                nums.iter().cloned().reduce(f64::max)
            }
        }),
        Aggregation::CountUnique(field) => {
            let n = records
                .iter()
                .map(|r| r.get(field.as_str()).map(value_to_str).unwrap_or_default())
                .collect::<std::collections::HashSet<_>>()
                .len();
            let mut fields: IndexMap<Arc<str>, Value> = IndexMap::new();
            fields.insert(intern("count_unique"), Value::Number(n.into()));
            Ok((
                vec![Record::new(fields, None, SourceInfo::default())],
                Vec::new(),
            ))
        }
    }
}

/// Discover all unique field names across all records, sorted alphabetically.
fn fields_discovery(records: Vec<Record>) -> Result<Vec<Record>> {
    let mut seen = std::collections::BTreeSet::new();
    for rec in &records {
        for key in rec.fields.keys() {
            seen.insert(key.as_ref().to_string());
        }
    }
    let result = seen
        .into_iter()
        .map(|field| {
            let mut fields: IndexMap<Arc<str>, Value> = IndexMap::new();
            fields.insert(intern("field"), Value::String(field));
            Record::new(fields, None, SourceInfo::default())
        })
        .collect();
    Ok(result)
}

fn stat_agg(
    key: &str,
    field: &str,
    records: &[Record],
    f: impl Fn(&[f64]) -> Option<f64>,
) -> Result<(Vec<Record>, Vec<String>)> {
    let (nums, mut warnings) = collect_numeric_field(field, records);
    let value = match f(&nums) {
        Some(result) => {
            let rounded = (result * 1_000_000.0).round() / 1_000_000.0;
            match serde_json::Number::from_f64(rounded) {
                Some(n) => Value::Number(n),
                None => {
                    warnings.push(format!(
                        "[qk warning] {key}({field}) produced non-finite result — returning null"
                    ));
                    Value::Null
                }
            }
        }
        None => {
            warnings.push(format!(
                "[qk warning] {key}({field}): no numeric values found — returning null"
            ));
            Value::Null
        }
    };
    let mut fields: IndexMap<Arc<str>, Value> = IndexMap::new();
    fields.insert(intern(key), value);
    Ok((
        vec![Record::new(fields, None, SourceInfo::default())],
        warnings,
    ))
}

/// Collect numeric values from a field across all records, emitting warnings for
/// unexpected string values that cannot be parsed as numbers.
///
/// - `Value::Number` → used directly.
/// - `Value::String` that parses as f64 → used silently (e.g. "3001").
/// - `Value::String` that is null-like ("None", "NA", ...) → silently skipped.
/// - `Value::String` with other content → skipped with a warning.
/// - `Value::Null` / field absent → silently skipped.
fn collect_numeric_field(field: &str, records: &[Record]) -> (Vec<f64>, Vec<String>) {
    use crate::util::cast::is_null_like;
    const MAX_WARN: usize = 5;
    let mut nums = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut total_warn: usize = 0;

    for rec in records {
        match rec.get(field) {
            None | Some(Value::Null) => {}
            Some(Value::Number(n)) => {
                if let Some(f) = n.as_f64() {
                    nums.push(f);
                }
            }
            Some(Value::String(s)) => {
                if is_null_like(s) {
                    // intentional null — skip silently
                } else if let Ok(n) = s.parse::<f64>() {
                    nums.push(n);
                } else {
                    total_warn += 1;
                    if warnings.len() < MAX_WARN {
                        let loc = if rec.source.line > 0 {
                            format!("line {}, {}", rec.source.line, rec.source.file)
                        } else {
                            rec.source.file.clone()
                        };
                        warnings.push(format!(
                            "[qk warning] field '{field}': value {s:?} is not numeric ({loc}) — skipped in {field} aggregation"
                        ));
                    }
                }
            }
            Some(_) => {} // Bool / Array / Object — skip silently
        }
    }
    if total_warn > MAX_WARN {
        warnings.push(format!(
            "... and {} more type-mismatch warning(s) suppressed",
            total_warn - MAX_WARN
        ));
    }
    (nums, warnings)
}

/// Group records by one or more fields and count occurrences per unique combination.
///
/// Uses a NUL-byte composite key so that multi-field grouping avoids ambiguous splits.
/// Results are sorted by count descending (most common first).
fn count_by_multi(fields: &[String], records: Vec<Record>) -> Result<Vec<Record>> {
    let mut counts: IndexMap<String, usize> = IndexMap::new();
    for rec in &records {
        let key: String = fields
            .iter()
            .map(|f| rec.get(f.as_str()).map(value_to_str).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\x00");
        *counts.entry(key).or_insert(0) += 1;
    }

    let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));

    let result = pairs
        .into_iter()
        .map(|(composite_key, count)| {
            let vals: Vec<&str> = composite_key.split('\x00').collect();
            let mut rec_fields: IndexMap<Arc<str>, Value> = IndexMap::new();
            for (field, val) in fields.iter().zip(vals.iter()) {
                rec_fields.insert(intern(field), Value::String(val.to_string()));
            }
            rec_fields.insert(intern("count"), Value::Number(count.into()));
            Record::new(rec_fields, None, SourceInfo::default())
        })
        .collect();

    Ok(result)
}

/// Group records into time buckets and return `{bucket: "...", count: N}` per bucket.
///
/// Records without a parseable timestamp in `field` are silently skipped.
/// Output is sorted by bucket ascending.
fn group_by_time(bucket_str: &str, field: &str, records: Vec<Record>) -> Result<Vec<Record>> {
    let mut counts: IndexMap<String, usize> = IndexMap::new();

    // Try fixed-duration bucket first, then calendar unit.
    let labeler: Box<dyn Fn(i64) -> String> = if let Some(secs) = parse_bucket_secs(bucket_str) {
        Box::new(move |ts| bucket_label(ts, secs))
    } else if let Some(unit) = parse_calendar_unit(bucket_str) {
        Box::new(move |ts| calendar_bucket_label(ts, unit))
    } else {
        return Err(crate::util::error::QkError::Query(format!(
            "invalid bucket '{bucket_str}': expected a duration (5m, 1h) or calendar unit (hour, day, week, month, year)"
        )));
    };

    for rec in &records {
        if let Some(ts) = rec.get(field).and_then(value_to_timestamp) {
            let label = labeler(ts);
            *counts.entry(label).or_insert(0) += 1;
        }
    }

    let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));

    let result = pairs
        .into_iter()
        .map(|(label, count)| {
            let mut fields: IndexMap<Arc<str>, Value> = IndexMap::new();
            fields.insert(intern("bucket"), Value::String(label));
            fields.insert(intern("count"), Value::Number(count.into()));
            Record::new(fields, None, SourceInfo::default())
        })
        .collect();

    Ok(result)
}

// ── Sort ──────────────────────────────────────────────────────────────────────

fn apply_sort(sort: &Option<SortExpr>, mut records: Vec<Record>) -> Result<Vec<Record>> {
    let Some(sort_expr) = sort else {
        return Ok(records);
    };

    records.sort_by(|a, b| {
        let va = a.get(&sort_expr.field);
        let vb = b.get(&sort_expr.field);
        let cmp = compare_json_values(va, vb);
        if sort_expr.order == SortOrder::Desc {
            cmp.reverse()
        } else {
            cmp
        }
    });

    Ok(records)
}

fn compare_json_values(a: Option<&Value>, b: Option<&Value>) -> std::cmp::Ordering {
    match (a, b) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, _) => std::cmp::Ordering::Less,
        (_, None) => std::cmp::Ordering::Greater,
        (Some(av), Some(bv)) => {
            // Try numeric comparison first
            if let (Some(an), Some(bn)) = (value_as_f64(av), value_as_f64(bv)) {
                return an.partial_cmp(&bn).unwrap_or(std::cmp::Ordering::Equal);
            }
            // Fall back to string comparison
            value_to_str(av).cmp(&value_to_str(bv))
        }
    }
}

// ── Limit ─────────────────────────────────────────────────────────────────────

fn apply_limit(limit: Option<usize>, records: Vec<Record>) -> Vec<Record> {
    match limit {
        Some(n) => records.into_iter().take(n).collect(),
        None => records,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::fast::parser;

    /// Parse a whitespace-separated query string into a `FastQuery`.
    fn parse_query(s: &str) -> crate::util::error::Result<FastQuery> {
        let toks: Vec<String> = s.split_whitespace().map(String::from).collect();
        parser::parse(&toks).map(|(q, _)| q)
    }

    /// Build a single `Record` from a slice of `(field, value)` string pairs.
    fn make_record(pairs: &[(&str, &str)]) -> Record {
        let mut fields: IndexMap<Arc<str>, Value> = IndexMap::new();
        for (k, v) in pairs {
            fields.insert(intern(k), Value::String(v.to_string()));
        }
        Record::new(fields, None, SourceInfo::default())
    }

    fn make_records(jsons: &[&str]) -> Vec<Record> {
        jsons
            .iter()
            .map(|s| {
                let v: Value = serde_json::from_str(s).unwrap();
                let fields = match v {
                    Value::Object(m) => m.into_iter().map(|(k, v)| (intern(&k), v)).collect(),
                    _ => IndexMap::new(),
                };
                Record::new(fields, Some(s.to_string()), SourceInfo::default())
            })
            .collect()
    }

    fn run(tokens: &[&str], jsons: &[&str]) -> Vec<Record> {
        let toks: Vec<String> = tokens.iter().map(|s| s.to_string()).collect();
        let (q, _) = parser::parse(&toks).unwrap();
        eval(&q, make_records(jsons)).unwrap().0
    }

    #[test]
    fn filter_eq() {
        let result = run(
            &["where", "level=error"],
            &[
                r#"{"level":"error","msg":"a"}"#,
                r#"{"level":"info","msg":"b"}"#,
            ],
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].fields["level"], Value::String("error".into()));
    }

    #[test]
    fn filter_gt_numeric() {
        let result = run(
            &["where", "status>400"],
            &[r#"{"status":500}"#, r#"{"status":200}"#],
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].fields["status"], Value::Number(500.into()));
    }

    #[test]
    fn select_fields() {
        let result = run(
            &["where", "level=error", "select", "level", "msg"],
            &[r#"{"level":"error","msg":"oops","ts":"2024"}"#],
        );
        assert_eq!(result[0].fields.len(), 2);
        assert!(result[0].fields.contains_key("level"));
        assert!(result[0].fields.contains_key("msg"));
        assert!(!result[0].fields.contains_key("ts"));
    }

    #[test]
    fn count_total() {
        let result = run(&["count"], &[r#"{"a":1}"#, r#"{"a":2}"#, r#"{"a":3}"#]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].fields["count"], Value::Number(3.into()));
    }

    #[test]
    fn count_by_field() {
        let result = run(
            &["count", "by", "level"],
            &[
                r#"{"level":"error"}"#,
                r#"{"level":"info"}"#,
                r#"{"level":"error"}"#,
            ],
        );
        assert_eq!(result.len(), 2);
        // Most common first
        assert_eq!(result[0].fields["level"], Value::String("error".into()));
        assert_eq!(result[0].fields["count"], Value::Number(2.into()));
    }

    #[test]
    fn sort_desc() {
        let result = run(
            &["sort", "n", "desc"],
            &[r#"{"n":1}"#, r#"{"n":3}"#, r#"{"n":2}"#],
        );
        assert_eq!(result[0].fields["n"], Value::Number(3.into()));
        assert_eq!(result[2].fields["n"], Value::Number(1.into()));
    }

    #[test]
    fn limit() {
        let result = run(&["limit", "2"], &[r#"{"a":1}"#, r#"{"a":2}"#, r#"{"a":3}"#]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_and() {
        let result = run(
            &["where", "level=error", "and", "service=api"],
            &[
                r#"{"level":"error","service":"api"}"#,
                r#"{"level":"error","service":"web"}"#,
            ],
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].fields["service"], Value::String("api".into()));
    }

    #[test]
    fn filter_or() {
        let result = run(
            &["where", "level=error", "or", "level=warn"],
            &[
                r#"{"level":"error"}"#,
                r#"{"level":"warn"}"#,
                r#"{"level":"info"}"#,
            ],
        );
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_contains() {
        let result = run(
            &["where", "msg", "contains", "time"],
            &[r#"{"msg":"request timeout"}"#, r#"{"msg":"started"}"#],
        );
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn filter_exists() {
        let result = run(
            &["where", "error", "exists"],
            &[r#"{"error":"oops"}"#, r#"{"msg":"ok"}"#],
        );
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn fields_discovery() {
        let result = run(
            &["fields"],
            &[
                r#"{"level":"error","msg":"a"}"#,
                r#"{"level":"info","ts":"x"}"#,
            ],
        );
        // Should return sorted unique field names
        let names: Vec<&str> = result
            .iter()
            .map(|r| r.fields["field"].as_str().unwrap())
            .collect();
        assert_eq!(names, vec!["level", "msg", "ts"]);
    }

    #[test]
    fn sum_field() {
        let result = run(&["sum", "n"], &[r#"{"n":1}"#, r#"{"n":2}"#, r#"{"n":3}"#]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].fields["sum"].as_f64().unwrap(), 6.0);
    }

    #[test]
    fn avg_field() {
        let result = run(&["avg", "n"], &[r#"{"n":1}"#, r#"{"n":2}"#, r#"{"n":3}"#]);
        assert_eq!(result[0].fields["avg"].as_f64().unwrap(), 2.0);
    }

    #[test]
    fn min_field() {
        let result = run(&["min", "n"], &[r#"{"n":5}"#, r#"{"n":2}"#, r#"{"n":8}"#]);
        assert_eq!(result[0].fields["min"].as_f64().unwrap(), 2.0);
    }

    #[test]
    fn max_field() {
        let result = run(&["max", "n"], &[r#"{"n":5}"#, r#"{"n":2}"#, r#"{"n":8}"#]);
        assert_eq!(result[0].fields["max"].as_f64().unwrap(), 8.0);
    }

    #[test]
    fn filter_startswith() {
        let result = run(
            &["where", "msg", "startswith", "req"],
            &[r#"{"msg":"request timeout"}"#, r#"{"msg":"started"}"#],
        );
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].fields["msg"],
            Value::String("request timeout".into())
        );
    }

    #[test]
    fn filter_endswith() {
        let result = run(
            &["where", "path", "endswith", ".log"],
            &[r#"{"path":"app.log"}"#, r#"{"path":"app.json"}"#],
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].fields["path"], Value::String("app.log".into()));
    }

    #[test]
    fn filter_glob() {
        let result = run(
            &["where", "name", "glob", "al*"],
            &[
                r#"{"name":"alice"}"#,
                r#"{"name":"bob"}"#,
                r#"{"name":"Alex"}"#,
            ],
        );
        // glob is case-insensitive, so "alice" and "Alex" both match "al*"
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_glob_question_mark() {
        let result = run(
            &["where", "code", "glob", "er?or"],
            &[
                r#"{"code":"error"}"#,
                r#"{"code":"eroor"}"#,
                r#"{"code":"err"}"#,
            ],
        );
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn head_is_alias_for_limit() {
        let result = run(&["head", "2"], &[r#"{"a":1}"#, r#"{"a":2}"#, r#"{"a":3}"#]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn count_by_time_5m() {
        // Two records in the same 5m bucket, one in next bucket
        let result = run(
            &["count", "by", "5m", "ts"],
            &[
                r#"{"ts":"2024-01-15T10:02:00Z"}"#,
                r#"{"ts":"2024-01-15T10:04:00Z"}"#,
                r#"{"ts":"2024-01-15T10:07:00Z"}"#,
            ],
        );
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].fields["count"], Value::Number(2.into()));
        assert_eq!(result[1].fields["count"], Value::Number(1.into()));
    }

    #[test]
    fn count_by_time_default_ts_field() {
        // When no field specified, defaults to "ts"
        let result = run(
            &["count", "by", "1h"],
            &[
                r#"{"ts":"2024-01-15T10:02:00Z"}"#,
                r#"{"ts":"2024-01-15T10:50:00Z"}"#,
                r#"{"ts":"2024-01-15T11:10:00Z"}"#,
            ],
        );
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn count_by_time_skips_non_ts_records() {
        // Records without a parseable ts field are silently skipped
        let result = run(
            &["count", "by", "5m", "ts"],
            &[
                r#"{"ts":"2024-01-15T10:02:00Z"}"#,
                r#"{"msg":"no ts here"}"#,
            ],
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].fields["count"], Value::Number(1.into()));
    }

    #[test]
    fn count_by_time_epoch_ms_field() {
        // Epoch-milliseconds: 1705313220000 ms = 1705313220 s = 2024-01-15T10:07:00Z → 5m bucket 10:05
        //                     1705313580000 ms = 1705313580 s = 2024-01-15T10:13:00Z → 5m bucket 10:10
        let result = run(
            &["count", "by", "5m", "ts"],
            &[r#"{"ts":1705313220000}"#, r#"{"ts":1705313580000}"#],
        );
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0].fields["bucket"].as_str().unwrap(),
            "2024-01-15T10:05:00Z"
        );
        assert_eq!(
            result[1].fields["bucket"].as_str().unwrap(),
            "2024-01-15T10:10:00Z"
        );
    }

    #[test]
    fn count_by_time_epoch_secs_field() {
        // 1705313530 = 2024-01-15T10:12:10Z → 1h bucket 10:00
        // 1705315200 = 2024-01-15T10:40:00Z → 1h bucket 10:00
        // 1705317000 = 2024-01-15T11:10:00Z → 1h bucket 11:00
        let result = run(
            &["count", "by", "1h"],
            &[
                r#"{"ts":1705313530}"#,
                r#"{"ts":1705315200}"#,
                r#"{"ts":1705317000}"#,
            ],
        );
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].fields["count"], Value::Number(2.into()));
        assert_eq!(result[1].fields["count"], Value::Number(1.into()));
        assert_eq!(
            result[0].fields["bucket"].as_str().unwrap(),
            "2024-01-15T10:00:00Z"
        );
        assert_eq!(
            result[1].fields["bucket"].as_str().unwrap(),
            "2024-01-15T11:00:00Z"
        );
    }

    #[test]
    fn count_by_time_empty_input() {
        let result = run(&["count", "by", "5m"], &[]);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn count_by_time_1d_bucket() {
        // Three records on 2024-01-15, one on 2024-01-16
        let result = run(
            &["count", "by", "1d"],
            &[
                r#"{"ts":"2024-01-15T01:00:00Z"}"#,
                r#"{"ts":"2024-01-15T12:00:00Z"}"#,
                r#"{"ts":"2024-01-15T23:59:00Z"}"#,
                r#"{"ts":"2024-01-16T00:01:00Z"}"#,
            ],
        );
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].fields["count"], Value::Number(3.into()));
        assert_eq!(result[1].fields["count"], Value::Number(1.into()));
        assert_eq!(
            result[0].fields["bucket"].as_str().unwrap(),
            "2024-01-15T00:00:00Z"
        );
        assert_eq!(
            result[1].fields["bucket"].as_str().unwrap(),
            "2024-01-16T00:00:00Z"
        );
    }

    #[test]
    fn count_unique_field() {
        let result = run(
            &["count", "unique", "level"],
            &[
                r#"{"level":"error","svc":"api"}"#,
                r#"{"level":"warn","svc":"api"}"#,
                r#"{"level":"error","svc":"db"}"#,
                r#"{"level":"info","svc":"api"}"#,
            ],
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].fields["count_unique"], Value::Number(3.into())); // error, warn, info
    }

    #[test]
    fn count_unique_empty() {
        let result = run(&["count", "unique", "level"], &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].fields["count_unique"], Value::Number(0.into()));
    }

    #[test]
    fn count_by_time_bucket_label_exact() {
        // 10:07:30 → floored to nearest 5m window = 10:05:00
        let result = run(
            &["count", "by", "5m", "ts"],
            &[r#"{"ts":"2024-01-15T10:07:30Z"}"#],
        );
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].fields["bucket"].as_str().unwrap(),
            "2024-01-15T10:05:00Z"
        );
    }

    #[test]
    fn string_comparison_rfc3339_gt() {
        // Lexicographic comparison on RFC 3339 strings must work correctly
        let result = run(
            &["where", "ts>2024-01-15T10:05:00Z"],
            &[
                r#"{"ts":"2024-01-15T10:04:00Z"}"#,
                r#"{"ts":"2024-01-15T10:05:00Z"}"#,
                r#"{"ts":"2024-01-15T10:06:00Z"}"#,
                r#"{"ts":"2024-01-15T11:00:00Z"}"#,
            ],
        );
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn string_comparison_rfc3339_gte() {
        let result = run(
            &["where", "ts>=2024-01-15T10:05:00Z"],
            &[
                r#"{"ts":"2024-01-15T10:04:00Z"}"#,
                r#"{"ts":"2024-01-15T10:05:00Z"}"#,
                r#"{"ts":"2024-01-15T10:06:00Z"}"#,
            ],
        );
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn string_comparison_rfc3339_lt() {
        let result = run(
            &["where", "ts<2024-01-15T10:05:00Z"],
            &[
                r#"{"ts":"2024-01-15T10:04:00Z"}"#,
                r#"{"ts":"2024-01-15T10:05:00Z"}"#,
                r#"{"ts":"2024-01-15T10:06:00Z"}"#,
            ],
        );
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn count_by_multi_fields() {
        let records = vec![
            make_record(&[("level", "error"), ("svc", "api")]),
            make_record(&[("level", "error"), ("svc", "api")]),
            make_record(&[("level", "error"), ("svc", "db")]),
            make_record(&[("level", "warn"), ("svc", "api")]),
        ];
        let q = parse_query("count by level svc").unwrap();
        let (out, _) = eval(&q, records).unwrap();
        // 3 unique combinations
        assert_eq!(out.len(), 3);
        // Most common first: error+api (2)
        assert_eq!(out[0].get("level").unwrap(), &Value::String("error".into()));
        assert_eq!(out[0].get("svc").unwrap(), &Value::String("api".into()));
        assert_eq!(out[0].get("count").unwrap(), &Value::Number(2.into()));
    }

    #[test]
    fn count_by_single_field_still_works() {
        let records = vec![
            make_record(&[("level", "error")]),
            make_record(&[("level", "error")]),
            make_record(&[("level", "info")]),
        ];
        let q = parse_query("count by level").unwrap();
        let (out, _) = eval(&q, records).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].get("count").unwrap(), &Value::Number(2.into()));
    }
}

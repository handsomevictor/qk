use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::Value;

use crate::record::{Record, SourceInfo};
use crate::util::error::Result;
use crate::util::intern::intern;
use crate::util::time::{bucket_label, parse_bucket_secs, value_to_timestamp};

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
        FilterOp::Exists | FilterOp::Ne => unreachable!(),
    }
}

/// Compare a JSON value against a string literal, attempting numeric comparison first.
fn compare_values(val: &Value, literal: &str, cmp: impl Fn(f64, f64) -> bool) -> Result<bool> {
    if let Some(n) = value_as_f64(val) {
        if let Ok(m) = literal.parse::<f64>() {
            return Ok(cmp(n, m));
        }
    }
    // Fall back to lexicographic comparison
    Ok(cmp(
        value_to_str(val).as_str().len() as f64,
        literal.len() as f64,
    ))
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
                vec![Record::new(fields, String::new(), SourceInfo::default())],
                Vec::new(),
            ))
        }
        Aggregation::CountBy(group_field) => {
            let recs = count_by(group_field, records)?;
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
        Aggregation::Sum(field) => stat_agg("sum", field, &records, |nums| nums.iter().sum()),
        Aggregation::Avg(field) => stat_agg("avg", field, &records, |nums| {
            if nums.is_empty() {
                0.0
            } else {
                nums.iter().sum::<f64>() / nums.len() as f64
            }
        }),
        Aggregation::Min(field) => stat_agg("min", field, &records, |nums| {
            nums.iter().cloned().fold(f64::INFINITY, f64::min)
        }),
        Aggregation::Max(field) => stat_agg("max", field, &records, |nums| {
            nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        }),
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
            Record::new(fields, String::new(), SourceInfo::default())
        })
        .collect();
    Ok(result)
}

fn stat_agg(
    key: &str,
    field: &str,
    records: &[Record],
    f: impl Fn(Vec<f64>) -> f64,
) -> Result<(Vec<Record>, Vec<String>)> {
    let (nums, warnings) = collect_numeric_field(field, records);
    let result = f(nums);
    let rounded = (result * 1_000_000.0).round() / 1_000_000.0;
    let json_num =
        serde_json::Number::from_f64(rounded).unwrap_or_else(|| serde_json::Number::from(0));
    let mut fields: IndexMap<Arc<str>, Value> = IndexMap::new();
    fields.insert(intern(key), Value::Number(json_num));
    Ok((
        vec![Record::new(fields, String::new(), SourceInfo::default())],
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

fn count_by(field: &str, records: Vec<Record>) -> Result<Vec<Record>> {
    let mut counts: IndexMap<String, usize> = IndexMap::new();
    for rec in &records {
        let key = rec.get(field).map(value_to_str).unwrap_or_default();
        *counts.entry(key).or_insert(0) += 1;
    }

    // Sort by count descending (most common first)
    let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));

    let result = pairs
        .into_iter()
        .map(|(key, count)| {
            let mut fields: IndexMap<Arc<str>, Value> = IndexMap::new();
            fields.insert(intern(field), Value::String(key));
            fields.insert(intern("count"), Value::Number(count.into()));
            Record::new(fields, String::new(), SourceInfo::default())
        })
        .collect();

    Ok(result)
}

/// Group records into time buckets and return `{bucket: "...", count: N}` per bucket.
///
/// Records without a parseable timestamp in `field` are silently skipped.
/// Output is sorted by bucket ascending.
fn group_by_time(bucket_str: &str, field: &str, records: Vec<Record>) -> Result<Vec<Record>> {
    let bucket_secs = parse_bucket_secs(bucket_str).ok_or_else(|| {
        crate::util::error::QkError::Query(format!(
            "invalid bucket size '{bucket_str}': expected a duration like 5m, 1h, 30s"
        ))
    })?;

    let mut counts: IndexMap<String, usize> = IndexMap::new();
    for rec in &records {
        if let Some(ts) = rec.get(field).and_then(value_to_timestamp) {
            let label = bucket_label(ts, bucket_secs);
            *counts.entry(label).or_insert(0) += 1;
        }
    }

    // Sort buckets chronologically
    let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));

    let result = pairs
        .into_iter()
        .map(|(label, count)| {
            let mut fields: IndexMap<Arc<str>, Value> = IndexMap::new();
            fields.insert(intern("bucket"), Value::String(label));
            fields.insert(intern("count"), Value::Number(count.into()));
            Record::new(fields, String::new(), SourceInfo::default())
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

    fn make_records(jsons: &[&str]) -> Vec<Record> {
        jsons
            .iter()
            .map(|s| {
                let v: Value = serde_json::from_str(s).unwrap();
                let fields = match v {
                    Value::Object(m) => m.into_iter().map(|(k, v)| (intern(&k), v)).collect(),
                    _ => IndexMap::new(),
                };
                Record::new(fields, s.to_string(), SourceInfo::default())
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
}

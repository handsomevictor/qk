use std::sync::Arc;

use indexmap::IndexMap;
use regex::Regex;
use serde_json::Value;

use crate::query::fast::parser::SortOrder;
use crate::record::{Record, SourceInfo};
use crate::util::error::{QkError, Result};
use crate::util::intern::intern;
use chrono::{TimeZone, Utc};

use crate::util::time::{
    bucket_label, calendar_bucket_label, parse_bucket_secs, parse_calendar_unit, value_to_timestamp,
};

use super::ast::{ArithExpr, ArithOp, CmpOp, DslQuery, Expr, FieldPath, Literal, Stage};

/// Apply a `DslQuery` to a list of records and return `(results, warnings)`.
///
/// Warnings are emitted when string values cannot be coerced to numbers during
/// sum/avg/min/max stages. Null-like strings are silently skipped.
pub fn eval(query: &DslQuery, records: Vec<Record>) -> Result<(Vec<Record>, Vec<String>)> {
    let filtered: Vec<Record> = records
        .into_iter()
        .filter(|r| eval_expr(&query.filter, r))
        .collect();
    apply_stages(&query.transforms, filtered)
}

// ── Stages ────────────────────────────────────────────────────────────────────

fn apply_stages(stages: &[Stage], mut records: Vec<Record>) -> Result<(Vec<Record>, Vec<String>)> {
    let mut all_warnings: Vec<String> = Vec::new();
    for stage in stages {
        let (new_records, warnings) = apply_stage(stage, records)?;
        records = new_records;
        all_warnings.extend(warnings);
    }
    Ok((records, all_warnings))
}

fn apply_stage(stage: &Stage, records: Vec<Record>) -> Result<(Vec<Record>, Vec<String>)> {
    match stage {
        Stage::Pick(paths) => Ok((apply_pick(paths, records), Vec::new())),
        Stage::Omit(paths) => Ok((apply_omit(paths, records), Vec::new())),
        Stage::Count => Ok((vec![count_record(records.len())], Vec::new())),
        Stage::SortBy(path, ord) => {
            Ok((sort_by(path, *ord == SortOrder::Desc, records), Vec::new()))
        }
        Stage::GroupBy(paths) => Ok((group_by_multi(paths, records), Vec::new())),
        Stage::Limit(n) => Ok((records.into_iter().take(*n).collect(), Vec::new())),
        Stage::Skip(n) => Ok((records.into_iter().skip(*n).collect(), Vec::new())),
        Stage::Dedup(path) => Ok((dedup_by(path, records), Vec::new())),
        Stage::Sum(path) => {
            let (val, w) = aggregate_sum_with_warn(path, &records);
            Ok((vec![stat_record("sum", val)], w))
        }
        Stage::Avg(path) => {
            let (val, w) = aggregate_avg_with_warn(path, &records);
            Ok((vec![stat_record("avg", val)], w))
        }
        Stage::Min(path) => {
            let (val, w) = aggregate_min_with_warn(path, &records);
            Ok((vec![stat_record("min", val)], w))
        }
        Stage::Max(path) => {
            let (val, w) = aggregate_max_with_warn(path, &records);
            Ok((vec![stat_record("max", val)], w))
        }
        Stage::GroupByTime { path, bucket } => {
            let recs = group_by_time_dsl(path, bucket, records)?;
            Ok((recs, Vec::new()))
        }
        Stage::HourOfDay(path) => Ok((
            apply_time_attr(path, "hour_of_day", extract_hour, records),
            Vec::new(),
        )),
        Stage::DayOfWeek(path) => Ok((
            apply_time_attr(path, "day_of_week", extract_day_of_week, records),
            Vec::new(),
        )),
        Stage::IsWeekend(path) => Ok((
            apply_time_attr_bool(path, "is_weekend", extract_is_weekend, records),
            Vec::new(),
        )),
        Stage::CountUnique(path) => {
            let key = path.join(".");
            let n = records
                .iter()
                .filter_map(|r| r.get(key.as_str()))
                .map(|v| v.to_string())
                .collect::<std::collections::HashSet<_>>()
                .len();
            let mut fields: IndexMap<Arc<str>, Value> = IndexMap::new();
            fields.insert(intern("count_unique"), Value::Number(n.into()));
            Ok((
                vec![Record::new(fields, None, SourceInfo::default())],
                Vec::new(),
            ))
        }
        Stage::ToLower(path) => {
            let key_str = path.join(".");
            let key = intern(&key_str);
            let mapped: Vec<Record> = records
                .into_iter()
                .map(|mut rec| {
                    if let Some(Value::String(s)) = rec.fields.get(&key).cloned() {
                        rec.fields
                            .insert(Arc::clone(&key), Value::String(s.to_lowercase()));
                    }
                    rec
                })
                .collect();
            Ok((mapped, Vec::new()))
        }
        Stage::ToUpper(path) => {
            let key_str = path.join(".");
            let key = intern(&key_str);
            let mapped: Vec<Record> = records
                .into_iter()
                .map(|mut rec| {
                    if let Some(Value::String(s)) = rec.fields.get(&key).cloned() {
                        rec.fields
                            .insert(Arc::clone(&key), Value::String(s.to_uppercase()));
                    }
                    rec
                })
                .collect();
            Ok((mapped, Vec::new()))
        }
        Stage::Replace { path, from, to } => {
            let key_str = path.join(".");
            let key = intern(&key_str);
            let mapped: Vec<Record> = records
                .into_iter()
                .map(|mut rec| {
                    if let Some(Value::String(s)) = rec.fields.get(&key).cloned() {
                        rec.fields.insert(
                            Arc::clone(&key),
                            Value::String(s.replace(from.as_str(), to.as_str())),
                        );
                    }
                    rec
                })
                .collect();
            Ok((mapped, Vec::new()))
        }
        Stage::Split { path, sep } => {
            let key_str = path.join(".");
            let key = intern(&key_str);
            let mapped: Vec<Record> = records
                .into_iter()
                .map(|mut rec| {
                    if let Some(Value::String(s)) = rec.fields.get(&key).cloned() {
                        let parts: Vec<Value> = s
                            .split(sep.as_str())
                            .map(|p| Value::String(p.to_string()))
                            .collect();
                        rec.fields.insert(Arc::clone(&key), Value::Array(parts));
                    }
                    rec
                })
                .collect();
            Ok((mapped, Vec::new()))
        }
        Stage::Map { output, expr } => {
            let out_key = intern(output);
            let mapped: Vec<Record> = records
                .into_iter()
                .map(|mut rec| {
                    if let Some(val) = eval_arith(expr, &rec) {
                        let n = if val.fract() == 0.0
                            && val >= i64::MIN as f64
                            && val <= i64::MAX as f64
                        {
                            // Store whole numbers as integers to preserve JSON type parity
                            serde_json::Number::from(val as i64)
                        } else {
                            match serde_json::Number::from_f64(val) {
                                Some(n) => n,
                                None => return rec,
                            }
                        };
                        rec.fields.insert(Arc::clone(&out_key), Value::Number(n));
                    }
                    rec
                })
                .collect();
            Ok((mapped, Vec::new()))
        }
    }
}

/// Group records into time buckets and return `{bucket, count}` per bucket.
///
/// Records without a parseable timestamp are silently skipped.
fn group_by_time_dsl(
    path: &[String],
    bucket_str: &str,
    records: Vec<Record>,
) -> Result<Vec<Record>> {
    let key = path.join(".");
    let labeler: Box<dyn Fn(i64) -> String> = if let Some(secs) = parse_bucket_secs(bucket_str) {
        Box::new(move |ts| bucket_label(ts, secs))
    } else if let Some(unit) = parse_calendar_unit(bucket_str) {
        Box::new(move |ts| calendar_bucket_label(ts, unit))
    } else {
        return Err(QkError::Query(format!(
                "invalid bucket '{bucket_str}': expected a duration (5m, 1h) or calendar unit (hour, day, week, month, year)"
            )));
    };

    let mut counts: IndexMap<String, usize> = IndexMap::new();
    for rec in &records {
        if let Some(ts) = rec.get(&key).and_then(value_to_timestamp) {
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

/// Add a computed numeric field to every record based on a timestamp field.
fn apply_time_attr(
    path: &[String],
    out_field: &str,
    extract: impl Fn(i64) -> i64,
    records: Vec<Record>,
) -> Vec<Record> {
    let key = path.join(".");
    records
        .into_iter()
        .map(|mut rec| {
            if let Some(ts) = rec.get(&key).and_then(value_to_timestamp) {
                rec.fields
                    .insert(intern(out_field), Value::Number(extract(ts).into()));
            }
            rec
        })
        .collect()
}

/// Add a computed boolean field to every record based on a timestamp field.
fn apply_time_attr_bool(
    path: &[String],
    out_field: &str,
    extract: impl Fn(i64) -> bool,
    records: Vec<Record>,
) -> Vec<Record> {
    let key = path.join(".");
    records
        .into_iter()
        .map(|mut rec| {
            if let Some(ts) = rec.get(&key).and_then(value_to_timestamp) {
                rec.fields
                    .insert(intern(out_field), Value::Bool(extract(ts)));
            }
            rec
        })
        .collect()
}

fn extract_hour(ts: i64) -> i64 {
    use chrono::Timelike;
    match Utc.timestamp_opt(ts, 0) {
        chrono::LocalResult::Single(dt) => dt.hour() as i64,
        _ => 0,
    }
}

fn extract_day_of_week(ts: i64) -> i64 {
    use chrono::Datelike;
    match Utc.timestamp_opt(ts, 0) {
        chrono::LocalResult::Single(dt) => dt.weekday().number_from_monday() as i64,
        _ => 0,
    }
}

fn extract_is_weekend(ts: i64) -> bool {
    use chrono::Datelike;
    match Utc.timestamp_opt(ts, 0) {
        chrono::LocalResult::Single(dt) => {
            matches!(dt.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun)
        }
        _ => false,
    }
}

fn apply_pick(paths: &[FieldPath], records: Vec<Record>) -> Vec<Record> {
    let keys: Vec<String> = paths.iter().map(|p| p.join(".")).collect();
    records
        .into_iter()
        .map(|mut rec| {
            let kept: IndexMap<Arc<str>, Value> = keys
                .iter()
                .filter_map(|k| rec.fields.swap_remove(k.as_str()).map(|v| (intern(k), v)))
                .collect();
            rec.fields = kept;
            rec
        })
        .collect()
}

fn apply_omit(paths: &[FieldPath], records: Vec<Record>) -> Vec<Record> {
    let keys: Vec<String> = paths.iter().map(|p| p.join(".")).collect();
    records
        .into_iter()
        .map(|mut rec| {
            for k in &keys {
                rec.fields.swap_remove(k.as_str());
            }
            rec
        })
        .collect()
}

fn count_record(n: usize) -> Record {
    let mut fields: IndexMap<Arc<str>, Value> = IndexMap::new();
    fields.insert(intern("count"), Value::Number(n.into()));
    Record::new(fields, None, SourceInfo::default())
}

fn sort_by(path: &FieldPath, desc: bool, mut records: Vec<Record>) -> Vec<Record> {
    let key = path.join(".");
    records.sort_by(|a, b| {
        let va = a.get(&key);
        let vb = b.get(&key);
        let cmp = compare_values(va, vb);
        if desc {
            cmp.reverse()
        } else {
            cmp
        }
    });
    records
}

fn compare_values(a: Option<&Value>, b: Option<&Value>) -> std::cmp::Ordering {
    match (a, b) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, _) => std::cmp::Ordering::Less,
        (_, None) => std::cmp::Ordering::Greater,
        (Some(av), Some(bv)) => {
            let an = value_as_f64(av);
            let bn = value_as_f64(bv);
            if let (Some(a), Some(b)) = (an, bn) {
                return a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal);
            }
            value_to_str(av).cmp(&value_to_str(bv))
        }
    }
}

/// Group records by one or more field paths, counting occurrences per unique combination.
///
/// Uses a NUL-byte composite key internally to avoid ambiguous splits.
/// Results are sorted by count descending (most common first).
fn group_by_multi(paths: &[FieldPath], records: Vec<Record>) -> Vec<Record> {
    let mut counts: IndexMap<String, usize> = IndexMap::new();
    for rec in &records {
        let key: String = paths
            .iter()
            .map(|p| {
                let k = p.join(".");
                rec.get(k.as_str()).map(value_to_str).unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join("\x00");
        *counts.entry(key).or_insert(0) += 1;
    }

    let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));

    pairs
        .into_iter()
        .map(|(composite_key, count)| {
            let vals: Vec<String> = composite_key.split('\x00').map(|s| s.to_string()).collect();
            let mut rec_fields: IndexMap<Arc<str>, Value> = IndexMap::new();
            for (path, val) in paths.iter().zip(vals.iter()) {
                let key = path.join(".");
                rec_fields.insert(intern(&key), Value::String(val.clone()));
            }
            rec_fields.insert(intern("count"), Value::Number(count.into()));
            Record::new(rec_fields, None, SourceInfo::default())
        })
        .collect()
}

fn dedup_by(path: &FieldPath, records: Vec<Record>) -> Vec<Record> {
    let key = path.join(".");
    let mut seen = std::collections::HashSet::new();
    records
        .into_iter()
        .filter(|rec| {
            let v = rec.get(&key).map(value_to_str).unwrap_or_default();
            seen.insert(v)
        })
        .collect()
}

fn aggregate_sum_with_warn(path: &FieldPath, records: &[Record]) -> (f64, Vec<String>) {
    let (nums, w) = collect_numeric_field_dsl(&path.join("."), records);
    (nums.iter().sum(), w)
}

fn aggregate_avg_with_warn(path: &FieldPath, records: &[Record]) -> (f64, Vec<String>) {
    let (nums, w) = collect_numeric_field_dsl(&path.join("."), records);
    let avg = if nums.is_empty() {
        0.0
    } else {
        nums.iter().sum::<f64>() / nums.len() as f64
    };
    (avg, w)
}

fn aggregate_min_with_warn(path: &FieldPath, records: &[Record]) -> (f64, Vec<String>) {
    let (nums, w) = collect_numeric_field_dsl(&path.join("."), records);
    (nums.iter().cloned().fold(f64::INFINITY, f64::min), w)
}

fn aggregate_max_with_warn(path: &FieldPath, records: &[Record]) -> (f64, Vec<String>) {
    let (nums, w) = collect_numeric_field_dsl(&path.join("."), records);
    (nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max), w)
}

/// Collect numeric values from a field, emitting warnings for unexpected string values.
fn collect_numeric_field_dsl(field: &str, records: &[Record]) -> (Vec<f64>, Vec<String>) {
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
            Some(_) => {}
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

fn stat_record(key: &str, value: f64) -> Record {
    let mut fields: IndexMap<Arc<str>, Value> = IndexMap::new();
    // Round to 6 decimal places to avoid floating-point noise in output
    let rounded = (value * 1_000_000.0).round() / 1_000_000.0;
    let json_num =
        serde_json::Number::from_f64(rounded).unwrap_or_else(|| serde_json::Number::from(0));
    fields.insert(intern(key), Value::Number(json_num));
    Record::new(fields, None, SourceInfo::default())
}

// ── Arithmetic expression evaluation ─────────────────────────────────────────

/// Evaluate an arithmetic expression against a record, returning a numeric result.
/// Returns `None` if a referenced field is missing or non-numeric.
fn eval_arith(expr: &ArithExpr, rec: &Record) -> Option<f64> {
    match expr {
        ArithExpr::Num(n) => Some(*n),
        ArithExpr::Field(path) => {
            let key = path.join(".");
            rec.get(key.as_str()).and_then(|v| match v {
                Value::Number(n) => n.as_f64(),
                Value::String(s) => s.parse::<f64>().ok(),
                _ => None,
            })
        }
        ArithExpr::Length(path) => {
            let key = path.join(".");
            rec.get(key.as_str()).and_then(|v| match v {
                Value::String(s) => Some(s.chars().count() as f64),
                Value::Array(a) => Some(a.len() as f64),
                _ => None,
            })
        }
        ArithExpr::BinOp(lhs, op, rhs) => {
            let l = eval_arith(lhs, rec)?;
            let r = eval_arith(rhs, rec)?;
            match op {
                ArithOp::Add => Some(l + r),
                ArithOp::Sub => Some(l - r),
                ArithOp::Mul => Some(l * r),
                ArithOp::Div => {
                    if r == 0.0 {
                        None
                    } else {
                        Some(l / r)
                    }
                }
            }
        }
    }
}

// ── Expression evaluation ─────────────────────────────────────────────────────

fn eval_expr(expr: &Expr, rec: &Record) -> bool {
    match expr {
        Expr::True => true,
        Expr::Exists(path) => rec.get(&path.join(".")).is_some(),
        Expr::Compare { path, op, value } => {
            let key = path.join(".");
            match op {
                CmpOp::Eq => compare_eq(rec.get(&key), value),
                CmpOp::Ne => !compare_eq(rec.get(&key), value),
                CmpOp::Gt => compare_num(rec.get(&key), value, |a, b| a > b),
                CmpOp::Lt => compare_num(rec.get(&key), value, |a, b| a < b),
                CmpOp::Gte => compare_num(rec.get(&key), value, |a, b| a >= b),
                CmpOp::Lte => compare_num(rec.get(&key), value, |a, b| a <= b),
                CmpOp::Contains => compare_contains(rec.get(&key), value),
                CmpOp::Matches => compare_regex(rec.get(&key), value),
            }
        }
        Expr::And(lhs, rhs) => eval_expr(lhs, rec) && eval_expr(rhs, rec),
        Expr::Or(lhs, rhs) => eval_expr(lhs, rec) || eval_expr(rhs, rec),
        Expr::Not(inner) => !eval_expr(inner, rec),
    }
}

fn compare_eq(field: Option<&Value>, lit: &Literal) -> bool {
    match (field, lit) {
        (Some(Value::String(s)), Literal::Str(t)) => s == t,
        (Some(Value::Number(n)), Literal::Num(t)) => n.as_f64().map(|f| f == *t).unwrap_or(false),
        (Some(Value::Bool(b)), Literal::Bool(t)) => b == t,
        (Some(Value::Null), Literal::Null) | (None, Literal::Null) => true,
        (Some(v), Literal::Str(t)) => value_to_str(v) == *t,
        _ => false,
    }
}

fn compare_num(field: Option<&Value>, lit: &Literal, cmp: impl Fn(f64, f64) -> bool) -> bool {
    let fv = field.and_then(value_as_f64);
    let lv = lit_as_f64(lit);
    match (fv, lv) {
        (Some(a), Some(b)) => cmp(a, b),
        _ => false,
    }
}

fn compare_contains(field: Option<&Value>, lit: &Literal) -> bool {
    let needle = match lit {
        Literal::Str(s) => s.as_str(),
        _ => return false,
    };
    // Array contains: check if any element's string representation equals the literal
    if let Some(Value::Array(arr)) = field {
        return arr.iter().any(|el| {
            if let Value::String(s) = el {
                s.as_str() == needle
            } else {
                value_to_str(el) == needle
            }
        });
    }
    let haystack = field.map(value_to_str).unwrap_or_default();
    memchr::memmem::find(haystack.as_bytes(), needle.as_bytes()).is_some()
}

fn compare_regex(field: Option<&Value>, lit: &Literal) -> bool {
    let haystack = field.map(value_to_str).unwrap_or_default();
    match lit {
        // Pre-compiled path: zero-cost per record (regex compiled once at parse time).
        Literal::Regex(re) => re.is_match(&haystack),
        // Fallback for invalid patterns that couldn't be compiled at parse time.
        Literal::Str(pattern) => Regex::new(pattern)
            .map(|re| re.is_match(&haystack))
            .unwrap_or(false),
        _ => false,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn value_to_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

fn value_as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn lit_as_f64(lit: &Literal) -> Option<f64> {
    match lit {
        Literal::Num(n) => Some(*n),
        Literal::Str(s) => s.parse().ok(),
        _ => None,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::dsl::parser;

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

    fn run(expr: &str, jsons: &[&str]) -> Vec<Record> {
        let (q, _) = parser::parse(expr).unwrap();
        eval(&q, make_records(jsons)).unwrap().0
    }

    #[test]
    fn eq_string_filter() {
        let r = run(
            r#".level == "error""#,
            &[r#"{"level":"error"}"#, r#"{"level":"info"}"#],
        );
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].fields["level"], Value::String("error".into()));
    }

    #[test]
    fn gt_numeric_filter() {
        let r = run(".status > 400", &[r#"{"status":500}"#, r#"{"status":200}"#]);
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn and_filter() {
        let r = run(
            r#".level == "error" and .service == "api""#,
            &[
                r#"{"level":"error","service":"api"}"#,
                r#"{"level":"error","service":"web"}"#,
            ],
        );
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn or_filter() {
        let r = run(
            r#".level == "error" or .level == "warn""#,
            &[
                r#"{"level":"error"}"#,
                r#"{"level":"warn"}"#,
                r#"{"level":"info"}"#,
            ],
        );
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn not_filter() {
        let r = run(
            r#"not .level == "info""#,
            &[r#"{"level":"error"}"#, r#"{"level":"info"}"#],
        );
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn exists_filter() {
        let r = run(".error exists", &[r#"{"error":"oops"}"#, r#"{"msg":"ok"}"#]);
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn contains_filter() {
        let r = run(
            r#".msg contains "time""#,
            &[r#"{"msg":"request timeout"}"#, r#"{"msg":"ok"}"#],
        );
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn matches_regex() {
        let r = run(
            r#".msg matches "time.*""#,
            &[r#"{"msg":"timeout error"}"#, r#"{"msg":"ok"}"#],
        );
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn pick_stage() {
        let r = run(
            r#".level == "error" | pick(.level, .msg)"#,
            &[r#"{"level":"error","msg":"oops","ts":"2024"}"#],
        );
        assert_eq!(r[0].fields.len(), 2);
        assert!(r[0].fields.contains_key("level"));
        assert!(r[0].fields.contains_key("msg"));
        assert!(!r[0].fields.contains_key("ts"));
    }

    #[test]
    fn omit_stage() {
        let r = run(
            r#".level == "error" | omit(.ts)"#,
            &[r#"{"level":"error","msg":"oops","ts":"2024"}"#],
        );
        assert!(!r[0].fields.contains_key("ts"));
        assert!(r[0].fields.contains_key("msg"));
    }

    #[test]
    fn count_stage() {
        let r = run(
            ".level == \"error\" | count()",
            &[
                r#"{"level":"error"}"#,
                r#"{"level":"error"}"#,
                r#"{"level":"info"}"#,
            ],
        );
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].fields["count"], Value::Number(2.into()));
    }

    #[test]
    fn sort_by_desc() {
        let r = run(
            ".n > 0 | sort_by(.n desc)",
            &[r#"{"n":1}"#, r#"{"n":3}"#, r#"{"n":2}"#],
        );
        assert_eq!(r[0].fields["n"], Value::Number(3.into()));
    }

    #[test]
    fn group_by_stage() {
        let r = run(
            ".level == \"error\" | group_by(.service)",
            &[
                r#"{"level":"error","service":"api"}"#,
                r#"{"level":"error","service":"api"}"#,
                r#"{"level":"error","service":"web"}"#,
            ],
        );
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].fields["service"], Value::String("api".into()));
        assert_eq!(r[0].fields["count"], Value::Number(2.into()));
    }

    #[test]
    fn limit_stage() {
        let r = run(
            ".n > 0 | limit(2)",
            &[r#"{"n":1}"#, r#"{"n":2}"#, r#"{"n":3}"#],
        );
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn skip_stage() {
        let r = run(
            ".n > 0 | skip(1)",
            &[r#"{"n":1}"#, r#"{"n":2}"#, r#"{"n":3}"#],
        );
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].fields["n"], Value::Number(2.into()));
    }

    #[test]
    fn dedup_stage() {
        let r = run(
            ".n > 0 | dedup(.svc)",
            &[
                r#"{"n":1,"svc":"api"}"#,
                r#"{"n":2,"svc":"api"}"#,
                r#"{"n":3,"svc":"web"}"#,
            ],
        );
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn sum_stage() {
        let r = run(
            ".n > 0 | sum(.n)",
            &[r#"{"n":1}"#, r#"{"n":2}"#, r#"{"n":3}"#],
        );
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].fields["sum"].as_f64().unwrap(), 6.0);
    }

    #[test]
    fn avg_stage() {
        let r = run(
            ".n > 0 | avg(.n)",
            &[r#"{"n":1}"#, r#"{"n":2}"#, r#"{"n":3}"#],
        );
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].fields["avg"].as_f64().unwrap(), 2.0);
    }

    #[test]
    fn min_stage() {
        let r = run(
            ".n > 0 | min(.n)",
            &[r#"{"n":5}"#, r#"{"n":2}"#, r#"{"n":8}"#],
        );
        assert_eq!(r[0].fields["min"].as_f64().unwrap(), 2.0);
    }

    #[test]
    fn max_stage() {
        let r = run(
            ".n > 0 | max(.n)",
            &[r#"{"n":5}"#, r#"{"n":2}"#, r#"{"n":8}"#],
        );
        assert_eq!(r[0].fields["max"].as_f64().unwrap(), 8.0);
    }

    #[test]
    fn group_by_time_stage() {
        // Two records in the same 5-minute bucket, one in a different bucket
        let r = run(
            ".level == \"error\" | group_by_time(.ts, \"5m\")",
            &[
                r#"{"level":"error","ts":"2024-01-15T10:02:00Z"}"#,
                r#"{"level":"error","ts":"2024-01-15T10:04:00Z"}"#,
                r#"{"level":"error","ts":"2024-01-15T10:07:00Z"}"#,
            ],
        );
        // First two fall in the 10:00 bucket, third falls in 10:05 bucket
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].fields["count"], Value::Number(2.into()));
        assert_eq!(r[1].fields["count"], Value::Number(1.into()));
        // Buckets are RFC 3339 strings
        assert!(r[0].fields["bucket"].as_str().unwrap().contains('T'));
    }

    #[test]
    fn group_by_time_epoch_secs() {
        // 1705312800 is exactly on an hour boundary.
        // +100 and +1200 fall in the same 1h bucket; +3600 falls in the next.
        let r = run(
            ".n > 0 | group_by_time(.ts, \"1h\")",
            &[
                r#"{"n":1,"ts":1705312900}"#, // +100s → same bucket as 1705312800
                r#"{"n":2,"ts":1705314000}"#, // +1200s → same bucket as 1705312800
                r#"{"n":3,"ts":1705316400}"#, // +3600s → next hour bucket
            ],
        );
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].fields["count"], Value::Number(2.into()));
        assert_eq!(r[1].fields["count"], Value::Number(1.into()));
    }

    #[test]
    fn nested_field_access() {
        let r = run(
            ".response.status == 503",
            &[
                r#"{"response":{"status":503}}"#,
                r#"{"response":{"status":200}}"#,
            ],
        );
        assert_eq!(r.len(), 1);
    }
}

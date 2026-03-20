use indexmap::IndexMap;
use serde_json::Value;

use crate::record::{Record, SourceInfo};
use crate::util::error::Result;

use super::parser::{Aggregation, FastQuery, FilterExpr, FilterOp, LogicalOp, SortExpr, SortOrder};

/// Apply a `FastQuery` to a list of records, returning the result set.
pub fn eval(query: &FastQuery, records: Vec<Record>) -> Result<Vec<Record>> {
    let filtered = filter_records(query, records)?;

    if let Some(agg) = &query.aggregation {
        return aggregate(agg, filtered);
    }

    let projected = apply_projection(&query.projection, filtered);
    let sorted = apply_sort(&query.sort, projected)?;
    let limited = apply_limit(query.limit, sorted);

    Ok(limited)
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
fn eval_filters(
    filters: &[FilterExpr],
    ops: &[LogicalOp],
    rec: &Record,
) -> Result<bool> {
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
        FilterOp::Regex => eval_regex(val, &f.value),
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

fn eval_regex(val: &Value, pattern: &str) -> Result<bool> {
    use regex::Regex;
    use crate::util::error::QkError;
    let re = Regex::new(pattern)
        .map_err(|e| QkError::Query(format!("invalid regex '{pattern}': {e}")))?;
    Ok(re.is_match(&value_to_str(val)))
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
            let kept: IndexMap<String, Value> = fields
                .iter()
                .filter_map(|f| rec.fields.swap_remove(f).map(|v| (f.clone(), v)))
                .collect();
            rec.fields = kept;
            rec
        })
        .collect()
}

// ── Aggregation ───────────────────────────────────────────────────────────────

fn aggregate(agg: &Aggregation, records: Vec<Record>) -> Result<Vec<Record>> {
    match agg {
        Aggregation::Count => {
            let mut fields: IndexMap<String, Value> = IndexMap::new();
            fields.insert("count".to_string(), Value::Number(records.len().into()));
            Ok(vec![Record::new(fields, String::new(), SourceInfo::default())])
        }
        Aggregation::CountBy(group_field) => count_by(group_field, records),
        Aggregation::Fields => fields_discovery(records),
        Aggregation::Sum(field) => stat_agg("sum", field, &records, |nums| nums.iter().sum()),
        Aggregation::Avg(field) => stat_agg("avg", field, &records, |nums| {
            if nums.is_empty() { 0.0 } else { nums.iter().sum::<f64>() / nums.len() as f64 }
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
            seen.insert(key.clone());
        }
    }
    let result = seen
        .into_iter()
        .map(|field| {
            let mut fields: IndexMap<String, Value> = IndexMap::new();
            fields.insert("field".to_string(), Value::String(field));
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
) -> Result<Vec<Record>> {
    let nums: Vec<f64> = records
        .iter()
        .filter_map(|r| r.get(field).and_then(value_as_f64))
        .collect();
    let result = f(nums);
    let rounded = (result * 1_000_000.0).round() / 1_000_000.0;
    let json_num = serde_json::Number::from_f64(rounded)
        .unwrap_or_else(|| serde_json::Number::from(0));
    let mut fields: IndexMap<String, Value> = IndexMap::new();
    fields.insert(key.to_string(), Value::Number(json_num));
    Ok(vec![Record::new(fields, String::new(), SourceInfo::default())])
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
            let mut fields: IndexMap<String, Value> = IndexMap::new();
            fields.insert(field.to_string(), Value::String(key));
            fields.insert("count".to_string(), Value::Number(count.into()));
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
                    Value::Object(m) => m.into_iter().collect(),
                    _ => IndexMap::new(),
                };
                Record::new(fields, s.to_string(), SourceInfo::default())
            })
            .collect()
    }

    fn run(tokens: &[&str], jsons: &[&str]) -> Vec<Record> {
        let toks: Vec<String> = tokens.iter().map(|s| s.to_string()).collect();
        let (q, _) = parser::parse(&toks).unwrap();
        eval(&q, make_records(jsons)).unwrap()
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
        let result = run(
            &["count"],
            &[r#"{"a":1}"#, r#"{"a":2}"#, r#"{"a":3}"#],
        );
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
        let result = run(
            &["limit", "2"],
            &[r#"{"a":1}"#, r#"{"a":2}"#, r#"{"a":3}"#],
        );
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
            &[r#"{"level":"error","msg":"a"}"#, r#"{"level":"info","ts":"x"}"#],
        );
        // Should return sorted unique field names
        let names: Vec<&str> = result.iter().map(|r| {
            r.fields["field"].as_str().unwrap()
        }).collect();
        assert_eq!(names, vec!["level", "msg", "ts"]);
    }

    #[test]
    fn sum_field() {
        let result = run(
            &["sum", "n"],
            &[r#"{"n":1}"#, r#"{"n":2}"#, r#"{"n":3}"#],
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].fields["sum"].as_f64().unwrap(), 6.0);
    }

    #[test]
    fn avg_field() {
        let result = run(
            &["avg", "n"],
            &[r#"{"n":1}"#, r#"{"n":2}"#, r#"{"n":3}"#],
        );
        assert_eq!(result[0].fields["avg"].as_f64().unwrap(), 2.0);
    }

    #[test]
    fn min_field() {
        let result = run(
            &["min", "n"],
            &[r#"{"n":5}"#, r#"{"n":2}"#, r#"{"n":8}"#],
        );
        assert_eq!(result[0].fields["min"].as_f64().unwrap(), 2.0);
    }

    #[test]
    fn max_field() {
        let result = run(
            &["max", "n"],
            &[r#"{"n":5}"#, r#"{"n":2}"#, r#"{"n":8}"#],
        );
        assert_eq!(result[0].fields["max"].as_f64().unwrap(), 8.0);
    }

    #[test]
    fn head_is_alias_for_limit() {
        let result = run(
            &["head", "2"],
            &[r#"{"a":1}"#, r#"{"a":2}"#, r#"{"a":3}"#],
        );
        assert_eq!(result.len(), 2);
    }
}

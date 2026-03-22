use std::sync::Arc;

use regex::Regex;

use crate::util::error::{QkError, Result};
use crate::util::time::looks_like_duration;

/// A complete parsed fast-layer query (keyword syntax).
#[derive(Debug, Default)]
pub struct FastQuery {
    /// Filter predicates (connected by `logical_ops`).
    pub filters: Vec<FilterExpr>,
    /// Logical operators between consecutive filters (`and` / `or`).
    pub logical_ops: Vec<LogicalOp>,
    /// Field names to keep (`select f1 f2 ...`).
    pub projection: Option<Vec<String>>,
    /// Aggregation operation.
    pub aggregation: Option<Aggregation>,
    /// Sort expression.
    pub sort: Option<SortExpr>,
    /// Maximum number of records to return.
    pub limit: Option<usize>,
}

/// A single filter predicate: `field OP value`.
#[derive(Debug, Clone)]
pub struct FilterExpr {
    pub field: String,
    pub op: FilterOp,
    pub value: String,
    /// Pre-compiled regex for `Regex` and `Glob` ops; `None` for all other ops.
    /// Compiled once at parse time — never recompiled per record.
    pub compiled: Option<Arc<Regex>>,
    /// Byte offset range `(start, end)` of this filter's primary token in the
    /// space-joined query string. Reserved for future tooling (e.g. --explain output).
    #[allow(dead_code)]
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterOp {
    Eq,
    Ne,
    Gt,
    Lt,
    Gte,
    Lte,
    Regex,
    Contains,
    Exists,
    StartsWith,
    EndsWith,
    Glob,
    /// `field between LOW HIGH` — inclusive range check (numeric or lexicographic).
    Between,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
}

#[derive(Debug)]
pub enum Aggregation {
    Count,
    CountBy(Vec<String>),
    /// Count distinct values of a field → `{"count_unique": N}`.
    CountUnique(String),
    /// Group records into time buckets and emit `{bucket, count}` per bucket.
    ///
    /// `bucket` is a duration string like `"5m"`, `"1h"`.
    /// `field` is the timestamp field name (default `"ts"`).
    /// `asc` controls output order: `true` = ascending, `false` (default) = descending.
    GroupByTime {
        bucket: String,
        field: String,
        /// If true, output ascending; if false (default), descending.
        asc: bool,
    },
    /// Discover all field names present across records.
    Fields,
    /// Count JSON-type distribution of a field → one `{type, count}` record per type.
    TypeCount(String),
    Sum(String),
    Avg(String),
    Min(String),
    Max(String),
}

#[derive(Debug)]
pub struct SortExpr {
    pub field: String,
    pub order: SortOrder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Parse keyword query tokens into a `FastQuery`.
///
/// Returns `(FastQuery, file_paths)`. Everything after the recognized query
/// keywords is treated as file paths.
/// Uses `"ts"` as the default timestamp field for `count by DURATION`.
pub fn parse(tokens: &[String]) -> Result<(FastQuery, Vec<String>)> {
    parse_with_defaults(tokens, "ts")
}

/// Parse keyword query tokens, using `default_time_field` as the fallback
/// timestamp field name for `count by DURATION` when no explicit field is given.
pub fn parse_with_defaults(
    tokens: &[String],
    default_time_field: &str,
) -> Result<(FastQuery, Vec<String>)> {
    parse_inner(tokens, default_time_field)
}

/// Internal parser implementation; called by `parse` and `parse_with_defaults`.
fn parse_inner(tokens: &[String], default_time_field: &str) -> Result<(FastQuery, Vec<String>)> {
    let mut q = FastQuery::default();
    let mut i = 0;

    while i < tokens.len() {
        match tokens[i].to_ascii_lowercase().as_str() {
            "where" => {
                i += 1;
                i = parse_where_clause(tokens, i, &mut q)?;
            }
            "select" => {
                i += 1;
                i = parse_select(tokens, i, &mut q);
            }
            "count" => {
                i += 1;
                i = parse_count(tokens, i, &mut q, default_time_field);
            }
            "fields" => {
                q.aggregation = Some(Aggregation::Fields);
                i += 1;
            }
            "sum" => {
                i += 1;
                i = parse_stat(tokens, i, &mut q, Aggregation::Sum)?;
            }
            "avg" => {
                i += 1;
                i = parse_stat(tokens, i, &mut q, Aggregation::Avg)?;
            }
            "min" => {
                i += 1;
                i = parse_stat(tokens, i, &mut q, Aggregation::Min)?;
            }
            "max" => {
                i += 1;
                i = parse_stat(tokens, i, &mut q, Aggregation::Max)?;
            }
            "sort" => {
                i += 1;
                i = parse_sort(tokens, i, &mut q)?;
            }
            "limit" | "head" => {
                i += 1;
                i = parse_limit(tokens, i, &mut q)?;
            }
            _ => break,
        }
    }

    let files = tokens[i..].to_vec();
    Ok((q, files))
}

/// Parse one or more filter predicates after `where`, connected by `and`/`or`/`,`.
///
/// Comma is a visual alias for `and`:
///   `where level=error, service=api`   (comma attached to previous token)
///   `where level=error , service=api`  (comma as separate token)
fn parse_where_clause(tokens: &[String], mut i: usize, q: &mut FastQuery) -> Result<usize> {
    loop {
        if i >= tokens.len() {
            break;
        }
        let (filter, consumed) = parse_filter(tokens, i)?;

        // Detect trailing comma on the last consumed token (e.g. "level=error,")
        let last_consumed = i + consumed - 1;
        let trailing_comma = tokens
            .get(last_consumed)
            .map(|t| t.ends_with(','))
            .unwrap_or(false);

        q.filters.push(filter);
        i += consumed;

        if trailing_comma {
            // Trailing comma on a token acts as `and` — but only if the next token is
            // another filter expression. If the next token is a clause keyword (select,
            // count, sort, …) or a file path, the trailing comma is just cosmetic punctuation
            // and we stop here (e.g. `where level=error, select ts msg` is valid).
            let next_is_clause_end = tokens
                .get(i)
                .map(|t| {
                    let lc = t.to_ascii_lowercase();
                    matches!(
                        lc.as_str(),
                        "select"
                            | "count"
                            | "sort"
                            | "limit"
                            | "head"
                            | "fields"
                            | "sum"
                            | "avg"
                            | "min"
                            | "max"
                            | "where"
                    ) || looks_like_file(t)
                })
                .unwrap_or(true); // end of token stream

            if next_is_clause_end {
                break;
            }
            q.logical_ops.push(LogicalOp::And);
            continue;
        }

        // Look for `and` / `or` / `,` connector token
        match tokens.get(i).map(|s| s.to_ascii_lowercase()) {
            Some(ref s) if s == "and" || s == "," => {
                q.logical_ops.push(LogicalOp::And);
                i += 1;
            }
            Some(ref s) if s == "or" => {
                q.logical_ops.push(LogicalOp::Or);
                i += 1;
            }
            _ => break,
        }
    }
    Ok(i)
}

/// Parse a single filter expression starting at `tokens[i]`.
///
/// Returns `(FilterExpr, tokens_consumed)`.
/// Trailing commas on tokens are stripped to support `level=error, service=api` syntax.
fn parse_filter(tokens: &[String], i: usize) -> Result<(FilterExpr, usize)> {
    let raw_tok = tokens.get(i).ok_or_else(|| {
        query_error_with_hint(tokens, i, "expected filter expression after 'where'")
    })?;
    // Strip trailing comma so `level=error,` is treated the same as `level=error`
    let tok = raw_tok.trim_end_matches(',');
    let span = token_span(tokens, i);

    // Detect == (double equals) — common mistake; qk uses single = for equality
    if let Some(pos) = tok.find("==") {
        if pos > 0 && !tok.starts_with("!=") {
            let field = &tok[..pos];
            let value = &tok[pos + 2..];
            return Err(QkError::Query(format!(
                "invalid operator '==' in '{tok}'\n  \
                 qk uses a single '=' for equality\n  \
                 Hint: try `where {field}={value}`"
            )));
        }
    }

    // Try embedded operators in priority order (longer first to avoid mis-parse)
    for op_str in &["!=", ">=", "<=", "~=", "=", ">", "<"] {
        if let Some(pos) = tok.find(op_str) {
            let field = tok[..pos].to_string();
            let value = tok[pos + op_str.len()..].to_string();
            if field.is_empty() {
                continue;
            }
            let op = match *op_str {
                "=" => FilterOp::Eq,
                "!=" => FilterOp::Ne,
                ">" => FilterOp::Gt,
                "<" => FilterOp::Lt,
                ">=" => FilterOp::Gte,
                "<=" => FilterOp::Lte,
                "~=" => FilterOp::Regex,
                _ => unreachable!(),
            };
            return Ok((build_filter(field, op, value, span)?, 1));
        }
    }

    // Multi-token operators (avoid shell metacharacter issues with > and <)
    if let Some(next) = tokens.get(i + 1) {
        // Strip trailing comma from the next token too (handles "contains,")
        let next_clean = next.trim_end_matches(',');
        match next_clean.to_ascii_lowercase().as_str() {
            // Word aliases for comparison operators: safe to type without quoting
            "gt" | "lt" | "gte" | "lte" | "eq" | "ne" => {
                let op = match next_clean.to_ascii_lowercase().as_str() {
                    "gt" => FilterOp::Gt,
                    "lt" => FilterOp::Lt,
                    "gte" => FilterOp::Gte,
                    "lte" => FilterOp::Lte,
                    "eq" => FilterOp::Eq,
                    "ne" => FilterOp::Ne,
                    _ => unreachable!(),
                };
                let value = tokens
                    .get(i + 2)
                    .map(|v| v.trim_end_matches(',').to_string())
                    .unwrap_or_default();
                return Ok((build_filter(tok.to_string(), op, value, span)?, 3));
            }
            "contains" => {
                let value = tokens
                    .get(i + 2)
                    .map(|v| v.trim_end_matches(',').to_string())
                    .unwrap_or_default();
                return Ok((
                    build_filter(tok.to_string(), FilterOp::Contains, value, span)?,
                    3,
                ));
            }
            "exists" => {
                return Ok((
                    build_filter(tok.to_string(), FilterOp::Exists, String::new(), span)?,
                    2,
                ));
            }
            "startswith" => {
                let value = tokens
                    .get(i + 2)
                    .map(|v| v.trim_end_matches(',').to_string())
                    .unwrap_or_default();
                return Ok((
                    build_filter(tok.to_string(), FilterOp::StartsWith, value, span)?,
                    3,
                ));
            }
            "endswith" => {
                let value = tokens
                    .get(i + 2)
                    .map(|v| v.trim_end_matches(',').to_string())
                    .unwrap_or_default();
                return Ok((
                    build_filter(tok.to_string(), FilterOp::EndsWith, value, span)?,
                    3,
                ));
            }
            "glob" => {
                let value = tokens
                    .get(i + 2)
                    .map(|v| v.trim_end_matches(',').to_string())
                    .unwrap_or_default();
                return Ok((
                    build_filter(tok.to_string(), FilterOp::Glob, value, span)?,
                    3,
                ));
            }
            "between" => {
                // `field between LOW HIGH` — consumes 4 tokens total
                let low = tokens
                    .get(i + 2)
                    .map(|v| v.trim_end_matches(',').to_string())
                    .unwrap_or_default();
                let high = tokens
                    .get(i + 3)
                    .map(|v| v.trim_end_matches(',').to_string())
                    .unwrap_or_default();
                // Store as "LOW\x00HIGH" — the null byte is not valid in JSON values
                let combined = format!("{low}\x00{high}");
                return Ok((
                    build_filter(tok.to_string(), FilterOp::Between, combined, span)?,
                    4,
                ));
            }
            _ => {}
        }
    }

    Err(query_error_with_hint(
        tokens,
        i,
        &format!(
            "cannot parse filter '{raw_tok}': expected FIELD=VALUE, FIELD!=VALUE, FIELD>VALUE, \
             FIELD contains TEXT, or FIELD exists\n  \
             hint: shell metacharacters must be quoted — use 'latency>100' or word operators: \
             latency gt 100  /  latency lt 50  /  latency gte 200  /  latency lte 500"
        ),
    ))
}

/// Parse `select FIELD [FIELD...]`, stopping at the next keyword or file-like arg.
fn parse_select(tokens: &[String], mut i: usize, q: &mut FastQuery) -> usize {
    let mut fields = Vec::new();
    while i < tokens.len() {
        let tok = &tokens[i];
        if is_query_keyword(tok) || looks_like_file(tok) {
            break;
        }
        fields.push(tok.clone());
        i += 1;
    }
    if !fields.is_empty() {
        q.projection = Some(fields);
    }
    i
}

/// Parse `count [by FIELD|DURATION [FIELD] [asc|desc]]`.
///
/// - `count` → total count
/// - `count by level` → count_by("level")
/// - `count by 5m` → group_by_time("5m", default_time_field)
/// - `count by 5m ts` / `count by 5m @timestamp` → group_by_time("5m", field)
/// - `count by 5m ts asc` → group_by_time ascending
fn parse_count(
    tokens: &[String],
    mut i: usize,
    q: &mut FastQuery,
    default_time_field: &str,
) -> usize {
    if tokens.get(i).map(|s| s.to_ascii_lowercase()) == Some("by".to_string()) {
        i += 1;
        if let Some(arg) = tokens.get(i) {
            if looks_like_duration(arg) {
                // Time-bucket mode: `count by 5m [FIELD] [asc|desc]`
                let bucket = arg.to_string();
                i += 1;
                let field = tokens
                    .get(i)
                    .filter(|f| {
                        !looks_like_file(f)
                            && !is_query_keyword(f)
                            && !f.eq_ignore_ascii_case("asc")
                            && !f.eq_ignore_ascii_case("desc")
                    })
                    .cloned()
                    .unwrap_or_else(|| default_time_field.to_string());
                if tokens
                    .get(i)
                    .filter(|f| {
                        !looks_like_file(f)
                            && !is_query_keyword(f)
                            && !f.eq_ignore_ascii_case("asc")
                            && !f.eq_ignore_ascii_case("desc")
                    })
                    .is_some()
                {
                    i += 1;
                }
                // Check for optional asc/desc after the field name
                let asc = match tokens.get(i).map(|s| s.to_ascii_lowercase()).as_deref() {
                    Some("asc") => {
                        i += 1;
                        true
                    }
                    Some("desc") => {
                        i += 1;
                        false
                    }
                    _ => false, // default: descending
                };
                q.aggregation = Some(Aggregation::GroupByTime { bucket, field, asc });
                return i;
            } else if !looks_like_file(arg) && !is_query_keyword(arg) {
                // Collect one or more field names (space- or comma-separated)
                let mut fields: Vec<String> = Vec::new();
                while let Some(tok) = tokens.get(i) {
                    if looks_like_file(tok) || is_query_keyword(tok) || looks_like_duration(tok) {
                        break;
                    }
                    let clean = tok.trim_end_matches(',').to_string();
                    if !clean.is_empty() {
                        fields.push(clean);
                    }
                    i += 1;
                }
                if fields.is_empty() {
                    q.aggregation = Some(Aggregation::Count);
                } else {
                    q.aggregation = Some(Aggregation::CountBy(fields));
                }
                return i;
            }
        }
        q.aggregation = Some(Aggregation::Count);
    } else if tokens.get(i).map(|s| s.to_ascii_lowercase()) == Some("unique".to_string()) {
        // count unique FIELD
        i += 1;
        if let Some(field) = tokens
            .get(i)
            .filter(|f| !looks_like_file(f) && !is_query_keyword(f))
        {
            q.aggregation = Some(Aggregation::CountUnique(field.clone()));
            return i + 1;
        }
        q.aggregation = Some(Aggregation::Count);
    } else if tokens.get(i).map(|s| s.to_ascii_lowercase()) == Some("types".to_string()) {
        // count types FIELD
        i += 1;
        if let Some(field) = tokens
            .get(i)
            .filter(|f| !looks_like_file(f) && !is_query_keyword(f))
        {
            q.aggregation = Some(Aggregation::TypeCount(field.clone()));
            return i + 1;
        }
        q.aggregation = Some(Aggregation::TypeCount("*".to_string()));
    } else {
        q.aggregation = Some(Aggregation::Count);
    }
    i
}

/// Parse `sort FIELD [asc|desc]`.
fn parse_sort(tokens: &[String], mut i: usize, q: &mut FastQuery) -> Result<usize> {
    let field = tokens
        .get(i)
        .ok_or_else(|| query_error_with_hint(tokens, i, "expected field name after 'sort'"))?;

    if looks_like_file(field) || is_query_keyword(field) {
        return Err(query_error_with_hint(
            tokens,
            i,
            "expected field name after 'sort'",
        ));
    }

    i += 1;
    let next_lc = tokens.get(i).map(|s| s.to_ascii_lowercase());
    let order = match next_lc.as_deref() {
        Some("desc") => {
            i += 1;
            SortOrder::Desc
        }
        Some("asc") => {
            i += 1;
            SortOrder::Asc
        }
        Some(other) if !looks_like_file(other) && !is_query_keyword(other) => {
            return Err(query_error_with_hint(
                tokens,
                i,
                &format!("unknown sort direction '{other}': expected 'asc' or 'desc'"),
            ));
        }
        _ => SortOrder::Asc,
    };

    q.sort = Some(SortExpr {
        field: field.clone(),
        order,
    });
    Ok(i)
}

/// Parse `limit N`.
fn parse_limit(tokens: &[String], mut i: usize, q: &mut FastQuery) -> Result<usize> {
    let n_str = tokens
        .get(i)
        .ok_or_else(|| query_error_with_hint(tokens, i, "expected number after 'limit'"))?;
    let n: usize = n_str.parse().map_err(|_| {
        query_error_with_hint(
            tokens,
            i,
            &format!("'limit' expects a positive integer, got '{n_str}'"),
        )
    })?;
    q.limit = Some(n);
    i += 1;
    Ok(i)
}

/// Parse `STAT FIELD` — e.g. `sum latency`, `avg duration`.
fn parse_stat(
    tokens: &[String],
    mut i: usize,
    q: &mut FastQuery,
    make: impl Fn(String) -> Aggregation,
) -> Result<usize> {
    let field = tokens.get(i).ok_or_else(|| {
        query_error_with_hint(tokens, i, "expected field name after stat keyword")
    })?;
    if looks_like_file(field) || is_query_keyword(field) {
        return Err(query_error_with_hint(
            tokens,
            i,
            "expected field name after stat keyword",
        ));
    }
    q.aggregation = Some(make(field.clone()));
    i += 1;
    Ok(i)
}

fn is_query_keyword(s: &str) -> bool {
    matches!(
        s.to_ascii_lowercase().as_str(),
        "where"
            | "select"
            | "count"
            | "sort"
            | "limit"
            | "head"
            | "by"
            | "and"
            | "or"
            | "uniq"
            | "unique"
            | "fields"
            | "sum"
            | "avg"
            | "min"
            | "max"
            // word comparison operators (safe shell alternatives to >, <, >=, <=)
            | "gt"
            | "lt"
            | "gte"
            | "lte"
            | "eq"
            | "ne"
            | "startswith"
            | "endswith"
            | "glob"
            | "between"
            | "contains"
            | "exists"
            | "types"
    )
}

/// Convert a shell-style glob pattern to a regex string.
///
/// - `*`  → `.*`  (any sequence)
/// - `?`  → `.`   (any single character)
/// - All other regex metacharacters are escaped.
/// - Pattern is case-insensitive and fully anchored: `(?i)^...$`
pub fn glob_to_regex(glob: &str) -> String {
    let mut re = String::from("(?i)^");
    for ch in glob.chars() {
        match ch {
            '*' => re.push_str(".*"),
            '?' => re.push('.'),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                re.push('\\');
                re.push(ch);
            }
            c => re.push(c),
        }
    }
    re.push('$');
    re
}

/// Format a query error with a `^` pointer to the offending token.
///
/// `token_idx` is the index of the problematic token in the `tokens` slice.
/// Builds a human-readable error like:
/// ```text
/// unexpected token 'gte'
///   where level=error gte 5
///                     ^^^
/// ```
pub fn query_error_with_hint(tokens: &[String], token_idx: usize, msg: &str) -> QkError {
    let query = tokens.join(" ");
    let offset: usize = tokens[..token_idx.min(tokens.len())]
        .iter()
        .map(|t| t.len() + 1)
        .sum();
    let width = tokens.get(token_idx).map(|t| t.len()).unwrap_or(1).max(1);
    let pointer = format!("{}{}", " ".repeat(offset), "^".repeat(width));
    QkError::Query(format!("{msg}\n  {query}\n  {pointer}"))
}

/// Compute the byte span `(start, end)` of `tokens[idx]` in the space-joined query string.
fn token_span(tokens: &[String], idx: usize) -> (usize, usize) {
    let start: usize = tokens[..idx].iter().map(|t| t.len() + 1).sum();
    let end = start + tokens.get(idx).map(|t| t.len()).unwrap_or(0);
    (start, end)
}

/// Construct a `FilterExpr`, pre-compiling the regex for `Regex` and `Glob` ops.
fn build_filter(
    field: impl Into<String>,
    op: FilterOp,
    value: impl Into<String>,
    span: (usize, usize),
) -> Result<FilterExpr> {
    let field = field.into();
    let value = value.into();
    let compiled = match op {
        FilterOp::Regex => {
            let re = Regex::new(&value)
                .map_err(|e| QkError::Query(format!("invalid regex '{value}': {e}")))?;
            Some(Arc::new(re))
        }
        FilterOp::Glob => {
            let re_pat = glob_to_regex(&value);
            let re = Regex::new(&re_pat)
                .map_err(|e| QkError::Query(format!("invalid glob '{value}': {e}")))?;
            Some(Arc::new(re))
        }
        _ => None,
    };
    Ok(FilterExpr {
        field,
        op,
        value,
        compiled,
        span,
    })
}

/// Heuristic: does this token look like a file path rather than a field name?
pub fn looks_like_file(s: &str) -> bool {
    s.contains('/')
        || s.contains('*')
        || s.contains('?')
        || s.ends_with(".log")
        || s.ends_with(".json")
        || s.ends_with(".ndjson")
        || s.ends_with(".csv")
        || s.ends_with(".tsv")
        || s.ends_with(".yaml")
        || s.ends_with(".yml")
        || s.ends_with(".toml")
        || s.ends_with(".txt")
        || s == "-" // stdin marker
        || std::path::Path::new(s).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_empty_query() {
        let (q, files) = parse(&tok(&["app.log"])).unwrap();
        assert!(q.filters.is_empty());
        assert_eq!(files, vec!["app.log"]);
    }

    #[test]
    fn parses_where_eq() {
        let (q, files) = parse(&tok(&["where", "level=error", "app.log"])).unwrap();
        assert_eq!(q.filters.len(), 1);
        assert_eq!(q.filters[0].field, "level");
        assert!(matches!(q.filters[0].op, FilterOp::Eq));
        assert_eq!(q.filters[0].value, "error");
        assert_eq!(files, vec!["app.log"]);
    }

    #[test]
    fn parses_where_select() {
        let (q, _) = parse(&tok(&[
            "where",
            "level=error",
            "select",
            "ts",
            "msg",
            "app.log",
        ]))
        .unwrap();
        assert_eq!(
            q.projection,
            Some(vec!["ts".to_string(), "msg".to_string()])
        );
    }

    #[test]
    fn parses_count_by() {
        let (q, _) = parse(&tok(&["count", "by", "service", "app.log"])).unwrap();
        assert!(
            matches!(q.aggregation, Some(Aggregation::CountBy(ref f)) if f == &vec!["service".to_string()])
        );
    }

    #[test]
    fn parses_sort_desc() {
        let (q, _) = parse(&tok(&["sort", "latency", "desc", "app.log"])).unwrap();
        let sort = q.sort.unwrap();
        assert_eq!(sort.field, "latency");
        assert_eq!(sort.order, SortOrder::Desc);
    }

    #[test]
    fn parses_limit() {
        let (q, _) = parse(&tok(&["limit", "10", "app.log"])).unwrap();
        assert_eq!(q.limit, Some(10));
    }

    #[test]
    fn parses_and_or() {
        let (q, _) = parse(&tok(&[
            "where",
            "level=error",
            "and",
            "service=api",
            "app.log",
        ]))
        .unwrap();
        assert_eq!(q.filters.len(), 2);
        assert_eq!(q.logical_ops, vec![LogicalOp::And]);
    }

    #[test]
    fn parses_exists() {
        let (q, _) = parse(&tok(&["where", "error", "exists", "app.log"])).unwrap();
        assert!(matches!(q.filters[0].op, FilterOp::Exists));
    }

    #[test]
    fn parses_contains() {
        let (q, _) = parse(&tok(&["where", "msg", "contains", "timeout", "app.log"])).unwrap();
        assert!(matches!(q.filters[0].op, FilterOp::Contains));
        assert_eq!(q.filters[0].value, "timeout");
    }

    #[test]
    fn parses_startswith() {
        let (q, _) = parse(&tok(&["where", "path", "startswith", "/api", "app.log"])).unwrap();
        assert!(matches!(q.filters[0].op, FilterOp::StartsWith));
        assert_eq!(q.filters[0].field, "path");
        assert_eq!(q.filters[0].value, "/api");
    }

    #[test]
    fn parses_endswith() {
        let (q, _) = parse(&tok(&["where", "file", "endswith", ".log", "app.log"])).unwrap();
        assert!(matches!(q.filters[0].op, FilterOp::EndsWith));
        assert_eq!(q.filters[0].field, "file");
        assert_eq!(q.filters[0].value, ".log");
    }

    #[test]
    fn parses_glob() {
        let (q, _) = parse(&tok(&["where", "name", "glob", "al*", "app.log"])).unwrap();
        assert!(matches!(q.filters[0].op, FilterOp::Glob));
        assert_eq!(q.filters[0].field, "name");
        assert_eq!(q.filters[0].value, "al*");
    }

    #[test]
    fn error_includes_caret_pointer() {
        // "where BADTOKEN" — parse error should include ^^^ pointer
        let result = parse(&tok(&["where", "BADTOKEN"]));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains('^'), "expected caret pointer in: {msg}");
        assert!(msg.contains("BADTOKEN"), "expected token name in: {msg}");
    }

    #[test]
    fn sort_bad_direction_includes_caret_pointer() {
        let result = parse(&tok(&["sort", "latency", "sideways"]));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains('^'), "expected caret pointer in: {msg}");
        assert!(msg.contains("sideways"), "expected bad token in: {msg}");
    }

    #[test]
    fn filter_span_is_set() {
        // "where level=error" — token 1 "level=error" → span starts at 6 (len("where ")==6)
        let (q, _) = parse(&tok(&["where", "level=error"])).unwrap();
        assert_eq!(q.filters[0].span.0, 6); // "where " is 6 bytes
        assert_eq!(q.filters[0].span.1, 6 + "level=error".len());
    }
}

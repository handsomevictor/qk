use crate::util::error::{QkError, Result};

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
}

#[derive(Debug)]
pub enum Aggregation {
    Count,
    CountBy(String),
    /// Discover all field names present across records.
    Fields,
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
pub fn parse(tokens: &[String]) -> Result<(FastQuery, Vec<String>)> {
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
                i = parse_count(tokens, i, &mut q);
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
            // Trailing comma on a token acts as `and` — continue parsing next predicate
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
        QkError::Query("expected filter expression after 'where'".to_string())
    })?;
    // Strip trailing comma so `level=error,` is treated the same as `level=error`
    let tok = raw_tok.trim_end_matches(',');

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
            return Ok((FilterExpr { field, op, value }, 1));
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
                    "gt"  => FilterOp::Gt,
                    "lt"  => FilterOp::Lt,
                    "gte" => FilterOp::Gte,
                    "lte" => FilterOp::Lte,
                    "eq"  => FilterOp::Eq,
                    "ne"  => FilterOp::Ne,
                    _     => unreachable!(),
                };
                // Strip trailing comma from value too
                let value = tokens.get(i + 2)
                    .map(|v| v.trim_end_matches(',').to_string())
                    .unwrap_or_default();
                return Ok((FilterExpr { field: tok.to_string(), op, value }, 3));
            }
            "contains" => {
                let value = tokens.get(i + 2)
                    .map(|v| v.trim_end_matches(',').to_string())
                    .unwrap_or_default();
                return Ok((
                    FilterExpr { field: tok.to_string(), op: FilterOp::Contains, value },
                    3,
                ));
            }
            "exists" => {
                return Ok((
                    FilterExpr { field: tok.to_string(), op: FilterOp::Exists, value: String::new() },
                    2,
                ));
            }
            _ => {}
        }
    }

    Err(QkError::Query(format!(
        "cannot parse filter '{raw_tok}': expected FIELD=VALUE, FIELD!=VALUE, FIELD>VALUE, \
         FIELD contains TEXT, or FIELD exists\n  \
         hint: shell metacharacters must be quoted — use 'latency>100' or word operators: \
         latency gt 100  /  latency lt 50  /  latency gte 200  /  latency lte 500"
    )))
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

/// Parse `count [by FIELD]`.
fn parse_count(tokens: &[String], mut i: usize, q: &mut FastQuery) -> usize {
    if tokens.get(i).map(|s| s.to_ascii_lowercase()) == Some("by".to_string()) {
        i += 1;
        if let Some(field) = tokens.get(i) {
            if !looks_like_file(field) && !is_query_keyword(field) {
                q.aggregation = Some(Aggregation::CountBy(field.clone()));
                return i + 1;
            }
        }
        q.aggregation = Some(Aggregation::Count);
    } else {
        q.aggregation = Some(Aggregation::Count);
    }
    i
}

/// Parse `sort FIELD [asc|desc]`.
fn parse_sort(tokens: &[String], mut i: usize, q: &mut FastQuery) -> Result<usize> {
    let field = tokens.get(i).ok_or_else(|| {
        QkError::Query("expected field name after 'sort'".to_string())
    })?;

    if looks_like_file(field) || is_query_keyword(field) {
        return Err(QkError::Query("expected field name after 'sort'".to_string()));
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
            return Err(QkError::Query(format!(
                "unknown sort direction '{other}': expected 'asc' or 'desc'"
            )));
        }
        _ => SortOrder::Asc,
    };

    q.sort = Some(SortExpr { field: field.clone(), order });
    Ok(i)
}

/// Parse `limit N`.
fn parse_limit(tokens: &[String], mut i: usize, q: &mut FastQuery) -> Result<usize> {
    let n_str = tokens.get(i).ok_or_else(|| {
        QkError::Query("expected number after 'limit'".to_string())
    })?;
    let n: usize = n_str.parse().map_err(|_| {
        QkError::Query(format!("'limit' expects a positive integer, got '{n_str}'"))
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
        QkError::Query("expected field name after stat keyword".to_string())
    })?;
    if looks_like_file(field) || is_query_keyword(field) {
        return Err(QkError::Query("expected field name after stat keyword".to_string()));
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
    )
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
        let (q, _) = parse(&tok(&["where", "level=error", "select", "ts", "msg", "app.log"])).unwrap();
        assert_eq!(q.projection, Some(vec!["ts".to_string(), "msg".to_string()]));
    }

    #[test]
    fn parses_count_by() {
        let (q, _) = parse(&tok(&["count", "by", "service", "app.log"])).unwrap();
        assert!(matches!(q.aggregation, Some(Aggregation::CountBy(ref f)) if f == "service"));
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
        let (q, _) = parse(&tok(&["where", "level=error", "and", "service=api", "app.log"])).unwrap();
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
}

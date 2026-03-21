use std::sync::Arc;

use crate::query::fast::parser::SortOrder;

/// A dot-separated field path: `.response.status` → `["response", "status"]`.
pub type FieldPath = Vec<String>;

/// A complete DSL query: a filter expression optionally chained with transforms.
///
/// Example: `.level == "error" | pick(.ts, .msg)`
#[derive(Debug)]
pub struct DslQuery {
    /// Boolean expression used to filter records (defaults to `Expr::True`).
    pub filter: Expr,
    /// Ordered pipeline of transforms applied after filtering.
    pub transforms: Vec<Stage>,
}

/// A boolean expression evaluated against a single `Record`.
#[derive(Debug)]
pub enum Expr {
    /// Always true; used when no filter is specified.
    True,
    /// `.field.path OP literal`
    Compare {
        path: FieldPath,
        op: CmpOp,
        value: Literal,
    },
    /// `.field exists` — the field is present in the record.
    Exists(FieldPath),
    /// Logical AND.
    And(Box<Expr>, Box<Expr>),
    /// Logical OR.
    Or(Box<Expr>, Box<Expr>),
    /// Logical NOT.
    Not(Box<Expr>),
}

/// Comparison operators for DSL filter expressions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Gt,
    Lt,
    Gte,
    Lte,
    /// Substring match: `.msg contains "error"`.
    Contains,
    /// Regex match: `.msg matches "err.*"`.
    Matches,
}

/// A literal value used on the right-hand side of a comparison.
#[derive(Debug, Clone)]
pub enum Literal {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
    /// Pre-compiled regex for `CmpOp::Matches`.
    /// Compiled once at parse time — never recompiled per record.
    Regex(Arc<regex::Regex>),
}

/// A pipeline transform stage applied after filtering.
#[derive(Debug)]
pub enum Stage {
    /// Keep only the listed fields.
    Pick(Vec<FieldPath>),
    /// Drop the listed fields.
    Omit(Vec<FieldPath>),
    /// Count matching records (produces a single `{"count": N}` record).
    Count,
    /// Sort records by the given field.
    SortBy(FieldPath, SortOrder),
    /// Group records and count occurrences per group value.
    GroupBy(FieldPath),
    /// Take the first N records.
    Limit(usize),
    /// Skip the first N records (offset / pagination).
    Skip(usize),
    /// Remove duplicate records by a field value.
    Dedup(FieldPath),
    /// Sum a numeric field across all records → `{"sum": N}`.
    Sum(FieldPath),
    /// Average a numeric field across all records → `{"avg": N}`.
    Avg(FieldPath),
    /// Minimum value of a numeric field → `{"min": N}`.
    Min(FieldPath),
    /// Maximum value of a numeric field → `{"max": N}`.
    Max(FieldPath),
    /// Group records into time buckets → `{"bucket": "2024-...", "count": N}` per bucket.
    ///
    /// `path` is the timestamp field; `bucket` is a duration string like `"5m"`.
    GroupByTime { path: FieldPath, bucket: String },
    /// Add an `hour_of_day` field (0–23 UTC) extracted from a timestamp field.
    HourOfDay(FieldPath),
    /// Add a `day_of_week` field (1=Mon … 7=Sun, ISO 8601) extracted from a timestamp field.
    DayOfWeek(FieldPath),
    /// Add an `is_weekend` bool field (true if Sat or Sun UTC) extracted from a timestamp field.
    IsWeekend(FieldPath),
}

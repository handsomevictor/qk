use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while, take_while1},
    character::complete::{char, multispace0, multispace1},
    combinator::{map, opt, value},
    multi::separated_list1,
    number::complete::double,
    sequence::{delimited, preceded, tuple},
};

use crate::query::fast::parser::SortOrder;
use crate::util::error::{QkError, Result};

use super::ast::{CmpOp, DslQuery, Expr, FieldPath, Literal, Stage};

// ── Public entry point ────────────────────────────────────────────────────────

/// Parse a DSL expression string into a `DslQuery`.
///
/// Format: `FILTER_EXPR ("|" STAGE)*`
pub fn parse(input: &str) -> Result<(DslQuery, Vec<String>)> {
    let trimmed = input.trim();
    match parse_query(trimmed) {
        Ok(("", q)) | Ok((" ", q)) => Ok((q, vec![])),
        Ok((rest, q)) => {
            let rest = rest.trim();
            if rest.is_empty() {
                return Ok((q, vec![]));
            }
            // Anything left over is file paths
            let files: Vec<String> = rest.split_whitespace().map(String::from).collect();
            Ok((q, files))
        }
        Err(e) => Err(QkError::Query(format!("DSL parse error: {e}"))),
    }
}

// ── Top-level ─────────────────────────────────────────────────────────────────

fn parse_query(i: &str) -> IResult<&str, DslQuery> {
    // Allow a query that starts directly with `|` (no filter → pass all records)
    if i.trim_start().starts_with('|') {
        let (i, transforms) = parse_pipe_chain(i)?;
        return Ok((i, DslQuery { filter: Expr::True, transforms }));
    }
    let (i, filter) = parse_or(i)?;
    let (i, transforms) = parse_pipe_chain(i)?;
    Ok((i, DslQuery { filter, transforms }))
}

/// Parse zero or more `| stage` segments.
fn parse_pipe_chain(i: &str) -> IResult<&str, Vec<Stage>> {
    let mut stages = Vec::new();
    let mut remaining = i;
    loop {
        let trimmed = remaining.trim_start();
        if !trimmed.starts_with('|') {
            break;
        }
        let after_pipe = trimmed[1..].trim_start();
        match parse_stage(after_pipe) {
            Ok((rest, stage)) => {
                stages.push(stage);
                remaining = rest;
            }
            Err(_) => break,
        }
    }
    Ok((remaining, stages))
}

// ── Boolean expression layers ─────────────────────────────────────────────────

fn parse_or(i: &str) -> IResult<&str, Expr> {
    let (i, first) = parse_and(i)?;
    let (i, rest) = nom::multi::many0(preceded(
        tuple((multispace1, tag_no_case("or"), multispace1)),
        parse_and,
    ))(i)?;
    Ok((i, rest.into_iter().fold(first, |acc, e| Expr::Or(Box::new(acc), Box::new(e)))))
}

fn parse_and(i: &str) -> IResult<&str, Expr> {
    let (i, first) = parse_not(i)?;
    let (i, rest) = nom::multi::many0(preceded(
        tuple((multispace1, tag_no_case("and"), multispace1)),
        parse_not,
    ))(i)?;
    Ok((i, rest.into_iter().fold(first, |acc, e| Expr::And(Box::new(acc), Box::new(e)))))
}

fn parse_not(i: &str) -> IResult<&str, Expr> {
    alt((
        map(preceded(tuple((tag_no_case("not"), multispace1)), parse_not), |e| {
            Expr::Not(Box::new(e))
        }),
        parse_comparison,
    ))(i)
}

// ── Comparison ────────────────────────────────────────────────────────────────

fn parse_comparison(i: &str) -> IResult<&str, Expr> {
    let (i, path) = parse_field_path(i)?;
    let (i, _) = multispace0(i)?;

    // `.field exists`
    if let Ok((rest, _)) = tag_no_case::<&str, &str, nom::error::Error<&str>>("exists")(i) {
        return Ok((rest, Expr::Exists(path)));
    }

    // `.field OP value`  or  `.field contains "text"`
    let (i, op) = parse_cmp_op(i)?;
    let (i, _) = multispace0(i)?;
    let (i, lit) = parse_literal(i)?;
    Ok((i, Expr::Compare { path, op, value: lit }))
}

fn parse_cmp_op(i: &str) -> IResult<&str, CmpOp> {
    alt((
        value(CmpOp::Gte, tag(">=")),
        value(CmpOp::Lte, tag("<=")),
        value(CmpOp::Ne, tag("!=")),
        value(CmpOp::Eq, tag("==")),
        value(CmpOp::Gt, tag(">")),
        value(CmpOp::Lt, tag("<")),
        value(CmpOp::Contains, tag_no_case("contains")),
        value(CmpOp::Matches, tag_no_case("matches")),
    ))(i)
}

// ── Field path ────────────────────────────────────────────────────────────────

fn parse_field_path(i: &str) -> IResult<&str, FieldPath> {
    let (i, _) = char('.')(i)?;
    let (i, first) = parse_ident(i)?;
    let (i, rest) = nom::multi::many0(preceded(char('.'), parse_ident))(i)?;
    let mut path = vec![first.to_string()];
    path.extend(rest.into_iter().map(String::from));
    Ok((i, path))
}

fn parse_ident(i: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '-')(i)
}

// ── Literals ──────────────────────────────────────────────────────────────────

fn parse_literal(i: &str) -> IResult<&str, Literal> {
    alt((
        map(parse_string, Literal::Str),
        map(double, Literal::Num),
        value(Literal::Bool(true), tag_no_case("true")),
        value(Literal::Bool(false), tag_no_case("false")),
        value(Literal::Null, tag_no_case("null")),
    ))(i)
}

fn parse_string(i: &str) -> IResult<&str, String> {
    let inner = take_while(|c: char| c != '"');
    map(delimited(char('"'), inner, char('"')), str::to_string)(i)
}

// ── Transform stages ──────────────────────────────────────────────────────────

fn parse_stage(i: &str) -> IResult<&str, Stage> {
    alt((
        parse_pick,
        parse_omit,
        parse_count,
        parse_sort_by,
        parse_group_by,
        parse_limit_stage,
        parse_skip_stage,
        parse_dedup,
        parse_sum,
        parse_avg,
        parse_min_stage,
        parse_max_stage,
    ))(i)
}

fn parse_pick(i: &str) -> IResult<&str, Stage> {
    let (i, _) = tag_no_case("pick")(i)?;
    let (i, paths) = parse_field_list(i)?;
    Ok((i, Stage::Pick(paths)))
}

fn parse_omit(i: &str) -> IResult<&str, Stage> {
    let (i, _) = tag_no_case("omit")(i)?;
    let (i, paths) = parse_field_list(i)?;
    Ok((i, Stage::Omit(paths)))
}

fn parse_count(i: &str) -> IResult<&str, Stage> {
    let (i, _) = tag_no_case("count")(i)?;
    let (i, _) = delimited(multispace0, tag("()"), multispace0)(i)?;
    Ok((i, Stage::Count))
}

fn parse_sort_by(i: &str) -> IResult<&str, Stage> {
    let (i, _) = tag_no_case("sort_by")(i)?;
    let (i, _) = delimited(multispace0, char('('), multispace0)(i)?;
    let (i, path) = parse_field_path(i)?;
    let (i, order) = opt(preceded(
        multispace1,
        alt((
            value(SortOrder::Desc, tag_no_case("desc")),
            value(SortOrder::Asc, tag_no_case("asc")),
        )),
    ))(i)?;
    let (i, _) = delimited(multispace0, char(')'), multispace0)(i)?;
    Ok((i, Stage::SortBy(path, order.unwrap_or(SortOrder::Asc))))
}

fn parse_group_by(i: &str) -> IResult<&str, Stage> {
    let (i, _) = tag_no_case("group_by")(i)?;
    let (i, _) = delimited(multispace0, char('('), multispace0)(i)?;
    let (i, path) = parse_field_path(i)?;
    let (i, _) = delimited(multispace0, char(')'), multispace0)(i)?;
    Ok((i, Stage::GroupBy(path)))
}

fn parse_limit_stage(i: &str) -> IResult<&str, Stage> {
    let (i, _) = tag_no_case("limit")(i)?;
    let (i, _) = delimited(multispace0, char('('), multispace0)(i)?;
    let (i, n) = map(
        take_while1(|c: char| c.is_ascii_digit()),
        |s: &str| s.parse::<usize>().unwrap_or(0),
    )(i)?;
    let (i, _) = delimited(multispace0, char(')'), multispace0)(i)?;
    Ok((i, Stage::Limit(n)))
}

fn parse_skip_stage(i: &str) -> IResult<&str, Stage> {
    let (i, _) = tag_no_case("skip")(i)?;
    let (i, _) = delimited(multispace0, char('('), multispace0)(i)?;
    let (i, n) = map(
        take_while1(|c: char| c.is_ascii_digit()),
        |s: &str| s.parse::<usize>().unwrap_or(0),
    )(i)?;
    let (i, _) = delimited(multispace0, char(')'), multispace0)(i)?;
    Ok((i, Stage::Skip(n)))
}

fn parse_dedup(i: &str) -> IResult<&str, Stage> {
    let (i, _) = tag_no_case("dedup")(i)?;
    let (i, _) = delimited(multispace0, char('('), multispace0)(i)?;
    let (i, path) = parse_field_path(i)?;
    let (i, _) = delimited(multispace0, char(')'), multispace0)(i)?;
    Ok((i, Stage::Dedup(path)))
}

fn parse_sum(i: &str) -> IResult<&str, Stage> {
    let (i, _) = tag_no_case("sum")(i)?;
    let (i, _) = delimited(multispace0, char('('), multispace0)(i)?;
    let (i, path) = parse_field_path(i)?;
    let (i, _) = delimited(multispace0, char(')'), multispace0)(i)?;
    Ok((i, Stage::Sum(path)))
}

fn parse_avg(i: &str) -> IResult<&str, Stage> {
    let (i, _) = tag_no_case("avg")(i)?;
    let (i, _) = delimited(multispace0, char('('), multispace0)(i)?;
    let (i, path) = parse_field_path(i)?;
    let (i, _) = delimited(multispace0, char(')'), multispace0)(i)?;
    Ok((i, Stage::Avg(path)))
}

fn parse_min_stage(i: &str) -> IResult<&str, Stage> {
    let (i, _) = tag_no_case("min")(i)?;
    let (i, _) = delimited(multispace0, char('('), multispace0)(i)?;
    let (i, path) = parse_field_path(i)?;
    let (i, _) = delimited(multispace0, char(')'), multispace0)(i)?;
    Ok((i, Stage::Min(path)))
}

fn parse_max_stage(i: &str) -> IResult<&str, Stage> {
    let (i, _) = tag_no_case("max")(i)?;
    let (i, _) = delimited(multispace0, char('('), multispace0)(i)?;
    let (i, path) = parse_field_path(i)?;
    let (i, _) = delimited(multispace0, char(')'), multispace0)(i)?;
    Ok((i, Stage::Max(path)))
}

fn parse_field_list(i: &str) -> IResult<&str, Vec<FieldPath>> {
    delimited(
        tuple((multispace0, char('('), multispace0)),
        separated_list1(tuple((multispace0, char(','), multispace0)), parse_field_path),
        tuple((multispace0, char(')'), multispace0)),
    )(i)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn q(s: &str) -> DslQuery {
        parse(s).unwrap().0
    }

    #[test]
    fn parses_eq() {
        let dq = q(r#".level == "error""#);
        assert!(matches!(dq.filter, Expr::Compare { op: CmpOp::Eq, .. }));
    }

    #[test]
    fn parses_gt_numeric() {
        let dq = q(".latency > 1000");
        if let Expr::Compare { op, value: Literal::Num(n), .. } = dq.filter {
            assert_eq!(op, CmpOp::Gt);
            assert!((n - 1000.0).abs() < f64::EPSILON);
        } else {
            panic!("expected Compare");
        }
    }

    #[test]
    fn parses_exists() {
        let dq = q(".error exists");
        assert!(matches!(dq.filter, Expr::Exists(_)));
    }

    #[test]
    fn parses_contains() {
        let dq = q(r#".msg contains "time""#);
        assert!(matches!(dq.filter, Expr::Compare { op: CmpOp::Contains, .. }));
    }

    #[test]
    fn parses_and() {
        let dq = q(r#".level == "error" and .service == "api""#);
        assert!(matches!(dq.filter, Expr::And(..)));
    }

    #[test]
    fn parses_or() {
        let dq = q(r#".level == "error" or .level == "warn""#);
        assert!(matches!(dq.filter, Expr::Or(..)));
    }

    #[test]
    fn parses_not() {
        let dq = q(r#"not .level == "info""#);
        assert!(matches!(dq.filter, Expr::Not(_)));
    }

    #[test]
    fn parses_pipe_pick() {
        let dq = q(r#".level == "error" | pick(.ts, .msg)"#);
        assert_eq!(dq.transforms.len(), 1);
        assert!(matches!(dq.transforms[0], Stage::Pick(_)));
    }

    #[test]
    fn parses_pipe_count() {
        let dq = q(".level == \"error\" | count()");
        assert!(matches!(dq.transforms[0], Stage::Count));
    }

    #[test]
    fn parses_sort_by_desc() {
        let dq = q(".latency > 0 | sort_by(.latency desc)");
        if let Stage::SortBy(_, order) = &dq.transforms[0] {
            assert_eq!(*order, SortOrder::Desc);
        } else {
            panic!("expected SortBy");
        }
    }

    #[test]
    fn parses_group_by() {
        let dq = q(".level == \"error\" | group_by(.service)");
        assert!(matches!(dq.transforms[0], Stage::GroupBy(_)));
    }

    #[test]
    fn parses_chained_stages() {
        let dq = q(".latency > 100 | sort_by(.latency desc) | limit(5)");
        assert_eq!(dq.transforms.len(), 2);
    }

    #[test]
    fn parses_skip() {
        let dq = q(".n > 0 | skip(5)");
        assert!(matches!(dq.transforms[0], Stage::Skip(5)));
    }

    #[test]
    fn parses_dedup() {
        let dq = q(".n > 0 | dedup(.service)");
        assert!(matches!(dq.transforms[0], Stage::Dedup(_)));
    }

    #[test]
    fn parses_sum() {
        let dq = q(".n > 0 | sum(.latency)");
        assert!(matches!(dq.transforms[0], Stage::Sum(_)));
    }

    #[test]
    fn parses_avg() {
        let dq = q(".n > 0 | avg(.latency)");
        assert!(matches!(dq.transforms[0], Stage::Avg(_)));
    }

    #[test]
    fn parses_min() {
        let dq = q(".n > 0 | min(.latency)");
        assert!(matches!(dq.transforms[0], Stage::Min(_)));
    }

    #[test]
    fn parses_max() {
        let dq = q(".n > 0 | max(.latency)");
        assert!(matches!(dq.transforms[0], Stage::Max(_)));
    }

    #[test]
    fn error_on_invalid_expr() {
        assert!(parse("not_a_field == 1").is_err());
    }

    #[test]
    fn nested_field_path() {
        let dq = q(".response.status == 503");
        if let Expr::Compare { path, .. } = &dq.filter {
            assert_eq!(path, &vec!["response".to_string(), "status".to_string()]);
        } else {
            panic!("expected Compare");
        }
    }
}

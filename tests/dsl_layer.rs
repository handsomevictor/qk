use assert_cmd::Command;

fn qk() -> Command {
    Command::cargo_bin("qk").unwrap()
}

fn ndjson_input(records: &[&str]) -> String {
    records.join("\n") + "\n"
}

/// Run a DSL query against raw NDJSON input and return trimmed stdout.
fn run_dsl(query: &str, input: &str) -> String {
    let out = qk().arg(query).write_stdin(input).output().unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Run a fast-layer keyword query against raw NDJSON input and return trimmed stdout.
fn run_fast(query: &str, input: &str) -> String {
    let args: Vec<&str> = query.split_whitespace().collect();
    let out = qk().args(&args).write_stdin(input).output().unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

const SAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/sample.ndjson");

// ── Filter expressions ────────────────────────────────────────────────────────

#[test]
fn dsl_eq_filter() {
    let input = ndjson_input(&[
        r#"{"level":"error","msg":"oops"}"#,
        r#"{"level":"info","msg":"ok"}"#,
    ]);
    let out = qk()
        .arg(r#".level == "error""#)
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
    let v: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(v["level"], "error");
}

#[test]
fn dsl_ne_filter() {
    let input = ndjson_input(&[
        r#"{"level":"error"}"#,
        r#"{"level":"info"}"#,
        r#"{"level":"warn"}"#,
    ]);
    let out = qk()
        .arg(r#".level != "error""#)
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
}

#[test]
fn dsl_numeric_gt_filter() {
    let input = ndjson_input(&[
        r#"{"status":500}"#,
        r#"{"status":200}"#,
        r#"{"status":404}"#,
    ]);
    let out = qk()
        .arg(".status > 400")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
}

#[test]
fn dsl_numeric_lte_filter() {
    let input = ndjson_input(&[r#"{"n":1}"#, r#"{"n":2}"#, r#"{"n":3}"#]);
    let out = qk().arg(".n <= 2").write_stdin(input).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
}

#[test]
fn dsl_and_filter() {
    let input = ndjson_input(&[
        r#"{"level":"error","service":"api"}"#,
        r#"{"level":"error","service":"web"}"#,
        r#"{"level":"info","service":"api"}"#,
    ]);
    let out = qk()
        .arg(r#".level == "error" and .service == "api""#)
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
}

#[test]
fn dsl_or_filter() {
    let input = ndjson_input(&[
        r#"{"level":"error"}"#,
        r#"{"level":"warn"}"#,
        r#"{"level":"info"}"#,
    ]);
    let out = qk()
        .arg(r#".level == "error" or .level == "warn""#)
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
}

#[test]
fn dsl_not_filter() {
    let input = ndjson_input(&[r#"{"level":"error"}"#, r#"{"level":"info"}"#]);
    let out = qk()
        .arg(r#"not .level == "info""#)
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
    let v: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(v["level"], "error");
}

#[test]
fn dsl_exists_filter() {
    let input = ndjson_input(&[r#"{"error":"oops","msg":"bad"}"#, r#"{"msg":"ok"}"#]);
    let out = qk()
        .arg(".error exists")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
}

#[test]
fn dsl_contains_filter() {
    let input = ndjson_input(&[r#"{"msg":"request timeout"}"#, r#"{"msg":"ok"}"#]);
    let out = qk()
        .arg(r#".msg contains "timeout""#)
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
}

#[test]
fn dsl_matches_regex() {
    let input = ndjson_input(&[r#"{"msg":"timeout error"}"#, r#"{"msg":"ok"}"#]);
    let out = qk()
        .arg(r#".msg matches "time.*""#)
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
}

#[test]
fn dsl_nested_field_filter() {
    let input = ndjson_input(&[
        r#"{"response":{"status":503}}"#,
        r#"{"response":{"status":200}}"#,
    ]);
    let out = qk()
        .arg(".response.status == 503")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
}

// ── Pipe stages ───────────────────────────────────────────────────────────────

#[test]
fn dsl_pick_stage() {
    let input = ndjson_input(&[r#"{"level":"error","msg":"oops","ts":"2024"}"#]);
    let out = qk()
        .arg(r#".level == "error" | pick(.level, .msg)"#)
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert!(v.get("level").is_some());
    assert!(v.get("msg").is_some());
    assert!(v.get("ts").is_none());
}

#[test]
fn dsl_omit_stage() {
    let input = ndjson_input(&[r#"{"level":"error","msg":"oops","ts":"2024"}"#]);
    let out = qk()
        .arg(r#".level == "error" | omit(.ts)"#)
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert!(v.get("ts").is_none());
    assert!(v.get("level").is_some());
    assert!(v.get("msg").is_some());
}

#[test]
fn dsl_count_stage() {
    let input = ndjson_input(&[
        r#"{"level":"error"}"#,
        r#"{"level":"error"}"#,
        r#"{"level":"info"}"#,
    ]);
    let out = qk()
        .arg(r#".level == "error" | count()"#)
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["count"], 2);
}

#[test]
fn dsl_count_all_no_filter() {
    let input = ndjson_input(&[r#"{"a":1}"#, r#"{"a":2}"#, r#"{"a":3}"#]);
    let out = qk().arg("| count()").write_stdin(input).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["count"], 3);
}

#[test]
fn dsl_sort_by_desc() {
    let input = ndjson_input(&[r#"{"n":1}"#, r#"{"n":3}"#, r#"{"n":2}"#]);
    let out = qk()
        .arg(".n > 0 | sort_by(.n desc)")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["n"], 3);
}

#[test]
fn dsl_sort_by_asc() {
    let input = ndjson_input(&[r#"{"n":3}"#, r#"{"n":1}"#, r#"{"n":2}"#]);
    let out = qk()
        .arg(".n > 0 | sort_by(.n asc)")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["n"], 1);
}

#[test]
fn dsl_group_by_stage() {
    let input = ndjson_input(&[
        r#"{"level":"error","service":"api"}"#,
        r#"{"level":"error","service":"api"}"#,
        r#"{"level":"error","service":"web"}"#,
    ]);
    let out = qk()
        .arg(r#".level == "error" | group_by(.service)"#)
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["service"], "api");
    assert_eq!(first["count"], 2);
}

#[test]
fn dsl_limit_stage() {
    let input = ndjson_input(&[r#"{"n":1}"#, r#"{"n":2}"#, r#"{"n":3}"#, r#"{"n":4}"#]);
    let out = qk()
        .arg(".n > 0 | limit(2)")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
}

// ── File input ────────────────────────────────────────────────────────────────

#[test]
fn dsl_filter_ndjson_file() {
    let out = qk()
        .arg(r#".level == "error""#)
        .arg(SAMPLE)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
    for line in &lines {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(v["level"], "error");
    }
}

#[test]
fn dsl_count_ndjson_file() {
    let out = qk()
        .args([r#".level == "error" | count()"#, SAMPLE])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["count"], 2);
}

// ── Output formats ────────────────────────────────────────────────────────────

#[test]
fn dsl_table_output() {
    let input = ndjson_input(&[r#"{"level":"error","msg":"oops"}"#]);
    // --fmt must come before the DSL expression (trailing_var_arg captures everything after)
    let out = qk()
        .args(["--fmt", "table", r#".level == "error""#])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("level"));
    assert!(stdout.contains("error"));
}

#[test]
fn dsl_csv_output() {
    let input = ndjson_input(&[r#"{"level":"error","msg":"oops"}"#]);
    // --fmt must come before the DSL expression (trailing_var_arg captures everything after)
    let out = qk()
        .args(["--fmt", "csv", r#".level == "error""#])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    // First line is the header
    assert!(lines[0].contains("level"));
    // Second line is the data
    assert!(lines[1].contains("error"));
}

#[test]
fn dsl_explain_flag() {
    // --explain must come before the DSL expression
    let out = qk()
        .args(["--explain", r#".level == "error""#, SAMPLE])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("DSL"));
}

// ── New stages ────────────────────────────────────────────────────────────────

#[test]
fn dsl_skip_stage() {
    let input = ndjson_input(&[r#"{"n":1}"#, r#"{"n":2}"#, r#"{"n":3}"#]);
    let out = qk()
        .arg(".n > 0 | skip(1)")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["n"], 2);
}

#[test]
fn dsl_dedup_stage() {
    let input = ndjson_input(&[
        r#"{"svc":"api","n":1}"#,
        r#"{"svc":"api","n":2}"#,
        r#"{"svc":"web","n":3}"#,
    ]);
    let out = qk()
        .arg(".n > 0 | dedup(.svc)")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
}

#[test]
fn dsl_sum_stage() {
    let input = ndjson_input(&[r#"{"n":1}"#, r#"{"n":2}"#, r#"{"n":3}"#]);
    let out = qk()
        .arg(".n > 0 | sum(.n)")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["sum"].as_f64().unwrap(), 6.0);
}

#[test]
fn dsl_avg_stage() {
    let input = ndjson_input(&[r#"{"n":10}"#, r#"{"n":20}"#, r#"{"n":30}"#]);
    let out = qk()
        .arg(".n > 0 | avg(.n)")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["avg"].as_f64().unwrap(), 20.0);
}

#[test]
fn dsl_min_max_stage() {
    let input = ndjson_input(&[r#"{"n":5}"#, r#"{"n":2}"#, r#"{"n":8}"#]);
    let min_out = qk()
        .arg(".n > 0 | min(.n)")
        .write_stdin(input.clone())
        .output()
        .unwrap();
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8(min_out.stdout).unwrap().trim()).unwrap();
    assert_eq!(v["min"].as_f64().unwrap(), 2.0);

    let max_out = qk()
        .arg(".n > 0 | max(.n)")
        .write_stdin(input)
        .output()
        .unwrap();
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8(max_out.stdout).unwrap().trim()).unwrap();
    assert_eq!(v["max"].as_f64().unwrap(), 8.0);
}

#[test]
fn dsl_pretty_output() {
    let input = ndjson_input(&[r#"{"level":"error","msg":"oops"}"#]);
    let out = qk()
        .args(["--fmt", "pretty", r#".level == "error""#])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    // Pretty output must be indented and valid JSON
    assert!(stdout.contains('\n'));
    serde_json::from_str::<serde_json::Value>(stdout.trim()).unwrap();
}

// ── Edge cases & robustness ───────────────────────────────────────────────────

#[test]
fn dsl_deep_nested_filter_3_levels() {
    let input = ndjson_input(&[
        r#"{"a":{"b":{"c":42}}}"#,
        r#"{"a":{"b":{"c":99}}}"#,
        r#"{"a":{"b":{"c":1}}}"#,
    ]);
    let out = qk().arg(".a.b.c > 10").write_stdin(input).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
    let v: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(v["a"]["b"]["c"], 42);
}

#[test]
fn dsl_deep_nested_filter_4_levels() {
    let input = ndjson_input(&[
        r#"{"a":{"b":{"c":{"d":"found"}}}}"#,
        r#"{"a":{"b":{"c":{"d":"nope"}}}}"#,
    ]);
    let out = qk()
        .arg(r#".a.b.c.d == "found""#)
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
}

#[test]
fn dsl_malformed_unclosed_paren_returns_error() {
    let input = ndjson_input(&[r#"{"a":1}"#]);
    // '(.a > 0' — no closing paren; should fail, not panic
    let out = qk().arg("(.a > 0").write_stdin(input).output().unwrap();
    // qk should exit non-zero with an error message
    assert!(
        !out.status.success(),
        "expected failure for malformed DSL, got success"
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        !stderr.is_empty(),
        "expected error message on stderr for malformed DSL"
    );
}

#[test]
fn dsl_malformed_missing_rhs_returns_error() {
    let input = ndjson_input(&[r#"{"a":1}"#]);
    // '.a ==' — missing right-hand side
    let out = qk().arg(".a ==").write_stdin(input).output().unwrap();
    assert!(
        !out.status.success(),
        "expected failure for .a == with no RHS"
    );
}

#[test]
fn dsl_empty_stdin_returns_empty_output() {
    let out = qk()
        .arg(r#".level == "error""#)
        .write_stdin("")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.trim().is_empty(),
        "expected empty output for empty stdin"
    );
}

#[test]
fn dsl_all_whitespace_stdin_returns_empty_output() {
    let out = qk()
        .arg(".x > 0")
        .write_stdin("   \n\n  \n")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.trim().is_empty());
}

#[test]
fn dsl_long_field_name_and_value() {
    let long_field = "a".repeat(200);
    let long_value = "x".repeat(500);
    let record = format!(r#"{{"{long_field}":"{long_value}"}}"#);
    let query = format!(r#".{long_field} == "{long_value}""#);
    let out = qk()
        .arg(&query)
        .write_stdin(format!("{record}\n"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
}

#[test]
fn dsl_field_missing_in_some_records() {
    // Records where the filtered field is absent should be excluded, not panic
    let input = ndjson_input(&[
        r#"{"level":"error","code":500}"#,
        r#"{"level":"info"}"#, // no "code" field
        r#"{"level":"warn","code":404}"#,
    ]);
    let out = qk().arg(".code > 400").write_stdin(input).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    // Only records with code field present and > 400
    assert_eq!(lines.len(), 2);
}

#[test]
fn dsl_and_or_precedence() {
    // 'A or B and C' should be parsed as 'A or (B and C)'
    let input = ndjson_input(&[
        r#"{"a":1,"b":0,"c":0}"#, // a=1 → matches 'a==1 or ...'
        r#"{"a":0,"b":1,"c":1}"#, // b=1 and c=1 → matches '... or (b==1 and c==1)'
        r#"{"a":0,"b":1,"c":0}"#, // b=1 but c=0 → should NOT match
    ]);
    let out = qk()
        .arg(".a == 1 or .b == 1 and .c == 1")
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        2,
        "expected 2 matches for a==1 or (b==1 and c==1)"
    );
}

#[test]
fn dsl_not_expression() {
    let input = ndjson_input(&[
        r#"{"level":"error"}"#,
        r#"{"level":"info"}"#,
        r#"{"level":"warn"}"#,
    ]);
    let out = qk()
        .arg(r#"not .level == "error""#)
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
}

#[test]
fn dsl_pipeline_no_filter_pass_all() {
    let input = ndjson_input(&[r#"{"n":1}"#, r#"{"n":2}"#, r#"{"n":3}"#]);
    let out = qk().arg("| limit(2)").write_stdin(input).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
}

#[test]
fn dsl_corrupt_lines_mid_stream_skipped() {
    // Three valid records with a corrupt line between them.
    // The corrupt line should be skipped with a warning; valid records still appear.
    let input = concat!(
        "{\"level\":\"error\"}\n",
        "this-is-not-json\n",
        "{\"level\":\"info\"}\n",
        "{\"level\":\"warn\"}\n",
    );
    let out = qk()
        .arg(".level exists")
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success(), "qk should not abort on corrupt line");
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 3, "expected 3 valid records");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("[qk warning]"),
        "expected warning for corrupt line"
    );
}

// ── Time attribute extraction ──────────────────────────────────────────────────

#[test]
fn dsl_hour_of_day_adds_field() {
    // 2024-01-15T14:30:00Z → hour_of_day = 14
    let input = ndjson_input(&[
        r#"{"ts":"2024-01-15T14:30:00Z","level":"info"}"#,
        r#"{"ts":"2024-01-15T08:05:00Z","level":"warn"}"#,
    ]);
    let out = qk()
        .arg(".level exists | hour_of_day(.ts)")
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(first["hour_of_day"], 14);
    assert_eq!(second["hour_of_day"], 8);
}

#[test]
fn dsl_day_of_week_adds_field() {
    // 2024-01-15 is a Monday → day_of_week = 1
    let input = ndjson_input(&[r#"{"ts":"2024-01-15T10:00:00Z"}"#]);
    let out = qk()
        .arg("| day_of_week(.ts)")
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["day_of_week"], 1, "2024-01-15 is Monday = 1");
}

#[test]
fn dsl_is_weekend_saturday() {
    // 2024-01-20 is a Saturday → is_weekend = true
    let input = ndjson_input(&[r#"{"ts":"2024-01-20T10:00:00Z"}"#]);
    let out = qk()
        .arg("| is_weekend(.ts)")
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["is_weekend"], true, "2024-01-20 is Saturday");
}

#[test]
fn dsl_is_weekend_monday() {
    // 2024-01-15 is a Monday → is_weekend = false
    let input = ndjson_input(&[r#"{"ts":"2024-01-15T10:00:00Z"}"#]);
    let out = qk()
        .arg("| is_weekend(.ts)")
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["is_weekend"], false, "2024-01-15 is Monday");
}

#[test]
fn dsl_hour_of_day_then_group_by() {
    // Combine hour_of_day with group_by to get per-hour counts
    let input = ndjson_input(&[
        r#"{"ts":"2024-01-15T10:00:00Z"}"#,
        r#"{"ts":"2024-01-15T10:30:00Z"}"#,
        r#"{"ts":"2024-01-15T14:00:00Z"}"#,
    ]);
    let out = qk()
        .arg("| hour_of_day(.ts) | group_by(.hour_of_day)")
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2, "expected 2 distinct hours");
}

#[test]
fn dsl_calendar_group_by_time_day() {
    // DSL layer: '| group_by_time(.ts, "day")'
    let input = ndjson_input(&[
        r#"{"ts":"2024-01-15T08:00:00Z"}"#,
        r#"{"ts":"2024-01-15T20:00:00Z"}"#,
        r#"{"ts":"2024-01-16T10:00:00Z"}"#,
    ]);
    let out = qk()
        .arg(r#"| group_by_time(.ts, "day")"#)
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["bucket"], "2024-01-15");
    assert_eq!(first["count"], 2);
}

// ── count_unique tests ────────────────────────────────────────────────────────

#[test]
fn dsl_count_unique_basic() {
    let input = r#"{"level":"error","svc":"api"}
{"level":"warn","svc":"api"}
{"level":"error","svc":"db"}
{"level":"info","svc":"web"}
"#;
    let out = run_dsl("| count_unique(.level)", input);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["count_unique"], 3); // error, warn, info
}

#[test]
fn dsl_count_unique_single_value() {
    let input = r#"{"k":"a"}
{"k":"a"}
{"k":"a"}
"#;
    let out = run_dsl("| count_unique(.k)", input);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["count_unique"], 1);
}

// ── DSL arithmetic (map) tests ────────────────────────────────────────────────

#[test]
fn dsl_map_divide_field_by_constant() {
    // latency is 2000 ms, map to seconds: 2000 / 1000 = 2
    let input = r#"{"latency":2000,"svc":"api"}
"#;
    let out = run_dsl("| map(.latency_s = .latency / 1000.0)", input);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!((v["latency_s"].as_f64().unwrap() - 2.0).abs() < 1e-9);
}

#[test]
fn dsl_map_add_fields() {
    // a=3, b=7, sum=10
    let input = r#"{"a":3,"b":7}
"#;
    let out = run_dsl("| map(.total = .a + .b)", input);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["total"].as_f64().unwrap(), 10.0);
}

#[test]
fn dsl_map_multiply() {
    let input = r#"{"bytes":1024}
"#;
    let out = run_dsl("| map(.kb = .bytes / 1024.0)", input);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!((v["kb"].as_f64().unwrap() - 1.0).abs() < 1e-9);
}

#[test]
fn dsl_map_missing_field_skips_silently() {
    // field .x does not exist — record should still appear, just without .result
    let input = r#"{"a":5}
"#;
    let out = run_dsl("| map(.result = .x / 2.0)", input);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["result"].is_null() || v.get("result").is_none());
    assert_eq!(v["a"], 5);
}

#[test]
fn dsl_map_divide_by_zero_skips() {
    let input = r#"{"v":10}
"#;
    let out = run_dsl("| map(.r = .v / 0.0)", input);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    // result field should be absent (division by zero returns None → no insert)
    assert!(v.get("r").is_none());
    assert_eq!(v["v"], 10);
}

#[test]
fn dsl_map_complex_expression() {
    // (a + b) * 2 = (3 + 7) * 2 = 20
    let input = r#"{"a":3,"b":7}
"#;
    let out = run_dsl("| map(.result = (.a + .b) * 2.0)", input);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["result"].as_f64().unwrap(), 20.0);
}

// ── Multi-field group_by ──────────────────────────────────────────────────────

#[test]
fn dsl_group_by_two_fields() {
    let input = concat!(
        "{\"level\":\"error\",\"svc\":\"api\"}\n",
        "{\"level\":\"error\",\"svc\":\"api\"}\n",
        "{\"level\":\"error\",\"svc\":\"db\"}\n",
        "{\"level\":\"warn\",\"svc\":\"api\"}\n",
    );
    let out = run_dsl("| group_by(.level, .svc)", input);
    let lines: Vec<serde_json::Value> = out
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 3);
}

// ── String functions ──────────────────────────────────────────────────────────

#[test]
fn dsl_to_lower() {
    let input = "{\"msg\":\"Hello World\"}\n";
    let out = run_dsl("| to_lower(.msg)", input);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["msg"], "hello world");
}

#[test]
fn dsl_to_upper() {
    let input = "{\"msg\":\"hello\"}\n";
    let out = run_dsl("| to_upper(.msg)", input);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["msg"], "HELLO");
}

#[test]
fn dsl_replace() {
    let input = "{\"msg\":\"foo bar foo\"}\n";
    let out = run_dsl(r#"| replace(.msg, "foo", "baz")"#, input);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["msg"], "baz bar baz");
}

#[test]
fn dsl_split() {
    let input = "{\"tags\":\"a,b,c\"}\n";
    let out = run_dsl(r#"| split(.tags, ",")"#, input);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["tags"], serde_json::json!(["a", "b", "c"]));
}

#[test]
fn dsl_map_length_string() {
    let input = "{\"msg\":\"hello\"}\n";
    let out = run_dsl("| map(.n = length(.msg))", input);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["n"], 5);
}

#[test]
fn dsl_map_length_array() {
    let input = "{\"tags\":[\"a\",\"b\",\"c\"]}\n";
    let out = run_dsl("| map(.n = length(.tags))", input);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["n"], 3);
}

#[test]
fn dsl_array_contains() {
    let input = concat!(
        "{\"tags\":[\"prod\",\"web\"]}\n",
        "{\"tags\":[\"staging\",\"api\"]}\n",
    );
    let out = run_dsl(".tags contains \"prod\"", input);
    let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
}

// ── DSL parse error — caret visualization ────────────────────────────────────

/// An incomplete DSL expression produces an error with a `^` caret in stderr.
#[test]
fn dsl_parse_error_shows_caret_pointer() {
    let out = qk().args([".level =="]).write_stdin("").output().unwrap();
    assert!(!out.status.success(), "invalid DSL should exit non-zero");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains('^'),
        "error output should contain a caret (^) pointing to the failure position; got: {stderr}"
    );
}

/// DSL error includes "failed at position N" so users can locate the problem.
/// Uses an expression with a missing RHS after `==` which triggers a nom error.
#[test]
fn dsl_parse_error_includes_position_offset() {
    // ".level ==" has no value after ==  — nom fails trying to parse the RHS.
    let out = qk().args([".level =="]).write_stdin("").output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("position"),
        "error should mention byte position; got: {stderr}"
    );
    // The failure occurs after ".level ==" (offset > 0), not at position 0.
    assert!(
        !stderr.contains("position 0"),
        "failure position should be after the valid prefix; got: {stderr}"
    );
}

/// DSL error echoes the original input expression so users can see context.
#[test]
fn dsl_parse_error_echoes_input() {
    // ".latency ==" — no RHS value; nom reports a parse error at this position.
    let out = qk().args([".latency =="]).write_stdin("").output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    // The error should echo back the field name so the user can locate the problem.
    assert!(
        stderr.contains(".latency"),
        "error should echo the input expression; got: {stderr}"
    );
}

/// A completely invalid DSL expression still produces a readable error, not a panic.
#[test]
fn dsl_parse_error_on_garbage_input() {
    let out = qk().args([".$$$invalid"]).write_stdin("").output().unwrap();
    assert!(
        !out.status.success(),
        "garbage DSL expression should fail cleanly"
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        !stderr.is_empty(),
        "error output should be non-empty for invalid expression"
    );
}

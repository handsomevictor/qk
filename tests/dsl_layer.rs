use assert_cmd::Command;

fn qk() -> Command {
    Command::cargo_bin("qk").unwrap()
}

fn ndjson_input(records: &[&str]) -> String {
    records.join("\n") + "\n"
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

use assert_cmd::Command;

fn qk() -> Command {
    Command::cargo_bin("qk").unwrap()
}

const SAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/sample.ndjson");

#[test]
fn pipe_stdin_ndjson() {
    let input = "{\"level\":\"error\"}\n{\"level\":\"info\"}\n";
    let out = qk()
        .args(["where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
}

#[test]
fn count_total_from_stdin() {
    let input = "{\"a\":1}\n{\"a\":2}\n{\"a\":3}\n";
    let out = qk().arg("count").write_stdin(input).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["count"], 3);
}

#[test]
fn chained_pipes_filter_then_count() {
    let input = concat!(
        "{\"level\":\"error\",\"service\":\"api\"}\n",
        "{\"level\":\"error\",\"service\":\"web\"}\n",
        "{\"level\":\"info\",\"service\":\"api\"}\n",
    );
    // Simulate: qk where level=error | qk count
    let step1 = qk()
        .args(["where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    let step2 = qk()
        .arg("count")
        .write_stdin(step1.stdout)
        .output()
        .unwrap();
    let stdout = String::from_utf8(step2.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["count"], 2);
}

#[test]
fn explain_flag_exits_cleanly() {
    qk().args(["--explain", "where", "level=error"])
        .arg(SAMPLE)
        .assert()
        .success();
}

#[test]
fn no_args_empty_stdin_exits_cleanly() {
    qk().write_stdin("").assert().success();
}

#[test]
fn invalid_limit_exits_with_error() {
    qk().args(["limit", "abc"])
        .write_stdin("{\"a\":1}\n")
        .assert()
        .failure();
}

#[test]
fn gt_numeric_filter() {
    let input = "{\"status\":500}\n{\"status\":200}\n{\"status\":404}\n";
    let out = qk()
        .args(["where", "status>400"])
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
}

// ── New aggregations ──────────────────────────────────────────────────────────

#[test]
fn fields_discovery() {
    let input = "{\"level\":\"error\",\"msg\":\"a\"}\n{\"level\":\"info\",\"ts\":\"x\"}\n";
    let out = qk().arg("fields").write_stdin(input).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let names: Vec<String> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let v: serde_json::Value = serde_json::from_str(l).unwrap();
            v["field"].as_str().unwrap().to_string()
        })
        .collect();
    assert!(names.contains(&"level".to_string()));
    assert!(names.contains(&"msg".to_string()));
    assert!(names.contains(&"ts".to_string()));
}

#[test]
fn sum_field_keyword() {
    let input = "{\"n\":1}\n{\"n\":2}\n{\"n\":3}\n";
    let out = qk().args(["sum", "n"]).write_stdin(input).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["sum"].as_f64().unwrap(), 6.0);
}

#[test]
fn avg_field_keyword() {
    let input = "{\"n\":1}\n{\"n\":2}\n{\"n\":3}\n";
    let out = qk().args(["avg", "n"]).write_stdin(input).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["avg"].as_f64().unwrap(), 2.0);
}

#[test]
fn min_max_field_keyword() {
    let input = "{\"n\":5}\n{\"n\":2}\n{\"n\":8}\n";
    let min_out = qk().args(["min", "n"]).write_stdin(input).output().unwrap();
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8(min_out.stdout).unwrap().trim()).unwrap();
    assert_eq!(v["min"].as_f64().unwrap(), 2.0);

    let max_out = qk()
        .args(["max", "n"])
        .write_stdin("{\"n\":5}\n{\"n\":2}\n{\"n\":8}\n")
        .output()
        .unwrap();
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8(max_out.stdout).unwrap().trim()).unwrap();
    assert_eq!(v["max"].as_f64().unwrap(), 8.0);
}

#[test]
fn head_is_alias_for_limit() {
    let input = "{\"a\":1}\n{\"a\":2}\n{\"a\":3}\n";
    let out = qk()
        .args(["head", "2"])
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
}

#[test]
fn pretty_format_is_indented_json() {
    let input = "{\"level\":\"error\",\"msg\":\"oops\"}\n";
    let out = qk()
        .args(["--fmt", "pretty", "where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains('\n'), "pretty output should be multi-line");
    serde_json::from_str::<serde_json::Value>(stdout.trim()).unwrap();
}

// ── Color flags ────────────────────────────────────────────────────────────────

#[test]
fn no_color_flag_output_is_valid_json() {
    let input = "{\"level\":\"error\",\"msg\":\"oops\"}\n{\"level\":\"info\",\"msg\":\"ok\"}\n";
    let out = qk()
        .args(["--no-color", "where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    // Output must be plain, parseable JSON (no ANSI codes)
    for line in stdout.lines().filter(|l| !l.is_empty()) {
        assert!(
            serde_json::from_str::<serde_json::Value>(line).is_ok(),
            "line is not valid JSON: {line}"
        );
    }
}

#[test]
fn color_flag_produces_ansi_codes() {
    let input = "{\"level\":\"error\",\"msg\":\"oops\"}\n";
    let out = qk()
        .args(["--color", "where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    // --color forces ANSI codes even when stdout is piped (not a terminal)
    assert!(
        stdout.contains('\x1b'),
        "expected ANSI escape codes with --color flag"
    );
}

#[test]
fn color_flag_error_level_contains_red() {
    let input = "{\"level\":\"error\",\"msg\":\"boom\"}\n";
    let out = qk()
        .args(["--color", "where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    // ANSI red = ESC[31m or as part of bold+red ESC[1;31m / ESC[31m
    assert!(
        stdout.contains("31"),
        "expected red ANSI code for error level"
    );
}

#[test]
fn no_color_flag_takes_priority_over_color_flag() {
    // --no-color wins even if --color is also present
    let input = "{\"level\":\"error\"}\n";
    let out = qk()
        .args(["--no-color", "--color", "where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    // No ANSI codes — --no-color takes priority
    assert!(
        !stdout.contains('\x1b'),
        "expected no ANSI codes when --no-color is set"
    );
}

/// Regression test for LL-008 + T-01: regex with `.*` wildcards must match correctly.
/// The previous implementation used `str::contains()` instead of real regex matching,
/// so `.*timeout.*` would search for the literal string `.*timeout.*`.
/// The fix pre-compiles the regex once at parse time (zero per-record cost).
/// Streaming stdin: filter-only queries now process records line-by-line (no EOF block).
/// This test verifies the streaming path produces correct results.
#[test]
fn streaming_filter_produces_correct_results() {
    let input = concat!(
        "{\"level\":\"error\",\"msg\":\"a\"}\n",
        "{\"level\":\"info\",\"msg\":\"b\"}\n",
        "{\"level\":\"error\",\"msg\":\"c\"}\n",
    );
    let out = qk()
        .args(["where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("\"a\""));
    assert!(lines[1].contains("\"c\""));
}

#[test]
fn streaming_limit_stops_early() {
    let input = concat!(
        "{\"level\":\"error\",\"n\":1}\n",
        "{\"level\":\"error\",\"n\":2}\n",
        "{\"level\":\"error\",\"n\":3}\n",
    );
    let out = qk()
        .args(["where", "level=error", "limit", "2"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2, "limit 2 should stop after 2 matches");
}

#[test]
fn streaming_select_projection() {
    let input = "{\"level\":\"error\",\"msg\":\"oops\",\"ts\":\"2024\"}\n";
    let out = qk()
        .args(["where", "level=error", "select", "level", "msg"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert!(v.get("level").is_some());
    assert!(v.get("msg").is_some());
    assert!(v.get("ts").is_none(), "ts should be projected out");
}

#[test]
fn regex_dotstar_pattern_matches_correctly() {
    let input = concat!(
        "{\"msg\":\"connection timeout occurred\"}\n",
        "{\"msg\":\"request timed out\"}\n",
        "{\"msg\":\"started\"}\n",
    );
    let out = qk()
        .args(["where", "msg~=.*timeout.*"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    // Only the first record matches `.*timeout.*`
    assert_eq!(lines.len(), 1, "expected exactly 1 match for .*timeout.*");
    assert!(lines[0].contains("timeout"));
}

#[test]
fn regex_case_sensitive_pattern() {
    let input = concat!(
        "{\"level\":\"ERROR\"}\n",
        "{\"level\":\"error\"}\n",
        "{\"level\":\"info\"}\n",
    );
    // Regex is case-sensitive by default (unlike glob which adds (?i))
    let out = qk()
        .args(["where", "level~=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("\"error\""));
}

#[test]
fn raw_output_format_returns_original_line() {
    let input = "{\"level\":\"error\",\"msg\":\"oops\"}\n{\"level\":\"info\"}\n";
    let out = qk()
        .args(["--fmt", "raw", "where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    // Should return the original matched line exactly
    assert!(stdout.contains("{\"level\":\"error\",\"msg\":\"oops\"}"));
    // info record should not appear
    assert!(!stdout.contains("\"info\""));
}

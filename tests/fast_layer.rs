use assert_cmd::Command;

fn qk() -> Command {
    Command::cargo_bin("qk").unwrap()
}

/// Run a fast-layer keyword query against raw NDJSON input and return trimmed stdout.
fn run_fast(query: &str, input: &str) -> String {
    let args: Vec<&str> = query.split_whitespace().collect();
    let out = qk().args(&args).write_stdin(input).output().unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

const SAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/sample.ndjson");
const TIMESERIES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/timeseries.ndjson"
);

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

// ── Time-bucket integration tests ────────────────────────────────────────────

#[test]
fn count_by_5m_produces_correct_buckets() {
    // timeseries.ndjson: 12 records spanning 10:01–11:12 UTC
    // 5-minute windows: 10:00, 10:05, 10:10, 11:00, 11:05, 11:10 → 6 buckets
    let out = qk()
        .args(["count", "by", "5m", TIMESERIES])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        6,
        "expected 6 five-minute buckets, got:\n{stdout}"
    );
    // Default order is descending (newest bucket first).
    // Last bucket label = 11:10 window (the latest records in timeseries.ndjson)
    assert!(
        lines[0].contains("\"2024-01-15T11:10:00Z\""),
        "first bucket (should be newest): {}",
        lines[0]
    );
    // All output lines must contain both "bucket" and "count" keys
    for line in &lines {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        assert!(v.get("bucket").is_some(), "missing 'bucket' key: {line}");
        assert!(v.get("count").is_some(), "missing 'count' key: {line}");
    }
}

#[test]
fn count_by_1h_produces_two_buckets() {
    // timeseries.ndjson: 8 records in 10:xx, 4 records in 11:xx → 2 hourly buckets
    let out = qk()
        .args(["count", "by", "1h", TIMESERIES])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<serde_json::Value> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 2);
    // Descending: newest bucket first
    assert_eq!(lines[0]["bucket"].as_str().unwrap(), "2024-01-15T11:00:00Z");
    assert_eq!(lines[0]["count"].as_u64().unwrap(), 4);
    assert_eq!(lines[1]["bucket"].as_str().unwrap(), "2024-01-15T10:00:00Z");
    assert_eq!(lines[1]["count"].as_u64().unwrap(), 8);
}

#[test]
fn filter_then_count_by_time() {
    // Only error-level records (3 in the fixture) should be bucketed
    let out = qk()
        .args(["where", "level=error,", "count", "by", "1h", TIMESERIES])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<serde_json::Value> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    // 2 error records in 10:xx hour, 1 error record in 11:xx hour.
    // Descending: newest bucket (11:xx, count=1) first.
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["count"].as_u64().unwrap(), 1);
    assert_eq!(lines[1]["count"].as_u64().unwrap(), 2);
}

#[test]
fn rfc3339_string_gt_comparison() {
    // After the compare_values fix, lexicographic comparison on RFC 3339 strings works.
    // Records after 10:05:00Z in timeseries.ndjson: 10:06, 10:08, 10:11, 10:13, 10:14, 11:02, 11:05, 11:09, 11:12 → 9
    let out = qk()
        .args(["where", "ts>2024-01-15T10:05:00Z", TIMESERIES])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        9,
        "expected 9 records after 10:05, got:\n{stdout}"
    );
}

#[test]
fn rfc3339_string_lt_comparison() {
    // Records strictly before 10:05:00Z: 10:01, 10:02, 10:04 → 3 records
    let out = qk()
        .args(["where", "ts<2024-01-15T10:05:00Z", TIMESERIES])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        3,
        "expected 3 records before 10:05, got:\n{stdout}"
    );
}

#[test]
fn rfc3339_string_eq_comparison() {
    // Exact match on a timestamp string
    let out = qk()
        .args(["where", "ts=2024-01-15T10:06:00Z", TIMESERIES])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
}

// ── Streaming resilience ──────────────────────────────────────────────────────

#[test]
fn streaming_corrupt_lines_skipped_with_warning() {
    // Valid NDJSON interleaved with corrupt lines on stdin.
    // Streaming mode (no aggregation, stdin) should skip bad lines and continue.
    let input = concat!(
        "{\"level\":\"error\",\"service\":\"api\"}\n",
        "this is not json\n",
        "{\"level\":\"info\",\"service\":\"web\"}\n",
        "also-bad\n",
        "{\"level\":\"error\",\"service\":\"db\"}\n",
    );
    let out = qk()
        .args(["where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success(), "qk must not abort on corrupt lines");
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2, "expected 2 error records, got: {stdout}");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("[qk warning]"),
        "expected warnings for corrupt lines, stderr: {stderr}"
    );
}

#[test]
fn streaming_empty_stdin_returns_empty() {
    let out = qk()
        .args(["where", "level=error"])
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
fn streaming_all_corrupt_stdin_returns_empty_with_warnings() {
    let input = "not-json\nalso-not-json\n{bad\n";
    let out = qk()
        .args(["where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "should succeed even if all lines are corrupt"
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.trim().is_empty(), "no records should pass filter");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("[qk warning]"),
        "expected warnings for corrupt lines"
    );
}

#[test]
fn streaming_only_blank_lines_stdin() {
    let input = "\n\n\n   \n\n";
    let out = qk()
        .args(["where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.trim().is_empty());
    // blank lines produce no warnings
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        !stderr.contains("[qk warning]"),
        "blank lines should not produce warnings"
    );
}

#[test]
fn streaming_count_on_empty_stdin_returns_zero() {
    let out = qk().arg("count").write_stdin("").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["count"], 0);
}

// ── Calendar-aligned time bucketing ──────────────────────────────────────────

#[test]
fn count_by_day_produces_calendar_days() {
    // 3 records on 2024-01-15, 2 on 2024-01-16
    let input = concat!(
        "{\"ts\":\"2024-01-15T08:00:00Z\",\"level\":\"info\"}\n",
        "{\"ts\":\"2024-01-15T14:30:00Z\",\"level\":\"info\"}\n",
        "{\"ts\":\"2024-01-15T23:59:00Z\",\"level\":\"info\"}\n",
        "{\"ts\":\"2024-01-16T00:01:00Z\",\"level\":\"info\"}\n",
        "{\"ts\":\"2024-01-16T12:00:00Z\",\"level\":\"info\"}\n",
    );
    let out = qk()
        .args(["count", "by", "day", "ts"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2, "expected 2 calendar days, got:\n{stdout}");
    // Descending: newest day first
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(first["bucket"], "2024-01-16");
    assert_eq!(first["count"], 2);
    assert_eq!(second["bucket"], "2024-01-15");
    assert_eq!(second["count"], 3);
}

#[test]
fn count_by_month_groups_by_calendar_month() {
    let input = concat!(
        "{\"ts\":\"2024-01-05T00:00:00Z\"}\n",
        "{\"ts\":\"2024-01-20T00:00:00Z\"}\n",
        "{\"ts\":\"2024-02-10T00:00:00Z\"}\n",
        "{\"ts\":\"2024-03-01T00:00:00Z\"}\n",
        "{\"ts\":\"2024-03-15T00:00:00Z\"}\n",
    );
    let out = qk()
        .args(["count", "by", "month", "ts"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 3, "expected 3 months");
    // Descending: newest month first
    let mar: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let feb: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    let jan: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
    assert_eq!(mar["bucket"], "2024-03");
    assert_eq!(mar["count"], 2);
    assert_eq!(feb["bucket"], "2024-02");
    assert_eq!(feb["count"], 1);
    assert_eq!(jan["bucket"], "2024-01");
    assert_eq!(jan["count"], 2);
}

#[test]
fn count_by_year_groups_by_calendar_year() {
    let input = concat!(
        "{\"ts\":\"2022-06-01T00:00:00Z\"}\n",
        "{\"ts\":\"2023-01-01T00:00:00Z\"}\n",
        "{\"ts\":\"2023-12-31T23:59:59Z\"}\n",
        "{\"ts\":\"2024-03-01T00:00:00Z\"}\n",
    );
    let out = qk()
        .args(["count", "by", "year", "ts"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 3);
    // Descending: newest year (2024) first
    let y2024: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(y2024["bucket"], "2024");
    assert_eq!(y2024["count"], 1);
}

#[test]
fn count_by_hour_groups_by_calendar_hour() {
    let input = concat!(
        "{\"ts\":\"2024-01-15T10:01:00Z\"}\n",
        "{\"ts\":\"2024-01-15T10:45:00Z\"}\n",
        "{\"ts\":\"2024-01-15T11:00:00Z\"}\n",
    );
    let out = qk()
        .args(["count", "by", "hour", "ts"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
    // Descending: newest hour (11:00) first
    let h11: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(h11["bucket"], "2024-01-15T11:00:00Z");
    assert_eq!(h11["count"], 1);
}

// ── count unique ──────────────────────────────────────────────────────────────

#[test]
fn count_unique_basic() {
    // 4 records, 3 unique levels
    let input = r#"{"level":"error","svc":"api"}
{"level":"warn","svc":"api"}
{"level":"error","svc":"db"}
{"level":"info","svc":"web"}
"#;
    let out = run_fast("count unique level", input);
    let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(v["count_unique"], 3);
}

#[test]
fn count_unique_all_same() {
    let input = r#"{"level":"error"}
{"level":"error"}
{"level":"error"}
"#;
    let out = run_fast("count unique level", input);
    let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(v["count_unique"], 1);
}

#[test]
fn count_unique_missing_field_counts_as_empty_string() {
    // two records have level, one doesn't — missing becomes "" so still 3 unique values
    let input = r#"{"level":"error"}
{"level":"warn"}
{"other":"x"}
"#;
    let out = run_fast("count unique level", input);
    let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    // "" (missing), "error", "warn" = 3 unique
    assert_eq!(v["count_unique"], 3);
}

// ── Multi-field count by ──────────────────────────────────────────────────────

#[test]
fn count_by_two_fields() {
    let input = concat!(
        "{\"level\":\"error\",\"svc\":\"api\"}\n",
        "{\"level\":\"error\",\"svc\":\"api\"}\n",
        "{\"level\":\"error\",\"svc\":\"db\"}\n",
        "{\"level\":\"warn\",\"svc\":\"api\"}\n",
    );
    let out = run_fast("count by level svc", input);
    let lines: Vec<serde_json::Value> = out
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 3);
    // First (most common): error + api, count 2
    assert_eq!(lines[0]["level"], "error");
    assert_eq!(lines[0]["svc"], "api");
    assert_eq!(lines[0]["count"], 2);
}

#[test]
fn count_by_two_fields_comma_syntax() {
    let input = concat!(
        "{\"level\":\"error\",\"svc\":\"api\"}\n",
        "{\"level\":\"error\",\"svc\":\"db\"}\n",
    );
    // comma-separated syntax
    let out = run_fast("count by level, svc", input);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2);
}

// ── contains (memmem SIMD path) ───────────────────────────────────────────────

/// `contains` returns all records whose field contains the substring.
/// This test covers the memmem code path (replaced naive str::contains).
#[test]
fn contains_ascii_substring() {
    let input = concat!(
        "{\"msg\":\"connection timeout error\"}\n",
        "{\"msg\":\"all good\"}\n",
        "{\"msg\":\"disk timeout warning\"}\n",
    );
    let out = run_fast("where msg contains timeout", input);
    let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        2,
        "should match 2 records containing 'timeout'"
    );
}

/// `contains` with multi-byte UTF-8 — memmem searches at the byte level.
#[test]
fn contains_multibyte_unicode() {
    let input = concat!(
        "{\"msg\":\"你好世界\"}\n",
        "{\"msg\":\"hello world\"}\n",
        "{\"msg\":\"世界很大\"}\n",
    );
    let out = run_fast("where msg contains 世界", input);
    let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        2,
        "should match both records containing '世界'"
    );
}

/// `contains` with the needle equal to the full field value — still matches.
#[test]
fn contains_exact_full_value() {
    let input = concat!("{\"env\":\"production\"}\n", "{\"env\":\"staging\"}\n",);
    let out = run_fast("where env contains production", input);
    let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
}

/// `contains` with no matching records returns empty output.
#[test]
fn contains_no_match_returns_empty() {
    let input = concat!(
        "{\"msg\":\"alpha\"}\n",
        "{\"msg\":\"beta\"}\n",
        "{\"msg\":\"gamma\"}\n",
    );
    let out = run_fast("where msg contains zzz", input);
    let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    assert!(lines.is_empty(), "no records should match needle 'zzz'");
}

// ── --stats flag ──────────────────────────────────────────────────────────────

/// --stats prints Stats: header, Records in/out, Time, and Output fmt to stderr.
#[test]
fn stats_flag_prints_summary_to_stderr() {
    let input = concat!(
        "{\"level\":\"error\"}\n",
        "{\"level\":\"info\"}\n",
        "{\"level\":\"error\"}\n",
    );
    let out = qk()
        .args(["--stats", "where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("Stats:"),
        "stderr should contain 'Stats:' header; got: {stderr}"
    );
    assert!(
        stderr.contains("Records in:"),
        "stderr should show input count; got: {stderr}"
    );
    assert!(
        stderr.contains("Records out:"),
        "stderr should show output count; got: {stderr}"
    );
    assert!(
        stderr.contains("Time:"),
        "stderr should show elapsed time; got: {stderr}"
    );
    assert!(
        stderr.contains("Output fmt:"),
        "stderr should show output format; got: {stderr}"
    );
    // Stdout must still contain the matched records.
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        2,
        "--stats must not suppress normal matched output"
    );
}

/// --stats records-in and records-out values match actual counts.
#[test]
fn stats_flag_record_counts_are_accurate() {
    let input = concat!(
        "{\"v\":1}\n",
        "{\"v\":2}\n",
        "{\"v\":3}\n",
        "{\"v\":4}\n",
        "{\"v\":5}\n",
    );
    // Each token must be a separate argument — the fast parser tokenizes on OS args, not spaces.
    let out = qk()
        .args(["--stats", "where", "v", "gt", "2"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    // 5 records in, 3 records out (v=3,4,5)
    assert!(
        stderr.contains("Records in:  5"),
        "expected 'Records in:  5'; got: {stderr}"
    );
    assert!(
        stderr.contains("Records out: 3"),
        "expected 'Records out: 3'; got: {stderr}"
    );
}

/// --stats shows the effective output format (ndjson by default).
#[test]
fn stats_flag_shows_output_format_name() {
    let input = "{\"a\":1}\n";
    let out = qk()
        .args(["--stats", "--fmt", "pretty", "count"])
        .write_stdin(input)
        .output()
        .unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("pretty"),
        "stats should show the format name 'pretty'; got: {stderr}"
    );
}

/// Without --stats, no Stats: block appears in stderr.
#[test]
fn no_stats_flag_produces_no_stats_output() {
    let input = "{\"level\":\"error\"}\n";
    let out = qk()
        .args(["where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        !stderr.contains("Stats:"),
        "Stats: block should not appear without --stats flag"
    );
}

// ── config file (default_fmt) ─────────────────────────────────────────────────

/// Config file default_fmt=pretty causes output to be pretty-printed when --fmt is absent.
#[test]
fn config_default_fmt_pretty_used_when_no_flag() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_dir = dir.path().join("qk");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(cfg_dir.join("config.toml"), "default_fmt = \"pretty\"\n").unwrap();

    let input = "{\"level\":\"error\",\"msg\":\"oops\"}\n";
    let out = qk()
        .env("XDG_CONFIG_HOME", dir.path())
        .args(["where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    // pretty output uses "key": "value" (space after colon) and is multi-line
    assert!(
        stdout.contains("\"level\": \"error\""),
        "pretty output should have space after colon; got: {stdout}"
    );
    assert!(
        stdout.contains('\n'),
        "pretty output should span multiple lines"
    );
}

/// --fmt flag takes priority over config file default_fmt.
#[test]
fn config_default_fmt_overridden_by_flag() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_dir = dir.path().join("qk");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(cfg_dir.join("config.toml"), "default_fmt = \"pretty\"\n").unwrap();

    let input = "{\"level\":\"error\"}\n";
    let out = qk()
        .env("XDG_CONFIG_HOME", dir.path())
        .args(["--fmt", "ndjson", "where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    // ndjson: no space after colon, single line per record
    assert!(
        !stdout.contains("\"level\": \"error\""),
        "ndjson should NOT have space after colon when --fmt ndjson overrides config"
    );
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1, "ndjson should be one line per record");
}

/// Missing config file is handled gracefully — falls back to ndjson default.
#[test]
fn config_missing_file_falls_back_to_ndjson() {
    let dir = tempfile::tempdir().unwrap();
    // Deliberately do NOT create any config file.
    let input = "{\"level\":\"error\"}\n";
    let out = qk()
        .env("XDG_CONFIG_HOME", dir.path())
        .args(["where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    // ndjson default: no space after colon
    assert!(
        !stdout.contains("\"level\": \"error\""),
        "default format should be ndjson without config file"
    );
}

/// Config file with an unrecognised default_fmt value falls back gracefully.
#[test]
fn config_unknown_fmt_value_falls_back_to_ndjson() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_dir = dir.path().join("qk");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(
        cfg_dir.join("config.toml"),
        "default_fmt = \"notaformat\"\n",
    )
    .unwrap();

    let input = "{\"level\":\"error\"}\n";
    let out = qk()
        .env("XDG_CONFIG_HOME", dir.path())
        .args(["where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    // Must not crash — unknown format silently falls back.
    assert!(
        out.status.success(),
        "unknown config fmt should not cause failure"
    );
}

// ── Progress spinner (TTY-gated, tested indirectly) ───────────────────────────

/// When reading from a file (not stdin), output is still correct with the
/// spinner code path active. The spinner itself is invisible in tests because
/// stderr is not a terminal in the test harness.
#[test]
fn progress_spinner_does_not_corrupt_output() {
    let out = qk()
        .args(["where", "level=error", SAMPLE])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    // Spinner must not bleed into stdout
    assert!(
        !stdout.contains('\r'),
        "spinner control characters must not appear in stdout"
    );
    assert!(
        stdout.lines().all(|l| l.is_empty() || l.starts_with('{')),
        "all non-empty stdout lines must be JSON objects"
    );
}

// ── count types ───────────────────────────────────────────────────────────────

#[test]
fn count_types_basic() {
    let input = concat!(
        "{\"v\":1}\n",
        "{\"v\":\"hello\"}\n",
        "{\"v\":null}\n",
        "{\"v\":true}\n",
        "{\"v\":2}\n",
    );
    let out = run_fast("count types v", input);
    let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    // Should have entries for: number (2), string (1), null (1), bool (1)
    assert_eq!(lines.len(), 4, "expected 4 type buckets");
    // First line should be number (highest count)
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["type"], "number");
    assert_eq!(first["count"], 2);
}

#[test]
fn count_types_with_missing_field() {
    let input = concat!("{\"v\":1}\n", "{\"other\":\"x\"}\n", "{\"v\":null}\n",);
    let out = run_fast("count types v", input);
    let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    // Types: number(1), null(1), missing(1)
    assert_eq!(lines.len(), 3);
    let types: Vec<String> = lines
        .iter()
        .map(|l| {
            serde_json::from_str::<serde_json::Value>(l).unwrap()["type"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert!(types.contains(&"missing".to_string()));
}

#[test]
fn count_types_sorted_by_count_desc() {
    let input = concat!(
        "{\"v\":1}\n",
        "{\"v\":2}\n",
        "{\"v\":3}\n",
        "{\"v\":\"s\"}\n",
    );
    let out = run_fast("count types v", input);
    let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["type"], "number", "most common type should be first");
    assert_eq!(first["count"], 3);
}

// ── --quiet flag ──────────────────────────────────────────────────────────────

#[test]
fn quiet_flag_suppresses_all_warnings() {
    // Use a corrupt stdin line that would normally produce a warning
    let input = concat!(
        "{\"level\":\"error\"}\n",
        "not-json-at-all\n",
        "{\"level\":\"info\"}\n",
    );
    let out = qk()
        .args(["--quiet", "where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.is_empty() || !stderr.contains("[qk warning]"),
        "--quiet should suppress all warnings; got: {stderr}"
    );
}

#[test]
fn quiet_flag_does_not_affect_stdout() {
    let input = concat!(
        "{\"level\":\"error\"}\n",
        "not-json\n",
        "{\"level\":\"error\"}\n",
    );
    let out = qk()
        .args(["--quiet", "where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        2,
        "--quiet should not affect matched output count"
    );
}

// ── auto-limit (TTY gated; tested via --all / explicit limit) ─────────────────

/// When an explicit `limit N` is in the query, auto-limit must not interfere.
#[test]
fn explicit_limit_not_overridden_by_auto_limit() {
    let input: String = (0..50).map(|i| format!("{{\"n\":{i}}}\n")).collect();
    let out = qk()
        .args(["limit", "10"])
        .write_stdin(input.as_str())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        10,
        "explicit limit 10 should return exactly 10"
    );
}

// ── gzip format variants ──────────────────────────────────────────────────────

use flate2::{write::GzEncoder, Compression};
use std::io::Write as _;

fn make_gz(content: &[u8]) -> Vec<u8> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(content).unwrap();
    enc.finish().unwrap()
}

/// CSV.gz: decompresses transparently, parses as CSV, returns correct records.
#[test]
fn csv_gz_parses_transparently() {
    let csv = b"name,age\nAlice,30\nBob,25\n";
    let gz = make_gz(csv);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("users.csv.gz");
    std::fs::write(&path, &gz).unwrap();

    let out = qk()
        .args(["count"])
        .arg(path.to_str().unwrap())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("\"count\":2"),
        "csv.gz should yield 2 records"
    );
}

/// TSV.gz: decompresses and parses as TSV.
#[test]
fn tsv_gz_parses_transparently() {
    let tsv = b"ts\tevent\n2024-01-01\tlogin\n2024-01-02\tlogout\n";
    let gz = make_gz(tsv);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.tsv.gz");
    std::fs::write(&path, &gz).unwrap();

    let out = qk()
        .args(["count"])
        .arg(path.to_str().unwrap())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("\"count\":2"),
        "tsv.gz should yield 2 records"
    );
}

/// JSON.gz: decompresses and parses as JSON array.
#[test]
fn json_gz_parses_transparently() {
    let json = b"[{\"id\":1,\"v\":\"a\"},{\"id\":2,\"v\":\"b\"}]";
    let gz = make_gz(json);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.json.gz");
    std::fs::write(&path, &gz).unwrap();

    let out = qk()
        .args(["count"])
        .arg(path.to_str().unwrap())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("\"count\":2"),
        "json.gz should yield 2 records"
    );
}

/// YAML.gz: decompresses and parses as YAML.
#[test]
fn yaml_gz_parses_transparently() {
    let yaml = b"---\nname: alice\nage: 30\n---\nname: bob\nage: 25\n";
    let gz = make_gz(yaml);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("people.yaml.gz");
    std::fs::write(&path, &gz).unwrap();

    let out = qk()
        .args(["count"])
        .arg(path.to_str().unwrap())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("\"count\":2"),
        "yaml.gz should yield 2 records"
    );
}

/// NDJSON.gz: the existing supported case still works.
#[test]
fn ndjson_gz_parses_transparently() {
    let ndjson = b"{\"level\":\"error\"}\n{\"level\":\"info\"}\n{\"level\":\"warn\"}\n";
    let gz = make_gz(ndjson);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log.gz");
    std::fs::write(&path, &gz).unwrap();

    let out = qk()
        .args(["where", "level=error", "count"])
        .arg(path.to_str().unwrap())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("\"count\":1"),
        "ndjson.gz where level=error should yield 1"
    );
}

/// gz file detected by magic bytes even without .gz extension.
#[test]
fn gz_detected_by_magic_bytes_without_gz_extension() {
    let ndjson = b"{\"x\":1}\n{\"x\":2}\n";
    let gz = make_gz(ndjson);

    let dir = tempfile::tempdir().unwrap();
    // No .gz extension — detection must fall back to magic bytes.
    let path = dir.path().join("compressed_log");
    std::fs::write(&path, &gz).unwrap();

    let out = qk()
        .args(["count"])
        .arg(path.to_str().unwrap())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("\"count\":2"),
        "magic-byte gzip detection should work"
    );
}

// ── config show / config reset ────────────────────────────────────────────────

/// `qk config show` exits successfully and prints the settings table.
#[test]
fn config_show_prints_table() {
    let out = qk().args(["config", "show"]).output().unwrap();
    assert!(out.status.success(), "config show should exit 0");
    let combined = String::from_utf8(out.stdout).unwrap() + &String::from_utf8(out.stderr).unwrap();
    assert!(
        combined.contains("default_fmt"),
        "table should show default_fmt row"
    );
    assert!(
        combined.contains("default_limit"),
        "table should show default_limit row"
    );
    assert!(
        combined.contains("no_color"),
        "table should show no_color row"
    );
}

/// `qk config reset` on a non-existent file reports already-at-defaults.
#[test]
fn config_reset_when_no_file_reports_already_default() {
    // Force XDG_CONFIG_HOME to a temp dir so we don't touch the real config.
    let dir = tempfile::tempdir().unwrap();
    let out = qk()
        .args(["config", "reset"])
        .env("XDG_CONFIG_HOME", dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("already at defaults") || stdout.contains("reset to"),
        "should report status: got {stdout}"
    );
}

/// `qk config reset` removes an existing config file and confirms removal.
#[test]
fn config_reset_removes_existing_config_file() {
    let dir = tempfile::tempdir().unwrap();
    // Create a fake config file.
    let cfg_dir = dir.path().join("qk");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(cfg_dir.join("config.toml"), "default_fmt = \"pretty\"\n").unwrap();

    let out = qk()
        .args(["config", "reset"])
        .env("XDG_CONFIG_HOME", dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("reset to built-in defaults"),
        "should confirm reset: got {stdout}"
    );
    assert!(
        !cfg_dir.join("config.toml").exists(),
        "config file should be removed"
    );
}

/// `qk config show` reflects values written to the config file.
#[test]
fn config_show_reflects_config_file_values() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_dir = dir.path().join("qk");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(
        cfg_dir.join("config.toml"),
        "default_fmt = \"table\"\ndefault_limit = 42\n",
    )
    .unwrap();

    let out = qk()
        .args(["config", "show"])
        .env("XDG_CONFIG_HOME", dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let combined = String::from_utf8(out.stdout).unwrap() + &String::from_utf8(out.stderr).unwrap();
    assert!(
        combined.contains("table"),
        "should show configured fmt=table"
    );
    assert!(combined.contains("42"), "should show configured limit=42");
}

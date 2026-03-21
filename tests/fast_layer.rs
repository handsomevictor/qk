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
    // First bucket label must be the floored start of the window, not the record time
    assert!(
        lines[0].contains("\"2024-01-15T10:00:00Z\""),
        "first bucket: {}",
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
    assert_eq!(lines[0]["bucket"].as_str().unwrap(), "2024-01-15T10:00:00Z");
    assert_eq!(lines[0]["count"].as_u64().unwrap(), 8);
    assert_eq!(lines[1]["bucket"].as_str().unwrap(), "2024-01-15T11:00:00Z");
    assert_eq!(lines[1]["count"].as_u64().unwrap(), 4);
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
    // 2 error records in 10:xx hour, 1 error record in 11:xx hour
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["count"].as_u64().unwrap(), 2);
    assert_eq!(lines[1]["count"].as_u64().unwrap(), 1);
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
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(first["bucket"], "2024-01-15");
    assert_eq!(first["count"], 3);
    assert_eq!(second["bucket"], "2024-01-16");
    assert_eq!(second["count"], 2);
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
    let jan: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let feb: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    let mar: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
    assert_eq!(jan["bucket"], "2024-01");
    assert_eq!(jan["count"], 2);
    assert_eq!(feb["bucket"], "2024-02");
    assert_eq!(feb["count"], 1);
    assert_eq!(mar["bucket"], "2024-03");
    assert_eq!(mar["count"], 2);
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
    let y2022: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(y2022["bucket"], "2022");
    assert_eq!(y2022["count"], 1);
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
    let h10: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(h10["bucket"], "2024-01-15T10:00:00Z");
    assert_eq!(h10["count"], 2);
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

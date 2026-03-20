use std::path::PathBuf;

use assert_cmd::Command;
use predicates::str::contains;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn qk() -> Command {
    Command::cargo_bin("qk").unwrap()
}

// ── NDJSON ────────────────────────────────────────────────────────────────────

#[test]
fn ndjson_outputs_all_records() {
    qk()
        .arg(fixture("sample.ndjson"))
        .assert()
        .success()
        .stdout(contains("\"level\""));
}

#[test]
fn ndjson_filter_error() {
    let out = qk()
        .args(["where", "level=error"])
        .arg(fixture("sample.ndjson"))
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
    for line in lines {
        assert!(line.contains("\"error\""), "expected error level in: {line}");
    }
}

#[test]
fn ndjson_count_by_service() {
    let out = qk()
        .args(["count", "by", "service"])
        .arg(fixture("sample.ndjson"))
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    // First line should be the most common service
    let first: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    assert_eq!(first["service"], "api");
    assert_eq!(first["count"], 3);
}

#[test]
fn ndjson_select_fields() {
    let out = qk()
        .args(["select", "level", "service"])
        .arg(fixture("sample.ndjson"))
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    for line in stdout.lines().filter(|l| !l.is_empty()) {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        assert!(v.get("level").is_some());
        assert!(v.get("service").is_some());
        assert!(v.get("ts").is_none(), "ts should not appear after select");
    }
}

#[test]
fn ndjson_sort_latency_desc() {
    let out = qk()
        .args(["sort", "latency", "desc", "limit", "1"])
        .arg(fixture("sample.ndjson"))
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let first: serde_json::Value =
        serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    assert_eq!(first["latency"], 3001);
}

// ── logfmt ────────────────────────────────────────────────────────────────────

#[test]
fn logfmt_filter_error() {
    let out = qk()
        .args(["where", "level=error"])
        .arg(fixture("sample.logfmt"))
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
}

#[test]
fn logfmt_filter_or() {
    let out = qk()
        .args(["where", "level=error", "or", "level=warn"])
        .arg(fixture("sample.logfmt"))
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 3);
}

// ── CSV ───────────────────────────────────────────────────────────────────────

#[test]
fn csv_all_records() {
    let out = qk()
        .arg(fixture("sample.csv"))
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 5);
}

#[test]
fn csv_filter_error() {
    let out = qk()
        .args(["where", "level=error"])
        .arg(fixture("sample.csv"))
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
}

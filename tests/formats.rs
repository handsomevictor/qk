use std::path::PathBuf;

use assert_cmd::Command;

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
    let out = qk().arg(fixture("sample.ndjson")).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 6);
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
    for line in &lines {
        assert!(line.contains("\"error\""));
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
    let first: serde_json::Value =
        serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
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
        assert!(v.get("ts").is_none());
    }
}

#[test]
fn ndjson_sort_latency_desc_limit_1() {
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
    let out = qk().arg(fixture("sample.csv")).output().unwrap();
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

// ── YAML ───────────────────────────────────────────────────────────────────────

#[test]
fn yaml_outputs_all_records() {
    let out = qk().arg(fixture("sample.yaml")).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 5);
}

#[test]
fn yaml_filter_error() {
    let out = qk()
        .args(["where", "level=error"])
        .arg(fixture("sample.yaml"))
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
fn yaml_dsl_filter() {
    let out = qk()
        .arg(r#".level == "error""#)
        .arg(fixture("sample.yaml"))
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
}

#[test]
fn yaml_dsl_sort_latency_desc() {
    let out = qk()
        .args([".latency >= 0 | sort_by(.latency desc) | limit(1)", &fixture("sample.yaml").to_string_lossy()])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["latency"], 3001);
}

// ── TOML ───────────────────────────────────────────────────────────────────────

#[test]
fn toml_loads_single_record() {
    let out = qk().arg(fixture("sample.toml")).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    // sample.toml is a single flat document → one record
    assert_eq!(lines.len(), 1);
    let v: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(v["level"], "error");
    assert_eq!(v["service"], "api");
}

#[test]
fn toml_dsl_filter_matches() {
    let out = qk()
        .arg(r#".level == "error""#)
        .arg(fixture("sample.toml"))
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
}

#[test]
fn toml_dsl_filter_no_match() {
    let out = qk()
        .arg(r#".level == "info""#)
        .arg(fixture("sample.toml"))
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 0);
}

#[test]
fn toml_numeric_field_accessible() {
    let out = qk()
        .arg(".latency > 1000")
        .arg(fixture("sample.toml"))
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
    let v: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(v["latency"], 3001);
}

// ── Output formats ────────────────────────────────────────────────────────────

#[test]
fn table_output_format() {
    // --fmt must precede query tokens (trailing_var_arg captures everything after first positional)
    let out = qk()
        .args(["--fmt", "table", "where", "level=error"])
        .arg(fixture("sample.ndjson"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("level"));
    assert!(stdout.contains("error"));
}

#[test]
fn csv_output_format() {
    // --fmt must precede query tokens (trailing_var_arg captures everything after first positional)
    let out = qk()
        .args(["--fmt", "csv", "where", "level=error"])
        .arg(fixture("sample.ndjson"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    // First line is header
    assert!(lines[0].contains("level"));
    // Data lines have "error"
    assert!(lines[1..].iter().any(|l| l.contains("error")));
}

// ── Gzip decompression ────────────────────────────────────────────────────────

#[test]
fn gz_decompression_and_query() {
    use std::io::Write;
    use flate2::write::GzEncoder;
    use flate2::Compression;

    // Build gzip bytes from sample ndjson
    let ndjson = r#"{"level":"error","msg":"gz test"}
{"level":"info","msg":"ok"}
"#;
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(ndjson.as_bytes()).unwrap();
    let gz_bytes = enc.finish().unwrap();

    // Write to a temp file with .ndjson.gz extension
    let tmp = tempfile::Builder::new()
        .suffix(".ndjson.gz")
        .tempfile()
        .unwrap();
    std::fs::write(tmp.path(), &gz_bytes).unwrap();

    let out = qk()
        .arg(r#".level == "error""#)
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
    let v: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(v["level"], "error");
}

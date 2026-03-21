//! Lightweight stability tests — run in regular `cargo test` (no `#[ignore]`).
//!
//! These tests generate moderate-size data in memory to verify:
//! 1. Streaming stdin path handles large input without incorrect results
//! 2. Corrupt-line resilience doesn't silently discard valid records
//! 3. Memory is not obviously leaked (process exits cleanly)

use assert_cmd::Command;

fn qk() -> Command {
    Command::cargo_bin("qk").unwrap()
}

/// Generate N records of NDJSON with alternating levels.
fn gen_ndjson(n: usize) -> String {
    let mut buf = String::with_capacity(n * 60);
    for i in 0..n {
        let level = if i % 3 == 0 {
            "error"
        } else if i % 3 == 1 {
            "warn"
        } else {
            "info"
        };
        buf.push_str(&format!(
            "{{\"level\":\"{level}\",\"i\":{i},\"svc\":\"api\"}}\n"
        ));
    }
    buf
}

/// Generate N records with ~10% corrupt lines interspersed.
fn gen_ndjson_with_corrupt(n: usize) -> String {
    let mut buf = String::with_capacity(n * 70);
    for i in 0..n {
        if i % 10 == 5 {
            buf.push_str("NOT_JSON_AT_ALL{corrupt\n");
        } else {
            buf.push_str(&format!("{{\"level\":\"info\",\"i\":{i}}}\n"));
        }
    }
    buf
}

#[test]
fn streaming_10k_records_filter_count() {
    // 10,000 records; 1/3 are level=error → should get ~3333 results
    let input = gen_ndjson(10_000);
    let out = qk()
        .args(["where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let lines: Vec<&str> = std::str::from_utf8(&out.stdout)
        .unwrap()
        .lines()
        .filter(|l| !l.is_empty())
        .collect();
    // Every 3rd record (0,3,6,...) is error → ceil(10000/3) = 3334
    assert_eq!(lines.len(), 3334);
}

#[test]
fn streaming_10k_records_count_aggregation() {
    let input = gen_ndjson(10_000);
    let out = qk().arg("count").write_stdin(input).output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&out.stdout).unwrap().trim()).unwrap();
    assert_eq!(v["count"], 10_000);
}

#[test]
fn corrupt_lines_do_not_abort_processing() {
    // 1000 records, ~100 corrupt → 900 valid
    let input = gen_ndjson_with_corrupt(1_000);
    let out = qk()
        .args(["where", "level=info"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let valid_count = std::str::from_utf8(&out.stdout)
        .unwrap()
        .lines()
        .filter(|l| !l.is_empty())
        .count();
    // 100 corrupt, 900 valid info records
    assert_eq!(valid_count, 900);
}

#[test]
fn corrupt_lines_emit_warnings_to_stderr() {
    let input = gen_ndjson_with_corrupt(100);
    let out = qk()
        .args(["where", "level=info"])
        .write_stdin(input)
        .output()
        .unwrap();
    let stderr = std::str::from_utf8(&out.stderr).unwrap();
    assert!(
        stderr.contains("[qk warning]"),
        "expected warnings on stderr for corrupt lines, got: {stderr:?}"
    );
}

#[test]
fn streaming_count_unique_10k() {
    // 10,000 records with level cycling through 3 values → count unique = 3
    let input = gen_ndjson(10_000);
    let out = qk()
        .args(["count", "unique", "level"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&out.stdout).unwrap().trim()).unwrap();
    assert_eq!(v["count_unique"], 3);
}

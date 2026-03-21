//! Large-file performance tests for qk.
//!
//! # Memory model
//!
//! ## Streaming path (stdin, filter-only, no aggregation)
//! qk detects when reading from stdin with a streaming-compatible query (no
//! aggregation, no sort) and processes records one at a time.  Memory usage is
//! O(output) — only matched records are buffered for rendering.  Suitable for
//! 2 GB+ inputs with <500 MB peak RSS.
//!
//! ## Batch path (file path argument, or any aggregation/sort query)
//! qk reads the entire input into memory before evaluating.  Each parsed record
//! occupies ~500–600 bytes on the heap (IndexMap overhead + field strings).
//! A 200 MB NDJSON file yields roughly 2.36 M records → ~1.2 GB heap.
//!
//! ## Known limitation
//! Batch mode on files >1 GB may OOM on machines with <16 GB RAM.  See T-04 in
//! ROADMAP.md for the planned fix (external-sort + streaming aggregation).
//!
//! # Running
//!
//! ```sh
//! cargo test --test large_file -- --ignored --nocapture
//! ```
//!
//! Each test is marked `#[ignore]` so it does not run in the normal `cargo test`
//! suite.  Pass `-- --ignored` to opt-in.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufWriter, Write};
use std::process::{Command, Stdio};
use std::time::Instant;

use assert_cmd::cargo::cargo_bin;
use tempfile::NamedTempFile;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Schema: `{"level":"...","service":"...","latency":N,"ts":EPOCH,"msg":"request completed"}`
///
/// - level:   error / warn / info / debug  (n % 4)
/// - service: api / web / db / cache / auth (n % 5)
/// - latency: 10 + (n % 990)  → range [10, 999]
/// - ts:      1_705_313_130 + n  (Unix epoch seconds)
///
/// Each line is approximately 90 bytes.
fn write_ndjson(path: &std::path::Path, target_bytes: u64) -> u64 {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .expect("open file for writing");
    let mut writer = BufWriter::with_capacity(1 << 20, file);

    let levels = ["error", "warn", "info", "debug"];
    let services = ["api", "web", "db", "cache", "auth"];

    let mut n: u64 = 0;
    let mut written: u64 = 0;
    while written < target_bytes {
        let level = levels[(n % 4) as usize];
        let service = services[(n % 5) as usize];
        let latency = 10 + (n % 990);
        let ts = 1_705_313_130u64 + n;
        let line = format!(
            "{{\"level\":\"{}\",\"service\":\"{}\",\"latency\":{},\"ts\":{},\"msg\":\"request completed\"}}\n",
            level, service, latency, ts
        );
        let bytes = line.as_bytes();
        writer.write_all(bytes).expect("write line");
        written += bytes.len() as u64;
        n += 1;
    }
    writer.flush().expect("flush");
    n
}

/// Append `count` corrupt lines to the file at `path`.
fn append_corrupt_lines(path: &std::path::Path, count: usize) {
    let file = OpenOptions::new()
        .append(true)
        .open(path)
        .expect("open file for appending");
    let mut writer = BufWriter::new(file);
    for i in 0..count {
        writeln!(writer, "{{this is not valid json #{}}}", i).expect("write corrupt line");
    }
    writer.flush().expect("flush");
}

/// Count lines from a `BufReader` without storing them all in memory.
fn count_lines_from_reader<R: std::io::Read>(reader: R) -> usize {
    std::io::BufReader::new(reader)
        .lines()
        .filter_map(|l| l.ok())
        .filter(|l| !l.is_empty())
        .count()
}

// ---------------------------------------------------------------------------
// Test 1 — Streaming filter on 2 GB stdin
// ---------------------------------------------------------------------------

/// Stream 2 GB of NDJSON through stdin, filter `where level=error`, count
/// matching lines from piped stdout without storing the full output in memory.
///
/// Expected:  exactly total/4 lines (every 4th record is level=error).
/// Threshold: elapsed < 120 s.
#[test]
#[ignore = "large file — run with: cargo test --test large_file -- --ignored --nocapture"]
fn large_file_streaming_filter_2gb() {
    let tmp = NamedTempFile::new().expect("tempfile");
    let target: u64 = 2 * 1024 * 1024 * 1024; // 2 GiB
    println!("[large_file_streaming_filter_2gb] writing ~2 GB …");
    let total_records = write_ndjson(tmp.path(), target);
    let file_size = tmp.path().metadata().expect("metadata").len();
    println!(
        "[large_file_streaming_filter_2gb] wrote {} records ({:.1} MB)",
        total_records,
        file_size as f64 / 1_048_576.0
    );

    let start = Instant::now();
    let stdin_file = File::open(tmp.path()).expect("open tempfile for stdin");

    let mut child = Command::new(cargo_bin("qk"))
        .args(["where", "level=error"])
        .stdin(Stdio::from(stdin_file))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn qk");

    let stdout = child.stdout.take().expect("child stdout");
    let matched = count_lines_from_reader(stdout);

    let status = child.wait().expect("wait");
    let elapsed = start.elapsed();
    let mb_per_sec = file_size as f64 / 1_048_576.0 / elapsed.as_secs_f64();

    println!(
        "[large_file_streaming_filter_2gb] matched={} total={} elapsed={:.1}s throughput={:.0} MB/s",
        matched,
        total_records,
        elapsed.as_secs_f64(),
        mb_per_sec
    );

    assert!(status.success(), "qk exited with non-zero status");
    assert_eq!(
        matched,
        (total_records / 4) as usize,
        "expected exactly total/4 error records"
    );
    assert!(
        elapsed.as_secs() < 120,
        "streaming filter took too long: {:.1}s",
        elapsed.as_secs_f64()
    );
}

// ---------------------------------------------------------------------------
// Test 2 — Streaming where latency gt 500 on 2 GB stdin
// ---------------------------------------------------------------------------

/// Stream 2 GB stdin with `where latency gt 500`.
///
/// latency = 10 + (n % 990).  Records with latency > 500:
///   n%990 >= 491 → values 501..999 = 499 values out of 990.
/// Expected fraction: 499/990 ≈ 50.4 % → allow ±1 % tolerance.
#[test]
#[ignore = "large file — run with: cargo test --test large_file -- --ignored --nocapture"]
fn large_file_streaming_latency_filter_2gb() {
    let tmp = NamedTempFile::new().expect("tempfile");
    let target: u64 = 2 * 1024 * 1024 * 1024;
    println!("[large_file_streaming_latency_filter_2gb] writing ~2 GB …");
    let total_records = write_ndjson(tmp.path(), target);
    let file_size = tmp.path().metadata().expect("metadata").len();
    println!(
        "[large_file_streaming_latency_filter_2gb] wrote {} records ({:.1} MB)",
        total_records,
        file_size as f64 / 1_048_576.0
    );

    let start = Instant::now();
    let stdin_file = File::open(tmp.path()).expect("open tempfile for stdin");

    let mut child = Command::new(cargo_bin("qk"))
        .args(["where", "latency", "gt", "500"])
        .stdin(Stdio::from(stdin_file))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn qk");

    let stdout = child.stdout.take().expect("child stdout");
    let matched = count_lines_from_reader(stdout);

    let status = child.wait().expect("wait");
    let elapsed = start.elapsed();
    let mb_per_sec = file_size as f64 / 1_048_576.0 / elapsed.as_secs_f64();

    println!(
        "[large_file_streaming_latency_filter_2gb] matched={} total={} elapsed={:.1}s throughput={:.0} MB/s",
        matched,
        total_records,
        elapsed.as_secs_f64(),
        mb_per_sec
    );

    // 499 out of every 990 records match latency > 500
    let full_cycles = total_records / 990;
    let remainder = total_records % 990;
    // remainder records with n%990 in 0..remainder; those with n%990 >= 491 match
    let remainder_matches = if remainder > 491 { remainder - 491 } else { 0 };
    let expected = full_cycles * 499 + remainder_matches;

    assert!(status.success(), "qk exited with non-zero status");
    // allow ±1 record due to boundary arithmetic
    let diff = (matched as i64 - expected as i64).unsigned_abs();
    assert!(
        diff <= 1,
        "expected ~{} records with latency>500, got {}",
        expected,
        matched
    );
    assert!(
        elapsed.as_secs() < 120,
        "streaming filter took too long: {:.1}s",
        elapsed.as_secs_f64()
    );
}

// ---------------------------------------------------------------------------
// Test 3 — count by level on 200 MB file
// ---------------------------------------------------------------------------

/// Batch mode: `count by level` on a 200 MB file.
///
/// Expected: 4 groups, each with count = total/4 records.
/// Threshold: elapsed < 60 s.
#[test]
#[ignore = "large file — run with: cargo test --test large_file -- --ignored --nocapture"]
fn large_file_count_by_200mb() {
    let tmp = NamedTempFile::new().expect("tempfile");
    let target: u64 = 200 * 1024 * 1024;
    println!("[large_file_count_by_200mb] writing ~200 MB …");
    let total_records = write_ndjson(tmp.path(), target);
    let file_size = tmp.path().metadata().expect("metadata").len();
    println!(
        "[large_file_count_by_200mb] wrote {} records ({:.1} MB)",
        total_records,
        file_size as f64 / 1_048_576.0
    );

    let start = Instant::now();
    let output = Command::new(cargo_bin("qk"))
        .args(["count", "by", "level", tmp.path().to_str().unwrap()])
        .output()
        .expect("run qk");
    let elapsed = start.elapsed();

    println!(
        "[large_file_count_by_200mb] elapsed={:.1}s",
        elapsed.as_secs_f64()
    );

    assert!(output.status.success(), "qk exited with non-zero status");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();

    println!("[large_file_count_by_200mb] output:\n{}", stdout);

    assert_eq!(
        lines.len(),
        4,
        "expected 4 level groups, got {}",
        lines.len()
    );

    // Parse each group and verify counts
    let expected_per_group = total_records / 4;
    for line in &lines {
        let v: serde_json::Value = serde_json::from_str(line).expect("output line is valid JSON");
        let count = v["count"].as_u64().expect("count field");
        // allow ±1 for rounding when total_records is not divisible by 4
        let diff = (count as i64 - expected_per_group as i64).unsigned_abs();
        assert!(
            diff <= 1,
            "group count {} differs from expected {} by more than 1 (line: {})",
            count,
            expected_per_group,
            line
        );
    }

    assert!(
        elapsed.as_secs() < 60,
        "count by took too long: {:.1}s",
        elapsed.as_secs_f64()
    );
}

// ---------------------------------------------------------------------------
// Test 4 — count total on 200 MB file
// ---------------------------------------------------------------------------

/// Batch mode: `count` on a 200 MB file.
///
/// Expected: output is `{"count": N}` where N = total records written.
/// Threshold: elapsed < 60 s.
#[test]
#[ignore = "large file — run with: cargo test --test large_file -- --ignored --nocapture"]
fn large_file_count_total_200mb() {
    let tmp = NamedTempFile::new().expect("tempfile");
    let target: u64 = 200 * 1024 * 1024;
    println!("[large_file_count_total_200mb] writing ~200 MB …");
    let total_records = write_ndjson(tmp.path(), target);
    let file_size = tmp.path().metadata().expect("metadata").len();
    println!(
        "[large_file_count_total_200mb] wrote {} records ({:.1} MB)",
        total_records,
        file_size as f64 / 1_048_576.0
    );

    let start = Instant::now();
    let output = Command::new(cargo_bin("qk"))
        .args(["count", tmp.path().to_str().unwrap()])
        .output()
        .expect("run qk");
    let elapsed = start.elapsed();

    println!(
        "[large_file_count_total_200mb] elapsed={:.1}s",
        elapsed.as_secs_f64()
    );

    assert!(output.status.success(), "qk exited with non-zero status");

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("[large_file_count_total_200mb] output: {}", stdout.trim());

    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("output is valid JSON");
    let count = v["count"].as_u64().expect("count field");

    assert_eq!(
        count, total_records,
        "count mismatch: got {} expected {}",
        count, total_records
    );

    assert!(
        elapsed.as_secs() < 60,
        "count took too long: {:.1}s",
        elapsed.as_secs_f64()
    );
}

// ---------------------------------------------------------------------------
// Test 5 — sum latency on 200 MB file
// ---------------------------------------------------------------------------

/// Batch mode: `sum latency` on a 200 MB file.
///
/// Exact formula:
///   latency(n) = 10 + (n % 990)
///   cycle_sum  = sum of 10..=999 = sum(10..999+1) = 990 * (10+999) / 2 = 499_455
///   full_cycles = total / 990
///   remainder   = total % 990   → sum of 10..(10+remainder-1)
///   total_sum   = full_cycles * 499_455 + sum(10..10+remainder)
///
/// All values are integers so the result fits exactly in i64.  We compare the
/// qk output (parsed as i64 via f64) for relative error < 1e-9.
#[test]
#[ignore = "large file — run with: cargo test --test large_file -- --ignored --nocapture"]
fn large_file_sum_latency_200mb() {
    let tmp = NamedTempFile::new().expect("tempfile");
    let target: u64 = 200 * 1024 * 1024;
    println!("[large_file_sum_latency_200mb] writing ~200 MB …");
    let total_records = write_ndjson(tmp.path(), target);
    let file_size = tmp.path().metadata().expect("metadata").len();
    println!(
        "[large_file_sum_latency_200mb] wrote {} records ({:.1} MB)",
        total_records,
        file_size as f64 / 1_048_576.0
    );

    let start = Instant::now();
    let output = Command::new(cargo_bin("qk"))
        .args(["sum", "latency", tmp.path().to_str().unwrap()])
        .output()
        .expect("run qk");
    let elapsed = start.elapsed();

    println!(
        "[large_file_sum_latency_200mb] elapsed={:.1}s",
        elapsed.as_secs_f64()
    );

    assert!(output.status.success(), "qk exited with non-zero status");

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("[large_file_sum_latency_200mb] output: {}", stdout.trim());

    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("output is valid JSON");
    let got_sum = v["sum"].as_f64().expect("sum field");

    // Exact expected sum
    // cycle_sum = sum of (10 + k) for k in 0..990 = 990*10 + (0+989)*990/2 = 9900 + 489555 = 499455
    const CYCLE_SUM: u64 = 499_455;
    let full_cycles = total_records / 990;
    let remainder = total_records % 990;
    // sum of latency for remainder records: 10+(n%990) for n=0..remainder-1
    // = remainder*10 + sum(0..remainder-1) = remainder*10 + remainder*(remainder-1)/2
    let remainder_sum = remainder * 10 + remainder * (remainder - 1) / 2;
    let expected_sum = (full_cycles * CYCLE_SUM + remainder_sum) as f64;

    let rel_error = (got_sum - expected_sum).abs() / expected_sum.max(1.0);
    println!(
        "[large_file_sum_latency_200mb] expected={} got={} rel_error={:.2e}",
        expected_sum, got_sum, rel_error
    );

    assert!(
        rel_error < 1e-9,
        "sum latency relative error {:.2e} exceeds 1e-9 (expected={} got={})",
        rel_error,
        expected_sum,
        got_sum
    );

    assert!(
        elapsed.as_secs() < 60,
        "sum took too long: {:.1}s",
        elapsed.as_secs_f64()
    );
}

// ---------------------------------------------------------------------------
// Test 6 — avg latency on 200 MB file
// ---------------------------------------------------------------------------

/// Batch mode: `avg latency` on a 200 MB file.
///
/// cycle_avg = 499_455 / 990 ≈ 504.5
/// For a large file the per-record distribution converges to the cycle average.
/// We check that the result is within 0.5 of the cycle average.
#[test]
#[ignore = "large file — run with: cargo test --test large_file -- --ignored --nocapture"]
fn large_file_avg_latency_200mb() {
    let tmp = NamedTempFile::new().expect("tempfile");
    let target: u64 = 200 * 1024 * 1024;
    println!("[large_file_avg_latency_200mb] writing ~200 MB …");
    let total_records = write_ndjson(tmp.path(), target);
    let file_size = tmp.path().metadata().expect("metadata").len();
    println!(
        "[large_file_avg_latency_200mb] wrote {} records ({:.1} MB)",
        total_records,
        file_size as f64 / 1_048_576.0
    );

    let start = Instant::now();
    let output = Command::new(cargo_bin("qk"))
        .args(["avg", "latency", tmp.path().to_str().unwrap()])
        .output()
        .expect("run qk");
    let elapsed = start.elapsed();

    println!(
        "[large_file_avg_latency_200mb] elapsed={:.1}s",
        elapsed.as_secs_f64()
    );

    assert!(output.status.success(), "qk exited with non-zero status");

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("[large_file_avg_latency_200mb] output: {}", stdout.trim());

    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("output is valid JSON");
    let got_avg = v["avg"].as_f64().expect("avg field");

    // Exact cycle average: 499455 / 990 = 504.5
    const CYCLE_AVG: f64 = 499_455.0 / 990.0;
    let diff = (got_avg - CYCLE_AVG).abs();

    println!(
        "[large_file_avg_latency_200mb] cycle_avg={:.4} got_avg={:.4} diff={:.4}",
        CYCLE_AVG, got_avg, diff
    );

    assert!(
        diff < 0.5,
        "avg latency {:.4} differs from cycle avg {:.4} by {:.4} (> 0.5)",
        got_avg,
        CYCLE_AVG,
        diff
    );

    assert!(
        elapsed.as_secs() < 60,
        "avg took too long: {:.1}s",
        elapsed.as_secs_f64()
    );
}

// ---------------------------------------------------------------------------
// Test 7 — corrupt-line resilience on 50 MB file
// ---------------------------------------------------------------------------

/// Write 50 MB of good NDJSON, then append 200 corrupt lines.
///
/// Run `count` via file path.  Expected:
///   - exit status success
///   - count == total_good (corrupt lines are skipped)
///   - stderr contains "[qk warning]"
#[test]
#[ignore = "large file — run with: cargo test --test large_file -- --ignored --nocapture"]
fn large_file_corrupt_lines_resilience_50mb() {
    let tmp = NamedTempFile::new().expect("tempfile");
    let target: u64 = 50 * 1024 * 1024;
    println!("[large_file_corrupt_lines_resilience_50mb] writing ~50 MB …");
    let total_good = write_ndjson(tmp.path(), target);
    append_corrupt_lines(tmp.path(), 200);
    let file_size = tmp.path().metadata().expect("metadata").len();
    println!(
        "[large_file_corrupt_lines_resilience_50mb] wrote {} good + 200 corrupt lines ({:.1} MB total)",
        total_good,
        file_size as f64 / 1_048_576.0
    );

    let output = Command::new(cargo_bin("qk"))
        .args(["count", tmp.path().to_str().unwrap()])
        .output()
        .expect("run qk");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!(
        "[large_file_corrupt_lines_resilience_50mb] stdout: {}",
        stdout.trim()
    );
    println!(
        "[large_file_corrupt_lines_resilience_50mb] stderr (first 500 chars): {}",
        &stderr[..stderr.len().min(500)]
    );

    assert!(
        output.status.success(),
        "qk should succeed even with corrupt lines"
    );

    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("output is valid JSON");
    let count = v["count"].as_u64().expect("count field");

    assert_eq!(
        count, total_good,
        "count should equal good-line count (got {} expected {})",
        count, total_good
    );

    assert!(
        stderr.contains("[qk warning]"),
        "stderr should contain '[qk warning]' for corrupt lines; got: {}",
        &stderr[..stderr.len().min(300)]
    );
}

// ---------------------------------------------------------------------------
// Test 8 — avg on nonexistent field on 50 MB file
// ---------------------------------------------------------------------------

/// Run `avg nonexistent_field` on a 50 MB file.
///
/// Expected:
///   - exit status success
///   - stdout JSON has `"avg": null` (no numeric values found)
///   - stderr contains "[qk warning]"
#[test]
#[ignore = "large file — run with: cargo test --test large_file -- --ignored --nocapture"]
fn large_file_avg_null_field_50mb() {
    let tmp = NamedTempFile::new().expect("tempfile");
    let target: u64 = 50 * 1024 * 1024;
    println!("[large_file_avg_null_field_50mb] writing ~50 MB …");
    let total_records = write_ndjson(tmp.path(), target);
    let file_size = tmp.path().metadata().expect("metadata").len();
    println!(
        "[large_file_avg_null_field_50mb] wrote {} records ({:.1} MB)",
        total_records,
        file_size as f64 / 1_048_576.0
    );

    let output = Command::new(cargo_bin("qk"))
        .args(["avg", "nonexistent_field", tmp.path().to_str().unwrap()])
        .output()
        .expect("run qk");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("[large_file_avg_null_field_50mb] stdout: {}", stdout.trim());
    println!(
        "[large_file_avg_null_field_50mb] stderr (first 300 chars): {}",
        &stderr[..stderr.len().min(300)]
    );

    assert!(
        output.status.success(),
        "qk should succeed even when field is absent"
    );

    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("output is valid JSON");

    assert!(
        v["avg"].is_null(),
        "avg of nonexistent field should be null; got: {}",
        v["avg"]
    );

    assert!(
        stderr.contains("[qk warning]"),
        "stderr should contain '[qk warning]' for missing field; got: {}",
        &stderr[..stderr.len().min(300)]
    );
}

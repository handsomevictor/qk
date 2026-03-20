use assert_cmd::Command;

fn qk() -> Command {
    Command::cargo_bin("qk").unwrap()
}

const SAMPLE_NDJSON: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/sample.ndjson"
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
fn count_total() {
    let input = "{\"a\":1}\n{\"a\":2}\n{\"a\":3}\n";
    let out = qk().arg("count").write_stdin(input).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["count"], 3);
}

#[test]
fn chain_qk_pipes() {
    // First qk filters, second qk counts — simulates piping
    let input = concat!(
        "{\"level\":\"error\",\"service\":\"api\"}\n",
        "{\"level\":\"error\",\"service\":\"web\"}\n",
        "{\"level\":\"info\",\"service\":\"api\"}\n",
    );
    // Filter to errors first
    let step1 = qk()
        .args(["where", "level=error"])
        .write_stdin(input)
        .output()
        .unwrap();
    // Count from step1 output
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
    qk()
        .args(["--explain", "where", "level=error", SAMPLE_NDJSON])
        .assert()
        .success();
}

#[test]
fn no_args_reads_empty_stdin() {
    qk().write_stdin("").assert().success();
}

#[test]
fn invalid_limit_errors() {
    qk()
        .args(["limit", "abc"])
        .write_stdin("{\"a\":1}\n")
        .assert()
        .failure();
}

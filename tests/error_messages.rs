//! Comprehensive integration tests for qk error messages.
//!
//! Every recognised error scenario has at least one test here:
//!   - Unknown / misspelled flags (with "did you mean?" suggestions)
//!   - Position-independent flags (flags after positional args)
//!   - Bad --fmt values
//!   - Bad --cast syntax (missing '=', unknown type name with suggestion)
//!   - File not found
//!   - No query provided
//!   - Query syntax errors
//!   - --explain output smoke-test
//!
//! All tests use `assert_cmd` so they run against the real compiled binary.

use assert_cmd::Command;
use predicates::prelude::*;

fn qk() -> Command {
    Command::cargo_bin("qk").unwrap()
}

const NDJSON: &str = "tests/fixtures/sample.ndjson";
const CSV: &str = "tests/fixtures/sample.csv";

// ── Unknown / misspelled flags ────────────────────────────────────────────────

#[test]
fn unknown_flag_exact_typo_quiet() {
    // --quite is 1 edit away from --quiet → should suggest --quiet
    qk().args(["--quite", NDJSON])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown flag '--quite'"))
        .stderr(predicate::str::contains("Did you mean: --quiet?"));
}

#[test]
fn unknown_flag_exact_typo_all() {
    // --al is 1 edit from --all
    qk().args(["--al", NDJSON])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown flag '--al'"))
        .stderr(predicate::str::contains("Did you mean: --all?"));
}

#[test]
fn unknown_flag_exact_typo_stats() {
    // --stat is 1 edit from --stats
    qk().args(["--stat", NDJSON])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown flag '--stat'"))
        .stderr(predicate::str::contains("Did you mean: --stats?"));
}

#[test]
fn unknown_flag_exact_typo_no_color() {
    // --no-colour (UK spelling) is 2 edits from --no-color
    qk().args(["--no-colour", NDJSON])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown flag '--no-colour'"));
}

#[test]
fn unknown_flag_completely_unknown_shows_valid_flags() {
    qk().args(["--xyzzy", NDJSON])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown flag '--xyzzy'"))
        .stderr(predicate::str::contains("Valid flags:"))
        .stderr(predicate::str::contains("qk --help"));
}

#[test]
fn unknown_flag_shows_help_hint() {
    qk().args(["--nope", NDJSON])
        .assert()
        .failure()
        .stderr(predicate::str::contains("qk --help"));
}

#[test]
fn unknown_short_flag_shows_error() {
    qk().args(["-z", NDJSON])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown flag '-z'"));
}

// ── Position-independent flags ────────────────────────────────────────────────

#[test]
fn quiet_after_query_tokens_is_accepted() {
    // --quiet placed after the query keyword (not at the start)
    qk().args(["count", "--quiet", NDJSON]).assert().success();
}

#[test]
fn quiet_after_file_is_accepted() {
    // --quiet as the very last token
    qk().args(["count", NDJSON, "--quiet"]).assert().success();
}

#[test]
fn all_flag_after_query_is_accepted() {
    qk().args(["count", "--all", NDJSON]).assert().success();
}

#[test]
fn stats_flag_after_query_is_accepted() {
    qk().args(["count", NDJSON, "--stats"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Stats:"));
}

#[test]
fn no_color_after_query_is_accepted() {
    qk().args(["count", NDJSON, "--no-color"])
        .assert()
        .success();
}

#[test]
fn fmt_flag_after_query_is_accepted() {
    qk().args(["count", "--fmt", "pretty", NDJSON])
        .assert()
        .success();
}

#[test]
fn cast_flag_after_query_is_accepted() {
    qk().args(["avg", "latency", "--cast", "latency=number", NDJSON])
        .assert()
        .success();
}

#[test]
fn mixed_flags_before_and_after_query() {
    // --no-color before query, --quiet after file
    qk().args(["--no-color", "count", NDJSON, "--quiet"])
        .assert()
        .success();
}

#[test]
fn typo_flag_after_positional_gives_helpful_error() {
    // Flag typo placed after query tokens — should still give the good error
    qk().args(["count", NDJSON, "--quite"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown flag '--quite'"))
        .stderr(predicate::str::contains("Did you mean: --quiet?"));
}

// ── Bad --fmt values ──────────────────────────────────────────────────────────

#[test]
fn bad_fmt_value_shows_error() {
    qk().args(["--fmt", "xml", NDJSON])
        .assert()
        .failure()
        .stderr(predicate::str::contains("xml").or(predicate::str::contains("fmt")));
}

#[test]
fn bad_fmt_value_empty_shows_error() {
    qk().args(["--fmt", "", NDJSON]).assert().failure();
}

#[test]
fn fmt_ndjson_is_valid() {
    qk().args(["--fmt", "ndjson", "count", NDJSON])
        .assert()
        .success();
}

#[test]
fn fmt_pretty_is_valid() {
    qk().args(["--fmt", "pretty", "count", NDJSON])
        .assert()
        .success();
}

#[test]
fn fmt_table_is_valid() {
    qk().args(["--fmt", "table", "count", NDJSON])
        .assert()
        .success();
}

#[test]
fn fmt_csv_is_valid() {
    qk().args(["--fmt", "csv", "count", NDJSON])
        .assert()
        .success();
}

#[test]
fn fmt_raw_is_valid() {
    qk().args(["--fmt", "raw", "count", NDJSON])
        .assert()
        .success();
}

// ── Bad --cast syntax ─────────────────────────────────────────────────────────

#[test]
fn cast_missing_equals_gives_actionable_error() {
    qk().args(["--cast", "latencynumber", "count", NDJSON])
        .assert()
        .failure()
        .stderr(predicate::str::contains("FIELD=TYPE"))
        .stderr(predicate::str::contains("latency=number"));
}

#[test]
fn cast_unknown_type_gives_supported_list() {
    qk().args(["--cast", "latency=foobar", "count", NDJSON])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown type"))
        .stderr(predicate::str::contains("Supported:"));
}

#[test]
fn cast_typo_type_suggests_correction_nubmer() {
    // "nubmer" is 2 edits from "number"
    qk().args(["--cast", "latency=nubmer", "count", NDJSON])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Did you mean: num"));
}

#[test]
fn cast_typo_type_suggests_correction_strng() {
    // "strng" is close to "string" or "str"
    qk().args(["--cast", "field=strng", "count", NDJSON])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Did you mean:"));
}

#[test]
fn cast_typo_type_suggests_correction_boolian() {
    // "boolian" is close to "boolean"
    qk().args(["--cast", "field=boolian", "count", NDJSON])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Did you mean:"));
}

#[test]
fn cast_number_type_works() {
    qk().args(["--cast", "latency=number", "avg", "latency", NDJSON])
        .assert()
        .success();
}

#[test]
fn cast_num_alias_works() {
    qk().args(["--cast", "latency=num", "avg", "latency", NDJSON])
        .assert()
        .success();
}

#[test]
fn cast_str_alias_works() {
    qk().args(["--cast", "latency=str", "count", NDJSON])
        .assert()
        .success();
}

#[test]
fn cast_string_type_works() {
    qk().args(["--cast", "latency=string", "count", NDJSON])
        .assert()
        .success();
}

#[test]
fn cast_bool_type_works() {
    qk().args(["--cast", "latency=bool", "count", NDJSON])
        .assert()
        .success();
}

#[test]
fn cast_auto_type_works() {
    qk().args(["--cast", "latency=auto", "count", NDJSON])
        .assert()
        .success();
}

#[test]
fn cast_null_type_works() {
    qk().args(["--cast", "latency=null", "count", NDJSON])
        .assert()
        .success();
}

// ── File not found ────────────────────────────────────────────────────────────

#[test]
fn file_not_found_shows_path() {
    qk().args(["count", "nonexistent_file.ndjson"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("nonexistent_file.ndjson"));
}

#[test]
fn file_not_found_shows_io_error() {
    qk().args(["count", "/tmp/definitely_does_not_exist_qk_test.ndjson"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("IO error").or(predicate::str::contains("No such file")));
}

#[test]
fn file_not_found_not_treated_as_flag() {
    // A non-existent file that doesn't start with '-' should say IO error, not flag error
    qk().args(["count", "missing.log"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match("IO error|No such file|missing.log").unwrap());
}

// ── Flag-looking path gives flag error (not IO error) ─────────────────────────

#[test]
fn dash_prefixed_nonexistent_gives_flag_error_not_io_error() {
    // A path like "--quite" should produce an "unknown flag" error,
    // not "IO error reading '--quite': No such file or directory"
    qk().args(["count", "--quite"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown flag"))
        .stderr(predicate::str::contains("Did you mean: --quiet?"));
}

// ── Query syntax errors ───────────────────────────────────────────────────────

#[test]
fn dsl_syntax_error_shows_caret() {
    // Invalid DSL expression should show a caret pointing at the error
    qk().args([".field ==", NDJSON])
        .assert()
        .failure()
        .stderr(predicate::str::contains("^^^").or(predicate::str::contains("syntax error")));
}

#[test]
fn keyword_query_unknown_aggregation_shows_error() {
    qk().args(["frobulate", "latency", NDJSON])
        .assert()
        .failure();
}

#[test]
fn where_clause_without_field_errors_gracefully() {
    qk().args(["where", NDJSON])
        .assert()
        .failure()
        .stderr(predicate::str::is_match("error|Error|missing|require").unwrap());
}

// ── --cast with position-independent placement ────────────────────────────────

#[test]
fn cast_before_query_works() {
    qk().args(["--cast", "latency=number", "avg", "latency", NDJSON])
        .assert()
        .success();
}

#[test]
fn cast_after_query_before_file_works() {
    qk().args(["avg", "latency", "--cast", "latency=number", NDJSON])
        .assert()
        .success();
}

#[test]
fn cast_after_file_works() {
    qk().args(["avg", "latency", NDJSON, "--cast", "latency=number"])
        .assert()
        .success();
}

// ── Multiple files ────────────────────────────────────────────────────────────

#[test]
fn multiple_files_second_missing_reports_error() {
    qk().args(["count", NDJSON, "missing_second.ndjson"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing_second.ndjson"));
}

// ── Valid operations smoke tests (ensure no false-positive failures) ──────────

#[test]
fn count_on_ndjson_succeeds() {
    qk().args(["count", NDJSON]).assert().success();
}

#[test]
fn count_on_csv_succeeds() {
    qk().args(["count", CSV]).assert().success();
}

#[test]
fn select_field_on_ndjson_succeeds() {
    qk().args(["select", "level", NDJSON]).assert().success();
}

#[test]
fn where_eq_filter_succeeds() {
    // 'where' filter requires FIELD=VALUE as a single token (no spaces around '=')
    qk().args(["where", "level=info", NDJSON])
        .assert()
        .success();
}

#[test]
fn head_n_succeeds() {
    qk().args(["head", "2", NDJSON]).assert().success();
}

#[test]
fn sort_asc_succeeds() {
    qk().args(["sort", "latency", NDJSON]).assert().success();
}

#[test]
fn sort_desc_succeeds() {
    qk().args(["sort", "latency", "desc", NDJSON])
        .assert()
        .success();
}

#[test]
fn avg_numeric_field_succeeds() {
    qk().args(["avg", "latency", NDJSON]).assert().success();
}

#[test]
fn sum_numeric_field_succeeds() {
    qk().args(["sum", "latency", NDJSON]).assert().success();
}

#[test]
fn min_numeric_field_succeeds() {
    qk().args(["min", "latency", NDJSON]).assert().success();
}

#[test]
fn max_numeric_field_succeeds() {
    qk().args(["max", "latency", NDJSON]).assert().success();
}

#[test]
fn count_by_field_succeeds() {
    qk().args(["count", "by", "level", NDJSON])
        .assert()
        .success();
}

#[test]
fn explain_flag_succeeds() {
    // --explain prints the query plan to stdout
    qk().args(["--explain", "count", NDJSON])
        .assert()
        .success()
        .stdout(predicate::str::contains("Query Parse").or(predicate::str::contains("FastQuery")));
}

// ── --cast embedded-equals syntax ────────────────────────────────────────────

#[test]
fn cast_embedded_equals_syntax_works() {
    // --cast=latency=number (embedded '=') should also work
    qk().args(["--cast=latency=number", "avg", "latency", NDJSON])
        .assert()
        .success();
}

// ── Error message does NOT leak internal details ──────────────────────────────

#[test]
fn unknown_flag_error_does_not_say_no_such_file() {
    // The old bug: --quite produced "IO error reading '--quite': No such file or directory"
    // Now it must say "unknown flag" instead.
    qk().args(["--quite", NDJSON])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No such file or directory").not())
        .stderr(predicate::str::contains("unknown flag"));
}

#[test]
fn typo_flag_after_positional_does_not_say_no_such_file() {
    qk().args(["count", "--quite"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No such file or directory").not())
        .stderr(predicate::str::contains("unknown flag"));
}

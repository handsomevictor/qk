# LESSON LEARNED — Debug Log

Every meaningful bug, compile error, design mistake, or surprising discovery is recorded here.
Goal: never debug the same problem twice.

Entry format:
```
## LL-NNN — short title
- **Date**: YYYY-MM-DD
- **Phase**: Phase N
- **Symptom**: what went wrong / what was confusing
- **Root cause**: why it happened
- **Fix**: how it was resolved
- **Lesson**: the general rule to remember
```

---

## LL-001 — Cargo version incompatible with newer crates

- **Date**: 2026-03-20
- **Phase**: Phase 1
- **Symptom**: `cargo build` failed with `feature 'edition2024' is required`; clap 4.6.0 could not be downloaded
- **Root cause**: System Rust version was 1.76.0 (early 2024), but clap 4.6.0's `Cargo.toml` uses `edition = "2024"`, which requires Cargo 1.79+ to parse
- **Fix**: Ran `rustup update stable` to upgrade Rust to 1.94.0; all dependencies then resolved normally
- **Lesson**: Before starting a new Rust project, run `rustup update stable` to ensure the toolchain is up to date. If upgrading is not possible, use `cargo update <crate>@<version> --precise <old_version>` to pin a specific dependency to an older version

---

## LL-002 — Module name conflicts with crate name (preventive record, did not actually occur)

- **Date**: 2026-03-20
- **Phase**: Phase 1
- **Symptom**: If `pub mod csv` is declared in `src/parser/mod.rs`, and then `csv::ReaderBuilder::new()` is written in `src/parser/csv.rs`, there could be ambiguity
- **Root cause**: In Rust, `csv` can refer to (1) an external crate (from Cargo.toml) or (2) a submodule within the current module. In practice, inside `src/parser/csv.rs` itself, `csv` refers to the external crate; the submodule path is `crate::parser::csv`, so no ambiguity arises within the file
- **Fix**: No actual conflict occurred; compiled normally. If encountered in future, use `::csv::ReaderBuilder` to explicitly access the external crate from the crate root
- **Lesson**: In Rust, name resolution within a file: external crate names are valid in the current file scope; submodule names are only introduced as symbols in the parent module file (`mod.rs`) that declared them

---

## LL-003 — Unused import warning revealed a missing import

- **Date**: 2026-03-20
- **Phase**: Phase 2
- **Symptom**: `eval.rs` kept `use crate::util::error::{QkError, Result}` but `QkError` was unused, triggering an unused import warning
- **Root cause**: The eval functions were originally going to create `QkError` directly, but the design changed to only use `Result`; the import was not cleaned up
- **Fix**: Changed `use crate::util::error::{QkError, Result}` to `use crate::util::error::Result`
- **Lesson**: Rust's `unused_imports` warning is very valuable. After running `cargo build`, look at warnings before errors — warnings often point to logic gaps in the code

---

## Common Rust Pitfalls (project-wide)

The following are the most common mistakes beginner and intermediate Rust developers encounter in CLI/parser projects like this one. Recorded preemptively; will be updated with real occurrences.

### Lifetime errors in parser code
`nom` parsers return `&str` slices borrowing from the input. If parse results are stored in a struct that outlives the input buffer, the compiler will error. Fix: either clone the strings (`.to_string()`), or carry lifetime parameters in the struct definition.

### unwrap() panics in release builds
Using `.unwrap()` on `Result` and `Option` during development is convenient. But a single malformed input line will crash the entire process. Fix: before Phase 3, switch hot paths to `?` propagation or explicit `match` with fallbacks.

### rayon and Send constraints
rayon's parallel iterators require closures to capture types that implement `Send`. If a non-`Send` type (e.g. `Rc`, raw pointer, or `MutexGuard`) is accidentally captured, the compile error can be confusing. Fix: prefer `Arc` over `Rc` in shared state, and release locks before spawning tasks.

### serde field renaming
JSON keys like `"Content-Type"` cannot directly be Rust field names. Use `#[serde(rename = "Content-Type")]` or `#[serde(rename_all = "kebab-case")]` to handle them.

### memmap2 and empty files
On some platforms, calling mmap on a zero-length file will panic. Always check `file.metadata()?.len() > 0` before calling `MmapOptions::new().map()`.

### Field name vs file name ambiguity in select syntax
In `qk select ts msg app.log`, `app.log` is recognized as a file because the `looks_like_file()` function detects the `.log` extension. For files without extensions (e.g. `data`), use `./data` or an absolute path to disambiguate.

---

## LL-004 — clap trailing_var_arg swallows subsequent flags

- **Date**: 2026-03-20
- **Phase**: Phase 6
- **Symptom**: `qk where level=error --fmt table file.ndjson` errored with `IO error reading '--fmt': No such file or directory`
- **Root cause**: The `args` field in the CLI uses `trailing_var_arg = true`, so once clap encounters the first positional argument (`where`), it treats everything after — including `--fmt table` — as values for `args` rather than named flags
- **Fix**: Place `--fmt` and other flags before the query expression: `qk --fmt table where level=error file.ndjson`
- **Lesson**: `trailing_var_arg = true` is a "capture everything" mode. Named flags **must** appear before the first positional argument. Document this clearly in the CLI help text and TUTORIAL.md

---

## LL-005 — DSL mode detection only covered `.` prefix

- **Date**: 2026-03-20
- **Phase**: Phase 4
- **Symptom**: `qk 'not .level == "info"'` or `qk '| count()'` errored with `IO error reading 'not ...'` instead of executing the DSL query
- **Root cause**: `determine_mode` only checked whether the first argument starts with `.` to detect DSL mode. Expressions starting with `not` and `|` are also valid DSL, but were routed to keyword mode and then mistakenly treated as file paths
- **Fix**: Extended the condition in `determine_mode`: `first.starts_with("not ")` or `first.starts_with('|')` also trigger DSL mode
- **Lesson**: Mode detection must cover all valid starting tokens. When adding new syntax (e.g. `not expr`), remember to update the routing logic at the same time

---

## LL-006 — TOML section header `[section]` misidentified as JSON array

- **Date**: 2026-03-20
- **Phase**: Phase 5
- **Symptom**: `detect::tests::detects_toml_section_by_content` failed; `[server]\nport = 8080` was classified as `Json` instead of `Toml`
- **Root cause**: In `detect_from_content`, `if trimmed.starts_with('[')` returned `Format::Json` immediately; the TOML section header detection (`looks_like_toml`) came after and was never reached
- **Fix**: Inside the `[` branch, call `looks_like_toml` first; and tighten `looks_like_toml`'s section header detection — only treat as a TOML section header if the brackets contain no `{`, `"`, or `'`
- **Lesson**: Priority order in format detection is critical. When two formats share the same starting character (`[` is both a JSON array and a TOML section header), finer-grained disambiguation must happen within the same branch, not by relying on ordering

---

## LL-007 — Stale installed binary hides source-code fixes

- **Date**: 2026-03-21
- **Phase**: Phase 7
- **Symptom**: `--explain` still showed Chinese text; `gt`/`lt` operators still errored; comma separator not working — even though the source code was already correct
- **Root cause**: `~/.cargo/bin/qk` was the old binary installed from a different directory (`~/Downloads/qk`). Running `qk` from anywhere used the stale binary. Source changes in `~/Documents/GitHub/qk` were never compiled into the installed binary
- **Fix**: `cargo install --path .` from the correct project directory; confirmed with `which qk` and then `qk --explain where level=error` showing English output
- **Lesson**: After changing source code, `cargo run` uses the local build but the installed binary (`~/.cargo/bin/qk`) is only updated by `cargo install --path .`. Always confirm the active binary with `which qk` and a smoke test before debugging source code

---

## LL-008 — Fast layer regex (`~=`) was a stub using `str::contains()` instead of real regex

- **Date**: 2026-03-21
- **Phase**: Phase 7
- **Symptom**: `qk where 'msg~=.*timeout.*' app.log` returned no results. `qk where 'msg~=timeout' app.log` worked. Users reported regex filtering broken
- **Root cause**: `eval_regex()` in `src/query/fast/eval.rs` had a TODO comment: _"Simple regex: just check if the string contains the pattern for now. Phase 4 will add a proper regex engine."_ Phase 4 added real regex to the DSL layer only; the fast layer was never updated, so `~=` performed a literal substring match (`str::contains(pattern)`) instead of regex matching. `.*timeout.*` was searched as a literal string — never found
- **Fix**: Replaced `str::contains()` with `regex::Regex::new(pattern)?.is_match()`, using the same `regex` crate already in `Cargo.toml`
- **Lesson**: When a feature is implemented incrementally across phases, track all places that need updating. TODO comments like "Phase N will add X" must be converted to tracked tasks, not left as silent stubs. Regex tests should verify that `.*` patterns actually match, not just literal substrings

---

## LL-009 — zsh glob expansion breaks regex patterns containing `*`

- **Date**: 2026-03-21
- **Phase**: Phase 7
- **Symptom**: `qk where msg~=.*timeout.* app.log` triggered `zsh: no matches found: msg~=.*timeout.*`
- **Root cause**: zsh (and bash with `globbing`) treats `*` as a glob pattern. Before `qk` ever sees the argument, zsh tries to expand `msg~=.*timeout.*` as a file glob. When no files match, zsh errors out instead of passing the literal string
- **Fix**: Quote the argument: `qk where 'msg~=.*timeout.*' app.log`. Single quotes prevent any shell expansion
- **Lesson**: Any argument containing shell metacharacters (`*`, `?`, `[`, `]`, `{`, `}`, `~`) must be quoted. Document this prominently wherever regex syntax is shown. The DSL layer has the same issue: `qk '.msg matches ".*fail.*"'` — the outer single quotes are mandatory

---

## LL-010 — Trailing comma before a clause keyword caused a parse error

- **Date**: 2026-03-21
- **Phase**: Phase 7
- **Symptom**: `qk where level=error, select ts service msg app.log` errored with `cannot parse filter 'select'`. Users expected the trailing comma to work as a cosmetic separator before `select`, `count`, `avg`, etc.
- **Root cause**: In `parse_where_clause`, when a trailing comma was detected on a filter token (e.g. `level=error,`), the code unconditionally pushed `LogicalOp::And` and called `continue` to loop back. At the top of the loop, `parse_filter` was called on the next token (`select`), which is not a valid filter expression — hence the error
- **Fix**: Before pushing `And` and continuing, check if the next token is a clause-terminating keyword (`select`, `count`, `sort`, `limit`, `head`, `fields`, `sum`, `avg`, `min`, `max`, `where`) or a file path. If it is, `break` instead of `continue`. The trailing comma is then treated as optional punctuation
- **Lesson**: Separator tokens (comma, `and`) should be "greedy but bounded" — they imply more input is coming, but only if what follows is actually a valid continuation. Always check the lookahead before committing to a parse direction

---

## LL-011 — NDJSON mixed-type fields were silently wrong without warnings

- **Date**: 2026-03-21
- **Phase**: Phase 9
- **Symptom**: `qk avg latency app.log` returned a silently wrong result when some records had `latency: "None"` or `latency: "unknown"` as strings. No error, no indication anything was skipped.
- **Root cause**: `value_as_f64()` returned `None` for non-numeric strings, causing `filter_map` to silently drop those records from the aggregation. The caller had no visibility into how many records were skipped or why.
- **Fix**: Replaced `filter_map(...and_then(value_as_f64))` in `stat_agg` with a new `collect_numeric_field()` helper that distinguishes three cases: (1) null-like strings → silently skip; (2) parseable strings → use; (3) unexpected strings → skip AND emit a `[qk warning]` to stderr.
- **Lesson**: Aggregations that silently skip rows are dangerous — the user cannot tell if the result is trustworthy. Always make "unexpected skip" visible via a warning. Use stderr so the warning doesn't break piped output.

---

## LL-012 — Pre-compiling regex in AST requires Clone + Debug on Arc<Regex>

- **Date**: 2026-03-21
- **Phase**: T-01 (ROADMAP)
- **Symptom**: Adding `Literal::Regex(Arc<Regex>, String)` triggered a dead_code warning on the `String` field; clippy with `-D warnings` would fail in CI
- **Root cause**: The `String` field (original pattern) was stored for debugging purposes but never actually read anywhere in eval or error paths. The compiler correctly flagged it
- **Fix**: Removed the `String` field — `Regex` already stores the pattern internally and its `Debug` impl prints it. Variant became `Literal::Regex(Arc<regex::Regex>)`
- **Lesson**: When adding data to an enum variant "for future use", do not store it unless it is actually used. Dead_code warnings from `#[derive(Debug, Clone)]` are suppressed for derived impls, which can mask the issue — always check with clippy after adding new variants

---

## LL-013 — Streaming stdin path needed `Write` trait in scope for BufWriter::flush

- **Date**: 2026-03-21
- **Phase**: T-04 (ROADMAP)
- **Symptom**: `out.flush()` on `BufWriter<StdoutLock>` failed with "method not found" even though `BufWriter` implements `Write`
- **Root cause**: The `flush()` method lives on the `Write` trait. Even though `BufWriter` implements `Write`, calling `flush()` requires `Write` to be in scope via a `use` statement
- **Fix**: Added `use std::io::Write;` to `main.rs`
- **Lesson**: In Rust, trait methods are only callable when the trait is in scope. When adding new code that calls trait methods (flush, seek, etc.), always check that the relevant trait is imported — the compiler error message does tell you this, but it can be easy to miss when the type obviously should support the method

---

## LL-014 — `compare_values()` compared string length instead of lexicographic order

- **Date**: 2026-03-21
- **Phase**: Datetime audit
- **Symptom**: `qk where ts gt "2024-01-15T10:05:00Z"` returned wrong results — records with timestamps clearly before the threshold were included, and some records clearly after were excluded
- **Root cause**: The string fallback branch of `compare_values()` was using `val.len() as f64` vs `literal.len() as f64`, i.e. comparing the *lengths* of the two strings numerically. RFC 3339 timestamps are fixed-length and zero-padded ASCII, so length comparison always returned 0 (equal), making every `gt`/`lt` comparison useless
- **Fix**: Changed the string branch to use `value_to_str(val).as_str().cmp(literal)` — standard lexicographic comparison. Because RFC 3339 strings are zero-padded ISO 8601, dictionary order equals chronological order
- **Lesson**: Fallback branches for "can't convert to number" must never use string length as a proxy for ordering. Always use lexicographic `cmp`. Write at least one integration test that verifies a timestamp string filter actually excludes records on the wrong side of the threshold

---

## LL-015 — `cargo fmt` rejects single-line `if/else` with closures inside multiline expressions

- **Date**: 2026-03-21
- **Phase**: P1 audit fix / CI
- **Symptom**: GitHub Actions `cargo fmt --check` failed on `eval.rs` even though the code compiled and all tests passed locally. CI diff showed lines like `if nums.is_empty() { None } else { Some(nums.iter().sum::<f64>()) }` being flagged
- **Root cause**: `rustfmt` enforces that closures or blocks passed as function arguments must be expanded to multiline if the overall expression is already multiline. The `stat_agg` call-sites had a single-line `if/else` inline inside a closure that was passed to a function spanning multiple lines — rustfmt requires vertical expansion in this case
- **Fix**: Ran `cargo fmt` locally which auto-expanded the `if/else` blocks to multiline. The developer had not run `cargo fmt` before pushing
- **Lesson**: Always run `cargo fmt` locally before pushing, not just `cargo clippy`. `clippy` and `fmt` are independent. The CI `cargo fmt --check` step is a hard gate. One-liner `if/else` inside closure arguments is a common rustfmt rejection point

---

## LL-016 — Dead code from reverted feature caused clippy failure

- **Date**: 2026-03-21
- **Phase**: P6 audit fix
- **Symptom**: After partially implementing P6 (string interning for `Value::String` contents), then deciding P6 was not implementable due to `serde_json::Value::String(String)` owning its string, the implementation was reverted — but `intern_short()` and the `MAX_INTERN_VALUE_BYTES` constant were left in `src/util/intern.rs`. `cargo clippy -- -D warnings` then failed with `dead_code` warnings
- **Root cause**: A revert of a feature must delete all code that was only introduced to support that feature. Leaving behind helper functions "just in case" is not harmless in Rust — the compiler flags every unused item with a warning, and with `-D warnings` on CI, this becomes a build failure
- **Fix**: Removed both `const MAX_INTERN_VALUE_BYTES` and `pub fn intern_short` from `intern.rs`
- **Lesson**: When reverting a feature, delete *all* of it — constants, helper functions, imports, and tests. Do not leave "dead" code as a comment or a stub. Run `cargo clippy -- -D warnings` after every revert to confirm clean

---

## LL-017 — `detect_json_variant` too strict: corrupt NDJSON misdetected as JSON array

- **Date**: 2026-03-21
- **Phase**: Audit Step 2 / streaming resilience
- **Symptom**: DSL mode (`qk '.level == "error"' file.ndjson`) returned zero results when the file contained corrupt lines (e.g. truncated JSON). The NDJSON resilience fix (P0) correctly skipped corrupt lines in `parse()`, but DSL mode never reached the NDJSON parser
- **Root cause**: `detect_json_variant` in `detect.rs` required *all* sampled lines to parse successfully before classifying a file as `Ndjson`. A corrupt line anywhere in the sample caused fallback to `Json`, which then tried to parse the entire file as a JSON array — and failed entirely
- **Fix**: Changed `detect_json_variant` to return `Ndjson` as soon as the *first* line is a complete JSON object, regardless of what subsequent lines look like. The NDJSON parser already handles corrupt subsequent lines gracefully
- **Lesson**: Format detection and format parsing should have the same resilience contract. If the parser tolerates corrupt lines, the detector must not require perfect lines. Detect on the *common case* (first valid line), not the *global case* (all lines valid)

---

## LL-018 — `extract_is_weekend` used wrong weekday constant (`Wed` instead of `Sun`)

- **Date**: 2026-03-21
- **Phase**: Audit Step 5 (DSL time attributes)
- **Symptom**: `| is_weekend(.ts)` returned `true` for Wednesday records and `false` for Sunday records — the exact opposite of the intended behaviour
- **Root cause**: A typo in the initial implementation of `extract_is_weekend`: `chrono::Weekday::Sat | chrono::Weekday::Wed` instead of `chrono::Weekday::Sat | chrono::Weekday::Sun`. The code compiled without error because both `Wed` and `Sun` are valid `Weekday` enum variants
- **Fix**: Changed `Wed` to `Sun` in the match arm
- **Lesson**: Enum variants that are all valid spellings (all days of the week) will never produce a compile error for a typo. Write at least two tests for boolean classifiers: one "should be true" and one "should be false", using specific known values (a real Saturday and a real Sunday epoch) to catch this class of bug

---

## LL-019 — NDJSON `parse()` aborted on first corrupt line instead of skipping

- **Date**: 2026-03-21
- **Phase**: P0 audit fix
- **Symptom**: A single malformed line (truncated JSON, encoding error, etc.) in a large NDJSON file caused `qk` to return an error and produce zero output for the entire file
- **Root cause**: `parse()` in `src/parser/ndjson.rs` propagated the `?` operator from `parse_line()`. The first `Err` short-circuited the loop and returned immediately
- **Fix**: Changed `?` to an explicit `match`: on `Ok(record)` push to results; on `Err(e)` print `[qk warning] skipping corrupt line {n}: {e}` to stderr and `continue`. The file always produces partial results even with corrupt records
- **Lesson**: For streaming or multi-record parsers, never propagate errors with `?` at the per-record level. A single corrupt record in a million-line file should not abort the entire job. Use `match` with a `continue` + stderr warning pattern. Always test: (1) mixed corrupt+valid → correct records returned, (2) all corrupt → empty vec, not error

---

## LL-020 — `stat_agg` returned `0.0` for empty numeric slices (wrong implicit default)

- **Date**: 2026-03-21
- **Phase**: P1 audit fix
- **Symptom**: `qk avg latency app.log` returned `0` when the `latency` field was missing from all records, instead of `null` or an error. Users could not tell whether the average was genuinely zero or whether no records matched
- **Root cause**: `stat_agg` computed over an empty `Vec<f64>`. The `sum` implementation returned `0.0` (identity of addition); `avg` returned `0.0/0` which is `NaN`, which was then silently coerced to `0` in the JSON output path
- **Fix**: Changed `stat_agg`'s closure signature to `Fn(&[f64]) -> Option<f64>`. Each aggregator (`sum`, `avg`, `min`, `max`) now returns `None` for an empty slice. The caller maps `None` to `Value::Null` and emits a `[qk warning]` to stderr
- **Lesson**: Aggregations over empty sets do not have a meaningful numeric result. Returning `0` is actively harmful (confuses "no data" with "data that sums to zero"). Always return `null`/`None` for empty aggregations and warn the user. Design aggregation closures to return `Option<f64>` from the start

---

## LL-021 — `Record.raw` as `String` allocated empty strings for every synthetic record

- **Date**: 2026-03-21
- **Phase**: P4 audit fix
- **Symptom**: Not a crash — a memory efficiency issue. Every record produced by aggregation (`count by`, `sum`, `avg`, etc.) and DSL stages carried a `raw: String::new()` field — an empty string heap allocation that served no purpose for synthetic records
- **Root cause**: `Record.raw` was always `String`. Synthetic records (those not parsed from a raw input line) had no meaningful raw content, so they used `String::new()` — which still allocates a heap object
- **Fix**: Changed `Record.raw` to `Option<String>`. Parsers pass `Some(line.to_string())`; synthetic record constructors pass `None`. `write_raw` uses `rec.raw.as_deref().unwrap_or("")`. This eliminates heap allocations for all aggregation output records
- **Lesson**: Fields that are only meaningful for some variants of a type should be `Option<T>`, not `T`. Using `String::new()` as a sentinel "empty" value looks harmless but wastes heap space at scale (e.g. 1M aggregation buckets × 24 bytes each). Model absence explicitly with `Option`

---

## LL-022 — nom error from DSL parser gave no actionable location or context

- **Date**: 2026-03-21
- **Phase**: Audit Step 3
- **Symptom**: A malformed DSL expression like `'.level == "error" | pick(.a, .b'` (unclosed paren) produced a raw nom error string that showed internal combinator names (`alt`, `many0`, `tag`) and no indication of where in the input the problem occurred
- **Root cause**: nom's default `Error` type records the *remaining input* at the point of failure, not the *position* in the original string. Surfacing the raw nom error to the user exposed library internals
- **Fix**: Extracted `dsl_parse_error(input: &str, err: nom::Err<...>) -> String` that: (1) computes the failure position as `input.len() - remaining.len()`; (2) shows up to 40 chars of context before the failure; (3) adds targeted hints — "unclosed `(`", "expected right-hand side", "unexpected token"
- **Lesson**: Never surface raw parser library error types to users. Always add an adapter that translates internal error state (remaining input offset) to human-readable position + context. The translation logic is small (~30 lines) but dramatically improves debuggability

---

## LL-023 — Multi-field grouping requires composite keys, not single-field enum variant

- **Date**: 2026-03-21
- **Phase**: P1 (multi-field grouping)
- **Symptom**: N/A — design decision recorded preemptively
- **Root cause**: `Aggregation::CountBy(String)` and `Stage::GroupBy(FieldPath)` only held one field, making `count by level service` impossible without a new variant
- **Fix**: Changed both to hold `Vec<String>` / `Vec<FieldPath>`. Grouping uses a NUL-byte (`\x00`) joined composite key for the `IndexMap`, then splits the key back on output to populate individual field columns in the result record. This is backward-compatible — single-field use becomes `vec![field]`
- **Lesson**: When designing group-by enums, use `Vec<FieldName>` from the start rather than `FieldName`. The single-field case is `Vec` of length 1 — no special casing needed. Using a NUL-byte separator for composite keys is safe as long as field values don't contain NUL (NDJSON strings cannot)

---

## LL-024 — `length` in arithmetic expressions requires registration before `parse_field_path`

- **Date**: 2026-03-21
- **Phase**: P2 (string/array functions)
- **Symptom**: `| map(.n = length(.msg))` parsed successfully but `length` was silently treated as a field named `length` rather than a function call
- **Root cause**: `parse_arith_primary` tried `parse_field_path` first via `alt(...)`. `.length` would fail (requires leading dot), but the nom `alt` then tries the next option. However, `length(.msg)` without the leading dot could be parsed as an identifier in some contexts
- **Fix**: In `parse_arith_primary`, put the `length(...)` branch first in the `alt(...)` list, before `parse_field_path` and `double`. nom's `alt` returns the first successful match; placing `length` first ensures function-call syntax is tried before falling back to field path parsing
- **Lesson**: In nom `alt(...)`, order matters. More specific/longer patterns must precede shorter/more generic ones. A function call like `length(.x)` starts with the same characters as a potential identifier, so it must appear first

---

<!-- Add new entries above this line, incrementing LL-NNN -->

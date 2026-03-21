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

<!-- Add new entries above this line, incrementing LL-NNN -->

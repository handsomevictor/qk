# ROADMAP — qk Execution Plan

This document converts the post-review technical debt and feature gaps into a
concrete, implementation-ready plan organized by phase and priority.

Last updated: 2026-03-21

---

## Phase 1 — Immediate Fixes (1–2 days)

### T-01 · Fix Regex Per-Record Recompilation

**Priority:** P0
**Complexity:** S
**Why:** `Regex::new()` is called inside the hot loop for every record.
On a 1M-line file this compiles the regex 1M times, making regex filtering
10–100× slower than it should be.

**Affected locations (3 call sites):**
- `src/query/fast/eval.rs:107` — `eval_regex()`
- `src/query/fast/eval.rs:118` — `eval_glob()`
- `src/query/dsl/eval.rs:296` — `compare_regex()`

**Implementation steps:**

1. **Fast layer — `src/query/fast/parser.rs`**
   - Add `use std::sync::Arc; use regex::Regex;` at top.
   - Add field `pub compiled: Option<Arc<Regex>>` to `FilterExpr`.
   - In `parse_filter()`, after recognizing `FilterOp::Regex`, call
     `Regex::new(&value)` (return `QkError::Query` on compile failure) and store
     in `compiled`.
   - For `FilterOp::Glob`, call `glob_to_regex(&value)` (move the function here
     from `eval.rs`), then compile and store in `compiled`.

2. **Fast layer — `src/query/fast/eval.rs`**
   - Change `eval_regex(val, pattern: &str)` →
     `eval_regex(val, re: &Regex) -> Result<bool>`.
   - Change `eval_glob(val, pattern: &str)` →
     `eval_glob(val, re: &Regex) -> Result<bool>`.
   - In `eval_filter()`, call these with `f.compiled.as_deref().unwrap()`
     (the regex is guaranteed compiled when `op` is `Regex`/`Glob`).
   - Delete `glob_to_regex()` from this file (moved to `parser.rs`).

3. **DSL layer — `src/query/dsl/ast.rs`**
   - Add `Regex(Arc<Regex>, String)` variant to `Literal` enum.
     The `String` holds the original source pattern for error messages / debug.
   - `Arc<Regex>` does not need `#[derive(Debug)]`; add a manual `Debug` impl
     that prints `Regex(<pattern>)`.

4. **DSL layer — `src/query/dsl/parser.rs`**
   - In the `matches` operator branch, compile `Regex::new(&pattern)` immediately.
   - On compile failure, return a `nom` custom error that includes the bad pattern.
   - Store as `Literal::Regex(Arc::new(re), pattern)`.

5. **DSL layer — `src/query/dsl/eval.rs`**
   - Change `compare_regex(haystack, pattern: &str)` →
     `compare_regex(haystack, re: &Regex) -> bool`.
   - In the `CmpOp::Matches` branch, match on `Literal::Regex(re, _)` and pass
     `re` directly.

6. **Add regression test** in `tests/fast_layer.rs`:
   a query with `msg~=.*timeout.*` on a 10-record fixture; verify it returns
   the expected count. This would have caught LL-008.

**Files to modify:** `src/query/fast/parser.rs`, `src/query/fast/eval.rs`,
`src/query/dsl/ast.rs`, `src/query/dsl/parser.rs`, `src/query/dsl/eval.rs`

---

### T-02 · Remove Broken `tail -f` Examples from Documentation

**Priority:** P0
**Complexity:** S
**Why:** `COMMANDS.md` and `TUTORIAL.md` both show `tail -f app.log | qk …`.
`read_to_string` blocks until EOF; `tail -f` never closes the pipe.
Every user who tries this command will be left with a hung terminal.

**Implementation steps:**

1. In `COMMANDS.md`, search for every `tail -f` occurrence.
   Replace with a comment block:
   ```
   # NOTE: tail -f is not yet supported. qk reads stdin to EOF.
   # To filter a growing log in real time, use: tail -n 1000 app.log | qk …
   ```

2. In `TUTORIAL.md`, same replacement pattern.

3. In `COMMANDS_CN.md` and `TUTORIAL_CN.md`, apply the same fix in Chinese.

4. Add a prominent "Known Limitations" section at the top of `README.md` and
   `README_CN.md`:
   ```
   ## Known Limitations
   - **No streaming / tail -f support yet**: qk reads stdin to EOF before
     processing. `tail -f file | qk …` will block. Workaround: use
     `tail -n 1000 file | qk …` for finite input.
   ```

**Files to modify:** `COMMANDS.md`, `TUTORIAL.md`, `README.md`,
`COMMANDS_CN.md`, `TUTORIAL_CN.md`, `README_CN.md`

---

### T-03 · Set Up GitHub Actions CI/CD

**Priority:** P1
**Complexity:** S
**Why:** No automated test gate means broken changes can be pushed silently.
This is the single biggest barrier to external contributors.

**Implementation steps:**

1. Create `.github/workflows/ci.yml`:
   ```yaml
   name: CI
   on: [push, pull_request]
   jobs:
     test:
       runs-on: ubuntu-latest
       steps:
         - uses: actions/checkout@v4
         - uses: dtolnay/rust-toolchain@stable
         - run: cargo test --all
         - run: cargo clippy -- -D warnings
         - run: cargo fmt --check
   ```

2. Create `.github/workflows/release.yml`:
   - Trigger on `push: tags: ['v*']`
   - Use `cross` (or GitHub matrix) to build for:
     `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`,
     `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`
   - Upload binaries as GitHub Release assets using `softprops/action-gh-release`

3. Create `CONTRIBUTING.md` with:
   - "Run `cargo test && cargo clippy -- -D warnings && cargo fmt` before
     every commit"
   - PR template: what to include, how to add tests, how to update STRUCTURE.md

4. Add CI badge to `README.md` top.

**Files to create:** `.github/workflows/ci.yml`,
`.github/workflows/release.yml`, `CONTRIBUTING.md`
**Files to modify:** `README.md`

---

## Phase 2 — Core Architecture Changes (2–6 weeks)

### T-04 · Streaming Execution Engine

**Priority:** P0 (blocks real-world adoption)
**Complexity:** L
**Why:** `read_to_string` materializes the entire input before eval.
This makes `tail -f` impossible, limits file sizes to available RAM,
and prevents any form of real-time output.

This is the largest architectural change in the roadmap.
It must be designed carefully to preserve rayon file-level parallelism.

**Design: three-tier streaming model**

```
Tier 1 — Source: yields records one at a time
  ├── File source: parser returns impl Iterator<Item = Result<Record>>
  └── Stdin source: line-by-line reading (BufReader::lines())

Tier 2 — Eval pipeline: processes records lazily
  ├── Filter stage: streaming (pass/drop per record)
  ├── Limit/head stage: streaming with early termination
  ├── Count/Sum/Avg/Min/Max: streaming (accumulate, emit one record at end)
  └── Sort/GroupBy/Dedup: must buffer (inherently requires all records)

Tier 3 — Output: streams records to writer as they arrive
```

**Implementation steps:**

1. **Redesign parser interface** (`src/parser/mod.rs`):
   Change `parse(…) -> Result<Vec<Record>>` to
   `parse_iter(…) -> Box<dyn Iterator<Item = Result<Record>> + Send>`.
   Each parser (`ndjson.rs`, `csv.rs`, etc.) implements an iterator variant.
   Keep the current `parse()` as a convenience wrapper: `parse_iter().collect()`.

2. **Redesign stdin reading** (`src/main.rs`):
   Replace `read_to_string` with `BufReader::new(io::stdin()).lines()`.
   Each line is parsed as NDJSON (the only format that is line-oriented on
   stdin). Detect format from first line only.
   This immediately unblocks `tail -f` for NDJSON streams.

3. **Redesign eval pipeline** (`src/query/fast/eval.rs`,
   `src/query/dsl/eval.rs`):
   - Add a `StreamingContext` enum: `Buffering` (sort/dedup/group_by required)
     vs `Streaming` (filter-only or accumulate-only queries).
   - For streaming queries: pipe records through `filter → limit → output`
     without intermediate `Vec`.
   - For buffering queries: collect into `Vec` only when a `SortBy`, `Dedup`,
     or `GroupBy` stage is present.

4. **Streaming output** (`src/output/mod.rs`):
   Change `render(records: &[Record], …)` to
   `render_iter(iter: impl Iterator<Item = &Record>, …)`.
   This allows table/ndjson/csv output to write rows as records arrive.
   Exception: `--fmt table` needs column width pre-computation — buffer only
   column headers, not all records (use fixed 60-char truncation already in
   place).

5. **File-level parallelism** (keep rayon):
   When processing multiple files, rayon `par_iter` over files still applies.
   Each file is streamed independently. Results are merged in order using
   `rayon::iter::chain()` or a bounded channel.

**Key invariant to preserve:** Single-file queries become fully streaming.
Multi-file queries with sort/group_by may still buffer one file at a time.

**Files to modify:** `src/main.rs`, `src/parser/mod.rs`,
`src/parser/ndjson.rs`, `src/parser/csv.rs`, `src/parser/logfmt.rs`,
`src/parser/yaml.rs`, `src/parser/toml_fmt.rs`, `src/parser/plaintext.rs`,
`src/query/fast/eval.rs`, `src/query/dsl/eval.rs`, `src/output/mod.rs`

---

### T-05 · String Interning for Field Names

**Priority:** P2
**Complexity:** M
**Why:** Each `Record` allocates a fresh `String` for field names like
`"level"`, `"msg"`, `"ts"` — identical across millions of records.
For large log files with many records, this causes significant heap fragmentation.

**Implementation steps:**

1. Add `string-interner` or `lasso` crate to `Cargo.toml`.

2. Create `src/util/intern.rs` with a module-level `Interner`:
   ```rust
   use std::sync::OnceLock;
   use lasso::{Rodeo, Spur};
   static INTERNER: OnceLock<Mutex<Rodeo>> = OnceLock::new();
   pub fn intern(s: &str) -> Spur { … }
   pub fn resolve(k: Spur) -> &'static str { … }
   ```

3. Change `IndexMap<String, Value>` in `record.rs` to `IndexMap<Spur, Value>`.

4. Update all parsers to call `intern(field_name)` when building records.

5. Update all query eval code that does `rec.get("field")` to use
   `intern("field")` for the lookup key.

**Note:** Do NOT implement this before T-04. The streaming engine refactor
will touch all the same code paths, and doing both at once creates merge hell.

**Files to modify:** `src/record.rs`, `src/parser/*.rs`,
`src/query/fast/eval.rs`, `src/query/dsl/eval.rs`, `Cargo.toml`

---

## Phase 3 — Improvements & Polish (1–3 months)

### T-06 · Distribution: Install Script + Homebrew Tap

**Priority:** P1
**Complexity:** M
**Why:** `cargo install --path .` is a non-starter for non-Rust users.
The primary adoption barrier for ops/SRE users who don't have a Rust toolchain.

**Implementation steps:**

1. **Install script** (`install.sh`):
   ```bash
   #!/usr/bin/env bash
   # Detect OS/arch, download the correct binary from GitHub Releases,
   # place in /usr/local/bin/qk, chmod +x
   ```
   Follow the pattern of `fd`, `ripgrep`, `bat` install scripts.

2. **Homebrew Tap** (separate repo: `homebrew-qk`):
   - Create `Formula/qk.rb` using `brew create` template.
   - Point to GitHub Releases `.tar.gz` artifacts (produced by T-03 release CI).
   - Register tap: `brew tap <org>/qk && brew install qk`

3. **Update README.md** installation section:
   ```
   ## Install
   # Homebrew (macOS / Linux)
   brew install <org>/qk/qk

   # One-line script (Linux / macOS)
   curl -fsSL https://raw.githubusercontent.com/<org>/qk/main/install.sh | bash

   # From source
   cargo install --git https://github.com/<org>/qk
   ```

**Files to create:** `install.sh`, `homebrew-qk/Formula/qk.rb`
**Files to modify:** `README.md`, `README_CN.md`

---

### T-07 · Error Messages with Source Location

**Priority:** P2
**Complexity:** M
**Why:** Syntax errors in the fast keyword layer return a plain string
with no indication of which token caused the failure.
Power users debugging complex queries lose time without position hints.

**Implementation steps:**

1. Add a `span: (usize, usize)` (byte offset range) to `FilterExpr`.
   Set it during parse when each token is consumed.

2. In `QkError::Query`, include the original query string + a `^` pointer:
   ```
   parse error at position 12: unexpected token 'gte'
   where level=error gte 5 msg~=fail
                     ^^^
   ```

3. For DSL layer, `nom` already tracks positions — propagate them through to
   the final error message.

**Files to modify:** `src/query/fast/parser.rs`, `src/util/error.rs`,
`src/query/dsl/parser.rs`

---

### T-08 · Time-Series Bucketing

**Priority:** P2
**Complexity:** L
**Why:** The #1 use case for log analysis is "how many errors per minute?"
Currently requires piping to `awk` or Python, defeating qk's purpose.

**Implementation steps:**

1. Add `group_by_time` stage to both fast layer and DSL:
   - Fast: `count by 1m`, `count by 5m`, `count by 1h`
   - DSL: `| group_by_time(.ts, "5m")`

2. Add timestamp auto-detection to `record.rs` `get()`:
   When a field value matches common timestamp formats (RFC 3339, Unix epoch,
   epoch-ms), parse it to `chrono::DateTime<Utc>`.

3. New dependency: `chrono = "0.4"` in `Cargo.toml`.

4. Bucketing logic: `floor(timestamp / bucket_size) * bucket_size` →
   emit `{"bucket": "2024-01-15T10:05:00Z", "count": 42}`.

5. Fixture: add time-bucketing examples to `tutorial/app.log` and
   document in `COMMANDS.md` and `TUTORIAL.md`.

**Files to create:** `src/util/time.rs`
**Files to modify:** `src/query/fast/parser.rs`, `src/query/fast/eval.rs`,
`src/query/dsl/ast.rs`, `src/query/dsl/parser.rs`, `src/query/dsl/eval.rs`,
`Cargo.toml`

---

### T-09 · Interactive TUI Mode

**Priority:** P2 (long-term bet)
**Complexity:** L
**Why:** A `k9s`/`lnav`-style interface would make qk the default local log
browser for SREs. Query in real time, scroll results, switch files — all
without leaving the terminal.

**Implementation steps:**

1. Add `ratatui = "0.26"` and `crossterm = "0.27"` to `Cargo.toml`.
2. Gate behind `--ui` flag (not in default binary path).
3. `src/tui/` module:
   - `app.rs` — `App` state: current query string, result records, scroll offset
   - `ui.rs` — `ratatui` layout: top query input, middle record list, bottom status bar
   - `events.rs` — `crossterm` event loop: typing updates query, debounce 100ms,
     re-run eval, refresh display
4. Re-use existing parser + eval pipeline unchanged (TUI is purely a new
   frontend).
5. Streaming engine (T-04) is a prerequisite: TUI needs incremental results.

**Files to create:** `src/tui/app.rs`, `src/tui/ui.rs`, `src/tui/events.rs`,
`src/tui/mod.rs`
**Files to modify:** `src/main.rs`, `src/cli.rs`, `Cargo.toml`

---

## Top 5 Tasks to Start (in order)

| # | Task | Priority | Complexity | Rationale |
|---|------|----------|-----------|-----------|
| 1 | T-01 — Regex recompilation fix | P0 | S | Real correctness bug, ~30 lines, no risk |
| 2 | T-02 — Remove broken tail -f docs | P0 | S | Stops misleading users today, 15 min |
| 3 | T-03 — GitHub Actions CI/CD | P1 | S | Enables safe iteration; required before external PRs |
| 4 | T-04 — Streaming execution engine | P0 | L | Core architectural fix; unlocks tail -f and large files |
| 5 | T-06 — Distribution (install script + Homebrew) | P1 | M | Adoption gate for non-Rust users |

---

## If You Only Have 3 Days

**Day 1 — Morning (2h):** Implement T-01 (regex caching).
Confirm with `cargo test` (all 206 tests pass).
Confirm with `cargo clippy -- -D warnings`.

**Day 1 — Afternoon (1h):** Implement T-02 (documentation fixes).
Update all 6 files. Verify no remaining `tail -f` examples that lack caveats.

**Day 2 (4h):** Implement T-03 (CI/CD).
Push `.github/workflows/ci.yml` with test + clippy + fmt gates.
Add a draft release workflow. Verify the workflow passes in GitHub Actions.
Write `CONTRIBUTING.md`.

**Day 3 (full day):** Begin T-04, focusing on the stdin streaming fix first
(the minimal slice that unblocks `tail -f`):
- Replace `read_to_string` with `BufReader::lines()` in `read_stdin()`.
- NDJSON line-by-line processing through a lazy iterator.
- Wire into eval without collecting to `Vec`.
- Test: `yes '{"level":"error"}' | head -n 1000000 | qk where level=error count`
  should complete with constant memory instead of OOM.

The full streaming engine for file inputs and non-NDJSON formats is a
follow-on milestone after Day 3.

---

## Dependency Map

```
T-01 (regex) ──────────────────► can ship immediately
T-02 (docs) ───────────────────► can ship immediately
T-03 (CI) ─────────────────────► can ship immediately
T-04 (streaming) ──────────────► blocks T-09 (TUI)
T-05 (interning) ──────────────► depends on T-04 (same files)
T-06 (distribution) ───────────► depends on T-03 (release CI)
T-07 (error messages) ─────────► independent
T-08 (time bucketing) ─────────► independent; depends on T-04 for streaming support
T-09 (TUI) ────────────────────► depends on T-04 (streaming)
```

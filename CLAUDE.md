# CLAUDE.md — Claude Code Instructions

This file is automatically read by Claude Code at the start of every session. Follow all rules below without being prompted.

---

## Project Identity

- **Tool name**: `qk`
- **Language**: Rust (stable toolchain, minimum 1.75)
- **Purpose**: Single CLI tool replacing grep / awk / sed / jq / yq / cut / sort+uniq
- **Architecture**: Input → Format Detector → Parser → Record IR → Query Engine → Output Renderer

---

## Mandatory Rules Per Session

### 1. Always update PROGRESS.md
After every meaningful change (new file, new function, bug fix, refactor), add an entry to `PROGRESS.md`.
Format:
```
## YYYY-MM-DD — short description
### Added / Modified / Deleted
- bullet points
```

### 2. Update LESSON_LEARNED.md for every non-trivial bug
When encountering compile errors, logic bugs, surprising Rust behavior, or anything that took multiple attempts to solve — record it in `LESSON_LEARNED.md` using the LL-NNN format.

### 3. Always update STRUCTURE.md when files change
When creating, renaming, or significantly changing a file's responsibilities, update the tree and description table in `STRUCTURE.md`. Do not let STRUCTURE.md drift from the actual code.

### 4. Keep functions short
Each function should be at most ~40 lines. Split when longer.

### 5. No unwrap() in library code
Use `?` propagation or explicit error handling. `unwrap()` is only allowed in:
- The top level of `main.rs`
- Test code

### 6. Error messages must be actionable
When returning errors, include enough context so the user knows which file/line/field caused it.
**Bad**: `Err("parse error")`
**Good**: `Err(format!("failed to parse field '{}' at line {}: {}", field, line_num, e))`

### 7. Every parser must have tests
Each format parser in `src/parser/` must have at least one in-file unit test and an integration test in `tests/formats.rs`.

### 8. Benchmark before optimizing
Before making any performance claims, run `cargo bench` and record the numbers in `PROGRESS.md`.

---

## Code Style

- **Comments**: English (identifiers and doc comments, i.e. `///`)
- **Formatting**: Run `cargo fmt` before every commit, no exceptions
- **Lint**: `cargo clippy -- -D warnings` must pass with zero reports
- **Doc comments**: Add `///` to every public function and struct
- **Naming**: Follow Rust conventions (functions: snake_case, types: CamelCase, constants: SCREAMING_SNAKE)

---

## Documentation Language

All markdown documents (README.md, TUTORIAL.md, STRUCTURE.md, PROGRESS.md, LESSON_LEARNED.md, CLAUDE.md) are written in **English**.
Chinese versions are saved as `<filename>_CN.md`.
Comments and identifiers in code remain in **English**.

---

## Architecture Constraints

- The `Record` type in `record.rs` is the **only** type that crosses the parser→query boundary. Parsers must not leak format-specific types to the query layer.
- The query engine (`query/`) must not directly import `parser/`.
- Performance-critical paths (NDJSON line splitting, field lookup) must use `memchr`, not `str::find`.
- File-level parallelism uses `rayon::par_iter()`. Do not manually spawn threads.

---

## Workflow for Adding a New Format

1. Add a variant to the `Format` enum in `detect.rs`
2. Add detection logic in `detect::sniff()`
3. Create `src/parser/<format>.rs` with a `parse(input: &str) -> Result<Vec<Record>>` function
4. Register it in `src/parser/mod.rs`
5. Add a fixture file in `tests/fixtures/`
6. Add tests in `tests/formats.rs`
7. Update `STRUCTURE.md` and `PROGRESS.md`

---

## Current Phase

**Phases 1–9 — All Complete ✅**

Implemented features:
- Auto format detection (NDJSON / JSON / CSV / TSV / logfmt / YAML / TOML / Gzip / plaintext)
- NDJSON, logfmt, CSV, YAML, TOML, plaintext parsers; transparent gzip decompression
- Fast query layer: where / select / count / count by / sort / limit / head / fields / sum / avg / min / max (with `and/or/not/exists/contains/regex/startswith/endswith/glob`)
- DSL expression layer: `.field == val | pick() | omit() | count() | sort_by() | group_by() | limit() | skip() | dedup() | sum() | avg() | min() | max()`
- Nested field dot-notation access (`response.status`)
- Piping (stdin auto-detected as NDJSON)
- Output formats: ndjson (default) / pretty (indented JSON) / table (comfy-table colored) / csv / raw
- `--fmt` / `--color` / `--no-color` / `--explain` / `--cast` / `--no-header` flags
- rayon file-level parallelism, mmap large-file optimization (≥ 64 KiB)
- Semantically-aware ANSI color output: error=red, warn=yellow, info=green, msg=bright white, ts=dim, HTTP status codes colored by range
- Type-mismatch warnings on stderr for numeric aggregations; null-like strings silently skipped
- **206 tests all passing** (138 unit + 68 integration)
- `cargo clippy -- -D warnings` zero reports

**Known limitations (see ROADMAP.md for fix plans):**
- `tail -f file | qk …` will hang — stdin uses `read_to_string`, which blocks until EOF (T-04)
- Regex patterns are recompiled per record in the eval hot loop (T-01, fix is next)
- Full file materialization before eval: large files (>1 GB) may OOM (T-04)

**Important usage notes:**
- `--fmt`, `--color`, `--cast` and other flags must come **before** the query expression (clap `trailing_var_arg` semantics)
- DSL mode triggers when first argument starts with `.`, `not `, or `|`
- TOML files always output 1 record (entire document as one object)
- Color priority: `--no-color` > `--color` > `NO_COLOR` env > tty auto-detection

**Next tasks:** See `ROADMAP.md` — start with T-01 (regex recompilation fix).

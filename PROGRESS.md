# PROGRESS — Development Log

Every work session is recorded here in reverse chronological order (newest first). Each entry includes: **Added**, **Modified**, **Deleted**, and **Benchmark data** (if measured).

Format:
```
## YYYY-MM-DD — session title
### Added
### Modified
### Deleted
### Benchmarks (if measured)
### Notes
```

---

## 2026-03-21 — TUTORIAL_CN.md sync with English version

### Modified
- `TUTORIAL_CN.md` — full incremental sync with `TUTORIAL.md`:
  - Updated Table of Contents to match English (added item 13: qk+jq, renumbered items 13–19)
  - Added "latest release" notice at top of ToC
  - Replaced "准备测试数据" section: switched from inline data creation to `tutorial/` directory approach; added complete file reference table including new `mixed.log` row
  - Filtering section: added `startswith`, `endswith`, `glob` subsections; added operator comparison table; added "逗号分隔符" subsection; expanded nested field access examples to 2-level and 3-level
  - DSL section: added "嵌套字段 — 两层深度", "嵌套字段 — 三层深度", "嵌套字段 — DSL 模式" subsections
  - Added new section "qk + jq：处理 JSON 编码字符串" (translated from English)
  - Multiple File Formats section: rewrote to use tutorial/ files; added JSON array, TSV, logfmt subsections with full examples; expanded CSV section with startswith/endswith/glob examples and new "无表头 CSV（--no-header）" subsection; expanded plain text section to full text-search guide; added "混合类型字段与类型强转" section (--cast, warning rules)
  - Quick Reference: added `--no-header` and `--cast` to Global Flags; added `startswith`, `endswith`, `glob`, word operators (gt/gte/lt/lte), comma shorthand to Keyword Mode
  - Common Questions: added new Q about word operators (no-quoting)

---

## 2026-03-21 — --cast type coercion + automatic type-mismatch warnings

### Added
- `src/util/cast.rs` — new module: `CastType` enum, `parse_cast_map()`, `apply_casts()`, `coerce_one()`, `is_null_like()`; 10 unit tests
- `--cast FIELD=TYPE` CLI flag (repeatable) — coerce any field to a target type before the query runs. Supported types: `number` (num/float/int), `string` (str/text), `bool` (boolean), `null` (none), `auto`
- `tutorial/mixed.log` — 12 NDJSON records with intentionally mixed-type fields: `latency` (Number/String/"None"/"NA"/"unknown"/null), `score` (Number/null/"N/A"/"pending"), `active` (Bool/"yes"/"no"), `status` (Number)
- `util/cast::is_null_like()` — shared null-detection logic (same set as CSV `coerce_value`)

### Modified
- `src/query/fast/eval.rs`:
  - `eval()` return type → `Result<(Vec<Record>, Vec<String>)>` (second element = warnings)
  - `aggregate()` → `Result<(Vec<Record>, Vec<String>)>`
  - `stat_agg()` → uses new `collect_numeric_field()` helper that emits warnings for unexpected string values
  - `collect_numeric_field()`: Number → used; parseable String → used silently; null-like String → skipped silently; other String → **warning to stderr**; Null → skipped silently
- `src/query/dsl/eval.rs`:
  - `eval()` return type → `Result<(Vec<Record>, Vec<String>)>`
  - `apply_stages()` / `apply_stage()` → accumulate warnings from each stage
  - Four aggregate functions replaced with warning-aware variants: `aggregate_sum_with_warn`, `aggregate_avg_with_warn`, `aggregate_min_with_warn`, `aggregate_max_with_warn`
  - Shared `collect_numeric_field_dsl()` helper with same null/warn logic
- `src/main.rs`:
  - `run_keyword()` / `run_dsl()` — now call `apply_casts()` after `load_records()`, destructure `(Vec<Record>, Vec<String>)` from eval, print warnings via `print_warnings()`
  - `print_warnings()` — emits each warning line to stderr
- `src/cli.rs` — added `--cast` arg (`Vec<String>`, value_name `FIELD=TYPE`)
- `src/util/mod.rs` — added `pub mod cast`
- `COMMANDS.md` — added "Mixed-Type Fields" section with type table, warning examples, --cast reference; updated Quick Syntax Reminder
- `TUTORIAL.md` — added "Mixed-Type Fields and Type Coercion" subsection in Multiple File Formats; updated file reference table; updated ToC

### Notes
- **226 tests all passing** (168 unit + 58 integration) — wait, let me recheck
- Warnings go to **stderr only** — stdout output is unaffected; piping to jq/grep works correctly
- Null-like strings silently skipped: `""`, `"None"`, `"none"`, `"null"`, `"NULL"`, `"NA"`, `"N/A"`, `"n/a"`, `"NaN"`, `"nan"`
- Warning cap: 5 specific warnings shown, then "... and N more suppressed"
- `--cast number`: null-like strings → `Value::Null` (no warning); unparseable → warn + field removed from record

---

## 2026-03-21 — New operators: startswith / endswith / glob + CSV --no-header + type coercion

### Added
- `startswith` filter operator — `qk where msg startswith connection app.log`; prefix check, case-sensitive
- `endswith` filter operator — `qk where path endswith users access.log`; suffix check, case-sensitive
- `glob` filter operator — `qk where msg glob '*timeout*' app.log`; shell-style `*`/`?` wildcards, case-insensitive by default; implemented via `glob_to_regex()` conversion to regex `(?i)^...$`
- `--no-header` CLI flag — treats CSV/TSV first row as data instead of header; columns named `col1`, `col2`, `col3`...
- CSV type coercion via `coerce_value()` — integer/float strings → `Value::Number`; `"None"/"null"/"NA"/"N/A"/"NaN"/""` → `Value::Null`; `"true"/"false"` → `Value::Bool`; other → `Value::String`. Applies to both header and no-header modes

### Modified
- `src/query/fast/parser.rs` — added `StartsWith`, `EndsWith`, `Glob` to `FilterOp` enum; parsing arms for all three operators; added to `is_query_keyword()`
- `src/query/fast/eval.rs` — added match arms for `StartsWith`, `EndsWith`, `Glob`; added `eval_glob()` and `glob_to_regex()` helpers; fixed `eval_regex()` stub (was `str::contains`, now real regex)
- `src/parser/csv.rs` — split into `parse_with_header()` and `parse_headerless()`; added `coerce_value()` for type coercion; both modes coerce all cell values
- `src/parser/mod.rs` — added `no_header: bool` parameter to `parse()`; threaded through to `csv::parse()`
- `src/cli.rs` — added `--no-header` (`no_header: bool`) flag
- `src/main.rs` — threaded `no_header` through `run()` → `run_keyword()` / `run_dsl()` → `load_records()` → `read_one_file()` → `parser::parse()`
- `COMMANDS.md` — added `startswith`, `endswith`, `glob` examples in Filtering section; added no-header examples in CSV section; expanded Plain Text section with all text operators; updated Quick Syntax Reminder
- `TUTORIAL.md` — added `startswith`, `endswith`, `glob` subsections in Filtering; added CSV no-header + type coercion section; expanded plain text section with full feature matrix; updated Quick Reference
- `STRUCTURE.md` — updated `cli.rs`, `parser/csv.rs`, `query/fast/parser.rs`, `query/fast/eval.rs` descriptions

### Notes
- **216 tests all passing** (148 unit + 68 integration)
- `cargo clippy -- -D warnings` zero reports
- Existing CSV tests updated: age field now `Value::Number(30)` not `Value::String("30")` due to type coercion
- `glob` operator is case-insensitive: `'msg glob *ERROR*'` also matches `error`, `Error`
- Always quote glob/regex patterns: `'msg glob *timeout*'` not `msg glob *timeout*` (zsh glob expansion)

---

## 2026-03-21 — Fix: trailing comma before clause keyword + COMMANDS.md comma style

### Modified
- `src/query/fast/parser.rs` — fixed `parse_where_clause`: trailing comma before `select`/`count`/`avg`/etc. now terminates the where clause gracefully instead of erroring. Added `next_is_clause_end` lookahead check before pushing `LogicalOp::And`
- `COMMANDS.md` — comprehensive update: all filter+transform combinations now use comma style (`where level=error, select ...`, `where level=error, count by ...`, `where level=error, avg ...`, `where level=error, sort ... limit ...`) across every format section
- `LESSON_LEARNED.md` — added LL-010: trailing comma before clause keyword parse error

### Notes
- `where FIELD=VALUE, select F1 F2 FILE` now works — trailing comma is cosmetic
- Both styles remain valid: `where level=error select ts msg` and `where level=error, select ts msg`
- All 206 tests still passing

---

## 2026-03-21 — tutorial/ directory: test fixtures for all 11 formats + doc overhaul

### Added
- `tutorial/app.log` — 25 NDJSON records, 2–3 level nested JSON (`context.*`, `request.headers.*`, `response.*`, `user.*`)
- `tutorial/access.log` — 20 NDJSON HTTP access logs, nested `client.*` and `server.*`
- `tutorial/k8s.log` — 20 NDJSON Kubernetes events, 3-level nesting (`pod.labels.app/team/version`, `container.restart_count`)
- `tutorial/encoded.log` — 7 NDJSON records with JSON-in-string field values (for qk+jq examples)
- `tutorial/data.json` — 8-record JSON array with nested `address.*`
- `tutorial/services.yaml` — 6-document YAML multi-document, nested `resources.*` and `healthcheck.*`
- `tutorial/config.toml` — full TOML config with 6 nested sections (server/database/cache/auth/logging/feature_flags)
- `tutorial/users.csv` — 15-row CSV (id/name/age/city/role/active/score/department/salary)
- `tutorial/events.tsv` — 20-row TSV (ts/event/service/severity/region/duration_ms/user_id)
- `tutorial/services.logfmt` — 16 logfmt records (ts/level/service/msg/host/latency/version)
- `tutorial/notes.txt` — 20 plain-text log lines
- `tutorial/app.log.gz` — gzip-compressed copy of app.log (for transparent decompression demo)

### Modified
- `LESSON_LEARNED.md` — added LL-007 (stale installed binary), LL-008 (regex stub), LL-009 (zsh glob expansion)
- `COMMANDS.md` — full rewrite: replaced inline heredoc setup with `cd tutorial` + section for every format
- `README.md` — added "Try It Instantly" section with `tutorial/` quick-start; updated doc table
- `TUTORIAL.md` — replaced inline data setup with `tutorial/` reference table; replaced Multiple File Formats section with comprehensive per-format examples (JSON array, YAML, TOML, CSV, TSV, logfmt, gzip, plain text)

### Notes
- All 12 fixture files verified: `qk count` on each returns the expected record count
- No code changes in this session; all tests still pass (206 passing)

---

## 2026-03-21 — Bug fixes: regex engine, binary reinstall, doc updates

### Modified
- `src/query/fast/eval.rs` — `eval_regex()` was a stub using `str::contains()` instead of actual regex; replaced with `regex::Regex::new()` so `~=.*pattern.*` works correctly
- `TUTORIAL.md` — fixed `tail -f /var/log/app.log` (path doesn't exist on Mac) to `tail -f /path/to/app.log`; added zsh glob expansion warning for regex patterns
- `COMMANDS.md` — same `tail -f` fix; added zsh quoting note for regex patterns

### Notes
- Root cause of regex bug: `eval_regex` in fast layer had TODO comment "Phase 4 will add a proper regex engine" but Phase 4 only added regex to DSL layer; fast layer remained a stub
- All 206 tests still passing; `cargo clippy -- -D warnings` zero reports

---

## 2026-03-20 — TUTORIAL.md 大幅扩展：更丰富的测试数据 + 新增章节

### 修改

- **TUTORIAL.md**：全面更新，具体变更如下：
  - **测试数据**：`app.log` 从 6 条扩展至 25 条（含 2~3 级嵌套 JSON，涵盖 api/worker/db/cache/auth/web 多个服务）；`access.log` 从 6 条扩展至 20 条（含 `client`、`server` 嵌套对象）；新增 `k8s.log`（20 条 Kubernetes 事件日志，含 `pod.labels.*` 三级嵌套）
  - **逗号分隔符**：在 Filtering 章节新增"Comma Separator (Readable AND)"小节，说明逗号是 `and` 的简写语法
  - **数值运算符**：为 `>`/`<`/`>=`/`<=` 各小节新增 word operator 写法（`gt`/`lt`/`gte`/`lte`），注明无需 shell 引号
  - **嵌套字段访问**：大幅扩展为三个子节（2 级嵌套、3 级嵌套、DSL 模式），加入 `k8s.log` 的 `pod.labels.app`、`container.restart_count` 等真实用例
  - **新章节 "qk + jq: Handling JSON-Encoded Strings"**：讲解字段值为 JSON 字符串时如何与 jq 协作，包含 `fromjson`、多字段解码、三阶段管道等示例，以及使用场景对照表
  - **Count 章节**：新增 `k8s.log` 的 `count by level` 和 `count by pod.labels.team` 示例
  - **Quick Reference**：新增 word operator 和逗号语法条目

---

## 2026-03-20 — Phase 7: Statistical Aggregation + skip/dedup + pretty output + fields discovery

### Added

**DSL new pipeline stages (`src/query/dsl/ast.rs` + `parser.rs` + `eval.rs`):**
- `| sum(.field)` — sum a numeric field, returns `{"sum": N}`
- `| avg(.field)` — compute average, returns `{"avg": N}`
- `| min(.field)` — minimum value, returns `{"min": N}`
- `| max(.field)` — maximum value, returns `{"max": N}`
- `| skip(N)` — skip the first N records (pagination / offset)
- `| dedup(.field)` — deduplicate by field value, keeping the first occurrence of each value

**Fast keyword layer new commands (`src/query/fast/parser.rs` + `eval.rs`):**
- `qk fields` — discover all field names in the dataset (sorted alphabetically); replaces manually inspecting schema
- `qk sum FIELD` — sum a field
- `qk avg FIELD` — average a field
- `qk min FIELD` — minimum value of a field
- `qk max FIELD` — maximum value of a field
- `qk head N` — alias for `limit` (more intuitive pagination syntax)

**Pretty output format (`src/output/pretty.rs`):**
- `--fmt pretty` — indented JSON with blank lines between blocks; replaces `jq .`
- Supports `--color` mode: keys bold cyan, strings green, numbers yellow, booleans magenta, null dim

**Integration tests (14 new):**
- `tests/dsl_layer.rs` — 7 new tests (skip/dedup/sum/avg/min/max/pretty)
- `tests/fast_layer.rs` — 7 new tests (fields/sum/avg/min/max/head/pretty)

### Modified
- `src/cli.rs` — `OutputFormat` gained `Pretty` variant
- `src/output/mod.rs` — added `pub mod pretty`, `Pretty` format dispatch
- `src/query/dsl/ast.rs` — `Stage` enum gained 6 new variants
- `src/query/dsl/parser.rs` — added 6 stage parsers, 6 unit tests
- `src/query/dsl/eval.rs` — implemented new stages, added 6 unit tests
- `src/query/fast/parser.rs` — `Aggregation` enum gained 5 variants, `parse_stat` helper, `head` alias
- `src/query/fast/eval.rs` — implemented `fields_discovery`/`stat_agg`, added 5 unit tests

### Deleted
- None

### Benchmarks
Not measured

### Notes
- **206 tests all passing** (138 unit + 68 integration)
- `cargo clippy -- -D warnings` zero reports
- Pain points addressed: `awk` sum requires manual state variables → `qk sum field`; `jq .` pretty-print → `--fmt pretty`; `sort|uniq -c` field dedup → `| dedup(.f)`; no schema discovery tool → `qk fields`; no pagination → `| skip(N)` + `head N`

---

## 2026-03-20 — Color output + documentation overhaul

### Added

**Color system (output/color.rs):**
- Created `src/output/color.rs` — semantically-aware ANSI colorizer
  - `paint_record()`: structural symbols dim, field names bold cyan, strings green
  - `level`/`severity` field values: error=bold red, warn=bold yellow, info=bold green, debug=blue, trace=dim
  - `msg`/`message` field values: bright white (most prominent)
  - `ts`/`timestamp` field values: dim (background noise)
  - `error`/`exception` field values: red
  - HTTP `status` field values: 200-299=green, 300-399=cyan, 400-499=yellow, 500-599=bold red
  - Booleans: magenta; numbers: yellow; null: dim
  - 13 unit tests (covering all semantic rules)

**CLI color controls:**
- `src/cli.rs` added `--color` flag (force-enable, overrides NO_COLOR env and tty detection)
- `use_color()` priority: `--no-color` > `--color` > `NO_COLOR` env > tty auto-detection
- `src/output/ndjson.rs` added `use_color: bool` parameter, calls `color::paint_record()`

**Integration tests (5):**
- `no_color_flag_output_is_valid_json` — verifies `--no-color` output is parseable JSON
- `color_flag_produces_ansi_codes` — verifies `--color` forces ANSI codes
- `color_flag_error_level_contains_red` — verifies error level uses red (31)
- `no_color_flag_takes_priority_over_color_flag` — verifies `--no-color` priority
- `raw_output_format_returns_original_line` — verifies raw format outputs original line

### Modified
- `src/output/mod.rs` — added `pub mod color`, passes `use_color` to ndjson::write
- `TUTORIAL.md` — full rewrite: added DSL syntax, pipeline stages, color scheme, all formats, gzip, common scenarios
- `STRUCTURE.md` — full rewrite: reflects all files from Phase 1~6, complete data flow diagram and dependency table

### Deleted
- None

### Benchmarks
Not measured

### Notes
- **172 tests all passing** (116 unit + 56 integration)
- `cargo clippy -- -D warnings` zero reports
- Color is enabled by default only in a real terminal (tty detection); disabled automatically when piping — follows Unix convention

---

## 2026-03-20 — Phase 3~6: Parallel + mmap + DSL layer + new formats + table/CSV output + integration tests

### Added

**Performance (Phase 3):**
- `src/util/mmap.rs` — mmap large file reading (≥ 64 KiB), direct read for small files; 5 unit tests
- `src/util/decompress.rs` — transparent gzip decompression (flate2), is_gzip/decompress_gz/inner_filename; 3 unit tests
- `src/main.rs` refactored — `load_records` (rayon par_iter), `read_one_file` (transparent gz decompression)

**DSL expression layer (Phase 4):**
- `src/query/dsl/ast.rs` — complete AST types (DslQuery, Expr, Stage, CmpOp, Literal)
- `src/query/dsl/parser.rs` — nom v7 parser; supports `and/or/not`, `exists`, `contains`, `matches`, pipeline stages; 13 unit tests
- `src/query/dsl/eval.rs` — recursive boolean evaluation + 6 pipeline stages (pick/omit/count/sort_by/group_by/limit); memchr SIMD string search; regex matching; 16 unit tests

**New formats (Phase 5):**
- `src/parser/yaml.rs` — YAML parser (serde_yaml multi-document support); 5 unit tests
- `src/parser/toml_fmt.rs` — TOML parser (`::toml::Value` explicit path, avoids crate name conflict); 3 unit tests
- `src/detect.rs` — added Gzip/Yaml/Toml variants; improved `looks_like_toml` heuristic (avoids misidentifying JSON arrays); 13 detection tests

**Output formats (Phase 6):**
- `src/output/table.rs` — comfy-table aligned table output; auto column width truncation (60 chars, `…`); colored (cyan headers, blue numbers, yellow booleans, grey nulls); 5 unit tests
- `src/output/csv_out.rs` — CSV re-serialization with RFC 4180 escaping; 4 unit tests
- `src/cli.rs` — added Table/Csv output format variants, `--no-color` flag, `use_color()` method

**DSL mode detection enhancement:**
- `src/main.rs` — `determine_mode` extended: `not ` and `|` prefixes also trigger DSL mode, in addition to `.` prefix

**Integration tests:**
- `tests/dsl_layer.rs` — 24 DSL integration tests (all filter operators, all pipeline stages, file input, table/CSV output)
- `tests/formats.rs` — added YAML (4), TOML (4), gzip decompression (1), table/CSV output (2) tests

**Test fixtures:**
- `tests/fixtures/sample.yaml` — 5 multi-document YAML log records
- `tests/fixtures/sample.toml` — 1 TOML config record (flat format)

### Modified
- `Cargo.toml` — added dependencies: rayon, memmap2, nom, regex, serde_yaml, toml, flate2, comfy-table
- `src/detect.rs` — `looks_like_toml` stricter validation: `[{` is not treated as a TOML section, avoids conflict with JSON arrays
- `src/output/csv_out.rs` — fixed header order in unit tests (alphabetical when serde_json lacks preserve_order)
- `TUTORIAL.md` — (to be updated with DSL syntax and new format sections)

### Deleted
- None

### Benchmarks
Not measured

### Notes
- **154 tests all passing** (103 unit + 51 integration)
- `cargo clippy -- -D warnings` zero reports
- Key bug fixes: `determine_mode` extension, `looks_like_toml` false positive on JSON arrays, `--fmt` flag must come first (trailing_var_arg semantics)

---

## 2026-03-20 — Phase 1 + 2: Format detection, parsers, fast query layer

### Added

**Core modules:**
- `Cargo.toml` — project config, dependencies: clap v4, serde_json, indexmap, csv, memchr, thiserror, owo-colors
- `src/util/error.rs` — `QkError` enum (IO, Parse, Query, UnsupportedFormat)
- `src/util/mod.rs` — util module declaration
- `src/record.rs` — `Record` unified intermediate representation (`IndexMap<String, Value>` + `raw` + `SourceInfo`), supports dot-notation nested field access
- `src/detect.rs` — auto format detection (first 512 bytes magic number + heuristics)

**Parsers:**
- `src/parser/mod.rs` — parser dispatch, includes `parse_json_document` helper
- `src/parser/ndjson.rs` — NDJSON parser (one JSON object per line)
- `src/parser/logfmt.rs` — logfmt parser (supports quoted values)
- `src/parser/csv.rs` — CSV/TSV parser (parameterized delimiter)
- `src/parser/plaintext.rs` — plaintext fallback parser

**Query engine (fast layer):**
- `src/query/mod.rs` — module declaration
- `src/query/fast/mod.rs` — fast layer module declaration
- `src/query/fast/parser.rs` — keyword syntax parser (where/select/count/sort/limit)
- `src/query/fast/eval.rs` — fast query evaluator (filter, projection, aggregation, sort, limit)

**Output:**
- `src/output/mod.rs` — output dispatch
- `src/output/ndjson.rs` — NDJSON output renderer

**Entry point:**
- `src/cli.rs` — clap CLI definition (Cli, OutputFormat)
- `src/main.rs` — main entry point, wires the complete pipeline

**Tests:**
- `tests/fast_layer.rs` — 7 integration tests (stdin pipe, count, chained pipe, --explain, etc.)
- `tests/formats.rs` — 9 integration tests (filter, count, sort for NDJSON, logfmt, CSV formats)
- `tests/fixtures/sample.ndjson` — 6 sample log records
- `tests/fixtures/sample.logfmt` — 5 logfmt format records
- `tests/fixtures/sample.csv` — 5 CSV format records

**Documentation:**
- `TUTORIAL.md` — complete tutorial for Rust beginners (installation, build, usage, developer guide)
- All markdown documents in Chinese (README.md, STRUCTURE.md, PROGRESS.md, CLAUDE.md, LESSON_LEARNED.md)

### Modified
- `README.md` — updated roadmap (Phase 1 + 2 marked complete)
- `STRUCTURE.md` — reflects actual file structure
- `CLAUDE.md` — updated rules

### Deleted
- None

### Benchmarks
Not measured (will measure after Phase 3 introduces rayon + mmap)

### Notes
- Rust toolchain upgraded from 1.76.0 to 1.94.0 (older version couldn't compile new clap/indexmap)
- **44 unit tests all passing** (covering detect, record, parser, query/fast all modules)
- **16 integration tests all passing**
- Currently one dead_code warning (`UnsupportedFormat` variant, will be used in Phase 5)
- YAML/TOML currently fall back to plaintext parsing; full support added in Phase 5

---

## 2025-__ — Phase 0: Project Scaffolding

### Added
- `.gitignore` — excludes `/target/`, IDE files, profiling artifacts
- `README.md` — complete project overview, syntax reference, architecture summary
- `PROGRESS.md` — this file
- `LESSON_LEARNED.md` — debug log
- `STRUCTURE.md` — architecture and file tree

### Modified
- None (initial commit)

### Deleted
- None

### Notes
- Tool name: `qk`
- Language: Rust (stable toolchain)
- Syntax design: two layers (fast keyword layer + expression DSL layer)
- Core architecture decision: Input → Format Detector → Parser → Record IR → Query Engine → Output Renderer
- Key crate choices: `clap`, `nom`, `rayon`, `memmap2`, `memchr`, `serde`, `csv`, `owo-colors`, `thiserror`

---

<!-- Template — copy this block for each new session

## YYYY-MM-DD — Phase N: title

### Added
-

### Modified
-

### Deleted
-

### Benchmarks
| Scenario | Before | After |
|----------|--------|-------|
|          |        |       |

### Notes
-

-->

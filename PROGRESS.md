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

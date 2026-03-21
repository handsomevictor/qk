# STRUCTURE — Authoritative Map of the Codebase

This document is the authoritative map of the codebase. It must be kept in sync whenever files are added, moved, or significantly changed.

---

## Current Project Structure (Phases 1–9 Complete)

```
qk/
│
├── Cargo.toml                  # workspace manifest + binary crate configuration
├── Cargo.lock                  # locked dependency versions (must be committed for binaries)
│
├── .gitignore
├── README.md                   # project overview and usage instructions
├── TUTORIAL.md                 # full beginner tutorial (install, build, usage, DSL, color, etc.)
├── PROGRESS.md                 # changelog — additions/changes/deletions per session
├── LESSON_LEARNED.md           # bug and lesson log
├── STRUCTURE.md                # this file
├── ROADMAP.md                  # phased execution plan: T-01 through T-09 (regex fix → TUI)
├── CONTRIBUTING.md             # contributor guide: setup, code style, PR checklist
├── install.sh                  # one-line binary installer (detects OS/arch, downloads from Releases)
├── homebrew-qk/
│   └── Formula/qk.rb           # Homebrew formula for `brew tap handsomevictor/qk && brew install qk`
└── CLAUDE.md                   # AI-assisted development rules

.github/
├── workflows/
│   ├── ci.yml                  # CI: fmt + clippy + test on ubuntu/macos/windows
│   └── release.yml             # Release: cross-compile binaries on tag push (v*)

src/
├── main.rs                     # entry point — parses CLI, wires up the full pipeline
│                               #   run_dsl() / run_keyword() / load_records()
│                               #   determine_mode(): . / not  / | → DSL, otherwise keyword layer
│
├── cli.rs                      # CLI argument definitions (clap structs)
│                               #   Cli { args, fmt, explain, color, no_color, no_header, ui, cast }
│                               #   OutputFormat { Ndjson, Pretty, Table, Csv, Raw }
│                               #   use_color(): priority --no-color > --color > NO_COLOR env > tty detection
│                               #   --no-header: treat CSV/TSV first row as data, not header
│                               #   --ui: launch interactive TUI browser
│
├── detect.rs                   # automatic format detection
│                               #   reads first 512 bytes (magic bytes + heuristics)
│                               #   Format enum: Ndjson | Json | Csv | Tsv | Logfmt
│                               #               | Yaml | Toml | Gzip | Plaintext
│
├── record.rs                   # unified Record IR (intermediate representation)
│                               #   Record { fields: IndexMap<Arc<str>, Value>, raw: String, source: SourceInfo }
│                               #   get(key): supports dot-notation nested access (response.status)
│
├── parser/
│   ├── mod.rs                  # dispatches to the appropriate parser based on Format enum
│   │                           #   parse_json_document(): handles JSON array or single object
│   ├── ndjson.rs               # NDJSON parser (one JSON object per line)
│   ├── logfmt.rs               # logfmt parser (key=value pairs, supports quoted values)
│   ├── csv.rs                  # CSV/TSV parser (delimiter parameterized)
│   │                           #   parse_with_header(): header row → field names
│   │                           #   parse_headerless(): columns named col1, col2, col3...
│   │                           #   coerce_value(): str→Number/Bool/Null/String type coercion
│   ├── yaml.rs                 # YAML parser (serde_yaml multi-document support, yaml_to_json conversion)
│   ├── toml_fmt.rs             # TOML parser (explicit ::toml::Value path to avoid crate name ambiguity)
│   └── plaintext.rs            # fallback — each line becomes Record { line: "..." }
│
├── query/
│   ├── mod.rs                  # module declarations
│   │
│   ├── fast/                   # fast keyword layer
│   │   ├── mod.rs
│   │   ├── parser.rs           # parses "where level=error select ts msg" → FastQuery AST
│   │   │                       #   FilterOp: Eq/Ne/Gt/Lt/Gte/Lte/Regex/Contains/Exists
│   │   │                       #            StartsWith/EndsWith/Glob
│   │   └── eval.rs             # applies FastQuery to a stream of Records
│   │                           #   eval() → Result<(Vec<Record>, Vec<String>)> (records + warnings)
│   │                           #   collect_numeric_field(): emit warnings for unexpected string values
│   │                           #   eval_glob() / glob_to_regex(): shell wildcard → regex conversion
│   │                           #   eval_regex(): real regex matching (regex crate)
│   │
│   └── dsl/                    # DSL expression layer (nom parser)
│       ├── mod.rs
│       ├── ast.rs              # DslQuery { filter: Expr, transforms: Vec<Stage> }
│       │                       #   Expr: True | Compare | Exists | And | Or | Not
│       │                       #   Stage: Pick | Omit | Count | SortBy | GroupBy | GroupByTime
│       │                       #          Limit | Skip | Dedup | Sum | Avg | Min | Max
│       ├── parser.rs           # nom v7 parser; supports full boolean syntax and pipeline stages
│       └── eval.rs             # recursive boolean evaluation + 13 pipeline stages
│                               #   eval() → Result<(Vec<Record>, Vec<String>)> (records + warnings)
│                               #   collect_numeric_field_dsl(): emit warnings for unexpected strings
│                               #   compare_contains: memchr SIMD substring search
│                               #   compare_regex: regex crate pattern matching
│
├── output/
│   ├── mod.rs                  # output dispatch (based on OutputFormat and use_color)
│   │                           #   render(records, fmt, use_color)
│   ├── color.rs                # terminal color NDJSON renderer
│   │                           #   paint_record(): semantically-aware colorization
│   │                           #   level→red/yellow/green/blue, msg→bright white, ts→dim, status→HTTP status color
│   ├── ndjson.rs               # NDJSON output (write(records, out, use_color))
│   ├── pretty.rs               # indented JSON output, blank line between blocks (replaces jq .), color support
│   ├── table.rs                # comfy-table aligned table (column width truncated at 60 chars, colored header)
│   └── csv_out.rs              # CSV re-serialization (RFC 4180 escaping)
│
├── tui/
│   ├── mod.rs                  # pub fn run(records, file_names) — TUI entry point
│   ├── app.rs                  # App state: query, cursor, all_records, results, scroll, error
│   │                           #   insert_char / delete_char_before / move_cursor_left/right
│   │                           #   eval(): re-runs query live; detects DSL vs keyword automatically
│   ├── ui.rs                   # ratatui layout: input block / results pane / status bar
│   └── events.rs               # crossterm event loop; handle_key → insert/backspace/scroll/quit
│                               #   run(): enable raw mode, alternate screen, event loop, restore
│
└── util/
    ├── mod.rs
    ├── cast.rs                 # --cast FIELD=TYPE type coercion
    │                           #   CastType enum: Number | Str | Bool | Null | Auto
    │                           #   parse_cast_map(): parse --cast CLI args
    │                           #   apply_casts(): transform record fields before query eval
    │                           #   is_null_like(): shared null sentinel detection
    ├── error.rs                # QkError enum (Io | Parse | Query | UnsupportedFormat)
    ├── intern.rs               # global Arc<str> interning pool (OnceLock<RwLock<HashMap>>)
    ├── mmap.rs                 # mmap large file reading (≥ 64 KiB) + direct read for small files
    ├── time.rs                 # timestamp parsing + time-series bucketing utilities
    │                           #   parse_bucket_secs("5m") → 300; value_to_timestamp(RFC3339/epoch)
    │                           #   bucket_label(ts, bucket_secs) → RFC 3339 string; looks_like_duration
    └── decompress.rs           # transparent gzip decompression (flate2); is_gzip / decompress_gz

tests/
├── fast_layer.rs               # integration tests: keyword syntax end-to-end (including --color / --no-color)
├── dsl_layer.rs                # integration tests: DSL expression layer end-to-end (all operators + pipeline stages)
├── formats.rs                  # integration tests: per-format parsing + output formats
└── fixtures/
    ├── sample.ndjson           # 6 NDJSON log records
    ├── sample.logfmt           # 5 logfmt records
    ├── sample.csv              # 5 CSV records
    ├── sample.yaml             # 5 multi-document YAML records
    ├── sample.toml             # 1 flat TOML config record
    └── timeseries.ndjson       # 12 NDJSON records with RFC 3339 timestamps (for bucket tests)

tutorial/                       # ready-made test fixtures — no setup needed (cd tutorial)
├── app.log                     # 25 NDJSON, 2–3 level nested (context.*, request.headers.*, response.*)
├── access.log                  # 20 NDJSON HTTP logs (client.*, server.*)
├── k8s.log                     # 20 NDJSON Kubernetes events (pod.labels.*, container.restart_count)
├── encoded.log                 # 7 NDJSON with JSON-in-string field values
├── data.json                   # 8-record JSON array (address.*)
├── services.yaml               # 6-document YAML multi-doc (resources.*, healthcheck.*)
├── config.toml                 # TOML with 6 nested sections (server/database/cache/auth/logging/feature_flags)
├── users.csv                   # 15 CSV rows (id/name/age/city/role/active/score/department/salary)
├── events.tsv                  # 20 TSV rows (ts/event/service/severity/region/duration_ms/user_id)
├── services.logfmt             # 16 logfmt records (ts/level/service/msg/host/latency/version)
├── notes.txt                   # 20 plain-text log lines (each → {"line":"..."})
└── app.log.gz                  # gzip of app.log (transparent decompression demo)
```

---

## Key Data Flow

```
CLI arguments
    │
    ├── cli.rs          parse Cli struct (clap derive)
    │
    ├── main.rs         determine_mode() → Dsl | Keyword | Empty
    │
    │   ┌── [DSL mode] ──────────────────────────────────────┐
    │   │  query/dsl/parser.rs → DslQuery + file list        │
    │   │  load_records() → detect + parse                   │
    │   │  query/dsl/eval.rs → filter + transforms           │
    │   └──────────────────────────────────────────────────── ┘
    │
    │   ┌── [Keyword mode] ──────────────────────────────────┐
    │   │  query/fast/parser.rs → FastQuery + file list      │
    │   │  load_records() → detect + parse                   │
    │   │  query/fast/eval.rs → filter + project + sort      │
    │   └──────────────────────────────────────────────────── ┘
    │
    └── output/mod.rs   render(records, fmt, use_color)
        ├── ndjson.rs   → color.rs (if color enabled)
        ├── table.rs    → comfy-table
        ├── csv_out.rs
        └── [raw]       → rec.raw

File reading flow:
    load_records() → rayon par_iter
        → read_one_file()
            → util/mmap.rs        (large file mmap)
            → util/decompress.rs  (transparent gzip decompression)
            → detect.rs           (format sniffing)
            → parser/*.rs         (format parsing)
```

---

## Crate Dependencies

| Crate | Version | Used in | Purpose |
|-------|---------|---------|---------|
| `clap` | 4 | `cli.rs` | CLI argument parsing (derive macro) |
| `serde` + `serde_json` | 1 | global | serialization backbone, Record field types |
| `indexmap` | 2 | `record.rs`, output | ordered HashMap, preserves field insertion order |
| `csv` | 1 | `parser/csv.rs` | robust CSV/TSV parsing |
| `memchr` | 2 | `query/dsl/eval.rs`, `detect.rs` | SIMD byte search (`\n`, substring) |
| `thiserror` | 1 | `util/error.rs` | error type derive macro |
| `owo-colors` | 3 | `output/color.rs` | terminal ANSI colors, respects `NO_COLOR` |
| `rayon` | 1 | `main.rs` | file-level parallelism (`par_iter`) |
| `memmap2` | 0.9 | `util/mmap.rs` | near-zero-copy reading for large files |
| `nom` | 7 | `query/dsl/parser.rs` | parser combinators, DSL expression parsing |
| `regex` | 1 | `query/dsl/eval.rs` | regex matching (`.matches`) |
| `serde_yaml` | 0.9 | `parser/yaml.rs` | YAML parsing, multi-document support |
| `toml` | 0.8 | `parser/toml_fmt.rs` | TOML parsing |
| `flate2` | 1 | `util/decompress.rs` | gzip decompression |
| `comfy-table` | 7 | `output/table.rs` | terminal-aligned tables, dynamic column widths |

---

## Phase Completion Checklist

| Phase | Status | Key Files Added |
|-------|--------|----------------|
| 0 — Scaffolding | ✅ | documentation files |
| 1 — Format detection + parsers | ✅ | detect.rs, parser/ndjson+logfmt+csv+plaintext, record.rs |
| 2 — Fast query layer | ✅ | query/fast/parser.rs, query/fast/eval.rs |
| 3 — Parallelism + performance | ✅ | util/mmap.rs, rayon integration, memchr search |
| 4 — Expression DSL | ✅ | query/dsl/ast.rs, query/dsl/parser.rs, query/dsl/eval.rs |
| 5 — Full format support | ✅ | parser/yaml.rs, parser/toml_fmt.rs, util/decompress.rs |
| 6 — Output + color | ✅ | output/color.rs, output/table.rs, output/csv_out.rs, --color/--no-color |
| 7 — Aggregations + pretty | ✅ | output/pretty.rs, sum/avg/min/max/skip/dedup/fields/head |
| 8 — Text operators + CSV improvements | ✅ | startswith/endswith/glob operators, --no-header, CSV type coercion |
| 9 — Type coercion + warnings | ✅ | util/cast.rs, --cast flag, type-mismatch warnings in all aggregations |

# STRUCTURE — Authoritative Map of the Codebase

This document is the authoritative map of the codebase. It must be kept in sync whenever files are added, moved, or significantly changed.

---

## Current Project Structure (Phases 1–6 Complete)

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
└── CLAUDE.md                   # AI-assisted development rules

src/
├── main.rs                     # entry point — parses CLI, wires up the full pipeline
│                               #   run_dsl() / run_keyword() / load_records()
│                               #   determine_mode(): . / not  / | → DSL, otherwise keyword layer
│
├── cli.rs                      # CLI argument definitions (clap structs)
│                               #   Cli { args, fmt, explain, color, no_color }
│                               #   OutputFormat { Ndjson, Table, Csv, Raw }
│                               #   use_color(): priority --no-color > --color > NO_COLOR env > tty detection
│
├── detect.rs                   # automatic format detection
│                               #   reads first 512 bytes (magic bytes + heuristics)
│                               #   Format enum: Ndjson | Json | Csv | Tsv | Logfmt
│                               #               | Yaml | Toml | Gzip | Plaintext
│
├── record.rs                   # unified Record IR (intermediate representation)
│                               #   Record { fields: IndexMap<String, Value>, raw: String, source: SourceInfo }
│                               #   get(key): supports dot-notation nested access (response.status)
│
├── parser/
│   ├── mod.rs                  # dispatches to the appropriate parser based on Format enum
│   │                           #   parse_json_document(): handles JSON array or single object
│   ├── ndjson.rs               # NDJSON parser (one JSON object per line)
│   ├── logfmt.rs               # logfmt parser (key=value pairs, supports quoted values)
│   ├── csv.rs                  # CSV/TSV parser (delimiter parameterized, header row → field names)
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
│   │   └── eval.rs             # applies FastQuery to a stream of Records
│   │                           #   filter_records / aggregate / apply_projection / sort / limit
│   │
│   └── dsl/                    # DSL expression layer (nom parser)
│       ├── mod.rs
│       ├── ast.rs              # DslQuery { filter: Expr, transforms: Vec<Stage> }
│       │                       #   Expr: True | Compare | Exists | And | Or | Not
│       │                       #   Stage: Pick | Omit | Count | SortBy | GroupBy | Limit
│       │                       #          Skip | Dedup | Sum | Avg | Min | Max
│       ├── parser.rs           # nom v7 parser; supports full boolean syntax and pipeline stages
│       └── eval.rs             # recursive boolean evaluation + 12 pipeline stages
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
└── util/
    ├── mod.rs
    ├── error.rs                # QkError enum (Io | Parse | Query | UnsupportedFormat)
    ├── mmap.rs                 # mmap large file reading (≥ 64 KiB) + direct read for small files
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
    └── sample.toml             # 1 flat TOML config record
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

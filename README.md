# qk — One Tool to Replace Them All

[![CI](https://github.com/handsomevictor/qk/actions/workflows/ci.yml/badge.svg)](https://github.com/handsomevictor/qk/actions/workflows/ci.yml)

`qk` is a fast structured query tool for the terminal. It replaces `grep`, `awk`, `sed`, `jq`, `yq`, `cut`, `sort | uniq`, and more with a single, consistent interface.

No more stacking pipes just to extract two fields from a log file. No more switching between `jq` syntax and `awk` syntax depending on the format. One binary, one syntax, all formats.

---

## Known Limitations

- **No `tail -f` support yet:** qk reads stdin to EOF before processing. `tail -f file | qk ...` will block indefinitely. **Workaround:** use `tail -n 1000 file | qk ...` for finite input. Streaming filter-only queries on stdin (e.g. `cat bigfile | qk where level=error`) are O(output) memory and work on 2 GB+ files.
- **Full file materialization:** when a file path is passed as an argument (not stdin), qk loads the entire file before eval. Files >1 GB may OOM on machines with <16 GB RAM; use stdin piping for the streaming path instead.
- **`--fmt raw` and aggregation records:** synthetic aggregation results (from `count`, `sum`, `avg`, etc.) have no raw source line, so `--fmt raw` outputs an empty line for each such record. Use the default `ndjson` or `pretty` format for aggregation output.

---

## Why qk?

| Task | Before | With qk |
|------|--------|---------|
| Filter error logs | `grep "error" app.log \| awk '{print $3, $5}'` | `qk where level=error select ts msg` |
| Query JSON API logs | `cat req.json \| jq '.[] \| select(.status > 499) \| .path'` | `qk where 'status>499' select path` |
| Count by field | `awk '{print $2}' \| sort \| uniq -c \| sort -rn` | `qk count by service` |
| Cross-format query | ❌ No single tool can do this | `qk where level=error *.log *.json` |
| Nested field access | `jq '.response.headers["x-trace"]'` | `qk select response.headers.x-trace` |
| Multiple conditions | `grep \| awk 'cond1 && cond2'` | `qk where level=error, service=api` |
| Shell-safe comparisons | `awk '$5 > 100'` | `qk where latency gt 100` |
| Deeply nested filter | `jq 'select(.pod.labels.app=="api")'` | `qk where pod.labels.app=api` |

---

## Features

- **Auto format detection** — NDJSON, JSON, YAML, TOML, CSV, TSV, logfmt, plain text; no `-f json` flag needed
- **Record-level model** — matches complete log entries / JSON objects / YAML documents, not just lines
- **Two syntax layers** — fast keyword layer (covers 80% of cases) + expression DSL (covers the remaining 20%)
- **Deeply nested field access** — `pod.labels.app`, `response.headers.x-trace`, any depth via dot-path
- **Readable multi-condition filters** — `where level=error, service=api, latency gt 100` (comma = and)
- **Shell-safe word operators** — `gt`, `lt`, `gte`, `lte` avoid `>` / `<` shell conflicts; `between LOW HIGH` for range checks
- **Relative-time filters** — `ts gt now-5m`, `ts lt now+1h`; supports `s`, `m`, `h`, `d` suffixes; reads RFC 3339 strings, Unix epoch seconds, or epoch milliseconds
- **Structured output** — defaults to NDJSON; pipe directly into another `qk` or `jq`
- **Parallel processing** — uses all CPU cores via `rayon`; scales linearly with file count
- **Transparent decompression** — reads `.gz` files directly, no `gunzip` needed
- **Rich output modes** — `ndjson` (default) / `pretty` (indented JSON, replaces `jq .`) / `table` / `csv` / `raw`
- **Semantic color** — error=red, warn=yellow, info=green, HTTP 5xx=bold red; auto-off when piping
- **Statistical aggregation** — `sum`, `avg`, `min`, `max`, `count by`, `group_by`, `dedup`
- **Time-series bucketing** — `count by 5m` / `count by 1h` groups events into fixed windows; reads RFC 3339 strings, Unix epoch seconds, or epoch milliseconds automatically
- **Written in Rust** — binary size <5MB, startup time <2ms

---

## Installation

### Build from Source

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

git clone https://github.com/handsomevictor/qk.git
cd qk
cargo install --path .
```

### Pre-built Binaries (macOS / Linux)

**Homebrew** (recommended):
```bash
brew tap handsomevictor/qk
brew install qk
```

**One-line install script** (Linux / macOS):
```bash
curl -fsSL https://raw.githubusercontent.com/handsomevictor/qk/main/install.sh | bash
```

**Specific version**:
```bash
QK_VERSION=v0.1.0 bash <(curl -fsSL https://raw.githubusercontent.com/handsomevictor/qk/main/install.sh)
```

**From source** (requires Rust ≥ 1.75):
```bash
cargo install --git https://github.com/handsomevictor/qk
```

Pre-built binaries for x86_64 and aarch64 on Linux, macOS, and Windows are
attached to every [GitHub Release](https://github.com/handsomevictor/qk/releases).

---

## Try It Instantly

The `tutorial/` directory contains ready-made test files for **all supported formats**. No setup needed:

```bash
cd tutorial

# Verify all 11 formats parse:
qk count app.log           # 25 records — NDJSON with 2–3 level nested JSON
qk count access.log        # 20 records — NDJSON (nested client/server)
qk count k8s.log           # 20 records — NDJSON (3-level: pod.labels.app)
qk count data.json         # 8  records — JSON array
qk count services.yaml     # 6  records — YAML multi-document
qk count config.toml       # 1  record  — TOML
qk count users.csv         # 15 records — CSV
qk count events.tsv        # 20 records — TSV
qk count services.logfmt   # 16 records — logfmt
qk count notes.txt         # 20 records — plain text
qk count app.log.gz        # 25 records — transparent gzip

# Start querying immediately:
qk where level=error app.log
qk where level=error, service=api app.log
qk where pod.labels.app=api k8s.log
qk count by service app.log
qk avg latency app.log
```

See [`COMMANDS.md`](./COMMANDS.md) for the complete copy-paste reference (all formats, all operators).

---

## Quick Start

```bash
# Filter errors (replaces grep)
qk where level=error app.log

# Multiple conditions — comma is a readable alias for 'and'
qk where level=error, service=api app.log
qk where level=error, latency gt 100 app.log

# Shell-safe numeric comparisons (gt/lt/gte/lte — no quoting needed)
qk where latency gt 100 app.log
qk where status gte 500 access.log

# Nested field access — any depth
qk where response.status=503 app.log
qk where pod.labels.app=api k8s.log
qk where request.headers.x-trace exists app.log

# Select specific fields
qk where level=error select ts service msg app.log

# Count and aggregate
qk count by service app.log
qk where level=error avg latency app.log
qk sum latency app.log

# Sort and limit
qk sort latency desc limit 10 app.log

# DSL mode for complex logic
qk '.level == "error" and .latency > 1000' app.log
qk '.response.status >= 500 | pick(.ts, .service, .response.status)' app.log
qk '| group_by(.context.region)' app.log

# Pipeline: filter then count
qk where level=error app.log | qk count by service

# Range filter with between
qk where latency between 100 500 app.log

# Relative-time filter (events from the last 5 minutes)
qk where ts gt now-5m app.log

# Pretty-print (replaces jq .)
qk --fmt pretty where level=error app.log

# Works with any format — auto-detected
qk where level=error app.logfmt
qk where city=NYC data.csv
qk where enabled=true services.yaml
qk where level=error app.log.gz     # transparent gzip
```

---

## Syntax Reference

### Keyword Layer (Fast, No Quoting Needed)

```
qk [FILTER] [TRANSFORM] [FILES...]

FILTER:
  where FIELD=VALUE              exact match
  where FIELD!=VALUE             not equal
  where FIELD>VALUE              numeric > (quote or use word op)
  where FIELD gt VALUE           numeric >  (shell-safe, no quoting)
  where FIELD lt VALUE           numeric <  (shell-safe)
  where FIELD gte VALUE          numeric >= (shell-safe)
  where FIELD lte VALUE          numeric <= (shell-safe)
  where FIELD~=PATTERN           regex match
  where FIELD contains TEXT      substring match
  where FIELD between LOW HIGH   inclusive range check (numeric or timestamp)
  where FIELD exists             field presence check
  where FIELD gt now-5m          relative-time: 5 minutes ago (s/m/h/d suffixes)
  where A=1 and B=2              logical AND
  where A=1 or B=2               logical OR
  where A=1, B=2                 comma = alias for 'and' (readable style)
  where A=1, B gt 10, C=x        comma-chain: multiple conditions

TRANSFORM:
  select FIELD [FIELD...]        keep only these fields
  count                          count total matching records
  count by FIELD                 group and count
  count by DURATION [FIELD]      time-bucket: count by 5m, 1h, 1d (reads 'ts' by default)
  fields                         discover all field names in dataset
  sum FIELD                      sum a numeric field
  avg FIELD                      average a numeric field
  min FIELD                      minimum of a numeric field
  max FIELD                      maximum of a numeric field
  sort FIELD [asc|desc]          sort results
  limit N                        take first N records
  head N                         alias for limit
```

### Expression Layer (DSL)

Activated when the first argument starts with `.`, `not `, or `|`:

```
qk 'EXPRESSION' [FILES...]

.field                         access a top-level field
.a.b.c                         nested field access (any depth)
.field == "value"              equality (strings need quotes in DSL)
.field != "value"              not equal
.field > N                     numeric comparison
.field exists                  field presence
.field contains "text"         substring
.field matches "pattern"       regex
not EXPR                       logical NOT
EXPR and EXPR                  logical AND
EXPR or EXPR                   logical OR
EXPR | pick(.a, .b)            keep only specified fields
EXPR | omit(.a, .b)            remove fields
EXPR | count()                 count
EXPR | sort_by(.f desc)        sort
EXPR | group_by(.f)            group and count
EXPR | limit(N)                first N records
EXPR | skip(N)                 skip N records (pagination)
EXPR | dedup(.f)               deduplicate by field
EXPR | sum(.f)                 sum
EXPR | avg(.f)                 average
EXPR | min(.f)                 minimum
EXPR | max(.f)                 maximum
| STAGE                        skip filter, go straight to pipeline
```

---

## Comma Separator

Long filter chains are now readable:

```bash
# Old style (still works)
qk where level=error and service=api and latency gt 100 app.log

# New style — comma is an alias for 'and'
qk where level=error, service=api, latency gt 100 app.log

# Trailing comma on a token also works
qk where level=error, service=api app.log
```

---

## Shell-Safe Numeric Operators

`>` and `<` are shell metacharacters. Two solutions:

```bash
# Option 1: quote the filter (embedded syntax)
qk where 'latency>100' app.log
qk where 'status>=500' access.log

# Option 2: word operators — recommended, no quoting ever needed
qk where latency gt 100 app.log      # >
qk where latency lt 50 app.log       # <
qk where latency gte 88 app.log      # >=
qk where status lte 499 access.log   # <=
```

---

## Nested JSON

Access fields at any depth with dot notation:

```bash
# Two levels deep
qk where response.status=503 app.log
qk where context.region=us-east app.log

# Three levels deep
qk where pod.labels.app=api k8s.log
qk '.request.headers.x-trace exists' app.log

# DSL — filter + project on nested fields
qk '.response.status >= 500 | pick(.ts, .service, .response.status)' app.log
qk '| group_by(.context.region)' app.log
```

### JSON-Encoded String Fields

If a field's value is itself a JSON string (`"payload": "{\"level\":\"error\"}"`), combine with jq:

```bash
# Decode the string field with jq, then query with qk
cat app.log | jq -c '.payload = (.payload | fromjson)' | qk where payload.level=error

# Full pipeline: qk pre-filters → jq decodes → qk aggregates
cat app.log | qk where service=api | jq -c '.meta = (.metadata | fromjson)' | qk count by meta.env
```

---

## Output Formats

```bash
qk --fmt ndjson where level=error app.log   # NDJSON (default)
qk --fmt pretty where level=error app.log   # indented JSON (replaces jq .)
qk --fmt table where level=error app.log    # aligned table
qk --fmt csv where level=error app.log      # CSV (openable in Excel)
qk --fmt raw where level=error app.log      # original source lines

# --fmt must come BEFORE the query expression
qk --fmt table where level=error app.log    # ✅
qk where level=error --fmt table app.log    # ❌
```

---

## Supported Input Formats (Auto-Detected)

| Format | Detection | Notes |
|--------|-----------|-------|
| NDJSON | each line starts with `{` | one JSON object per line |
| JSON | file starts with `[` or `{` | full JSON document or array |
| YAML | `---` header or `.yml`/`.yaml` extension | multi-document supported |
| TOML | `.toml` extension | whole file = one record |
| CSV | comma-separated header row | `.csv` extension |
| TSV | `.tsv` extension | |
| logfmt | `key=value key2=value2` pattern | common in Go services |
| Gzip | magic bytes `0x1f 0x8b` / `.gz` extension | transparent decompression |
| Plain text | fallback | each line → `{"line": "..."}` |

---

## Architecture

```
Input → Format Detector → Parser → Record IR → Query Engine → Output Renderer
                                               ↑
                               Fast Layer (keywords) | DSL Layer (expressions)
```

All formats are normalized to a unified `Record` IR before querying. The query engine never knows the source format. See [`STRUCTURE.md`](./STRUCTURE.md) for the full codebase map.

---

## Performance Targets

| Scenario | Target | Comparison |
|----------|--------|-----------|
| 1 GB NDJSON, simple filter | <2s | ripgrep: ~1s (no parsing), jq: ~30s |
| 1 GB NDJSON, group_by | <5s | awk: ~8s |
| 10,000 files, recursive | <3s | ripgrep: ~1s |

---

## Project Documentation

| File | Purpose |
|------|---------|
| [`README.md`](./README.md) | This file — overview and syntax reference |
| [`tutorial/`](./tutorial/) | Ready-made test files for all 11 supported formats |
| [`COMMANDS.md`](./COMMANDS.md) | All commands in one file — copy-paste reference (uses `tutorial/`) |
| [`TUTORIAL.md`](./TUTORIAL.md) | Full tutorial with runnable examples |
| [`STRUCTURE.md`](./STRUCTURE.md) | Architecture and per-file descriptions |
| [`PROGRESS.md`](./PROGRESS.md) | Changelog — per-session additions/changes |
| [`LESSON_LEARNED.md`](./LESSON_LEARNED.md) | Bug log and lessons |
| [`CLAUDE.md`](./CLAUDE.md) | AI-assisted development rules |

---

## Development

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt
cargo check
echo '{"level":"error","msg":"timeout","service":"api"}' | cargo run -- where level=error
```

---

## Roadmap

- [x] Phase 0 — Project scaffolding and architecture design
- [x] Phase 1 — Format detection + NDJSON/logfmt/CSV parsers + Record IR
- [x] Phase 2 — Fast keyword query layer (where / select / count / sort / limit)
- [x] Phase 3 — Parallel processing (rayon) + mmap + SIMD search
- [x] Phase 4 — Expression DSL layer (nom parser + evaluator)
- [x] Phase 5 — Full format support (YAML / TOML / gzip)
- [x] Phase 6 — Output formatting (table / color / --explain)
- [x] Phase 7 — Statistical aggregation + pretty output + field discovery
- [ ] Phase 8 — GitHub Releases + install script

---

## License

MIT

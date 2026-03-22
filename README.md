# qk — One Tool to Replace Them All

[中文版 README](./README_CN.md)

[![CI](https://github.com/handsomevictor/qk/actions/workflows/ci.yml/badge.svg)](https://github.com/handsomevictor/qk/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey)]()
[![Version](https://img.shields.io/badge/version-0.1.0-green)]()

`qk` is a fast, structured query tool for the terminal.
It replaces `grep`, `awk`, `sed`, `jq`, `yq`, `cut`, `sort | uniq` — with a single consistent command, one syntax, and zero format flags.

---

## Why qk?

### Task comparison

| Task | Traditional tools | qk |
|------|------------------|----|
| Filter error logs | `grep "error" app.log \| awk '{print $3, $5}'` | `qk where level=error select ts msg` |
| Query JSON API logs | `cat req.json \| jq '.[] \| select(.status > 499) \| .path'` | `qk where status>499 select path` |
| Count by field | `awk '{print $2}' \| sort \| uniq -c \| sort -rn` | `qk count by service` |
| Cross-format query | ❌ One tool can't do this | `qk where level=error *.log *.json` |
| Nested field access | `jq '.response.headers["x-trace"]'` | `qk select response.headers.x-trace` |
| Multiple conditions | `grep \| awk 'cond1 && cond2'` | `qk where level=error, service=api` |
| Shell-safe comparisons | `awk '$5 > 100'` (shell metachar risk) | `qk where latency gt 100` |
| Time-series bucketing | ❌ No standard single-tool solution | `qk count by 5m` |

### Feature matrix

| Feature | grep | awk | sed | jq | yq | **qk** |
|---------|:----:|:---:|:---:|:--:|:--:|:------:|
| Auto format detection | ❌ | ❌ | ❌ | ❌ | partial | ✅ |
| Nested field access | ❌ | ❌ | ❌ | ✅ | ✅ | ✅ |
| Cross-format queries | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Aggregation (sum/avg/count) | ❌ | manual | ❌ | partial | partial | ✅ |
| Time-series bucketing | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Shell-safe numeric operators | ❌ | partial | ❌ | ✅ | ✅ | ✅ |
| Transparent gzip | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Multiple output formats | ❌ | ❌ | ❌ | partial | partial | ✅ |
| Interactive TUI | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Single binary, <5 MB | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Startup time | <1ms | <1ms | <1ms | <5ms | <10ms | <2ms |

---

## Features

### Input & Format Support

| | |
|--|--|
| **9 auto-detected formats** | NDJSON, JSON array, YAML (multi-doc), TOML, CSV, TSV, logfmt, plain text, gzip |
| **Transparent gzip** | `data.csv.gz`, `app.log.gz`, `events.tsv.gz` — reads directly, no `gunzip` needed |
| **Record model** | Matches complete log entries / JSON objects / YAML docs, not just lines |
| **Dot-path nesting** | `pod.labels.app`, `response.headers.x-trace` — any depth |

### Query Language

| | |
|--|--|
| **Two syntax layers** | Fast keyword layer (80% of cases) + expression DSL (complex logic) |
| **Filter operators** | `=` `!=` `>` `<` `>=` `<=` `~=` (regex) `contains` `startswith` `endswith` `glob` `between` `exists` |
| **Shell-safe word ops** | `gt` `lt` `gte` `lte` — avoid `>` / `<` quoting issues in shell |
| **Relative time** | `where ts gt now-5m` — reads RFC 3339, Unix epoch seconds, or epoch milliseconds |
| **Comma syntax** | `where level=error, service=api, latency gt 100` — readable AND chain |
| **Logical ops** | `and` / `or` / `not` in both layers |

### Aggregation & Analytics

| | |
|--|--|
| **Numeric aggregates** | `sum`, `avg`, `min`, `max` |
| **Grouping** | `count by FIELD` — group and count |
| **Time bucketing** | `count by 5m` / `count by 1h` / `count by 1d` — newest bucket first by default |
| **Type distribution** | `count types FIELD` — shows number/string/bool/null/missing breakdown |
| **Deduplication** | `dedup` / `count unique FIELD` |
| **Field discovery** | `fields` — lists all field names in dataset |

### Output & Display

| | |
|--|--|
| **5 output formats** | `ndjson` (default) · `pretty` (indented JSON) · `table` · `csv` · `raw` |
| **Semantic color** | error=red, warn=yellow, info=green, HTTP 5xx=bold red; auto-off when piping |
| **Auto-limit** | Caps terminal output at 20 records; notice box shown after output; `--all` / `-A` disables |
| **Stats mode** | `--stats` prints records-in/out and elapsed time to stderr |
| **Interactive TUI** | `--ui` launches a full-screen browser (capped at 50,000 records) |

### Developer Experience

| | |
|--|--|
| **Position-independent flags** | `--fmt`, `--cast`, `--stats`, `--quiet`, `--all` work anywhere in the command |
| **Config file** | `~/.config/qk/config.toml` — `default_fmt`, `default_limit`, `no_color`, `default_time_field` |
| **Actionable errors** | Typo flags show "Did you mean: --quiet?"; bad `--cast` types list valid alternatives |
| **Type cast** | `--cast FIELD=number` forces a field's type before querying |
| **Type warnings** | `latency > "abc"` warns once and returns no results instead of silently wrong answers |
| **`==` detection** | Gives "did you mean `=`?" instead of silent mismatch |
| **Parallel processing** | `rayon` file-level parallelism; scales linearly with file count |
| **Streaming** | Filter-only stdin queries run in streaming mode — O(output) memory, works on 2 GB+ files |

---

## Project Documentation

| File | Purpose |
|------|---------|
| [`README.md`](./README.md) | This file — overview, installation, quick start |
| [`README_CN.md`](./README_CN.md) | Chinese version of this README |
| [`COMMANDS.md`](./COMMANDS.md) | **Complete copy-paste reference** — all commands, all formats (EN) |
| [`COMMANDS_CN.md`](./COMMANDS_CN.md) | Complete copy-paste reference — Chinese version |
| [`COMMANDS_WRONG.md`](./COMMANDS_WRONG.md) | Wrong command examples with expected error output and fixes (EN) |
| [`COMMANDS_WRONG_CN.md`](./COMMANDS_WRONG_CN.md) | Wrong command examples — Chinese version |
| [`TUTORIAL.md`](./TUTORIAL.md) | Full tutorial with runnable examples (EN) |
| [`TUTORIAL_CN.md`](./TUTORIAL_CN.md) | Full tutorial — Chinese version |
| [`tutorial/`](./tutorial/) | Ready-made test files for all 9 supported formats |
| [`STRUCTURE.md`](./STRUCTURE.md) | Architecture and per-file descriptions |
| [`RELEASE.md`](./RELEASE.md) | How to publish a GitHub Release and Homebrew tap |
| [`ROADMAP.md`](./ROADMAP.md) | Detailed upcoming work items |
| [`PROGRESS.md`](./PROGRESS.md) | Changelog — per-session additions/changes |
| [`LESSON_LEARNED.md`](./LESSON_LEARNED.md) | Bug log and lessons |

---

## Installation

### Build from Source

Requires Rust ≥ 1.75:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

git clone https://github.com/handsomevictor/qk.git
cd qk
cargo install --path .
```

Or install directly from git without cloning:

```bash
cargo install --git https://github.com/handsomevictor/qk
```

### Homebrew (macOS / Linux) — coming with v0.1.0

```bash
brew tap handsomevictor/qk
brew install qk
```

### Pre-built Binaries — coming with v0.1.0

Pre-built binaries for x86_64 and aarch64 on Linux, macOS, and Windows will be
available on the [GitHub Releases](https://github.com/handsomevictor/qk/releases) page.

---

## Quick Start

> **Full command reference:** see [`COMMANDS.md`](./COMMANDS.md) (EN) or [`COMMANDS_CN.md`](./COMMANDS_CN.md) (CN)
> for every operator, every format, and every flag — all copy-paste ready.

### Tutorial files

The `tutorial/` directory contains ready-made test files for all supported formats — no setup needed:

```bash
cd tutorial

qk count app.log           # 25 records — NDJSON (2–3 level nested)
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
```

### Common patterns

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

# Time-series bucketing (newest bucket first by default)
qk count by 5m app.log
qk count by 1h ts asc app.log      # chronological order

# Sort and limit
qk sort latency desc limit 10 app.log

# DSL mode for complex logic
qk '.level == "error" and .latency > 1000' app.log
qk '.response.status >= 500 | pick(.ts, .service, .response.status)' app.log
qk '| group_by(.context.region)' app.log

# Pipeline: filter then count
qk where level=error app.log | qk count by service

# Range filter
qk where latency between 100 500 app.log

# Relative-time filter (events from the last 5 minutes)
qk where ts gt now-5m app.log

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
  where FIELD>VALUE              numeric >  (quote or use word op)
  where FIELD gt VALUE           numeric >  (shell-safe)
  where FIELD lt VALUE           numeric <  (shell-safe)
  where FIELD gte VALUE          numeric >= (shell-safe)
  where FIELD lte VALUE          numeric <= (shell-safe)
  where FIELD~=PATTERN           regex match
  where FIELD contains TEXT      substring match
  where FIELD startswith PREFIX  prefix match
  where FIELD endswith SUFFIX    suffix match
  where FIELD glob PATTERN       wildcard match (* and ?)
  where FIELD between LOW HIGH   inclusive range (numeric or timestamp)
  where FIELD exists             field presence check
  where FIELD gt now-5m          relative time: 5 min ago (s/m/h/d)
  where A=1 and B=2              logical AND
  where A=1 or B=2               logical OR
  where A=1, B=2                 comma = alias for 'and'

TRANSFORM:
  select FIELD [FIELD...]        keep only these fields
  count                          count total matching records
  count by FIELD                 group and count
  count by DURATION [FIELD]      time-bucket: 5m, 1h, 1d (default field: ts)
  count by DURATION FIELD asc    time-bucket, chronological order
  count unique FIELD             count distinct values
  count types FIELD              value-type distribution
  fields                         discover all field names
  sum FIELD                      sum a numeric field
  avg FIELD                      average a numeric field
  min FIELD                      minimum
  max FIELD                      maximum
  sort FIELD [asc|desc]          sort results
  limit N                        take first N records
  head N                         alias for limit

FLAGS (position-independent — work anywhere in the command):
  --fmt ndjson|pretty|table|csv|raw
  --cast FIELD=TYPE[,FIELD=TYPE]
  --stats                        print processing statistics to stderr
  --quiet / -q                   suppress stderr warnings
  --all / -A                     disable auto-limit
  --no-color                     disable ANSI color output
  --explain                      print parsed query AST
  --ui                           interactive TUI browser
```

### Expression Layer (DSL)

Activated when the first argument starts with `.`, `not `, or `|`:

```
qk 'EXPRESSION' [FILES...]

.field                         access a top-level field
.a.b.c                         nested field access
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
EXPR | skip(N)                 skip N records
EXPR | dedup(.f)               deduplicate by field
EXPR | sum(.f)                 sum
EXPR | avg(.f)                 average
EXPR | min(.f)                 minimum
EXPR | max(.f)                 maximum
| STAGE                        skip filter, apply pipeline directly
```

---

## Output Formats

```bash
qk --fmt ndjson where level=error app.log   # NDJSON (default)
qk --fmt pretty where level=error app.log   # indented JSON (replaces jq .)
qk --fmt table  where level=error app.log   # aligned table
qk --fmt csv    where level=error app.log   # CSV (openable in Excel)
qk --fmt raw    where level=error app.log   # original source lines

# All flags are position-independent — these are all equivalent:
qk --fmt table where level=error app.log
qk where level=error --fmt table app.log
qk where level=error app.log --fmt table
```

Set a persistent default in `~/.config/qk/config.toml`:

```toml
default_fmt = "pretty"
```

---

## Supported Input Formats

| Format | Detection | Notes |
|--------|-----------|-------|
| NDJSON | each line starts with `{` | one JSON object per line |
| JSON | file starts with `[` or `{` | full document or array |
| YAML | `---` header or `.yml`/`.yaml` extension | multi-document supported |
| TOML | `.toml` extension | whole file = one record |
| CSV | comma-separated header row | `.csv` extension |
| TSV | `.tsv` extension | |
| logfmt | `key=value key2=value2` pattern | common in Go services |
| Gzip | magic bytes `0x1f 0x8b` / `.gz` extension | transparent decompression of any inner format |
| Plain text | fallback | each line → `{"line": "..."}` |

---

## Config File

`~/.config/qk/config.toml` (XDG-aware):

```toml
default_fmt         = "pretty"   # ndjson | pretty | table | csv | raw
default_limit       = 20         # auto-limit cap (0 = disabled)
no_color            = false      # true to always disable ANSI color
default_time_field  = "ts"       # default timestamp field for count by DURATION
```

View current config: `qk config show`
Reset to defaults: `qk config reset`

---

## Architecture

```
Input → Format Detector → Parser → Record IR → Query Engine → Output Renderer
                                               ↑
                               Fast Layer (keywords) | DSL Layer (expressions)
```

All formats are normalized to a unified `Record` IR before querying. The query engine never sees the source format. See [`STRUCTURE.md`](./STRUCTURE.md) for the full codebase map.

---

## Performance

| Scenario | Target | Comparison |
|----------|--------|------------|
| 1 GB NDJSON, simple filter | < 2 s | ripgrep ~1 s (no parsing), jq ~30 s |
| 1 GB NDJSON, group_by | < 5 s | awk ~8 s |
| 10,000 files, recursive | < 3 s | ripgrep ~1 s |

Implementation details:
- `rayon` file-level parallelism (all CPU cores)
- `mmap` for files ≥ 64 KiB
- `memmem` SIMD-accelerated `contains` matching

---

## Development

```bash
cargo test                          # run all 446 tests
cargo clippy -- -D warnings         # zero warnings required
cargo fmt                           # format before committing
cargo bench                         # run benchmarks

# Quick smoke test
echo '{"level":"error","msg":"timeout","service":"api"}' | cargo run -- where level=error
```

---

## Roadmap

### Completed

- [x] Phase 0 — Project scaffolding and architecture
- [x] Phase 1 — Format detection + NDJSON/logfmt/CSV parsers + Record IR
- [x] Phase 2 — Fast keyword query layer (where / select / count / sort / limit)
- [x] Phase 3 — Parallel processing (rayon) + mmap + SIMD search
- [x] Phase 4 — Expression DSL layer (nom parser + evaluator)
- [x] Phase 5 — Full format support (YAML / TOML / gzip)
- [x] Phase 6 — Output formatting (table / color / --explain / TUI)
- [x] Phase 7 — Statistical aggregation + pretty output + field discovery
- [x] Phase 8 — String operators + CSV improvements (startswith/endswith/glob, --no-header, --cast)
- [x] Phase 9 — UX polish: position-independent flags, actionable errors, time-bucket sort, auto-limit box, TUI cap

### Upcoming

- [ ] **T-01** — Fix regex recompilation: compile regex once per query, not per record (10–100× speedup for regex filters)
- [ ] **v0.1.0** — GitHub Release with pre-built binaries + Homebrew tap (see [`RELEASE.md`](./RELEASE.md))
- [ ] **T-04** — Streaming file reads: replace full-file materialization with a chunked/streaming approach to eliminate OOM risk on files > 1 GB
- [ ] **T-05** — Streaming stdin: support `tail -f file | qk …` without blocking on EOF
- [ ] **T-06** — `JOIN` across two files: `qk join users.csv orders.csv on id`
- [ ] **T-07** — `--output-file` flag for writing results to a file instead of stdout
- [ ] **T-08** — Watch mode: re-run query on file change (`--watch`)

---

## Known Limitations

- **No `tail -f` support:** qk reads stdin to EOF before processing. `tail -f file | qk ...` will block. **Workaround:** use `tail -n 1000 file | qk ...` for finite input. Filter-only stdin queries (e.g. `cat bigfile | qk where level=error`) are O(output) memory and work on 2 GB+ files.
- **Full file materialization:** when a file path is passed as an argument (not stdin), qk loads the entire file before eval. Files > 1 GB may OOM on machines with < 16 GB RAM; use stdin piping for the streaming path.
- **`--fmt raw` and aggregation:** synthetic aggregation results (from `count`, `sum`, `avg`, etc.) have no raw source line, so `--fmt raw` outputs an empty line per record. Use `ndjson` or `pretty` for aggregation output.

---

## License

MIT

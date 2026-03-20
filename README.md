# qk — One Tool to Replace Them All

`qk` is a fast structured query tool for the terminal. It replaces `grep`, `awk`, `sed`, `jq`, `yq`, `cut`, `sort | uniq`, and more with a single, consistent interface.

No more stacking pipes just to extract two fields from a log file. No more switching between `jq` syntax and `awk` syntax depending on the format. One binary, one syntax, all formats.

---

## Why qk?

| Task | Before | With qk |
|------|--------|---------|
| Filter error logs | `grep "error" app.log \| awk '{print $3, $5}'` | `qk where level=error select ts msg` |
| Query JSON API logs | `cat req.json \| jq '.[] \| select(.status > 499) \| .path'` | `qk where status>499 select path` |
| Count by field | `awk '{print $2}' \| sort \| uniq -c \| sort -rn` | `qk count by service` |
| Cross-format query | ❌ No single tool can do this | `qk where error!=null *.log *.json` |
| Nested field access | `jq '.response.headers["x-trace"]'` | `qk select response.headers.x-trace` |

---

## Features

- **Auto format detection** — JSON, NDJSON, YAML, TOML, CSV, logfmt, syslog, nginx/CLF, plain text, no `-f json` flag needed
- **Record-level model** — matches complete log entries / JSON objects / YAML documents, not just lines
- **Two syntax layers** — fast keyword layer (covers 80% of use cases) + expression DSL (covers the remaining 20%)
- **Structured output** — defaults to NDJSON, making it easy to pipe into another `qk` or other tools
- **Parallel processing** — uses all CPU cores via `rayon`, scales linearly with number of files (Phase 3)
- **Transparent decompression** — reads `.gz` and `.zst` files directly (Phase 5)
- **Written in Rust** — binary size <5MB, startup time <2ms

---

## Installation

### Build from Source (Development)

```bash
# Prerequisite: install the Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone and build
git clone https://github.com/handsomevictor/qk.git
cd qk
cargo build --release

# Binary is located at:
./target/release/qk

# Optional: install to PATH
cargo install --path .
```

### Pre-built Binaries

Coming soon via GitHub Releases.

---

## Quick Start

```bash
# Filter lines containing "error" (replaces grep)
qk where level=error app.log

# Select specific fields (replaces awk)
qk where level=error select ts service msg app.log

# Count occurrences (replaces sort | uniq -c)
qk count by level app.log

# Query JSON without knowing the schema
qk where status>499 select path status latency requests.json

# Query across multiple formats at once
qk where error!=null *.log *.json k8s/*.yaml

# Sort and limit results
qk where latency>200 sort latency desc limit 20 app.log

# Pipeline: filter then count
qk where level=error app.log | qk count by service

# Inspect parse results (debug mode)
qk where level=error --explain app.log
```

---

## Syntax Reference

### Fast Layer (Keyword Syntax)

```
qk [FILTER] [TRANSFORM] [FILES...]

FILTER:
  where FIELD=VALUE          exact match
  where FIELD!=VALUE         not equal
  where FIELD>VALUE          numeric greater than
  where FIELD<VALUE          numeric less than
  where FIELD>=VALUE         numeric greater than or equal
  where FIELD<=VALUE         numeric less than or equal
  where FIELD~=PATTERN       regex match
  where FIELD contains TEXT  substring match
  where FIELD exists         field exists
  where FIELD=A or FIELD=B   logical OR
  where FIELD=A and OTHER=B  logical AND (default when chained)

TRANSFORM:
  select FIELD [FIELD...]    keep only these fields
  count                      count total matching records
  count by FIELD             group and count by field
  sort FIELD [asc|desc]      sort results
  limit N                    take first N records
```

### Expression Layer (DSL Syntax)

Wrap in single quotes to use the expression DSL (Phase 4):

```
qk 'EXPRESSION' [FILES...]

.field                       access a field
.a.b.c                       nested field access
.field == value              equality
.field > value               comparison
not .field                   logical NOT
.a and .b                    logical AND
.a or .b                     logical OR
expr | fn()                  pipe into a function
pick(.a, .b)                 select fields
omit(.a, .b)                 remove fields
group_by(.field)             group by field
map(expr)                    transform each record
count()                      count
sort_by(.field)              sort by field
```

---

## Output Formats

```bash
qk where level=error app.log              # NDJSON (default, pipe-friendly)
qk where level=error app.log --fmt raw    # raw matching lines
```

---

## Supported Formats

| Format | Auto-detected by | Notes |
|--------|-----------------|-------|
| NDJSON | each line starts with `{` | one JSON object per line |
| JSON | file starts with `[` or `{` | full JSON document |
| YAML | `---` header or `.yml`/`.yaml` extension | multi-document supported (Phase 5) |
| TOML | `.toml` extension | full support in Phase 5 |
| CSV | comma-separated header row | |
| TSV | tab-separated | |
| logfmt | `key=value key2=value2` pattern | common in Go services |
| Plain text | fallback | line = record, `line` field |

---

## Architecture Overview

See [`STRUCTURE.md`](./STRUCTURE.md) for details.

Summary:

```
Input → Format Detector → Parser → Record IR → Query Engine → Output Renderer
                                               ↑
                                   Fast Layer (keywords) | Expression Layer (DSL)
```

All formats are normalized into a unified `Record` intermediate representation before querying. The query engine never knows which format the data came from.

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
| [`README.md`](./README.md) | This file — project overview and usage |
| [`TUTORIAL.md`](./TUTORIAL.md) | Full tutorial — installation, building, usage, development |
| [`STRUCTURE.md`](./STRUCTURE.md) | Architecture and per-file descriptions |
| [`PROGRESS.md`](./PROGRESS.md) | Changelog — additions/changes/deletions per session |
| [`LESSON_LEARNED.md`](./LESSON_LEARNED.md) | Bug log — bugs encountered, debugging process, lessons |
| [`CLAUDE.md`](./CLAUDE.md) | AI-assisted development rules (auto-read by Claude Code) |

---

## Development

```bash
# Run tests
cargo test

# Run with sample data
echo '{"level":"error","msg":"timeout","service":"api"}' | cargo run -- where level=error

# Check lint
cargo clippy -- -D warnings

# Format code
cargo fmt

# Check compilation only (no binary output, fastest)
cargo check
```

---

## Roadmap

- [x] Phase 0 — Project scaffolding and architecture design
- [x] Phase 1 — Format detection + NDJSON/logfmt/CSV parsers + Record IR
- [x] Phase 2 — Fast keyword query layer (where / select / count / sort / limit)
- [ ] Phase 3 — Parallel processing (rayon) + mmap + SIMD search
- [ ] Phase 4 — Expression DSL layer (nom parser + evaluator)
- [ ] Phase 5 — Full format support (YAML / TOML / syslog / gz / zst)
- [ ] Phase 6 — Output formatting (table / color / --explain enhancements)
- [ ] Phase 7 — GitHub Releases + install script

---

## License

MIT

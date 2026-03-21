# qk Complete Tutorial

Every feature in this tutorial includes **copy-paste-ready examples** with expected output.

---

## Table of Contents

1. [Installation](#installation)
2. [Preparing Test Data](#preparing-test-data)
3. [Basic Usage](#basic-usage)
4. [Filtering (where)](#filtering-where)
5. [Field Selection (select)](#field-selection-select)
6. [Counting (count)](#counting-count)
7. [Sorting (sort)](#sorting-sort)
8. [Limiting Results (limit / head)](#limiting-results-limit--head)
9. [Numeric Aggregation (sum / avg / min / max)](#numeric-aggregation-sum--avg--min--max)
10. [Field Discovery (fields)](#field-discovery-fields)
11. [DSL Expression Syntax](#dsl-expression-syntax)
12. [DSL Pipeline Stages](#dsl-pipeline-stages)
13. [qk + jq: Handling JSON-Encoded Strings](#qk--jq-handling-json-encoded-strings)
14. [Output Formats (--fmt)](#output-formats---fmt)
15. [Color Output (--color)](#color-output---color)
16. [Multiple File Formats](#multiple-file-formats)
17. [Pipeline Composition](#pipeline-composition)
18. [Common Questions](#common-questions)
19. [Quick Reference](#quick-reference)

> **New in latest release**: `startswith`, `endswith`, `glob` operators; `--no-header` for CSV; `--cast FIELD=TYPE` for type coercion; automatic warnings for non-numeric values in aggregations.

---

## Installation

### Step 1: Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# After installation, reopen your terminal or run:
source ~/.cargo/env
```

### Step 2: Build and Install qk

```bash
git clone https://github.com/handsomevictor/qk.git
cd qk
cargo install --path .
```

Verify the installation:

```bash
qk --version
```

### Using Without Installing During Development

```bash
cargo run -- where level=error app.log
# Equivalent to the installed version:
qk where level=error app.log
```

---

## Preparing Test Data

The `tutorial/` directory in the repository contains ready-made files for all 11 supported formats — no setup needed. Just `cd tutorial` before running any examples:

```bash
cd qk/tutorial    # all commands below assume this directory

# Verify everything works — each should print a record count:
qk count app.log           # 25 — NDJSON with 2–3 level nested JSON
qk count access.log        # 20 — NDJSON (nested client/server objects)
qk count k8s.log           # 20 — NDJSON (3-level: pod.labels.app/team)
qk count encoded.log       # 7  — NDJSON (JSON-in-string fields)
qk count data.json         # 8  — JSON array
qk count services.yaml     # 6  — YAML multi-document
qk count config.toml       # 1  — TOML (whole file = one record)
qk count users.csv         # 15 — CSV
qk count events.tsv        # 20 — TSV
qk count services.logfmt   # 16 — logfmt (key=value, common in Go)
qk count notes.txt         # 20 — plain text (each line → {"line":"..."})
qk count app.log.gz        # 25 — transparent gzip decompression
```

**File reference:**

| File | Format | Records | Key fields |
|------|--------|---------|------------|
| `app.log` | NDJSON | 25 | `level service msg latency host context.region request.path response.status` |
| `access.log` | NDJSON | 20 | `method path status latency client.ip client.country server.host` |
| `k8s.log` | NDJSON | 20 | `level msg pod.name pod.namespace pod.labels.app pod.labels.team container.restart_count` |
| `encoded.log` | NDJSON | 7 | `service metadata payload` (values are JSON strings) |
| `data.json` | JSON array | 8 | `id name age city role active score address.country` |
| `services.yaml` | YAML | 6 | `name status replicas enabled port env resources.cpu` |
| `config.toml` | TOML | 1 | `server.port server.workers database.pool_max logging.level feature_flags.*` |
| `users.csv` | CSV | 15 | `name age city role active score department salary` |
| `events.tsv` | TSV | 20 | `ts event service severity region duration_ms user_id` |
| `services.logfmt` | logfmt | 16 | `ts level service msg host latency version` |
| `notes.txt` | plain text | 20 | `line` (the full text of each line) |
| `app.log.gz` | gzip | 25 | same as `app.log` |
| `mixed.log` | NDJSON | 12 | intentionally mixed-type fields: `latency` (Number/String/null), `score` (Number/String/null), `active` (Bool/String), `status` (Number) |

---

## Basic Usage

### Display All Records

```bash
qk app.log
# → {"ts":"2024-01-01T10:00:00Z","level":"info","service":"api","msg":"server started","latency":0,...}
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → (25 records total)
```

In a terminal, output is colorized: error=red, warn=yellow, info=green.

### Read From stdin

```bash
echo '{"level":"error","msg":"oops"}' | qk
# → {"level":"error","msg":"oops"}
```

### Inspect Parsing (--explain)

```bash
qk --explain where level=error app.log
# → mode:    Keyword
# → format:  Ndjson (detected)
# → query:   FastQuery { filters: [level = error], ... }
# → files:   ["app.log"]
```

The `--explain` flag prints the detected format and parsed query, then exits.

---

## Filtering (where)

### Equals (=)

```bash
qk where level=error app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → {"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0,...}
# → (all error records)
```

### Not Equals (!=)

```bash
qk where level!=info app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error",...}
# → {"ts":"2024-01-01T10:02:00Z","level":"warn",...}
# → {"ts":"2024-01-01T10:04:00Z","level":"error",...}
# → (all non-info entries)
```

### Numeric Greater Than (>)

```bash
# Quoted (embedded operators work when quoted)
qk where 'latency>100' app.log
# Word operators — no quoting needed, shell-safe
qk where latency gt 100 app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → {"ts":"2024-01-01T10:02:00Z","level":"warn","service":"worker","msg":"queue depth high","latency":150,...}
# → (all records with latency > 100)
```

### Numeric Less Than (<)

```bash
# Quoted style
qk where 'latency<50' app.log
# Word operator style — shell-safe
qk where latency lt 50 app.log
# → {"ts":"2024-01-01T10:00:00Z","level":"info","service":"api","msg":"server started","latency":0,...}
# → {"ts":"2024-01-01T10:03:00Z","level":"info","service":"api","msg":"request ok","latency":42,...}
# → (all records with latency < 50)
```

### Greater Than or Equal (>=)

```bash
# Quoted style
qk where 'status>=500' access.log
# Word operator style — shell-safe
qk where status gte 500 access.log
# → {"ts":"2024-01-01T10:02:00Z","method":"GET","path":"/api/orders","status":500,"latency":3200,...}
# → {"ts":"2024-01-01T10:04:00Z","method":"GET","path":"/api/users","status":503,"latency":9800,...}
# → (all 5xx responses)
```

### Less Than or Equal (<=)

```bash
# Quoted style
qk where 'latency<=42' app.log
# Word operator style — shell-safe
qk where latency lte 42 app.log
# → {"ts":"2024-01-01T10:00:00Z",...,"latency":0}
# → {"ts":"2024-01-01T10:03:00Z",...,"latency":42}
# → (all records with latency <= 42)
```

### Regex Match (~=)

> **zsh/bash note**: `*` is a glob metacharacter in shells. Always quote regex patterns to prevent glob expansion.

```bash
# Quote the pattern to prevent shell glob expansion
qk where 'msg~=.*timeout.*' app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → {"ts":"2024-01-01T10:07:00Z","level":"error","service":"db","msg":"query timeout","latency":5001,...}

qk where 'msg~=pan.*pointer' app.log
# → (records where msg matches the pattern)
```

### Contains Substring (contains)

```bash
qk where msg contains queue app.log
# → {"ts":"2024-01-01T10:02:00Z","level":"warn","service":"worker","msg":"queue depth high","latency":150,...}
```

### Starts With (startswith)

```bash
qk where msg startswith connection app.log
# → {"level":"error","msg":"connection timeout",...}
# → (all records where msg begins with "connection")

qk where path startswith /api/ access.log
# → (all paths beginning with /api/)

qk where name startswith Al users.csv
# → (Alice, Alex, Alfred, ...)

qk where line startswith ERROR notes.txt
# → (lines that begin with the word "ERROR")
```

### Ends With (endswith)

```bash
qk where path endswith users access.log
# → (all paths ending in "users" — e.g. /api/users)

qk where msg endswith timeout app.log
# → (messages ending in "timeout")

qk where name endswith son users.csv
# → (Jackson, Wilson, ...)
```

### Shell-Style Wildcards (glob)

> **Shell note**: `*` and `?` are shell metacharacters. Always quote glob patterns with single quotes.

`glob` is **case-insensitive** — `'*error*'` also matches `ERROR` or `Error`.

```bash
qk where msg glob '*timeout*' app.log
# → (all records where msg contains "timeout" anywhere — case-insensitive)

qk where name glob 'Al*' users.csv
# → Alice, Alex, Alfred, ... (starts with "Al", any suffix)

qk where name glob '*son' users.csv
# → Jackson, Wilson, ... (ends with "son")

qk where path glob '/api/*' access.log
# → (all API paths)

qk where line glob '*ERROR*' notes.txt
# → (lines containing ERROR — matches error, Error, ERROR)

# ? matches any single character
qk where msg glob 'timeout?' app.log
# → (e.g. "timeouts", "timeout.")
```

**Comparison of text search operators:**

| Operator | Example | Case sensitive? | Notes |
|----------|---------|----------------|-------|
| `contains` | `where msg contains timeout` | Yes | Simple substring |
| `startswith` | `where path startswith /api/` | Yes | Prefix check |
| `endswith` | `where path endswith users` | Yes | Suffix check |
| `glob` | `where msg glob '*timeout*'` | **No** | `*` = any chars, `?` = one char |
| `~=` | `where 'msg~=.*timeout.*'` | Depends on pattern | Full regex, use `(?i)` for case-insensitive |

### Field Exists (exists)

```bash
# Find all records that have a field named "error" (note: this is the field name, not level=error)
echo '{"level":"info","msg":"ok"}
{"level":"error","msg":"bad","error":"connection refused"}' | qk where error exists
# → {"level":"error","msg":"bad","error":"connection refused"}
```

### AND — Multiple Conditions

```bash
qk where level=error and service=api app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → (all error records from the api service)
```

### OR — Multiple Conditions

```bash
qk where level=error or level=warn app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error",...}
# → {"ts":"2024-01-01T10:02:00Z","level":"warn",...}
# → {"ts":"2024-01-01T10:04:00Z","level":"error",...}
# → (all error and warn records)
```

### Comma Separator (Readable AND)

Comma is an alias for `and` — write conditions as a comma-separated list for clarity:

```bash
qk where level=error, service=api app.log
# → {"level":"error","service":"api","msg":"connection timeout","latency":3001,...}

# Comma can also stand alone as a token
qk where level=error , service=api app.log

# Mix comma with and/or (comma binds as and)
qk where level=error, latency gt 100 app.log
# → {"level":"error","latency":3001,...}
# → {"level":"error","latency":5001,...}
```

Before commas, the only option was:
`qk where level=error and service=api and latency gt 100 app.log`

With commas:
`qk where level=error, service=api, latency gt 100 app.log`

### Nested Field Access (dot path)

```bash
# Simple two-level nested field filter
qk where response.status=503 app.log
# → {"level":"error","service":"api","msg":"upstream service unavailable","response":{"status":503,...},...}

# Word operators on nested numeric fields
qk where response.status gte 500 app.log
qk where 'response.status>=500' app.log

# Access context (2-level nesting)
qk where context.region=us-east app.log

# Three-level nesting: pod.labels.app in Kubernetes logs
qk where pod.labels.app=api k8s.log
qk where pod.labels.team=platform, level=error k8s.log
```

---

## Field Selection (select)

### Keep Only Specified Fields

```bash
qk where level=error select ts service msg app.log
# → {"ts":"2024-01-01T10:01:00Z","service":"api","msg":"connection timeout"}
# → {"ts":"2024-01-01T10:04:00Z","service":"worker","msg":"panic: nil pointer"}
# → (all error records with only ts, service, msg)
```

### Select Fields Without Filtering

```bash
qk select level msg service app.log
# → {"level":"info","msg":"server started","service":"api"}
# → {"level":"error","msg":"connection timeout","service":"api"}
# → {"level":"warn","msg":"queue depth high","service":"worker"}
# → (all 25 records, only level, msg, service retained)
```

### Select Nested Fields

```bash
qk where response.status=503 select service response.status response.error app.log
# → {"service":"api","response.status":503,"response.error":"connection refused: ml-service:8080"}
```

---

## Counting (count)

### Count Total Records

```bash
qk count app.log
# → {"count":25}
```

### Count After Filtering

```bash
qk where level=error count app.log
# → {"count":7}
```

### Count Grouped By Field

```bash
qk count by level app.log
# → {"level":"info","count":10}
# → {"level":"error","count":7}
# → {"level":"warn","count":5}
# → {"level":"debug","count":2}
# → (sorted by count descending)
```

```bash
qk count by level k8s.log
# → {"level":"info","count":9}
# → {"level":"warn","count":6}
# → {"level":"error","count":5}
```

Results are sorted by count descending.

### Group By Another Field

```bash
qk count by service app.log
# → {"service":"api","count":9}
# → {"service":"worker","count":4}
# → (all services by count)
```

```bash
# Three-level nested group-by
qk count by pod.labels.team k8s.log
# → {"pod.labels.team":"platform","count":8}
# → {"pod.labels.team":"infra","count":4}
# → {"pod.labels.team":"data","count":4}
# → (all teams by count)
```

### Filter Then Group

```bash
qk where latency gt 0 count by service app.log
# → records filtered to latency > 0, then grouped by service
```

### Count by Time Bucket

Group events into fixed-width time windows.  Use a duration suffix: `s` (seconds),
`m` (minutes), `h` (hours), `d` (days).  The timestamp field defaults to `ts`.

```bash
# 5-minute buckets (reads 'ts' field)
qk count by 5m app.log
# → {"bucket":"2024-01-15T10:00:00Z","count":3}
# → {"bucket":"2024-01-15T10:05:00Z","count":5}
# → {"bucket":"2024-01-15T10:10:00Z","count":2}

# 1-hour buckets
qk count by 1h app.log
# → {"bucket":"2024-01-15T10:00:00Z","count":42}

# Explicit field name (when field is not called 'ts')
qk count by 1h timestamp app.log

# Filter before bucketing
qk where level=error, count by 5m app.log
```

The timestamp can be:
- An RFC 3339 string: `"2024-01-15T10:05:30Z"`
- Unix epoch seconds (integer ≥ 1 000 000 000)
- Unix epoch milliseconds (integer ≥ 1 000 000 000 000)

Records whose timestamp field is missing or unparseable are silently skipped.

#### DSL equivalent

```bash
qk '| group_by_time(.ts, "5m")' app.log
# → same output as 'count by 5m app.log'

qk '| group_by_time(.timestamp, "1h")' events.ndjson
```

---

## Sorting (sort)

### Numeric Descending (largest first)

```bash
qk sort latency desc app.log
# → {"ts":"...","level":"info","service":"db","msg":"backup complete","latency":12000,...}
# → {"ts":"...","level":"error","service":"db","msg":"query timeout","latency":5001,...}
# → {"ts":"...","level":"info","service":"api","msg":"batch job complete","latency":4500,...}
# → (all records sorted by latency high to low)
```

### Numeric Ascending (smallest first)

```bash
qk sort latency asc app.log
# → {"ts":"...","latency":0}   (multiple records with latency=0)
# → {"ts":"...","latency":1}
# → {"ts":"...","latency":2}
# → ...
```

### Sort By String Field

```bash
qk sort service app.log
# → {"service":"api",...}
# → {"service":"api",...}
# → (sorted alphabetically by service)
```

Sorted alphabetically by service.

### Combined: Filter Then Sort

```bash
qk where level=error sort latency desc app.log
# → {"ts":"...","level":"error","service":"db","msg":"query timeout","latency":5001,...}
# → {"ts":"...","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → (errors sorted by latency descending)
```

---

## Limiting Results (limit / head)

### Take First N Records

```bash
qk limit 3 app.log
# → {"ts":"2024-01-01T10:00:00Z","level":"info",...}
# → {"ts":"2024-01-01T10:01:00Z","level":"error",...}
# → {"ts":"2024-01-01T10:02:00Z","level":"warn",...}
```

### head Is an Alias for limit

```bash
qk head 2 app.log
# → {"ts":"2024-01-01T10:00:00Z","level":"info",...}
# → {"ts":"2024-01-01T10:01:00Z","level":"error",...}
```

Identical behavior to `limit 2`.

### Combined: Sort Then Take Top N

```bash
qk sort latency desc limit 3 app.log
# → {"latency":12000,...}
# → {"latency":5001,...}
# → {"latency":4500,...}
```

---

## Numeric Aggregation (sum / avg / min / max)

### Sum

```bash
qk sum latency app.log
# → {"sum":<total of all 25 latency values>}
```

### Sum After Filtering

```bash
qk where level=error sum latency app.log
# → {"sum":<sum of latency for error records>}
```

### Average

```bash
qk avg latency app.log
# → {"avg":<average latency across all 25 records>}
```

### Average After Filtering

```bash
qk where latency gt 0 avg latency app.log
# → {"avg":<average of non-zero latency records>}
```

### Minimum

```bash
qk min latency app.log
# → {"min":0}
```

### Minimum (Excluding Zero)

```bash
qk where latency gt 0 min latency app.log
# → {"min":1}
```

The smallest non-zero latency.

### Maximum

```bash
qk max latency app.log
# → {"max":12000}
```

### Worst HTTP Response Time

```bash
qk where status gte 500 max latency access.log
# → {"max":9800}
```

The slowest 5xx response.

---

## Field Discovery (fields)

### Discover All Field Names

```bash
qk fields app.log
# → {"field":"context"}
# → {"field":"host"}
# → {"field":"latency"}
# → {"field":"level"}
# → {"field":"msg"}
# → {"field":"service"}
# → {"field":"ts"}
# → (sorted alphabetically; nested objects shown as top-level keys)
```

### Discover Fields After Filtering

```bash
qk where level=error fields app.log
# → (field names present in error records)
```

### Field Discovery on a Different File

```bash
qk fields access.log
# → {"field":"client"}
# → {"field":"latency"}
# → {"field":"method"}
# → {"field":"path"}
# → {"field":"server"}
# → {"field":"status"}
# → {"field":"ts"}
```

### Count How Many Fields Exist

```bash
qk fields app.log | qk count
# → {"count":<number of top-level fields>}
```

---

## DSL Expression Syntax

DSL mode is activated automatically when the first argument starts with `.`, `not `, or `|`.

### Equals

```bash
qk '.level == "error"' app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → (all error records)
```

### Not Equals

```bash
qk '.level != "info"' app.log
# → {"level":"error",...}
# → {"level":"warn",...}
# → {"level":"debug",...}
# → (all non-info records)
```

### Numeric Comparison

```bash
qk '.latency > 100' app.log
# → {"latency":3001,...}
# → {"latency":150,...}
# → (all records with latency > 100)
```

```bash
qk '.latency >= 88' app.log
# → records with latency 88, 120, 150, 230, 380, ... (all >= 88)
```

### Boolean Values

```bash
echo '{"service":"api","enabled":true}
{"service":"worker","enabled":false}' | qk '.enabled == true'
# → {"service":"api","enabled":true}
```

### null Comparison

```bash
echo '{"service":"api","error":null}
{"service":"web"}
{"service":"worker","error":"timeout"}' | qk '.error != null'
# → {"service":"worker","error":"timeout"}
```

Records where `error` is null or the field is absent are excluded; only records with an actual value are kept.

### Field Exists (exists)

```bash
qk '.latency exists' app.log
# → (all 25 records — every record has a latency field)
```

### Contains Substring (contains)

```bash
qk '.msg contains "timeout"' app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → {"ts":"2024-01-01T10:07:00Z","level":"error","service":"db","msg":"query timeout","latency":5001,...}
```

### Regex Match (matches)

```bash
qk '.msg matches "pan.*pointer"' app.log
# → {"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0,...}
```

### AND

```bash
qk '.level == "error" and .service == "api"' app.log
# → (all error records from service=api)
```

### OR

```bash
qk '.level == "error" or .level == "warn"' app.log
# → {"level":"error",...}
# → {"level":"warn",...}
# → (all error and warn records)
```

### NOT

```bash
qk 'not .level == "info"' app.log
# → {"level":"error",...}
# → {"level":"warn",...}
# → {"level":"debug",...}
# → (all non-info records — equivalent to != info)
```

### Compound Logic

```bash
qk '.latency > 100 and (.level == "error" or .level == "warn")' app.log
# → records where latency > 100 AND (error or warn)
```

### Nested Fields — 2 Levels Deep

```bash
# Match on a nested field
qk where response.status=503 app.log
# → {"level":"error","service":"api","msg":"upstream service unavailable","response":{"status":503,...},...}

# Word operators on nested numeric fields
qk where response.status gte 500 app.log
qk where 'response.status>=500' app.log

# Select nested fields
qk where response.status=503 select service response.status response.error app.log

# Count by nested field
qk count by response.status app.log
qk count by context.region app.log
```

### Nested Fields — 3 Levels Deep

```bash
# context.region is 2 levels; request.headers.x-trace is 3 levels
qk where context.region=us-east app.log
qk where context.env=prod, level=error app.log

# DSL — three-level access
qk '.request.headers.x-trace exists' app.log
qk '.request.headers.user-agent contains "Mozilla"' app.log

# Kubernetes logs: pod.labels.app is 3 levels deep
qk where pod.labels.app=api k8s.log
qk where pod.labels.team=platform, level=error k8s.log

# Even deeper: container info
qk where 'container.restart_count gt 2' k8s.log
qk where container.restart_count gt 2, level=warn k8s.log
```

### Nested Fields — DSL Mode

```bash
# Filter on deeply nested field, then pick only the fields you want
qk '.response.status >= 500 | pick(.ts, .service, .response.status, .response.error)' app.log

# Group by nested field
qk '| group_by(.context.region)' app.log
qk '| group_by(.response.status)' app.log

# Aggregate on nested numeric
qk '.response.status >= 200 | avg(.latency)' app.log
qk '.response.status >= 500 | max(.latency)' app.log

# Three-level access in DSL
qk '.pod.labels.app == "api" | group_by(.level)' k8s.log
qk '.pod.labels.team == "platform" and .level == "error"' k8s.log
qk '.container.restart_count > 5 | pick(.ts, .pod.name, .container.restart_count, .reason)' k8s.log
```

### No Filter (Pass All Records Through)

```bash
qk '| count()' app.log
# → {"count":25}
```

Starting with `|` skips filtering and goes directly to the pipeline stage.

---

## DSL Pipeline Stages

### pick (Keep Only Specified Fields)

```bash
qk '.level == "error" | pick(.ts, .service, .msg)' app.log
# → {"ts":"2024-01-01T10:01:00Z","service":"api","msg":"connection timeout"}
# → {"ts":"2024-01-01T10:04:00Z","service":"worker","msg":"panic: nil pointer"}
# → (all error records with only ts, service, msg)
```

`latency` is dropped.

### omit (Remove Specified Fields)

```bash
qk '.level == "error" | omit(.ts, .latency)' app.log
# → {"level":"error","service":"api","msg":"connection timeout",...}
# → {"level":"error","service":"worker","msg":"panic: nil pointer",...}
```

`ts` and `latency` are removed.

### count (Count Records)

```bash
qk '.level == "error" | count()' app.log
# → {"count":7}
```

### sort\_by (Sort Records)

```bash
qk '.latency > 0 | sort_by(.latency desc)' app.log
# → {"latency":12000,...}
# → {"latency":5001,...}
# → {"latency":4500,...}
# → (non-zero latency records, highest first)
```

```bash
qk '.latency > 0 | sort_by(.latency asc)' app.log
# → {"latency":1,...}
# → {"latency":2,...}
# → {"latency":3,...}
# → (non-zero latency records, lowest first)
```

### group\_by (Group and Count)

```bash
qk '| group_by(.level)' app.log
# → {"level":"info","count":10}
# → {"level":"error","count":7}
# → {"level":"warn","count":5}
# → {"level":"debug","count":2}
```

Sorted by count descending.

```bash
qk '.level == "error" | group_by(.service)' app.log
# → (error records grouped by service)
```

### limit (Take First N Records)

```bash
qk '.latency >= 0 | sort_by(.latency desc) | limit(3)' app.log
# → {"latency":12000,...}
# → {"latency":5001,...}
# → {"latency":4500,...}
```

Top 3 by highest latency.

### skip (Skip First N Records — Pagination)

```bash
qk '.latency >= 0 | sort_by(.latency desc) | skip(2)' app.log
# → starts from the 3rd record (skips top 2)
```

### skip + limit for Pagination

```bash
# Page 1 (records 1–3)
qk '.latency >= 0 | sort_by(.latency desc) | limit(3)' app.log
# Page 2 (records 4–6)
qk '.latency >= 0 | sort_by(.latency desc) | skip(3) | limit(3)' app.log
# Page 3 (records 7–9)
qk '.latency >= 0 | sort_by(.latency desc) | skip(6) | limit(3)' app.log
```

### dedup (Deduplicate)

```bash
qk '| dedup(.service)' app.log
# → {"service":"api",...}    (first occurrence of api)
# → {"service":"worker",...} (first occurrence of worker)
# → (one record per unique service)
```

Only the first record for each unique service value is kept.

```bash
# Count distinct service values
qk '| dedup(.service) | count()' app.log
# → {"count":<number of unique services>}
```

### sum (Sum a Field)

```bash
qk '.latency >= 0 | sum(.latency)' app.log
# → {"sum":<sum of all latency values>}
```

### avg (Average a Field)

```bash
qk '.latency > 0 | avg(.latency)' app.log
# → {"avg":<average of non-zero latency records>}
```

### min (Minimum of a Field)

```bash
qk '.latency > 0 | min(.latency)' app.log
# → {"min":1}
```

Smallest non-zero latency.

### max (Maximum of a Field)

```bash
qk '.latency > 0 | max(.latency)' app.log
# → {"max":12000}
```

### Chained Pipelines (Multi-Stage)

```bash
# Filter errors → sort by latency descending → keep key fields only
qk '.level == "error" | sort_by(.latency desc) | pick(.ts, .service, .msg, .latency)' app.log
# → {"ts":"2024-01-01T10:07:00Z","service":"db","msg":"query timeout","latency":5001}
# → {"ts":"2024-01-01T10:01:00Z","service":"api","msg":"connection timeout","latency":3001}
# → (errors sorted by latency, key fields only)
```

```bash
# All records → group by service → take top 3 groups
qk '| group_by(.service) | limit(3)' app.log
# → {"service":"api","count":9}
# → (top 3 services by record count)
```

```bash
# Filter slow requests → deduplicate (one entry per service) → keep key fields
qk '.latency > 50 | dedup(.service) | pick(.service, .latency, .msg)' app.log
# → (first slow record per service)
```

---

## qk + jq: Handling JSON-Encoded Strings

Sometimes a field's **value** is itself a JSON string (double-encoded):

```json
{"service":"api","metadata":"{\"region\":\"us-east\",\"env\":\"prod\"}","payload":"{\"level\":\"error\",\"code\":500}"}
```

qk cannot drill into a string — it sees `metadata` as a plain string. The solution is to combine qk and jq. These tools compose naturally because qk outputs NDJSON.

### Decode the nested string, then query with qk

```bash
cat > encoded.log << 'EOF'
{"service":"api","metadata":"{\"region\":\"us-east\",\"env\":\"prod\"}","payload":"{\"level\":\"error\",\"code\":500}","ts":"2024-01-01T10:01:00Z"}
{"service":"worker","metadata":"{\"region\":\"us-west\",\"env\":\"staging\"}","payload":"{\"level\":\"info\",\"code\":200}","ts":"2024-01-01T10:02:00Z"}
{"service":"web","metadata":"{\"region\":\"us-east\",\"env\":\"prod\"}","payload":"{\"level\":\"warn\",\"code\":429}","ts":"2024-01-01T10:03:00Z"}
{"service":"db","metadata":"{\"region\":\"us-east\",\"env\":\"prod\"}","payload":"{\"level\":\"error\",\"code\":503}","ts":"2024-01-01T10:04:00Z"}
EOF

# Step 1: use jq to decode the string field into a real object
# Step 2: pipe to qk to filter on the decoded field
cat encoded.log | jq -c '.payload = (.payload | fromjson)' | qk where payload.level=error
# → {"service":"api","metadata":"...","payload":{"level":"error","code":500},"ts":"..."}
# → {"service":"db","metadata":"...","payload":{"level":"error","code":503},"ts":"..."}
```

### Decode multiple string fields at once

```bash
cat encoded.log | jq -c '{service, ts, payload: (.payload | fromjson), meta: (.metadata | fromjson)}' \
  | qk where meta.env=prod, payload.level=error
# → {"service":"api","ts":"...","payload":{"level":"error","code":500},"meta":{"region":"us-east","env":"prod"}}
# → {"service":"db","ts":"...","payload":{"level":"error","code":503},"meta":{"region":"us-east","env":"prod"}}
```

### qk first, jq drills deeper

```bash
# qk does the fast filter on top-level fields, jq extracts the encoded sub-field
cat encoded.log | qk where service=api | jq -r '.payload | fromjson | .code'
# → 500
```

### Full pipeline: qk filters → jq decodes → qk aggregates

```bash
# Three-stage pipeline: qk pre-filters by service → jq decodes payload → qk counts by decoded level
cat encoded.log \
  | qk where metadata contains prod \
  | jq -c '.payload = (.payload | fromjson)' \
  | qk count by payload.level
# → {"payload.level":"error","count":2}
# → {"payload.level":"warn","count":1}
```

### When to use qk vs jq vs both

| Situation | Tool |
|-----------|------|
| Fields are real JSON objects (nested) | qk alone handles it |
| A field's **value** is a JSON-encoded string | Use `jq ... \| fromjson` to decode first, then qk |
| Fast filtering on millions of records, then decode | qk first (fast), then jq (precise) |
| Complex reshaping / math / conditionals | jq |
| Counting, aggregating, tabular output | qk |

---

## Output Formats (--fmt)

> **`--fmt` must be placed before the query expression!**
> Correct: `qk --fmt table where level=error app.log`
> Wrong: `qk where level=error --fmt table app.log`

### ndjson (Default)

```bash
qk --fmt ndjson where level=error app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001,...}
# → (all error records, one JSON object per line)
```

One JSON object per line — same as the default output.

### pretty (Indented JSON — replaces `jq .`)

```bash
qk --fmt pretty where level=error limit 1 app.log
# → {
# →   "ts": "2024-01-01T10:01:00Z",
# →   "level": "error",
# →   "service": "api",
# →   "msg": "connection timeout",
# →   "latency": 3001,
# →   ...
# → }
```

Indented format with blank lines between blocks.

### pretty + color (Pretty-Print With Semantic Color)

```bash
qk --fmt pretty --color where level=error app.log
```

In a terminal: key names are bold cyan, strings are green, numbers are yellow, null is dim.

### table (Aligned Table)

```bash
qk --fmt table where level=error select ts service msg latency app.log
# →  ts                       service  msg                              latency
# →  2024-01-01T10:01:00Z     api      connection timeout               3001
# →  2024-01-01T10:04:00Z     worker   panic: nil pointer               0
# →  (all error records in table format)
```

Auto-aligned columns with bold headers.

### table + Field Selection

```bash
qk --fmt table where level=error select ts service msg app.log
# →  ts                       service  msg
# →  2024-01-01T10:01:00Z     api      connection timeout
# →  2024-01-01T10:04:00Z     worker   panic: nil pointer
```

Only 3 columns.

### csv (Openable in Excel)

```bash
qk --fmt csv where level=error select ts service msg latency app.log
# → latency,msg,service,ts
# → 3001,connection timeout,api,2024-01-01T10:01:00Z
# → 0,panic: nil pointer,worker,2024-01-01T10:04:00Z
```

First row is the header.

### Export csv to a File

```bash
qk --fmt csv where level=error app.log > errors.csv
cat errors.csv
```

### raw (Original Lines, No Re-serialization)

```bash
qk --fmt raw where level=error app.log
# → (original text lines from the file, field order exactly as in the source)
```

The original text line from the file, with field order exactly as in the source.

### DSL + pretty

```bash
qk --fmt pretty '.level == "error" | pick(.service, .msg, .latency)' app.log
# → {
# →   "service": "api",
# →   "msg": "connection timeout",
# →   "latency": 3001
# → }
# →
# → (one pretty block per error record)
```

---

## Color Output (--color)

### Default Behavior

- **Terminal**: colors are enabled automatically
- **Pipe** (`qk ... | other`): colors are disabled automatically

### Force Colors On (Piping to less)

```bash
qk --color where level=error app.log | less -R
```

`less -R` renders ANSI color codes; `--color` forces qk to emit them even in a pipe.

### Force Colors Off

```bash
qk --no-color where level=error app.log
```

Plain text output with no color codes — suitable for writing to files or tools that don't support color.

### Disable Via Environment Variable (NO\_COLOR Standard)

```bash
NO_COLOR=1 qk where level=error app.log
```

### Priority Verification

```bash
# --no-color takes precedence over --color; output has no color
qk --no-color --color where level=error app.log
```

### Color Scheme (NDJSON Output)

| Field / Value                    | Color           |
| -------------------------------- | --------------- |
| Field names (all keys)           | Bold cyan       |
| `level: "error"` / `"fatal"`    | **Bold red**    |
| `level: "warn"`                  | **Bold yellow** |
| `level: "info"`                  | **Bold green**  |
| `level: "debug"`                 | Blue            |
| `level: "trace"`                 | Dim             |
| `msg` / `message` values         | Bright white    |
| `ts` / `timestamp` values        | Dim             |
| `error` / `exception` field values | Red           |
| HTTP `status` 200–299            | Green           |
| HTTP `status` 300–399            | Cyan            |
| HTTP `status` 400–499            | Yellow          |
| HTTP `status` 500–599            | **Bold red**    |
| Numbers (other fields)           | Yellow          |
| Booleans                         | Magenta         |
| null                             | Dim             |

---

## Multiple File Formats

`qk` detects the format automatically — no flags needed. All examples below use files from `tutorial/`.

### JSON Array (data.json)

```bash
# Each element of the JSON array becomes one record
qk data.json
# → {"id":1,"name":"Alice","age":30,"city":"New York","role":"admin",...}
# → (8 records total)

qk where role=admin data.json
# → (records where role is admin)

qk where address.country=US data.json
# → (nested field access — 2-level deep)

qk count by role data.json
# → {"role":"viewer","count":4}
# → {"role":"admin","count":3}
# → {"role":"editor","count":2} (sorted by count desc)

qk sort score desc limit 3 data.json
# → top 3 by score
```

### YAML Format — Multi-Document (services.yaml)

```bash
# Each --- document becomes one record; 6 services total
qk services.yaml
qk where status=running services.yaml
# → (services with status=running)

qk where enabled=true services.yaml
# → (enabled services only)

qk count by status services.yaml
# → {"status":"running","count":4}
# → {"status":"stopped","count":1}
# → {"status":"degraded","count":1}

qk select name status replicas services.yaml
# → {"name":"api-gateway","status":"running","replicas":3}
# → (6 records with just name/status/replicas)
```

### TOML Format (config.toml)

```bash
# Whole file = one record; nested sections accessible via dot notation
qk config.toml
# → (one record with all config values)

# Access nested section fields
qk select server.port server.workers database.pool_max config.toml
# → {"server.port":8080,"server.workers":4,"database.pool_max":50}

qk '.server.port > 8000' config.toml
# → (the record, since server.port is 8080)

qk '.logging.level == "info"' config.toml
# → (the record)
```

### CSV Format (users.csv)

```bash
# Header row becomes field names; 15 users
# Numeric columns are auto-coerced: age=30 stored as Number(30), not String("30")
# Null-like cells ("None", "null", "NA", "N/A", "") stored as null — skipped in avg/sum
qk users.csv

qk where role=admin users.csv
qk where city=New\ York users.csv     # escape the space
qk where department=Engineering users.csv
qk where score gt 90 users.csv        # works: score is Number, not String
qk where age lt 30 users.csv
qk where name startswith Al users.csv
qk where name endswith son users.csv
qk where name glob 'Al*' users.csv    # case-insensitive: Alice, Alex, Alfred...

qk count by role users.csv
# → {"role":"viewer","count":5}
# → {"role":"editor","count":5}
# → {"role":"admin","count":3} ...

qk count by department users.csv
qk sort score desc users.csv
qk sort salary desc limit 5 users.csv
qk where role=admin select name city score salary users.csv

# Statistics
qk avg score users.csv
qk max salary users.csv
qk sum salary users.csv
qk where department=Engineering avg salary users.csv
```

#### CSV Without a Header Row (--no-header)

Use `--no-header` when the CSV file has no header row. Columns are automatically named `col1`, `col2`, `col3`, etc.

> `--no-header` must come **before** the query expression (`clap trailing_var_arg` semantics).

```bash
# Example: a CSV file with no header
# (create a test file from users.csv by removing the header)
tail -n +2 users.csv > users_no_header.csv

# --no-header generates col1, col2, col3... as field names
qk --no-header users_no_header.csv
# → {"col1":"1","col2":"Alice","col3":30,"col4":"New York","col5":"admin",...}

# View first 5 rows to understand column layout
qk --no-header head 5 users_no_header.csv

# Once you know which column is which, filter by column index
qk --no-header where col5=admin users_no_header.csv      # col5 = role
qk --no-header where col4=Engineering users_no_header.csv  # col4 = department

# Numeric comparisons work (type coercion still applies)
qk --no-header where col3 lt 30 users_no_header.csv      # col3 = age

# Aggregation by column
qk --no-header count by col5 users_no_header.csv          # count by role
qk --no-header sort col8 desc limit 5 users_no_header.csv # sort by salary

# Type coercion in no-header mode
# Cells like "None", "null", "NA", "", "NaN" → stored as null (skipped in numeric ops)
# Integer-looking cells → stored as Number (supports gt/lt/avg/sum)
# "true"/"false" → stored as Bool
```

**How null-like values are handled:**

| CSV cell value | Stored as | Behavior |
|----------------|-----------|---------|
| `30`, `1000` | `Number` | Works with `gt`/`lt`/`avg`/`sum` |
| `true`, `false` | `Bool` | Works with `=true`/`=false` |
| `""`, `None`, `null`, `NA`, `N/A`, `NaN` | `null` | Skipped in numeric ops; `exists` returns false |
| `"New York"`, `"api"` | `String` | Works with `=`/`contains`/`glob` |

### TSV Format (events.tsv)

```bash
# Tab-separated; auto-detected from .tsv extension; 20 events
qk events.tsv

qk where severity=error events.tsv
qk where event=login events.tsv
qk where region=us-east events.tsv
qk where duration_ms gt 1000 events.tsv

qk count by event events.tsv
qk count by severity events.tsv
qk count by region events.tsv
qk where severity=error count by event events.tsv

qk sort duration_ms desc limit 5 events.tsv
qk avg duration_ms events.tsv
qk where severity=error avg duration_ms events.tsv
qk max duration_ms events.tsv
```

### logfmt Format (services.logfmt)

```bash
# key=value pairs common in Go services (Logrus, Zap, zerolog); 16 records
qk services.logfmt

qk where level=error services.logfmt
qk where service=api services.logfmt
qk where latency gt 1000 services.logfmt
qk where level=error, service=db services.logfmt
qk where msg contains timeout services.logfmt

qk count by level services.logfmt
# → {"level":"info","count":7}
# → {"level":"error","count":4}
# → {"level":"warn","count":4}
# → {"level":"debug","count":1}

qk where level=error select ts service msg services.logfmt
qk avg latency services.logfmt
qk sort latency desc limit 5 services.logfmt
```

### Gzip Compressed Files (app.log.gz)

```bash
# No gunzip needed — qk decompresses transparently via magic bytes detection
qk where level=error app.log.gz
# → (same error records as querying app.log directly)

qk count app.log.gz
# → {"count":25}

# Cross-check: compressed and uncompressed must match
qk count by level app.log
qk count by level app.log.gz
# → (identical output from both)
```

### Plain Text (notes.txt)

Each line of the file becomes one record with a single field: `{"line": "..."}`. Use `line` as the field name in all queries.

```bash
# View all lines
qk notes.txt
# → {"line":"2024-01-01 10:00 [INFO] api server started on port 8080"}
# → (20 records total)

# View first N lines (like head -N)
qk head 5 notes.txt
qk limit 3 notes.txt

# Count total lines
qk count notes.txt
# → {"count":20}
```

#### Substring Match (case-sensitive)

```bash
qk where line contains error notes.txt
qk where line contains timeout notes.txt
qk where line contains WARN notes.txt      # uppercase WARN only
```

#### Starts With / Ends With

```bash
qk where line startswith 2024 notes.txt
qk where line startswith ERROR notes.txt
qk where line startswith '[WARN]' notes.txt

qk where line endswith ok notes.txt
qk where line endswith done notes.txt
```

#### Shell-Style Wildcards (glob — case-insensitive, always quote)

```bash
# glob is case-insensitive: *ERROR* also matches error, Error, ERROR
qk where line glob '*ERROR*' notes.txt
qk where line glob '*warn*' notes.txt       # matches WARN, Warn, warn
qk where line glob '*timeout*' notes.txt
qk where line glob '2024*ERROR*' notes.txt  # starts with 2024 AND contains ERROR
qk where line glob '*[WARN]*' notes.txt     # literal bracket (escaped by glob_to_regex)
qk where line glob 'ERROR?*' notes.txt      # ERROR followed by any char, then anything
```

#### Regex (full pattern support — always quote)

```bash
# Quote to prevent shell glob expansion of * and ?
qk where 'line~=.*error.*' notes.txt
qk where 'line~=.*\[ERROR\].*' notes.txt     # literal brackets
qk where 'line~=(WARN|ERROR)' notes.txt      # alternation
qk where 'line~=^\d{4}-\d{2}-\d{2}' notes.txt  # lines starting with a date
qk where 'line~=(?i)error' notes.txt         # case-insensitive regex
```

#### Combining Multiple Conditions

```bash
qk where line contains error, line startswith 2024 notes.txt
# → lines that contain "error" AND start with "2024"
```

#### Full-Text Capabilities Summary

| Feature | Command | Notes |
|---------|---------|-------|
| Keyword search | `where line contains TEXT` | Case-sensitive |
| Prefix match | `where line startswith PREFIX` | Case-sensitive |
| Suffix match | `where line endswith SUFFIX` | Case-sensitive |
| Wildcard search | `where line glob '*PATTERN*'` | Case-insensitive; quote `*` |
| Regex search | `where 'line~=PATTERN'` | Always quote; use `(?i)` for case-insensitive |
| Count matching lines | `where line contains X count notes.txt` | |
| View first N lines | `head N notes.txt` | Equivalent to `head -N` |
| **Not supported** | Fuzzy search | Use regex `~=` with `(?i)` as alternative |

### Mixed-Type Fields and Type Coercion

Real-world log files often have fields where the value type varies between records — for example, a `latency` field that is normally a number but is `"None"` or `null` in some records, or a `status` field that is a number in one source and a string in another.

`tutorial/mixed.log` is designed to demonstrate this. It has 12 records with intentionally varied types:
- `latency`: mostly `Number`, but also `"None"`, `"NA"`, `"unknown"`, and `null`
- `score`: mostly `Number`, but also `null`, `"N/A"`, and `"pending"`
- `active`: mostly `Bool`, but also `"yes"` and `"no"` as strings
- `status`: always `Number`

#### Default Behavior (no --cast)

```bash
qk count mixed.log
# → {"count":12}

# Numeric aggregation automatically handles mixed values:
# - Number values → used in calculation
# - null / "None" / "NA" / "N/A" / "NaN" / "" → silently skipped (treated as null)
# - Unparseable strings like "unknown" / "pending" → skipped WITH a warning to stderr
qk avg latency mixed.log
# stdout: {"avg":1199.625}
# stderr: [qk warning] field 'latency': value "unknown" is not numeric (line 5, mixed.log) — skipped

# Filter: rows with non-numeric latency simply don't match numeric comparisons
qk where latency gt 100 mixed.log     # "None", null, "unknown" rows are excluded silently
qk where latency gt 100, count mixed.log

# The warning goes to stderr — piping to other tools is unaffected
qk avg latency mixed.log 2>/dev/null  # suppress warnings, keep only JSON output
qk avg latency mixed.log | jq '.avg'  # warning on stderr, jq processes stdout
```

**Warning rules summary:**

| Field value | In numeric ops (avg/sum/gt/lt) | Warning? |
|-------------|-------------------------------|---------|
| `3001` (Number) | used normally | no |
| `"3001"` (String that parses as number) | used normally | no |
| `null` | silently skipped | no |
| `"None"` / `"NA"` / `"N/A"` / `"NaN"` / `""` | silently skipped | no |
| `"unknown"` / `"pending"` / `"abc"` | skipped | **yes — warning to stderr** |

#### --cast: Force Type Before the Query

`--cast FIELD=TYPE` converts a field to the specified type before the query runs. Must come **before** the query expression.

**Supported types:**

| Type | Aliases | What it does |
|------|---------|-------------|
| `number` | `num`, `float`, `int`, `integer` | Parse string → Number; null-like strings → Null; other strings → warn + field removed |
| `string` | `str`, `text` | Convert to String: `200` → `"200"`, `true` → `"true"`, `null` → `"null"` |
| `bool` | `boolean` | `"true"/"1"/"yes"/"on"` → true; `"false"/"0"/"no"/"off"` → false; others → warn + removed |
| `null` | `none` | Force field to null (effectively removes it from numeric operations) |
| `auto` | | CSV-style inference: numbers, booleans, null-likes, strings |

```bash
# --cast latency=number: explicit coercion; "None"/"NA" → Null, "unknown" → warn + skip
qk --cast latency=number avg latency mixed.log
# → {"avg":1199.625}
# stderr: [qk warning] --cast latency=number: value "unknown" is not numeric (line 5) — field skipped

# --cast status=string: converts Number 200 → String "200"
# Now text operators (contains, startswith, glob) work on status
qk --cast status=string where status contains 20 mixed.log    # matches 200, 201
qk --cast status=string where status startswith 5 mixed.log   # matches 500, 503, 504
qk --cast status=string where status glob '5??' mixed.log     # 5xx codes

# --cast active=bool: coerce "yes"/"no" strings → Bool; works with =true/=false filter
qk --cast active=bool where active=true mixed.log
qk --cast active=bool count by active mixed.log

# Multiple --cast flags (each takes one FIELD=TYPE)
qk --cast latency=number --cast score=number avg latency mixed.log
qk --cast latency=number --cast score=number where latency gt 100, score gt 7.0 mixed.log

# --cast score=auto: CSV-style inference
# "N/A" → Null, "9.5" → Number(9.5), "pending" → String("pending")
qk --cast score=auto avg score mixed.log
```

#### Practical Use Cases

```bash
# Python logs where None is emitted as the string "None"
# Without --cast: avg will warn about "None" values
# With --cast: "None" → Null silently, no warning
qk --cast latency=number avg latency app.log

# Log pipeline mixing number and string status codes
qk --cast status=string count by status access.log

# CSV with a column that SHOULD be numeric but has some text sentinel values
# Use --cast to get proper numbers and warnings for bad rows
qk --cast age=number avg age users.csv

# Force a field to null to exclude it from aggregation
qk --cast score=null avg latency mixed.log  # score is ignored entirely
```

### Query Multiple Files and Formats Simultaneously

```bash
# Files processed in parallel — output merged; each file auto-detected
qk where level=error app.log k8s.log services.logfmt
qk count by level app.log k8s.log
qk where level=error count by service app.log k8s.log
```

### Glob Patterns

```bash
# Shell expands the glob; qk processes all matching files in parallel
qk where level=error *.log
qk count *.log
```

---

## Pipeline Composition

### Two qk Commands Chained

```bash
# Filter errors, then count by service
qk where level=error app.log | qk count by service
# → (error records grouped by service)
```

### Three-Stage Pipeline

```bash
# Filter → sort → limit
qk where level=error app.log | qk sort latency desc | qk limit 1
# → (the single error record with the highest latency)
```

The slowest error record.

### Combined With jq

```bash
# qk filters, jq does further processing
qk where level=error app.log | jq '.latency'
# → 3001
# → 0
# → (latency values for all error records)
```

### Combined With grep

```bash
# qk filters by format, grep does exact text matching
qk where service=api app.log | grep timeout
```

### Processing Recent Log Entries

```bash
# Process the last 1000 lines of a log file
tail -n 1000 /path/to/app.log | qk where level=error

# Process the last 500 lines and count by service
tail -n 500 /path/to/app.log | qk count by service
```

> **Known limitation:** `tail -f file | qk ...` will **block indefinitely**.
> qk reads stdin to EOF before processing. Real-time streaming (`tail -f`) is
> not yet supported. Use `tail -n <count>` for finite input instead.

---

## Common Questions

### Q: `--fmt` has no effect and output is still NDJSON?

Flags must come before the query:

```bash
# Correct
qk --fmt table where level=error app.log

# Wrong (--fmt is treated as a file name)
qk where level=error --fmt table app.log
```

### Q: Why do string comparisons in DSL require quotes?

In keyword mode the `=` operator takes a bare value; in DSL mode `==` requires JSON-style double quotes:

```bash
# Keyword mode: no quotes needed
qk where level=error app.log

# DSL mode: strings must be double-quoted
qk '.level == "error"' app.log
```

### Q: How do I filter records where a field is null?

```bash
# Field exists but its value is null
echo '{"service":"api","error":null}
{"service":"web","error":"timeout"}' | qk '.error == null'
# → {"service":"api","error":null}
```

### Q: Colors don't render in less?

```bash
qk --color where level=error app.log | less -R
```

You need both `--color` (to force qk to emit ANSI codes) and `less -R` (to render them).

### Q: How do I suppress colors when writing to a file?

```bash
qk --no-color where level=error app.log > filtered.log
```

### Q: How do I count records that match a condition?

```bash
# Method 1: keyword syntax
qk where level=error count app.log

# Method 2: DSL syntax
qk '.level == "error" | count()' app.log
```

Both produce the same output:

```bash
qk where level=error count app.log
# → {"count":7}
```

### Q: How do I use numeric operators without shell quoting issues?

Use word operators instead of symbol operators — they require no quoting:

```bash
# Symbol operators require quoting in most shells
qk where 'latency>=100' app.log
qk where 'status>=500' access.log

# Word operators are always shell-safe
qk where latency gte 100 app.log
qk where status gte 500 access.log
qk where latency gt 100 app.log      # >
qk where latency lt 100 app.log      # <
qk where latency lte 100 app.log     # <=
```

---

## Quick Reference

### Global Flags (Must Come Before the Query)

```bash
qk --fmt ndjson   # NDJSON (default)
qk --fmt pretty   # indented JSON
qk --fmt table    # aligned table
qk --fmt csv      # CSV
qk --fmt raw      # original lines
qk --color        # force colors on
qk --no-color     # force colors off
qk --no-header    # treat CSV/TSV first row as data; columns named col1, col2...
qk --explain      # print parsed query then exit
```

### Keyword Mode

```bash
# Filtering
qk where FIELD=VALUE                    # equals
qk where FIELD!=VALUE                   # not equals
qk where FIELD>N                        # numeric greater than (>=  <  <= also work)
qk where FIELD gt N                     # word operator: greater than (shell-safe)
qk where FIELD gte N                    # word operator: >= (shell-safe)
qk where FIELD lt N                     # word operator: < (shell-safe)
qk where FIELD lte N                    # word operator: <= (shell-safe)
qk where FIELD contains TEXT            # substring match (case-sensitive)
qk where FIELD startswith PREFIX        # prefix match (case-sensitive)
qk where FIELD endswith SUFFIX          # suffix match (case-sensitive)
qk where 'FIELD glob PATTERN'           # shell wildcard: * any chars, ? one char (case-insensitive)
qk where 'FIELD~=PATTERN'              # regex match (always quote!)
qk where FIELD exists                   # field presence check
qk where A=1 and B=2                    # AND
qk where A=1 or B=2                     # OR
qk where A=1, B=2                       # comma = AND (readable shorthand)
qk where A.B.C=VALUE                    # nested field (dot path)

# Field selection
qk select F1 F2 F3

# Counting
qk count                                # total count
qk count by FIELD                       # grouped count

# Aggregation
qk fields                               # discover all field names
qk sum FIELD                            # sum
qk avg FIELD                            # average
qk min FIELD                            # minimum
qk max FIELD                            # maximum

# Sorting / pagination
qk sort FIELD [asc|desc]
qk limit N
qk head N                               # alias for limit
```

### DSL Mode (First Argument Starts With `.` / `not ` / `|`)

```bash
# Filter expressions
qk '.f == "v"'                          # equals
qk '.f != "v"'                          # not equals
qk '.f > N'  '.f < N'  '.f >= N'  '.f <= N'
qk '.f exists'
qk '.f contains "text"'
qk '.f matches "regex"'
qk 'EXPR and EXPR'
qk 'EXPR or EXPR'
qk 'not EXPR'
qk '.a.b.c == 1'                        # nested field access (2+ levels)

# Pipeline stages
qk 'FILTER | pick(.f1, .f2)'           # keep only specified fields
qk 'FILTER | omit(.f1, .f2)'           # remove specified fields
qk 'FILTER | count()'                  # count records
qk 'FILTER | sort_by(.f desc)'         # sort
qk 'FILTER | group_by(.f)'             # group and count
qk 'FILTER | limit(N)'                 # first N records
qk 'FILTER | skip(N)'                  # skip N records
qk 'FILTER | dedup(.f)'                # deduplicate
qk 'FILTER | sum(.f)'                  # sum
qk 'FILTER | avg(.f)'                  # average
qk 'FILTER | min(.f)'                  # minimum
qk 'FILTER | max(.f)'                  # maximum

# Pass all records directly to pipeline (no filter)
qk '| count()'
qk '| group_by(.level)'
qk '| sort_by(.latency desc) | limit(10)'
```

### Input Formats (Auto-Detected, No Flags Required)

| Format     | Detection Criteria                                  |
| ---------- | --------------------------------------------------- |
| NDJSON     | Content starts with `{`, multiple lines             |
| JSON array | Content starts with `[`                             |
| YAML       | Starts with `---` / `.yaml` or `.yml` extension    |
| TOML       | `key = value` pattern / `.toml` extension           |
| CSV        | Comma-separated / `.csv` extension                  |
| TSV        | `.tsv` extension                                    |
| logfmt     | `key=value key=value` pattern                       |
| Gzip       | Magic bytes `0x1f 0x8b` / `.gz` (transparent decomp)|
| Plain text | Everything else                                     |

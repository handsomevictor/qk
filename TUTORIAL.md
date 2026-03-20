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
13. [Output Formats (--fmt)](#output-formats---fmt)
14. [Color Output (--color)](#color-output---color)
15. [Multiple File Formats](#multiple-file-formats)
16. [Pipeline Composition](#pipeline-composition)
17. [Common Questions](#common-questions)
18. [Quick Reference](#quick-reference)

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

All examples below use the following files. Create them first:

```bash
cat > app.log << 'EOF'
{"ts":"2024-01-01T10:00:00Z","level":"info","service":"api","msg":"server started","latency":0}
{"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
{"ts":"2024-01-01T10:02:00Z","level":"warn","service":"worker","msg":"queue depth high","latency":150}
{"ts":"2024-01-01T10:03:00Z","level":"info","service":"api","msg":"request ok","latency":42}
{"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
{"ts":"2024-01-01T10:05:00Z","level":"info","service":"web","msg":"page loaded","latency":88}
EOF
```

```bash
cat > access.log << 'EOF'
{"ts":"2024-01-01T10:00:00Z","method":"GET","path":"/api/users","status":200,"latency":42}
{"ts":"2024-01-01T10:01:00Z","method":"POST","path":"/api/login","status":401,"latency":15}
{"ts":"2024-01-01T10:02:00Z","method":"GET","path":"/api/orders","status":500,"latency":3200}
{"ts":"2024-01-01T10:03:00Z","method":"DELETE","path":"/api/cache","status":200,"latency":8}
{"ts":"2024-01-01T10:04:00Z","method":"GET","path":"/api/users","status":503,"latency":9800}
{"ts":"2024-01-01T10:05:00Z","method":"GET","path":"/health","status":200,"latency":1}
EOF
```

---

## Basic Usage

### Display All Records

```bash
qk app.log
# → {"ts":"2024-01-01T10:00:00Z","level":"info","service":"api","msg":"server started","latency":0}
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
# → {"ts":"2024-01-01T10:02:00Z","level":"warn","service":"worker","msg":"queue depth high","latency":150}
# → {"ts":"2024-01-01T10:03:00Z","level":"info","service":"api","msg":"request ok","latency":42}
# → {"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
# → {"ts":"2024-01-01T10:05:00Z","level":"info","service":"web","msg":"page loaded","latency":88}
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
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
# → {"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
```

### Not Equals (!=)

```bash
qk where level!=info app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error",...}
# → {"ts":"2024-01-01T10:02:00Z","level":"warn",...}
# → {"ts":"2024-01-01T10:04:00Z","level":"error",...}
```

3 records — all non-info entries.

### Numeric Greater Than (>)

```bash
qk where latency>100 app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
# → {"ts":"2024-01-01T10:02:00Z","level":"warn","service":"worker","msg":"queue depth high","latency":150}
```

### Numeric Less Than (<)

```bash
qk where latency<50 app.log
# → {"ts":"2024-01-01T10:00:00Z","level":"info","service":"api","msg":"server started","latency":0}
# → {"ts":"2024-01-01T10:03:00Z","level":"info","service":"api","msg":"request ok","latency":42}
# → {"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
```

### Greater Than or Equal (>=)

```bash
qk where status>=500 access.log
# → {"ts":"2024-01-01T10:02:00Z","method":"GET","path":"/api/orders","status":500,"latency":3200}
# → {"ts":"2024-01-01T10:04:00Z","method":"GET","path":"/api/users","status":503,"latency":9800}
```

### Less Than or Equal (<=)

```bash
qk where latency<=42 app.log
# → {"ts":"2024-01-01T10:00:00Z",...,"latency":0}
# → {"ts":"2024-01-01T10:03:00Z",...,"latency":42}
# → {"ts":"2024-01-01T10:04:00Z",...,"latency":0}
```

### Regex Match (~=)

```bash
qk where msg~=.*timeout.* app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
```

### Contains Substring (contains)

```bash
qk where msg contains queue app.log
# → {"ts":"2024-01-01T10:02:00Z","level":"warn","service":"worker","msg":"queue depth high","latency":150}
```

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
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
```

### OR — Multiple Conditions

```bash
qk where level=error or level=warn app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error",...}
# → {"ts":"2024-01-01T10:02:00Z","level":"warn",...}
# → {"ts":"2024-01-01T10:04:00Z","level":"error",...}
```

3 records: 2 error + 1 warn.

### Nested Field Access (dot path)

```bash
echo '{"response":{"status":503,"latency":1200},"service":"api"}
{"response":{"status":200,"latency":30},"service":"web"}' | qk where response.status=503
# → {"response":{"status":503,"latency":1200},"service":"api"}
```

---

## Field Selection (select)

### Keep Only Specified Fields

```bash
qk where level=error select ts service msg app.log
# → {"ts":"2024-01-01T10:01:00Z","service":"api","msg":"connection timeout"}
# → {"ts":"2024-01-01T10:04:00Z","service":"worker","msg":"panic: nil pointer"}
```

### Select Fields Without Filtering

```bash
qk select level msg app.log
# → {"level":"info","msg":"server started"}
# → {"level":"error","msg":"connection timeout"}
# → {"level":"warn","msg":"queue depth high"}
# → {"level":"info","msg":"request ok"}
# → {"level":"error","msg":"panic: nil pointer"}
# → {"level":"info","msg":"page loaded"}
```

All 6 records, but only `level` and `msg` are retained.

---

## Counting (count)

### Count Total Records

```bash
qk count app.log
# → {"count":6}
```

### Count After Filtering

```bash
qk where level=error count app.log
# → {"count":2}
```

### Count Grouped By Field

```bash
qk count by level app.log
# → {"level":"info","count":3}
# → {"level":"error","count":2}
# → {"level":"warn","count":1}
```

Results are sorted by count descending.

### Group By Another Field

```bash
qk count by service app.log
# → {"service":"api","count":3}
# → {"service":"worker","count":2}
# → {"service":"web","count":1}
```

### Filter Then Group

```bash
qk where latency>0 count by service app.log
# → {"service":"api","count":1}
# → {"service":"worker","count":1}
# → {"service":"web","count":1}
```

Only records with latency > 0 (3 records total, excluding latency=0 entries).

---

## Sorting (sort)

### Numeric Descending (largest first)

```bash
qk sort latency desc app.log
# → {"ts":"...","level":"error","service":"api","msg":"connection timeout","latency":3001}
# → {"ts":"...","level":"warn","service":"worker","msg":"queue depth high","latency":150}
# → {"ts":"...","level":"info","service":"web","msg":"page loaded","latency":88}
# → {"ts":"...","level":"info","service":"api","msg":"request ok","latency":42}
# → {"ts":"...","level":"info","service":"api","msg":"server started","latency":0}
# → {"ts":"...","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
```

### Numeric Ascending (smallest first)

```bash
qk sort latency asc app.log
# → {"ts":"...","latency":0}   ← two records with latency=0
# → {"ts":"...","latency":0}
# → {"ts":"...","latency":42}
# → ...
```

### Sort By String Field

```bash
qk sort service app.log
# → {"service":"api",...}
# → {"service":"api",...}
# → {"service":"api",...}
# → {"service":"web",...}
# → {"service":"worker",...}
# → {"service":"worker",...}
```

Sorted alphabetically by service.

### Combined: Filter Then Sort

```bash
qk where level=error sort latency desc app.log
# → {"ts":"...","level":"error","service":"api","msg":"connection timeout","latency":3001}
# → {"ts":"...","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
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
# → {"latency":3001,...}
# → {"latency":150,...}
# → {"latency":88,...}
```

---

## Numeric Aggregation (sum / avg / min / max)

### Sum

```bash
qk sum latency app.log
# → {"sum":3281}
```

0+3001+150+42+0+88 = 3281.

### Sum After Filtering

```bash
qk where level=error sum latency app.log
# → {"sum":3001}
```

3001+0 = 3001.

### Average

```bash
qk avg latency app.log
# → {"avg":546.833333}
```

3281 / 6 ≈ 546.83.

### Average After Filtering

```bash
qk where latency>0 avg latency app.log
# → {"avg":820.25}
```

4 records with latency > 0: (3001+150+42+88)/4 = 820.25.

### Minimum

```bash
qk min latency app.log
# → {"min":0}
```

### Minimum (Excluding Zero)

```bash
qk where latency>0 min latency app.log
# → {"min":42}
```

The smallest non-zero latency.

### Maximum

```bash
qk max latency app.log
# → {"max":3001}
```

### Worst HTTP Response Time

```bash
qk where status>=500 max latency access.log
# → {"max":9800}
```

The slowest 5xx response.

---

## Field Discovery (fields)

### Discover All Field Names

```bash
qk fields app.log
# → {"field":"latency"}
# → {"field":"level"}
# → {"field":"msg"}
# → {"field":"service"}
# → {"field":"ts"}
```

Results are sorted alphabetically.

### Discover Fields After Filtering

```bash
qk where level=error fields app.log
# → {"field":"latency"}
# → {"field":"level"}
# → {"field":"msg"}
# → {"field":"service"}
# → {"field":"ts"}
```

Same fields as the full dataset — error records have all fields.

### Field Discovery on a Different File

```bash
qk fields access.log
# → {"field":"latency"}
# → {"field":"method"}
# → {"field":"path"}
# → {"field":"status"}
# → {"field":"ts"}
```

### Count How Many Fields Exist

```bash
qk fields app.log | qk count
# → {"count":5}
```

---

## DSL Expression Syntax

DSL mode is activated automatically when the first argument starts with `.`, `not `, or `|`.

### Equals

```bash
qk '.level == "error"' app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
# → {"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
```

### Not Equals

```bash
qk '.level != "info"' app.log
# → {"level":"error",...}
# → {"level":"warn",...}
# → {"level":"error",...}
```

3 records, excluding info.

### Numeric Comparison

```bash
qk '.latency > 100' app.log
# → {"latency":3001,...}
# → {"latency":150,...}
```

```bash
qk '.latency >= 88' app.log
# → {"latency":88,...}
# → {"latency":150,...}
# → {"latency":3001,...}
```

3 records: 88, 150, 3001.

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
# → (all 6 records — every record has a latency field)
```

### Contains Substring (contains)

```bash
qk '.msg contains "timeout"' app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
```

### Regex Match (matches)

```bash
qk '.msg matches "pan.*pointer"' app.log
# → {"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
```

### AND

```bash
qk '.level == "error" and .service == "api"' app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
```

### OR

```bash
qk '.level == "error" or .level == "warn"' app.log
# → {"level":"error",...}
# → {"level":"warn",...}
# → {"level":"error",...}
```

3 records.

### NOT

```bash
qk 'not .level == "info"' app.log
# → {"level":"error",...}
# → {"level":"warn",...}
# → {"level":"error",...}
```

3 records — equivalent to `!= info`.

### Compound Logic

```bash
qk '.latency > 100 and (.level == "error" or .level == "warn")' app.log
# → {"level":"error","latency":3001,...}
# → {"level":"warn","latency":150,...}
```

2 records: latency > 100 AND (error or warn).

### Nested Fields

```bash
echo '{"request":{"method":"GET","path":"/api"},"response":{"status":500}}
{"request":{"method":"POST","path":"/login"},"response":{"status":200}}' | qk '.response.status >= 500'
# → {"request":{"method":"GET","path":"/api"},"response":{"status":500}}
```

### No Filter (Pass All Records Through)

```bash
qk '| count()' app.log
# → {"count":6}
```

Starting with `|` skips filtering and goes directly to the pipeline stage.

---

## DSL Pipeline Stages

### pick (Keep Only Specified Fields)

```bash
qk '.level == "error" | pick(.ts, .service, .msg)' app.log
# → {"ts":"2024-01-01T10:01:00Z","service":"api","msg":"connection timeout"}
# → {"ts":"2024-01-01T10:04:00Z","service":"worker","msg":"panic: nil pointer"}
```

`latency` is dropped.

### omit (Remove Specified Fields)

```bash
qk '.level == "error" | omit(.ts, .latency)' app.log
# → {"level":"error","service":"api","msg":"connection timeout"}
# → {"level":"error","service":"worker","msg":"panic: nil pointer"}
```

`ts` and `latency` are removed.

### count (Count Records)

```bash
qk '.level == "error" | count()' app.log
# → {"count":2}
```

### sort\_by (Sort Records)

```bash
qk '.latency > 0 | sort_by(.latency desc)' app.log
# → {"latency":3001,...}
# → {"latency":150,...}
# → {"latency":88,...}
# → {"latency":42,...}
```

Records with latency > 0, sorted high to low.

```bash
qk '.latency > 0 | sort_by(.latency asc)' app.log
# → {"latency":42,...}
# → {"latency":88,...}
# → {"latency":150,...}
# → {"latency":3001,...}
```

Sorted low to high.

### group\_by (Group and Count)

```bash
qk '| group_by(.level)' app.log
# → {"level":"info","count":3}
# → {"level":"error","count":2}
# → {"level":"warn","count":1}
```

Sorted by count descending.

```bash
qk '.level == "error" | group_by(.service)' app.log
# → {"service":"api","count":1}
# → {"service":"worker","count":1}
```

### limit (Take First N Records)

```bash
qk '.latency >= 0 | sort_by(.latency desc) | limit(3)' app.log
# → {"latency":3001,...}
# → {"latency":150,...}
# → {"latency":88,...}
```

Top 3 by highest latency.

### skip (Skip First N Records — Pagination)

```bash
qk '.latency >= 0 | sort_by(.latency desc) | skip(2)' app.log
# → {"latency":88,...}
# → {"latency":42,...}
# → {"latency":0,...}
# → {"latency":0,...}
```

Skips the top 2, starts from record 3.

### skip + limit for Pagination

```bash
# Page 1 (records 1–2)
qk '.latency >= 0 | sort_by(.latency desc) | limit(2)' app.log
# Page 2 (records 3–4)
qk '.latency >= 0 | sort_by(.latency desc) | skip(2) | limit(2)' app.log
# Page 3 (records 5–6)
qk '.latency >= 0 | sort_by(.latency desc) | skip(4) | limit(2)' app.log
```

Page 2 expected output:

```bash
qk '.latency >= 0 | sort_by(.latency desc) | skip(2) | limit(2)' app.log
# → {"latency":88,...}
# → {"latency":42,...}
```

### dedup (Deduplicate)

```bash
qk '| dedup(.service)' app.log
# → {"service":"api",...}    ← first occurrence of api
# → {"service":"worker",...} ← first occurrence of worker
# → {"service":"web",...}    ← first occurrence of web
```

Only the first record for each unique service value is kept.

```bash
# Count distinct service values
qk '| dedup(.service) | count()' app.log
# → {"count":3}
```

### sum (Sum a Field)

```bash
qk '.latency >= 0 | sum(.latency)' app.log
# → {"sum":3281}
```

Total of all latency values: 0+3001+150+42+0+88 = 3281.

### avg (Average a Field)

```bash
qk '.latency > 0 | avg(.latency)' app.log
# → {"avg":820.25}
```

4 non-zero latency records: (3001+150+42+88)/4 = 820.25.

### min (Minimum of a Field)

```bash
qk '.latency > 0 | min(.latency)' app.log
# → {"min":42}
```

Smallest non-zero latency.

### max (Maximum of a Field)

```bash
qk '.latency > 0 | max(.latency)' app.log
# → {"max":3001}
```

### Chained Pipelines (Multi-Stage)

```bash
# Filter errors → sort by latency descending → keep key fields only
qk '.level == "error" | sort_by(.latency desc) | pick(.ts, .service, .msg, .latency)' app.log
# → {"ts":"2024-01-01T10:01:00Z","service":"api","msg":"connection timeout","latency":3001}
# → {"ts":"2024-01-01T10:04:00Z","service":"worker","msg":"panic: nil pointer","latency":0}
```

```bash
# All records → group by service → take top 2 groups
qk '| group_by(.service) | limit(2)' app.log
# → {"service":"api","count":3}
# → {"service":"worker","count":2}
```

```bash
# Filter slow requests → deduplicate (one entry per service) → keep key fields
qk '.latency > 50 | dedup(.service) | pick(.service, .latency, .msg)' app.log
# → {"service":"api","latency":3001,"msg":"connection timeout"}
# → {"service":"worker","latency":150,"msg":"queue depth high"}
# → {"service":"web","latency":88,"msg":"page loaded"}
```

---

## Output Formats (--fmt)

> **`--fmt` must be placed before the query expression!**
> ✅ `qk --fmt table where level=error app.log`
> ❌ `qk where level=error --fmt table app.log`

### ndjson (Default)

```bash
qk --fmt ndjson where level=error app.log
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
# → {"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
```

One JSON object per line — same as the default output.

### pretty (Indented JSON — replaces `jq .`)

```bash
qk --fmt pretty where level=error app.log
# → {
# →   "ts": "2024-01-01T10:01:00Z",
# →   "level": "error",
# →   "service": "api",
# →   "msg": "connection timeout",
# →   "latency": 3001
# → }
# →
# → {
# →   "ts": "2024-01-01T10:04:00Z",
# →   "level": "error",
# →   "service": "worker",
# →   "msg": "panic: nil pointer",
# →   "latency": 0
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
qk --fmt table where level=error app.log
# →  ts                       level   service  msg                   latency
# →  2024-01-01T10:01:00Z     error   api      connection timeout    3001
# →  2024-01-01T10:04:00Z     error   worker   panic: nil pointer    0
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
qk --fmt csv where level=error app.log
# → latency,level,msg,service,ts
# → 3001,error,connection timeout,api,2024-01-01T10:01:00Z
# → 0,error,panic: nil pointer,worker,2024-01-01T10:04:00Z
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
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
# → {"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
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
# → {
# →   "service": "worker",
# →   "msg": "panic: nil pointer",
# →   "latency": 0
# → }
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

`qk` detects the format automatically — no flags needed.

### logfmt Format

```bash
cat > app.logfmt << 'EOF'
level=info service=api msg="server started" latency=0
level=error service=api msg="connection timeout" latency=3001
level=warn service=worker msg="queue depth high" latency=150
EOF

qk where level=error app.logfmt
# → {"level":"error","service":"api","msg":"connection timeout","latency":"3001"}
```

### CSV Format

```bash
cat > data.csv << 'EOF'
name,age,city
alice,30,NYC
bob,25,SF
carol,35,NYC
EOF

qk where city=NYC data.csv
# → {"name":"alice","age":"30","city":"NYC"}
# → {"name":"carol","age":"35","city":"NYC"}
```

### YAML Format (Multi-Document)

```bash
cat > services.yaml << 'EOF'
---
name: api
port: 8080
enabled: true
---
name: worker
port: 9090
enabled: false
---
name: web
port: 3000
enabled: true
EOF

qk where enabled=true services.yaml
# → {"name":"api","port":8080,"enabled":true}
# → {"name":"web","port":3000,"enabled":true}
```

2 records with enabled=true.

### TOML Format

```bash
cat > config.toml << 'EOF'
port = 8080
host = "localhost"
debug = false
max_connections = 100
EOF

qk config.toml
# → {"port":8080,"host":"localhost","debug":false,"max_connections":100}
```

The entire TOML file is treated as a single record.

```bash
qk '.port > 8000' config.toml
# → {"port":8080,"host":"localhost","debug":false,"max_connections":100}
```

### Gzip Compressed Files (Transparent Decompression)

```bash
# Compress the log first
gzip -k app.log      # creates app.log.gz, keeps the original

# Query directly — no manual decompression needed
qk where level=error app.log.gz
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
# → {"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
```

Identical output to querying `app.log`.

### Plain Text (Each Line Becomes a `line` Field)

```bash
cat > notes.txt << 'EOF'
error: connection refused at 10:01
info: server started
error: timeout after 30s
EOF

qk where line contains error notes.txt
# → {"line":"error: connection refused at 10:01"}
# → {"line":"error: timeout after 30s"}
```

### Query Multiple Files and Formats Simultaneously

```bash
qk where level=error app.log app.logfmt
```

Both files are processed in parallel and output is merged.

### Glob Patterns

```bash
qk where level=error *.log
```

The shell expands the glob; qk processes all matching files in parallel.

---

## Pipeline Composition

### Two qk Commands Chained

```bash
# Filter errors, then count by service
qk where level=error app.log | qk count by service
# → {"service":"api","count":1}
# → {"service":"worker","count":1}
```

### Three-Stage Pipeline

```bash
# Filter → sort → limit
qk where level=error app.log | qk sort latency desc | qk limit 1
# → {"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
```

The slowest error record.

### Combined With jq

```bash
# qk filters, jq does further processing
qk where level=error app.log | jq '.latency'
# → 3001
# → 0
```

### Combined With grep

```bash
# qk filters by format, grep does exact text matching
qk where service=api app.log | grep timeout
```

### Live Log Tailing (tail -f)

```bash
# Monitor errors in a live log stream (requires a real log file)
tail -f /var/log/app.log | qk where level=error
```

---

## Common Questions

### Q: `--fmt` has no effect and output is still NDJSON?

Flags must come before the query:

```bash
# ✅ Correct
qk --fmt table where level=error app.log

# ❌ Wrong (--fmt is treated as a file name)
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
# → {"count":2}
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
qk --explain      # print parsed query then exit
```

### Keyword Mode

```bash
# Filtering
qk where FIELD=VALUE                    # equals
qk where FIELD!=VALUE                   # not equals
qk where FIELD>N                        # numeric greater than (>=  <  <= also work)
qk where FIELD~=PATTERN                 # regex match
qk where FIELD contains TEXT            # substring match
qk where FIELD exists                   # field presence check
qk where A=1 and B=2                    # AND
qk where A=1 or B=2                     # OR

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
qk '.a.b.c == 1'                        # nested field access

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

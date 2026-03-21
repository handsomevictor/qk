# COMMANDS — Quick Copy-Paste Reference

All runnable commands. **No setup needed** — test files live in `tutorial/`. Just `cd tutorial` first.

```bash
git clone https://github.com/handsomevictor/qk.git
cd qk && cargo install --path .
cd tutorial      # all commands below assume this directory
```

---

## Mixed-Type Fields (Type Mismatch Handling)

When a numeric field contains non-numeric values across records, qk handles each case:

| Value in record | Behavior in `avg`/`sum`/`gt`/`lt` | Warning? |
|-----------------|-----------------------------------|---------|
| `3001` (Number) | used normally | no |
| `"3001"` (String) | auto-parsed to Number | no |
| `null` | silently skipped | no |
| `"None"` / `"NA"` / `"N/A"` / `"NaN"` / `""` | treated as null, silently skipped | no |
| `"unknown"` / `"error"` / `"abc"` | skipped — **warning printed to stderr** | **yes** |

```bash
# mixed.log has latency="None", latency="unknown", latency=null mixed with real numbers
qk avg latency mixed.log
# → {"avg":1199.625}
# stderr: [qk warning] field 'latency': value "unknown" is not numeric (line 5, mixed.log) — skipped

# null and "None" are silently skipped — no warning
qk count mixed.log    # → 12 total records
qk where latency gt 100 mixed.log   # rows with "None"/null latency are simply excluded
```

### Force Type Coercion (--cast FIELD=TYPE)

`--cast` converts a field to the specified type before the query runs. Must come **before** the query expression.

**Supported types:**

| Type | Aliases | Behavior |
|------|---------|---------|
| `number` | `num`, `float`, `int`, `integer` | Parse string → Number; null-like → Null; unparseable → **warn + field removed** |
| `string` | `str`, `text` | Convert to String: `200` → `"200"`, `true` → `"true"`, `null` → `"null"` |
| `bool` | `boolean` | Parse `"true"/"1"/"yes"/"on"` → true; `"false"/"0"/"no"/"off"` → false; others → **warn + removed** |
| `null` | `none` | Force field to null regardless of original value |
| `auto` | | CSV-style inference: numbers, booleans, null-likes, strings |

```bash
# --cast latency=number: coerce string latency to Number; warn for unparseable
qk --cast latency=number avg latency mixed.log
# → {"avg":1199.625}
# stderr: [qk warning] --cast latency=number: value "unknown" is not numeric (line 5) — field skipped

# --cast status=string: convert numeric status to String — enables text operators
qk --cast status=string where status contains 20 mixed.log    # matches 200, 201
qk --cast status=string where status startswith 5 mixed.log   # matches 500, 503, 504

# --cast active=bool: coerce "yes"/"no" strings to Bool
qk --cast active=bool count by active mixed.log

# Multiple --cast flags (each takes one FIELD=TYPE)
qk --cast latency=number --cast score=number avg latency mixed.log

# --cast score=auto: auto-infer type (same as CSV coerce_value)
# "N/A" → Null, "9.5" → 9.5, "pending" → String("pending")
qk --cast score=auto avg score mixed.log
```

---

## Verify All Formats Parse

```bash
qk count app.log          # 25 records — NDJSON, nested JSON
qk count access.log       # 20 records — NDJSON, nested client/server
qk count k8s.log          # 20 records — NDJSON, 3-level nested
qk count encoded.log      # 7  records — NDJSON with JSON-in-string values
qk count data.json        # 8  records — JSON array
qk count services.yaml    # 6  records — YAML multi-document
qk count config.toml      # 1  record  — TOML (whole file = one record)
qk count users.csv        # 15 records — CSV
qk count events.tsv       # 20 records — TSV
qk count services.logfmt  # 16 records — logfmt (key=value)
qk count notes.txt        # 20 records — plain text (each line → {"line":"..."})
qk count app.log.gz       # 25 records — transparent gzip decompression
qk count mixed.log        # 12 records — NDJSON with intentionally mixed-type fields
```

---

## Basic Usage

```bash
# Pass through all records (useful to check format and count)
qk app.log
qk data.json

# Pipe from stdin
echo '{"level":"error","msg":"oops","service":"api"}' | qk
cat app.log | qk where level=error

# Discover all field names in a file
qk fields app.log
qk fields users.csv
qk fields k8s.log

# Explain what qk parsed (debug mode)
qk --explain where level=error app.log
qk --explain where latency gt 100 app.log
```

---

## Filtering (where)

### Equality

```bash
qk where level=error app.log
qk where level!=info app.log
qk where service=api app.log
qk where method=POST access.log
qk where role=admin users.csv
qk where severity=error events.tsv
```

### Numeric Comparisons (word operators — shell-safe, no quoting)

```bash
# Word operators: gt lt gte lte (no shell quoting ever needed)
qk where latency gt 1000 app.log
qk where latency lt 100 app.log
qk where latency gte 3001 app.log
qk where latency lte 50 app.log
qk where status gte 500 access.log
qk where status lt 400 access.log
qk where score gt 90 users.csv
qk where age gte 35 users.csv
qk where duration_ms gt 1000 events.tsv

# Alternative: quote the embedded operators
qk where 'latency>1000' app.log
qk where 'status>=500' access.log
qk where 'score<80' users.csv
```

### Regex Match (always quote to prevent shell glob expansion)

```bash
# NOTE: * is a glob in zsh/bash — always quote regex patterns
qk where 'msg~=.*timeout.*' app.log
qk where 'msg~=.*panic.*' app.log
qk where 'reason~=.*failed.*' k8s.log
qk where 'path~=/api/.*' access.log
qk where 'name~=.*admin.*' users.csv
```

### Substring Match (contains)

```bash
qk where msg contains timeout app.log
qk where msg contains panic app.log
qk where reason contains failed k8s.log
qk where path contains /api/ access.log
qk where name contains ar users.csv
qk where line contains error notes.txt
```

### Starts With (startswith)

```bash
qk where msg startswith connection app.log
qk where msg startswith queue app.log
qk where path startswith /api/ access.log
qk where path startswith /health access.log
qk where name startswith Al users.csv
qk where line startswith 2024 notes.txt
qk where line startswith ERROR notes.txt
```

### Ends With (endswith)

```bash
qk where path endswith users access.log
qk where path endswith orders access.log
qk where msg endswith timeout app.log
qk where msg endswith pointer app.log
qk where name endswith son users.csv
qk where line endswith ok notes.txt
```

### Shell-Style Wildcards (glob — always quote to prevent shell expansion)

```bash
# NOTE: * and ? are shell metacharacters — always quote glob patterns
# Glob is case-insensitive by default
qk where msg glob '*timeout*' app.log
qk where msg glob '*panic*' app.log
qk where path glob '/api/*' access.log
qk where name glob 'Al*' users.csv     # matches Alice, Alex, Alfred...
qk where name glob '*son' users.csv    # matches Jackson, Wilson...
qk where name glob 'A*n' users.csv    # starts with A, ends with n
qk where line glob '*ERROR*' notes.txt
qk where line glob '*warn*' notes.txt  # case-insensitive: matches WARN, Warn, warn
```

### Field Existence

```bash
qk where request exists app.log
qk where response.error exists app.log
qk where metrics exists app.log
qk where user exists app.log
qk where probe exists k8s.log
```

### Multi-Condition — Comma Style (readable AND)

```bash
# Comma = alias for 'and'
qk where level=error, service=api app.log
qk where level=error, latency gt 1000 app.log
qk where level=error, service=api, latency gt 1000 app.log
qk where status gte 500, method=GET access.log
qk where severity=error, region=us-east events.tsv
qk where role=admin, active=true users.csv
```

### Multi-Condition — Explicit and/or

```bash
qk where level=error and service=api app.log
qk where level=error or level=warn app.log
qk where status gte 500 and method=GET access.log
qk where level=error and service=db and latency gt 3000 app.log
```

---

## Nested JSON — 2 Levels

```bash
# app.log has: context.region, context.env, request.method, request.path, response.status
qk where context.region=us-east app.log
qk where context.env=prod app.log
qk where response.status=504 app.log
qk where response.status gte 500 app.log
qk where request.method=POST app.log
qk where request.path contains /api/ app.log

# access.log has: client.ip, client.country, server.host, server.region
qk where client.country=US access.log
qk where server.region=us-east access.log
qk where client.country!=US access.log
qk where server.host=web-01 access.log

# services.yaml has: resources.cpu, healthcheck.path
qk where status=running services.yaml
qk where enabled=true services.yaml

# data.json has: address.country, address.zip
qk where address.country=US data.json
qk where city=New\ York data.json
```

### Multi-condition on nested fields

```bash
qk where response.status gte 500, service=api app.log
qk where client.country=US, status gte 500 access.log
qk where context.env=prod, level=error app.log
```

---

## Nested JSON — 3 Levels

```bash
# k8s.log has: pod.labels.app, pod.labels.team, pod.labels.version
qk where pod.labels.app=api k8s.log
qk where pod.labels.team=platform k8s.log
qk where pod.labels.team=infra k8s.log
qk where pod.namespace=production k8s.log
qk where container.restart_count gt 0 k8s.log

# app.log has: request.headers.x-trace (3 levels)
qk where request.headers.x-trace exists app.log

# Combine 3-level nested with other conditions
qk where pod.labels.app=api, level=error k8s.log
qk where pod.labels.team=infra, level=warn k8s.log
qk where container.restart_count gte 3, pod.namespace=production k8s.log
```

---

## Select (Projection)

```bash
# Comma after the last filter condition is optional but allowed — both styles work:
qk where level=error select ts service msg app.log
qk where level=error, select ts service msg app.log   # trailing comma style

# More examples
qk where level=error, select ts msg latency app.log
qk where status gte 500, select ts method path status access.log
qk where pod.labels.app=api, select ts msg reason k8s.log
qk where role=admin, select name city score users.csv
qk where severity=error, select ts event region duration_ms events.tsv
qk select name role city users.csv
qk select ts event severity duration_ms events.tsv
qk select ts level service msg latency app.log
```

---

## Count and Aggregation

### Count

```bash
qk count app.log
qk where level=error, count app.log
qk count by level app.log
qk count by service app.log
qk count by method access.log
qk count by status access.log
qk count by severity events.tsv
qk count by event events.tsv
qk count by role users.csv
qk count by city users.csv
qk count by level k8s.log
qk count by pod.labels.team k8s.log
qk count by pod.labels.app k8s.log
qk where level=error, count by service app.log
qk where level=error, service=api, count by host app.log
qk where status gte 500, count by method access.log
qk where severity=error, count by event events.tsv
```

### Count by Time Bucket

Group events into time buckets using a duration suffix (`s`, `m`, `h`, `d`).
The timestamp field defaults to `ts`; override with an explicit field name.

```bash
# Count by 5-minute buckets (reads 'ts' field automatically)
qk count by 5m app.log

# Count by 1-hour buckets
qk count by 1h app.log

# Count by 1-day buckets
qk count by 1d app.log

# Explicit timestamp field name
qk count by 1h timestamp app.log

# Filter then bucket
qk where level=error, count by 5m app.log

# DSL equivalent
qk '.[] | group_by_time(.ts, "5m")' app.log
qk '.[] | group_by_time(.timestamp, "1h")' app.log
```

Output format (one record per bucket):
```json
{"bucket":"2024-01-15T10:00:00Z","count":42}
{"bucket":"2024-01-15T10:05:00Z","count":17}
```

### Sum / Avg / Min / Max

```bash
# Sum
qk sum latency app.log
qk where level=error, sum latency app.log
qk where service=api, sum latency app.log
qk sum duration_ms events.tsv
qk sum salary users.csv

# Average
qk avg latency app.log
qk where level=error, avg latency app.log
qk where service=db, avg latency app.log
qk avg score users.csv
qk where severity=error, avg duration_ms events.tsv

# Min / Max
qk min latency app.log
qk max latency app.log
qk where service=api, min latency app.log
qk where service=api, max latency app.log
qk min score users.csv
qk max score users.csv
qk where department=Engineering, max salary users.csv
qk min status access.log
qk max status access.log
```

---

## Sort and Limit

```bash
# Sort
qk sort latency desc app.log
qk sort latency asc app.log
qk sort ts desc app.log
qk sort score desc users.csv
qk sort age asc users.csv
qk sort duration_ms desc events.tsv
qk where level=error, sort latency desc app.log
qk where service=api, sort latency desc app.log
qk where severity=error, sort duration_ms desc events.tsv

# Limit / head (aliases)
qk limit 5 app.log
qk head 5 app.log
qk sort latency desc limit 3 app.log
qk sort latency desc head 5 access.log
qk where level=error, sort latency desc limit 1 app.log
qk where level=error, sort latency desc limit 5 app.log
qk where status gte 500, sort latency desc limit 3 access.log
qk where score gt 80, sort score desc limit 5 users.csv

# Skip (DSL only — for pagination)
qk '| skip(5) | limit(5)' app.log    # records 6-10
```

---

## DSL Expression Layer

DSL mode activates when the first argument starts with `.`, `not `, or `|`.

### Filter Expressions

```bash
# Equality
qk '.level == "error"' app.log
qk '.service == "api"' app.log
qk '.method == "POST"' access.log
qk '.role == "admin"' users.csv

# Not equal
qk '.level != "info"' app.log

# Numeric comparisons (DSL: quote the whole expression, not the operator)
qk '.latency > 1000' app.log
qk '.latency < 100' app.log
qk '.status >= 500' access.log
qk '.score > 90' users.csv
qk '.age <= 30' users.csv

# Nested field access
qk '.response.status >= 500' app.log
qk '.client.country == "US"' access.log
qk '.pod.labels.app == "api"' k8s.log
qk '.pod.labels.team == "infra"' k8s.log
qk '.address.country == "US"' data.json

# Substring and regex
qk '.msg contains "timeout"' app.log
qk '.msg matches ".*panic.*"' app.log
qk '.reason contains "failed"' k8s.log

# Field existence
qk '.request exists' app.log
qk '.probe exists' k8s.log

# Boolean logic
qk '.level == "error" and .latency > 1000' app.log
qk '.level == "error" or .level == "warn"' app.log
qk 'not .level == "info"' app.log
qk '.status >= 500 and .method == "GET"' access.log
qk '.pod.labels.app == "api" and .level == "error"' k8s.log
```

### Pipeline Stages

```bash
# pick — keep only specified fields
qk '.level == "error" | pick(.ts, .service, .msg)' app.log
qk '.status >= 500 | pick(.ts, .method, .path, .status)' access.log
qk '| pick(.name, .role, .score)' users.csv

# omit — remove specified fields
qk '.level == "error" | omit(.host, .context)' app.log
qk '| omit(.address)' data.json

# count
qk '.level == "error" | count()' app.log
qk '| count()' users.csv

# sort_by
qk '| sort_by(.latency desc)' app.log
qk '| sort_by(.score desc)' users.csv
qk '| sort_by(.age asc)' users.csv

# group_by — groups and counts
qk '| group_by(.level)' app.log
qk '| group_by(.service)' app.log
qk '| group_by(.method)' access.log
qk '| group_by(.pod.labels.team)' k8s.log
qk '| group_by(.role)' users.csv

# limit and skip
qk '| limit(5)' app.log
qk '| skip(10) | limit(5)' app.log   # pagination: page 3 of 5

# dedup — keep first occurrence of each unique value
qk '| dedup(.service)' app.log
qk '| dedup(.role)' users.csv
qk '| dedup(.event)' events.tsv

# sum / avg / min / max
qk '| sum(.latency)' app.log
qk '.level == "error" | sum(.latency)' app.log
qk '| avg(.latency)' app.log
qk '| min(.latency)' app.log
qk '| max(.latency)' app.log
qk '| avg(.score)' users.csv
qk '| max(.score)' users.csv
```

### Chained Pipeline

```bash
qk '.level == "error" | pick(.ts, .service, .msg) | sort_by(.ts desc)' app.log
qk '.response.status >= 500 | pick(.ts, .service, .response.status) | limit(5)' app.log
qk '.status >= 500 | pick(.method, .path, .status) | group_by(.method)' access.log
qk '.pod.labels.team == "platform" | pick(.ts, .msg, .level) | sort_by(.ts asc)' k8s.log
```

### Pipeline-Only (no filter)

```bash
qk '| group_by(.level)' app.log
qk '| sort_by(.latency desc)' app.log
qk '| sort_by(.score desc) | limit(5)' users.csv
qk '| group_by(.pod.labels.team)' k8s.log
qk '| group_by(.country)' access.log
```

---

## Format-Specific Commands

### NDJSON (app.log, access.log, k8s.log) — default format

```bash
qk where level=error app.log
qk where level=error, service=api app.log
qk where level=error, service=api, latency gt 1000 app.log
qk where level=error, select ts service msg app.log
qk where level=error, select ts service msg latency app.log
qk where level=error, count by service app.log
qk where level=error, sort latency desc limit 5 app.log
qk where level=error, avg latency app.log
qk where response.status gte 500 app.log
qk where response.status gte 500, service=api app.log
qk '.level == "error" | pick(.ts, .service, .msg, .latency)' app.log
qk count by service app.log
qk avg latency app.log
```

### JSON Array (data.json)

```bash
# Auto-detected from [ prefix — each array element becomes a record
qk data.json
qk where role=admin data.json
qk where city=New\ York data.json
qk where active=true data.json
qk where score gt 80 data.json
qk where address.country=US data.json
qk where role=admin, active=true data.json
qk where role=admin, score gt 90 data.json
qk where role=admin, select name city score data.json
qk where score gt 80, sort score desc data.json
qk where active=true, count by role data.json
qk where active=true, avg score data.json
qk count by role data.json
qk count by city data.json
qk sort score desc limit 3 data.json
qk avg score data.json
qk max score data.json
```

### YAML Multi-Document (services.yaml)

```bash
# Each --- document becomes a record
qk services.yaml
qk where status=running services.yaml
qk where enabled=true services.yaml
qk where status=degraded services.yaml
qk where env=production services.yaml
qk where status=running, enabled=true services.yaml
qk where env=production, status=running services.yaml
qk where enabled=true, select name port replicas services.yaml
qk where status=running, count by env services.yaml
qk count by status services.yaml
qk select name status replicas services.yaml
```

### TOML (config.toml)

```bash
# Whole file = one record; access nested sections with dot notation
qk config.toml
qk select server.port server.workers database.pool_max config.toml
qk '.server.port > 8000' config.toml
qk '.logging.level == "info"' config.toml
qk '.feature_flags.enable_new_dashboard == true' config.toml
```

### CSV (users.csv)

```bash
# Header row becomes field names; numeric values auto-coerced (30 → Number, not String)
qk users.csv
qk where role=admin users.csv
qk where city=New\ York users.csv
qk where active=true users.csv
qk where department=Engineering users.csv
qk where score gt 80 users.csv
qk where age lt 30 users.csv
qk where name startswith Al users.csv
qk where name endswith son users.csv
qk where name glob 'Al*' users.csv
qk where role=admin, department=Engineering users.csv
qk where active=true, score gt 80 users.csv
qk where active=true, age lt 30 users.csv

# CSV without a header row — use --no-header; columns become col1, col2, col3...
# --no-header must come BEFORE the query expression (clap trailing_var_arg semantics)
qk --no-header users_no_header.csv
qk --no-header head 5 users_no_header.csv
qk --no-header where col3=Engineering users_no_header.csv
qk --no-header count by col4 users_no_header.csv
qk --no-header sort col6 desc limit 5 users_no_header.csv
qk where role=admin, select name city score salary users.csv
qk where department=Engineering, sort salary desc users.csv
qk where active=true, count by role users.csv
qk where active=true, count by department users.csv
qk where department=Engineering, avg salary users.csv
qk where role=admin, max salary users.csv
qk count by role users.csv
qk count by city users.csv
qk count by department users.csv
qk sort score desc users.csv
qk sort salary desc limit 5 users.csv
qk avg score users.csv
qk max salary users.csv
qk sum salary users.csv
```

### TSV (events.tsv)

```bash
# Tab-separated; auto-detected from .tsv extension
qk events.tsv
qk where severity=error events.tsv
qk where event=login events.tsv
qk where region=us-east events.tsv
qk where duration_ms gt 1000 events.tsv
qk where severity=error, region=us-east events.tsv
qk where event=login, region=us-east events.tsv
qk where severity=error, select ts event service region events.tsv
qk where severity=error, count by event events.tsv
qk where severity=error, sort duration_ms desc limit 3 events.tsv
qk where severity=error, avg duration_ms events.tsv
qk count by event events.tsv
qk count by severity events.tsv
qk count by region events.tsv
qk sort duration_ms desc limit 5 events.tsv
qk avg duration_ms events.tsv
qk max duration_ms events.tsv
```

### logfmt (services.logfmt)

```bash
# key=value pairs; common in Go services (Logrus, Zap)
qk services.logfmt
qk where level=error services.logfmt
qk where service=api services.logfmt
qk where latency gt 1000 services.logfmt
qk where level=error, service=db services.logfmt
qk where level=error, service=api services.logfmt
qk where level=error, latency gt 1000 services.logfmt
qk where level=error, select ts service msg services.logfmt
qk where level=error, count by service services.logfmt
qk where level=error, sort latency desc services.logfmt
qk where level=error, avg latency services.logfmt
qk where service=api, sort latency desc limit 3 services.logfmt
qk count by level services.logfmt
qk count by service services.logfmt
qk avg latency services.logfmt
qk max latency services.logfmt
qk sort latency desc limit 5 services.logfmt
```

### Gzip (app.log.gz)

```bash
# Transparent decompression — no gunzip needed
qk app.log.gz
qk count app.log.gz
qk where level=error app.log.gz
qk where level=error, service=api app.log.gz
qk where level=error, select ts service msg app.log.gz
qk where level=error, count by service app.log.gz
qk where latency gt 1000 app.log.gz
qk count by service app.log.gz
qk avg latency app.log.gz

# Same query across compressed and uncompressed — results must match
qk count by level app.log
qk count by level app.log.gz
```

### Plain Text (notes.txt)

```bash
# Each line → {"line": "..."} — use 'line' as the field name
qk notes.txt
qk head 5 notes.txt
qk count notes.txt

# Exact substring match
qk where line contains error notes.txt
qk where line contains timeout notes.txt
qk where line contains WARN notes.txt

# Starts with / ends with
qk where line startswith 2024 notes.txt
qk where line startswith ERROR notes.txt
qk where line endswith ok notes.txt
qk where line endswith done notes.txt

# Shell-style wildcards (case-insensitive, always quote)
qk where line glob '*ERROR*' notes.txt
qk where line glob '*warn*' notes.txt     # matches WARN, Warn, warn
qk where line glob '*timeout*' notes.txt
qk where line glob '2024*ERROR*' notes.txt  # starts with 2024 and contains ERROR

# Regex (always quote to prevent shell glob expansion)
qk where 'line~=.*error.*' notes.txt
qk where 'line~=.*\[ERROR\].*' notes.txt
qk where 'line~=(WARN|ERROR)' notes.txt

# Combine with grep for additional text patterns
qk notes.txt | grep -i error
```

---

## Output Formats

```bash
# --fmt must come BEFORE the query expression
qk --fmt ndjson where level=error app.log    # NDJSON (default)
qk --fmt pretty where level=error app.log    # indented JSON with blank lines
qk --fmt table where level=error app.log     # aligned table (like psql)
qk --fmt csv where level=error app.log       # CSV (openable in Excel)
qk --fmt raw where level=error app.log       # original source line unchanged

# Pretty-print all fields
qk --fmt pretty data.json
qk --fmt pretty services.yaml
qk --fmt pretty config.toml

# Table output for comparisons
qk --fmt table count by level app.log
qk --fmt table count by service app.log
qk --fmt table sort score desc users.csv
qk --fmt table where level=error select ts service msg latency app.log

# CSV output for Excel / Google Sheets
qk --fmt csv users.csv                      # re-export filtered CSV
qk --fmt csv where level=error app.log      # export errors to CSV
qk --fmt csv sort salary desc users.csv
```

---

## Color Control

```bash
qk --color where level=error app.log         # force ANSI color on
qk --no-color where level=error app.log      # force color off (for piping)

# Color is auto-enabled in a terminal, auto-disabled when piping
qk where level=error app.log | cat           # piped — no color
qk where level=error app.log | qk count by service  # piped — no color
```

---

## Multiple Files

```bash
# Query across multiple files at once (processed in parallel)
qk where level=error app.log access.log k8s.log
qk count by level app.log k8s.log services.logfmt
qk where level=error count by service app.log k8s.log

# Glob patterns (quote to prevent shell expansion if needed)
qk where level=error *.log
qk count *.log
```

---

## qk + jq: Handling JSON-Encoded String Fields

`encoded.log` has fields where the **value is itself a JSON string** — a common pattern in some log pipelines.

```bash
# Inspect the raw data first
qk encoded.log

# Decode one field, then filter with qk
cat encoded.log | jq -c '.payload = (.payload | fromjson)' | qk where payload.level=error

# Decode both fields, filter on decoded content
cat encoded.log | jq -c '{service, ts, payload: (.payload | fromjson), meta: (.metadata | fromjson)}' \
  | qk where meta.env=prod, payload.level=error

# qk pre-filter → jq decodes → qk aggregates
cat encoded.log | qk where service=api \
  | jq -c '.payload = (.payload | fromjson)' \
  | qk count by payload.level

# Extract a single field from the decoded payload
cat encoded.log | qk where service=api | jq -r '.payload | fromjson | .msg'

# Full pipeline: qk filter → jq decode → qk count by decoded field
cat encoded.log | jq -c '.payload = (.payload | fromjson)' | qk count by payload.level
```

---

## Pipeline Composition

```bash
# Two qk commands chained
qk where level=error app.log | qk count by service
qk sort latency desc app.log | qk limit 5

# Three stages
qk where level=error app.log | qk sort latency desc | qk limit 1

# With jq
qk where level=error app.log | jq '.latency'
qk where level=error app.log | jq '{service: .service, ms: .latency}'
qk where level=error app.log | jq -s 'map(.latency) | add'

# With grep (for text patterns not expressible in qk)
qk where service=api app.log | grep timeout

# With sort and uniq (for field values qk doesn't know about)
qk where level=error app.log | jq -r '.service' | sort | uniq -c | sort -rn

# Process the last 1000 lines of a log file
tail -n 1000 /path/to/app.log | qk where level=error

# NOTE: tail -f is not yet supported. qk reads stdin to EOF before processing.
# `tail -f file | qk ...` will block indefinitely. Use tail -n instead.
```

---

## Quick Syntax Reminder

```
qk [--fmt FORMAT] [--color|--no-color] [--no-header] [--explain] QUERY [FILES...]

Fast layer:
  where FIELD=VALUE              exact match
  where FIELD!=VALUE             not equal
  where FIELD gt/lt/gte/lte N   numeric comparison (shell-safe)
  where FIELD contains TEXT      substring
  where FIELD startswith PREFIX  starts with
  where FIELD endswith SUFFIX    ends with
  where 'FIELD glob PATTERN'     shell wildcard (* ? — always quote!)
  where 'FIELD~=PATTERN'         regex (always quote!)
  where FIELD exists             field presence
  where A=1, B=2                 comma = and
  select F1 F2 ...               projection
  count / count by FIELD         count
  fields                         discover field names
  sum/avg/min/max FIELD          statistics
  sort FIELD asc|desc            sort
  limit N / head N               take first N

Flags:
  --no-header                    treat CSV/TSV first row as data, not header
                                 columns named col1, col2, col3 ...
  --cast FIELD=TYPE              coerce a field to a type before the query runs
                                 types: number(num/float/int) string(str/text) bool(boolean) null(none) auto
                                 can be repeated: --cast f1=number --cast f2=string

DSL layer (first arg starts with . not | ):
  '.field == "val" | pick(.a, .b) | sort_by(.f desc) | limit(N)'
  stages: pick omit count() sort_by() group_by() limit() skip() dedup() sum() avg() min() max()
```

# COMMANDS — Quick Copy-Paste Reference

All runnable commands. **No setup needed** — test files live in `tutorial/`. Just `cd tutorial` first.

```bash
git clone https://github.com/handsomevictor/qk.git
cd qk && cargo install --path .
cd tutorial      # all commands below assume this directory
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
# Keep only specified fields
qk where level=error select ts service msg app.log
qk where level=error select ts msg latency app.log
qk where status gte 500 select ts method path status access.log
qk where pod.labels.app=api select ts msg reason k8s.log
qk select name role city users.csv
qk select ts event severity duration_ms events.tsv
qk select ts level service msg latency app.log
```

---

## Count and Aggregation

### Count

```bash
qk count app.log
qk where level=error count app.log
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
qk where level=error count by service app.log
qk where status gte 500 count by method access.log
```

### Sum / Avg / Min / Max

```bash
# Sum
qk sum latency app.log
qk where level=error sum latency app.log
qk sum latency access.log
qk sum duration_ms events.tsv
qk sum salary users.csv

# Average
qk avg latency app.log
qk where level=error avg latency app.log
qk avg latency access.log
qk avg score users.csv
qk avg duration_ms events.tsv

# Min / Max
qk min latency app.log
qk max latency app.log
qk min score users.csv
qk max score users.csv
qk min age users.csv
qk max age users.csv
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

# Limit / head (aliases)
qk limit 5 app.log
qk head 5 app.log
qk sort latency desc limit 3 app.log
qk sort latency desc head 5 access.log
qk where level=error sort latency desc limit 1 app.log

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
qk where response.status gte 500 app.log
qk '.level == "error" | pick(.ts, .service, .msg, .latency)' app.log
qk count by service app.log
qk avg latency app.log
qk where level=error avg latency app.log
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
qk count by role data.json
qk count by city data.json
qk sort score desc data.json
qk sort score desc limit 3 data.json
qk where role=admin select name city score data.json
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
qk count by status services.yaml
qk select name status replicas services.yaml
qk where enabled=true select name port replicas services.yaml
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
# Header row becomes field names; all values start as strings
qk users.csv
qk where role=admin users.csv
qk where city=New\ York users.csv
qk where active=true users.csv
qk where department=Engineering users.csv
qk where score gt 80 users.csv
qk where age lt 30 users.csv
qk count by role users.csv
qk count by city users.csv
qk count by department users.csv
qk sort score desc users.csv
qk sort salary desc limit 5 users.csv
qk where role=admin select name city score salary users.csv
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
qk count by event events.tsv
qk count by severity events.tsv
qk count by region events.tsv
qk where severity=error count by event events.tsv
qk sort duration_ms desc events.tsv
qk sort duration_ms desc limit 5 events.tsv
qk avg duration_ms events.tsv
qk where severity=error avg duration_ms events.tsv
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
qk count by level services.logfmt
qk count by service services.logfmt
qk where level=error select ts service msg services.logfmt
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
qk where line contains error notes.txt
qk where line contains timeout notes.txt
qk where line contains WARN notes.txt
qk count notes.txt

# Combine with grep for complex text patterns
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

# Live log tailing (replace with your actual log file path)
tail -f /path/to/app.log | qk where level=error
tail -f /path/to/app.log | qk where level=error | qk count by service
```

---

## Quick Syntax Reminder

```
qk [--fmt FORMAT] [--color|--no-color] [--explain] QUERY [FILES...]

Fast layer:
  where FIELD=VALUE           exact match
  where FIELD!=VALUE          not equal
  where FIELD gt/lt/gte/lte N numeric comparison (shell-safe)
  where FIELD contains TEXT   substring
  where 'FIELD~=PATTERN'      regex (always quote!)
  where FIELD exists          field presence
  where A=1, B=2              comma = and
  select F1 F2 ...            projection
  count / count by FIELD      count
  fields                      discover field names
  sum/avg/min/max FIELD        statistics
  sort FIELD asc|desc         sort
  limit N / head N            take first N

DSL layer (first arg starts with . not | ):
  '.field == "val" | pick(.a, .b) | sort_by(.f desc) | limit(N)'
  stages: pick omit count() sort_by() group_by() limit() skip() dedup() sum() avg() min() max()
```

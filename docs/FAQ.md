# FAQ — Frequently Asked Questions

## General

### What is qk?

`qk` is a single CLI tool that replaces `grep`, `awk`, `sed`, `jq`, `yq`, `cut`, `sort | uniq` for structured log and data files. It auto-detects formats (NDJSON, JSON, CSV, TSV, logfmt, YAML, TOML, plaintext, gzip) and provides a fast keyword query syntax plus a DSL expression syntax.

### Which file formats are supported?

| Format | Auto-detected? | Notes |
|--------|---------------|-------|
| NDJSON | Yes | One JSON object per line |
| JSON   | Yes | Single array or object |
| CSV    | Yes | Header row auto-inferred |
| TSV    | Yes | Tab-separated, header auto-inferred |
| logfmt | Yes | `key=value key="value with spaces"` |
| YAML   | Yes | Single document |
| TOML   | Yes | Entire file as one record |
| Gzip   | Yes | Transparent decompression |
| Plaintext | Yes | Each line becomes `{"line": "..."}` |

---

## Queries

### How do I debug a query that matches nothing?

1. Use `--explain` to see how qk parsed the query:
   ```
   qk --explain where level=error app.log
   ```

2. Check the field name is correct — use `fields` to list all field names:
   ```
   qk fields app.log
   ```

3. Check value capitalisation — `qk` string filters (`=`, `!=`, `contains`,
   `startswith`, `endswith`) are **case-insensitive by default**, so
   `where level=ERROR` also matches `"error"` and `"Error"`.
   If you need an exact case match, add `--case-sensitive` / `-S`:
   ```
   qk -S where level=Error app.log
   ```

4. For regex matching:
   ```
   qk where msg ~= "(?i)error" app.log
   ```

### How do I process a JSON string that itself contains nested JSON?

Combine `qk` with `jq`:
```bash
qk where level=error app.log | jq '.payload | fromjson | .user_id'
```

`qk` does the fast filtering; `jq` handles the deeply-nested transformation.

### How do I query across multiple files?

Pass all file paths as arguments:
```bash
qk where level=error *.log
qk where status>499 /var/log/nginx/access.*.log
```

All records from all files are merged before the query runs. Files are read
in parallel (rayon).

### Why does `>` / `<` not work in the shell?

Shell metacharacters need quoting:
```bash
qk where 'latency>100' app.log      # single-quote the whole token
qk where latency gt 100 app.log     # or use word operators
```

Word operators: `eq`, `ne`, `gt`, `lt`, `gte`, `lte`, `contains`,
`startswith`, `endswith`, `exists`, `between`, `glob`.

---

## Large Files

### How do I handle very large files (>1 GB)?

qk uses `mmap` for files ≥ 64 KiB, which avoids reading the entire file into
memory at once on most platforms.

For files too large to fit in RAM:
- Use streaming mode: pipe a tool that outputs records line by line into qk:
  ```bash
  tail -n 1000000 huge.ndjson | qk where level=error
  ```
- Split the file first:
  ```bash
  split -l 1000000 huge.ndjson part_
  qk where level=error part_* | qk count
  ```

> Known limitation: full-file materialisation is still required before
> aggregation (`count by`, `sum`, `avg`, etc.). Streaming mode applies only
> to pure filter + select + limit queries.

### What happens to corrupt / malformed lines?

Corrupt lines are skipped with a warning printed to stderr — processing
continues with the next line. Output records are never mixed with warnings
(warnings always go to stderr).

```
[qk warning] failed to parse field 'ts' at line 42: ...
```

To suppress warnings, redirect stderr:
```bash
qk where level=error app.log 2>/dev/null
```

---

## Configuration & Defaults

### How do I set a default output format?

Create `~/.config/qk/config.toml`:
```toml
default_fmt = "pretty"
```

Accepted values: `ndjson`, `pretty`, `table`, `csv`, `raw`.

The `--fmt` flag always takes priority over the config file.

---

## Performance

### How fast is qk compared to jq?

For NDJSON filter queries on a 100 MB file, qk is typically **3-5× faster**
than `jq` because:
- No DOM allocation: qk streams NDJSON line by line
- `memchr`/`memmem` SIMD-accelerated string search
- `rayon` parallel reads across multiple files

For benchmarks, run:
```bash
cargo bench
```
and see the recorded numbers in `PROGRESS.md`.

### Why is my query slow on CSV/YAML/TOML?

These formats require full-file parsing before any query runs, unlike NDJSON
which is line-by-line. For large CSV files, converting to NDJSON first is
faster:
```bash
qk . data.csv --fmt ndjson > data.ndjson
qk where status>499 data.ndjson
```

---

## Output

### How do I pipe qk output to another tool?

`qk` defaults to NDJSON output, which is friendly to further processing:
```bash
qk where level=error app.log | jq '.msg'
qk where level=error app.log | wc -l
qk where level=error app.log | sort | uniq
```

### How do I get pretty output in the terminal but NDJSON when piping?

Set your config default to `pretty`. When piping, override:
```bash
echo 'default_fmt = "pretty"' >> ~/.config/qk/config.toml
# Terminal: pretty automatically
qk where level=error app.log
# Piping: override back to ndjson
qk where level=error app.log --fmt ndjson | jq .
```

Or use `--fmt auto` (feature request — see ROADMAP.md).

### Why does `--fmt table` look broken in some terminals?

`comfy-table` requires a terminal that supports Unicode box-drawing characters
(U+2500 range). Most modern terminals (iTerm2, Terminal.app, Windows Terminal,
GNOME Terminal) support this. If you see garbled characters, try `--fmt pretty`
or `--fmt csv` instead.

---

## Flags Reference

| Flag | Purpose |
|------|---------|
| `--fmt <format>` | Output format: ndjson / pretty / table / csv / raw |
| `--color` | Force ANSI color on |
| `--no-color` | Disable ANSI color |
| `--explain` | Print parsed query plan and exit |
| `--stats` | Print records-in / records-out / elapsed time to stderr |
| `--cast FIELD=TYPE` | Override field type before querying |
| `--no-header` | Treat CSV/TSV first row as data, not a header |
| `--ui` | Launch interactive TUI browser |

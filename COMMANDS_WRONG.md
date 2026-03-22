# qk — Common Mistakes and Expected Output

This file shows what happens when you run **invalid or imperfect queries** in qk.
Use it as a reference to understand error messages and how to fix them.

---

## Table of Contents

1. [Wrong Operator: `==` instead of `=`](#wrong-operator--instead-of-)
2. [Unknown / Misspelled Flags](#unknown--misspelled-flags)
3. [Bad `--cast` Syntax](#bad---cast-syntax)
4. [Bad `--fmt` Value](#bad---fmt-value)
5. [Numeric Field vs Non-Numeric Literal](#numeric-field-vs-non-numeric-literal)
6. [File Not Found](#file-not-found)
7. [Shell Metacharacter Conflicts](#shell-metacharacter-conflicts)
8. [DSL String Comparison with `<` / `>`](#dsl-string-comparison-with---)
9. [Auto-Limit Notice (Not an Error)](#auto-limit-notice-not-an-error)
10. [Missing DSL Quotes](#missing-dsl-quotes)

---

## Wrong Operator: `==` instead of `=`

qk uses a single `=` for equality. Using `==` (as in Python/JavaScript) is a common mistake.

```bash
qk where level==error app.log
```

**Output (stderr):**
```
qk: query syntax error: invalid operator '==' in 'level==error'
  qk uses a single '=' for equality
  Hint: try `where level=error`
```

**Fix:**
```bash
qk where level=error app.log
```

---

## Unknown / Misspelled Flags

### Typo in flag name

```bash
qk --quite app.log
```

**Output (stderr):**
```
qk: unknown flag '--quite'
  Did you mean: --quiet?
  Valid flags: --quiet (-q), --all (-A), --color, --no-color, --stats, --explain, --ui, --no-header, --fmt (-f), --cast
  Run 'qk --help' for full usage.
```

**Fix:**
```bash
qk --quiet app.log
```

---

### Completely unknown flag

```bash
qk --xyzzy app.log
```

**Output (stderr):**
```
qk: unknown flag '--xyzzy'
  Valid flags: --quiet (-q), --all (-A), --color, --no-color, --stats, --explain, --ui, --no-header, --fmt (-f), --cast
  Run 'qk --help' for full usage.
```

---

### Flag typo after positional args (old behaviour was confusing)

Before this fix, `--quite` after query tokens caused: `IO error reading '--quite': No such file or directory`. This is now fixed — qk detects it as a flag typo at any position.

```bash
qk count app.log --quite
```

**Output (stderr):**
```
qk: unknown flag '--quite'
  Did you mean: --quiet?
  ...
```

---

## Bad `--cast` Syntax

### Missing `=` separator

```bash
qk --cast latencynumber avg latency app.log
```

**Output (stderr):**
```
qk: query syntax error: --cast requires FIELD=TYPE (e.g. --cast latency=number), got: "latencynumber"
```

**Fix:**
```bash
qk --cast latency=number avg latency app.log
```

---

### Unknown type name

```bash
qk --cast latency=foobar avg latency app.log
```

**Output (stderr):**
```
qk: query syntax error: --cast latency="foobar": unknown type.
  Supported: number (num/float/int), string (str/text), bool (boolean), null (none), auto
```

---

### Type name typo (with suggestion)

```bash
qk --cast latency=nubmer avg latency app.log
```

**Output (stderr):**
```
qk: query syntax error: --cast latency="nubmer": unknown type.
  Did you mean: num?
  Supported: number (num/float/int), string (str/text), bool (boolean), null (none), auto
```

---

## Bad `--fmt` Value

```bash
qk --fmt xml count app.log
```

**Output (stderr):**
```
error: invalid value 'xml' for '--fmt <FMT>'
  [possible values: ndjson, pretty, table, csv, raw]
```

**Valid values:** `ndjson` (default), `pretty`, `table`, `csv`, `raw`

---

## Numeric Field vs Non-Numeric Literal

When you compare a numeric field against a non-numeric value, qk warns once and returns no matches.

```bash
qk where 'latency>zxc' app.log
```

**Output (stderr, once per query):**
```
[qk warning] field 'latency' is numeric but literal "zxc" is not a number — comparison always false (use a number, or check field name)
```

**Output (stdout):** *(empty — no records match)*

**Fix:**
```bash
qk where 'latency>100' app.log
# or word operator (no quoting needed):
qk where latency gt 100 app.log
```

---

## File Not Found

```bash
qk count missing_file.log
```

**Output (stderr):**
```
qk: IO error reading 'missing_file.log': No such file or directory (os error 2)
```

---

### Flag that looks like a file path

If a path starts with `-`, qk detects it as a likely flag typo (not a file):

```bash
qk count --quite
```

**Output (stderr):**
```
qk: unknown flag '--quite'
  Did you mean: --quiet?
  ...
```

---

## Shell Metacharacter Conflicts

### `<` and `>` without quoting

```bash
qk where latency > 100 app.log
```

**What the shell does:** `>` is treated as output redirection. The command becomes `qk where latency` and the output is redirected to file `100`. qk gets no comparison value and either silently processes nothing or produces wrong results.

**Fix — use single quotes:**
```bash
qk where 'latency>100' app.log
```

**Fix — use word operators (recommended, no quoting):**
```bash
qk where latency gt 100 app.log
qk where latency lt 50 app.log
qk where latency gte 200 app.log
qk where latency lte 500 app.log
```

---

### `<=` with spaces

```bash
qk where latency <= 100 app.log
```

**What the shell does:** `<` redirects stdin from file `=`, `100` becomes an argument. This results in a shell error, not a qk error.

**Fix:**
```bash
qk where 'latency<=100' app.log
# or:
qk where latency lte 100 app.log
```

---

## DSL String Comparison with `<` / `>`

Comparing a string field with `<` or `>` in DSL mode is lexicographic — it may not give the intended result.

```bash
qk '.level <= "error"' app.log
```

**Output (stderr, once per query):**
```
[qk warning] comparing a string value with '<' / '>' uses lexicographic order, not numeric order — did you mean a numeric field? Use a number literal or --cast FIELD=number for numeric comparison.
```

**Output (stdout):** Records where `level` is lexicographically ≤ `"error"` (e.g. `"debug"`, `"error"` — not `"info"` or `"warn"` since `"i" > "e"` and `"w" > "e"` in ASCII order). This is rarely what users intend.

**Intended fix:** If you want to filter by severity level, use equality:
```bash
qk '.level == "error"' app.log
qk where level=error app.log
```

---

## Auto-Limit Notice (Not an Error)

When stdout is a terminal and your results exceed the default limit (20), qk shows a notice **after the output**. This is not an error — it's informational.

```bash
qk count by level app.log   # (returns more than 20 records)
```

**Output (stderr, after results):**
```
╭─ qk ───────────────────────────────────────────────────────────╮
│  25 records matched · showing first 20 · stdout is a terminal  │
│  Use --all / -A to show all, or pipe output to disable limit.  │
╰────────────────────────────────────────────────────────────────╯
```

**To show all records:**
```bash
qk count by level --all app.log
qk count by level -A app.log
```

**To disable the limit permanently:**
```toml
# ~/.config/qk/config.toml
default_limit = 0   # 0 = no limit
```

---

## Missing DSL Quotes

DSL expressions that start with `|` **must** be wrapped in single quotes. Without quotes, `|` is interpreted as a shell pipe operator.

### Wrong (no quotes):
```bash
qk | count() app.log
```

**What happens:** The shell treats `|` as a pipe. `qk` receives no stdin and `count() app.log` is run as a separate command. You'll see an error like `command not found: count()`.

### Wrong (smart/curly quotes from word processors):
```bash
qk '| count()' app.log   ← if the quotes are ' ' (curly), not ' ' (ASCII)
```

The shell does not recognise curly quotes as quote characters. The `|` becomes a pipe.

### Correct (ASCII single quotes):
```bash
qk '| count()' app.log
qk '| group_by_time(.ts, "5m")' app.log
qk '.level == "error" | pick(.ts, .msg)' app.log
```

> **Tip:** Always type single quotes manually in your terminal. Never copy-paste from Word, Pages, or rendered PDFs — they convert `'` to `'` automatically.

---

## Summary Table

| Wrong command | What goes wrong | Fix |
|---------------|----------------|-----|
| `where level==error` | `==` is invalid, silent no-match | `where level=error` |
| `--quite` | Unknown flag typo | `--quiet` |
| `--cast latency=nubmer` | Unknown type, no suggestion before fix | `--cast latency=number` |
| `--fmt xml` | Invalid format name | `--fmt table` |
| `where 'latency>zxc'` | Numeric vs string, always false | `where 'latency>100'` |
| `where latency > 100` (unquoted) | Shell redirect | `where latency gt 100` |
| `\| count()` (no quotes) | Shell pipe, not DSL | `'\| count()'` |
| `.level <= "error"` | Lexicographic, not semantic | Use `==` for string equality |

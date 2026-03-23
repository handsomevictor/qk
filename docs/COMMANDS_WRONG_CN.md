# qk — 常见错误用法与预期输出

本文件展示运行**错误或不完善的查询**时 qk 的输出，帮助用户理解错误信息并学会修正。

---

## 目录

1. [运算符错误：`==` 代替 `=`](#运算符错误-代替-)
2. [未知或拼写错误的标志](#未知或拼写错误的标志)
3. [`--cast` 语法错误](#--cast-语法错误)
4. [`--fmt` 值无效](#--fmt-值无效)
5. [数字字段与非数字字面量比较](#数字字段与非数字字面量比较)
6. [文件未找到](#文件未找到)
7. [Shell 元字符冲突](#shell-元字符冲突)
8. [DSL 中字符串字段使用 `<` / `>`](#dsl-中字符串字段使用--)
9. [自动限制提示（非错误）](#自动限制提示非错误)
10. [DSL 表达式缺少引号](#dsl-表达式缺少引号)

---

## 运算符错误：`==` 代替 `=`

qk 使用单个 `=` 表示等值比较。使用 `==`（Python/JavaScript 风格）是常见错误。

```bash
qk where level==error app.log
```

**输出（stderr）：**
```
qk: query syntax error: invalid operator '==' in 'level==error'
  qk uses a single '=' for equality
  Hint: try `where level=error`
```

**修正：**
```bash
qk where level=error app.log
```

---

## 未知或拼写错误的标志

### 标志名拼写错误

```bash
qk --quite app.log
```

**输出（stderr）：**
```
qk: unknown flag '--quite'
  Did you mean: --quiet?
  Valid flags: --quiet (-q), --all (-A), --color, --no-color, --stats, --explain, --ui, --no-header, --fmt (-f), --cast
  Run 'qk --help' for full usage.
```

**修正：**
```bash
qk --quiet app.log
```

---

### 完全未知的标志

```bash
qk --xyzzy app.log
```

**输出（stderr）：**
```
qk: unknown flag '--xyzzy'
  Valid flags: --quiet (-q), --all (-A), ...
  Run 'qk --help' for full usage.
```

---

### 标志写在位置参数之后（历史问题已修复）

旧版本中，`--quite` 写在查询词之后会产生：`IO error reading '--quite': No such file or directory`，现已修复——qk 在任意位置都能检测标志拼写错误。

```bash
qk count app.log --quite
```

**输出（stderr）：**
```
qk: unknown flag '--quite'
  Did you mean: --quiet?
  ...
```

---

## `--cast` 语法错误

### 缺少 `=` 分隔符

```bash
qk --cast latencynumber avg latency app.log
```

**输出（stderr）：**
```
qk: query syntax error: --cast requires FIELD=TYPE (e.g. --cast latency=number), got: "latencynumber"
```

**修正：**
```bash
qk --cast latency=number avg latency app.log
```

---

### 类型名未知

```bash
qk --cast latency=foobar avg latency app.log
```

**输出（stderr）：**
```
qk: query syntax error: --cast latency="foobar": unknown type.
  Supported: number (num/float/int), string (str/text), bool (boolean), null (none), auto
```

---

### 类型名拼写错误（含建议）

```bash
qk --cast latency=nubmer avg latency app.log
```

**输出（stderr）：**
```
qk: query syntax error: --cast latency="nubmer": unknown type.
  Did you mean: num?
  Supported: number (num/float/int), string (str/text), bool (boolean), null (none), auto
```

---

## `--fmt` 值无效

```bash
qk --fmt xml count app.log
```

**输出（stderr）：**
```
error: invalid value 'xml' for '--fmt <FMT>'
  [possible values: ndjson, pretty, table, csv, raw]
```

**有效值：** `ndjson`（默认）、`pretty`、`table`、`csv`、`raw`

---

## 数字字段与非数字字面量比较

当数字字段与非数字值比较时，qk 发出一次警告并返回空结果。

```bash
qk where 'latency>zxc' app.log
```

**输出（stderr，每次查询仅提示一次）：**
```
[qk warning] field 'latency' is numeric but literal "zxc" is not a number — comparison always false (use a number, or check field name)
```

**输出（stdout）：** *（空——没有记录匹配）*

**修正：**
```bash
qk where 'latency>100' app.log
# 或使用 word operator（无需引号）：
qk where latency gt 100 app.log
```

---

## 文件未找到

```bash
qk count missing_file.log
```

**输出（stderr）：**
```
qk: IO error reading 'missing_file.log': No such file or directory (os error 2)
```

---

### 路径以 `-` 开头被当成标志

如果路径以 `-` 开头，qk 会将其识别为可能的标志拼写错误：

```bash
qk count --quite
```

**输出（stderr）：**
```
qk: unknown flag '--quite'
  Did you mean: --quiet?
  ...
```

---

## Shell 元字符冲突

### `<` 和 `>` 未引用

```bash
qk where latency > 100 app.log
```

**Shell 的处理：** `>` 被解释为输出重定向，命令变为 `qk where latency`，输出重定向到文件 `100`。qk 得不到比较值，结果错误或为空。

**修正——使用单引号：**
```bash
qk where 'latency>100' app.log
```

**修正——使用 word operator（推荐，无需引号）：**
```bash
qk where latency gt 100 app.log
qk where latency lt 50 app.log
qk where latency gte 200 app.log
qk where latency lte 500 app.log
```

---

### `<=` 带空格

```bash
qk where latency <= 100 app.log
```

**Shell 的处理：** `<` 将 stdin 重定向自文件 `=`，`100` 变成参数。Shell 会报错，不是 qk 的问题。

**修正：**
```bash
qk where 'latency<=100' app.log
# 或：
qk where latency lte 100 app.log
```

---

## DSL 中字符串字段使用 `<` / `>`

在 DSL 模式中，对字符串字段使用 `<`/`>` 是按字典序比较——通常不是预期结果。

```bash
qk '.level <= "error"' app.log
```

**输出（stderr，每次查询仅提示一次）：**
```
[qk warning] comparing a string value with '<' / '>' uses lexicographic order, not numeric order — did you mean a numeric field? Use a number literal or --cast FIELD=number for numeric comparison.
```

**输出（stdout）：** `level` 字典序 ≤ `"error"` 的记录（例如 `"debug"`、`"error"`——但 `"info"` 和 `"warn"` 因为 `"i" > "e"` 而不包含）。这通常不是用户的本意。

**正确做法：**
```bash
qk '.level == "error"' app.log
qk where level=error app.log
```

---

## 自动限制提示（非错误）

当 stdout 连接到终端且结果超过默认限制（20条）时，qk 会在**输出之后**显示提示框。这不是错误。

```bash
qk count by level app.log
```

**输出（stderr，在结果之后）：**
```
╭─ qk ───────────────────────────────────────────────────────────╮
│  25 records matched · showing first 20 · stdout is a terminal  │
│  Use --all / -A to show all, or pipe output to disable limit.  │
╰────────────────────────────────────────────────────────────────╯
```

**显示全部记录：**
```bash
qk count by level --all app.log
qk count by level -A app.log
```

**永久禁用限制：**
```toml
# ~/.config/qk/config.toml
default_limit = 0   # 0 = 不限制
```

---

## DSL 表达式缺少引号

以 `|` 开头的 DSL 表达式**必须**用 ASCII 单引号括起来。不加引号时，`|` 会被 shell 解析为管道操作符。

### 错误（无引号）：
```bash
qk | count() app.log
```

**发生什么：** shell 将 `|` 解析为管道。`qk` 从 stdin 读取，`count() app.log` 作为单独命令运行，导致 `command not found: count()`。

### 错误（使用弯引号 / 智能引号）：
```bash
qk '| count()' app.log   ← 如果引号是 ' ' 而非 ' '（ASCII）
```

Shell 不识别弯引号，`|` 仍会被解析为管道。

### 正确（ASCII 单引号）：
```bash
qk '| count()' app.log
qk '| group_by_time(.ts, "5m")' app.log
qk '.level == "error" | pick(.ts, .msg)' app.log
```

> **提示：** 始终在终端中**手动输入**单引号，不要从 Word、Pages 或渲染后的 PDF 中复制粘贴——这些工具会自动将 `'` 转换为 `'`（弯引号）。

---

## 错误汇总表

| 错误命令 | 问题所在 | 修正方法 |
|---------|---------|---------|
| `where level==error` | `==` 无效，静默无匹配 | `where level=error` |
| `--quite` | 标志名拼写错误 | `--quiet` |
| `--cast latency=nubmer` | 类型名未知 | `--cast latency=number` |
| `--fmt xml` | 格式名无效 | `--fmt table` |
| `where 'latency>zxc'` | 数字与字符串比较，永远为 false | `where 'latency>100'` |
| `where latency > 100`（未引用） | Shell 重定向 | `where latency gt 100` |
| `\| count()`（无引号） | Shell 管道，非 DSL | `'\| count()'` |
| `.level <= "error"` | 字典序比较，非语义比较 | 使用 `==` 进行字符串相等判断 |

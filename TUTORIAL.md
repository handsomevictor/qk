# qk 完整教程

本文档面向 Rust 新手，从零开始介绍如何安装、编译、使用和开发 `qk`。

---

## 目录

1. [qk 是什么](#qk-是什么)
2. [安装 Rust 环境](#安装-rust-环境)
3. [获取源码并编译](#获取源码并编译)
4. [快速上手：10 分钟入门](#快速上手10-分钟入门)
5. [关键字查询语法](#关键字查询语法)
6. [DSL 表达式语法](#dsl-表达式语法)
7. [支持的文件格式](#支持的文件格式)
8. [输出格式](#输出格式)
9. [颜色输出](#颜色输出)
10. [管道用法：连接多个 qk](#管道用法连接多个-qk)
11. [常见使用场景](#常见使用场景)
12. [开发者指南](#开发者指南)
13. [常见问题](#常见问题)
14. [完整命令速查表](#完整命令速查表)

---

## qk 是什么

`qk`（quick）是一个命令行数据查询工具，用一条命令替代 `grep`、`awk`、`sed`、`jq`、`yq`、`cut`、`sort | uniq` 等多个工具。

**它能做什么：**

| 需求 | 以前怎么做 | 用 qk 怎么做 |
|------|-----------|-------------|
| 过滤错误日志 | `grep "error" app.log \| awk '{print $3, $5}'` | `qk where level=error select ts msg app.log` |
| 查询 JSON 日志 | `cat req.json \| jq '.[] \| select(.status > 499) \| .path'` | `qk where status>499 select path req.json` |
| 按字段统计次数 | `awk '{print $2}' \| sort \| uniq -c \| sort -rn` | `qk count by service app.log` |
| 求字段总和 | `awk '{sum+=$3} END {print sum}'` | `qk sum latency app.log` |
| 发现有哪些字段 | `jq 'keys'` / 手工查看 | `qk fields app.log` |
| 跨多种格式查询 | ❌ 不可能用一个工具做到 | `qk where error!=null *.log *.json` |
| 漂亮打印 JSON | `cat req.json \| jq .` | `qk --fmt pretty req.json` |
| 表达式过滤 + 管道 | `jq '.[] \| select(.level=="error") \| {level,msg}'` | `qk '.level == "error" \| pick(.level, .msg)'` |

**它的特点：**
- **自动识别格式**：NDJSON、JSON、YAML、TOML、CSV、logfmt……不需要指定格式参数
- **两套语法**：关键字语法（简单快速）+ 表达式 DSL（复杂场景）
- **终端彩色输出**：语义感知着色——error 红、warn 黄、info 绿，消息内容最醒目
- **并行处理**：rayon 多核并行处理多个文件
- **透明解压**：直接读取 `.gz` 文件，无需手动解压
- **Rust 编写**：二进制体积 <5MB，启动时间 <2ms

---

## 安装 Rust 环境

`qk` 用 Rust 编写，需要先安装 Rust 工具链。

### 第一步：安装 rustup（Rust 版本管理器）

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

安装过程中选择默认选项（按 Enter 即可）。

安装完成后，**重新打开终端**，或者运行：

```bash
source ~/.cargo/env
```

### 第二步：验证安装成功

```bash
rustc --version    # 应显示 rustc 1.75.0 或更高版本
cargo --version    # 应显示 cargo 1.75.0 或更高版本
```

> **rustc** 是 Rust 编译器，**cargo** 是 Rust 的包管理器和构建工具（类似 Python 的 pip + make）。

---

## 获取源码并编译

### 克隆仓库

```bash
git clone https://github.com/YOUR_USERNAME/qk.git
cd qk
```

### 开发模式编译（调试用，速度快）

```bash
cargo build
```

编译后的二进制文件在：`./target/debug/qk`

### 发布模式编译（性能优化，体积更小）

```bash
cargo build --release
```

编译后的二进制文件在：`./target/release/qk`

> **发布模式**：开启了 LTO（链接时优化）和代码优化，运行速度更快，体积更小，但编译时间更长。
> **调试模式**：编译快，包含调试符号，运行慢。开发时用这个。

### 安装到系统 PATH（可选）

```bash
cargo install --path .
```

安装后可以直接在任意目录运行 `qk`（安装到 `~/.cargo/bin/`）。

验证安装：

```bash
qk --version
```

---

## 快速上手：10 分钟入门

### 准备测试数据

```bash
cat > app.log << 'EOF'
{"ts":"2024-01-01T10:00:00Z","level":"info","service":"api","msg":"server started","latency":0}
{"ts":"2024-01-01T10:01:00Z","level":"error","service":"api","msg":"connection timeout","latency":3001}
{"ts":"2024-01-01T10:02:00Z","level":"warn","service":"worker","msg":"queue depth high","latency":150}
{"ts":"2024-01-01T10:03:00Z","level":"info","service":"api","msg":"request ok","latency":42}
{"ts":"2024-01-01T10:04:00Z","level":"error","service":"worker","msg":"panic: nil pointer","latency":0}
EOF
```

### 查看所有记录（有漂亮颜色）

```bash
qk app.log
```

在终端中，`level` 字段会自动着色（error=红，warn=黄，info=绿），消息内容以亮白色显示。

### 过滤记录（grep 的替代）

```bash
qk where level=error app.log
```

### 选择字段（awk 的替代）

```bash
qk where level=error select ts service msg app.log
```

### 统计（sort | uniq -c 的替代）

```bash
qk count by service app.log
```

输出：
```json
{"service":"api","count":3}
{"service":"worker","count":2}
```

### 排序 + 限制数量

```bash
# 找出延迟最高的前 3 条
qk sort latency desc limit 3 app.log
```

### 从 stdin 读取（管道）

```bash
echo '{"level":"error","msg":"oops"}' | qk where level=error
```

---

## 关键字查询语法

`qk` 的关键字语法覆盖 80% 的日常场景，直观易记。

```
qk [FLAGS] [where 条件...] [select 字段...] [COMMAND] [文件...]

FLAGS:   --fmt FORMAT   --color   --no-color   --explain
COMMAND: count [by 字段] | sort 字段 [asc|desc] | limit N | head N
       | fields | sum 字段 | avg 字段 | min 字段 | max 字段
```

> **重要**：`--fmt`、`--color` 等标志必须置于查询关键字**之前**。
> ✅ `qk --fmt table where level=error app.log`
> ❌ `qk where level=error --fmt table app.log`（`--fmt` 会被当成文件路径）

### 过滤条件（where）

| 写法 | 含义 | 示例 |
|------|------|------|
| `FIELD=VALUE` | 精确匹配 | `where level=error` |
| `FIELD!=VALUE` | 不等于 | `where level!=info` |
| `FIELD>VALUE` | 数值大于 | `where latency>1000` |
| `FIELD<VALUE` | 数值小于 | `where status<400` |
| `FIELD>=VALUE` | 数值大于等于 | `where status>=500` |
| `FIELD<=VALUE` | 数值小于等于 | `where latency<=100` |
| `FIELD~=PATTERN` | 正则匹配 | `where msg~=time.*out` |
| `FIELD contains TEXT` | 包含子字符串 | `where msg contains timeout` |
| `FIELD exists` | 字段存在 | `where error exists` |

**多条件组合：**
```bash
# AND（两个条件都要满足）
qk where level=error and service=api app.log

# OR（满足其中一个）
qk where level=error or level=warn app.log

# NOT（取反）
qk where not level=info app.log
```

### 选择字段（select）

```bash
# 只保留 ts, level, msg 三个字段
qk where level=error select ts level msg app.log
```

### 统计（count）

```bash
# 统计总数
qk count app.log
qk where level=error count app.log

# 按字段分组统计（输出按数量降序）
qk count by service app.log
qk count by level app.log
```

### 排序（sort）

```bash
qk sort latency desc app.log      # 降序
qk sort latency asc app.log       # 升序（默认）
qk sort service app.log           # 字符串排序
```

### 限制数量（limit / head）

```bash
qk limit 10 app.log
qk head 10 app.log                      # head 是 limit 的别名，更直觉
qk sort latency desc limit 5 app.log    # 延迟最高的 5 条
```

### 数值统计（sum / avg / min / max）

对某个数值字段做快速聚合，输出单条 `{"key": N}` 记录：

```bash
# 延迟总和（替代 awk '{sum+=$n} END{print sum}'）
qk sum latency app.log
# 输出：{"sum": 12345}

# 平均延迟
qk avg latency app.log
# 输出：{"avg": 247.5}

# 最小 / 最大延迟
qk min latency app.log
qk max latency app.log

# 先过滤，再聚合
qk where level=error sum latency app.log
qk where service=api avg latency app.log
```

### 字段发现（fields）

列出数据集中所有出现过的字段名，适合第一次接触陌生日志文件时快速了解结构：

```bash
# 有哪些字段？
qk fields app.log
# 输出（每行一个字段）：
# {"field":"latency"}
# {"field":"level"}
# {"field":"msg"}
# {"field":"service"}
# {"field":"ts"}

# 先过滤再发现（只看 error 记录的字段）
qk where level=error fields app.log
```

### 嵌套字段访问

用点号（`.`）访问嵌套的 JSON 字段：

```bash
# JSON: {"response":{"status":503,"latency":1200}}
qk where response.status=503 app.log
qk select response.status response.latency app.log
```

### 调试模式（--explain）

查看 `qk` 如何解析你的查询：

```bash
qk --explain where level=error select ts msg app.log
```

---

## DSL 表达式语法

当关键字语法不够用时，切换到 DSL 层——功能更强大，语法类似 `jq`。

**DSL 模式触发**：第一个参数以 `.`、`not ` 或 `|` 开头时自动切换。

```bash
# 最简单的 DSL：字段比较
qk '.level == "error"' app.log

# 多条件
qk '.level == "error" and .service == "api"' app.log

# 管道 + 变换阶段
qk '.level == "error" | pick(.ts, .msg)' app.log
```

### 过滤表达式

| 语法 | 说明 | 示例 |
|------|------|------|
| `.field == "val"` | 字符串等于 | `.level == "error"` |
| `.field != "val"` | 不等于 | `.level != "info"` |
| `.field > N` | 数值比较 | `.latency > 1000` |
| `.field < N` | 数值比较 | `.status < 500` |
| `.field >= N` | 大于等于 | `.status >= 200` |
| `.field <= N` | 小于等于 | `.latency <= 100` |
| `.field exists` | 字段存在 | `.error exists` |
| `.field contains "text"` | 包含子字符串 | `.msg contains "timeout"` |
| `.field matches "regex"` | 正则匹配 | `.msg matches "time.*"` |
| `EXPR and EXPR` | 逻辑与 | `.level == "error" and .service == "api"` |
| `EXPR or EXPR` | 逻辑或 | `.level == "error" or .level == "warn"` |
| `not EXPR` | 逻辑非 | `not .level == "info"` |
| `.a.b.c` | 嵌套字段 | `.response.status == 503` |

### 管道阶段（`|`）

所有阶段语法一览：

| 阶段 | 语法 | 说明 |
|------|------|------|
| pick | `\| pick(.f1, .f2)` | 只保留指定字段 |
| omit | `\| omit(.f1, .f2)` | 去掉指定字段 |
| count | `\| count()` | 统计记录数 → `{"count": N}` |
| sort_by | `\| sort_by(.f asc\|desc)` | 按字段排序 |
| group_by | `\| group_by(.f)` | 分组统计 → `{"f": val, "count": N}` |
| limit | `\| limit(N)` | 取前 N 条 |
| skip | `\| skip(N)` | 跳过前 N 条（分页 offset） |
| dedup | `\| dedup(.f)` | 按字段值去重，保留首次出现 |
| sum | `\| sum(.f)` | 数值求和 → `{"sum": N}` |
| avg | `\| avg(.f)` | 数值平均 → `{"avg": N}` |
| min | `\| min(.f)` | 最小值 → `{"min": N}` |
| max | `\| max(.f)` | 最大值 → `{"max": N}` |

```bash
# pick：只保留指定字段
qk '.level == "error" | pick(.ts, .level, .msg)' app.log

# omit：去掉指定字段
qk '.level == "error" | omit(.ts)' app.log

# count：统计记录数
qk '.level == "error" | count()' app.log

# 仅统计，不过滤（| 开头 → 全部记录）
qk '| count()' app.log

# sort_by：按字段排序
qk '.n > 0 | sort_by(.latency desc)' app.log
qk '.n > 0 | sort_by(.latency asc)' app.log

# group_by：分组统计（类似 count by）
qk '.level == "error" | group_by(.service)' app.log

# limit：取前 N 条
qk '.level == "error" | limit(10)' app.log

# skip：跳过前 N 条（分页）
qk '.level == "error" | skip(20) | limit(10)' app.log   # 第 3 页

# dedup：按字段去重（每个 service 只保留一条）
qk '.level == "error" | dedup(.service)' app.log

# sum / avg / min / max：数值聚合
qk '.level == "error" | sum(.latency)' app.log
qk '.level == "error" | avg(.latency)' app.log
qk '.latency > 0 | min(.latency)' app.log
qk '.latency > 0 | max(.latency)' app.log
```

### 链式管道

管道阶段可以连续链接：

```bash
# 过滤 error → 按 service 分组 → 取前 5 组
qk '.level == "error" | group_by(.service) | limit(5)' app.log

# 过滤高延迟 → 降序排序 → 只保留两个字段
qk '.latency > 1000 | sort_by(.latency desc) | pick(.ts, .latency, .msg)' app.log

# 分页：跳过前 100 条，取接下来 20 条
qk '.level == "error" | skip(100) | limit(20)' app.log

# 去重后统计唯一 service 数
qk '| dedup(.service) | count()' app.log
```

### 关键字模式 vs DSL 模式对照

| 功能 | 关键字模式 | DSL 模式 |
|------|-----------|---------|
| 过滤 | `where level=error` | `.level == "error"` |
| 多条件 AND | `where level=error and service=api` | `.level == "error" and .service == "api"` |
| 选字段 | `select ts msg` | `\| pick(.ts, .msg)` |
| 去掉字段 | —— | `\| omit(.ts)` |
| 统计 | `count` | `\| count()` |
| 分组统计 | `count by service` | `\| group_by(.service)` |
| 排序 | `sort latency desc` | `\| sort_by(.latency desc)` |
| 限制 | `limit 10` / `head 10` | `\| limit(10)` |
| 跳过 | —— | `\| skip(N)` |
| 去重 | —— | `\| dedup(.field)` |
| 求和 | `sum latency` | `\| sum(.latency)` |
| 平均 | `avg latency` | `\| avg(.latency)` |
| 最小值 | `min latency` | `\| min(.latency)` |
| 最大值 | `max latency` | `\| max(.latency)` |
| 字段发现 | `fields` | —— |
| 正则 | `where msg~=time.*` | `.msg matches "time.*"` |
| 字段存在 | `where error exists` | `.error exists` |

---

## 支持的文件格式

`qk` 自动检测格式，无需指定参数。

### NDJSON（换行分隔 JSON）

最常见的结构化日志格式：

```
{"level":"error","service":"api","msg":"timeout","latency":3001}
{"level":"info","service":"web","msg":"ok","latency":42}
```

**自动检测**：内容以 `{` 开头，多行都有 `{`；或 `.ndjson` 扩展名

### JSON 文件

完整的 JSON 数组（每个元素变成一条记录）：

```json
[
  {"status": 200, "path": "/api/users"},
  {"status": 404, "path": "/api/missing"}
]
```

**自动检测**：内容以 `[` 开头；或 `.json` 扩展名

### YAML 文件

支持多文档 YAML（`---` 分隔符，每个文档变成一条记录）：

```yaml
---
level: error
service: api
msg: connection timeout
latency: 3001
---
level: info
service: web
msg: ok
```

**自动检测**：`.yaml` / `.yml` 扩展名；或内容以 `---` / `- ` 开头

### TOML 文件

适合配置文件查询（整个文档变成一条记录）：

```toml
level = "error"
service = "api"
msg = "connection timeout"
latency = 3001
port = 8080
```

**自动检测**：`.toml` 扩展名；或内容有 `[section]` 节头或 `key = value`

### CSV 文件

逗号分隔，第一行是列名：

```
ts,level,service,msg
2024-01-01,error,api,timeout
2024-01-01,info,web,ok
```

**自动检测**：`.csv` 扩展名；或内容包含逗号分隔的多列

### TSV 文件

制表符分隔，类似 CSV：

```
name	age	city
alice	30	NYC
```

**自动检测**：`.tsv` 扩展名

### logfmt

Go 语言服务常用的日志格式，`key=value` 对：

```
level=error service=api msg="connection timeout" latency=3001
level=info service=web msg="page loaded" latency=88
```

**自动检测**：内容中有多个 `key=value` 对

### Gzip 压缩文件

`.gz` 结尾的压缩文件，透明解压后按内部格式处理：

```bash
# 直接查询 gzip 日志，无需手动解压
qk where level=error /var/log/app.log.gz
qk '.level == "error"' access.ndjson.gz
```

**自动检测**：文件魔数 `0x1f 0x8b`；或 `.gz` 扩展名

### 纯文本

任何其他文本，每行变成一条记录，字段名为 `line`：

```bash
qk where line contains "error" app.txt
```

### 文件扩展名优先级

| 扩展名 | 解析为 |
|--------|--------|
| `.ndjson` | NDJSON |
| `.yaml` / `.yml` | YAML |
| `.toml` | TOML |
| `.tsv` | TSV |
| `.csv` | CSV |
| `.gz` | Gzip（解压后再检测内部格式）|
| `.json` / `.log` / 其他 | 内容自动检测 |

---

## 输出格式

通过 `--fmt` 标志（简写 `-f`）选择输出格式。**必须放在查询之前。**

```bash
# NDJSON（默认，适合管道传输）
qk --fmt ndjson where level=error app.log
qk where level=error app.log              # 同上，ndjson 是默认

# Pretty（缩进 JSON，替代 jq .）
qk --fmt pretty where level=error app.log
qk --fmt pretty app.log                   # 漂亮显示所有记录

# 对齐表格（适合人工阅读，带颜色）
qk --fmt table where level=error app.log

# CSV（适合导入 Excel / 数据库）
qk --fmt csv where level=error app.log

# 原始行（直接输出匹配到的原始文本行，不重新序列化）
qk --fmt raw where level=error app.log
```

### 各格式对比

| 格式 | 适合场景 | 特点 |
|------|---------|------|
| `ndjson` | 管道传输、程序处理 | 每行一个 JSON 对象，支持颜色 |
| `pretty` | 人工审查单条记录 | 缩进 JSON，块间空行，支持 `--color` 语义着色 |
| `table` | 终端阅读多条记录 | 自动对齐列，截断超长内容（60 字符+`…`），彩色表头 |
| `csv` | 导出数据 | 标准 CSV，含头部行，自动转义逗号/引号 |
| `raw` | 查看原始日志 | 原样输出匹配到的原始行，不做任何处理 |

**pretty 输出示例：**

```bash
$ qk --fmt pretty --color where level=error app.log
{
  "ts": "2024-01-01T10:01:00Z",
  "level": "error",
  "service": "api",
  "msg": "connection timeout",
  "latency": 3001
}

{
  "ts": "2024-01-01T10:04:00Z",
  "level": "error",
  "service": "worker",
  "msg": "panic: nil pointer",
  "latency": 0
}
```

---

## 颜色输出

`qk` 在终端下默认开启彩色输出，在管道传输时自动关闭（遵守 Unix 惯例）。

### 颜色方案

**NDJSON 输出的语义着色：**

| 字段/值类型 | 颜色 |
|------------|------|
| 字段名（Keys） | 粗体青色 |
| `level: "error"` / `"fatal"` | **粗体红色** |
| `level: "warn"` | **粗体黄色** |
| `level: "info"` | **粗体绿色** |
| `level: "debug"` | 蓝色 |
| `level: "trace"` | 暗淡 |
| `msg` / `message` | 亮白色（最醒目）|
| `ts` / `timestamp` | 暗淡（背景噪声）|
| `error` / `exception` 字段 | 红色 |
| HTTP `status` 200-299 | 绿色 |
| HTTP `status` 400-499 | 黄色 |
| HTTP `status` 500-599 | **粗体红色** |
| 数字 | 黄色 |
| 布尔值 | 洋红色 |
| null | 暗淡 |
| 结构符号 `{} [] : ,` | 暗淡 |

### 颜色控制

```bash
# 默认：终端下自动开启颜色，管道传输时关闭
qk where level=error app.log

# 强制开启颜色（适合管道给 less -R）
qk --color where level=error app.log | less -R

# 强制关闭颜色
qk --no-color where level=error app.log

# 通过环境变量禁用（NO_COLOR 是行业标准）
NO_COLOR=1 qk where level=error app.log
```

**优先级规则（从高到低）：**
1. `--no-color` 标志 → 始终关闭
2. `--color` 标志 → 始终开启（覆盖 `NO_COLOR` 环境变量）
3. `NO_COLOR` 环境变量 → 关闭
4. 自动检测 → stdout 是终端则开启，管道传输则关闭

---

## 管道用法：连接多个 qk

`qk` 默认输出 NDJSON，可以直接管道给另一个 `qk`：

```bash
# 先过滤错误，再按 service 统计
qk where level=error app.log | qk count by service

# 先过滤高延迟，再只取最差的 3 条
qk where latency>1000 app.log | qk sort latency desc | qk limit 3

# 多步骤处理
cat app.log | qk where level=error | qk where service=api | qk count

# 带颜色输出给 less（--color 强制开启）
qk --color where level=error app.log | less -R
```

---

## 常见使用场景

### 场景一：分析服务器日志

```bash
# 有多少个错误？
qk where level=error count app.log

# 哪个服务出错最多？
qk where level=error count by service app.log

# 最慢的 10 个请求（表格显示）
qk --fmt table sort latency desc limit 10 app.log

# 所有 API 超时（DSL 语法）
qk '.level == "error" and .service == "api" and .msg contains "timeout"' app.log
```

### 场景二：分析 HTTP 访问日志

```bash
# 所有 5xx 响应
qk where status>=500 access.log

# 按状态码分组统计
qk count by status access.log

# 最慢的接口（DSL：过滤 + 排序 + 选字段）
qk '.status >= 200 | sort_by(.latency desc) | limit(10) | pick(.path, .status, .latency)' access.log
```

### 场景三：分析 YAML / TOML 配置

```bash
# 查询 YAML 配置中 enabled=false 的服务
qk where enabled=false services.yaml

# 查询 TOML 配置的某个字段
qk '.port > 8000' config.toml
```

### 场景四：查询 gzip 日志（无需解压）

```bash
# 直接查询压缩日志
qk where level=error /var/log/app.log.gz

# 统计压缩日志中的错误数
qk where level=error count /var/log/app.log.gz

# DSL 查询
qk '.level == "error" | group_by(.service)' /var/log/app.log.gz
```

### 场景五：跨格式批量查询

```bash
# 同时查询多种格式的文件
qk where error exists *.log *.json *.yaml

# 查询整个目录（包括 gzip 压缩文件）
qk where level=error /var/log/*.log /var/log/*.log.gz
```

### 场景六：导出数据

```bash
# 导出为 CSV（可以用 Excel 打开）
qk --fmt csv where level=error app.log > errors.csv

# 导出错误明细到 CSV
qk --fmt csv '.level == "error" | pick(.ts, .service, .msg, .latency)' app.log > errors.csv
```

### 场景七：快速统计分析（替代 awk）

```bash
# 所有请求的总延迟
qk sum latency app.log

# error 请求的平均延迟
qk where level=error avg latency app.log

# 延迟的最大 / 最小值
qk min latency app.log
qk max latency app.log

# DSL：过滤后统计（延迟超 1s 的总延迟）
qk '.latency > 1000 | sum(.latency)' app.log

# 统计所有服务的平均延迟
qk '.latency > 0 | avg(.latency)' app.log
```

### 场景八：数据探索（不熟悉的日志文件）

```bash
# 第一步：发现有哪些字段
qk fields unknown.log

# 第二步：查看几条记录（pretty 格式更易读）
qk --fmt pretty head 3 unknown.log

# 第三步：按感兴趣的字段统计
qk count by level unknown.log

# 第四步：深入分析
qk where level=error fields unknown.log   # error 记录有哪些额外字段？
```

### 场景九：分页浏览大文件

```bash
# 第 1 页（前 20 条）
qk where level=error limit 20 app.log

# 第 2 页（第 21-40 条）—— DSL
qk '.level == "error" | skip(20) | limit(20)' app.log

# 第 3 页
qk '.level == "error" | skip(40) | limit(20)' app.log
```

### 场景十：去重查看

```bash
# 每个 service 只看一条 error（了解哪些服务有错）
qk '.level == "error" | dedup(.service) | pick(.service, .msg)' app.log

# 去重后统计有多少个不同的 service 有错
qk '.level == "error" | dedup(.service) | count()' app.log
```

---

## 开发者指南

本节介绍如何参与 `qk` 的开发。

### 项目结构

```
qk/
├── Cargo.toml          # 项目配置和依赖声明
├── src/
│   ├── main.rs         # 入口：解析命令行、串联整个流水线
│   ├── cli.rs          # 命令行参数定义（clap）
│   ├── detect.rs       # 格式自动检测
│   ├── record.rs       # Record 类型（统一中间表示）
│   ├── parser/         # 各格式的解析器
│   ├── query/
│   │   ├── fast/       # 关键字查询层
│   │   └── dsl/        # DSL 表达式层（nom 解析器）
│   ├── output/
│   │   ├── color.rs    # 语义感知 ANSI 颜色渲染
│   │   ├── ndjson.rs   # NDJSON 序列化（支持颜色）
│   │   ├── pretty.rs   # 缩进 JSON 输出（块间空行，支持颜色）
│   │   ├── table.rs    # comfy-table 对齐表格
│   │   └── csv_out.rs  # CSV 序列化
│   └── util/           # 工具：错误类型、mmap、gzip 解压
├── tests/
│   ├── fast_layer.rs   # 集成测试：关键字查询 + 颜色标志
│   ├── dsl_layer.rs    # 集成测试：DSL 表达式层
│   ├── formats.rs      # 集成测试：各格式 + 输出格式
│   └── fixtures/       # 测试数据（ndjson/logfmt/csv/yaml/toml）
└── CLAUDE.md           # AI 辅助开发规则
```

### 常用开发命令

```bash
# 编译（调试模式，速度快）
cargo build

# 运行（直接运行，不需要先 build）
cargo run -- where level=error app.log
cargo run -- '.level == "error" | pick(.ts, .msg)' app.log

# 运行所有测试
cargo test

# 只运行某个测试
cargo test color_flag_produces_ansi

# 只运行某个模块的测试
cargo test detect::tests

# 格式化代码（提交前必须运行）
cargo fmt

# Lint 检查（零警告）
cargo clippy -- -D warnings

# 编译发布版本
cargo build --release
```

### 数据流

```
命令行参数
    ↓
cli.rs：解析 Cli 结构体（--fmt, --color, --no-color, args...）
    ↓
main.rs：determine_mode()
    ├── 首参数以 . / not  / | 开头 → DSL 模式
    └── 否则 → 关键字模式
    ↓
load_records()：rayon par_iter → 并行读取多个文件
    ├── util/mmap.rs：大文件用 mmap，小文件直接读
    ├── util/decompress.rs：.gz 透明解压
    ├── detect.rs：嗅探格式
    └── parser/*.rs：解析 → Vec<Record>
    ↓
查询引擎：过滤 + 变换 → Vec<Record>
    ↓
output/mod.rs：render(records, fmt, use_color)
    ├── ndjson + color.rs：语义着色 NDJSON
    ├── table.rs：comfy-table 对齐表格
    ├── csv_out.rs：CSV 序列化
    └── raw：原样输出 rec.raw
```

### 添加新格式（步骤）

详见 `CLAUDE.md` 中的"添加新格式工作流"。简要步骤：

1. `detect.rs` — 在 `Format` 枚举中添加变体
2. `detect::sniff()` — 添加检测逻辑（扩展名或内容启发式）
3. `src/parser/<格式>.rs` — 实现 `parse(input, source_file) -> Result<Vec<Record>>`
4. `src/parser/mod.rs` — 注册到 match 分支
5. `tests/fixtures/` — 添加 fixture 文件
6. `tests/formats.rs` — 添加集成测试
7. 更新 `STRUCTURE.md` 和 `PROGRESS.md`

---

## 常见问题

### Q: `--fmt`、`--color` 等标志没生效？

`qk` 使用 `trailing_var_arg` 语义——一旦遇到第一个非标志参数（如 `where`、`.level`），后面所有参数都被视为位置参数。因此，**标志必须放在查询之前**：

```bash
# ✅ 正确
qk --fmt table --color where level=error app.log
qk --fmt table '.level == "error"' app.log

# ❌ 错误（--fmt 会被当成文件名）
qk where level=error --fmt table app.log
```

### Q: 字段名里有点号怎么办？

如果字段名本身包含 `.`（比如 `"user.id": 123`），目前会被解释为嵌套访问。这是已知限制，建议在上游避免字段名含点号。

### Q: 文件名没有扩展名怎么处理？

通过内容自动检测。如果无扩展名的文件名被误认为查询字段（如 `select data`），用 `./data` 或绝对路径消歧。

### Q: 如何查询不等于 null 的记录？

```bash
qk where error exists app.log        # error 字段存在
qk '.error != null' app.log          # DSL：error 字段不为 null（存在且不是 JSON null）
```

### Q: 支持 glob 通配符吗？

支持，由 Shell 展开：

```bash
qk where level=error /var/log/*.log /var/log/*.log.gz
```

### Q: 颜色在 less 里显示不出来？

用 `less -R` 参数开启 ANSI 颜色支持，并用 `--color` 强制 qk 输出颜色：

```bash
qk --color where level=error app.log | less -R
```

### Q: 输出乱序是正常的吗？

是的，多文件并行处理时输出顺序不固定（rayon 并行）。同一个文件内部的记录顺序与输入一致。`sort` 会重新排序。

---

## 下一步

- 阅读 `README.md` 了解项目全貌
- 阅读 `STRUCTURE.md` 了解代码架构
- 阅读 `CLAUDE.md` 了解开发规范
- 查看 `PROGRESS.md` 了解开发进度
- 查看 `LESSON_LEARNED.md` 了解踩过的坑

---

## 完整命令速查表

### 全局标志（必须在查询之前）

| 标志 | 简写 | 说明 |
|------|------|------|
| `--fmt ndjson` | `-f ndjson` | NDJSON 输出（默认） |
| `--fmt pretty` | `-f pretty` | 缩进 JSON 输出 |
| `--fmt table` | `-f table` | 对齐表格输出 |
| `--fmt csv` | `-f csv` | CSV 输出 |
| `--fmt raw` | `-f raw` | 原始行输出 |
| `--color` | | 强制开启颜色 |
| `--no-color` | | 强制关闭颜色 |
| `--explain` | | 打印解析结果后退出 |

### 关键字模式完整语法

```
qk [FLAGS] [where FILTER [and|or FILTER]...] [select FIELD...] [COMMAND] [FILE...]
```

**过滤操作符：**

| 操作符 | 示例 | 说明 |
|--------|------|------|
| `=` | `level=error` | 等于 |
| `!=` | `level!=info` | 不等于 |
| `>` | `latency>1000` | 数值大于 |
| `<` | `status<400` | 数值小于 |
| `>=` | `status>=500` | 数值大于等于 |
| `<=` | `latency<=100` | 数值小于等于 |
| `~=` | `msg~=time.*` | 正则匹配 |
| `contains` | `msg contains timeout` | 包含子字符串 |
| `exists` | `error exists` | 字段存在 |

**聚合命令：**

| 命令 | 示例 | 输出 |
|------|------|------|
| `count` | `qk count app.log` | `{"count": N}` |
| `count by FIELD` | `qk count by service app.log` | `{"service":"x","count":N}` |
| `fields` | `qk fields app.log` | 每行 `{"field":"name"}` |
| `sum FIELD` | `qk sum latency app.log` | `{"sum": N}` |
| `avg FIELD` | `qk avg latency app.log` | `{"avg": N}` |
| `min FIELD` | `qk min latency app.log` | `{"min": N}` |
| `max FIELD` | `qk max latency app.log` | `{"max": N}` |

**排序 / 限制：**

| 命令 | 示例 |
|------|------|
| `sort FIELD [asc\|desc]` | `qk sort latency desc app.log` |
| `limit N` | `qk limit 10 app.log` |
| `head N` | `qk head 10 app.log`（同 limit）|
| `select FIELD...` | `qk select ts msg app.log` |

### DSL 模式完整语法

```
qk [FLAGS] 'FILTER_EXPR [| STAGE]...' [FILE...]
```

触发条件：第一个参数以 `.`、`not ` 或 `|` 开头。

**过滤表达式：**

| 语法 | 说明 |
|------|------|
| `.f == "val"` | 字符串等于 |
| `.f != "val"` | 不等于 |
| `.f > N` / `.f < N` / `.f >= N` / `.f <= N` | 数值比较 |
| `.f exists` | 字段存在 |
| `.f contains "text"` | 包含子字符串 |
| `.f matches "regex"` | 正则匹配 |
| `EXPR and EXPR` | 逻辑与 |
| `EXPR or EXPR` | 逻辑或 |
| `not EXPR` | 逻辑非 |
| `.a.b.c` | 嵌套字段访问 |

**管道阶段：**

| 阶段 | 语法 | 说明 |
|------|------|------|
| pick | `\| pick(.f1, .f2)` | 只保留指定字段 |
| omit | `\| omit(.f1, .f2)` | 去掉指定字段 |
| count | `\| count()` | 统计数量 |
| sort_by | `\| sort_by(.f desc)` | 排序 |
| group_by | `\| group_by(.f)` | 分组统计 |
| limit | `\| limit(N)` | 取前 N 条 |
| skip | `\| skip(N)` | 跳过前 N 条 |
| dedup | `\| dedup(.f)` | 按字段去重 |
| sum | `\| sum(.f)` | 数值求和 |
| avg | `\| avg(.f)` | 数值平均 |
| min | `\| min(.f)` | 最小值 |
| max | `\| max(.f)` | 最大值 |

### 支持的输入格式

| 格式 | 自动检测方式 |
|------|------------|
| NDJSON | 内容以 `{` 开头 / `.ndjson` 扩展名 |
| JSON 数组 | 内容以 `[` 开头 / `.json` 扩展名 |
| YAML | `---` / `- ` 开头 / `.yaml` / `.yml` |
| TOML | `key = value` 或 `[section]` / `.toml` |
| CSV | 逗号分隔多列 / `.csv` |
| TSV | `.tsv` 扩展名 |
| logfmt | `key=value key=value` 格式 |
| Gzip | 魔数 `0x1f 0x8b` / `.gz` 扩展名（透明解压）|
| 纯文本 | 其他所有格式（每行 → `{"line":"..."}` ）|

# qk — 一个工具，替代所有它们

`qk` 是一个快速结构化查询工具，用于终端。它用单一、一致的接口替代了 `grep`、`awk`、`sed`、`jq`、`yq`、`cut`、`sort | uniq` 等工具。

不再需要堆叠管道只是为了从日志文件中提取两个字段。不再需要根据格式在 `jq` 语法和 `awk` 语法之间切换。一个二进制，一套语法，支持所有格式。

---

## 已知限制

- **暂不支持流式处理（`tail -f`）：** qk 需要读到 stdin 的 EOF 才开始处理。`tail -f file | qk ...` 会无限阻塞。**临时替代方案：** 使用 `tail -n 1000 file | qk ...` 处理有限输入。
- **全量物化：** 大文件（>1 GB）会在处理前全部加载到内存中。对超大数据集，建议先用 `split` 分割文件或使用 `tail -n`。

---

## 为什么选 qk？

| 任务 | 以前的做法 | qk 的做法 |
|------|-----------|----------|
| 过滤错误日志 | `grep "error" app.log \| awk '{print $3, $5}'` | `qk where level=error select ts msg` |
| 查询 JSON API 日志 | `cat req.json \| jq '.[] \| select(.status > 499) \| .path'` | `qk where 'status>499' select path` |
| 按字段统计次数 | `awk '{print $2}' \| sort \| uniq -c \| sort -rn` | `qk count by service` |
| 跨格式查询 | ❌ 一个工具无法做到 | `qk where level=error *.log *.json` |
| 嵌套字段访问 | `jq '.response.headers["x-trace"]'` | `qk select response.headers.x-trace` |
| 多条件过滤 | `grep \| awk 'cond1 && cond2'` | `qk where level=error, service=api` |
| Shell 安全数值比较 | `awk '$5 > 100'` | `qk where latency gt 100` |
| 深层嵌套过滤 | `jq 'select(.pod.labels.app=="api")'` | `qk where pod.labels.app=api` |

---

## 功能特性

- **自动格式检测** — NDJSON、JSON、YAML、TOML、CSV、TSV、logfmt、纯文本；无需 `-f json` 参数
- **记录级模型** — 匹配完整的日志条目 / JSON 对象 / YAML 文档，而不仅仅是行
- **两套语法** — 快速关键字层（覆盖 80% 场景）+ 表达式 DSL（覆盖剩余 20%）
- **任意深度嵌套字段访问** — `pod.labels.app`、`response.headers.x-trace`，通过点路径访问任意深度
- **可读多条件过滤** — `where level=error, service=api, latency gt 100`（逗号 = and）
- **Shell 安全单词算子** — `gt`、`lt`、`gte`、`lte` 避免 `>`/`<` 的 shell 冲突
- **文字算子** — `startswith`、`endswith`、`glob` 用于前缀/后缀/通配符匹配
- **混合类型处理** — `--cast FIELD=TYPE` 在查询前强转字段类型；自动类型不匹配警告
- **结构化输出** — 默认输出 NDJSON，方便管道给下一个 `qk` 或 `jq`
- **并行处理** — 通过 `rayon` 使用所有 CPU 核心，文件数量线性扩展
- **透明解压** — 直接读取 `.gz` 文件，无需 `gunzip`
- **丰富的输出模式** — `ndjson`（默认）/ `pretty`（缩进 JSON，替代 `jq .`）/ `table` / `csv` / `raw`
- **语义颜色** — error=红，warn=黄，info=绿，HTTP 5xx=粗体红；管道时自动关闭
- **统计聚合** — `sum`、`avg`、`min`、`max`、`count by`、`group_by`、`dedup`
- **时间序列分桶** — `count by 5m` / `count by 1h` 将事件分组到固定时间窗口；自动识别 RFC 3339 字符串、Unix epoch 秒、epoch 毫秒
- **--no-header** — 将 CSV/TSV 第一行视为数据而非表头
- **Rust 编写** — 二进制体积 <5MB，启动时间 <2ms

---

## 安装

### 从源码编译

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

git clone https://github.com/handsomevictor/qk.git
cd qk
cargo install --path .
```

### 预编译二进制（macOS / Linux）

**Homebrew**（推荐）：
```bash
brew tap OWNER/qk
brew install qk
```

**一行安装脚本**（Linux / macOS）：
```bash
curl -fsSL https://raw.githubusercontent.com/OWNER/qk/main/install.sh | bash
```

**指定版本**：
```bash
QK_VERSION=v0.1.0 bash <(curl -fsSL https://raw.githubusercontent.com/OWNER/qk/main/install.sh)
```

**从源码编译**（需要 Rust ≥ 1.75）：
```bash
cargo install --git https://github.com/OWNER/qk
```

x86_64 和 aarch64 平台（Linux、macOS、Windows）的预编译二进制文件附于每个 [GitHub Release](https://github.com/OWNER/qk/releases)。

---

## 立即试用

`tutorial/` 目录包含**所有支持格式**的开箱即用测试文件，无需任何准备：

```bash
cd tutorial

# 验证所有 11 种格式均可解析：
qk count app.log           # 25 条记录 — NDJSON，2~3 级嵌套 JSON
qk count access.log        # 20 条记录 — NDJSON（嵌套 client/server）
qk count k8s.log           # 20 条记录 — NDJSON（3 级：pod.labels.app）
qk count data.json         # 8  条记录 — JSON 数组
qk count services.yaml     # 6  条记录 — YAML 多文档
qk count config.toml       # 1  条记录 — TOML
qk count users.csv         # 15 条记录 — CSV
qk count events.tsv        # 20 条记录 — TSV
qk count services.logfmt   # 16 条记录 — logfmt
qk count notes.txt         # 20 条记录 — 纯文本
qk count app.log.gz        # 25 条记录 — 透明 gzip 解压

# 立即开始查询：
qk where level=error app.log
qk where level=error, service=api app.log
qk where pod.labels.app=api k8s.log
qk count by service app.log
qk avg latency app.log
```

完整的复制即用参考请见 [`COMMANDS.md`](./COMMANDS.md)（涵盖所有格式、所有算子）。

---

## 快速开始

```bash
# 过滤错误（替代 grep）
qk where level=error app.log

# 多条件——逗号是 'and' 的可读别名
qk where level=error, service=api app.log
qk where level=error, latency gt 100 app.log

# Shell 安全数值比较（gt/lt/gte/lte——无需引号）
qk where latency gt 100 app.log
qk where status gte 500 access.log

# 嵌套字段访问——任意深度
qk where response.status=503 app.log
qk where pod.labels.app=api k8s.log
qk where request.headers.x-trace exists app.log

# 文字算子
qk where msg startswith connection app.log
qk where path endswith users access.log
qk where msg glob '*timeout*' app.log

# 选择特定字段
qk where level=error select ts service msg app.log

# 统计与聚合
qk count by service app.log
qk where level=error avg latency app.log
qk sum latency app.log

# 排序与限制
qk sort latency desc limit 10 app.log

# DSL 模式用于复杂逻辑
qk '.level == "error" and .latency > 1000' app.log
qk '.response.status >= 500 | pick(.ts, .service, .response.status)' app.log
qk '| group_by(.context.region)' app.log

# 管道：先过滤再统计
qk where level=error app.log | qk count by service

# Pretty 输出（替代 jq .）
qk --fmt pretty where level=error app.log

# 类型强转（混合类型字段）
qk --cast latency=number avg latency app.log

# 任意格式自动检测
qk where level=error app.logfmt
qk where city=NYC data.csv
qk where enabled=true services.yaml
qk where level=error app.log.gz     # 透明 gzip
```

---

## 语法参考

### 关键字层（快速，无需引号）

```
qk [FILTER] [TRANSFORM] [FILES...]

FILTER:
  where FIELD=VALUE              精确匹配
  where FIELD!=VALUE             不等于
  where FIELD>VALUE              数值大于（需引号或使用单词算子）
  where FIELD gt VALUE           数值大于（shell 安全，无需引号）
  where FIELD lt VALUE           数值小于（shell 安全）
  where FIELD gte VALUE          数值大于等于（shell 安全）
  where FIELD lte VALUE          数值小于等于（shell 安全）
  where FIELD~=PATTERN           正则匹配
  where FIELD contains TEXT      子字符串匹配
  where FIELD exists             字段存在检查
  where FIELD startswith PREFIX  前缀匹配（大小写敏感）
  where FIELD endswith SUFFIX    后缀匹配（大小写敏感）
  where FIELD glob PATTERN       通配符匹配（*/?，大小写不敏感）
  where A=1 and B=2              逻辑 AND
  where A=1 or B=2               逻辑 OR
  where A=1, B=2                 逗号 = 'and' 的别名（可读风格）
  where A=1, B gt 10, C=x        逗号链：多条件

TRANSFORM:
  select FIELD [FIELD...]        只保留这些字段
  count                          统计匹配记录总数
  count by FIELD                 按字段分组统计
  count by DURATION [FIELD]      时间分桶：count by 5m、1h、1d（默认读取 ts 字段）
  fields                         发现数据集中所有字段名
  sum FIELD                      对数字字段求和
  avg FIELD                      对数字字段求平均
  min FIELD                      数字字段的最小值
  max FIELD                      数字字段的最大值
  sort FIELD [asc|desc]          排序结果
  limit N                        取前 N 条记录
  head N                         limit 的别名
```

### 表达式层（DSL）

当第一个参数以 `.`、`not ` 或 `|` 开头时激活：

```
qk 'EXPRESSION' [FILES...]

.field                         访问顶级字段
.a.b.c                         嵌套字段访问（任意深度）
.field == "value"              相等（DSL 中字符串需加引号）
.field != "value"              不等于
.field > N                     数值比较
.field exists                  字段存在
.field contains "text"         子字符串
.field matches "pattern"       正则
not EXPR                       逻辑 NOT
EXPR and EXPR                  逻辑 AND
EXPR or EXPR                   逻辑 OR
EXPR | pick(.a, .b)            只保留指定字段
EXPR | omit(.a, .b)            移除字段
EXPR | count()                 统计
EXPR | sort_by(.f desc)        排序
EXPR | group_by(.f)            按字段分组统计
EXPR | limit(N)                前 N 条记录
EXPR | skip(N)                 跳过 N 条记录（分页）
EXPR | dedup(.f)               按字段去重
EXPR | sum(.f)                 求和
EXPR | avg(.f)                 平均
EXPR | min(.f)                 最小值
EXPR | max(.f)                 最大值
| STAGE                        跳过过滤，直接进入管道
```

---

## 逗号分隔符

长过滤链现在更易读：

```bash
# 旧风格（仍然有效）
qk where level=error and service=api and latency gt 100 app.log

# 新风格——逗号是 'and' 的别名
qk where level=error, service=api, latency gt 100 app.log

# token 上的尾随逗号也有效
qk where level=error, service=api app.log
```

---

## Shell 安全数值算子

`>` 和 `<` 是 shell 元字符。两种解决方案：

```bash
# 方案一：给过滤器加引号（嵌入语法）
qk where 'latency>100' app.log
qk where 'status>=500' access.log

# 方案二：单词算子——推荐，永远不需要引号
qk where latency gt 100 app.log      # >
qk where latency lt 50 app.log       # <
qk where latency gte 88 app.log      # >=
qk where status lte 499 access.log   # <=
```

---

## 嵌套 JSON

用点号访问任意深度的字段：

```bash
# 两级嵌套
qk where response.status=503 app.log
qk where context.region=us-east app.log

# 三级嵌套
qk where pod.labels.app=api k8s.log
qk '.request.headers.x-trace exists' app.log

# DSL——嵌套字段的过滤 + 投影
qk '.response.status >= 500 | pick(.ts, .service, .response.status)' app.log
qk '| group_by(.context.region)' app.log
```

### JSON 编码的字符串字段

如果字段值本身是 JSON 字符串（`"payload": "{\"level\":\"error\"}"`），可以结合 jq 使用：

```bash
# 用 jq 解码字符串字段，再用 qk 查询
cat app.log | jq -c '.payload = (.payload | fromjson)' | qk where payload.level=error

# 完整管道：qk 预过滤 → jq 解码 → qk 聚合
cat app.log | qk where service=api | jq -c '.meta = (.metadata | fromjson)' | qk count by meta.env
```

---

## 输出格式

```bash
qk --fmt ndjson where level=error app.log   # NDJSON（默认）
qk --fmt pretty where level=error app.log   # 缩进 JSON（替代 jq .）
qk --fmt table where level=error app.log    # 对齐表格
qk --fmt csv where level=error app.log      # CSV（可在 Excel 打开）
qk --fmt raw where level=error app.log      # 原始源行

# --fmt 必须在查询表达式之前
qk --fmt table where level=error app.log    # ✅
qk where level=error --fmt table app.log    # ❌
```

---

## 支持的输入格式（自动检测）

| 格式 | 检测方式 | 说明 |
|------|---------|------|
| NDJSON | 每行以 `{` 开头 | 每行一个 JSON 对象 |
| JSON | 文件以 `[` 或 `{` 开头 | 完整 JSON 文档或数组 |
| YAML | `---` 头部或 `.yml`/`.yaml` 扩展名 | 支持多文档 |
| TOML | `.toml` 扩展名 | 整个文件 = 一条记录 |
| CSV | 逗号分隔的头部行 | `.csv` 扩展名 |
| TSV | `.tsv` 扩展名 | |
| logfmt | `key=value key2=value2` 模式 | Go 服务常用 |
| Gzip | 魔数 `0x1f 0x8b` / `.gz` 扩展名 | 透明解压 |
| 纯文本 | 回退 | 每行 → `{"line": "..."}` |

---

## 架构

```
输入 → 格式检测器 → 解析器 → Record IR → 查询引擎 → 输出渲染器
                                         ↑
                             快速层（关键字）| DSL 层（表达式）
```

所有格式在查询前都被规范化为统一的 `Record` 中间表示。查询引擎永远不知道数据来自哪种格式。详见 [`STRUCTURE.md`](./STRUCTURE.md) 查看完整代码库地图。

---

## 性能目标

| 场景 | 目标 | 对比 |
|------|------|------|
| 1 GB NDJSON，简单过滤 | <2s | ripgrep: ~1s（无解析），jq: ~30s |
| 1 GB NDJSON，group_by | <5s | awk: ~8s |
| 10000 个文件，递归 | <3s | ripgrep: ~1s |

---

## 项目文档

| 文件 | 用途 |
|------|------|
| [`README.md`](./README.md) | 本文件——项目概览和语法参考 |
| [`tutorial/`](./tutorial/) | 所有 11 种支持格式的开箱即用测试文件 |
| [`COMMANDS.md`](./COMMANDS.md) | 所有命令一览——复制即用参考（使用 `tutorial/`） |
| [`TUTORIAL.md`](./TUTORIAL.md) | 含可运行示例的完整教程 |
| [`STRUCTURE.md`](./STRUCTURE.md) | 架构和文件逐一说明 |
| [`PROGRESS.md`](./PROGRESS.md) | 变更日志——每个会话的新增/修改/删除 |
| [`LESSON_LEARNED.md`](./LESSON_LEARNED.md) | 踩坑日志和经验总结 |
| [`CLAUDE.md`](./CLAUDE.md) | AI 辅助开发规则 |

---

## 开发

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt
cargo check
echo '{"level":"error","msg":"timeout","service":"api"}' | cargo run -- where level=error
```

---

## 路线图

- [x] Phase 0 — 项目脚手架和架构设计
- [x] Phase 1 — 格式检测 + NDJSON/logfmt/CSV 解析器 + Record IR
- [x] Phase 2 — 快速关键字查询层（where / select / count / sort / limit）
- [x] Phase 3 — 并行处理（rayon）+ mmap + SIMD 搜索
- [x] Phase 4 — 表达式 DSL 层（nom 解析器 + 求值器）
- [x] Phase 5 — 完整格式支持（YAML / TOML / gzip）
- [x] Phase 6 — 输出格式（table / 颜色 / --explain）
- [x] Phase 7 — 统计聚合 + pretty 输出 + 字段发现
- [x] Phase 8 — 文字算子 + CSV 改进（startswith/endswith/glob, --no-header, 类型强转）
- [x] Phase 9 — 类型强转 + 警告（--cast, 聚合类型不匹配警告）

---

## 许可证

MIT

# qk — 一个工具，替代所有它们

[English README](./README.md)

[![CI](https://github.com/handsomevictor/qk/actions/workflows/ci.yml/badge.svg)](https://github.com/handsomevictor/qk/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey)]()
[![Version](https://img.shields.io/badge/version-0.1.0-green)]()

`qk` 是一个快速的终端结构化查询工具。
用一套统一的命令、一套语法、无需任何格式标志，替代 `grep`、`awk`、`sed`、`jq`、`yq`、`cut`、`sort | uniq`。

> **由 Claude Code 全程构建** — 本项目由多 Agent Claude Code 系统自主开发，零人工代码干预。

---

## 为什么选 qk？

### 任务对比

| 任务 | 传统工具 | qk |
|------|----------|----|
| 过滤错误日志 | `grep "error" app.log \| awk '{print $3, $5}'` | `qk where level=error select ts msg` |
| 查询 JSON API 日志 | `cat req.json \| jq '.[] \| select(.status > 499) \| .path'` | `qk where status>499 select path` |
| 按字段统计 | `awk '{print $2}' \| sort \| uniq -c \| sort -rn` | `qk count by service` |
| 跨格式查询 | ❌ 单一工具无法做到 | `qk where level=error *.log *.json` |
| 嵌套字段访问 | `jq '.response.headers["x-trace"]'` | `qk select response.headers.x-trace` |
| 多条件过滤 | `grep \| awk 'cond1 && cond2'` | `qk where level=error, service=api` |
| Shell 安全数值比较 | `awk '$5 > 100'`（有 shell 元字符风险） | `qk where latency gt 100` |
| 时间序列分桶 | ❌ 无标准单一工具方案 | `qk count by 5m` |

### 功能对比矩阵

| 功能 | grep | awk | sed | jq | yq | **qk** |
|------|:----:|:---:|:---:|:--:|:--:|:------:|
| 自动格式检测 | ❌ | ❌ | ❌ | ❌ | partial | ✅ |
| 嵌套字段访问 | ❌ | ❌ | ❌ | ✅ | ✅ | ✅ |
| 跨格式查询 | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| 聚合（sum/avg/count） | ❌ | 手动 | ❌ | partial | partial | ✅ |
| 时间序列分桶 | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Shell 安全数值算子 | ❌ | partial | ❌ | ✅ | ✅ | ✅ |
| 透明 gzip 解压 | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| 多种输出格式 | ❌ | ❌ | ❌ | partial | partial | ✅ |
| 交互式 TUI | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| 单一二进制 <5 MB | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 启动时间 | <1ms | <1ms | <1ms | <5ms | <10ms | <2ms |

---

## 功能特性

### 输入与格式支持

| | |
|--|--|
| **9 种自动检测格式** | NDJSON、JSON 数组、YAML（多文档）、TOML、CSV、TSV、logfmt、纯文本、gzip |
| **透明 gzip 解压** | `data.csv.gz`、`app.log.gz`、`events.tsv.gz` — 直接读取，无需 `gunzip` |
| **记录级模型** | 匹配完整的日志条目 / JSON 对象 / YAML 文档，而不只是行 |
| **点路径嵌套** | `pod.labels.app`、`response.headers.x-trace` — 任意深度 |

### 查询语言

| | |
|--|--|
| **两套语法** | 快速关键字层（覆盖 80% 场景）+ 表达式 DSL（复杂逻辑） |
| **过滤算子** | `=` `!=` `>` `<` `>=` `<=` `~=`（正则）`contains` `startswith` `endswith` `glob` `between` `exists` |
| **Shell 安全单词算子** | `gt` `lt` `gte` `lte` — 避免 `>` / `<` 在 shell 中的引号问题 |
| **相对时间** | `where ts gt now-5m` — 自动识别 RFC 3339、Unix epoch 秒、epoch 毫秒 |
| **逗号语法** | `where level=error, service=api, latency gt 100` — 可读 AND 链 |
| **逻辑算子** | 两套语法均支持 `and` / `or` / `not` |

### 聚合与分析

| | |
|--|--|
| **数值聚合** | `sum`、`avg`、`min`、`max` |
| **分组统计** | `count by FIELD` — 分组并计数 |
| **时间分桶** | `count by 5m` / `count by 1h` / `count by 1d` — 默认最新桶在前 |
| **类型分布** | `count types FIELD` — 显示 number/string/bool/null/missing 分布 |
| **去重** | `dedup` / `count unique FIELD` |
| **字段发现** | `fields` — 列出数据集中所有字段名 |

### 输出与显示

| | |
|--|--|
| **5 种输出格式** | `ndjson`（默认）· `pretty`（缩进 JSON）· `table` · `csv` · `raw` |
| **语义颜色** | error=红，warn=黄，info=绿，HTTP 5xx=粗体红；管道时自动关闭 |
| **自动限制** | 终端输出默认最多 20 条；通知框显示在输出末尾；`--all` / `-A` 禁用 |
| **统计模式** | `--stats` 将记录数和耗时输出到 stderr |
| **交互式 TUI** | `--ui` 启动全屏浏览器（上限 50,000 条，防止 OOM） |

### 开发体验

| | |
|--|--|
| **位置无关标志** | `--fmt`、`--cast`、`--stats`、`--quiet`、`--all` 可出现在命令的任意位置 |
| **配置文件** | `~/.config/qk/config.toml` — `default_fmt`、`default_limit`、`no_color`、`default_time_field` |
| **可操作错误提示** | 拼错的标志显示"你是否想输入 --quiet？"；错误的 `--cast` 类型列出有效选项 |
| **类型强转** | `--cast FIELD=number` 在查询前强制指定字段类型 |
| **类型警告** | `latency > "abc"` 只警告一次，返回无结果，而不是静默给出错误答案 |
| **`==` 检测** | 给出"你是否想用 `=`？"而不是静默不匹配 |
| **并行处理** | `rayon` 文件级并行，文件数线性扩展 |
| **流式处理** | 纯过滤的 stdin 查询以流式运行 — O(输出) 内存，支持 2 GB+ 文件 |

---

## 项目文档

| 用途 | 英文版 | 中文版 |
|------|--------|--------|
| 概览与快速开始 | [`README.md`](./README.md) | [`README_CN.md`](./README_CN.md) ← 当前文件 |
| 完整命令参考 | [`docs/COMMANDS.md`](./docs/COMMANDS.md) | [`docs/COMMANDS_CN.md`](./docs/COMMANDS_CN.md) |
| 错误命令示例与修复 | [`docs/COMMANDS_WRONG.md`](./docs/COMMANDS_WRONG.md) | [`docs/COMMANDS_WRONG_CN.md`](./docs/COMMANDS_WRONG_CN.md) |
| 完整教程与示例 | [`docs/TUTORIAL.md`](./docs/TUTORIAL.md) | [`docs/TUTORIAL_CN.md`](./docs/TUTORIAL_CN.md) |
| 架构与文件说明 | [`docs/STRUCTURE.md`](./docs/STRUCTURE.md) | [`docs/STRUCTURE_CN.md`](./docs/STRUCTURE_CN.md) |
| Rust 入门指南 | [`docs/RUST_GUIDE.md`](./docs/RUST_GUIDE.md) | [`docs/RUST_GUIDE_CN.md`](./docs/RUST_GUIDE_CN.md) |
| 常见问题 | [`docs/FAQ.md`](./docs/FAQ.md) | — |
| 贡献指南 | [`CONTRIBUTING.md`](./CONTRIBUTING.md) | — |
| Release 与 Homebrew 发布指南 | [`docs/RELEASE.md`](./docs/RELEASE.md) | — |
| 路线图 | [`docs/ROADMAP.md`](./docs/ROADMAP.md) | — |
| 变更日志 | [`docs/PROGRESS.md`](./docs/PROGRESS.md) | [`docs/PROGRESS_CN.md`](./docs/PROGRESS_CN.md) |
| 踩坑日志与经验总结 | [`docs/LESSON_LEARNED.md`](./docs/LESSON_LEARNED.md) | [`docs/LESSON_LEARNED_CN.md`](./docs/LESSON_LEARNED_CN.md) |
| 示例数据文件 | [`tutorial/`](./tutorial/) | — |

---

## 安装

### 从源码编译

需要 Rust ≥ 1.75：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

git clone https://github.com/handsomevictor/qk.git
cd qk
cargo install --path .
```

或者直接从 git 安装（无需先克隆）：

```bash
cargo install --git https://github.com/handsomevictor/qk
```

### Homebrew（macOS / Linux）— v0.1.0 发布后可用

```bash
brew tap handsomevictor/qk
brew install qk
```

### 预编译二进制 — v0.1.0 发布后可用

x86_64 和 aarch64 平台（Linux、macOS、Windows）的预编译二进制将发布于
[GitHub Releases](https://github.com/handsomevictor/qk/releases) 页面。

---

## 快速开始

> **完整命令参考：** 见 [`COMMANDS_CN.md`](./docs/COMMANDS_CN.md)（中文）或 [`COMMANDS.md`](./docs/COMMANDS.md)（英文）
> — 涵盖所有算子、所有格式、所有标志，复制即用。

### 教程文件

`tutorial/` 目录包含所有格式的开箱即用测试文件，无需任何准备：

```bash
cd tutorial

qk count app.log           # 25 条 — NDJSON（2~3 级嵌套）
qk count access.log        # 20 条 — NDJSON（嵌套 client/server）
qk count k8s.log           # 20 条 — NDJSON（3 级：pod.labels.app）
qk count data.json         # 8  条 — JSON 数组
qk count services.yaml     # 6  条 — YAML 多文档
qk count config.toml       # 1  条 — TOML
qk count users.csv         # 15 条 — CSV
qk count events.tsv        # 20 条 — TSV
qk count services.logfmt   # 16 条 — logfmt
qk count notes.txt         # 20 条 — 纯文本
qk count app.log.gz        # 25 条 — 透明 gzip 解压
```

### 常用模式

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

# 选择特定字段
qk where level=error select ts service msg app.log

# 统计与聚合
qk count by service app.log
qk where level=error avg latency app.log
qk sum latency app.log

# 时间序列分桶（默认最新桶在前）
qk count by 5m app.log
qk count by 1h ts asc app.log      # 时间正序

# 排序与限制
qk sort latency desc limit 10 app.log

# DSL 模式用于复杂逻辑
qk '.level == "error" and .latency > 1000' app.log
qk '.response.status >= 500 | pick(.ts, .service, .response.status)' app.log
qk '| group_by(.context.region)' app.log

# 管道：先过滤再统计
qk where level=error app.log | qk count by service

# 范围过滤
qk where latency between 100 500 app.log

# 相对时间过滤（最近 5 分钟的事件）
qk where ts gt now-5m app.log

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
  where FIELD>VALUE              数值大于（需引号或用单词算子）
  where FIELD gt VALUE           数值大于（shell 安全）
  where FIELD lt VALUE           数值小于（shell 安全）
  where FIELD gte VALUE          数值大于等于（shell 安全）
  where FIELD lte VALUE          数值小于等于（shell 安全）
  where FIELD~=PATTERN           正则匹配
  where FIELD contains TEXT      子字符串匹配
  where FIELD startswith PREFIX  前缀匹配
  where FIELD endswith SUFFIX    后缀匹配
  where FIELD glob PATTERN       通配符匹配（* 和 ?）
  where FIELD between LOW HIGH   闭区间（数值或时间戳）
  where FIELD exists             字段存在检查
  where FIELD gt now-5m          相对时间（支持 s/m/h/d 后缀）
  where A=1 and B=2              逻辑 AND
  where A=1 or B=2               逻辑 OR
  where A=1, B=2                 逗号 = 'and' 的别名

TRANSFORM:
  select FIELD [FIELD...]        只保留这些字段
  count                          统计匹配记录总数
  count by FIELD                 按字段分组统计
  count by DURATION [FIELD]      时间分桶（5m、1h、1d，默认字段：ts）
  count by DURATION FIELD asc    时间分桶，时间正序
  count unique FIELD             统计不同值的数量
  count types FIELD              值类型分布
  fields                         发现所有字段名
  sum FIELD                      数值求和
  avg FIELD                      数值求平均
  min FIELD                      最小值
  max FIELD                      最大值
  sort FIELD [asc|desc]          排序
  limit N                        取前 N 条
  head N                         limit 的别名

FLAGS（位置无关——可出现在命令的任意位置）：
  --fmt ndjson|pretty|table|csv|raw
  --cast FIELD=TYPE[,FIELD=TYPE]
  --stats                        将处理统计输出到 stderr
  --quiet / -q                   抑制 stderr 警告
  --all / -A                     禁用自动限制
  --no-color                     禁用 ANSI 颜色
  --explain                      打印解析后的查询 AST
  --ui                           交互式 TUI 浏览器
```

### 表达式层（DSL）

当第一个参数以 `.`、`not ` 或 `|` 开头时激活：

```
qk 'EXPRESSION' [FILES...]

.field                         访问顶级字段
.a.b.c                         嵌套字段访问
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
EXPR | limit(N)                前 N 条
EXPR | skip(N)                 跳过 N 条（分页）
EXPR | dedup(.f)               按字段去重
EXPR | sum(.f)                 求和
EXPR | avg(.f)                 平均
EXPR | min(.f)                 最小值
EXPR | max(.f)                 最大值
| STAGE                        跳过过滤，直接进入管道
```

---

## 输出格式

```bash
qk --fmt ndjson where level=error app.log   # NDJSON（默认）
qk --fmt pretty where level=error app.log   # 缩进 JSON（替代 jq .）
qk --fmt table  where level=error app.log   # 对齐表格
qk --fmt csv    where level=error app.log   # CSV（可在 Excel 打开）
qk --fmt raw    where level=error app.log   # 原始源行

# 所有标志均位置无关——以下写法完全等价：
qk --fmt table where level=error app.log
qk where level=error --fmt table app.log
qk where level=error app.log --fmt table
```

在 `~/.config/qk/config.toml` 中设置持久默认值：

```toml
default_fmt = "pretty"
```

---

## 支持的输入格式

| 格式 | 检测方式 | 说明 |
|------|---------|------|
| NDJSON | 每行以 `{` 开头 | 每行一个 JSON 对象 |
| JSON | 文件以 `[` 或 `{` 开头 | 完整文档或数组 |
| YAML | `---` 头部或 `.yml`/`.yaml` 扩展名 | 支持多文档 |
| TOML | `.toml` 扩展名 | 整个文件 = 一条记录 |
| CSV | 逗号分隔的头部行 | `.csv` 扩展名 |
| TSV | `.tsv` 扩展名 | |
| logfmt | `key=value key2=value2` 模式 | Go 服务常用 |
| Gzip | 魔数 `0x1f 0x8b` / `.gz` 扩展名 | 透明解压任意内层格式 |
| 纯文本 | 回退 | 每行 → `{"line": "..."}` |

---

## 配置文件

`~/.config/qk/config.toml`（支持 XDG 规范）：

```toml
default_fmt         = "pretty"   # ndjson | pretty | table | csv | raw
default_limit       = 20         # 自动限制上限（0 = 禁用）
no_color            = false      # true 始终禁用 ANSI 颜色
default_time_field  = "ts"       # count by DURATION 的默认时间戳字段
```

查看当前配置：`qk config show`
恢复默认值：`qk config reset`

---

## 架构

```
输入 → 格式检测器 → 解析器 → Record IR → 查询引擎 → 输出渲染器
                                         ↑
                             快速层（关键字）| DSL 层（表达式）
```

所有格式在查询前均规范化为统一的 `Record` IR，查询引擎对数据来源格式一无所知。
详见 [`STRUCTURE.md`](./docs/STRUCTURE.md) 查看完整代码库地图。

---

## 性能

| 场景 | 目标 | 对比 |
|------|------|------|
| 1 GB NDJSON，简单过滤 | < 2 s | ripgrep ~1 s（无解析），jq ~30 s |
| 1 GB NDJSON，group_by | < 5 s | awk ~8 s |
| 10,000 个文件，递归 | < 3 s | ripgrep ~1 s |

实现要点：
- `rayon` 文件级并行（使用所有 CPU 核心）
- 文件 ≥ 64 KiB 使用 `mmap`
- `memmem` SIMD 加速的 `contains` 匹配

---

## 开发

```bash
cargo test                          # 运行全部 446 个测试
cargo clippy -- -D warnings         # 零警告要求
cargo fmt                           # 提交前格式化
cargo bench                         # 运行基准测试

# 快速冒烟测试
echo '{"level":"error","msg":"timeout","service":"api"}' | cargo run -- where level=error
```

---

## 路线图

### 已完成

- [x] Phase 0 — 项目脚手架与架构设计
- [x] Phase 1 — 格式检测 + NDJSON/logfmt/CSV 解析器 + Record IR
- [x] Phase 2 — 快速关键字查询层（where / select / count / sort / limit）
- [x] Phase 3 — 并行处理（rayon）+ mmap + SIMD 搜索
- [x] Phase 4 — 表达式 DSL 层（nom 解析器 + 求值器）
- [x] Phase 5 — 完整格式支持（YAML / TOML / gzip）
- [x] Phase 6 — 输出格式（table / 颜色 / --explain / TUI）
- [x] Phase 7 — 统计聚合 + pretty 输出 + 字段发现
- [x] Phase 8 — 字符串算子 + CSV 改进（startswith/endswith/glob、--no-header、--cast）
- [x] Phase 9 — UX 打磨：位置无关标志、可操作错误提示、时间桶排序、自动限制方框、TUI 上限

### 待实现

- [ ] **T-01** — 修复正则重复编译：每次查询只编译一次（对正则过滤提速 10~100×）
- [ ] **v0.1.0** — GitHub Release + 预编译二进制 + Homebrew tap（见 [`RELEASE.md`](./docs/RELEASE.md)）
- [ ] **T-04** — 流式文件读取：将当前的全量加载替换为分块/流式方案，彻底消除 >1 GB 文件的 OOM 风险
- [ ] **T-05** — 流式 stdin：支持 `tail -f file | qk …` 不再阻塞等待 EOF
- [ ] **T-06** — 跨文件 JOIN：`qk join users.csv orders.csv on id`
- [ ] **T-07** — `--output-file` 标志：将结果写入文件而不是 stdout
- [ ] **T-08** — 监听模式：文件变更时自动重新执行查询（`--watch`）

---

## 已知限制

- **不支持 `tail -f`：** qk 需要读到 stdin 的 EOF 才开始处理。`tail -f file | qk ...` 会无限阻塞。**临时替代：** 使用 `tail -n 1000 file | qk ...` 处理有限输入。纯过滤的 stdin 查询（如 `cat bigfile | qk where level=error`）以流式运行，支持 2 GB+ 文件。
- **全量物化：** 以文件路径（而非 stdin）传入时，qk 在求值前加载整个文件。>1 GB 的文件在 <16 GB RAM 的机器上可能 OOM；改用 stdin 管道可走流式路径。
- **`--fmt raw` 与聚合：** 聚合结果（`count`、`sum`、`avg` 等）没有原始源行，`--fmt raw` 对每条聚合记录输出空行。聚合输出建议使用默认的 `ndjson` 或 `pretty`。

---

## 许可证

MIT

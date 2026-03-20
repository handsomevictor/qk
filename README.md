# qk — 一个工具，替代所有它们

`qk` 是一个快速结构化查询工具，用于终端。它用单一、一致的接口替代了 `grep`、`awk`、`sed`、`jq`、`yq`、`cut`、`sort | uniq` 等工具。

不再需要堆叠管道只是为了从日志文件中提取两个字段。不再需要根据格式在 `jq` 语法和 `awk` 语法之间切换。一个二进制，一套语法，支持所有格式。

---

## 为什么选 qk？

| 任务 | 以前的做法 | qk 的做法 |
|------|-----------|----------|
| 过滤错误日志 | `grep "error" app.log \| awk '{print $3, $5}'` | `qk where level=error select ts msg` |
| 查询 JSON API 日志 | `cat req.json \| jq '.[] \| select(.status > 499) \| .path'` | `qk where status>499 select path` |
| 按字段统计次数 | `awk '{print $2}' \| sort \| uniq -c \| sort -rn` | `qk count by service` |
| 跨格式查询 | ❌ 一个工具无法做到 | `qk where error!=null *.log *.json` |
| 嵌套字段访问 | `jq '.response.headers["x-trace"]'` | `qk select response.headers.x-trace` |

---

## 功能特性

- **自动检测格式** — JSON、NDJSON、YAML、TOML、CSV、logfmt、syslog、nginx/CLF、纯文本，无需 `-f json` 参数
- **记录级模型** — 匹配完整的日志条目 / JSON 对象 / YAML 文档，而不仅仅是行
- **两套语法** — 快速关键字层（覆盖 80% 场景）+ 表达式 DSL（覆盖剩余 20%）
- **结构化输出** — 默认输出 NDJSON，方便管道给下一个 `qk` 或其他工具
- **并行处理** — 通过 `rayon` 使用所有 CPU 核心，文件数量线性扩展（Phase 3）
- **透明解压** — 直接读取 `.gz` 和 `.zst` 文件（Phase 5）
- **Rust 编写** — 二进制体积 <5MB，启动时间 <2ms

---

## 安装

### 从源码编译（开发版）

```bash
# 前提：安装 Rust 工具链
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 克隆并编译
git clone https://github.com/YOUR_USERNAME/qk.git
cd qk
cargo build --release

# 二进制文件位于：
./target/release/qk

# 可选：安装到 PATH
cargo install --path .
```

### 预编译二进制

即将通过 GitHub Releases 提供。

---

## 快速开始

```bash
# 过滤包含 "error" 的行（替代 grep）
qk where level=error app.log

# 选择特定字段（替代 awk）
qk where level=error select ts service msg app.log

# 统计出现次数（替代 sort | uniq -c）
qk count by level app.log

# 查询 JSON 无需了解 schema
qk where status>499 select path status latency requests.json

# 同时查询多种格式的文件
qk where error!=null *.log *.json k8s/*.yaml

# 排序并限制结果数量
qk where latency>200 sort latency desc limit 20 app.log

# 管道：先过滤再统计
qk where level=error app.log | qk count by service

# 查看解析结果（调试模式）
qk where level=error --explain app.log
```

---

## 语法参考

### 快速层（关键字语法）

```
qk [FILTER] [TRANSFORM] [FILES...]

FILTER:
  where FIELD=VALUE          精确匹配
  where FIELD!=VALUE         不等于
  where FIELD>VALUE          数值大于
  where FIELD<VALUE          数值小于
  where FIELD>=VALUE         数值大于等于
  where FIELD<=VALUE         数值小于等于
  where FIELD~=PATTERN       正则匹配
  where FIELD contains TEXT  包含子字符串
  where FIELD exists         字段存在
  where FIELD=A or FIELD=B   逻辑 OR
  where FIELD=A and OTHER=B  逻辑 AND（链式默认）

TRANSFORM:
  select FIELD [FIELD...]    只保留这些字段
  count                      统计匹配记录总数
  count by FIELD             按字段分组统计
  sort FIELD [asc|desc]      排序
  limit N                    取前 N 条记录
```

### 表达式层（DSL 语法）

用单引号包裹，使用表达式 DSL（Phase 4）：

```
qk 'EXPRESSION' [FILES...]

.field                       访问字段
.a.b.c                       嵌套字段访问
.field == value              相等
.field > value               比较
not .field                   逻辑非
.a and .b                    逻辑与
.a or .b                     逻辑或
expr | fn()                  管道进函数
pick(.a, .b)                 选择字段
omit(.a, .b)                 删除字段
group_by(.field)             分组
map(expr)                    变换每条记录
count()                      统计
sort_by(.field)              按字段排序
```

---

## 输出格式

```bash
qk where level=error app.log              # NDJSON（默认，适合管道）
qk where level=error app.log --fmt raw    # 原始匹配行
```

---

## 支持的格式

| 格式 | 自动检测方式 | 说明 |
|------|------------|------|
| NDJSON | 每行以 `{` 开头 | 每行一个 JSON 对象 |
| JSON | 文件以 `[` 或 `{` 开头 | 完整 JSON 文档 |
| YAML | `---` 头部或 `.yml`/`.yaml` 扩展名 | 支持多文档（Phase 5）|
| TOML | `.toml` 扩展名 | Phase 5 完整支持 |
| CSV | 逗号分隔的头部行 | |
| TSV | 制表符分隔 | |
| logfmt | `key=value key2=value2` 模式 | Go 服务常用 |
| 纯文本 | 回退 | 行 = 记录，`line` 字段 |

---

## 架构概览

详见 [`STRUCTURE.md`](./STRUCTURE.md)。

简版：

```
输入 → 格式检测器 → 解析器 → Record IR → 查询引擎 → 输出渲染器
                                         ↑
                             快速层（关键字）| 表达式层（DSL）
```

所有格式在查询前都被规范化为统一的 `Record` 中间表示。查询引擎永远不知道数据来自哪种格式。

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
| [`README.md`](./README.md) | 本文件——项目概览和用法 |
| [`TUTORIAL.md`](./TUTORIAL.md) | 完整教程——安装、编译、使用、开发 |
| [`STRUCTURE.md`](./STRUCTURE.md) | 架构和文件逐一说明 |
| [`PROGRESS.md`](./PROGRESS.md) | 变更日志——每个会话的新增/修改/删除 |
| [`LESSON_LEARNED.md`](./LESSON_LEARNED.md) | 踩坑记录——遇到的 bug、调试过程、经验总结 |
| [`CLAUDE.md`](./CLAUDE.md) | AI 辅助开发规则（Claude Code 自动读取）|

---

## 开发

```bash
# 运行测试
cargo test

# 用样本数据运行
echo '{"level":"error","msg":"timeout","service":"api"}' | cargo run -- where level=error

# 检查 lint
cargo clippy -- -D warnings

# 格式化代码
cargo fmt

# 只检查编译（不生成二进制，速度最快）
cargo check
```

---

## 路线图

- [x] Phase 0 — 项目脚手架和架构设计
- [x] Phase 1 — 格式检测 + NDJSON/logfmt/CSV 解析器 + Record IR
- [x] Phase 2 — 快速关键字查询层（where / select / count / sort / limit）
- [ ] Phase 3 — 并行处理（rayon）+ mmap + SIMD 搜索
- [ ] Phase 4 — 表达式 DSL 层（nom 解析器 + 求值器）
- [ ] Phase 5 — 完整格式支持（YAML / TOML / syslog / gz / zst）
- [ ] Phase 6 — 输出美化（table / 颜色 / --explain 增强）
- [ ] Phase 7 — GitHub Releases + 安装脚本

---

## 许可证

MIT

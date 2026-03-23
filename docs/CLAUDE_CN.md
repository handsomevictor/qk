# CLAUDE.md — Claude Code 使用说明

本文件在每次会话开始时被 Claude Code 自动读取。请无需提示地遵守以下所有规则。

---

## 项目身份

- **工具名称**：`qk`
- **语言**：Rust（稳定工具链，最低 1.75）
- **目的**：单一 CLI 工具，替代 grep / awk / sed / jq / yq / cut / sort+uniq
- **架构**：输入 → 格式检测器 → 解析器 → Record IR → 查询引擎 → 输出渲染器

---

## 每次会话的强制规则

### 1. 始终更新 PROGRESS.md
每次有意义的修改（新文件、新函数、Bug 修复、重构）后，在 `PROGRESS.md` 中添加一条记录。
格式：
```
## YYYY-MM-DD — 简短描述
### 新增 / 修改 / 删除
- 要点
```

### 2. 每个非平凡 Bug 都要更新 LESSON_LEARNED.md
遇到编译错误、逻辑 Bug、令人惊讶的 Rust 行为，或任何需要多次尝试才能解决的问题——用 LL-NNN 格式记录在 `LESSON_LEARNED.md` 中。

### 3. 文件变动时始终更新 STRUCTURE.md
新建文件、重命名文件、或文件职责发生变化时，更新 `STRUCTURE.md` 中的树形结构和说明表格。不要让 STRUCTURE.md 与实际代码脱节。

### 4. 保持函数短小
每个函数最多约 40 行。超过时就拆分。

### 5. 库代码中禁止 unwrap()
使用 `?` 传播或显式错误处理。`unwrap()` 只允许在：
- `main.rs` 的最顶层
- 测试代码中

### 6. 错误消息必须有行动指导性
返回错误时，要包含足够的上下文，让用户知道是哪个文件/行/字段导致的。
**不好**：`Err("parse error")`
**好**：`Err(format!("failed to parse field '{}' at line {}: {}", field, line_num, e))`

### 7. 每个解析器都要有测试
`src/parser/` 中的每个格式解析器，必须有至少一个文件内的单元测试，以及 `tests/formats.rs` 中的集成测试。

### 8. 优化前先跑基准测试
做任何性能方面的声明之前，先运行 `cargo bench` 并将数字记录在 `PROGRESS.md` 中。

---

## 代码风格

- **注释**：英文（标识符和文档注释，即 `///`）
- **格式化**：每次提交前运行 `cargo fmt`，无例外
- **Lint**：`cargo clippy -- -D warnings` 必须零报告通过
- **文档注释**：每个公有函数和结构体都加 `///`
- **命名**：遵循 Rust 惯例（函数用 snake_case，类型用 CamelCase，常量用 SCREAMING_SNAKE）

---

## 文档语言

所有 markdown 文档（README.md、TUTORIAL.md、STRUCTURE.md、PROGRESS.md、LESSON_LEARNED.md、CLAUDE.md）均用**中文**书写。
代码中的注释和标识符保持**英文**。

---

## 架构约束

- `record.rs` 中的 `Record` 类型是跨越解析器→查询边界的**唯一**类型。解析器不得向查询层泄漏格式专用类型。
- 查询引擎（`query/`）不得直接 import `parser/`。
- 性能关键路径（NDJSON 分行、字段查找）必须使用 `memchr`，不用 `str::find`。
- 文件级并行使用 `rayon::par_iter()`。不要手动 spawn 线程。

---

## 添加新格式的工作流

1. 在 `detect.rs` 的 `Format` 枚举中添加变体
2. 在 `detect::sniff()` 中添加检测逻辑
3. 创建 `src/parser/<格式>.rs`，包含 `parse(input: &str) -> Result<Vec<Record>>` 函数
4. 在 `src/parser/mod.rs` 中注册
5. 在 `tests/fixtures/` 中添加 fixture 文件
6. 在 `tests/formats.rs` 中添加测试
7. 更新 `STRUCTURE.md` 和 `PROGRESS.md`

---

## 当前阶段

**Phase 3~7 — 全部完成 ✅**

已实现功能：
- 格式自动检测（NDJSON / JSON / CSV / TSV / logfmt / YAML / TOML / Gzip / 纯文本）
- NDJSON、logfmt、CSV、YAML、TOML、纯文本解析器，gzip 透明解压
- 快速查询层：where / select / count / count by / sort / limit / head / fields / sum / avg / min / max（含 `and/or/not/exists/contains/regex`）
- DSL 表达式层：`.field == val | pick() | omit() | count() | sort_by() | group_by() | limit() | skip() | dedup() | sum() | avg() | min() | max()`
- 嵌套字段点号访问（`response.status`）
- 管道（stdin 自动检测 NDJSON）
- 输出格式：ndjson（默认）/ pretty（缩进JSON）/ table（comfy-table 彩色）/ csv / raw
- `--fmt` / `--color` / `--no-color` / `--explain` 标志
- rayon 文件级并行，mmap 大文件优化（≥ 64 KiB）
- 语义感知 ANSI 彩色输出：error=红，warn=黄，info=绿，msg=亮白，ts=暗淡，HTTP状态码分范围着色
- **206 个测试全部通过**（138 单元 + 68 集成）
- `cargo clippy -- -D warnings` 零报告

**重要使用规范：**
- `--fmt`、`--color` 等标志必须置于查询表达式**之前**（clap `trailing_var_arg` 语义）
- DSL 模式触发条件：首参数以 `.`、`not ` 或 `|` 开头
- TOML 文件固定输出 1 条记录（整个文档作为一个对象）
- 颜色优先级：`--no-color` > `--color` > `NO_COLOR` env > tty 自动检测

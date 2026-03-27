# LESSON LEARNED — 踩坑日志

每个有意义的 Bug、编译错误、设计失误或令人惊讶的发现都记录在这里。
目标：同样的问题永远不要调试两次。

每条记录格式：
```
## LL-NNN — 简短标题
- **日期**: YYYY-MM-DD
- **Phase**: Phase N
- **症状**: 出了什么问题 / 哪里让人困惑
- **根本原因**: 为什么会发生
- **修复**: 怎么解决的
- **经验**: 需要记住的通用规则
```

---

## LL-001 — Cargo 版本与新 crate 不兼容

- **日期**: 2026-03-20
- **Phase**: Phase 1
- **症状**: `cargo build` 报错 `feature 'edition2024' is required`，clap 4.6.0 无法下载
- **根本原因**: 系统 Rust 版本是 1.76.0（2024年初），而 clap 4.6.0 的 `Cargo.toml` 使用了 `edition = "2024"`，需要 Cargo 1.79+ 才能解析
- **修复**: 运行 `rustup update stable` 将 Rust 升级到 1.94.0，之后所有依赖正常下载
- **经验**: 开始新 Rust 项目前，先运行 `rustup update stable` 确保工具链是最新的。如果无法升级，可以用 `cargo update <crate>@<version> --precise <旧版本>` 降级特定依赖

---

## LL-002 — 模块命名与 crate 命名冲突（未实际发生，预防性记录）

- **日期**: 2026-03-20
- **Phase**: Phase 1
- **症状**: 如果在 `src/parser/mod.rs` 中声明 `pub mod csv`，然后在 `src/parser/csv.rs` 中写 `csv::ReaderBuilder::new()`，有可能引起歧义
- **根本原因**: 在 Rust 中，`csv` 可以指 (1) 外部 crate（来自 Cargo.toml）或 (2) 当前模块内的子模块。实际上，在 `src/parser/csv.rs` **文件内部**，`csv` 指向外部 crate；子模块路径是 `crate::parser::csv`，不会在文件内产生歧义
- **修复**: 实际上没有冲突，正常编译。但如果将来遇到，用 `::csv::ReaderBuilder` 显式从 crate 根访问外部 crate
- **经验**: Rust 中文件内的名称解析：外部 crate 名在当前文件作用域有效；子模块名只在声明它的父模块文件（`mod.rs`）内才引入同名符号

---

## LL-003 — unused import 警告提示了遗漏的 import

- **日期**: 2026-03-20
- **Phase**: Phase 2
- **症状**: `eval.rs` 中保留了 `use crate::util::error::{QkError, Result}`，但 `QkError` 实际未用，触发 unused import 警告
- **根本原因**: 最初 eval 函数计划直接创建 `QkError`，后来改为只使用 `Result`，忘了清理 import
- **修复**: 将 `use crate::util::error::{QkError, Result}` 改为 `use crate::util::error::Result`
- **经验**: Rust 的 `unused_imports` 警告非常有价值。运行 `cargo build` 后看警告，比看错误还重要——警告往往指出了代码逻辑的遗漏

---

## 预置 Rust 常见陷阱（项目通用）

以下是此类 CLI/解析项目中初学者和中级 Rust 开发者最常遇到的错误，预先收录，遇到真实案例后更新。

### 解析器代码中的生命周期错误
`nom` 解析器返回借用自输入的 `&str` 切片。如果将解析结果存储在生命周期超过输入缓冲区的结构体中，编译器会报错。修复：要么克隆字符串（`.to_string()`），要么在结构体定义中携带生命周期参数。

### release 构建中的 unwrap() panic
开发期间在 `Result` 和 `Option` 上用 `.unwrap()` 很方便。但一行格式错误的输入就会让整个进程崩溃。修复：在 Phase 3 之前，将热路径切换为 `?` 传播或带有回退的显式 `match`。

### rayon 和 Send 约束
`rayon` 的并行迭代器要求闭包捕获实现 `Send`。如果不小心捕获了非 `Send` 类型（如 `Rc`、裸指针、或 `MutexGuard`），编译报错会令人困惑。修复：在共享状态中优先用 `Arc` 而不是 `Rc`，在 spawn 任务前释放锁。

### serde 字段重命名
像 `"Content-Type"` 这样的 JSON 键不能直接作为 Rust 字段名。用 `#[serde(rename = "Content-Type")]` 或 `#[serde(rename_all = "kebab-case")]` 处理。

### memmap2 和空文件
在某些平台上，对零长度文件调用 mmap 会 panic。在调用 `MmapOptions::new().map()` 之前，始终检查 `file.metadata()?.len() > 0`。

### select 语法中字段名与文件名的歧义
`qk select ts msg app.log` 中，`app.log` 能被识别为文件是因为 `looks_like_file()` 函数检测到了 `.log` 扩展名。对于没有扩展名的文件（如 `data`），需要用 `./data` 或绝对路径来消除歧义。

---

## LL-004 — clap trailing_var_arg 吞噬后续标志

- **日期**: 2026-03-20
- **Phase**: Phase 6
- **症状**: `qk where level=error --fmt table file.ndjson` 报错 `IO error reading '--fmt': No such file or directory`
- **根本原因**: CLI 中 `args` 字段使用了 `trailing_var_arg = true`，clap 一旦遇到第一个位置参数（`where`），就把后续所有内容——包括 `--fmt table`——都当成 `args` 的值，而非命名标志
- **修复**: 将 `--fmt` 等标志放在查询表达式**之前**：`qk --fmt table where level=error file.ndjson`
- **经验**: `trailing_var_arg = true` 是"一切都捕获"模式。命名标志（flags）**必须**出现在第一个位置参数之前。在 CLI 的帮助文档和 TUTORIAL.md 中要明确说明这一点

---

## LL-005 — DSL 模式检测仅覆盖 `.` 前缀

- **日期**: 2026-03-20
- **Phase**: Phase 4
- **症状**: `qk 'not .level == "info"'` 或 `qk '| count()'` 报错 `IO error reading 'not ...'`，而不是执行 DSL 查询
- **根本原因**: `determine_mode` 只检测第一个参数是否以 `.` 开头来判断 DSL 模式。`not` 和 `|` 开头的表达式也是合法 DSL，但被路由到了关键字模式，然后误当文件路径处理
- **修复**: 在 `determine_mode` 中扩展条件：`first.starts_with("not ")` 或 `first.starts_with('|')` 也触发 DSL 模式
- **经验**: 模式检测需要覆盖所有合法的起始 token。添加新语法（如 `not expr`）时，记得同步更新路由逻辑

---

## LL-006 — TOML 节头 `[section]` 被误识别为 JSON 数组

- **日期**: 2026-03-20
- **Phase**: Phase 5
- **症状**: `detect::tests::detects_toml_section_by_content` 失败，`[server]\nport = 8080` 被判定为 `Json` 而非 `Toml`
- **根本原因**: `detect_from_content` 中，`if trimmed.starts_with('[')` 直接返回 `Format::Json`，TOML 节头的检测（`looks_like_toml`）排在其后，永远不会被触发
- **修复**: 在 `[` 分支内先调用 `looks_like_toml`；并将 `looks_like_toml` 的节头检测改为严格模式——括号内不含 `{`、`"`、`'` 才视为 TOML 节头
- **经验**: 格式检测的优先级顺序至关重要。当两种格式共享同一起始字符时（`[` 既是 JSON 数组又是 TOML 节头），必须在同一分支内做更细粒度的区分，而不是依赖顺序先后

---

## LL-007 — 过时的已安装二进制掩盖了源码修复

- **日期**: 2026-03-21
- **Phase**: Phase 7
- **症状**: `--explain` 仍然显示中文；`gt`/`lt` 算子仍然报错；逗号分隔符不工作——即使源码已经正确
- **根本原因**: `~/.cargo/bin/qk` 是从另一个目录（`~/Downloads/qk`）安装的旧二进制。在任何地方运行 `qk` 都会用到这个过时的二进制。`~/Documents/GitHub/qk` 中的源码修改从未编译进已安装的二进制
- **修复**: 在正确的项目目录执行 `cargo install --path .`；通过 `which qk` 确认，再用 `qk --explain where level=error` 验证显示英文输出
- **经验**: 修改源码后，`cargo run` 使用本地构建，但已安装的二进制（`~/.cargo/bin/qk`）只有通过 `cargo install --path .` 才会更新。调试源码前，务必用 `which qk` 和冒烟测试确认当前使用的是哪个二进制

---

## LL-008 — fast 层正则（`~=`）是使用 `str::contains()` 的存根而非真正的正则

- **日期**: 2026-03-21
- **Phase**: Phase 7
- **症状**: `qk where 'msg~=.*timeout.*' app.log` 无结果。`qk where 'msg~=timeout' app.log` 能工作。用户反馈正则过滤坏了
- **根本原因**: `src/query/fast/eval.rs` 中的 `eval_regex()` 有 TODO 注释："Simple regex: just check if the string contains the pattern for now. Phase 4 will add a proper regex engine."Phase 4 只给 DSL 层加了真正的正则；fast 层从未更新，`~=` 实际执行的是字面量子串匹配（`str::contains(pattern)`）而非正则匹配。`.*timeout.*` 被当作字面字符串搜索——永远找不到
- **修复**: 将 `str::contains()` 替换为 `regex::Regex::new(pattern)?.is_match()`，使用 `Cargo.toml` 中已有的 `regex` crate
- **经验**: 跨阶段增量实现功能时，要追踪所有需要更新的位置。"Phase N will add X"这类 TODO 注释必须转化为被追踪的任务，而不能作为静默存根遗留。正则测试应该验证 `.*` 模式能真正匹配，而不只是字面子串

---

## LL-009 — zsh glob 展开破坏含 `*` 的正则模式

- **日期**: 2026-03-21
- **Phase**: Phase 7
- **症状**: `qk where msg~=.*timeout.* app.log` 触发 `zsh: no matches found: msg~=.*timeout.*`
- **根本原因**: zsh（以及开启了 globbing 的 bash）将 `*` 视为 glob 模式。在 `qk` 收到参数之前，zsh 就尝试将 `msg~=.*timeout.*` 作为文件 glob 展开。找不到匹配文件时，zsh 直接报错，而不是把字面字符串传递给 `qk`
- **修复**: 给参数加引号：`qk where 'msg~=.*timeout.*' app.log`。单引号可以阻止所有 shell 展开
- **经验**: 任何包含 shell 元字符（`*`、`?`、`[`、`]`、`{`、`}`、`~`）的参数都必须加引号。在所有展示正则语法的地方都要显眼地说明这一点。DSL 层也有相同问题：`qk '.msg matches ".*fail.*"'`——外层单引号是必须的

---

## LL-010 — 子句关键字前的尾随逗号导致解析错误

- **日期**: 2026-03-21
- **Phase**: Phase 7
- **症状**: `qk where level=error, select ts service msg app.log` 报错 `cannot parse filter 'select'`。用户期望尾随逗号作为 `select`、`count`、`avg` 等前的装饰性分隔符
- **根本原因**: 在 `parse_where_clause` 中，当过滤 token 上检测到尾随逗号（如 `level=error,`）时，代码无条件推入 `LogicalOp::And` 并 `continue` 回循环顶部。循环顶部又调用 `parse_filter` 处理下一个 token（`select`），而 `select` 不是合法的过滤表达式——于是报错
- **修复**: 在推入 `And` 并继续之前，检查下一个 token 是否是子句终止关键字（`select`、`count`、`sort`、`limit`、`head`、`fields`、`sum`、`avg`、`min`、`max`、`where`）或文件路径。如果是，则 `break` 而非 `continue`。尾随逗号由此被视为可选标点
- **经验**: 分隔符 token（逗号、`and`）应该"贪婪但有边界"——它们暗示后面还有输入，但只有后面的内容是合法的延续时才成立。在确定解析方向前，始终检查前瞻 token

---

## LL-011 — NDJSON 混合类型字段在没有警告的情况下静默产生错误结果

- **日期**: 2026-03-21
- **Phase**: Phase 9
- **症状**: 当某些记录的 `latency` 字段为字符串 `"None"` 或 `"unknown"` 时，`qk avg latency app.log` 返回静默错误的结果。没有报错，也没有任何提示表明有记录被跳过
- **根本原因**: `value_as_f64()` 对非数字字符串返回 `None`，导致 `filter_map` 从聚合中静默丢弃这些记录。调用方完全不知道跳过了多少条记录或原因
- **修复**: 在 `stat_agg` 中将 `filter_map(...and_then(value_as_f64))` 替换为新的 `collect_numeric_field()` 辅助函数，区分三种情况：(1) 空值类字符串 → 静默跳过；(2) 可解析字符串 → 使用；(3) 意外字符串 → 跳过 **并** 向 stderr 发出 `[qk warning]`
- **经验**: 静默跳过行的聚合很危险——用户无法判断结果是否可信。始终通过警告让"意外跳过"可见。使用 stderr，这样警告不会破坏管道输出

---

## LL-029 — 将新标志穿透多个 eval 层

- **问题**: 添加 `--case-sensitive` 需要同时修改 `cli.rs`、`main.rs`、`fast/parser.rs`、`fast/eval.rs`、`dsl/eval.rs` 和 `tui/app.rs`。遗漏任何一处调用都会导致静默回归（标志无效）或编译错误
- **经验**: 对于影响 eval 行为的标志，最安全的模式是：
  1. 将标志存入查询结构体（如 `FastQuery.case_sensitive`）——它随查询传递到所有地方，包括流式路径（`eval_one`）
  2. 对于 DSL 层，以显式参数形式传入 `eval()`，因为 `DslQuery` 从用户文本解析，而非 CLI 参数
  3. 搜索所有受影响 eval 函数的调用点（`cargo build` 捕获编译错误；`grep` 找出 TUI 等静默传递点）
- **如何应用**: 未来任何修改查询求值行为的跨层 CLI 标志都应遵循这一两步模式

---

## LL-031 — stdin 流式路径假定格式为 NDJSON，而不检测实际格式

- **问题**：`curl ... | jq '.data[]' | qk where ...` 产生数千条 "trailing characters" 警告，且无任何有效输出。流式路径（`run_stdin_streaming_keyword`）在未检测 stdin 实际格式的情况下直接进入 NDJSON 逐行循环。`jq '.data[]'` 默认输出多行缩进 JSON 对象，每一行（如 `  "brand": "Kaiko",`）本身都不是合法 JSON 对象。
- **修复**：在 `run_stdin_streaming_keyword` 开头，用 `BufReader::fill_buf()` 偷看第一块缓冲区字节（不消耗），对其运行 `detect::sniff`；若格式不是 `Ndjson`，回退到 batch 路径（读完整 stdin，按检测格式解析，求值，输出）。只有 stdin 确实是 NDJSON 时才进入流式循环。
- **关键 API**：`BufReader::fill_buf()` 填充内部缓冲区并返回 `&[u8]` 引用，不推进读游标——后续读取从同一位置开始。这是"偷看不消耗"的标准模式。
- **如何应用**：任何在读取前假定 stdin 格式的代码路径，都应先对偷看缓冲区调用 `detect::sniff`。

---

## LL-030 — `serde_json::from_str` 只读取一个顶层值；处理 JSON 文件应使用流式迭代器

- **问题**: 包含多个顶层对象的 JSON 文件（`{…}\n{…}`）会导致 `qk` 报错 `trailing characters at line N column 1`。`serde_json::from_str` 只消费一个值，剩余内容触发错误
- **修复**: 改用 `serde_json::Deserializer::from_str(input).into_iter::<Value>()`。流式迭代器依次读取每个完整的顶层值，透明支持单个对象、数组、连续拼接对象三种情况
- **遇到的 Clippy 问题**:
  - `while_let_on_iterator` — 应使用 `for result in stream`，而非 `while let Some(result) = stream.next()`
  - `unused_mut` — 写 `let stream =` 而非 `let mut stream =`（`for` 循环消费迭代器，无需 `mut`）
- **如何应用**: 凡是 JSON 输入源可能包含多个顶层文档的场景（API 日志转储、追加的格式化响应等），均应使用流式反序列化器，而非 `from_str`/`from_reader`

---

<!-- 在这一行上方添加新记录，递增 LL-NNN -->

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

<!-- 在这一行上方添加新记录，递增 LL-NNN -->

# PROGRESS — 开发进度日志

每个工作会话都按倒序（最新在前）记录在这里。每条记录包含：**新增**、**修改**、**删除**，以及相关的**基准测试数据**（如有）。

格式：
```
## YYYY-MM-DD — 会话标题
### 新增
### 修改
### 删除
### 基准测试（如有测量）
### 备注
```

---

## 2026-03-27 — 修复拼接 JSON 解析 + 默认不区分大小写

### 新增
- `--case-sensitive` / `-S` 标志：所有字符串过滤（`=`、`contains`、`startswith`、`endswith`、`glob`）默认不区分大小写；使用 `-S` 启用严格大小写匹配
- `-S` / `--case-sensitive` 已加入 `BOOL_FLAGS` 和 `ALL_KNOWN_FLAGS`，支持任意位置使用

### 修改
- `src/cli.rs` — 新增 `case_sensitive: bool` 字段
- `src/main.rs` — 将 `case_sensitive` 传入 `run_keyword` 和 `run_dsl`
- `src/query/fast/parser.rs` — `FastQuery.case_sensitive` 字段
- `src/query/fast/eval.rs` — `value_matches_str`、`FilterOp::Ne/Contains/StartsWith/EndsWith` 支持 `case_sensitive`
- `src/query/dsl/eval.rs` — `eval()`、`compare_eq()`、`compare_contains()` 接受 `case_sensitive` 参数
- `src/tui/app.rs` — DSL eval 调用传入 `false`（TUI 默认不区分大小写）
- `src/parser/mod.rs` — 用流式 `Deserializer::from_str().into_iter()` 替换 `serde_json::from_str`，支持拼接的完整 JSON 对象文件（修复 `trailing characters` 错误）

### 备注
- 拼接 JSON：文件中包含多个顶层对象（`{…}\n{…}\n{…}`）——常见于多次追加 API 响应的文件——现已正确解析；`serde_json` 的流式迭代器可透明处理单个对象、数组、连续拼接对象三种情况

---

## 2026-03-26 — 默认不区分大小写 + `--case-sensitive` / `-S` 标志

### 新增
- `--case-sensitive` / `-S` CLI 标志：启用后，字符串比较要求精确大小写匹配
- `FastQuery` 结构体新增 `case_sensitive: bool` 字段（默认 `false`）
- `dsl::eval::eval()` 新增 `case_sensitive: bool` 参数

### 修改
- `src/cli.rs`：新增带完整文档注释的 `case_sensitive: bool` 字段
- `src/main.rs`：在 `BOOL_FLAGS` 和 `ALL_KNOWN_FLAGS` 中注册 `--case-sensitive` / `-S`；将 `case_sensitive` 传递至 `run_keyword`、`run_dsl`、`run_stdin_streaming_keyword`
- `src/query/fast/eval.rs`：`eval_filter` 在 `!case_sensitive` 时对 `Eq`、`Ne`、`Contains`、`StartsWith`、`EndsWith` 的两端均转小写；`value_matches_str` 新增 `case_sensitive` 参数
- `src/query/dsl/eval.rs`：`compare_eq` 和 `compare_contains` 接受 `case_sensitive`；`eval_expr` 将其传递给 `And`/`Or`/`Not`
- `src/tui/app.rs`：TUI eval 调用更新——始终传入 `case_sensitive = false`
- 文档：`COMMANDS.md`、`COMMANDS_CN.md`、`TUTORIAL.md`、`TUTORIAL_CN.md`、`FAQ.md`、`README.md`、`README_CN.md` 全部更新

### 备注
- `glob` 和 `regex`/`matches` **不受影响**：glob 始终不区分大小写（编译时加了 `(?i)`）；regex 大小写由用户在 pattern 中通过 `(?i)` 控制
- `count by` 和 `count unique` 也不受影响——静默合并 "Error" + "error" 桶会改变聚合语义

---

## 2026-03-21 — --cast 类型强转 + 自动类型不匹配警告

### 新增
- `src/util/cast.rs` — 新模块：`CastType` 枚举、`parse_cast_map()`、`apply_casts()`、`coerce_one()`、`is_null_like()`；10 个单元测试
- `--cast FIELD=TYPE` CLI 标志（可重复使用）——在查询运行前将任意字段强转为目标类型。支持的类型：`number`（num/float/int）、`string`（str/text）、`bool`（boolean）、`null`（none）、`auto`
- `tutorial/mixed.log` — 12 条 NDJSON 记录，字段类型刻意混合：`latency`（Number/String/"None"/"NA"/"unknown"/null）、`score`（Number/null/"N/A"/"pending"）、`active`（Bool/"yes"/"no"）、`status`（Number）
- `util/cast::is_null_like()` — 共用的空值检测逻辑（与 CSV `coerce_value` 使用相同的集合）

### 修改
- `src/query/fast/eval.rs`:
  - `eval()` 返回类型 → `Result<(Vec<Record>, Vec<String>)>`（第二个元素为警告列表）
  - `aggregate()` → `Result<(Vec<Record>, Vec<String>)>`
  - `stat_agg()` → 使用新的 `collect_numeric_field()` 辅助函数，对意外字符串值发出警告
  - `collect_numeric_field()`: Number → 正常使用；可解析字符串 → 静默使用；空值类字符串 → 静默跳过；其他字符串 → **输出警告到 stderr**；Null → 静默跳过
- `src/query/dsl/eval.rs`:
  - `eval()` 返回类型 → `Result<(Vec<Record>, Vec<String>)>`
  - `apply_stages()` / `apply_stage()` → 从每个阶段累积警告
  - 四个聚合函数替换为带警告的变体：`aggregate_sum_with_warn`、`aggregate_avg_with_warn`、`aggregate_min_with_warn`、`aggregate_max_with_warn`
  - 共用 `collect_numeric_field_dsl()` 辅助函数，具有相同的空值/警告逻辑
- `src/main.rs`:
  - `run_keyword()` / `run_dsl()` — 在 `load_records()` 之后调用 `apply_casts()`，从 eval 中解构 `(Vec<Record>, Vec<String>)`，通过 `print_warnings()` 打印警告
  - `print_warnings()` — 将每条警告输出到 stderr
- `src/cli.rs` — 新增 `--cast` 参数（`Vec<String>`，value_name 为 `FIELD=TYPE`）
- `src/util/mod.rs` — 新增 `pub mod cast`
- `COMMANDS.md` — 新增"混合类型字段"章节，含类型表、警告示例、--cast 参考；更新快速语法摘要
- `TUTORIAL.md` — 在"多种文件格式"中新增"混合类型字段与类型强转"小节；更新文件引用表；更新目录

### 备注
- **226 个测试全部通过**（168 单元 + 58 集成）
- 警告仅输出到 **stderr**——stdout 输出不受影响，管道到 jq/grep 可正常使用
- 以下空值类字符串静默跳过：`""`、`"None"`、`"none"`、`"null"`、`"NULL"`、`"NA"`、`"N/A"`、`"n/a"`、`"NaN"`、`"nan"`
- 警告上限：最多显示 5 条具体警告，之后显示"... and N more suppressed"
- `--cast number`：空值类字符串 → `Value::Null`（无警告）；无法解析 → 警告 + 字段从记录中移除

---

## 2026-03-21 — 新算子：startswith / endswith / glob + CSV --no-header + 类型强转

### 新增
- `startswith` 过滤算子 — `qk where msg startswith connection app.log`；前缀检查，大小写敏感
- `endswith` 过滤算子 — `qk where path endswith users access.log`；后缀检查，大小写敏感
- `glob` 过滤算子 — `qk where msg glob '*timeout*' app.log`；shell 风格 `*`/`?` 通配符，默认大小写不敏感；通过 `glob_to_regex()` 转换为正则 `(?i)^...$` 实现
- `--no-header` CLI 标志 — 将 CSV/TSV 第一行视为数据而非表头；列名为 `col1`、`col2`、`col3`...
- CSV 类型强转（`coerce_value()`）— 整数/浮点字符串 → `Value::Number`；`"None"/"null"/"NA"/"N/A"/"NaN"/""` → `Value::Null`；`"true"/"false"` → `Value::Bool`；其他 → `Value::String`。适用于表头模式和无表头模式

### 修改
- `src/query/fast/parser.rs` — 在 `FilterOp` 枚举中新增 `StartsWith`、`EndsWith`、`Glob`；新增三个算子的解析分支；加入 `is_query_keyword()`
- `src/query/fast/eval.rs` — 新增 `StartsWith`、`EndsWith`、`Glob` 的匹配分支；新增 `eval_glob()` 和 `glob_to_regex()` 辅助函数；修复 `eval_regex()` 存根（原为 `str::contains`，改为真正的正则匹配）
- `src/parser/csv.rs` — 拆分为 `parse_with_header()` 和 `parse_headerless()`；新增 `coerce_value()` 进行类型强转；两种模式均对所有单元格值强转
- `src/parser/mod.rs` — 为 `parse()` 新增 `no_header: bool` 参数；传递给 `csv::parse()`
- `src/cli.rs` — 新增 `--no-header`（`no_header: bool`）标志
- `src/main.rs` — 将 `no_header` 贯穿 `run()` → `run_keyword()` / `run_dsl()` → `load_records()` → `read_one_file()` → `parser::parse()`
- `COMMANDS.md` — 在过滤章节新增 `startswith`、`endswith`、`glob` 示例；在 CSV 章节新增无表头示例；扩展纯文本章节，含所有文字算子；更新快速语法摘要
- `TUTORIAL.md` — 在过滤章节新增 `startswith`、`endswith`、`glob` 小节；新增 CSV 无表头 + 类型强转章节；扩展纯文本章节，含完整功能矩阵；更新快速参考
- `STRUCTURE.md` — 更新 `cli.rs`、`parser/csv.rs`、`query/fast/parser.rs`、`query/fast/eval.rs` 的描述

### 备注
- **216 个测试全部通过**（148 单元 + 68 集成）
- `cargo clippy -- -D warnings` 零报告
- 现有 CSV 测试已更新：因类型强转，age 字段现在为 `Value::Number(30)` 而非 `Value::String("30")`
- `glob` 算子大小写不敏感：`'msg glob *ERROR*'` 也能匹配 `error`、`Error`
- 始终对 glob/正则模式加引号：`'msg glob *timeout*'` 而非 `msg glob *timeout*`（zsh 会展开 glob）

---

## 2026-03-21 — 修复：子句关键字前的尾随逗号 + COMMANDS.md 逗号风格

### 修改
- `src/query/fast/parser.rs` — 修复 `parse_where_clause`：`select`/`count`/`avg` 等关键字前的尾随逗号现在能优雅地终止 where 子句，而非报错。在推入 `LogicalOp::And` 前新增 `next_is_clause_end` 前瞻检查
- `COMMANDS.md` — 全面更新：所有过滤+转换组合改用逗号风格（`where level=error, select ...`、`where level=error, count by ...`、`where level=error, avg ...`、`where level=error, sort ... limit ...`），覆盖每个格式章节
- `LESSON_LEARNED.md` — 新增 LL-010：子句关键字前尾随逗号导致解析错误

### 备注
- `where FIELD=VALUE, select F1 F2 FILE` 现在可以正常使用——尾随逗号仅为装饰
- 两种风格均有效：`where level=error select ts msg` 和 `where level=error, select ts msg`
- 全部 206 个测试仍然通过

---

## 2026-03-21 — tutorial/ 目录：11 种格式的测试数据 + 文档全面改版

### 新增
- `tutorial/app.log` — 25 条 NDJSON 记录，2~3 级嵌套 JSON（`context.*`、`request.headers.*`、`response.*`、`user.*`）
- `tutorial/access.log` — 20 条 NDJSON HTTP 访问日志，嵌套 `client.*` 和 `server.*`
- `tutorial/k8s.log` — 20 条 NDJSON Kubernetes 事件，3 级嵌套（`pod.labels.app/team/version`、`container.restart_count`）
- `tutorial/encoded.log` — 7 条 NDJSON 记录，字段值为 JSON 字符串（用于 qk+jq 示例）
- `tutorial/data.json` — 8 条 JSON 数组记录，含嵌套 `address.*`
- `tutorial/services.yaml` — 6 文档 YAML 多文档，嵌套 `resources.*` 和 `healthcheck.*`
- `tutorial/config.toml` — 包含 6 个嵌套节的完整 TOML 配置（server/database/cache/auth/logging/feature_flags）
- `tutorial/users.csv` — 15 行 CSV（id/name/age/city/role/active/score/department/salary）
- `tutorial/events.tsv` — 20 行 TSV（ts/event/service/severity/region/duration_ms/user_id）
- `tutorial/services.logfmt` — 16 条 logfmt 记录（ts/level/service/msg/host/latency/version）
- `tutorial/notes.txt` — 20 行纯文本日志
- `tutorial/app.log.gz` — app.log 的 gzip 压缩版（透明解压演示）

### 修改
- `LESSON_LEARNED.md` — 新增 LL-007（过时的已安装二进制）、LL-008（正则存根）、LL-009（zsh glob 展开）
- `COMMANDS.md` — 全面重写：将内联 heredoc 数据准备替换为 `cd tutorial` + 每种格式独立章节
- `README.md` — 新增"立即试用"章节（`tutorial/` 快速入门）；更新文档表格
- `TUTORIAL.md` — 将内联数据准备替换为 `tutorial/` 参考表；将"多种文件格式"章节替换为按格式分节的完整示例（JSON 数组、YAML、TOML、CSV、TSV、logfmt、gzip、纯文本）

### 备注
- 全部 12 个数据文件已验证：每个文件的 `qk count` 返回预期记录数
- 本次会话无代码修改；全部测试仍然通过（206 个通过）

---

## 2026-03-21 — 修复：正则引擎、二进制重装、文档更新

### 修改
- `src/query/fast/eval.rs` — `eval_regex()` 原为使用 `str::contains()` 的存根，而非真正的正则；替换为 `regex::Regex::new()`，使 `~=.*pattern.*` 能正确工作
- `TUTORIAL.md` — 修复 `tail -f /var/log/app.log`（Mac 上路径不存在）改为 `tail -f /path/to/app.log`；新增 zsh glob 展开对正则模式的警告
- `COMMANDS.md` — 同样修复 `tail -f`；新增正则模式的 zsh 引号提示

### 备注
- 正则 bug 根本原因：fast 层的 `eval_regex` 有 TODO 注释"Phase 4 will add a proper regex engine"，但 Phase 4 只给 DSL 层加了正则；fast 层始终是存根
- 全部 206 个测试仍然通过；`cargo clippy -- -D warnings` 零报告

---

## 2026-03-20 — Phase 7：统计聚合 + skip/dedup + pretty 输出 + fields 发现

### 新增

**DSL 新管道阶段（`src/query/dsl/ast.rs` + `parser.rs` + `eval.rs`）：**
- `| sum(.field)` — 对字段求和，返回 `{"sum": N}`
- `| avg(.field)` — 求平均值，返回 `{"avg": N}`
- `| min(.field)` — 最小值，返回 `{"min": N}`
- `| max(.field)` — 最大值，返回 `{"max": N}`
- `| skip(N)` — 跳过前 N 条记录（分页 / offset）
- `| dedup(.field)` — 按字段值去重，保留每个值的第一次出现

**快速关键字层新命令（`src/query/fast/parser.rs` + `eval.rs`）：**
- `qk fields` — 发现数据集中所有字段名（按字母排序），替代手工查看 schema
- `qk sum FIELD` — 字段求和
- `qk avg FIELD` — 字段平均
- `qk min FIELD` — 字段最小值
- `qk max FIELD` — 字段最大值
- `qk head N` — `limit` 的别名（更直觉的分页语法）

**Pretty 输出格式（`src/output/pretty.rs`）：**
- `--fmt pretty` — 带缩进的 JSON，块间空行，替代 `jq .`
- 支持 `--color` 模式：键名加粗青色，字符串绿色，数字黄色，布尔洋红，null 暗淡

**集成测试（14 个新增）：**
- `tests/dsl_layer.rs` 新增 7 个测试（skip/dedup/sum/avg/min/max/pretty）
- `tests/fast_layer.rs` 新增 7 个测试（fields/sum/avg/min/max/head/pretty）

### 修改
- `src/cli.rs` — `OutputFormat` 新增 `Pretty` 变体
- `src/output/mod.rs` — 新增 `pub mod pretty`，`Pretty` 格式分发
- `src/query/dsl/ast.rs` — `Stage` 枚举新增 6 个变体
- `src/query/dsl/parser.rs` — 新增 6 个阶段解析器，6 个单元测试
- `src/query/dsl/eval.rs` — 实现新阶段，新增 6 个单元测试
- `src/query/fast/parser.rs` — `Aggregation` 枚举新增 5 个变体，`parse_stat` 辅助函数，`head` 别名
- `src/query/fast/eval.rs` — 实现 `fields_discovery`/`stat_agg`，新增 5 个单元测试

### 删除
- 无

### 基准测试
未测量

### 备注
- **206 个测试全部通过**（138 单元 + 68 集成）
- `cargo clippy -- -D warnings` 零报告
- 痛点分析：`awk` 求和需手写状态变量 → `qk sum field`；`jq .` pretty-print → `--fmt pretty`；`sort|uniq -c` 字段去重 → `| dedup(.f)`；无 schema 发现工具 → `qk fields`；无分页 → `| skip(N)` + `head N`

---

## 2026-03-20 — 颜色输出 + 文档完善

### 新增

**颜色系统（output/color.rs）：**
- 新建 `src/output/color.rs` — 语义感知 ANSI 着色器
  - `paint_record()`: 结构符号暗淡，字段名粗体青色，字符串绿色
  - `level`/`severity` 字段值：error=粗体红，warn=粗体黄，info=粗体绿，debug=蓝，trace=暗淡
  - `msg`/`message` 字段值：亮白色（最醒目）
  - `ts`/`timestamp` 字段值：暗淡（背景噪声）
  - `error`/`exception` 字段值：红色
  - HTTP `status` 字段值：200-299=绿，300-399=青，400-499=黄，500-599=粗体红
  - 布尔值：洋红色；数字：黄色；null：暗淡
  - 13 个单元测试（覆盖全部语义规则）

**CLI 颜色控制：**
- `src/cli.rs` 新增 `--color` 标志（强制开启，覆盖 NO_COLOR env 和 tty 检测）
- `use_color()` 优先级：`--no-color` > `--color` > `NO_COLOR` env > tty 自动检测
- `src/output/ndjson.rs` 添加 `use_color: bool` 参数，调用 `color::paint_record()`

**集成测试（5 个）：**
- `no_color_flag_output_is_valid_json` — 验证 --no-color 输出为可解析 JSON
- `color_flag_produces_ansi_codes` — 验证 --color 强制产生 ANSI 码
- `color_flag_error_level_contains_red` — 验证 error 级别使用红色 (31)
- `no_color_flag_takes_priority_over_color_flag` — 验证 --no-color 优先级
- `raw_output_format_returns_original_line` — 验证 raw 格式原样输出

### 修改
- `src/output/mod.rs` — 新增 `pub mod color`，将 `use_color` 传给 ndjson::write
- `TUTORIAL.md` — 全面重写：新增 DSL 语法、管道阶段、颜色方案、各格式、gzip、常用场景
- `STRUCTURE.md` — 全面重写：反映 Phase 1~6 所有文件，含完整数据流图和依赖表格

### 删除
- 无

### 基准测试
未测量

### 备注
- **172 个测试全部通过**（116 单元 + 56 集成）
- `cargo clippy -- -D warnings` 零报告
- 颜色默认仅在真实终端启用（tty 检测），管道传输自动关闭——符合 Unix 惯例

---

## 2026-03-20 — Phase 3~6：并行 + mmap + DSL 层 + 新格式 + 表格/CSV 输出 + 集成测试

### 新增

**性能（Phase 3）：**
- `src/util/mmap.rs` — mmap 大文件读取（≥ 64 KiB），小文件直接 read；5 个单元测试
- `src/util/decompress.rs` — gzip 透明解压（flate2），is_gzip/decompress_gz/inner_filename；3 个单元测试
- `src/main.rs` 重构 — `load_records`（rayon par_iter）、`read_one_file`（gz 透明解压）

**DSL 表达式层（Phase 4）：**
- `src/query/dsl/ast.rs` — 完整 AST 类型（DslQuery、Expr、Stage、CmpOp、Literal）
- `src/query/dsl/parser.rs` — nom v7 解析器；支持 `and/or/not`、`exists`、`contains`、`matches`、管道阶段；13 个单元测试
- `src/query/dsl/eval.rs` — 递归布尔求值 + 6 个管道阶段（pick/omit/count/sort_by/group_by/limit）；memchr SIMD 字符串搜索；regex 匹配；16 个单元测试

**新格式（Phase 5）：**
- `src/parser/yaml.rs` — YAML 解析器（serde_yaml 多文档支持）；5 个单元测试
- `src/parser/toml_fmt.rs` — TOML 解析器（`::toml::Value` 明确路径，避免与 crate 名称冲突）；3 个单元测试
- `src/detect.rs` — 新增 Gzip/Yaml/Toml 变体；改进 `looks_like_toml` 启发式（避免误识别 JSON 数组）；13 个检测测试

**输出格式（Phase 6）：**
- `src/output/table.rs` — comfy-table 对齐表格输出；自动列宽截断（60 字符，`…`）；彩色（青色标题、蓝色数字、黄色布尔、灰色空值）；5 个单元测试
- `src/output/csv_out.rs` — CSV 重序列化，含 RFC 4180 转义；4 个单元测试
- `src/cli.rs` — 新增 Table/Csv 输出格式变体，`--no-color` 标志，`use_color()` 方法

**DSL 模式检测增强：**
- `src/main.rs` — `determine_mode` 扩展：除 `.` 前缀外，`not ` 和 `|` 前缀也触发 DSL 模式

**集成测试：**
- `tests/dsl_layer.rs` — 24 个 DSL 集成测试（全过滤算子、所有管道阶段、文件输入、表格/CSV 输出）
- `tests/formats.rs` — 新增 YAML（4 个）、TOML（4 个）、gzip 解压（1 个）、表格/CSV 输出（2 个）测试

**测试数据：**
- `tests/fixtures/sample.yaml` — 5 条多文档 YAML 日志记录
- `tests/fixtures/sample.toml` — 1 条 TOML 配置记录（扁平格式）

### 修改
- `Cargo.toml` — 新增依赖：rayon、memmap2、nom、regex、serde_yaml、toml、flate2、comfy-table
- `src/detect.rs` — `looks_like_toml` 加严校验：`[{` 不识别为 TOML 节，避免与 JSON 数组冲突
- `src/output/csv_out.rs` — 修正单元测试中 header 顺序（serde_json 无 preserve_order 时按字母排序）
- `TUTORIAL.md` — （待更新 DSL 语法和新格式章节）

### 删除
- 无

### 基准测试
未测量

### 备注
- **154 个测试全部通过**（103 单元 + 51 集成）
- `cargo clippy -- -D warnings` 零报告
- 关键 Bug 修复：`determine_mode` 扩展、`looks_like_toml` 对 JSON 数组的误判、`--fmt` 标志必须置于首位（trailing_var_arg 语义）

---

## 2026-03-20 — Phase 1 + 2：格式检测、解析器、快速查询层

### 新增

**核心模块：**
- `Cargo.toml` — 项目配置，依赖：clap v4、serde_json、indexmap、csv、memchr、thiserror、owo-colors
- `src/util/error.rs` — `QkError` 枚举（IO、Parse、Query、UnsupportedFormat）
- `src/util/mod.rs` — util 模块声明
- `src/record.rs` — `Record` 统一中间表示（`IndexMap<String, Value>` + `raw` + `SourceInfo`），支持点号嵌套字段访问
- `src/detect.rs` — 格式自动检测（前 512 字节魔数 + 启发式）

**解析器：**
- `src/parser/mod.rs` — 解析器分发，包含 `parse_json_document` 辅助函数
- `src/parser/ndjson.rs` — NDJSON 解析器（每行一个 JSON 对象）
- `src/parser/logfmt.rs` — logfmt 解析器（支持引号值）
- `src/parser/csv.rs` — CSV/TSV 解析器（分隔符参数化）
- `src/parser/plaintext.rs` — 纯文本回退解析器

**查询引擎（快速层）：**
- `src/query/mod.rs` — 模块声明
- `src/query/fast/mod.rs` — 快速层模块声明
- `src/query/fast/parser.rs` — 关键字语法解析器（where/select/count/sort/limit）
- `src/query/fast/eval.rs` — 快速查询求值器（过滤、投影、聚合、排序、限制）

**输出：**
- `src/output/mod.rs` — 输出分发
- `src/output/ndjson.rs` — NDJSON 输出渲染器

**入口：**
- `src/cli.rs` — clap CLI 定义（Cli、OutputFormat）
- `src/main.rs` — 主入口，串联完整流水线

**测试：**
- `tests/fast_layer.rs` — 7 个集成测试（stdin 管道、count、链式管道、--explain 等）
- `tests/formats.rs` — 9 个集成测试（NDJSON、logfmt、CSV 各格式的过滤、统计、排序）
- `tests/fixtures/sample.ndjson` — 6 条样本日志记录
- `tests/fixtures/sample.logfmt` — 5 条 logfmt 格式记录
- `tests/fixtures/sample.csv` — 5 条 CSV 格式记录

**文档：**
- `TUTORIAL.md` — 面向 Rust 新手的完整中文教程（安装、编译、用法、开发者指南）
- 全部 markdown 文档改为中文（README.md、STRUCTURE.md、PROGRESS.md、CLAUDE.md、LESSON_LEARNED.md）

### 修改
- `README.md` — 改为中文，更新路线图（Phase 1 + 2 标记完成）
- `STRUCTURE.md` — 改为中文，反映实际文件结构
- `CLAUDE.md` — 改为中文

### 删除
- 无

### 基准测试
未测量（Phase 3 引入 rayon + mmap 后再测）

### 备注
- Rust 工具链从 1.76.0 升级到 1.94.0（旧版本无法编译新版 clap/indexmap）
- **44 个单元测试全部通过**（涵盖 detect、record、parser、query/fast 所有模块）
- **16 个集成测试全部通过**
- 目前仅一个 dead_code 警告（`UnsupportedFormat` 变体，Phase 5 会用到）
- YAML/TOML 目前回退到纯文本解析，Phase 5 添加完整支持

---

## 2025-__ — Phase 0：项目脚手架

### 新增
- `.gitignore` — 排除 `/target/`、IDE 文件、性能分析产物
- `README.md` — 完整项目概览、语法参考、架构摘要
- `PROGRESS.md` — 本文件
- `LESSON_LEARNED.md` — 调试日志
- `STRUCTURE.md` — 架构和文件树

### 修改
- 无（初始提交）

### 删除
- 无

### 备注
- 工具名称：`qk`
- 语言：Rust（稳定工具链）
- 语法设计：两层（快速关键字层 + 表达式 DSL 层）
- 核心架构决定：输入 → 格式检测器 → 解析器 → Record IR → 查询引擎 → 输出渲染器
- 关键 crate 选型：`clap`、`nom`、`rayon`、`memmap2`、`memchr`、`serde`、`csv`、`owo-colors`、`thiserror`

---

<!-- 模板——每次新会话复制此块

## YYYY-MM-DD — Phase N：标题

### 新增
-

### 修改
-

### 删除
-

### 基准测试
| 场景 | 之前 | 之后 |
|------|------|------|
|      |      |      |

### 备注
-

-->

# qk 全面审计报告

基于对项目的深入重新审计（从零开始，不依赖之前的评估结果），以下是关于测试覆盖、性能与稳定性、功能性以及代码质量等方面的全面审计报告。同时，也列出了明确需要改进的地方以及推荐实现的新功能。

## 1. 测试覆盖情况

**优势:**
* **测试用例数量多且覆盖面广:** 目前有 328 个测试用例，涵盖了 `fast_layer`、`dsl_layer` 的核心逻辑、边界情况、异常恢复（如 corrupt lines）以及多种格式（NDJSON, CSV, YAML 等）的解析。
* **边界情况测试完备:** 新增的测试很好地覆盖了 DSL 中的空输入、深层嵌套、缺少闭合括号等边界问题；时间处理也覆盖了 UTC 跨天、时区偏移等。
* **大文件性能测试框架:** `tests/large_file.rs` 提供了 2GB 和 200MB 级别的大文件测试。

**需要改进 / 风险点:**
* **大文件测试未集成到常规 CI:** 大文件测试目前带有 `#[ignore]` 标记，需要手动触发。缺乏自动化的轻量级内存限制测试，无法在日常 PR 中防止内存泄漏或流式退化（如无意间引入缓冲导致 OOM）。
* **DSL 模式流式测试缺失:** 虽然关键字层（fast layer）有流式输入（stdin）的恢复测试，但缺少 DSL 模式处理巨型输入流时的内存稳定性和错误恢复测试。

## 2. 性能与稳定性

**优势:**
* **`Option<String>` 避免空分配:** P4 修复中将聚合结果的原始字符串改为 `Option<String>`，有效减少了内存分配。
* **容错性增强:** 对无法解析的行和计算错误（如聚合时的空字段、除零错误）采用“打印警告并继续”的策略，大幅提高了稳定性。
* **单遍流式处理 (Fast Layer):** `run_stdin_streaming_keyword` 在仅使用 `where` / `select` 时可以做到 O(1) 内存。

**需要改进 / 风险点:**
* **🔴 核心架构缺陷：批处理全量物化 (OOM 风险):**
  这是目前 `qk` 最大的架构瓶颈。当使用文件参数（`qk where level=error app.log`）而不是管道（`cat app.log | qk`），或者使用 DSL 时，`load_records` 会将整个文件加载到 `Vec<Record>` 中。对于 >1GB 的文件，极易在普通机器上发生 OOM。
* **🔴 阻塞式 `tail -f`:**
  目前 `qk` 必须读到 EOF 才会开始处理。因此 `tail -f app.log | qk ...` 会无限阻塞，没有任何输出。这与 `grep` 或 `awk` 的实时流式行为不符。
* **🟢 [DEFERRED] 字符串重复分配导致内存膨胀:**
  底层依赖 `serde_json::Value`，其中的 `Value::String` 无法实现字符串池化（String Interning）。处理数百万条记录时，相同的字段值会反复分配内存。（该问题涉及架构重构，已标记为 deferred 到下一个大版本）。

## 3. 功能性

**优势:**
* **核心功能完备:** 过滤、投影、聚合（`sum`, `avg`, `min`, `max`, `count unique`）、时间分桶（支持日历单位）和算术运算（`map`）均已实现，基本能够替代 `grep + awk + jq` 的日常流水线。
* **双层设计体验好:** fast layer 适合 80% 快速查询，无引号负担；DSL layer 应对复杂逻辑。

**需要改进的地方 / 缺失功能 (新功能建议):**

1. **🟡 数组与字符串处理函数匮乏 (DSL 层):**
   * **数组操作:** 目前 qk 主要处理顶层对象，缺乏对嵌套 JSON 数组的深入操作。例如，无法像 `jq` 那样展平数组（unnest/flatten）、无法映射数组内部元素、也缺乏 `length(.array)` 或 `contains(.array, "value")`（检查值是否在数组中）的函数。
   * **字符串操作:** 缺乏常用的字符串转换和提取功能。例如，`split`, `join`, `replace`, `to_lower`, `to_upper`，以及通过正则提取捕获组（Regex capture groups）。
2. **🟡 数据富化 (Data Enrichment / Join):**
   * `qk` 目前只能将多个文件当作同一个数据流合并处理，**不支持 Lookups 或 Joins**。在日志分析中，常常需要将 `app.log` 中的 `user_id` 与 `users.csv` 进行关联以提取用户名。
3. **🟡 多字段分组 (Multi-field Group By):**
   * 目前 `group_by` 和 `count by` 仅支持单一字段。不支持 `group_by(.level, .service)` 这种常见的 SQL `GROUP BY level, service` 操作。
4. **🟢 输出与格式化增强:**
   * 缺乏对输出日期时间格式的自定义能力（例如将 epoch 格式化为 `YYYY-MM-DD` 字符串输出）。

## 4. 代码质量

**优势:**
* 代码结构清晰，模块化良好 (`detect`, `parser`, `query`, `output`)。
* 错误提示 (`dsl_parse_error`) 带有上下文和 `^^^` 指针，用户体验好。

**需要改进 / 技术债:**
* **解析器与执行引擎耦合:** `load_records` 返回 `Vec<Record>` 的签名限制了流式处理的扩展。需要重构为基于 `Iterator<Item = Result<Record>>` 的流水线模型。

---

## 5. 总结与可操作建议

### 🔴 高优先级 (必须解决的核心痛点)

1. **彻底重构为流式迭代器管道 (Lazy Evaluation):**
   * **目标:** 解决 `tail -f` 阻塞问题和文件批处理时的 OOM 问题。
   * **方案:** 废弃 `load_records` 预先加载所有数据的做法。使 DSL 层和文件读取都支持基于 `Iterator` 的逐条处理，仅在遇到 `sort` 或 `group_by` 时才进行必要的内存缓冲。

### 🟡 中优先级 (强烈推荐的新功能)

1. **实现多字段分组 (Multi-field Grouping):**
   * 允许 `count by level, service` 或 DSL 中的 `group_by(.level, .service)`，输出复合键聚合结果。
2. **增强 DSL 数组与字符串函数:**
   * 添加 `length()`, `split()`, `replace()`, `to_lower()`, `to_upper()`。
   * 添加检查元素是否在数组中的操作符 (如 `in` 或 `contains`)。
3. **增加轻量级 OOM 防护测试:**
   * 编写生成中等规模数据流的自动化测试，断言内存峰值不会随数据量线性增长，并纳入常规 `cargo test`。

### 🟢 低优先级 (长期演进)

1. **实现文件 Join / Lookup 功能:**
   * 允许将小文件（如 CSV 字典）加载到内存作为 Lookup 表，用于富化主流数据。
2. **[DEFERRED] 自定义 Value 枚举与字符串池化:**
   * 替换 `serde_json::Value`，实现零拷贝和字符串驻留，以降低解析阶段的内存和 CPU 开销。
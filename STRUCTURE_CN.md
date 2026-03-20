# STRUCTURE — 代码库权威地图

本文档是代码库的权威地图。每当有文件新增、移动或重大修改时，必须同步更新本文档。

---

## 当前项目结构（Phase 1~6 全部完成）

```
qk/
│
├── Cargo.toml                  # 工作区清单 + 二进制 crate 配置
├── Cargo.lock                  # 锁定的依赖版本（二进制项目必须提交）
│
├── .gitignore
├── README.md                   # 项目概览和使用说明
├── TUTORIAL.md                 # 新手完整教程（安装、编译、用法、DSL、颜色等）
├── PROGRESS.md                 # 变更日志——每个会话记录新增/修改/删除
├── LESSON_LEARNED.md           # 踩坑日志
├── STRUCTURE.md                # 本文件
└── CLAUDE.md                   # AI 辅助开发规则

src/
├── main.rs                     # 入口——解析 CLI、串联整个流水线
│                               #   run_dsl() / run_keyword() / load_records()
│                               #   determine_mode(): . / not  / | → DSL，否则关键字层
│
├── cli.rs                      # CLI 参数定义（clap 结构体）
│                               #   Cli { args, fmt, explain, color, no_color }
│                               #   OutputFormat { Ndjson, Table, Csv, Raw }
│                               #   use_color(): 优先级 --no-color > --color > NO_COLOR env > tty 检测
│
├── detect.rs                   # 格式自动检测
│                               #   读取前 512 字节（魔数 + 启发式）
│                               #   Format 枚举: Ndjson | Json | Csv | Tsv | Logfmt
│                               #               | Yaml | Toml | Gzip | Plaintext
│
├── record.rs                   # 统一 Record IR（中间表示）
│                               #   Record { fields: IndexMap<String, Value>, raw: String, source: SourceInfo }
│                               #   get(key): 支持点号嵌套访问（response.status）
│
├── parser/
│   ├── mod.rs                  # 根据 Format 枚举分发到对应解析器
│   │                           #   parse_json_document(): 处理 JSON 数组或单对象
│   ├── ndjson.rs               # NDJSON 解析器（每行一个 JSON 对象）
│   ├── logfmt.rs               # logfmt 解析器（key=value 对，支持引号值）
│   ├── csv.rs                  # CSV/TSV 解析器（分隔符参数化，头部行→字段名）
│   ├── yaml.rs                 # YAML 解析器（serde_yaml 多文档支持，yaml_to_json 转换）
│   ├── toml_fmt.rs             # TOML 解析器（::toml::Value 显式路径，避免 crate 名歧义）
│   └── plaintext.rs            # 回退——每行变成 Record { line: "..." }
│
├── query/
│   ├── mod.rs                  # 模块声明
│   │
│   ├── fast/                   # 快速关键字层
│   │   ├── mod.rs
│   │   ├── parser.rs           # 解析 "where level=error select ts msg" → FastQuery AST
│   │   │                       #   FilterOp: Eq/Ne/Gt/Lt/Gte/Lte/Regex/Contains/Exists
│   │   └── eval.rs             # 将 FastQuery 应用于 Record 流
│   │                           #   filter_records / aggregate / apply_projection / sort / limit
│   │
│   └── dsl/                    # DSL 表达式层（nom 解析器）
│       ├── mod.rs
│       ├── ast.rs              # DslQuery { filter: Expr, transforms: Vec<Stage> }
│       │                       #   Expr: True | Compare | Exists | And | Or | Not
│       │                       #   Stage: Pick | Omit | Count | SortBy | GroupBy | Limit
│       │                       #          Skip | Dedup | Sum | Avg | Min | Max
│       ├── parser.rs           # nom v7 解析器；支持完整布尔语法和管道阶段
│       └── eval.rs             # 递归布尔求值 + 12 个管道阶段
│                               #   compare_contains: memchr SIMD 子串搜索
│                               #   compare_regex: regex crate 正则匹配
│
├── output/
│   ├── mod.rs                  # 输出分发（根据 OutputFormat 和 use_color）
│   │                           #   render(records, fmt, use_color)
│   ├── color.rs                # 终端彩色 NDJSON 渲染器
│   │                           #   paint_record(): 语义感知着色
│   │                           #   level→红/黄/绿/蓝，msg→亮白，ts→暗，status→HTTP状态色
│   ├── ndjson.rs               # NDJSON 输出（write(records, out, use_color)）
│   ├── pretty.rs               # 缩进 JSON 输出，块间空行（替代 jq .），支持彩色
│   ├── table.rs                # comfy-table 对齐表格（列宽截断 60 字符，彩色表头）
│   └── csv_out.rs              # CSV 重序列化（RFC 4180 转义）
│
└── util/
    ├── mod.rs
    ├── error.rs                # QkError 枚举（Io | Parse | Query | UnsupportedFormat）
    ├── mmap.rs                 # mmap 大文件读取（≥ 64 KiB）+ 小文件直接 read
    └── decompress.rs           # gzip 透明解压（flate2）；is_gzip / decompress_gz

tests/
├── fast_layer.rs               # 集成测试：关键字语法端到端（含 --color / --no-color）
├── dsl_layer.rs                # 集成测试：DSL 表达式层端到端（全算子 + 管道阶段）
├── formats.rs                  # 集成测试：各格式解析 + 输出格式
└── fixtures/
    ├── sample.ndjson           # 6 条 NDJSON 日志记录
    ├── sample.logfmt           # 5 条 logfmt 记录
    ├── sample.csv              # 5 条 CSV 记录
    ├── sample.yaml             # 5 条多文档 YAML 记录
    └── sample.toml             # 1 条扁平 TOML 配置记录
```

---

## 关键数据流

```
命令行参数
    │
    ├── cli.rs          解析 Cli 结构体（clap derive）
    │
    ├── main.rs         determine_mode() → Dsl | Keyword | Empty
    │
    │   ┌── [DSL 模式] ──────────────────────────────────┐
    │   │  query/dsl/parser.rs → DslQuery + 文件列表     │
    │   │  load_records() → detect + parse               │
    │   │  query/dsl/eval.rs → filter + transforms       │
    │   └──────────────────────────────────────────────── ┘
    │
    │   ┌── [关键字模式] ────────────────────────────────┐
    │   │  query/fast/parser.rs → FastQuery + 文件列表   │
    │   │  load_records() → detect + parse               │
    │   │  query/fast/eval.rs → filter + project + sort  │
    │   └──────────────────────────────────────────────── ┘
    │
    └── output/mod.rs   render(records, fmt, use_color)
        ├── ndjson.rs   → color.rs（如有颜色）
        ├── table.rs    → comfy-table
        ├── csv_out.rs
        └── [raw]       → rec.raw

文件读取流程:
    load_records() → rayon par_iter
        → read_one_file()
            → util/mmap.rs        （大文件 mmap）
            → util/decompress.rs  （gzip 透明解压）
            → detect.rs           （格式嗅探）
            → parser/*.rs         （格式解析）
```

---

## Crate 依赖说明

| Crate | 版本 | 使用位置 | 作用 |
|-------|------|---------|------|
| `clap` | 4 | `cli.rs` | CLI 参数解析（derive 宏） |
| `serde` + `serde_json` | 1 | 全局 | 序列化主干，Record 字段类型 |
| `indexmap` | 2 | `record.rs`、输出 | 有序 HashMap，保持字段插入顺序 |
| `csv` | 1 | `parser/csv.rs` | 健壮的 CSV/TSV 解析 |
| `memchr` | 2 | `query/dsl/eval.rs`、`detect.rs` | SIMD 字节搜索（`\n`、子串） |
| `thiserror` | 1 | `util/error.rs` | 错误类型 derive 宏 |
| `owo-colors` | 3 | `output/color.rs` | 终端 ANSI 颜色，尊重 `NO_COLOR` |
| `rayon` | 1 | `main.rs` | 文件级并行（`par_iter`） |
| `memmap2` | 0.9 | `util/mmap.rs` | 大文件近零拷贝读取 |
| `nom` | 7 | `query/dsl/parser.rs` | 解析组合子，DSL 表达式解析 |
| `regex` | 1 | `query/dsl/eval.rs` | 正则表达式匹配（`.matches`） |
| `serde_yaml` | 0.9 | `parser/yaml.rs` | YAML 解析，多文档支持 |
| `toml` | 0.8 | `parser/toml_fmt.rs` | TOML 解析 |
| `flate2` | 1 | `util/decompress.rs` | gzip 解压 |
| `comfy-table` | 7 | `output/table.rs` | 终端对齐表格，动态列宽 |

---

## Phase 完成清单

| Phase | 状态 | 新增的关键文件 |
|-------|------|--------------|
| 0 — 脚手架 | ✅ | 文档文件 |
| 1 — 格式检测 + 解析器 | ✅ | detect.rs, parser/ndjson+logfmt+csv+plaintext, record.rs |
| 2 — 快速查询层 | ✅ | query/fast/parser.rs, query/fast/eval.rs |
| 3 — 并行 + 性能 | ✅ | util/mmap.rs, rayon 集成，memchr 搜索 |
| 4 — 表达式 DSL | ✅ | query/dsl/ast.rs, query/dsl/parser.rs, query/dsl/eval.rs |
| 5 — 完整格式支持 | ✅ | parser/yaml.rs, parser/toml_fmt.rs, util/decompress.rs |
| 6 — 输出 + 颜色 | ✅ | output/color.rs, output/table.rs, output/csv_out.rs, --color/--no-color |
| 7 — 统计聚合 + pretty | ✅ | output/pretty.rs, sum/avg/min/max/skip/dedup/fields/head |

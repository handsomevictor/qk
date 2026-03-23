# RUST_GUIDE — Rust 入门，专为 qk 项目

这份指南只教你用 `qk` 需要的那部分 Rust。跳过理论，直接上命令和代码模式。

---

## 第一步：安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# 安装过程中按 1 选默认安装

# 安装完重启终端，或者：
source ~/.cargo/env

# 验证安装
rustc --version    # 应该输出 rustc 1.xx.x
cargo --version    # 应该输出 cargo 1.xx.x
```

---

## 每天用到的命令（最重要的 10 条）

```bash
# 检查代码能不能编译（不生成二进制，最快）
cargo check

# 编译 + 运行（开发模式，速度快，binary 大）
cargo run

# 带参数运行（-- 后面是传给程序的参数）
cargo run -- where level=error sample.log

# 读 stdin
echo '{"level":"error"}' | cargo run -- where level=error

# 生产编译（优化，速度快 3-10x，但编译慢）
cargo build --release
./target/release/qk where level=error sample.log

# 跑所有测试
cargo test

# 跑某个具体的测试
cargo test test_ndjson_parser

# 格式化代码（必须在提交前跑）
cargo fmt

# 检查代码风格问题
cargo clippy

# 添加依赖（不用手动改 Cargo.toml）
cargo add serde --features derive
cargo add clap --features derive
```

---

## 初始化项目

```bash
# 创建新项目
cargo new qk
cd qk

# 目录结构
# qk/
# ├── Cargo.toml    ← 配置文件（类似 package.json）
# ├── src/
# │   └── main.rs   ← 程序入口
# └── .git/
```

---

## Cargo.toml 怎么看

```toml
[package]
name = "qk"           # 二进制名字
version = "0.1.0"
edition = "2021"      # Rust 版本，用 2021

[[bin]]
name = "qk"
path = "src/main.rs"

[dependencies]
# 格式：crate名 = "版本"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
nom = "7"
rayon = "1"
memmap2 = "0.9"
memchr = "2"
csv = "1"
owo-colors = "4"
thiserror = "1"
indexmap = { version = "2", features = ["serde"] }

[dev-dependencies]   # 只在测试/bench 时用
criterion = "0.5"

[profile.release]    # 生产编译优化
opt-level = 3
lto = true
codegen-units = 1
strip = true         # 去掉调试符号，减小 binary 大小
```

---

## Rust 核心概念速览（只讲项目用到的）

### 变量和类型

```rust
let x = 5;              // 不可变（Rust 默认！）
let mut x = 5;          // 可变
let x: i32 = 5;         // 显式类型

// 常用类型
let s: String = String::from("hello");  // 拥有所有权的字符串
let s: &str = "hello";                  // 字符串引用（借用）
let n: i64 = 42;
let f: f64 = 3.14;
let b: bool = true;
let v: Vec<String> = vec!["a".to_string(), "b".to_string()];
```

### Result 和 ? 操作符（最常用的错误处理）

```rust
// Result<T, E> 表示可能失败的操作
// Ok(value) = 成功
// Err(e)    = 失败

fn read_file(path: &str) -> Result<String, std::io::Error> {
    let content = std::fs::read_to_string(path)?;  // ? = 失败时提前返回 Err
    Ok(content)
}

// 调用处
match read_file("app.log") {
    Ok(content) => println!("{}", content),
    Err(e) => eprintln!("Error: {}", e),
}

// 或者更简洁（在 main 里）
let content = read_file("app.log").unwrap();  // 失败直接 panic
let content = read_file("app.log").expect("failed to read log");  // panic + 自定义消息
```

### struct（数据结构）

```rust
// 定义
struct Record {
    fields: std::collections::HashMap<String, String>,
    raw: String,
    line_num: u32,
}

// 创建
let r = Record {
    fields: HashMap::new(),
    raw: String::from("{\"level\":\"error\"}"),
    line_num: 42,
};

// 访问字段
println!("{}", r.line_num);
```

### enum（特别重要，Rust 大量使用）

```rust
// 定义
enum Format {
    Ndjson,
    Json,
    Csv,
    Logfmt,
    PlainText,
}

// 使用 match（必须覆盖所有情况）
let fmt = Format::Ndjson;
match fmt {
    Format::Ndjson  => println!("parsing as ndjson"),
    Format::Json    => println!("parsing as json"),
    Format::Csv     => println!("parsing as csv"),
    Format::Logfmt  => println!("parsing as logfmt"),
    Format::PlainText => println!("plain text fallback"),
}
```

### impl（给 struct/enum 加方法）

```rust
struct Record { /* ... */ }

impl Record {
    // 构造函数（Rust 没有 new 关键字，但惯例用 new 方法名）
    fn new(raw: String, line_num: u32) -> Self {
        Record {
            fields: HashMap::new(),
            raw,
            line_num,
        }
    }

    // 方法（&self = 只读借用）
    fn get_field(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(|s| s.as_str())
    }

    // 可变方法（&mut self）
    fn set_field(&mut self, key: String, value: String) {
        self.fields.insert(key, value);
    }
}

// 调用
let r = Record::new("{...}".to_string(), 1);
let level = r.get_field("level");  // Option<&str>
```

### Option（可能不存在的值）

```rust
// Option<T> = Some(value) 或 None
let level: Option<&str> = record.get_field("level");

// 处理 Option
match level {
    Some(v) => println!("level = {}", v),
    None    => println!("no level field"),
}

// 更简洁的写法
if let Some(v) = level {
    println!("level = {}", v);
}

// unwrap_or 提供默认值
let v = level.unwrap_or("unknown");
```

### 迭代器（Rust 最强大的特性之一）

```rust
let records: Vec<Record> = parse_file("app.log");

// filter + map（类似 Python 的列表推导）
let errors: Vec<&Record> = records
    .iter()
    .filter(|r| r.get_field("level") == Some("error"))
    .collect();

// 计数
let error_count = records
    .iter()
    .filter(|r| r.get_field("level") == Some("error"))
    .count();

// 转换
let messages: Vec<String> = records
    .iter()
    .filter_map(|r| r.get_field("msg"))  // filter_map = filter + map，None 自动过滤
    .map(|s| s.to_uppercase())
    .collect();
```

### 并行迭代（rayon，几乎零改动）

```rust
use rayon::prelude::*;

// 把 .iter() 换成 .par_iter() 就自动并行了
let errors: Vec<&Record> = records
    .par_iter()                          // ← 只改这一行
    .filter(|r| r.get_field("level") == Some("error"))
    .collect();
```

---

## 写一个最小可用的 main.rs

```rust
use std::io::{self, BufRead};

fn main() {
    // 从 stdin 逐行读取
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.expect("failed to read line");
        // TODO: 解析 + 过滤 + 输出
        println!("{}", line);
    }
}
```

---

## 测试怎么写

```rust
// 在同一个文件末尾加 #[cfg(test)] 块
#[cfg(test)]
mod tests {
    use super::*;  // 导入当前模块的所有内容

    #[test]
    fn test_parse_logfmt_basic() {
        let input = r#"level=error msg="timeout" service=api latency=523"#;
        let records = parse_logfmt(input).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].get_field("level"), Some("error"));
        assert_eq!(records[0].get_field("service"), Some("api"));
    }

    #[test]
    fn test_empty_input() {
        let records = parse_logfmt("").unwrap();
        assert!(records.is_empty());
    }
}
```

运行：
```bash
cargo test                        # 跑所有测试
cargo test test_parse_logfmt      # 跑名字包含这个字符串的测试
cargo test -- --nocapture         # 测试失败时显示 println! 输出
```

---

## 常见编译错误和解决方法

### "cannot borrow as mutable"
```rust
// 错误
let v = vec![1, 2, 3];
v.push(4);  // ← 错误！v 是不可变的

// 修复
let mut v = vec![1, 2, 3];
v.push(4);  // ✓
```

### "value moved here"（所有权错误）
```rust
// 错误
let s = String::from("hello");
let s2 = s;         // s 的所有权转移给 s2
println!("{}", s);  // ← 错误！s 已经被 move 了

// 修复方案 1：clone（有复制开销）
let s2 = s.clone();
println!("{}", s);  // ✓

// 修复方案 2：借用（更常用）
let s2 = &s;        // 借用，不转移所有权
println!("{}", s);  // ✓
```

### "expected &str, found String"
```rust
fn greet(name: &str) { println!("Hello {}", name); }

let name = String::from("Victor");
greet(name);     // ← 错误：类型不匹配
greet(&name);    // ✓：自动解引用为 &str
```

### "trait bound not satisfied"
通常是少了 `use` 导入，或者没有实现某个 trait：
```rust
// 错误：不知道怎么序列化
serde_json::to_string(&my_struct);

// 修复：给 struct 加 derive
#[derive(serde::Serialize, serde::Deserialize)]
struct MyStruct { /* ... */ }
```

---

## 文件结构：多文件项目

```rust
// src/main.rs
mod detect;    // 对应 src/detect.rs 或 src/detect/mod.rs
mod record;
mod parser;
mod query;

use detect::Format;
use record::Record;

fn main() { /* ... */ }
```

```rust
// src/detect.rs
pub enum Format {   // pub = 公开，其他模块可以用
    Ndjson,
    Json,
}

pub fn sniff(bytes: &[u8]) -> Format {
    // ...
}
```

---

## 调试技巧

```bash
# 打印调试（最快）
println!("{:?}", my_value);   // 需要 #[derive(Debug)]
println!("{:#?}", my_value);  // 格式化打印，更易读

# 在代码里加断言
assert!(condition, "message if fails");
assert_eq!(a, b, "a and b should be equal");

# 运行时打印到 stderr（不影响 stdout 输出）
eprintln!("DEBUG: parsing line {}", line_num);

# 查看编译展开的宏
cargo expand  # 需要 cargo install cargo-expand
```

---

## 下一步

准备好了之后，在 Claude Code 里说：**"start phase 1"**，它会读取 `CLAUDE.md` 然后开始写代码。

每次开始新的 Claude Code 会话，它都会自动读取 `CLAUDE.md`，知道要更新哪些文档、遵守哪些约定。

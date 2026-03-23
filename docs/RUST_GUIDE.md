# RUST_GUIDE — Rust Primer for the qk Project

This guide teaches only the Rust you need to work on `qk`. Skip the theory — straight to commands and code patterns.

---

## Step 1: Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Press 1 for the default installation

# After installation, restart your terminal, or run:
source ~/.cargo/env

# Verify installation
rustc --version    # should print rustc 1.xx.x
cargo --version    # should print cargo 1.xx.x
```

---

## The 10 Commands You Use Every Day

```bash
# Check if the code compiles (no binary produced, fastest)
cargo check

# Compile and run (dev mode — fast compile, large binary)
cargo run

# Run with arguments (-- separates cargo args from program args)
cargo run -- where level=error sample.log

# Read from stdin
echo '{"level":"error"}' | cargo run -- where level=error

# Production build (optimized — 3-10x faster runtime, slow compile)
cargo build --release
./target/release/qk where level=error sample.log

# Run all tests
cargo test

# Run a specific test by name
cargo test test_ndjson_parser

# Format code (required before every commit)
cargo fmt

# Check code style
cargo clippy

# Add a dependency (no need to edit Cargo.toml manually)
cargo add serde --features derive
cargo add clap --features derive
```

---

## Initialize a Project

```bash
# Create a new project
cargo new qk
cd qk

# Directory structure:
# qk/
# ├── Cargo.toml    ← config file (like package.json)
# ├── src/
# │   └── main.rs   ← program entry point
# └── .git/
```

---

## Reading Cargo.toml

```toml
[package]
name = "qk"           # binary name
version = "0.1.0"
edition = "2021"      # Rust edition — use 2021

[[bin]]
name = "qk"
path = "src/main.rs"

[dependencies]
# format: crate_name = "version"
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

[dev-dependencies]   # only used in tests/benchmarks
criterion = "0.5"

[profile.release]    # production build optimizations
opt-level = 3
lto = true
codegen-units = 1
strip = true         # strip debug symbols to reduce binary size
```

---

## Core Rust Concepts (only what the project uses)

### Variables and Types

```rust
let x = 5;              // immutable (Rust default!)
let mut x = 5;          // mutable
let x: i32 = 5;         // explicit type

// Common types
let s: String = String::from("hello");  // owned string
let s: &str = "hello";                  // borrowed string slice
let n: i64 = 42;
let f: f64 = 3.14;
let b: bool = true;
let v: Vec<String> = vec!["a".to_string(), "b".to_string()];
```

### Result and the ? Operator (most common error handling)

```rust
// Result<T, E> represents a possibly-failing operation
// Ok(value) = success
// Err(e)    = failure

fn read_file(path: &str) -> Result<String, std::io::Error> {
    let content = std::fs::read_to_string(path)?;  // ? = return Err early on failure
    Ok(content)
}

// At the call site
match read_file("app.log") {
    Ok(content) => println!("{}", content),
    Err(e) => eprintln!("Error: {}", e),
}

// Or more concisely (in main)
let content = read_file("app.log").unwrap();  // panics on failure
let content = read_file("app.log").expect("failed to read log");  // panic + custom message
```

### struct (data structures)

```rust
// Define
struct Record {
    fields: std::collections::HashMap<String, String>,
    raw: String,
    line_num: u32,
}

// Create
let r = Record {
    fields: HashMap::new(),
    raw: String::from("{\"level\":\"error\"}"),
    line_num: 42,
};

// Access fields
println!("{}", r.line_num);
```

### enum (especially important — Rust uses them everywhere)

```rust
// Define
enum Format {
    Ndjson,
    Json,
    Csv,
    Logfmt,
    PlainText,
}

// Use with match (must cover all variants)
let fmt = Format::Ndjson;
match fmt {
    Format::Ndjson    => println!("parsing as ndjson"),
    Format::Json      => println!("parsing as json"),
    Format::Csv       => println!("parsing as csv"),
    Format::Logfmt    => println!("parsing as logfmt"),
    Format::PlainText => println!("plain text fallback"),
}
```

### impl (adding methods to struct/enum)

```rust
struct Record { /* ... */ }

impl Record {
    // Constructor (Rust has no `new` keyword, but using `new` as method name is idiomatic)
    fn new(raw: String, line_num: u32) -> Self {
        Record {
            fields: HashMap::new(),
            raw,
            line_num,
        }
    }

    // Method (&self = read-only borrow)
    fn get_field(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(|s| s.as_str())
    }

    // Mutable method (&mut self)
    fn set_field(&mut self, key: String, value: String) {
        self.fields.insert(key, value);
    }
}

// Usage
let r = Record::new("{...}".to_string(), 1);
let level = r.get_field("level");  // Option<&str>
```

### Option (a value that might not exist)

```rust
// Option<T> = Some(value) or None
let level: Option<&str> = record.get_field("level");

// Handle Option
match level {
    Some(v) => println!("level = {}", v),
    None    => println!("no level field"),
}

// More concise
if let Some(v) = level {
    println!("level = {}", v);
}

// Provide a default with unwrap_or
let v = level.unwrap_or("unknown");
```

### Iterators (one of Rust's most powerful features)

```rust
let records: Vec<Record> = parse_file("app.log");

// filter + map (like Python list comprehensions)
let errors: Vec<&Record> = records
    .iter()
    .filter(|r| r.get_field("level") == Some("error"))
    .collect();

// count
let error_count = records
    .iter()
    .filter(|r| r.get_field("level") == Some("error"))
    .count();

// transform
let messages: Vec<String> = records
    .iter()
    .filter_map(|r| r.get_field("msg"))  // filter_map = filter + map, None is automatically dropped
    .map(|s| s.to_uppercase())
    .collect();
```

### Parallel Iteration (rayon — almost zero code change)

```rust
use rayon::prelude::*;

// Just replace .iter() with .par_iter() for automatic parallelism
let errors: Vec<&Record> = records
    .par_iter()                          // ← only this line changes
    .filter(|r| r.get_field("level") == Some("error"))
    .collect();
```

---

## A Minimal Working main.rs

```rust
use std::io::{self, BufRead};

fn main() {
    // Read from stdin line by line
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.expect("failed to read line");
        // TODO: parse + filter + output
        println!("{}", line);
    }
}
```

---

## How to Write Tests

```rust
// Add a #[cfg(test)] block at the bottom of the same file
#[cfg(test)]
mod tests {
    use super::*;  // import everything from the current module

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

Run:
```bash
cargo test                        # run all tests
cargo test test_parse_logfmt      # run tests whose name contains this string
cargo test -- --nocapture         # show println! output when tests fail
```

---

## Common Compile Errors and Fixes

### "cannot borrow as mutable"
```rust
// Error
let v = vec![1, 2, 3];
v.push(4);  // ← error! v is immutable

// Fix
let mut v = vec![1, 2, 3];
v.push(4);  // ✓
```

### "value moved here" (ownership error)
```rust
// Error
let s = String::from("hello");
let s2 = s;         // ownership of s moves to s2
println!("{}", s);  // ← error! s has been moved

// Fix option 1: clone (has copy cost)
let s2 = s.clone();
println!("{}", s);  // ✓

// Fix option 2: borrow (more common)
let s2 = &s;        // borrow, no ownership transfer
println!("{}", s);  // ✓
```

### "expected &str, found String"
```rust
fn greet(name: &str) { println!("Hello {}", name); }

let name = String::from("Victor");
greet(name);     // ← error: type mismatch
greet(&name);    // ✓: auto-derefs to &str
```

### "trait bound not satisfied"
Usually a missing `use` import, or a trait not implemented:
```rust
// Error: doesn't know how to serialize
serde_json::to_string(&my_struct);

// Fix: add derive to the struct
#[derive(serde::Serialize, serde::Deserialize)]
struct MyStruct { /* ... */ }
```

---

## File Structure: Multi-file Projects

```rust
// src/main.rs
mod detect;    // corresponds to src/detect.rs or src/detect/mod.rs
mod record;
mod parser;
mod query;

use detect::Format;
use record::Record;

fn main() { /* ... */ }
```

```rust
// src/detect.rs
pub enum Format {   // pub = public, accessible from other modules
    Ndjson,
    Json,
}

pub fn sniff(bytes: &[u8]) -> Format {
    // ...
}
```

---

## Debugging Tips

```bash
# Print debug (fastest)
println!("{:?}", my_value);   # requires #[derive(Debug)]
println!("{:#?}", my_value);  # pretty-print, more readable

# Add assertions in code
assert!(condition, "message if fails");
assert_eq!(a, b, "a and b should be equal");

# Print to stderr at runtime (does not interfere with stdout output)
eprintln!("DEBUG: parsing line {}", line_num);

# View expanded macros
cargo expand  # requires: cargo install cargo-expand
```

---

## Next Steps

When ready, tell Claude Code: **"start phase 1"** — it will read `CLAUDE.md` and start writing code.

Every time you start a new Claude Code session, it automatically reads `CLAUDE.md` and knows which documents to update and which conventions to follow.

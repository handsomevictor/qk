mod cli;
mod detect;
mod output;
mod parser;
mod query;
mod record;
mod util;

use std::io::{self, Read};

use clap::Parser;
use rayon::prelude::*;

use cli::Cli;
use util::error::{QkError, Result};

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("qk: {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let color = cli.use_color();
    let mode = determine_mode(&cli.args);

    if cli.explain {
        return print_explain(&cli.args, mode);
    }

    match mode {
        Mode::Empty => Ok(()),
        Mode::Dsl => run_dsl(&cli.args, &cli.fmt, color),
        Mode::Keyword => run_keyword(&cli.args, &cli.fmt, color),
    }
}

fn run_dsl(args: &[String], fmt: &cli::OutputFormat, color: bool) -> Result<()> {
    let expr = args.first().map(String::as_str).unwrap_or("");
    let (dsl_query, extra_files) = query::dsl::parser::parse(expr)?;
    let file_paths = if extra_files.is_empty() { args[1..].to_vec() } else { extra_files };
    let recs = load_records(&file_paths)?;
    let result = query::dsl::eval::eval(&dsl_query, recs)?;
    output::render(&result, fmt, color)
}

fn run_keyword(args: &[String], fmt: &cli::OutputFormat, color: bool) -> Result<()> {
    let (fast_query, files) = query::fast::parser::parse(args)?;
    let recs = load_records(&files)?;
    let result = query::fast::eval::eval(&fast_query, recs)?;
    output::render(&result, fmt, color)
}

// ── Mode detection ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Dsl,
    Keyword,
    Empty,
}

fn determine_mode(args: &[String]) -> Mode {
    match args.first() {
        None => Mode::Empty,
        Some(first) if first.starts_with('.') => Mode::Dsl,
        // Expressions like "not .field == val" or "| count()" are also DSL
        Some(first) if first.starts_with("not ") || first.starts_with('|') => Mode::Dsl,
        _ => Mode::Keyword,
    }
}

// ── --explain ─────────────────────────────────────────────────────────────────

fn print_explain(args: &[String], mode: Mode) -> Result<()> {
    println!("=== 查询解析 ===");
    match mode {
        Mode::Dsl => {
            let expr = args.first().map(String::as_str).unwrap_or("");
            let (q, files) = query::dsl::parser::parse(expr)?;
            println!("模式: DSL 表达式层");
            println!("过滤: {:#?}", q.filter);
            if !q.transforms.is_empty() {
                println!("变换: {:#?}", q.transforms);
            }
            let file_paths: Vec<_> = if files.is_empty() { args[1..].to_vec() } else { files };
            println!("文件: {file_paths:?}");
        }
        Mode::Keyword => {
            let (q, files) = query::fast::parser::parse(args)?;
            println!("模式: 快速关键字层");
            println!("{q:#?}");
            println!("文件: {files:?}");
        }
        Mode::Empty => println!("（无查询）"),
    }
    Ok(())
}

// ── Record loading ────────────────────────────────────────────────────────────

/// Load records from file paths in parallel (rayon). Falls back to stdin if empty.
fn load_records(paths: &[String]) -> Result<Vec<record::Record>> {
    if paths.is_empty() {
        return read_stdin();
    }

    let results: Vec<Result<Vec<record::Record>>> =
        paths.par_iter().map(|p| read_one_file(p)).collect();

    let mut all = Vec::new();
    for result in results {
        all.extend(result?);
    }
    Ok(all)
}

/// Read and parse one file with transparent gzip decompression.
fn read_one_file(path: &str) -> Result<Vec<record::Record>> {
    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path);

    let raw = read_file_bytes(path)?;

    let (content, effective_name) = if util::decompress::is_gzip(&raw) {
        let text = util::decompress::decompress_gz(&raw, path)?;
        let inner = util::decompress::inner_filename(filename).to_string();
        (text, inner)
    } else {
        let text = String::from_utf8(raw).map_err(|_| QkError::Parse {
            file: path.to_string(),
            line: 0,
            msg: "file contains invalid UTF-8".to_string(),
        })?;
        (text, filename.to_string())
    };

    let format = detect::sniff(content.as_bytes(), Some(&effective_name));
    parser::parse(&content, &format, path)
}

/// Read raw file bytes; uses mmap for files ≥ 64 KiB (via `util::mmap`).
fn read_file_bytes(path: &str) -> Result<Vec<u8>> {
    util::mmap::read_bytes(path)
}

/// Read all records from stdin, auto-detecting format.
fn read_stdin() -> Result<Vec<record::Record>> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).map_err(|e| QkError::Io {
        path: "<stdin>".to_string(),
        source: e,
    })?;
    if buf.trim().is_empty() {
        return Ok(vec![]);
    }
    let format = detect::sniff(buf.as_bytes(), None);
    parser::parse(&buf, &format, "<stdin>")
}

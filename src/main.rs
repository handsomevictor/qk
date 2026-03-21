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

    let no_header = cli.no_header;
    let cast_map = util::cast::parse_cast_map(&cli.cast)?;

    match mode {
        // No query args — pass stdin through unchanged (allows: echo '...' | qk)
        Mode::Empty => run_keyword(&[], &cli.fmt, color, no_header, &cast_map),
        Mode::Dsl => run_dsl(&cli.args, &cli.fmt, color, no_header, &cast_map),
        Mode::Keyword => run_keyword(&cli.args, &cli.fmt, color, no_header, &cast_map),
    }
}

fn run_dsl(
    args: &[String],
    fmt: &cli::OutputFormat,
    color: bool,
    no_header: bool,
    cast_map: &std::collections::HashMap<String, util::cast::CastType>,
) -> Result<()> {
    let expr = args.first().map(String::as_str).unwrap_or("");
    let (dsl_query, extra_files) = query::dsl::parser::parse(expr)?;
    let file_paths = if extra_files.is_empty() { args[1..].to_vec() } else { extra_files };
    let recs = load_records(&file_paths, no_header)?;
    let (recs, cast_warnings) = util::cast::apply_casts(recs, cast_map);
    let (result, eval_warnings) = query::dsl::eval::eval(&dsl_query, recs)?;
    output::render(&result, fmt, color)?;
    print_warnings(&cast_warnings);
    print_warnings(&eval_warnings);
    Ok(())
}

fn run_keyword(
    args: &[String],
    fmt: &cli::OutputFormat,
    color: bool,
    no_header: bool,
    cast_map: &std::collections::HashMap<String, util::cast::CastType>,
) -> Result<()> {
    let (fast_query, files) = query::fast::parser::parse(args)?;
    let recs = load_records(&files, no_header)?;
    let (recs, cast_warnings) = util::cast::apply_casts(recs, cast_map);
    let (result, eval_warnings) = query::fast::eval::eval(&fast_query, recs)?;
    output::render(&result, fmt, color)?;
    print_warnings(&cast_warnings);
    print_warnings(&eval_warnings);
    Ok(())
}

/// Print collected warnings to stderr. Warnings never appear on stdout so they
/// don't interfere with piped output.
fn print_warnings(warnings: &[String]) {
    for w in warnings {
        eprintln!("{w}");
    }
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
    println!("=== Query Parse ===");
    match mode {
        Mode::Dsl => {
            let expr = args.first().map(String::as_str).unwrap_or("");
            let (q, files) = query::dsl::parser::parse(expr)?;
            println!("mode:    DSL expression layer");
            println!("filter:  {:#?}", q.filter);
            if !q.transforms.is_empty() {
                println!("stages:  {:#?}", q.transforms);
            }
            let file_paths: Vec<_> = if files.is_empty() { args[1..].to_vec() } else { files };
            println!("files:   {file_paths:?}");
        }
        Mode::Keyword => {
            let (q, files) = query::fast::parser::parse(args)?;
            println!("mode:    keyword layer");
            println!("{q:#?}");
            println!("files:   {files:?}");
        }
        Mode::Empty => println!("(no query)"),
    }
    Ok(())
}

// ── Record loading ────────────────────────────────────────────────────────────

/// Load records from file paths in parallel (rayon). Falls back to stdin if empty.
fn load_records(paths: &[String], no_header: bool) -> Result<Vec<record::Record>> {
    if paths.is_empty() {
        return read_stdin();
    }

    let results: Vec<Result<Vec<record::Record>>> =
        paths.par_iter().map(|p| read_one_file(p, no_header)).collect();

    let mut all = Vec::new();
    for result in results {
        all.extend(result?);
    }
    Ok(all)
}

/// Read and parse one file with transparent gzip decompression.
fn read_one_file(path: &str, no_header: bool) -> Result<Vec<record::Record>> {
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
    parser::parse(&content, &format, path, no_header)
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
    parser::parse(&buf, &format, "<stdin>", false)
}

mod cli;
mod detect;
mod output;
mod parser;
mod query;
mod record;
mod tui;
mod util;

use std::io::{self, BufRead, Read, Write};

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
    if cli.ui {
        return run_tui(cli);
    }

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

/// Launch the interactive TUI browser.
///
/// All `cli.args` are treated as file paths.  Records are loaded and
/// cast up-front; the query is typed live inside the TUI.
fn run_tui(cli: Cli) -> Result<()> {
    let no_header = cli.no_header;
    let cast_map = util::cast::parse_cast_map(&cli.cast)?;
    let file_paths = cli.args;
    let records = load_records(&file_paths, no_header)?;
    let (records, cast_warnings) = util::cast::apply_casts(records, &cast_map);
    print_warnings(&cast_warnings);
    tui::run(records, &file_paths)
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
    let file_paths = if extra_files.is_empty() {
        args[1..].to_vec()
    } else {
        extra_files
    };
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

    // Streaming stdin path: no file args, non-buffering query, streaming-compatible output format.
    // This enables `tail -f file | qk where level=error` to work without blocking until EOF.
    if files.is_empty()
        && !query::fast::eval::requires_buffering(&fast_query)
        && output::is_streaming_compatible(fmt)
    {
        return run_stdin_streaming_keyword(&fast_query, fmt, color, cast_map);
    }

    // Batch mode: collect all records first, then eval.
    let recs = load_records(&files, no_header)?;
    let (recs, cast_warnings) = util::cast::apply_casts(recs, cast_map);
    let (result, eval_warnings) = query::fast::eval::eval(&fast_query, recs)?;
    output::render(&result, fmt, color)?;
    print_warnings(&cast_warnings);
    print_warnings(&eval_warnings);
    Ok(())
}

/// Stream-eval keyword queries from stdin line by line (NDJSON only).
///
/// Each line is parsed and evaluated immediately — no buffering until EOF.
/// This enables `tail -f file | qk where level=error` to produce real-time output.
/// Only called when `!requires_buffering(query)` and reading from stdin.
fn run_stdin_streaming_keyword(
    query: &query::fast::parser::FastQuery,
    fmt: &cli::OutputFormat,
    color: bool,
    cast_map: &std::collections::HashMap<String, util::cast::CastType>,
) -> Result<()> {
    let stdin = io::stdin();
    let reader = io::BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    let mut line_num: usize = 0;
    let mut matched: usize = 0;
    let limit = query.limit.unwrap_or(usize::MAX);

    for line_result in reader.lines() {
        let line = line_result.map_err(|e| QkError::Io {
            path: "<stdin>".to_string(),
            source: e,
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        line_num += 1;

        let rec = match parser::ndjson::parse_line(trimmed, "<stdin>", line_num) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[qk warning] {e}");
                continue;
            }
        };

        // Apply per-record casts (cast_map is usually empty — fast path)
        let rec = if cast_map.is_empty() {
            rec
        } else {
            let (mut recs, cast_warns) = util::cast::apply_casts(vec![rec], cast_map);
            for w in cast_warns {
                eprintln!("{w}");
            }
            recs.pop().expect("apply_casts always returns same count")
        };

        if let Some(matched_rec) = query::fast::eval::eval_one(query, rec)? {
            output::render_one(&matched_rec, fmt, color, &mut out)?;
            // Flush after each record so piped consumers see output immediately
            out.flush().map_err(|e| QkError::Io {
                path: "<stdout>".to_string(),
                source: e,
            })?;
            matched += 1;
            if matched >= limit {
                break;
            }
        }
    }
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
            let file_paths: Vec<_> = if files.is_empty() {
                args[1..].to_vec()
            } else {
                files
            };
            println!("files:   {file_paths:?}");
        }
        Mode::Keyword => {
            let (q, files) = query::fast::parser::parse(args)?;
            println!("mode:    keyword layer");
            println!("{q:#?}");
            println!("files:   {files:?}");
            if query::fast::eval::requires_buffering(&q) {
                println!();
                println!("note:    batch mode forced (aggregation or sort requires all records)");
                println!("         stdin streaming is disabled for this query");
            }
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

    let results: Vec<Result<Vec<record::Record>>> = paths
        .par_iter()
        .map(|p| read_one_file(p, no_header))
        .collect();

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

/// Read all records from stdin to EOF, auto-detecting format.
///
/// This is the batch (non-streaming) path. For streaming NDJSON queries,
/// `run_stdin_streaming_keyword` is used instead.
fn read_stdin() -> Result<Vec<record::Record>> {
    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| QkError::Io {
            path: "<stdin>".to_string(),
            source: e,
        })?;
    if buf.trim().is_empty() {
        return Ok(vec![]);
    }
    let format = detect::sniff(buf.as_bytes(), None);
    parser::parse(&buf, &format, "<stdin>", false)
}

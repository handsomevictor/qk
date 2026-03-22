mod cli;
mod config;
mod detect;
mod output;
mod parser;
mod query;
mod record;
mod tui;
mod util;

use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::time::Instant;

use clap::Parser;
use rayon::prelude::*;

use cli::{Cli, OutputFormat};
use util::error::{QkError, Result};

// ── Processing statistics ─────────────────────────────────────────────────────

/// Collects per-run statistics, printed to stderr when `--stats` is set.
#[derive(Default)]
struct RunStats {
    start: Option<Instant>,
    records_in: usize,
    records_out: usize,
}

impl RunStats {
    fn start() -> Self {
        Self {
            start: Some(Instant::now()),
            records_in: 0,
            records_out: 0,
        }
    }

    fn print(&self, fmt: &OutputFormat) {
        let elapsed = self.start.map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);
        eprintln!("---");
        eprintln!("Stats:");
        eprintln!("  Records in:  {}", self.records_in);
        eprintln!("  Records out: {}", self.records_out);
        eprintln!("  Time:        {elapsed:.3}s");
        eprintln!("  Output fmt:  {}", fmt.as_str());
    }
}

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

    // Handle `qk config show` / `qk config reset` before any other dispatch.
    if let ["config", sub] = cli
        .args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        match *sub {
            "show" => {
                config::show();
                return Ok(());
            }
            "reset" => {
                return config::reset();
            }
            other => {
                return Err(QkError::Parse {
                    file: String::new(),
                    line: 0,
                    msg: format!(
                        "unknown config subcommand '{other}'. Valid subcommands: show, reset"
                    ),
                });
            }
        }
    }

    let mode = determine_mode(&cli.args);

    if cli.explain {
        return print_explain(&cli.args, mode);
    }

    let cfg = config::load();

    // Color: --no-color > --color > config no_color > NO_COLOR env > TTY
    let color = {
        let base = cli.use_color();
        if cfg.no_color.unwrap_or(false) && !cli.color {
            false
        } else {
            base
        }
    };

    // Auto-limit: only when stdout is a TTY, no --all, and default_limit != 0
    let auto_limit: Option<usize> = if std::io::stdout().is_terminal() && !cli.all {
        let n = cfg.default_limit.unwrap_or(20);
        if n == 0 {
            None
        } else {
            Some(n)
        }
    } else {
        None
    };

    let fmt = cli.fmt.unwrap_or_else(|| {
        cfg.default_fmt
            .as_deref()
            .and_then(OutputFormat::from_config_str)
            .unwrap_or_default()
    });

    let no_header = cli.no_header;
    let quiet = cli.quiet;
    let cast_map = util::cast::parse_cast_map(&cli.cast)?;
    let mut stats = if cli.stats {
        Some(RunStats::start())
    } else {
        None
    };

    match mode {
        Mode::Empty => run_keyword(
            &[],
            &fmt,
            color,
            no_header,
            &cast_map,
            &mut stats,
            quiet,
            auto_limit,
        ),
        Mode::Dsl => run_dsl(
            &cli.args, &fmt, color, no_header, &cast_map, &mut stats, quiet, auto_limit,
        ),
        Mode::Keyword => run_keyword(
            &cli.args, &fmt, color, no_header, &cast_map, &mut stats, quiet, auto_limit,
        ),
    }?;

    if let Some(s) = &stats {
        s.print(&fmt);
    }
    Ok(())
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
    print_warnings(&cast_warnings, false);
    tui::run(records, &file_paths)
}

#[allow(clippy::too_many_arguments)]
fn run_dsl(
    args: &[String],
    fmt: &cli::OutputFormat,
    color: bool,
    no_header: bool,
    cast_map: &std::collections::HashMap<String, util::cast::CastType>,
    stats: &mut Option<RunStats>,
    quiet: bool,
    auto_limit: Option<usize>,
) -> Result<()> {
    let expr = args.first().map(String::as_str).unwrap_or("");
    let (dsl_query, extra_files) = query::dsl::parser::parse(expr)?;
    let file_paths = if extra_files.is_empty() {
        args[1..].to_vec()
    } else {
        extra_files
    };
    let recs = load_records(&file_paths, no_header)?;
    if let Some(s) = stats.as_mut() {
        s.records_in = recs.len();
    }
    let (recs, cast_warnings) = util::cast::apply_casts(recs, cast_map);
    let (result, eval_warnings) = query::dsl::eval::eval(&dsl_query, recs)?;

    // Apply auto-limit when there is no explicit DSL limit stage.
    let has_dsl_limit = dsl_query
        .transforms
        .iter()
        .any(|s| matches!(s, query::dsl::ast::Stage::Limit(_)));
    let result = apply_auto_limit(result, auto_limit.filter(|_| !has_dsl_limit));

    if let Some(s) = stats.as_mut() {
        s.records_out = result.len();
    }
    output::render(&result, fmt, color)?;
    print_warnings(&cast_warnings, quiet);
    print_warnings(&eval_warnings, quiet);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_keyword(
    args: &[String],
    fmt: &cli::OutputFormat,
    color: bool,
    no_header: bool,
    cast_map: &std::collections::HashMap<String, util::cast::CastType>,
    stats: &mut Option<RunStats>,
    quiet: bool,
    auto_limit: Option<usize>,
) -> Result<()> {
    let (fast_query, files) = query::fast::parser::parse(args)?;

    if files.is_empty()
        && !query::fast::eval::requires_buffering(&fast_query)
        && output::is_streaming_compatible(fmt)
    {
        return run_stdin_streaming_keyword(
            &fast_query,
            fmt,
            color,
            cast_map,
            stats,
            quiet,
            auto_limit,
        );
    }

    let recs = load_records(&files, no_header)?;
    if let Some(s) = stats.as_mut() {
        s.records_in = recs.len();
    }
    let (recs, cast_warnings) = util::cast::apply_casts(recs, cast_map);
    let (result, eval_warnings) = query::fast::eval::eval(&fast_query, recs)?;

    // Apply auto-limit only when no explicit limit is set in the query.
    let result = apply_auto_limit(result, auto_limit.filter(|_| fast_query.limit.is_none()));

    if let Some(s) = stats.as_mut() {
        s.records_out = result.len();
    }
    output::render(&result, fmt, color)?;
    print_warnings(&cast_warnings, quiet);
    print_warnings(&eval_warnings, quiet);
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
    stats: &mut Option<RunStats>,
    quiet: bool,
    auto_limit: Option<usize>,
) -> Result<()> {
    let stdin = io::stdin();
    let reader = io::BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    let mut line_num: usize = 0;
    let mut matched: usize = 0;

    // When there is no explicit query limit, the auto-limit applies.
    let effective_limit = match (query.limit, auto_limit.filter(|_| query.limit.is_none())) {
        (Some(q), Some(a)) => q.min(a),
        (Some(q), None) => q,
        (None, Some(a)) => a,
        (None, None) => usize::MAX,
    };
    let show_auto_hint = auto_limit.is_some() && query.limit.is_none();

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
                if !quiet {
                    eprintln!("[qk warning] {e}");
                }
                continue;
            }
        };

        let rec = if cast_map.is_empty() {
            rec
        } else {
            let (mut recs, cast_warns) = util::cast::apply_casts(vec![rec], cast_map);
            if !quiet {
                for w in cast_warns {
                    eprintln!("{w}");
                }
            }
            recs.pop().expect("apply_casts always returns same count")
        };

        if let Some(matched_rec) = query::fast::eval::eval_one(query, rec)? {
            output::render_one(&matched_rec, fmt, color, &mut out)?;
            out.flush().map_err(|e| QkError::Io {
                path: "<stdout>".to_string(),
                source: e,
            })?;
            matched += 1;
            if matched >= effective_limit {
                if show_auto_hint && !quiet {
                    eprintln!(
                        "[qk] Auto-limit reached ({effective_limit} records). Use `--all` to show all, or pipe output to disable limit."
                    );
                }
                break;
            }
        }
    }
    if let Some(s) = stats.as_mut() {
        s.records_in = line_num;
        s.records_out = matched;
    }
    Ok(())
}

/// Print collected warnings to stderr. Warnings never appear on stdout so they
/// don't interfere with piped output.
fn print_warnings(warnings: &[String], quiet: bool) {
    if quiet {
        return;
    }
    for w in warnings {
        eprintln!("{w}");
    }
}

/// Truncate `records` to `limit` if set, printing a hint to stderr.
///
/// Returns the (possibly truncated) records. No-op when `limit` is `None`
/// or the record count is already within the limit.
fn apply_auto_limit(records: Vec<record::Record>, limit: Option<usize>) -> Vec<record::Record> {
    let Some(n) = limit else {
        return records;
    };
    let total = records.len();
    if total <= n {
        return records;
    }
    eprintln!(
        "[qk] {total} records matched. Showing first {n} (stdout is a terminal). \
         Use `--all` to show all, or pipe output to disable this limit."
    );
    records.into_iter().take(n).collect()
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
///
/// Shows a spinner on stderr while loading when stderr is a terminal and
/// the paths list is non-empty (no spinner for stdin — it may block forever).
fn load_records(paths: &[String], no_header: bool) -> Result<Vec<record::Record>> {
    if paths.is_empty() {
        return read_stdin();
    }

    // Spinner: only shown when stderr is connected to a terminal.
    let spinner = if io::stderr().is_terminal() {
        let pb = indicatif::ProgressBar::new_spinner();
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        let msg = if paths.len() == 1 {
            format!("Reading {}…", paths[0])
        } else {
            format!("Reading {} files…", paths.len())
        };
        pb.set_style(
            indicatif::ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap_or_else(|_| indicatif::ProgressStyle::default_spinner()),
        );
        pb.set_message(msg);
        Some(pb)
    } else {
        None
    };

    let results: Vec<Result<Vec<record::Record>>> = paths
        .par_iter()
        .map(|p| read_one_file(p, no_header))
        .collect();

    // Clear the spinner before any output reaches stdout.
    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

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

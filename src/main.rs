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

// ── Flag reordering (position-independent flags) ──────────────────────────────

/// Bool flags: consume no additional value.
const BOOL_FLAGS: &[&str] = &[
    "--quiet",
    "-q",
    "--all",
    "-A",
    "--color",
    "--no-color",
    "--stats",
    "--explain",
    "--ui",
    "--no-header",
    "--case-sensitive",
    "-S",
    // clap built-ins — pass through as-is so clap handles them
    "--help",
    "-h",
    "--version",
    "-V",
];

/// Value flags: consume the next token as their argument.
const VALUE_FLAGS: &[&str] = &["--fmt", "-f", "--cast"];

/// All recognised flag names (for error suggestions).
const ALL_KNOWN_FLAGS: &[&str] = &[
    "--quiet",
    "-q",
    "--all",
    "-A",
    "--color",
    "--no-color",
    "--stats",
    "--explain",
    "--ui",
    "--no-header",
    "--case-sensitive",
    "-S",
    "--fmt",
    "-f",
    "--cast",
    "--help",
    "-h",
    "--version",
    "-V",
];

/// Compute the Levenshtein edit distance between two strings.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, val) in dp[0].iter_mut().enumerate() {
        *val = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1]
            } else {
                1 + dp[i - 1][j].min(dp[i][j - 1]).min(dp[i - 1][j - 1])
            };
        }
    }
    dp[m][n]
}

/// Return the closest known flag to `unknown`, or `None` if no flag is close.
fn suggest_flag(unknown: &str) -> Option<&'static str> {
    ALL_KNOWN_FLAGS
        .iter()
        .filter(|&&f| levenshtein(unknown, f) <= 2)
        .min_by_key(|&&f| levenshtein(unknown, f))
        .copied()
}

/// Build a human-readable "Unknown flag" error message.
fn unknown_flag_error(flag: &str) -> QkError {
    let mut msg = format!("unknown flag '{flag}'");
    if let Some(suggestion) = suggest_flag(flag) {
        msg.push_str(&format!("\n  Did you mean: {suggestion}?"));
    }
    msg.push_str(
        "\n  Valid flags: --quiet (-q), --all (-A), --color, --no-color, \
         --stats, --explain, --ui, --no-header, --case-sensitive (-S), --fmt (-f), --cast",
    );
    msg.push_str("\n  Run 'qk --help' for full usage.");
    QkError::UnknownFlag { msg }
}

/// Re-order `args` so that all recognised flags (and their values) come first,
/// followed by all positional tokens (query keywords + file paths).
///
/// This makes every flag position-independent:
/// `qk avg latency --quiet app.log`  →  same as  `qk --quiet avg latency app.log`
///
/// Any argument that starts with `-` but is not a recognised flag returns an error
/// with a helpful "did you mean?" suggestion.
fn reorder_args(args: &[String]) -> Result<Vec<String>> {
    let mut flags: Vec<String> = Vec::new();
    let mut positional: Vec<String> = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        // Detect --flag=value embedded form: split at first '='
        let flag_part = if let Some(eq) = arg.find('=') {
            &arg[..eq]
        } else {
            arg.as_str()
        };
        let has_embedded_value = arg.contains('=') && flag_part != arg.as_str();

        if BOOL_FLAGS.contains(&flag_part) {
            flags.push(arg.clone());
            i += 1;
        } else if VALUE_FLAGS.contains(&flag_part) {
            if has_embedded_value {
                // --fmt=pretty — push whole token, no next arg consumed
                flags.push(arg.clone());
                i += 1;
            } else {
                // --fmt pretty — push flag, then consume next token as value
                flags.push(arg.clone());
                i += 1;
                if i < args.len() {
                    // The value might start with '-' (e.g. --fmt -bad) — let
                    // clap report the missing-value error; just push it.
                    flags.push(args[i].clone());
                    i += 1;
                }
                // If no next arg, clap will report "requires a value".
            }
        } else if arg.starts_with('-') {
            // Starts with '-' but not a recognised flag → helpful error
            return Err(unknown_flag_error(arg));
        } else {
            positional.push(arg.clone());
            i += 1;
        }
    }

    Ok(flags.into_iter().chain(positional).collect())
}

fn main() {
    let raw: Vec<String> = std::env::args().collect();
    let program = raw[0].clone();

    // Reorder: extract all recognised flags from any position, put them first.
    // Unknown flags (typos like --quite) are caught here with helpful messages.
    let reordered_rest = match reorder_args(&raw[1..]) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("qk: {e}");
            std::process::exit(1);
        }
    };

    let full_args: Vec<String> = std::iter::once(program).chain(reordered_rest).collect();

    let cli = match <Cli as clap::Parser>::try_parse_from(full_args) {
        Ok(c) => c,
        Err(e) => {
            e.print().unwrap_or(());
            std::process::exit(e.exit_code());
        }
    };

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

    let default_time_field = cfg
        .default_time_field
        .as_deref()
        .unwrap_or("ts")
        .to_string();

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
    let case_sensitive = cli.case_sensitive;
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
            &default_time_field,
            case_sensitive,
        ),
        Mode::Dsl => run_dsl(
            &cli.args,
            &fmt,
            color,
            no_header,
            &cast_map,
            &mut stats,
            quiet,
            auto_limit,
            case_sensitive,
        ),
        Mode::Keyword => run_keyword(
            &cli.args,
            &fmt,
            color,
            no_header,
            &cast_map,
            &mut stats,
            quiet,
            auto_limit,
            &default_time_field,
            case_sensitive,
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
    case_sensitive: bool,
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
    let (result, eval_warnings) = query::dsl::eval::eval(&dsl_query, recs, case_sensitive)?;

    // Apply auto-limit when there is no explicit DSL limit stage.
    let has_dsl_limit = dsl_query
        .transforms
        .iter()
        .any(|s| matches!(s, query::dsl::ast::Stage::Limit(_)));
    let (result, limit_info) = apply_auto_limit(result, auto_limit.filter(|_| !has_dsl_limit));

    if let Some(s) = stats.as_mut() {
        s.records_out = result.len();
    }
    output::render(&result, fmt, color)?;
    if let Some((shown, total)) = limit_info {
        print_auto_limit_notice(shown, total, quiet);
    }
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
    default_time_field: &str,
    case_sensitive: bool,
) -> Result<()> {
    let (mut fast_query, files) =
        query::fast::parser::parse_with_defaults(args, default_time_field)?;
    fast_query.case_sensitive = case_sensitive;

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
    let (result, limit_info) =
        apply_auto_limit(result, auto_limit.filter(|_| fast_query.limit.is_none()));

    if let Some(s) = stats.as_mut() {
        s.records_out = result.len();
    }
    output::render(&result, fmt, color)?;
    if let Some((shown, total)) = limit_info {
        print_auto_limit_notice(shown, total, quiet);
    }
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
    let mut reader = io::BufReader::new(stdin.lock());

    // Peek at the first buffer-full to detect the actual format of stdin.
    // fill_buf() does NOT consume bytes — the subsequent read will re-read them.
    let peeked = reader.fill_buf().map_err(|e| QkError::Io {
        path: "<stdin>".to_string(),
        source: e,
    })?;
    let stdin_format = detect::sniff(peeked, None);

    // If stdin is not NDJSON (e.g. pretty-printed JSON from `jq .data[]`),
    // fall back to the batch path: read everything, parse, eval, render.
    if stdin_format != detect::Format::Ndjson {
        let mut buf = String::new();
        reader.read_to_string(&mut buf).map_err(|e| QkError::Io {
            path: "<stdin>".to_string(),
            source: e,
        })?;
        let records = parser::parse(&buf, &stdin_format, "<stdin>", false)?;
        if let Some(s) = stats.as_mut() {
            s.records_in = records.len();
        }
        let (records, cast_warnings) = util::cast::apply_casts(records, cast_map);
        let (result, eval_warnings) = query::fast::eval::eval(query, records)?;
        let (result, limit_info) =
            apply_auto_limit(result, auto_limit.filter(|_| query.limit.is_none()));
        if let Some(s) = stats.as_mut() {
            s.records_out = result.len();
        }
        output::render(&result, fmt, color)?;
        if let Some((shown, total)) = limit_info {
            print_auto_limit_notice(shown, total, quiet);
        }
        print_warnings(&cast_warnings, quiet);
        print_warnings(&eval_warnings, quiet);
        return Ok(());
    }

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
                if show_auto_hint {
                    print_auto_limit_notice(effective_limit, matched, quiet);
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

/// Print the auto-limit notice to stderr as a Unicode box (after output completes).
fn print_auto_limit_notice(shown: usize, total: usize, quiet: bool) {
    if quiet {
        return;
    }
    let msg1 =
        format!("  {total} records matched · showing first {shown} · stdout is a terminal  ");
    let msg2 = "  Use --all / -A to show all, or pipe output to disable this limit.  ";
    let width = msg1.len().max(msg2.len());
    let bar = "─".repeat(width);
    eprintln!("╭─ qk {bar}╮");
    eprintln!("│{msg1:<width$}│");
    eprintln!("│{msg2:<width$}│");
    eprintln!("╰{bar}─────╯");
}

/// Truncate `records` to at most `limit`. Returns `(records, Option<(shown, total)>)`.
/// When the second element is `Some((n, total))`, the caller should print the auto-limit notice.
fn apply_auto_limit(
    records: Vec<record::Record>,
    limit: Option<usize>,
) -> (Vec<record::Record>, Option<(usize, usize)>) {
    let Some(n) = limit else {
        return (records, None);
    };
    let total = records.len();
    if total <= n {
        return (records, None);
    }
    (records.into_iter().take(n).collect(), Some((n, total)))
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
    // Safety net: if a path looks like a flag, it was likely a typo that
    // slipped past reorder_args (e.g. used via pipe or testing). Give a
    // better error than the OS "no such file" message.
    if path.starts_with('-') {
        return Err(unknown_flag_error(path));
    }

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

use clap::{Parser, ValueEnum};

/// qk — 一个替代 grep/awk/sed/jq 的快速结构化查询工具。
///
/// 用法示例:
///   qk where level=error app.log
///   qk where status>499 select path status requests.json
///   qk count by service app.log
///   qk sort latency desc limit 20 app.log
///   qk '.level == "error" | pick(.ts, .msg)' app.log
#[derive(Parser, Debug)]
#[command(name = "qk", version, about = "快速结构化查询工具")]
pub struct Cli {
    /// Query tokens and/or file paths (keyword mode).
    ///
    /// When the first token starts with `.`, `not `, or `|`, DSL mode is
    /// automatically selected. If no files are given, reads from stdin.
    #[arg(trailing_var_arg = true, allow_hyphen_values = false)]
    pub args: Vec<String>,

    /// Output format (ndjson / pretty / table / csv / raw).
    ///
    /// When omitted, the value from `~/.config/qk/config.toml` (`default_fmt`) is used,
    /// falling back to `ndjson`.
    #[arg(long, short = 'f')]
    pub fmt: Option<OutputFormat>,

    /// Print processing statistics to stderr after the query completes.
    ///
    /// Reports: records in, records out, elapsed time, output format.
    #[arg(long)]
    pub stats: bool,

    /// Print the detected format and parsed query, then exit.
    #[arg(long)]
    pub explain: bool,

    /// Force-enable color output (useful when piping to `less -R`).
    ///
    /// By default, color is used only when stdout is a terminal.
    /// `--color` overrides auto-detection and forces color on.
    #[arg(long)]
    pub color: bool,

    /// Disable color output entirely.
    ///
    /// Also honoured via the NO_COLOR environment variable.
    /// `--no-color` takes priority over `--color`.
    #[arg(long)]
    pub no_color: bool,

    /// Launch the interactive TUI browser.
    ///
    /// Opens a full-screen terminal UI where you can type a query and see
    /// results updating in real time.  All remaining args are treated as file
    /// paths.  Press Esc or Ctrl+C to exit.
    #[arg(long)]
    pub ui: bool,

    /// Treat CSV/TSV input as having no header row.
    ///
    /// Column names will be `col1`, `col2`, `col3`, etc. (1-indexed).
    /// Without this flag, the first row is always treated as a header.
    #[arg(long)]
    pub no_header: bool,

    /// Force a field to a specific type: `--cast FIELD=TYPE`.
    ///
    /// Can be specified multiple times. Applied before the query runs.
    /// Supported types: number (num/float/int), string (str/text), bool (boolean), null (none), auto.
    ///
    /// Examples:
    ///   --cast latency=number   → parse string "3001" as Number; warn + skip on failure
    ///   --cast status=string    → convert Number 200 → String "200"
    ///   --cast active=bool      → parse "true"/"1"/"yes" → Bool
    ///   --cast score=auto       → CSV-style inference (same logic as CSV parser)
    #[arg(long, value_name = "FIELD=TYPE")]
    pub cast: Vec<String>,

    /// Suppress all warnings (normally printed to stderr).
    ///
    /// Equivalent to redirecting stderr: `qk ... 2>/dev/null`.
    /// Use when warnings are expected and clutter is undesirable.
    #[arg(long, short = 'q')]
    pub quiet: bool,

    /// Show all matching records, disabling the auto-limit applied when
    /// stdout is a terminal.
    ///
    /// By default, when stdout is a terminal and no explicit `limit N` is
    /// given, qk shows only the first N records (N = `default_limit` in
    /// `~/.config/qk/config.toml`, default 20). `--all` bypasses this.
    #[arg(long, short = 'A')]
    pub all: bool,

    /// Enforce case-sensitive string matching.
    ///
    /// By default all string filters (`=`, `!=`, `contains`, `startswith`,
    /// `endswith`) are case-insensitive: `where level=ERROR` also matches
    /// `"error"` and `"Error"`. Use this flag to require an exact case match.
    ///
    /// Does not affect `regex` / `matches` (the regex pattern controls its
    /// own case via `(?i)`) or `glob` (always case-insensitive).
    #[arg(long, short = 'S')]
    pub case_sensitive: bool,
}

impl Cli {
    /// Returns `true` if color should be used for output.
    ///
    /// Priority (highest first):
    /// 1. `--no-color` flag → always off
    /// 2. `--color` flag    → always on (overrides `NO_COLOR` env)
    /// 3. `NO_COLOR` env    → off
    /// 4. default           → on when stdout is a terminal
    pub fn use_color(&self) -> bool {
        use std::io::IsTerminal;
        if self.no_color {
            return false;
        }
        if self.color {
            return true;
        }
        if std::env::var("NO_COLOR").is_ok() {
            return false;
        }
        std::io::stdout().is_terminal()
    }
}

/// Supported output formats.
#[derive(Debug, Clone, ValueEnum, Default)]
pub enum OutputFormat {
    /// Newline-delimited JSON (default, best for piping).
    #[default]
    Ndjson,
    /// Pretty-printed indented JSON (one record per block, blank line between).
    Pretty,
    /// Auto-aligned table with color highlighting.
    Table,
    /// CSV with a header row.
    Csv,
    /// Original matched lines without re-serialization.
    Raw,
}

impl OutputFormat {
    /// Parse a format name from a config-file string.
    ///
    /// Returns `None` for unrecognised values (config errors are non-fatal).
    pub fn from_config_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "ndjson" => Some(Self::Ndjson),
            "pretty" => Some(Self::Pretty),
            "table" => Some(Self::Table),
            "csv" => Some(Self::Csv),
            "raw" => Some(Self::Raw),
            _ => None,
        }
    }

    /// A short human-readable name for stats output.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ndjson => "ndjson",
            Self::Pretty => "pretty",
            Self::Table => "table",
            Self::Csv => "csv",
            Self::Raw => "raw",
        }
    }
}

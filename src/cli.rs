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

    /// Output format.
    #[arg(long, short = 'f', default_value = "ndjson")]
    pub fmt: OutputFormat,

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

    /// Treat CSV/TSV input as having no header row.
    ///
    /// Column names will be `col1`, `col2`, `col3`, etc. (1-indexed).
    /// Without this flag, the first row is always treated as a header.
    #[arg(long)]
    pub no_header: bool,
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

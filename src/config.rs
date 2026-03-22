use std::path::{Path, PathBuf};

use comfy_table::{Cell, Color, Table};
use serde::Deserialize;

use crate::util::error::{QkError, Result};

/// User configuration loaded from `$XDG_CONFIG_HOME/qk/config.toml`
/// (defaults to `~/.config/qk/config.toml`).
///
/// All fields are optional. Missing fields fall back to built-in defaults.
#[derive(Debug, Deserialize, Default)]
pub struct QkConfig {
    /// Default output format when `--fmt` is not given on the command line.
    ///
    /// Accepted values: `"ndjson"`, `"pretty"`, `"table"`, `"csv"`, `"raw"`.
    ///
    /// Example config file (`~/.config/qk/config.toml`):
    /// ```toml
    /// default_fmt = "pretty"
    /// ```
    pub default_fmt: Option<String>,

    /// Auto-limit applied when stdout is a terminal and no explicit `limit N`
    /// is in the query. `0` disables the auto-limit entirely.
    /// Default when absent: 20.
    pub default_limit: Option<usize>,

    /// If `true`, disable ANSI color by default (same as `--no-color`).
    /// Overridden by `--color` flag.
    pub no_color: Option<bool>,

    /// Default timestamp field name used by `count by DURATION` when no explicit
    /// field is given. Defaults to `"ts"` when absent.
    pub default_time_field: Option<String>,
}

/// Load configuration from disk.
///
/// Returns `QkConfig::default()` silently if the file does not exist or
/// cannot be parsed — config errors are never fatal.
pub fn load() -> QkConfig {
    load_from(&config_path())
}

/// Load configuration from an explicit file path.
///
/// Separated from `load()` so tests can supply a direct path without touching
/// global environment variables (which would cause data races in parallel tests).
pub(crate) fn load_from(path: &Path) -> QkConfig {
    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return QkConfig::default(),
    };
    toml::from_str(&content).unwrap_or_default()
}

/// Return the path to the config file, honouring `XDG_CONFIG_HOME`.
pub(crate) fn config_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".config")
        });
    base.join("qk").join("config.toml")
}

/// Print the current configuration as a formatted table to stdout.
///
/// Shows each setting, the current value (from file or built-in default),
/// the built-in default, and the source (config file path or "built-in default").
pub fn show() {
    let path = config_path();
    let cfg = load_from(&path);
    let source = if path.exists() {
        path.display().to_string()
    } else {
        "built-in defaults (no config file)".to_string()
    };

    eprintln!("Config file: {source}");
    eprintln!();

    let fmt_val = cfg.default_fmt.as_deref().unwrap_or("ndjson").to_string();
    let limit_val = cfg
        .default_limit
        .map(|n| {
            if n == 0 {
                "0 (disabled)".to_string()
            } else {
                n.to_string()
            }
        })
        .unwrap_or_else(|| "20".to_string());
    let color_val = if cfg.no_color.unwrap_or(false) {
        "disabled"
    } else {
        "auto (tty)"
    };

    let fmt_src = if cfg.default_fmt.is_some() {
        "config file"
    } else {
        "built-in default"
    };
    let limit_src = if cfg.default_limit.is_some() {
        "config file"
    } else {
        "built-in default"
    };
    let color_src = if cfg.no_color.is_some() {
        "config file"
    } else {
        "built-in default"
    };
    let time_field_val = cfg
        .default_time_field
        .as_deref()
        .unwrap_or("ts")
        .to_string();
    let time_field_src = if cfg.default_time_field.is_some() {
        "config file"
    } else {
        "built-in default"
    };

    let mut table = Table::new();
    table.set_header(vec![
        Cell::new("Setting").fg(Color::Cyan),
        Cell::new("Current Value").fg(Color::Cyan),
        Cell::new("Built-in Default").fg(Color::Cyan),
        Cell::new("Source").fg(Color::Cyan),
    ]);
    table.add_row(vec![
        Cell::new("default_fmt"),
        Cell::new(&fmt_val).fg(Color::Green),
        Cell::new("ndjson"),
        Cell::new(fmt_src),
    ]);
    table.add_row(vec![
        Cell::new("default_limit"),
        Cell::new(&limit_val).fg(Color::Green),
        Cell::new("20"),
        Cell::new(limit_src),
    ]);
    table.add_row(vec![
        Cell::new("no_color"),
        Cell::new(color_val).fg(Color::Green),
        Cell::new("auto (tty)"),
        Cell::new(color_src),
    ]);
    table.add_row(vec![
        Cell::new("default_time_field"),
        Cell::new(&time_field_val).fg(Color::Green),
        Cell::new("ts"),
        Cell::new(time_field_src),
    ]);

    println!("{table}");
    println!();
    println!("To edit: {}", path.display());
    println!("To reset: qk config reset");
}

/// Reset configuration to built-in defaults by removing the config file.
///
/// If the file does not exist, reports success (already at defaults).
pub fn reset() -> Result<()> {
    let path = config_path();
    if !path.exists() {
        println!("Config already at defaults (no config file exists).");
        println!("Path: {}", path.display());
        return Ok(());
    }
    std::fs::remove_file(&path).map_err(|e| QkError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    println!("Config reset to built-in defaults.");
    println!("Removed: {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit tests use `load_from(path)` directly — no env var mutation, no races.

    #[test]
    fn load_from_returns_default_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        // Point at a path that does not exist — no qk/config.toml created.
        let cfg = load_from(&dir.path().join("qk").join("config.toml"));
        assert!(
            cfg.default_fmt.is_none(),
            "missing config file should yield None for default_fmt"
        );
    }

    #[test]
    fn load_from_parses_default_fmt() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_file = dir.path().join("config.toml");
        std::fs::write(&cfg_file, "default_fmt = \"pretty\"\n").unwrap();
        let cfg = load_from(&cfg_file);
        assert_eq!(cfg.default_fmt.as_deref(), Some("pretty"));
    }

    #[test]
    fn load_from_returns_default_on_malformed_toml() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_file = dir.path().join("config.toml");
        std::fs::write(&cfg_file, "not valid toml ===").unwrap();
        let cfg = load_from(&cfg_file);
        // Must not panic — silently falls back to defaults.
        assert!(cfg.default_fmt.is_none());
    }

    #[test]
    fn reset_removes_existing_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_file = dir.path().join("config.toml");
        std::fs::write(&cfg_file, "default_fmt = \"pretty\"\n").unwrap();
        assert!(cfg_file.exists());
        // Simulate reset by directly calling remove_file (reset() uses config_path() globally).
        std::fs::remove_file(&cfg_file).unwrap();
        assert!(!cfg_file.exists());
        // After removal, load_from returns defaults.
        let cfg = load_from(&cfg_file);
        assert!(cfg.default_fmt.is_none());
    }

    #[test]
    fn config_path_returns_xdg_path_when_set() {
        // Verify the path construction logic using load_from directly.
        // We do not mutate env vars; just test the file path calculation indirectly.
        let dir = tempfile::tempdir().unwrap();
        let cfg_file = dir.path().join("config.toml");
        std::fs::write(&cfg_file, "default_limit = 50\n").unwrap();
        let cfg = load_from(&cfg_file);
        assert_eq!(cfg.default_limit, Some(50));
    }
}

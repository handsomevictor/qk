use std::path::{Path, PathBuf};

use serde::Deserialize;

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
fn config_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".config")
        });
    base.join("qk").join("config.toml")
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
}

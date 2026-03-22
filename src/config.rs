use std::path::PathBuf;

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
    let path = config_path();
    let content = match std::fs::read_to_string(&path) {
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

    #[test]
    fn load_returns_default_when_file_missing() {
        // Point XDG_CONFIG_HOME at a directory that has no qk/config.toml.
        let dir = tempfile::tempdir().unwrap();
        // Temporarily override the env var for this test.
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
        let cfg = load();
        std::env::remove_var("XDG_CONFIG_HOME");
        assert!(
            cfg.default_fmt.is_none(),
            "missing config file should yield None for default_fmt"
        );
    }

    #[test]
    fn load_parses_default_fmt() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_dir = dir.path().join("qk");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        std::fs::write(cfg_dir.join("config.toml"), "default_fmt = \"pretty\"\n").unwrap();
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
        let cfg = load();
        std::env::remove_var("XDG_CONFIG_HOME");
        assert_eq!(cfg.default_fmt.as_deref(), Some("pretty"));
    }

    #[test]
    fn load_returns_default_on_malformed_toml() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_dir = dir.path().join("qk");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        std::fs::write(cfg_dir.join("config.toml"), "not valid toml ===").unwrap();
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
        let cfg = load();
        std::env::remove_var("XDG_CONFIG_HOME");
        // Must not panic — silently falls back to defaults.
        assert!(cfg.default_fmt.is_none());
    }
}

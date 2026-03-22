use thiserror::Error;

/// Project-wide error type.
#[derive(Debug, Error)]
pub enum QkError {
    #[error("IO error reading '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("parse error in '{file}' at line {line}: {msg}")]
    Parse {
        file: String,
        line: usize,
        msg: String,
    },

    #[error("query syntax error: {0}")]
    Query(String),

    #[allow(dead_code)]
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    /// An argument that looks like a flag (`-x` / `--xxx`) but is not recognised.
    ///
    /// Common cause: a typo in a flag name, or a flag placed after query tokens
    /// (flags must come before the query in some shells — `reorder_args` fixes this
    /// automatically, but unrecognised flags are always an error).
    #[error("{msg}")]
    UnknownFlag { msg: String },
}

pub type Result<T> = std::result::Result<T, QkError>;

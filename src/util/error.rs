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
}

pub type Result<T> = std::result::Result<T, QkError>;

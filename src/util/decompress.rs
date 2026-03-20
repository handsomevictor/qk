use std::io::Read;

use flate2::read::GzDecoder;

use crate::util::error::{QkError, Result};

/// Magic bytes for gzip.
const GZIP_MAGIC: &[u8] = &[0x1f, 0x8b];

/// Returns `true` if `bytes` begins with the gzip magic signature.
pub fn is_gzip(bytes: &[u8]) -> bool {
    bytes.starts_with(GZIP_MAGIC)
}

/// Decompress a gzip byte slice into a UTF-8 String.
pub fn decompress_gz(bytes: &[u8], path: &str) -> Result<String> {
    let mut decoder = GzDecoder::new(bytes);
    let mut out = String::new();
    decoder.read_to_string(&mut out).map_err(|e| QkError::Io {
        path: path.to_string(),
        source: e,
    })?;
    Ok(out)
}

/// Given a compressed file path, return the inferred inner filename.
///
/// `"app.log.gz"` → `"app.log"`,  `"data.ndjson.gz"` → `"data.ndjson"`.
/// Falls back to the original path if no known compression extension is found.
pub fn inner_filename(path: &str) -> &str {
    if let Some(stem) = path.strip_suffix(".gz") {
        return stem;
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::GzEncoder, Compression};
    use std::io::Write;

    fn make_gz(content: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(content).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn detects_gzip_magic() {
        let gz = make_gz(b"hello");
        assert!(is_gzip(&gz));
        assert!(!is_gzip(b"hello world"));
        assert!(!is_gzip(b""));
    }

    #[test]
    fn decompresses_gzip() {
        let original = b"{\"level\":\"error\"}\n{\"level\":\"info\"}\n";
        let gz = make_gz(original);
        let result = decompress_gz(&gz, "test.log.gz").unwrap();
        assert_eq!(result.as_bytes(), original);
    }

    #[test]
    fn infers_inner_filename() {
        assert_eq!(inner_filename("app.log.gz"), "app.log");
        assert_eq!(inner_filename("data.ndjson.gz"), "data.ndjson");
        assert_eq!(inner_filename("plain.log"), "plain.log");
    }
}

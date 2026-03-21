use memchr::memchr;

/// All supported input formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Format {
    /// Newline-delimited JSON: one JSON object per line.
    Ndjson,
    /// Full JSON document: array or single object.
    Json,
    /// Comma-separated values with a header row.
    Csv,
    /// Tab-separated values with a header row.
    Tsv,
    /// logfmt: `key=value` pairs separated by whitespace.
    Logfmt,
    /// YAML document (single or multi-doc).
    Yaml,
    /// TOML document.
    Toml,
    /// Gzip-compressed file; the inner format is detected after decompression.
    Gzip,
    /// Fallback: each line becomes a record with a `line` field.
    Plaintext,
}

/// Detect the format of an input by examining its content and optional filename.
///
/// Checks the file extension first, then falls back to content heuristics
/// using at most the first 512 bytes.
pub fn sniff(bytes: &[u8], filename: Option<&str>) -> Format {
    // Gzip magic bytes — must be checked before anything else
    if bytes.starts_with(&[0x1f, 0x8b]) {
        return Format::Gzip;
    }

    if let Some(name) = filename {
        if let Some(fmt) = detect_by_extension(name) {
            return fmt;
        }
    }
    detect_from_content(bytes)
}

fn detect_by_extension(name: &str) -> Option<Format> {
    let lower = name.to_ascii_lowercase();

    if lower.ends_with(".gz") {
        return Some(Format::Gzip);
    }
    if lower.ends_with(".yaml") || lower.ends_with(".yml") {
        return Some(Format::Yaml);
    }
    if lower.ends_with(".toml") {
        return Some(Format::Toml);
    }
    if lower.ends_with(".tsv") {
        return Some(Format::Tsv);
    }
    if lower.ends_with(".csv") {
        return Some(Format::Csv);
    }
    if lower.ends_with(".ndjson") {
        return Some(Format::Ndjson);
    }
    // .json and .log fall through to content detection
    None
}

fn detect_from_content(bytes: &[u8]) -> Format {
    let head = &bytes[..bytes.len().min(512)];
    let s = String::from_utf8_lossy(head);
    let trimmed = s.trim_start();

    if trimmed.starts_with('{') {
        return detect_json_variant(head);
    }
    if trimmed.starts_with('[') {
        // A TOML section header looks like `[word]`, not `[{` or `["`.
        if looks_like_toml(trimmed) {
            return Format::Toml;
        }
        return Format::Json;
    }
    // YAML: document separator or sequence marker
    if trimmed.starts_with("---") || trimmed.starts_with("- ") {
        return Format::Yaml;
    }
    // TOML: key = value (with spaces around =)
    if looks_like_toml(trimmed) {
        return Format::Toml;
    }
    if looks_like_logfmt(trimmed) {
        return Format::Logfmt;
    }
    if looks_like_csv(trimmed) {
        return Format::Csv;
    }
    Format::Plaintext
}

/// Distinguish NDJSON (multiple `{…}` lines) from a single JSON object.
fn detect_json_variant(bytes: &[u8]) -> Format {
    match memchr(b'\n', bytes) {
        Some(nl) => {
            let after = String::from_utf8_lossy(&bytes[nl + 1..]);
            let after_trimmed = after.trim_start();
            if after_trimmed.starts_with('{') || after_trimmed.is_empty() {
                Format::Ndjson
            } else {
                Format::Json
            }
        }
        None => Format::Ndjson,
    }
}

fn looks_like_toml(s: &str) -> bool {
    let line = s.lines().next().unwrap_or("");
    let trimmed = line.trim_start();
    // TOML section header: [section_name] — no JSON-like chars ({, ", ') inside
    if trimmed.starts_with('[') {
        if let Some(close) = trimmed.find(']') {
            let between = &trimmed[1..close];
            if !between.is_empty()
                && !between.contains('{')
                && !between.contains('"')
                && !between.contains('\'')
            {
                return true;
            }
        }
    }
    // TOML key-value: key = value (spaces around =)
    if let Some(pos) = line.find(" = ") {
        let key = &line[..pos];
        return !key.is_empty() && !key.contains(':');
    }
    false
}

fn looks_like_logfmt(s: &str) -> bool {
    let line = s.lines().next().unwrap_or("");
    line.split_whitespace()
        .filter(|tok| tok.contains('='))
        .count()
        >= 2
}

fn looks_like_csv(s: &str) -> bool {
    let line = s.lines().next().unwrap_or("");
    line.contains(',') && line.split(',').count() >= 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_gzip_by_magic_bytes() {
        let gz_magic = &[0x1f, 0x8b, 0x08, 0x00];
        assert_eq!(sniff(gz_magic, None), Format::Gzip);
    }

    #[test]
    fn detects_gzip_by_extension() {
        assert_eq!(sniff(b"anything", Some("data.log.gz")), Format::Gzip);
    }

    #[test]
    fn detects_ndjson() {
        let data = b"{\"level\":\"error\"}\n{\"level\":\"info\"}\n";
        assert_eq!(sniff(data, None), Format::Ndjson);
    }

    #[test]
    fn detects_json_array() {
        assert_eq!(sniff(b"[{\"a\":1}]", None), Format::Json);
    }

    #[test]
    fn detects_csv_by_extension() {
        assert_eq!(sniff(b"name,age\n", Some("data.csv")), Format::Csv);
    }

    #[test]
    fn detects_tsv_by_extension() {
        assert_eq!(sniff(b"name\tage\n", Some("data.tsv")), Format::Tsv);
    }

    #[test]
    fn detects_logfmt() {
        let data = b"level=error service=api msg=\"timeout\"\n";
        assert_eq!(sniff(data, None), Format::Logfmt);
    }

    #[test]
    fn detects_yaml_by_extension() {
        assert_eq!(sniff(b"---\nfoo: bar\n", Some("config.yaml")), Format::Yaml);
    }

    #[test]
    fn detects_yaml_by_content() {
        assert_eq!(sniff(b"---\nfoo: bar\n", None), Format::Yaml);
    }

    #[test]
    fn detects_toml_by_extension() {
        assert_eq!(sniff(b"[section]\n", Some("config.toml")), Format::Toml);
    }

    #[test]
    fn detects_toml_section_by_content() {
        assert_eq!(sniff(b"[server]\nport = 8080\n", None), Format::Toml);
    }

    #[test]
    fn detects_toml_kv_by_content() {
        assert_eq!(
            sniff(b"name = \"qk\"\nversion = \"1.0\"\n", None),
            Format::Toml
        );
    }

    #[test]
    fn falls_back_to_plaintext() {
        assert_eq!(sniff(b"hello world\nplain text\n", None), Format::Plaintext);
    }
}

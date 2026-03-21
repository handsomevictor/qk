use std::fs::File;

use memmap2::MmapOptions;

use crate::util::error::{QkError, Result};

/// Files smaller than this threshold are read with `fs::read`.
/// Larger files use memory-mapped I/O.
pub const MMAP_THRESHOLD: u64 = 64 * 1024; // 64 KiB

/// Read a file's raw bytes.
///
/// Uses memory-mapped I/O for files larger than [`MMAP_THRESHOLD`], which
/// avoids an extra kernel-to-userspace copy and lets the OS page cache handle
/// large files efficiently.
pub fn read_bytes(path: &str) -> Result<Vec<u8>> {
    let meta = std::fs::metadata(path).map_err(|e| QkError::Io {
        path: path.to_string(),
        source: e,
    })?;

    if meta.len() == 0 {
        return Ok(Vec::new());
    }

    if meta.len() >= MMAP_THRESHOLD {
        let file = File::open(path).map_err(|e| QkError::Io {
            path: path.to_string(),
            source: e,
        })?;
        // Safety: opened read-only; we copy to Vec<u8> immediately.
        let mmap = unsafe { MmapOptions::new().map(&file) }.map_err(|e| QkError::Io {
            path: path.to_string(),
            source: e,
        })?;
        Ok(mmap.to_vec())
    } else {
        std::fs::read(path).map_err(|e| QkError::Io {
            path: path.to_string(),
            source: e,
        })
    }
}

/// Read a file's contents as a UTF-8 String.
///
/// Thin wrapper around [`read_bytes`] that validates UTF-8.
#[allow(dead_code)]
pub fn read_string(path: &str) -> Result<String> {
    let bytes = read_bytes(path)?;
    String::from_utf8(bytes).map_err(|_| QkError::Parse {
        file: path.to_string(),
        line: 0,
        msg: "file contains invalid UTF-8 bytes".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp(content: &[u8]) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        f
    }

    #[test]
    fn reads_small_file_as_bytes() {
        let f = write_temp(b"hello world");
        let bytes = read_bytes(f.path().to_str().unwrap()).unwrap();
        assert_eq!(bytes, b"hello world");
    }

    #[test]
    fn reads_empty_file() {
        let f = write_temp(b"");
        let bytes = read_bytes(f.path().to_str().unwrap()).unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn reads_large_file_via_mmap() {
        let data = vec![b'a'; (MMAP_THRESHOLD + 100) as usize];
        let f = write_temp(&data);
        let bytes = read_bytes(f.path().to_str().unwrap()).unwrap();
        assert_eq!(bytes.len(), data.len());
        assert!(bytes.iter().all(|&b| b == b'a'));
    }

    #[test]
    fn read_string_validates_utf8() {
        // Write invalid UTF-8
        let f = write_temp(&[0xff, 0xfe]);
        let result = read_string(f.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn returns_error_for_missing_file() {
        assert!(read_bytes("/nonexistent/path/file.txt").is_err());
    }
}

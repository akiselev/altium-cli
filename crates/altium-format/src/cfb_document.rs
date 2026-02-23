//! Layer 1 of the 5-layer parsing stack: raw CFB container I/O.
//! Wraps `cfb::CompoundFile` with error mapping to `AltiumFormatError`.
//! Holds no consumption state — see `TrackedCfbDocument` for stream tracking.
use std::collections::HashSet;
use std::io::{Cursor, Read, Write};
use std::path::Path;

use crate::{AltiumFormatError, Result};

pub(crate) struct CfbDocument {
    inner: cfb::CompoundFile<Cursor<Vec<u8>>>,
}

impl std::fmt::Debug for CfbDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CfbDocument").finish_non_exhaustive()
    }
}

impl CfbDocument {
    // Reads the file at `path` entirely into memory and opens it as a CFB container.
    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        let cursor = Cursor::new(bytes);
        let inner = cfb::CompoundFile::open(cursor)
            .map_err(|e| AltiumFormatError::CfbError(e.to_string()))?;
        Ok(Self { inner })
    }

    // Reads the entire stream at `path` into a Vec<u8>. Returns StreamNotFound if absent.
    pub(crate) fn read_stream(&mut self, path: &str) -> Result<Vec<u8>> {
        if !self.inner.exists(path) {
            return Err(AltiumFormatError::StreamNotFound(path.to_owned()));
        }
        let mut stream = self
            .inner
            .open_stream(path)
            .map_err(|e| AltiumFormatError::CfbError(e.to_string()))?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf)?;
        Ok(buf)
    }

    // Reads the stream at `path` if it exists; returns Ok(None) when absent.
    pub(crate) fn read_stream_optional(&mut self, path: &str) -> Result<Option<Vec<u8>>> {
        if !self.inner.exists(path) {
            return Ok(None);
        }
        self.read_stream(path).map(Some)
    }

    // Returns true if the entry at `path` exists in the CFB container.
    pub(crate) fn exists(&self, path: &str) -> bool {
        self.inner.exists(path)
    }

    // Returns (storages, streams) for the given storage path.
    pub(crate) fn list_entries(&mut self, path: &str) -> Result<(Vec<String>, Vec<String>)> {
        let entries = self
            .inner
            .read_storage(path)
            .map_err(|e| AltiumFormatError::CfbError(e.to_string()))?;
        let mut storages = Vec::new();
        let mut streams = Vec::new();
        for entry in entries {
            let name = entry.name().to_owned();
            if entry.is_storage() {
                storages.push(name);
            } else {
                streams.push(name);
            }
        }
        Ok((storages, streams))
    }

    // Creates a new in-memory CFB container (V3, 512-byte sectors).
    pub(crate) fn create() -> Result<Self> {
        let cursor = Cursor::new(Vec::new());
        let inner = cfb::CompoundFile::create(cursor)
            .map_err(|e| AltiumFormatError::CfbError(e.to_string()))?;
        Ok(Self { inner })
    }

    // Creates a sub-storage at the given path.
    pub(crate) fn create_storage(&mut self, path: &str) -> Result<()> {
        self.inner
            .create_storage(path)
            .map_err(|e| AltiumFormatError::CfbError(e.to_string()))?;
        Ok(())
    }

    // Creates (or overwrites) a stream at the given path with the provided data.
    pub(crate) fn write_stream(&mut self, path: &str, data: &[u8]) -> Result<()> {
        let mut stream = self
            .inner
            .create_stream(path)
            .map_err(|e| AltiumFormatError::CfbError(e.to_string()))?;
        stream
            .write_all(data)
            .map_err(|e| AltiumFormatError::CfbError(e.to_string()))?;
        stream
            .flush()
            .map_err(|e| AltiumFormatError::CfbError(e.to_string()))?;
        Ok(())
    }

    // Flushes the CFB container and writes the result to a file.
    // Consumes self because into_stream() takes ownership of the CompoundFile.
    pub(crate) fn save_to_file(mut self, path: impl AsRef<Path>) -> Result<()> {
        self.inner
            .flush()
            .map_err(|e| AltiumFormatError::CfbError(e.to_string()))?;
        let cursor = self.inner.into_inner();
        let buf = cursor.into_inner();
        std::fs::write(path, &buf)?;
        Ok(())
    }

    // Walks all CFB entries recursively from root and returns their full paths.
    pub(crate) fn enumerate_all_entries(&mut self) -> Result<HashSet<String>> {
        let mut result = HashSet::new();
        self.enumerate_recursive("/", &mut result)?;
        Ok(result)
    }

    // Recursively walks all CFB entries under `path`, appending paths to `out`.
    fn enumerate_recursive(&mut self, path: &str, out: &mut HashSet<String>) -> Result<()> {
        let entries = self
            .inner
            .read_storage(path)
            .map_err(|e| AltiumFormatError::CfbError(e.to_string()))?;
        let children: Vec<(String, bool)> = entries
            .map(|e| {
                (
                    e.path()
                        .display()
                        .to_string()
                        .trim_end_matches('/')
                        .to_owned(),
                    e.is_storage(),
                )
            })
            .collect();
        for (child_path, is_storage) in children {
            out.insert(child_path.clone());
            if is_storage {
                self.enumerate_recursive(&child_path, out)?;
            }
        }
        Ok(())
    }
}

// Tests are inline unit tests rather than integration tests in tests/ because
// CfbDocument is pub(crate) and cannot be accessed from a separate test crate.
#[cfg(test)]
mod tests {
    use super::*;

    fn data_path(filename: &str) -> std::path::PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        std::path::Path::new(manifest_dir)
            .join("../../data")
            .join(filename)
    }

    #[test]
    fn open_blank_schlib_and_read_file_header() {
        let path = data_path("BlankSchlibComponent.SchLib");
        let mut doc = CfbDocument::open(&path).expect("should open valid SchLib");
        let bytes = doc
            .read_stream("/FileHeader")
            .expect("FileHeader stream must exist");
        assert!(!bytes.is_empty(), "FileHeader stream must be non-empty");
    }

    #[test]
    fn enumerate_all_entries_returns_expected_paths() {
        let path = data_path("BlankSchlibComponent.SchLib");
        let mut doc = CfbDocument::open(&path).expect("should open valid SchLib");
        let entries = doc.enumerate_all_entries().expect("enumerate must succeed");
        // Top-level streams and storages known from BlankSchlibComponent.SchLib.
        assert!(
            entries.contains("/FileHeader"),
            "FileHeader must be in entry set"
        );
        assert!(entries.contains("/Storage"), "Storage must be in entry set");
        assert!(
            entries.contains("/Component_1"),
            "Component_1 storage must be in entry set"
        );
        // Nested stream — proves the recursive walk descends into storages.
        assert!(
            entries.contains("/Component_1/Data"),
            "nested /Component_1/Data must be in entry set"
        );
    }

    #[test]
    fn read_stream_optional_missing_returns_none() {
        let path = data_path("BlankSchlibComponent.SchLib");
        let mut doc = CfbDocument::open(&path).expect("should open valid SchLib");
        let result = doc
            .read_stream_optional("/NonExistentStream")
            .expect("read_stream_optional must not error on missing stream");
        assert!(result.is_none(), "missing stream must return None");
    }

    #[test]
    fn open_nonexistent_path_returns_io_error() {
        let result = CfbDocument::open("/no/such/file/exists.SchLib");
        assert!(
            matches!(result, Err(AltiumFormatError::Io(_))),
            "expected Io error"
        );
    }

    #[test]
    fn read_stream_missing_returns_stream_not_found() {
        let path = data_path("BlankSchlibComponent.SchLib");
        let mut doc = CfbDocument::open(&path).expect("should open valid SchLib");
        let result = doc.read_stream("/NonExistentStream");
        assert!(
            matches!(result, Err(AltiumFormatError::StreamNotFound(_))),
            "expected StreamNotFound error"
        );
    }
}

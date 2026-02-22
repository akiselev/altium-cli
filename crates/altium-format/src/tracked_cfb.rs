//! Layer 2 of the 5-layer parsing stack: CFB stream consumption tracking.
//! Wraps `CfbDocument` and records which entries have been read.
//! `assert_all_consumed` enforces the invariant that every CFB stream is
//! explicitly handled before `SchLib::open` returns.
use std::collections::HashSet;
use std::path::Path;

use crate::cfb_document::CfbDocument;
use crate::{AltiumFormatError, Result};

pub(crate) struct TrackedCfbDocument {
    inner: CfbDocument,
    all_entries: HashSet<String>,
    consumed: HashSet<String>,
}

impl TrackedCfbDocument {
    // Opens the CFB at `path`, enumerating all entries upfront for exhaustion tracking.
    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut inner = CfbDocument::open(path)?;
        let all_entries = inner.enumerate_all_entries()?;
        let mut consumed = HashSet::new();
        // Root storage is implicit; never appears as unconsumed.
        consumed.insert("/".to_owned());
        Ok(Self { inner, all_entries, consumed })
    }

    // Marks stream as consumed and reads it; returns StreamNotFound if absent.
    pub(crate) fn read_stream(&mut self, path: &str) -> Result<Vec<u8>> {
        self.consumed.insert(path.to_owned());
        self.inner.read_stream(path)
    }

    // Marks stream as consumed (whether or not it exists) and reads it; returns Ok(None) if absent.
    pub(crate) fn read_stream_optional(&mut self, path: &str) -> Result<Option<Vec<u8>>> {
        // Mark as consumed even when absent to avoid false-positive unconsumed errors.
        self.consumed.insert(path.to_owned());
        self.inner.read_stream_optional(path)
    }

    // Existence checks do not mark a stream as consumed; only read_stream and
    // read_stream_optional claim ownership. Call read_stream_optional to both
    // check and consume in one step.
    pub(crate) fn exists(&self, path: &str) -> bool {
        self.inner.exists(path)
    }

    // Marks the parent storage node as consumed and returns (storages, streams).
    // Trailing slashes are stripped before insertion so "/Foo" and "/Foo/" are equivalent.
    // Root "/" is preserved as-is because trim_end_matches('/') would produce an empty string.
    pub(crate) fn list_entries(&mut self, path: &str) -> Result<(Vec<String>, Vec<String>)> {
        let normalized = if path == "/" { "/" } else { path.trim_end_matches('/') };
        self.consumed.insert(normalized.to_owned());
        self.inner.list_entries(normalized)
    }

    // Returns Err(UnconsumedStreams) if any enumerated entry was never read or listed.
    // Call at the end of SchLib::open to enforce the total-consumption invariant.
    pub(crate) fn assert_all_consumed(&self) -> Result<()> {
        let mut unconsumed: Vec<String> = self
            .all_entries
            .difference(&self.consumed)
            .cloned()
            .collect();
        if unconsumed.is_empty() {
            return Ok(());
        }
        unconsumed.sort();
        Err(AltiumFormatError::UnconsumedStreams { paths: unconsumed })
    }
}

// Tests are inline unit tests rather than integration tests in tests/ because
// TrackedCfbDocument is pub(crate) and cannot be accessed from a separate test crate.
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
    fn assert_all_consumed_succeeds_when_all_streams_are_read() {
        let path = data_path("BlankSchlibComponent.SchLib");
        let mut doc = TrackedCfbDocument::open(&path).expect("should open valid SchLib");
        // List root to discover top-level storages and streams (returns bare names).
        let (root_storages, root_streams) =
            doc.list_entries("/").expect("list_entries root must succeed");
        // Read each top-level stream using its full path.
        for name in &root_streams {
            let full = format!("/{name}");
            doc.read_stream(&full).expect("top-level stream must be readable");
        }
        // For each top-level storage, list its children and read all nested streams.
        for storage_name in &root_storages {
            let storage_path = format!("/{storage_name}");
            let (_, nested_streams) =
                doc.list_entries(&storage_path).expect("list_entries storage must succeed");
            for stream_name in nested_streams {
                let full = format!("{storage_path}/{stream_name}");
                doc.read_stream(&full).expect("nested stream must be readable");
            }
        }
        doc.assert_all_consumed().expect("all streams consumed; assert must return Ok");
    }

    #[test]
    fn assert_all_consumed_fails_when_nothing_is_read() {
        let path = data_path("BlankSchlibComponent.SchLib");
        let doc = TrackedCfbDocument::open(&path).expect("should open valid SchLib");
        let result = doc.assert_all_consumed();
        match result {
            Err(AltiumFormatError::UnconsumedStreams { paths }) => {
                // The 4 known entries must all appear in the unconsumed list.
                assert!(paths.contains(&"/FileHeader".to_owned()), "/FileHeader must be unconsumed");
                assert!(paths.contains(&"/Storage".to_owned()), "/Storage must be unconsumed");
                assert!(
                    paths.contains(&"/Component_1".to_owned()),
                    "/Component_1 must be unconsumed"
                );
                assert!(
                    paths.contains(&"/Component_1/Data".to_owned()),
                    "/Component_1/Data must be unconsumed"
                );
            }
            Ok(()) => panic!("expected UnconsumedStreams error, got Ok"),
            Err(e) => panic!("expected UnconsumedStreams error, got: {e:?}"),
        }
    }

    #[test]
    fn read_stream_optional_absent_does_not_cause_false_positive_unconsumed() {
        let path = data_path("BlankSchlibComponent.SchLib");
        let mut doc = TrackedCfbDocument::open(&path).expect("should open valid SchLib");
        // Mark a non-existent stream as consumed; it must not appear as unconsumed later.
        let result = doc
            .read_stream_optional("/NonExistentStream")
            .expect("read_stream_optional must not error on missing stream");
        assert!(result.is_none(), "absent stream must return None");
        // Now consume all real entries so assert_all_consumed passes.
        let (root_storages, root_streams) =
            doc.list_entries("/").expect("list_entries root must succeed");
        for name in &root_streams {
            let full = format!("/{name}");
            doc.read_stream(&full).expect("top-level stream must be readable");
        }
        for storage_name in &root_storages {
            let storage_path = format!("/{storage_name}");
            let (_, nested_streams) =
                doc.list_entries(&storage_path).expect("list_entries storage must succeed");
            for stream_name in nested_streams {
                let full = format!("{storage_path}/{stream_name}");
                doc.read_stream(&full).expect("nested stream must be readable");
            }
        }
        doc.assert_all_consumed()
            .expect("non-existent optional read must not produce false-positive unconsumed error");
    }

}

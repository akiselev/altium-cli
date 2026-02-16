// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! CFB stream-level comparison utilities for integration tests.
//!
//! Compares two OLE/CFB compound files at the stream level rather than
//! byte-level file comparison (which would fail due to sector allocation,
//! timestamps, and directory ordering differences).
//!
//! Text streams (>80% printable bytes) are compared via normalized
//! parameter comparison (sorted keys). Binary streams use byte equality.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::Cursor;

use altium_format::v2::parameters::ParameterCollection;

/// Report of differences between two CFB files.
#[derive(Debug, Default)]
pub struct CfbDiffReport {
    /// Stream paths that matched perfectly.
    pub matched: Vec<String>,
    /// Text streams with parameter-level differences.
    pub text_diffs: Vec<TextStreamDiff>,
    /// Binary streams with byte-level differences.
    pub binary_diffs: Vec<BinaryStreamDiff>,
    /// Streams only present in the original file.
    pub only_in_original: Vec<String>,
    /// Streams only present in the rebuilt file.
    pub only_in_rebuilt: Vec<String>,
}

impl CfbDiffReport {
    /// Returns true if the files are equivalent (no diffs, nothing missing).
    pub fn is_match(&self) -> bool {
        self.text_diffs.is_empty()
            && self.binary_diffs.is_empty()
            && self.only_in_original.is_empty()
            && self.only_in_rebuilt.is_empty()
    }

    /// Total number of differences found.
    pub fn diff_count(&self) -> usize {
        self.text_diffs.len()
            + self.binary_diffs.len()
            + self.only_in_original.len()
            + self.only_in_rebuilt.len()
    }
}

impl fmt::Display for CfbDiffReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== CFB Diff Report ===")?;
        writeln!(f, "Matched streams: {}", self.matched.len())?;

        if !self.only_in_original.is_empty() {
            writeln!(f, "\n--- Only in original ({}) ---", self.only_in_original.len())?;
            for s in &self.only_in_original {
                writeln!(f, "  {}", s)?;
            }
        }
        if !self.only_in_rebuilt.is_empty() {
            writeln!(f, "\n--- Only in rebuilt ({}) ---", self.only_in_rebuilt.len())?;
            for s in &self.only_in_rebuilt {
                writeln!(f, "  {}", s)?;
            }
        }
        if !self.text_diffs.is_empty() {
            writeln!(f, "\n--- Text stream diffs ({}) ---", self.text_diffs.len())?;
            for diff in &self.text_diffs {
                writeln!(f, "  Stream: {}", diff.stream_name)?;
                for (key, orig, rebuilt) in &diff.param_diffs {
                    writeln!(f, "    {}: {:?} -> {:?}", key, orig, rebuilt)?;
                }
            }
        }
        if !self.binary_diffs.is_empty() {
            writeln!(f, "\n--- Binary stream diffs ({}) ---", self.binary_diffs.len())?;
            for diff in &self.binary_diffs {
                writeln!(
                    f,
                    "  Stream: {} (orig {} bytes, rebuilt {} bytes, first diff at byte {})",
                    diff.stream_name,
                    diff.original_len,
                    diff.rebuilt_len,
                    diff.first_diff_offset.map_or("N/A".to_string(), |o| o.to_string()),
                )?;
            }
        }
        if self.is_match() {
            writeln!(f, "\nResult: PERFECT MATCH")?;
        } else {
            writeln!(f, "\nResult: {} difference(s) found", self.diff_count())?;
        }
        Ok(())
    }
}

/// Differences in a text (parameter-based) stream.
#[derive(Debug)]
pub struct TextStreamDiff {
    /// CFB stream path.
    pub stream_name: String,
    /// Parameter differences: (key, original_value, rebuilt_value).
    /// `None` means the key was missing from that side.
    pub param_diffs: Vec<(String, Option<String>, Option<String>)>,
}

/// Differences in a binary stream.
#[derive(Debug)]
pub struct BinaryStreamDiff {
    /// CFB stream path.
    pub stream_name: String,
    /// Length of the original stream.
    pub original_len: usize,
    /// Length of the rebuilt stream.
    pub rebuilt_len: usize,
    /// Byte offset of the first difference (None if lengths differ and shorter is a prefix).
    pub first_diff_offset: Option<usize>,
}

/// Heuristic: a stream is "text" if >80% of its bytes are printable ASCII.
pub fn is_text_stream(data: &[u8]) -> bool {
    if data.is_empty() {
        return true;
    }
    let printable = data
        .iter()
        .filter(|&&b| b == b'\n' || b == b'\r' || b == b'\t' || (b >= 0x20 && b <= 0x7E))
        .count();
    (printable as f64 / data.len() as f64) > 0.80
}

/// Compare two CFB files at the stream level.
///
/// `original` and `rebuilt` are the raw bytes of the two CFB files.
pub fn compare_cfb_files(original: &[u8], rebuilt: &[u8]) -> CfbDiffReport {
    let mut report = CfbDiffReport::default();

    let mut orig_cfb = match cfb::CompoundFile::open(Cursor::new(original)) {
        Ok(c) => c,
        Err(e) => {
            report
                .only_in_original
                .push(format!("(failed to open original: {})", e));
            return report;
        }
    };
    let mut rebuilt_cfb = match cfb::CompoundFile::open(Cursor::new(rebuilt)) {
        Ok(c) => c,
        Err(e) => {
            report
                .only_in_rebuilt
                .push(format!("(failed to open rebuilt: {})", e));
            return report;
        }
    };

    // Collect stream paths from both
    let orig_paths: BTreeSet<String> = orig_cfb
        .walk()
        .filter(|e| e.is_stream())
        .filter_map(|e| Some(e.path().to_str()?.to_string()))
        .collect();
    let rebuilt_paths: BTreeSet<String> = rebuilt_cfb
        .walk()
        .filter(|e| e.is_stream())
        .filter_map(|e| Some(e.path().to_str()?.to_string()))
        .collect();

    let all_paths: BTreeSet<String> = orig_paths.union(&rebuilt_paths).cloned().collect();

    // Read stream data
    let orig_streams = read_streams(&mut orig_cfb, &orig_paths);
    let rebuilt_streams = read_streams(&mut rebuilt_cfb, &rebuilt_paths);

    for path in all_paths {
        match (orig_streams.get(&path), rebuilt_streams.get(&path)) {
            (Some(orig_data), Some(rebuilt_data)) => {
                if is_text_stream(orig_data) && is_text_stream(rebuilt_data) {
                    compare_text_streams(&path, orig_data, rebuilt_data, &mut report);
                } else {
                    compare_binary_streams(&path, orig_data, rebuilt_data, &mut report);
                }
            }
            (Some(_), None) => {
                report.only_in_original.push(path);
            }
            (None, Some(_)) => {
                report.only_in_rebuilt.push(path);
            }
            (None, None) => unreachable!(),
        }
    }

    report
}

/// Read all streams from a CFB into a map of path -> bytes.
fn read_streams<R: std::io::Read + std::io::Seek>(
    cfb: &mut cfb::CompoundFile<R>,
    paths: &BTreeSet<String>,
) -> BTreeMap<String, Vec<u8>> {
    use std::io::Read;
    let mut result = BTreeMap::new();
    for path in paths {
        if let Ok(mut stream) = cfb.open_stream(path) {
            let mut data = Vec::new();
            if stream.read_to_end(&mut data).is_ok() {
                result.insert(path.clone(), data);
            }
        }
    }
    result
}

/// Compare two text streams using normalized parameter comparison.
fn compare_text_streams(
    path: &str,
    orig_data: &[u8],
    rebuilt_data: &[u8],
    report: &mut CfbDiffReport,
) {
    let orig_text = String::from_utf8_lossy(orig_data);
    let rebuilt_text = String::from_utf8_lossy(rebuilt_data);

    let orig_params = ParameterCollection::from_string(&orig_text);
    let rebuilt_params = ParameterCollection::from_string(&rebuilt_text);

    // Collect all keys from both collections
    let mut all_keys: BTreeSet<String> = BTreeSet::new();
    for (k, _) in orig_params.iter() {
        all_keys.insert(k.to_string());
    }
    for (k, _) in rebuilt_params.iter() {
        all_keys.insert(k.to_string());
    }

    let mut diffs = Vec::new();
    for key in &all_keys {
        let orig_val = orig_params.get(key).map(|v| v.as_str().to_string());
        let rebuilt_val = rebuilt_params.get(key).map(|v| v.as_str().to_string());
        if orig_val != rebuilt_val {
            diffs.push((key.clone(), orig_val, rebuilt_val));
        }
    }

    if diffs.is_empty() {
        report.matched.push(path.to_string());
    } else {
        report.text_diffs.push(TextStreamDiff {
            stream_name: path.to_string(),
            param_diffs: diffs,
        });
    }
}

/// Compare two binary streams using byte equality.
fn compare_binary_streams(
    path: &str,
    orig_data: &[u8],
    rebuilt_data: &[u8],
    report: &mut CfbDiffReport,
) {
    if orig_data == rebuilt_data {
        report.matched.push(path.to_string());
        return;
    }

    let first_diff = orig_data
        .iter()
        .zip(rebuilt_data.iter())
        .position(|(a, b)| a != b);

    report.binary_diffs.push(BinaryStreamDiff {
        stream_name: path.to_string(),
        original_len: orig_data.len(),
        rebuilt_len: rebuilt_data.len(),
        first_diff_offset: first_diff,
    });
}

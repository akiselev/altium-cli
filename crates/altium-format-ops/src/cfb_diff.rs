// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! CFB stream-level comparison utilities.
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

use serde::{Deserialize, Serialize};

/// Report of differences between two CFB files.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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
            writeln!(
                f,
                "\n--- Only in original ({}) ---",
                self.only_in_original.len()
            )?;
            for s in &self.only_in_original {
                writeln!(f, "  {}", s)?;
            }
        }
        if !self.only_in_rebuilt.is_empty() {
            writeln!(
                f,
                "\n--- Only in rebuilt ({}) ---",
                self.only_in_rebuilt.len()
            )?;
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
            writeln!(
                f,
                "\n--- Binary stream diffs ({}) ---",
                self.binary_diffs.len()
            )?;
            for diff in &self.binary_diffs {
                writeln!(
                    f,
                    "  Stream: {} (orig {} bytes, rebuilt {} bytes, first diff at byte {})",
                    diff.stream_name,
                    diff.original_len,
                    diff.rebuilt_len,
                    diff.first_diff_offset
                        .map_or("N/A".to_string(), |o| o.to_string()),
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStreamDiff {
    /// CFB stream path.
    pub stream_name: String,
    /// Parameter differences: (key, original_value, rebuilt_value).
    /// `None` means the key was missing from that side.
    pub param_diffs: Vec<(String, Option<String>, Option<String>)>,
}

/// Differences in a binary stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Check if a stream path is a SchLib Data stream (e.g. `/ComponentName/Data`).
fn is_schlib_data_stream(path: &str, all_paths: &BTreeSet<String>) -> bool {
    let trimmed = path.trim_start_matches('/');
    // Must be exactly `<something>/Data`
    let Some((storage, leaf)) = trimmed.rsplit_once('/') else {
        return false;
    };
    if leaf != "Data" || storage.is_empty() {
        return false;
    }

    // PcbLib footprints also use `/.../Data`, but are identified by sibling
    // `Header` / `Parameters` streams in the same storage. Those must use PCB
    // binary Data parsing, not SchLib text-record parsing.
    let storage_path = format!("/{}", storage);
    let header = format!("{}/Header", storage_path);
    let parameters = format!("{}/Parameters", storage_path);
    if all_paths.contains(&header) || all_paths.contains(&parameters) {
        return false;
    }

    true
}

/// Parse raw pipe-delimited params preserving duplicate key occurrences.
///
/// Keys are normalized to uppercase to match Altium's case-insensitive lookup
/// behavior. Values are kept in encounter order.
fn parse_params_with_duplicates(text: &str) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in text.split('|').filter(|s| !s.is_empty()) {
        let entry = entry.trim_end_matches(&['\r', '\n'] as &[char]);
        let (k, v) = if let Some(eq) = entry.find('=') {
            (&entry[..eq], &entry[eq + 1..])
        } else {
            ("", entry)
        };
        out.entry(k.to_uppercase()).or_default().push(v.to_string());
    }
    out
}

/// Size flag mask: low 24 bits = length, upper bits = mode flag.
const SIZE_FLAG_MASK: u32 = 0x00FF_FFFF;

/// A parsed record from a SchLib Data stream.
#[derive(Debug)]
struct SchLibRecord {
    /// The raw length prefix (including mode flag).
    size_raw: u32,
    /// The record data bytes.
    data: Vec<u8>,
}

/// Parse a SchLib Data stream into individual records.
fn parse_schlib_data_records(data: &[u8]) -> Vec<SchLibRecord> {
    let mut records = Vec::new();
    let mut pos = 0;
    while pos + 4 <= data.len() {
        let size_raw = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
        let record_len = (size_raw & SIZE_FLAG_MASK) as usize;
        pos += 4;
        if record_len == 0 || pos + record_len > data.len() {
            break;
        }
        records.push(SchLibRecord {
            size_raw,
            data: data[pos..pos + record_len].to_vec(),
        });
        pos += record_len;
    }
    records
}

/// Compare two SchLib Data streams record-by-record.
fn compare_schlib_data_streams(
    path: &str,
    orig_data: &[u8],
    rebuilt_data: &[u8],
    report: &mut CfbDiffReport,
) {
    let orig_records = parse_schlib_data_records(orig_data);
    let rebuilt_records = parse_schlib_data_records(rebuilt_data);

    let mut diffs = Vec::new();

    // Compare record counts
    if orig_records.len() != rebuilt_records.len() {
        diffs.push((
            "RecordCount".to_string(),
            Some(orig_records.len().to_string()),
            Some(rebuilt_records.len().to_string()),
        ));
    }

    // Compare each record
    let max_len = orig_records.len().max(rebuilt_records.len());
    for i in 0..max_len {
        match (orig_records.get(i), rebuilt_records.get(i)) {
            (Some(orig), Some(rebuilt)) => {
                let is_binary_orig = (orig.size_raw & !SIZE_FLAG_MASK) != 0;
                let is_binary_rebuilt = (rebuilt.size_raw & !SIZE_FLAG_MASK) != 0;

                if is_binary_orig != is_binary_rebuilt {
                    diffs.push((
                        format!("Record[{}].mode", i),
                        Some(if is_binary_orig { "binary" } else { "text" }.to_string()),
                        Some(if is_binary_rebuilt { "binary" } else { "text" }.to_string()),
                    ));
                } else if is_binary_orig {
                    // Binary record: byte-level comparison
                    if orig.data != rebuilt.data {
                        let first_diff = orig
                            .data
                            .iter()
                            .zip(rebuilt.data.iter())
                            .position(|(a, b)| a != b);
                        diffs.push((
                            format!("Record[{}].binary", i),
                            Some(format!("{} bytes", orig.data.len())),
                            Some(format!(
                                "{} bytes, first_diff={:?}",
                                rebuilt.data.len(),
                                first_diff
                            )),
                        ));
                    }
                } else {
                    // Text record: parameter-level comparison
                    let orig_text = String::from_utf8_lossy(&orig.data);
                    let rebuilt_text = String::from_utf8_lossy(&rebuilt.data);
                    let orig_params = parse_params_with_duplicates(&orig_text);
                    let rebuilt_params = parse_params_with_duplicates(&rebuilt_text);

                    let mut all_keys: BTreeSet<String> = BTreeSet::new();
                    all_keys.extend(orig_params.keys().cloned());
                    all_keys.extend(rebuilt_params.keys().cloned());

                    for key in &all_keys {
                        let orig_val = orig_params.get(key).cloned();
                        let rebuilt_val = rebuilt_params.get(key).cloned();
                        if orig_val != rebuilt_val {
                            diffs.push((
                                format!("Record[{}].{}", i, key),
                                orig_val.map(|vals| vals.join("||")),
                                rebuilt_val.map(|vals| vals.join("||")),
                            ));
                        }
                    }
                }
            }
            (Some(_), None) => {
                diffs.push((format!("Record[{}]", i), Some("present".to_string()), None));
            }
            (None, Some(_)) => {
                diffs.push((format!("Record[{}]", i), None, Some("present".to_string())));
            }
            (None, None) => unreachable!(),
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

    for path in &all_paths {
        match (orig_streams.get(path), rebuilt_streams.get(path)) {
            (Some(orig_data), Some(rebuilt_data)) => {
                if is_schlib_data_stream(&path, &all_paths) {
                    compare_schlib_data_streams(&path, orig_data, rebuilt_data, &mut report);
                } else if is_text_stream(orig_data) && is_text_stream(rebuilt_data) {
                    compare_text_streams(&path, orig_data, rebuilt_data, &mut report);
                } else {
                    compare_binary_streams(&path, orig_data, rebuilt_data, &mut report);
                }
            }
            (Some(_), None) => {
                report.only_in_original.push(path.clone());
            }
            (None, Some(_)) => {
                report.only_in_rebuilt.push(path.clone());
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

    let orig_params = parse_params_with_duplicates(&orig_text);
    let rebuilt_params = parse_params_with_duplicates(&rebuilt_text);

    // Collect all keys from both collections
    let mut all_keys: BTreeSet<String> = BTreeSet::new();
    all_keys.extend(orig_params.keys().cloned());
    all_keys.extend(rebuilt_params.keys().cloned());

    let mut diffs = Vec::new();
    for key in &all_keys {
        let orig_val = orig_params.get(key).cloned();
        let rebuilt_val = rebuilt_params.get(key).cloned();
        if orig_val != rebuilt_val {
            diffs.push((
                key.clone(),
                orig_val.map(|vals| vals.join("||")),
                rebuilt_val.map(|vals| vals.join("||")),
            ));
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

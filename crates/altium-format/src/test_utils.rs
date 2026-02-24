use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use altium_format_types::constants::parsing::{C_SCH_BROKEN_BAR, C_SCH_UTF8_PREFIX};

use crate::block_stream::{Block, BlockFormat, parse_blocks};
use crate::embedded_object::parse_embedded_object;
use crate::{AltiumFormatError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfbSemanticDiffReport {
    pub file_a: PathBuf,
    pub file_b: PathBuf,
    pub issues: Vec<DiffIssue>,
}

impl CfbSemanticDiffReport {
    pub fn is_identical(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "CFB semantic diff: {} vs {}",
            self.file_a.display(),
            self.file_b.display()
        );
        if self.issues.is_empty() {
            let _ = writeln!(out, "no semantic differences");
            return out;
        }
        let _ = writeln!(out, "differences: {}", self.issues.len());
        for (idx, issue) in self.issues.iter().enumerate() {
            let _ = writeln!(out, "[{}] {}", idx + 1, issue.render());
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffIssue {
    EntryMissingInA {
        path: String,
        kind: &'static str,
    },
    EntryMissingInB {
        path: String,
        kind: &'static str,
    },
    EntryKindMismatch {
        path: String,
        kind_a: &'static str,
        kind_b: &'static str,
    },
    StreamLengthMismatch {
        path: String,
        len_a: usize,
        len_b: usize,
    },
    RawByteMismatch {
        path: String,
        offset: usize,
        byte_a: Option<u8>,
        byte_b: Option<u8>,
    },
    BlockParseError {
        path: String,
        side: &'static str,
        error: String,
    },
    BlockCountMismatch {
        path: String,
        count_a: usize,
        count_b: usize,
    },
    BlockTypeMismatch {
        path: String,
        block_index: usize,
        kind_a: &'static str,
        kind_b: &'static str,
    },
    BlockLengthMismatch {
        path: String,
        block_index: usize,
        len_a: usize,
        len_b: usize,
        kind: &'static str,
    },
    TextParamParseError {
        path: String,
        block_index: usize,
        side: &'static str,
        detail: String,
    },
    MissingParamPair {
        path: String,
        block_index: usize,
        side: &'static str,
        key: String,
        value: String,
    },
    DuplicateParamPairCountMismatch {
        path: String,
        block_index: usize,
        key: String,
        value: String,
        count_a: usize,
        count_b: usize,
    },
    UpdatedParamValues {
        path: String,
        block_index: usize,
        key: String,
        values_a: Vec<String>,
        values_b: Vec<String>,
    },
    BinaryBlockMismatch {
        path: String,
        block_index: usize,
        offset: usize,
        byte_a: Option<u8>,
        byte_b: Option<u8>,
    },
    EmbeddedObjectIdMismatch {
        path: String,
        block_index: usize,
        id_a: String,
        id_b: String,
    },
    EmbeddedObjectDataMismatch {
        path: String,
        block_index: usize,
        offset: usize,
        len_a: usize,
        len_b: usize,
        byte_a: Option<u8>,
        byte_b: Option<u8>,
    },
}

impl DiffIssue {
    fn render(&self) -> String {
        match self {
            Self::EntryMissingInA { path, kind } => {
                format!("missing in file A: {kind} {path}")
            }
            Self::EntryMissingInB { path, kind } => {
                format!("missing in file B: {kind} {path}")
            }
            Self::EntryKindMismatch {
                path,
                kind_a,
                kind_b,
            } => {
                format!("entry kind mismatch at {path}: A={kind_a}, B={kind_b}")
            }
            Self::StreamLengthMismatch { path, len_a, len_b } => {
                format!("stream length mismatch at {path}: A={len_a}, B={len_b}")
            }
            Self::RawByteMismatch {
                path,
                offset,
                byte_a,
                byte_b,
            } => {
                format!(
                    "raw byte mismatch at {path} offset {offset}: A={}, B={}",
                    fmt_opt_byte(*byte_a),
                    fmt_opt_byte(*byte_b)
                )
            }
            Self::BlockParseError { path, side, error } => {
                format!("block parse error at {path} ({side}): {error}")
            }
            Self::BlockCountMismatch {
                path,
                count_a,
                count_b,
            } => {
                format!("block count mismatch at {path}: A={count_a}, B={count_b}")
            }
            Self::BlockTypeMismatch {
                path,
                block_index,
                kind_a,
                kind_b,
            } => {
                format!("block type mismatch at {path}#{block_index}: A={kind_a}, B={kind_b}")
            }
            Self::BlockLengthMismatch {
                path,
                block_index,
                len_a,
                len_b,
                kind,
            } => {
                format!(
                    "{kind} block length mismatch at {path}#{block_index}: A={len_a}, B={len_b}"
                )
            }
            Self::TextParamParseError {
                path,
                block_index,
                side,
                detail,
            } => {
                format!("text param parse error at {path}#{block_index} ({side}): {detail}")
            }
            Self::MissingParamPair {
                path,
                block_index,
                side,
                key,
                value,
            } => {
                format!("param pair missing in {side} at {path}#{block_index}: {key}={value}")
            }
            Self::DuplicateParamPairCountMismatch {
                path,
                block_index,
                key,
                value,
                count_a,
                count_b,
            } => {
                format!(
                    "duplicate param count differs at {path}#{block_index}: {key}={value} (A={count_a}, B={count_b})"
                )
            }
            Self::UpdatedParamValues {
                path,
                block_index,
                key,
                values_a,
                values_b,
            } => {
                format!(
                    "param values differ at {path}#{block_index} for key {key}: A={values_a:?}, B={values_b:?}"
                )
            }
            Self::BinaryBlockMismatch {
                path,
                block_index,
                offset,
                byte_a,
                byte_b,
            } => {
                format!(
                    "binary block mismatch at {path}#{block_index} offset {offset}: A={}, B={}",
                    fmt_opt_byte(*byte_a),
                    fmt_opt_byte(*byte_b)
                )
            }
            Self::EmbeddedObjectIdMismatch {
                path,
                block_index,
                id_a,
                id_b,
            } => {
                format!("embedded object id mismatch at {path}#{block_index}: A={id_a}, B={id_b}")
            }
            Self::EmbeddedObjectDataMismatch {
                path,
                block_index,
                offset,
                len_a,
                len_b,
                byte_a,
                byte_b,
            } => {
                format!(
                    "embedded object data mismatch at {path}#{block_index}: first diff at {offset}, len A={len_a}, len B={len_b}, A={}, B={}",
                    fmt_opt_byte(*byte_a),
                    fmt_opt_byte(*byte_b)
                )
            }
        }
    }
}

fn fmt_opt_byte(v: Option<u8>) -> String {
    match v {
        Some(b) => format!("{b:#04x}"),
        None => "EOF".to_owned(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    Storage,
    Stream,
}

impl EntryKind {
    fn label(self) -> &'static str {
        match self {
            Self::Storage => "storage",
            Self::Stream => "stream",
        }
    }
}

pub fn diff_cfb_files_semantic(path_a: &Path, path_b: &Path) -> Result<CfbSemanticDiffReport> {
    let mut cfb_a = open_cfb(path_a)?;
    let mut cfb_b = open_cfb(path_b)?;

    let entries_a = collect_entries(&mut cfb_a);
    let entries_b = collect_entries(&mut cfb_b);
    let all_paths: BTreeSet<String> = entries_a.keys().chain(entries_b.keys()).cloned().collect();

    let mut issues = Vec::new();

    for path in all_paths {
        match (entries_a.get(&path), entries_b.get(&path)) {
            (Some(kind_a), None) => issues.push(DiffIssue::EntryMissingInB {
                path,
                kind: kind_a.label(),
            }),
            (None, Some(kind_b)) => issues.push(DiffIssue::EntryMissingInA {
                path,
                kind: kind_b.label(),
            }),
            (Some(kind_a), Some(kind_b)) => {
                if kind_a != kind_b {
                    issues.push(DiffIssue::EntryKindMismatch {
                        path,
                        kind_a: kind_a.label(),
                        kind_b: kind_b.label(),
                    });
                    continue;
                }

                if *kind_a == EntryKind::Storage {
                    continue;
                }

                let stream_a = read_stream(&mut cfb_a, &path)?;
                let stream_b = read_stream(&mut cfb_b, &path)?;
                compare_stream(&path, &stream_a, &stream_b, &mut issues);
            }
            (None, None) => {}
        }
    }

    Ok(CfbSemanticDiffReport {
        file_a: path_a.to_path_buf(),
        file_b: path_b.to_path_buf(),
        issues,
    })
}

pub fn assert_cfb_files_semantic_eq(path_a: &Path, path_b: &Path) {
    match diff_cfb_files_semantic(path_a, path_b) {
        Ok(report) => {
            assert!(report.is_identical(), "{}", report.render());
        }
        Err(err) => panic!(
            "failed to run semantic CFB diff for {} vs {}: {err}",
            path_a.display(),
            path_b.display()
        ),
    }
}

fn compare_stream(path: &str, stream_a: &[u8], stream_b: &[u8], issues: &mut Vec<DiffIssue>) {
    match (parse_blocks(stream_a), parse_blocks(stream_b)) {
        (Ok(blocks_a), Ok(blocks_b)) => {
            compare_blocks(path, &blocks_a, &blocks_b, issues);
            return;
        }
        (Err(err_a), Ok(_)) => issues.push(DiffIssue::BlockParseError {
            path: path.to_owned(),
            side: "A",
            error: err_a.to_string(),
        }),
        (Ok(_), Err(err_b)) => issues.push(DiffIssue::BlockParseError {
            path: path.to_owned(),
            side: "B",
            error: err_b.to_string(),
        }),
        (Err(err_a), Err(err_b)) => {
            issues.push(DiffIssue::BlockParseError {
                path: path.to_owned(),
                side: "A",
                error: err_a.to_string(),
            });
            issues.push(DiffIssue::BlockParseError {
                path: path.to_owned(),
                side: "B",
                error: err_b.to_string(),
            });
        }
    }

    if stream_a == stream_b {
        return;
    }
    if stream_a.len() != stream_b.len() {
        issues.push(DiffIssue::StreamLengthMismatch {
            path: path.to_owned(),
            len_a: stream_a.len(),
            len_b: stream_b.len(),
        });
    }
    let (offset, byte_a, byte_b) = first_byte_diff(stream_a, stream_b);
    issues.push(DiffIssue::RawByteMismatch {
        path: path.to_owned(),
        offset,
        byte_a,
        byte_b,
    });
}

fn compare_blocks(path: &str, blocks_a: &[Block], blocks_b: &[Block], issues: &mut Vec<DiffIssue>) {
    if blocks_a.len() != blocks_b.len() {
        issues.push(DiffIssue::BlockCountMismatch {
            path: path.to_owned(),
            count_a: blocks_a.len(),
            count_b: blocks_b.len(),
        });
    }

    for idx in 0..blocks_a.len().max(blocks_b.len()) {
        let (Some(block_a), Some(block_b)) = (blocks_a.get(idx), blocks_b.get(idx)) else {
            continue;
        };

        if block_a.format != block_b.format {
            issues.push(DiffIssue::BlockTypeMismatch {
                path: path.to_owned(),
                block_index: idx,
                kind_a: block_label(block_a.format),
                kind_b: block_label(block_b.format),
            });
            continue;
        }

        match block_a.format {
            BlockFormat::Text => {
                compare_text_block(path, idx, &block_a.data, &block_b.data, issues)
            }
            BlockFormat::Binary => {
                compare_binary_block(path, idx, &block_a.data, &block_b.data, issues)
            }
        }
    }
}

fn compare_text_block(
    path: &str,
    block_index: usize,
    data_a: &[u8],
    data_b: &[u8],
    issues: &mut Vec<DiffIssue>,
) {
    let params_a = match parse_param_pairs(data_a) {
        Ok(v) => v,
        Err(detail) => {
            issues.push(DiffIssue::TextParamParseError {
                path: path.to_owned(),
                block_index,
                side: "A",
                detail,
            });
            return;
        }
    };
    let params_b = match parse_param_pairs(data_b) {
        Ok(v) => v,
        Err(detail) => {
            issues.push(DiffIssue::TextParamParseError {
                path: path.to_owned(),
                block_index,
                side: "B",
                detail,
            });
            return;
        }
    };

    let pair_counts_a = pair_counts(&params_a);
    let pair_counts_b = pair_counts(&params_b);

    let pairs_a: BTreeSet<(String, String)> = pair_counts_a.keys().cloned().collect();
    let pairs_b: BTreeSet<(String, String)> = pair_counts_b.keys().cloned().collect();

    for (key, value) in pairs_a.difference(&pairs_b) {
        issues.push(DiffIssue::MissingParamPair {
            path: path.to_owned(),
            block_index,
            side: "B",
            key: key.clone(),
            value: value.clone(),
        });
    }
    for (key, value) in pairs_b.difference(&pairs_a) {
        issues.push(DiffIssue::MissingParamPair {
            path: path.to_owned(),
            block_index,
            side: "A",
            key: key.clone(),
            value: value.clone(),
        });
    }

    let mut shared_keys = BTreeSet::new();
    for (key, _) in pairs_a.intersection(&pairs_b) {
        shared_keys.insert(key.clone());
    }

    for key in shared_keys {
        let mut values_a: BTreeSet<String> = BTreeSet::new();
        let mut values_b: BTreeSet<String> = BTreeSet::new();
        for (k, v) in &pairs_a {
            if k == &key {
                values_a.insert(v.clone());
            }
        }
        for (k, v) in &pairs_b {
            if k == &key {
                values_b.insert(v.clone());
            }
        }
        if values_a != values_b {
            issues.push(DiffIssue::UpdatedParamValues {
                path: path.to_owned(),
                block_index,
                key,
                values_a: values_a.into_iter().collect(),
                values_b: values_b.into_iter().collect(),
            });
        }
    }

    let shared_pairs: BTreeSet<(String, String)> =
        pairs_a.intersection(&pairs_b).cloned().collect();
    for (key, value) in shared_pairs {
        let count_a = *pair_counts_a
            .get(&(key.clone(), value.clone()))
            .unwrap_or(&0);
        let count_b = *pair_counts_b
            .get(&(key.clone(), value.clone()))
            .unwrap_or(&0);
        if count_a != count_b && count_a > 1 && count_b > 1 {
            issues.push(DiffIssue::DuplicateParamPairCountMismatch {
                path: path.to_owned(),
                block_index,
                key,
                value,
                count_a,
                count_b,
            });
        }
    }
}

fn compare_binary_block(
    path: &str,
    block_index: usize,
    data_a: &[u8],
    data_b: &[u8],
    issues: &mut Vec<DiffIssue>,
) {
    if data_a.len() != data_b.len() {
        issues.push(DiffIssue::BlockLengthMismatch {
            path: path.to_owned(),
            block_index,
            len_a: data_a.len(),
            len_b: data_b.len(),
            kind: "binary",
        });
    }

    match (parse_embedded_object(data_a), parse_embedded_object(data_b)) {
        (Ok(obj_a), Ok(obj_b)) => {
            if obj_a.id != obj_b.id {
                issues.push(DiffIssue::EmbeddedObjectIdMismatch {
                    path: path.to_owned(),
                    block_index,
                    id_a: obj_a.id,
                    id_b: obj_b.id,
                });
                return;
            }
            if obj_a.inner_data != obj_b.inner_data {
                let (offset, byte_a, byte_b) =
                    first_byte_diff(&obj_a.inner_data, &obj_b.inner_data);
                issues.push(DiffIssue::EmbeddedObjectDataMismatch {
                    path: path.to_owned(),
                    block_index,
                    offset,
                    len_a: obj_a.inner_data.len(),
                    len_b: obj_b.inner_data.len(),
                    byte_a,
                    byte_b,
                });
            }
            return;
        }
        (Err(_), Err(_)) => {}
        (Err(_), Ok(_)) | (Ok(_), Err(_)) => {
            let (offset, byte_a, byte_b) = first_byte_diff(data_a, data_b);
            issues.push(DiffIssue::BinaryBlockMismatch {
                path: path.to_owned(),
                block_index,
                offset,
                byte_a,
                byte_b,
            });
            return;
        }
    }

    if data_a != data_b {
        let (offset, byte_a, byte_b) = first_byte_diff(data_a, data_b);
        issues.push(DiffIssue::BinaryBlockMismatch {
            path: path.to_owned(),
            block_index,
            offset,
            byte_a,
            byte_b,
        });
    }
}

fn block_label(format: BlockFormat) -> &'static str {
    match format {
        BlockFormat::Text => "text",
        BlockFormat::Binary => "binary",
    }
}

fn pair_counts(params: &[(String, String)]) -> BTreeMap<(String, String), usize> {
    let mut out = BTreeMap::new();
    for (k, v) in params {
        *out.entry((k.clone(), v.clone())).or_insert(0) += 1;
    }
    out
}

fn parse_param_pairs(data: &[u8]) -> std::result::Result<Vec<(String, String)>, String> {
    let data = data.strip_suffix(b"\0").unwrap_or(data);
    let mut out = Vec::new();
    for segment in data.split(|&b| b == b'|') {
        if segment.is_empty() {
            continue;
        }
        let Some(eq_pos) = segment.iter().position(|&b| b == b'=') else {
            let (raw, _) = encoding_rs::WINDOWS_1252.decode_without_bom_handling(segment);
            return Err(format!("parameter segment has no '=': '{}'", raw));
        };
        let raw_key = &segment[..eq_pos];
        let raw_value = &segment[eq_pos + 1..];
        let (key_decoded, _) = encoding_rs::WINDOWS_1252.decode_without_bom_handling(raw_key);
        let key_decoded = key_decoded.into_owned();
        if let Some(stripped) = key_decoded.strip_prefix(C_SCH_UTF8_PREFIX) {
            let utf8 = std::str::from_utf8(raw_value)
                .map_err(|e| format!("UTF-8 decode error for key '{stripped}': {e}"))?;
            out.push((stripped.to_owned(), unescape_param_value(utf8)));
            continue;
        }
        let (value_decoded, _) = encoding_rs::WINDOWS_1252.decode_without_bom_handling(raw_value);
        out.push((key_decoded, unescape_param_value(&value_decoded)));
    }
    Ok(out)
}

fn unescape_param_value(s: &str) -> String {
    let s = s.replace("\u{017D}\u{017D}", "\x00");
    let s = s.replace('\u{017D}', "|");
    let s = s.replace('\x00', "\u{017D}");
    s.replace(C_SCH_BROKEN_BAR, "|")
}

fn first_byte_diff(a: &[u8], b: &[u8]) -> (usize, Option<u8>, Option<u8>) {
    let idx = a
        .iter()
        .zip(b.iter())
        .position(|(x, y)| x != y)
        .unwrap_or_else(|| a.len().min(b.len()));
    (idx, a.get(idx).copied(), b.get(idx).copied())
}

fn open_cfb(path: &Path) -> Result<cfb::CompoundFile<std::io::Cursor<Vec<u8>>>> {
    let bytes = std::fs::read(path)?;
    cfb::CompoundFile::open(std::io::Cursor::new(bytes))
        .map_err(|e| AltiumFormatError::CfbError(e.to_string()))
}

fn collect_entries<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
) -> BTreeMap<String, EntryKind> {
    let mut entries = BTreeMap::new();
    for entry in comp.walk() {
        if entry.is_root() {
            continue;
        }
        let path = entry.path().to_string_lossy().into_owned();
        let kind = if entry.is_storage() {
            EntryKind::Storage
        } else {
            EntryKind::Stream
        };
        entries.insert(path, kind);
    }
    entries
}

fn read_stream<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
    path: &str,
) -> Result<Vec<u8>> {
    let mut stream = comp
        .open_stream(Path::new(path))
        .map_err(|e| AltiumFormatError::CfbError(e.to_string()))?;
    let mut out = Vec::new();
    stream.read_to_end(&mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    use flate2::Compression;
    use flate2::write::ZlibEncoder;

    use crate::block_stream::{write_binary_block, write_text_block};
    use crate::embedded_object::serialize_embedded_object;

    fn make_cfb(streams: &[(&str, Vec<u8>)]) -> tempfile::NamedTempFile {
        make_cfb_layout(streams, &[])
    }

    fn make_cfb_layout(streams: &[(&str, Vec<u8>)], storages: &[&str]) -> tempfile::NamedTempFile {
        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        let cursor = std::io::Cursor::new(Vec::<u8>::new());
        let mut cfb = cfb::CompoundFile::create(cursor).expect("create cfb");
        for path in storages {
            cfb.create_storage(Path::new(path))
                .expect("create storage in cfb");
        }
        for (path, data) in streams {
            let mut stream = cfb
                .create_stream(Path::new(path))
                .expect("create stream in cfb");
            stream.write_all(data).expect("write stream data");
            stream.flush().expect("flush stream");
        }
        cfb.flush().expect("flush cfb");
        let bytes = cfb.into_inner().into_inner();
        std::fs::write(tmp.path(), bytes).expect("write cfb file");
        tmp
    }

    fn envelope_with_level(id: &str, payload: &[u8], level: Compression) -> Vec<u8> {
        let mut z = ZlibEncoder::new(Vec::new(), level);
        z.write_all(payload).expect("compress payload");
        let compressed = z.finish().expect("finish zlib stream");

        let mut out = Vec::new();
        out.push(0xD0);
        out.push(id.len() as u8);
        out.extend_from_slice(id.as_bytes());
        out.extend_from_slice(&(compressed.len() as i32).to_le_bytes());
        out.extend_from_slice(&compressed);
        out
    }

    #[test]
    fn order_agnostic_param_block_compare() {
        let a = make_cfb(&[("/FileHeader", write_text_block(b"|A=1|B=2|C=3|\0"))]);
        let b = make_cfb(&[("/FileHeader", write_text_block(b"|C=3|A=1|B=2|\0"))]);

        let report = diff_cfb_files_semantic(a.path(), b.path()).expect("semantic diff");
        assert!(report.is_identical(), "{}", report.render());
    }

    #[test]
    fn duplicate_param_pairs_are_tolerated() {
        let a = make_cfb(&[("/FileHeader", write_text_block(b"|A=1|A=1|B=2|\0"))]);
        let b = make_cfb(&[("/FileHeader", write_text_block(b"|B=2|A=1|\0"))]);

        let report = diff_cfb_files_semantic(a.path(), b.path()).expect("semantic diff");
        assert!(report.is_identical(), "{}", report.render());
    }

    #[test]
    fn missing_param_pair_is_reported() {
        let a = make_cfb(&[("/FileHeader", write_text_block(b"|A=1|B=2|\0"))]);
        let b = make_cfb(&[("/FileHeader", write_text_block(b"|A=1|\0"))]);

        let report = diff_cfb_files_semantic(a.path(), b.path()).expect("semantic diff");
        assert!(
            report
                .issues
                .iter()
                .any(|i| matches!(i, DiffIssue::MissingParamPair { .. })),
            "{}",
            report.render()
        );
    }

    #[test]
    fn mixed_stream_preserves_binary_strictness() {
        let mut a_stream = write_text_block(b"|A=1|\0");
        a_stream.extend_from_slice(&write_binary_block(&[0x10, 0x20, 0x30]));

        let mut b_stream = write_text_block(b"|A=1|\0");
        b_stream.extend_from_slice(&write_binary_block(&[0x10, 0x21, 0x30]));

        let a = make_cfb(&[("/FileHeader", a_stream)]);
        let b = make_cfb(&[("/FileHeader", b_stream)]);

        let report = diff_cfb_files_semantic(a.path(), b.path()).expect("semantic diff");
        assert!(
            report
                .issues
                .iter()
                .any(|i| matches!(i, DiffIssue::BinaryBlockMismatch { .. })),
            "{}",
            report.render()
        );
    }

    #[test]
    fn compressed_embedded_object_uses_decompressed_compare() {
        let payload = b"same semantic payload";
        let a_env = envelope_with_level("obj", payload, Compression::fast());
        let b_env = envelope_with_level("obj", payload, Compression::best());

        let mut a_stream = write_text_block(b"|HEADER=Storage|Weight=1|\0");
        a_stream.extend_from_slice(&write_binary_block(&a_env));

        let mut b_stream = write_text_block(b"|Weight=1|HEADER=Storage|\0");
        b_stream.extend_from_slice(&write_binary_block(&b_env));

        let a = make_cfb(&[("/Storage", a_stream)]);
        let b = make_cfb(&[("/Storage", b_stream)]);

        let report = diff_cfb_files_semantic(a.path(), b.path()).expect("semantic diff");
        assert!(report.is_identical(), "{}", report.render());
    }

    #[test]
    fn embedded_object_data_mismatch_is_reported() {
        let a_env = serialize_embedded_object("obj", b"abc").expect("serialize obj A");
        let b_env = serialize_embedded_object("obj", b"abz").expect("serialize obj B");

        let mut a_stream = write_text_block(b"|HEADER=Storage|Weight=1|\0");
        a_stream.extend_from_slice(&write_binary_block(&a_env));

        let mut b_stream = write_text_block(b"|HEADER=Storage|Weight=1|\0");
        b_stream.extend_from_slice(&write_binary_block(&b_env));

        let a = make_cfb(&[("/Storage", a_stream)]);
        let b = make_cfb(&[("/Storage", b_stream)]);

        let report = diff_cfb_files_semantic(a.path(), b.path()).expect("semantic diff");
        assert!(
            report
                .issues
                .iter()
                .any(|i| matches!(i, DiffIssue::EmbeddedObjectDataMismatch { .. })),
            "{}",
            report.render()
        );
    }

    #[test]
    fn missing_entries_are_reported() {
        let a = make_cfb(&[("/OnlyInA", write_text_block(b"|A=1|\0"))]);
        let b = make_cfb(&[("/OnlyInB", write_text_block(b"|B=2|\0"))]);

        let report = diff_cfb_files_semantic(a.path(), b.path()).expect("semantic diff");
        assert!(
            report
                .issues
                .iter()
                .any(|i| matches!(i, DiffIssue::EntryMissingInA { .. })),
            "{}",
            report.render()
        );
        assert!(
            report
                .issues
                .iter()
                .any(|i| matches!(i, DiffIssue::EntryMissingInB { .. })),
            "{}",
            report.render()
        );
    }

    #[test]
    fn entry_kind_mismatch_is_reported() {
        let a = make_cfb_layout(&[("/X", write_text_block(b"|A=1|\0"))], &[]);
        let b = make_cfb_layout(&[("/X/Child", write_text_block(b"|A=1|\0"))], &["/X"]);

        let report = diff_cfb_files_semantic(a.path(), b.path()).expect("semantic diff");
        assert!(
            report
                .issues
                .iter()
                .any(|i| matches!(i, DiffIssue::EntryKindMismatch { .. })),
            "{}",
            report.render()
        );
    }

    #[test]
    fn block_count_mismatch_is_reported() {
        let mut a_stream = write_text_block(b"|A=1|\0");
        a_stream.extend_from_slice(&write_text_block(b"|B=2|\0"));
        let b_stream = write_text_block(b"|A=1|\0");

        let a = make_cfb(&[("/FileHeader", a_stream)]);
        let b = make_cfb(&[("/FileHeader", b_stream)]);

        let report = diff_cfb_files_semantic(a.path(), b.path()).expect("semantic diff");
        assert!(
            report
                .issues
                .iter()
                .any(|i| matches!(i, DiffIssue::BlockCountMismatch { .. })),
            "{}",
            report.render()
        );
    }

    #[test]
    fn block_type_mismatch_is_reported() {
        let a = make_cfb(&[("/FileHeader", write_text_block(b"|A=1|\0"))]);
        let b = make_cfb(&[("/FileHeader", write_binary_block(&[0xAA, 0xBB]))]);

        let report = diff_cfb_files_semantic(a.path(), b.path()).expect("semantic diff");
        assert!(
            report
                .issues
                .iter()
                .any(|i| matches!(i, DiffIssue::BlockTypeMismatch { .. })),
            "{}",
            report.render()
        );
    }

    #[test]
    fn block_parse_error_is_reported() {
        let a = make_cfb(&[("/Bad", vec![0x01, 0x02, 0x03])]);
        let b = make_cfb(&[("/Bad", write_text_block(b"|A=1|\0"))]);

        let report = diff_cfb_files_semantic(a.path(), b.path()).expect("semantic diff");
        assert!(
            report
                .issues
                .iter()
                .any(|i| matches!(i, DiffIssue::BlockParseError { .. })),
            "{}",
            report.render()
        );
    }

    #[test]
    fn utf8_prefixed_param_equivalence() {
        let a = make_cfb(&[("/FileHeader", write_text_block(b"|%UTF8%NAME=hello||\0"))]);
        let b = make_cfb(&[("/FileHeader", write_text_block(b"|NAME=hello|\0"))]);

        let report = diff_cfb_files_semantic(a.path(), b.path()).expect("semantic diff");
        assert!(report.is_identical(), "{}", report.render());
    }

    #[test]
    fn duplicate_param_pair_count_mismatch_is_reported() {
        let a = make_cfb(&[("/FileHeader", write_text_block(b"|A=1|A=1|\0"))]);
        let b = make_cfb(&[("/FileHeader", write_text_block(b"|A=1|A=1|A=1|\0"))]);

        let report = diff_cfb_files_semantic(a.path(), b.path()).expect("semantic diff");
        assert!(
            report
                .issues
                .iter()
                .any(|i| matches!(i, DiffIssue::DuplicateParamPairCountMismatch { .. })),
            "{}",
            report.render()
        );
    }
}

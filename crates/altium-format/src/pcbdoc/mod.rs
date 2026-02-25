pub(crate) mod primitives;
pub(crate) mod records;

use std::collections::HashSet;
use std::path::Path;

use altium_format_types::constants::file_headers::{
    PCB_DOC_BINARY_HEADER_V5, PCB_DOC_BINARY_HEADER_V6,
};
use altium_format_types::constants::streams::FILE_HEADER;

use crate::binary_io::BinaryReader;
use crate::pcb_binary_stream::parse_pcb_section_header;
use crate::pcb_file_header::{PcbFileHeader, parse_pcb_file_header, parse_pcb_legacy_header};
use crate::pcblib::library::{
    PcbEmbeddedFontEntry, PcbLayerKindMapping, PcbLibModelEntry, PcbPadViaLibraryConfig,
    parse_layer_kind_mapping, parse_model_metadata, parse_pad_via_library,
};
use crate::tracked_cfb::TrackedCfbDocument;
use crate::{AltiumFormatError, Result, ResultExt};

pub(crate) struct PrimitiveSectionData {
    pub(crate) kind: records::PrimitiveSectionKind,
    pub(crate) records: Vec<primitives::ParsedPrimitiveRecord>,
}

pub(crate) struct ParamSectionData {
    pub(crate) kind: records::ParamSectionKind,
    pub(crate) records: Vec<records::StandardParamRecord>,
}

pub(crate) struct PrefixedParamSectionData {
    pub(crate) kind: records::PrefixedParamSectionKind,
    pub(crate) records: Vec<records::PrefixedParamRecord>,
}

pub(crate) struct WideStringsSectionData {
    pub(crate) entries: Vec<records::WideString6Record>,
}

pub(crate) struct BinarySectionData {
    pub(crate) kind: records::BinaryLenSectionKind,
    pub(crate) records: Vec<records::BinaryLenRecord>,
}

pub(crate) struct UnionNamesSectionData {
    pub(crate) format_version: u32,
    pub(crate) records: Vec<records::UnionNameRecord>,
}

pub(crate) struct ModelsSectionData {
    pub(crate) metadata: Vec<PcbLibModelEntry>,
    pub(crate) blobs: Vec<(String, Vec<u8>)>,
}

pub(crate) struct EmbeddedFontsSectionData {
    pub(crate) header_count: u32,
    pub(crate) entries: Vec<PcbEmbeddedFontEntry>,
}

pub(crate) struct PadViaLibrarySectionData {
    pub(crate) section_name: String,
    pub(crate) config: Option<PcbPadViaLibraryConfig>,
}

pub(crate) struct LayerKindMappingSectionData {
    pub(crate) mapping: PcbLayerKindMapping,
}

pub(crate) enum PcbDocSection {
    Primitive(PrimitiveSectionData),
    Parameter(ParamSectionData),
    Binary(BinarySectionData),
    UnionNames(UnionNamesSectionData),
    PrefixedParameter(PrefixedParamSectionData),
    WideStrings(WideStringsSectionData),
    Models(ModelsSectionData),
    EmbeddedFonts(EmbeddedFontsSectionData),
    PadViaLibrary(PadViaLibrarySectionData),
    LayerKindMapping(LayerKindMappingSectionData),
}

pub struct PcbDoc {
    pub(crate) legacy_header: String,
    pub(crate) header: PcbFileHeader,
    pub(crate) sections: Vec<PcbDocSection>,
}

impl PcbDoc {
    pub fn version_header(&self) -> &str {
        &self.header.version_string
    }

    pub fn minor_version(&self) -> f64 {
        self.header.version
    }

    /// Validates strict structural invariants for a parsed PcbDoc document.
    pub fn validate_invariants(&self) -> Result<()> {
        validate_pcbdoc_invariants(self)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let mut doc = TrackedCfbDocument::open(path)?;

        let legacy_data = doc.read_stream(&format!("/{FILE_HEADER}"))?;
        let legacy_header = parse_pcb_legacy_header(&legacy_data).context("parsing /FileHeader")?;
        if !PCB_DOC_BINARY_HEADER_V5.starts_with(&legacy_header)
            || !legacy_header.starts_with("PCB ")
        {
            return Err(AltiumFormatError::InvalidParamValue {
                key: FILE_HEADER.to_owned(),
                detail: format!(
                    "expected legacy header prefix of \"{}\", got \"{}\"",
                    PCB_DOC_BINARY_HEADER_V5, legacy_header
                ),
            });
        }

        let header_six_data = doc
            .read_stream("/FileHeaderSix")
            .context("reading /FileHeaderSix")?;
        let header = parse_pcb_file_header(&header_six_data).context("parsing /FileHeaderSix")?;
        if header.version_string != PCB_DOC_BINARY_HEADER_V6 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "FileHeaderSix".to_owned(),
                detail: format!(
                    "expected v6 header \"{}\", got \"{}\"",
                    PCB_DOC_BINARY_HEADER_V6, header.version_string
                ),
            });
        }

        let (storages, _) = doc.list_entries("/")?;
        let mut sections = Vec::new();

        for storage_name in storages {
            let storage_name = storage_name.trim_start_matches('/').to_owned();
            if storage_name.is_empty() {
                continue;
            }
            if storage_name == FILE_HEADER || storage_name == "FileHeaderSix" {
                continue;
            }

            let storage_path = format!("/{storage_name}");
            if storage_name == "WideStrings6" {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data = doc.read_stream(&format!("{storage_path}/Data"))?;
                let expected_count = parse_pcb_section_header(&header_data)? as usize;
                let entries = records::parse_wide_strings6_records(&data)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                if expected_count != entries.len() {
                    return Err(AltiumFormatError::RecordCountMismatch {
                        section: storage_name.clone(),
                        expected: expected_count,
                        actual: entries.len(),
                    });
                }
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
                sections.push(PcbDocSection::WideStrings(WideStringsSectionData {
                    entries,
                }));
                continue;
            }

            if storage_name == "UnionNames" {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data = doc.read_stream(&format!("{storage_path}/Data"))?;
                let format_version = parse_pcb_section_header(&header_data)?;
                if format_version != 1 {
                    return Err(AltiumFormatError::InvalidParamValue {
                        key: "UnionNames/Header".to_owned(),
                        detail: format!("expected format version 1, got {format_version}"),
                    });
                }
                let records = records::parse_union_name_records(&data)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
                sections.push(PcbDocSection::UnionNames(UnionNamesSectionData {
                    format_version,
                    records,
                }));
                continue;
            }

            if storage_name == "Models" {
                sections.push(parse_models_storage(&mut doc, &storage_name)?);
                continue;
            }

            if storage_name == "PadViaLibrary" || storage_name == "PadViaLibraryCache" {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data = doc.read_stream(&format!("{storage_path}/Data"))?;
                let config = parse_pad_via_library(&header_data, &data)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
                sections.push(PcbDocSection::PadViaLibrary(PadViaLibrarySectionData {
                    section_name: storage_name.clone(),
                    config,
                }));
                continue;
            }

            if storage_name == "LayerKindMapping" {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data = doc.read_stream(&format!("{storage_path}/Data"))?;
                let mapping = parse_layer_kind_mapping(&header_data, &data)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
                sections.push(PcbDocSection::LayerKindMapping(
                    LayerKindMappingSectionData { mapping },
                ));
                continue;
            }

            if storage_name == "EmbeddedFonts6" {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data = doc.read_stream(&format!("{storage_path}/Data"))?;
                let header_count = parse_pcb_section_header(&header_data)?;
                let entries = parse_embedded_fonts6_data(&data, header_count as usize)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
                sections.push(PcbDocSection::EmbeddedFonts(EmbeddedFontsSectionData {
                    header_count,
                    entries,
                }));
                continue;
            }

            if let Some(kind) = records::BinaryLenSectionKind::from_storage_name(&storage_name) {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data = doc.read_stream(&format!("{storage_path}/Data"))?;
                let expected_count = parse_pcb_section_header(&header_data)? as usize;
                let records = records::parse_len_prefixed_binary_records(&data)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                if expected_count != records.len() {
                    return Err(AltiumFormatError::RecordCountMismatch {
                        section: storage_name.clone(),
                        expected: expected_count,
                        actual: records.len(),
                    });
                }
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
                sections.push(PcbDocSection::Binary(BinarySectionData { kind, records }));
                continue;
            }

            if let Some(kind) = records::PrimitiveSectionKind::from_storage_name(&storage_name) {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data = doc.read_stream(&format!("{storage_path}/Data"))?;
                let expected_count = parse_pcb_section_header(&header_data)? as usize;
                let records = primitives::parse_primitive_records(kind, &data)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                if expected_count != records.len() {
                    return Err(AltiumFormatError::RecordCountMismatch {
                        section: storage_name.clone(),
                        expected: expected_count,
                        actual: records.len(),
                    });
                }
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
                sections.push(PcbDocSection::Primitive(PrimitiveSectionData {
                    kind,
                    records,
                }));
                continue;
            }

            if let Some(kind) = records::ParamSectionKind::from_storage_name(&storage_name) {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data = doc.read_stream(&format!("{storage_path}/Data"))?;
                let expected_count = parse_pcb_section_header(&header_data)? as usize;
                let records = records::parse_standard_param_records(&data)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                if expected_count != records.len() {
                    return Err(AltiumFormatError::RecordCountMismatch {
                        section: storage_name.clone(),
                        expected: expected_count,
                        actual: records.len(),
                    });
                }
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
                sections.push(PcbDocSection::Parameter(ParamSectionData { kind, records }));
                continue;
            }

            if let Some(kind) = records::PrefixedParamSectionKind::from_storage_name(&storage_name)
            {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data = doc.read_stream(&format!("{storage_path}/Data"))?;
                let expected_count = parse_pcb_section_header(&header_data)? as usize;
                let records = records::parse_prefixed_param_records(&data)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                if expected_count != records.len() {
                    return Err(AltiumFormatError::RecordCountMismatch {
                        section: storage_name.clone(),
                        expected: expected_count,
                        actual: records.len(),
                    });
                }
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
                sections.push(PcbDocSection::PrefixedParameter(PrefixedParamSectionData {
                    kind,
                    records,
                }));
                continue;
            }

            return Err(AltiumFormatError::InvalidParamValue {
                key: "PcbDoc storage".to_owned(),
                detail: format!("unimplemented storage /{storage_name}"),
            });
        }

        doc.assert_all_consumed()?;

        let doc = Self {
            legacy_header,
            header,
            sections,
        };
        doc.validate_invariants()
            .context("validating PcbDoc invariants")?;
        Ok(doc)
    }
}

fn validate_pcbdoc_invariants(doc: &PcbDoc) -> Result<()> {
    if !PCB_DOC_BINARY_HEADER_V5.starts_with(&doc.legacy_header)
        || !doc.legacy_header.starts_with("PCB ")
    {
        return Err(AltiumFormatError::InvalidParamValue {
            key: FILE_HEADER.to_owned(),
            detail: format!(
                "expected legacy header prefix of {:?}, got {:?}",
                PCB_DOC_BINARY_HEADER_V5, doc.legacy_header
            ),
        });
    }
    if doc.header.version_string != PCB_DOC_BINARY_HEADER_V6 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "FileHeaderSix".to_owned(),
            detail: format!(
                "expected v6 header {:?}, got {:?}",
                PCB_DOC_BINARY_HEADER_V6, doc.header.version_string
            ),
        });
    }
    if !doc.header.version.is_finite() || doc.header.version <= 0.0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "FileHeaderSix.version".to_owned(),
            detail: format!("invalid version number {}", doc.header.version),
        });
    }

    let mut seen = HashSet::new();
    for section in &doc.sections {
        let id = section_identity(section);
        if !seen.insert(id.clone()) {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "PcbDoc.sections".to_owned(),
                detail: format!("duplicate section {id}"),
            });
        }

        if let PcbDocSection::UnionNames(v) = section {
            if v.format_version != 1 {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: "UnionNames/Header".to_owned(),
                    detail: format!("expected format version 1, got {}", v.format_version),
                });
            }
        }
        if let PcbDocSection::EmbeddedFonts(v) = section {
            if v.header_count as usize != v.entries.len() {
                return Err(AltiumFormatError::RecordCountMismatch {
                    section: "EmbeddedFonts6".to_owned(),
                    expected: v.header_count as usize,
                    actual: v.entries.len(),
                });
            }
        }
    }

    Ok(())
}

fn section_identity(section: &PcbDocSection) -> String {
    match section {
        PcbDocSection::Primitive(v) => format!("Primitive::{:?}", v.kind),
        PcbDocSection::Parameter(v) => format!("Parameter::{:?}", v.kind),
        PcbDocSection::Binary(v) => format!("Binary::{:?}", v.kind),
        PcbDocSection::UnionNames(_) => "UnionNames".to_owned(),
        PcbDocSection::PrefixedParameter(v) => format!("Prefixed::{:?}", v.kind),
        PcbDocSection::WideStrings(_) => "WideStrings6".to_owned(),
        PcbDocSection::Models(_) => "Models".to_owned(),
        PcbDocSection::EmbeddedFonts(_) => "EmbeddedFonts6".to_owned(),
        PcbDocSection::PadViaLibrary(v) => format!("PadVia::{:?}", v.section_name),
        PcbDocSection::LayerKindMapping(_) => "LayerKindMapping".to_owned(),
    }
}

fn assert_known_section_layout(
    doc: &mut TrackedCfbDocument,
    storage_name: &str,
    storage_path: &str,
) -> Result<()> {
    let (storages, streams) = doc.list_entries(storage_path)?;
    if !storages.is_empty() {
        return Err(AltiumFormatError::InvalidParamValue {
            key: storage_name.to_owned(),
            detail: format!("unexpected nested storages: {}", storages.join(", ")),
        });
    }

    let mut unexpected_streams = Vec::new();
    for stream in streams {
        if stream != "Header" && stream != "Data" {
            unexpected_streams.push(stream);
        }
    }
    if !unexpected_streams.is_empty() {
        return Err(AltiumFormatError::InvalidParamValue {
            key: storage_name.to_owned(),
            detail: format!("unexpected streams: {}", unexpected_streams.join(", ")),
        });
    }
    Ok(())
}

fn parse_models_storage(doc: &mut TrackedCfbDocument, storage_name: &str) -> Result<PcbDocSection> {
    let storage_path = format!("/{storage_name}");
    let (storages, streams) = doc.list_entries(&storage_path)?;
    if !storages.is_empty() {
        return Err(AltiumFormatError::InvalidParamValue {
            key: storage_name.to_owned(),
            detail: format!("unexpected nested storages: {}", storages.join(", ")),
        });
    }

    let has_header = streams.iter().any(|s| s == "Header");
    let has_data = streams.iter().any(|s| s == "Data");
    if has_header != has_data {
        return Err(AltiumFormatError::InvalidParamValue {
            key: storage_name.to_owned(),
            detail: "expected Header and Data to both exist or both be absent".to_owned(),
        });
    }

    let mut metadata = Vec::new();
    if has_header {
        let header = doc.read_stream(&format!("{storage_path}/Header"))?;
        let data = doc.read_stream(&format!("{storage_path}/Data"))?;
        metadata = parse_model_metadata(&header, &data)
            .with_context(|| format!("parsing {storage_path}/Data as model metadata"))?;
    }

    let mut blobs = Vec::new();
    for stream in streams {
        if stream == "Header" || stream == "Data" {
            continue;
        }
        let bytes = doc.read_stream(&format!("{storage_path}/{stream}"))?;
        blobs.push((stream, bytes));
    }

    Ok(PcbDocSection::Models(ModelsSectionData { metadata, blobs }))
}

fn read_utf16le_len_prefixed(reader: &mut BinaryReader, key: &str) -> Result<String> {
    let byte_len = reader.read_u32_le()? as usize;
    if (byte_len % 2) != 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: key.to_owned(),
            detail: format!("UTF-16LE byte length must be even, got {byte_len}"),
        });
    }
    let raw = reader.read_bytes(byte_len)?;
    let (decoded, _, had_errors) = encoding_rs::UTF_16LE.decode(raw);
    if had_errors {
        return Err(AltiumFormatError::InvalidParamValue {
            key: key.to_owned(),
            detail: "invalid UTF-16LE sequence".to_owned(),
        });
    }
    Ok(decoded.trim_end_matches('\0').to_owned())
}

fn parse_embedded_fonts6_data(
    data: &[u8],
    expected_count: usize,
) -> Result<Vec<PcbEmbeddedFontEntry>> {
    if expected_count == 0 {
        if !data.is_empty() {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "EmbeddedFonts6/Header".to_owned(),
                detail: "header count is 0 but data stream is not empty".to_owned(),
            });
        }
        return Ok(Vec::new());
    }

    let mut reader = BinaryReader::new(data);
    let mut entries = Vec::with_capacity(expected_count);
    for idx in 0..expected_count {
        let name = read_utf16le_len_prefixed(&mut reader, "EmbeddedFonts6.name")?;
        let style_name = read_utf16le_len_prefixed(&mut reader, "EmbeddedFonts6.style_name")?;
        let localized_name =
            read_utf16le_len_prefixed(&mut reader, "EmbeddedFonts6.localized_name")?;
        let unknown_u16 = reader.read_u16_le()?;
        let flag = reader.read_u8()?;
        let blob_size = reader.read_u32_le()? as usize;
        let blob = reader.read_bytes(blob_size)?;
        entries.push(PcbEmbeddedFontEntry {
            name,
            style_name,
            localized_name,
            unknown_u16,
            flag,
            data: blob.to_vec(),
        });
        if idx + 1 == expected_count {
            break;
        }
    }
    reader.assert_exhausted()?;
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::fs;

    fn fixture_paths() -> Vec<std::path::PathBuf> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/pcbdoc");
        let mut out = Vec::new();
        let entries = fs::read_dir(dir).expect("read data/pcbdoc");
        for entry in entries.flatten() {
            let path = entry.path();
            let is_pcbdoc = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("pcbdoc"))
                .unwrap_or(false);
            if is_pcbdoc {
                out.push(path);
            }
        }
        out.sort();
        out
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 16, .. ProptestConfig::default() })]

        #[test]
        fn prop_pcbdoc_invariants_hold_for_fixtures(idx in 0usize..4096usize) {
            let fixtures = fixture_paths();
            prop_assume!(!fixtures.is_empty());
            let path = &fixtures[idx % fixtures.len()];
            let doc = PcbDoc::open(path).expect("open pcbdoc");
            doc.validate_invariants().expect("pcbdoc invariant check");
        }

        #[test]
        fn prop_pcbdoc_invariants_reject_broken_header(idx in 0usize..4096usize) {
            let fixtures = fixture_paths();
            prop_assume!(!fixtures.is_empty());
            let path = &fixtures[idx % fixtures.len()];
            let mut doc = PcbDoc::open(path).expect("open pcbdoc");
            doc.header.version_string = "BROKEN".to_owned();
            let err = doc.validate_invariants().expect_err("broken header should fail");
            prop_assert!(err.to_string().contains("expected v6 header"));
        }
    }
}

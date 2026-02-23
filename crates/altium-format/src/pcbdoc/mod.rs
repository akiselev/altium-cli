mod primitives;
mod records;

use std::path::Path;

use altium_format_types::constants::file_headers::{
    PCB_DOC_BINARY_HEADER_V5, PCB_DOC_BINARY_HEADER_V6,
};
use altium_format_types::constants::streams::FILE_HEADER;

use crate::pcb_binary_stream::parse_pcb_section_header;
use crate::pcb_file_header::{PcbFileHeader, parse_pcb_file_header, parse_pcb_legacy_header};
use crate::tracked_cfb::TrackedCfbDocument;
use crate::wide_strings_tlv::{WideStringEntry, parse_wide_strings_tlv};
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
    pub(crate) entries: Vec<WideStringEntry>,
}

pub(crate) struct RawSectionData {
    pub(crate) storage_name: String,
    pub(crate) header: Option<Vec<u8>>,
    pub(crate) data: Option<Vec<u8>>,
    pub(crate) extra_streams: Vec<(String, Vec<u8>)>,
}

pub(crate) enum PcbDocSection {
    Primitive(PrimitiveSectionData),
    Parameter(ParamSectionData),
    PrefixedParameter(PrefixedParamSectionData),
    WideStrings(WideStringsSectionData),
    Raw(RawSectionData),
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

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let mut doc = TrackedCfbDocument::open(path)?;

        let legacy_data = doc.read_stream(&format!("/{FILE_HEADER}"))?;
        let legacy_header = parse_pcb_legacy_header(&legacy_data).context("parsing /FileHeader")?;
        if legacy_header != PCB_DOC_BINARY_HEADER_V5 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: FILE_HEADER.to_owned(),
                detail: format!(
                    "expected legacy header \"{}\", got \"{}\"",
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
                let entries = parse_wide_strings_tlv(&data)
                    .with_context(|| format!("parsing {storage_path}/Data as WideStrings6 TLV"))?;
                if expected_count != entries.len() {
                    return Err(AltiumFormatError::RecordCountMismatch {
                        section: storage_name.clone(),
                        expected: expected_count,
                        actual: entries.len(),
                    });
                }
                let _ = doc.list_entries(&storage_path)?;
                sections.push(PcbDocSection::WideStrings(WideStringsSectionData {
                    entries,
                }));
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
                let _ = doc.list_entries(&storage_path)?;
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
                let _ = doc.list_entries(&storage_path)?;
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
                let _ = doc.list_entries(&storage_path)?;
                sections.push(PcbDocSection::PrefixedParameter(PrefixedParamSectionData {
                    kind,
                    records,
                }));
                continue;
            }

            sections.push(parse_raw_storage(&mut doc, &storage_name)?);
        }

        doc.assert_all_consumed()?;

        Ok(Self {
            legacy_header,
            header,
            sections,
        })
    }
}

fn parse_raw_storage(doc: &mut TrackedCfbDocument, storage_name: &str) -> Result<PcbDocSection> {
    let storage_path = format!("/{storage_name}");
    let header_path = format!("{storage_path}/Header");
    let data_path = format!("{storage_path}/Data");

    let header = doc.read_stream_optional(&header_path)?;
    let data = doc.read_stream_optional(&data_path)?;
    let (_storages, streams) = doc.list_entries(&storage_path)?;

    let mut extra_streams = Vec::new();
    for stream in streams {
        if stream == "Header" || stream == "Data" {
            continue;
        }
        let stream_path = format!("{storage_path}/{stream}");
        let bytes = doc.read_stream(&stream_path)?;
        extra_streams.push((stream, bytes));
    }

    Ok(PcbDocSection::Raw(RawSectionData {
        storage_name: storage_name.to_owned(),
        header,
        data,
        extra_streams,
    }))
}

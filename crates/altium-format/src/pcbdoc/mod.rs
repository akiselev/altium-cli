pub(crate) mod drc;
pub(crate) mod primitives;
pub(crate) mod records;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use altium_format_types::Coord;
use altium_format_types::constants::file_headers::{
    PCB_DOC_BINARY_HEADER_V6,
};
use altium_format_types::constants::streams::FILE_HEADER;

use crate::binary_io::{BinaryReader, BinaryWriter};
use crate::cfb_document::CfbDocument;
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

pub(crate) struct SharedUnionsSectionData {
    pub(crate) entries: Vec<crate::shared_union::SharedUnionEntry>,
}

pub(crate) struct UnionRelationsSectionData {
    pub(crate) records: Vec<records::UnionRelationRecord>,
}

pub(crate) struct PrimitiveParametersSectionData {
    pub(crate) groups: Vec<records::PrimitiveParameterGroup>,
}

pub(crate) struct UnionFeaturesSectionData {
    pub(crate) records: Vec<records::IndexedParamRecord>,
}

pub(crate) struct SharedUnionParamSectionData {
    pub(crate) groups: Vec<records::SharedUnionParamGroup>,
}

pub(crate) struct ConstraintManagerSectionData {
    pub(crate) header_value: u32,
    pub(crate) xml: String,
}

pub(crate) struct PrimitiveGuidsSectionData {
    pub(crate) entries: Vec<crate::pcblib::sidecar::PrimitiveGuidEntryPcbDoc>,
}

/// DrillManager storage: drill symbol configuration with per-hole-group records.
/// Format: i32 sentinel(-1) + u32 count + N records (param text + pad/via index lists) + u32 trailing.
pub(crate) struct DrillManagerSectionData {
    pub(crate) records: Vec<DrillManagerRecord>,
}

pub(crate) struct DrillManagerRecord {
    pub(crate) params: crate::param_collection::ParameterCollection,
    pub(crate) pad_indices: Vec<u32>,
    pub(crate) via_indices: Vec<u32>,
}

/// LettersGeometry storage: cached TrueType font glyph tessellation data.
/// Contains Header (u32 count), PrimIndexes, and Data streams.
pub(crate) struct LettersGeometrySectionData {
    pub(crate) header_count: u32,
    pub(crate) prim_indexes: Vec<u8>,
    pub(crate) data: Vec<u8>,
}

pub(crate) enum PcbDocSection {
    Primitive(PrimitiveSectionData),
    Parameter(ParamSectionData),
    Binary(BinarySectionData),
    UnionNames(UnionNamesSectionData),
    SharedUnions(SharedUnionsSectionData),
    UnionRelations(UnionRelationsSectionData),
    PrefixedParameter(PrefixedParamSectionData),
    WideStrings(WideStringsSectionData),
    Models(ModelsSectionData),
    EmbeddedFonts(EmbeddedFontsSectionData),
    PadViaLibrary(PadViaLibrarySectionData),
    LayerKindMapping(LayerKindMappingSectionData),
    PrimitiveParameters(PrimitiveParametersSectionData),
    UnionFeatures(UnionFeaturesSectionData),
    SharedUnionParam(SharedUnionParamSectionData),
    ConstraintManager(ConstraintManagerSectionData),
    PrimitiveGuids(PrimitiveGuidsSectionData),
    DrillManager(DrillManagerSectionData),
    LettersGeometry(LettersGeometrySectionData),
}

pub struct PcbDoc {
    pub(crate) legacy_header: String,
    pub(crate) header: PcbFileHeader,
    pub(crate) sections: Vec<PcbDocSection>,
    pub(crate) rules: Vec<drc::PcbRule>,
    pub(crate) violations: indexmap::IndexMap<records::ParamSectionKind, Vec<drc::PcbViolation>>,
    pub(crate) waived_violations: Vec<drc::WaivedViolation>,
    pub(crate) drc_options: Option<drc::DrcOptions>,
    pub(crate) source_path: Option<PathBuf>,
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
        if legacy_header.trim().is_empty() {
            return Err(AltiumFormatError::InvalidParamValue {
                key: FILE_HEADER.to_owned(),
                detail: format!(
                    "expected non-empty legacy header, got \"{}\"",
                    legacy_header
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
        let mut rules = Vec::new();
        let mut violations: indexmap::IndexMap<records::ParamSectionKind, Vec<drc::PcbViolation>> =
            indexmap::IndexMap::new();
        let mut waived_violations = Vec::new();
        let mut drc_options = None;

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
                    .with_context(|| format!("parsing {storage_path}"))?;
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

            if storage_name == "SharedUnions" {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data = doc.read_stream(&format!("{storage_path}/Data"))?;
                let expected_count = parse_pcb_section_header(&header_data)? as usize;
                let entries = crate::shared_union::parse_shared_union_stream(&data)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                validate_record_count(&storage_name, expected_count, entries.len())?;
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
                sections.push(PcbDocSection::SharedUnions(SharedUnionsSectionData {
                    entries,
                }));
                continue;
            }

            if storage_name == "UnionFeatures" {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data = doc.read_stream(&format!("{storage_path}/Data"))?;
                let expected_count = parse_pcb_section_header(&header_data)? as usize;
                let records = records::parse_indexed_param_records(&data)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                validate_record_count(&storage_name, expected_count, records.len())?;
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
                sections.push(PcbDocSection::UnionFeatures(UnionFeaturesSectionData {
                    records,
                }));
                continue;
            }

            if storage_name == "SharedUnion" {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data = doc.read_stream(&format!("{storage_path}/Data"))?;
                let expected_count = parse_pcb_section_header(&header_data)? as usize;
                let groups = records::parse_shared_union_param_records(&data)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                validate_record_count(&storage_name, expected_count, groups.len())?;
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
                sections.push(PcbDocSection::SharedUnionParam(
                    SharedUnionParamSectionData { groups },
                ));
                continue;
            }

            if storage_name == "PrimitiveParameters" {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data = doc.read_stream(&format!("{storage_path}/Data"))?;
                let expected_count = parse_pcb_section_header(&header_data)? as usize;
                let groups = records::parse_primitive_parameter_records(&data)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                validate_record_count(&storage_name, expected_count, groups.len())?;
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
                sections.push(PcbDocSection::PrimitiveParameters(
                    PrimitiveParametersSectionData { groups },
                ));
                continue;
            }

            if storage_name == "UnionRelations" {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data = doc.read_stream(&format!("{storage_path}/Data"))?;
                let expected_count = parse_pcb_section_header(&header_data)? as usize;
                let records = records::parse_union_relation_records(&data)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                validate_record_count(&storage_name, expected_count, records.len())?;
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
                sections.push(PcbDocSection::UnionRelations(UnionRelationsSectionData {
                    records,
                }));
                continue;
            }

            if let Some(kind) = records::BinaryLenSectionKind::from_storage_name(&storage_name) {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data = doc.read_stream(&format!("{storage_path}/Data"))?;
                let expected_count = parse_pcb_section_header(&header_data)? as usize;
                let records = records::parse_len_prefixed_binary_records(&data)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                validate_record_count(&storage_name, expected_count, records.len())?;
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
                validate_record_count(&storage_name, expected_count, records.len())?;
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
                validate_record_count(&storage_name, expected_count, records.len())?;
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;

                if kind.is_violation() {
                    let mut parsed = Vec::new();
                    for (i, mut record) in records.into_iter().enumerate() {
                        let violation = drc::parse_violation(kind, &mut record.params)
                            .with_context(|| {
                                format!("parsing {storage_path}/Data record #{i}")
                            })?;
                        parsed.push(violation);
                    }
                    violations.entry(kind).or_default().extend(parsed);
                } else if kind == records::ParamSectionKind::WaivedViolations {
                    for (i, mut record) in records.into_iter().enumerate() {
                        record.params.apply_unicode_sidecars()?;
                        let wv = drc::WaivedViolation::from_params(&mut record.params)
                            .with_context(|| {
                                format!("parsing {storage_path}/Data record #{i}")
                            })?;
                        record.params.assert_exhausted()?;
                        waived_violations.push(wv);
                    }
                } else if kind == records::ParamSectionKind::DesignRuleCheckerOptions6 {
                    if records.len() != 1 {
                        return Err(AltiumFormatError::RecordCountMismatch {
                            section: storage_name.clone(),
                            expected: 1,
                            actual: records.len(),
                        });
                    }
                    let mut record = records.into_iter().next().unwrap();
                    drc_options = Some(
                        drc::DrcOptions::from_params(&mut record.params).with_context(|| {
                            format!("parsing {storage_path}/Data")
                        })?,
                    );
                } else {
                    sections
                        .push(PcbDocSection::Parameter(ParamSectionData { kind, records }));
                }
                continue;
            }

            if let Some(kind) = records::PrefixedParamSectionKind::from_storage_name(&storage_name)
            {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data = doc.read_stream(&format!("{storage_path}/Data"))?;
                let expected_count = parse_pcb_section_header(&header_data)? as usize;
                let records = records::parse_prefixed_param_records(&data)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                validate_record_count(&storage_name, expected_count, records.len())?;
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;

                if kind == records::PrefixedParamSectionKind::Rules6
                    || kind == records::PrefixedParamSectionKind::NewRules6
                {
                    for (i, mut record) in records.into_iter().enumerate() {
                        let rule = drc::parse_rule(record.prefix, &mut record.params)
                            .with_context(|| {
                                format!("parsing {storage_path}/Data record #{i}")
                            })?;
                        rules.push(rule);
                    }
                } else {
                    sections.push(PcbDocSection::PrefixedParameter(
                        PrefixedParamSectionData { kind, records },
                    ));
                }
                continue;
            }

            if storage_name == "ConstraintManager" {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data_bytes = doc.read_stream(&format!("{storage_path}/Data"))?;
                let header_value = parse_pcb_section_header(&header_data)?;
                let xml = decode_constraint_manager_data(&data_bytes)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
                sections.push(PcbDocSection::ConstraintManager(
                    ConstraintManagerSectionData { header_value, xml },
                ));
                continue;
            }

            if storage_name == "PrimitiveGuids" {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data_bytes = doc.read_stream(&format!("{storage_path}/Data"))?;
                let entries = crate::pcblib::sidecar::parse_primitive_guids_pcbdoc(&header_data, &data_bytes)
                    .with_context(|| format!("parsing {storage_path}"))?;
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
                sections.push(PcbDocSection::PrimitiveGuids(
                    PrimitiveGuidsSectionData { entries },
                ));
                continue;
            }

            if storage_name == "DrillManager" {
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let data_bytes = doc.read_stream(&format!("{storage_path}/Data"))?;
                let _header_value = parse_pcb_section_header(&header_data)?;
                let records = parse_drill_manager_data(&data_bytes)
                    .with_context(|| format!("parsing {storage_path}/Data"))?;
                assert_known_section_layout(&mut doc, &storage_name, &storage_path)?;
                sections.push(PcbDocSection::DrillManager(
                    DrillManagerSectionData { records },
                ));
                continue;
            }

            if storage_name == "LettersGeometry" {
                // LettersGeometry has 3 streams: Header, PrimIndexes, Data.
                // read_stream auto-marks each as consumed.
                let header_data = doc.read_stream(&format!("{storage_path}/Header"))?;
                let prim_indexes = doc.read_stream(&format!("{storage_path}/PrimIndexes"))?;
                let data = doc.read_stream(&format!("{storage_path}/Data"))?;
                let mut hr = BinaryReader::new(&header_data);
                let header_count = hr.read_u32_le()?;
                hr.assert_exhausted().with_context(|| {
                    format!("parsing {storage_path}/Header")
                })?;
                // Validate the storage layout: expect exactly Header, PrimIndexes, Data
                let (_storages, streams) = doc.list_entries(&storage_path)?;
                let expected: HashSet<&str> = ["Header", "PrimIndexes", "Data"].into_iter().collect();
                let actual: HashSet<&str> = streams.iter().map(|s| s.as_str()).collect();
                if actual != expected {
                    return Err(AltiumFormatError::InvalidParamValue {
                        key: storage_name.to_owned(),
                        detail: format!("unexpected streams: expected {{Header, PrimIndexes, Data}}, got {:?}", streams),
                    });
                }
                sections.push(PcbDocSection::LettersGeometry(
                    LettersGeometrySectionData {
                        header_count,
                        prim_indexes,
                        data,
                    },
                ));
                continue;
            }

            return Err(AltiumFormatError::InvalidParamValue {
                key: "PcbDoc storage".to_owned(),
                detail: format!(
                    "unsupported storage '/{storage_name}' encountered; typed parser required"
                ),
            });
        }

        doc.assert_all_consumed()?;

        let doc = Self {
            legacy_header,
            header,
            sections,
            rules,
            violations,
            waived_violations,
            drc_options,
            source_path: Some(path.to_path_buf()),
        };
        doc.validate_invariants()
            .context("validating PcbDoc invariants")?;
        Ok(doc)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let _ = path.as_ref();
        Err(AltiumFormatError::InvalidParamValue {
            key: "PcbDoc.save".to_owned(),
            detail: "full PcbDoc reserialization is required; source-backed/raw passthrough save is disabled".to_owned(),
        })
    }

    fn primitive_section(&self, kind: records::PrimitiveSectionKind) -> Option<&[primitives::ParsedPrimitiveRecord]> {
        self.sections.iter().find_map(|section| {
            if let PcbDocSection::Primitive(p) = section
                && p.kind == kind
            {
                Some(p.records.as_slice())
            } else {
                None
            }
        })
    }
}

/// Check that a Coord used as a non-negative dimension is in `[0, MAX_REASONABLE]`.
fn check_dimension(
    value: Coord,
    primitive: &str,
    index: usize,
    field: &str,
    section: &str,
) -> Result<()> {
    if value.to_internal() < 0 || value > Coord::MAX_REASONABLE_DIMENSION {
        return Err(AltiumFormatError::InvalidParamValue {
            key: format!("{primitive}[{index}].{field}"),
            detail: format!(
                "section {:?}: dimension {} out of range [0, {}]",
                section,
                value,
                Coord::MAX_REASONABLE_DIMENSION,
            ),
        });
    }
    Ok(())
}

/// Check that a Coord used as an expansion (can be negative) has `|value| <= MAX_REASONABLE`.
fn check_expansion(
    value: Coord,
    primitive: &str,
    index: usize,
    field: &str,
    section: &str,
) -> Result<()> {
    if value.abs() > Coord::MAX_REASONABLE_DIMENSION {
        return Err(AltiumFormatError::InvalidParamValue {
            key: format!("{primitive}[{index}].{field}"),
            detail: format!(
                "section {:?}: expansion {} out of range [-{}, {}]",
                section,
                value,
                Coord::MAX_REASONABLE_DIMENSION,
                Coord::MAX_REASONABLE_DIMENSION,
            ),
        });
    }
    Ok(())
}

fn validate_pcbdoc_primitive_coords(doc: &PcbDoc) -> Result<()> {
    for section in &doc.sections {
        if let PcbDocSection::Primitive(prim_section) = section {
            let section_name = format!("{:?}", prim_section.kind);
            for (idx, rec) in prim_section.records.iter().enumerate() {
                match &rec.primitive {
                    primitives::PcbPrimitive::Via(v) => {
                        check_dimension(v.diameter, "Via", idx, "diameter", &section_name)?;
                        check_dimension(v.hole_size, "Via", idx, "hole_size", &section_name)?;
                        check_expansion(v.thermal_relief_air_gap, "Via", idx, "thermal_relief_air_gap", &section_name)?;
                        check_expansion(v.thermal_relief_conductor_width, "Via", idx, "thermal_relief_conductor_width", &section_name)?;
                        check_expansion(v.power_plane_relief_expansion, "Via", idx, "power_plane_relief_expansion", &section_name)?;
                        check_expansion(v.power_plane_clearance, "Via", idx, "power_plane_clearance", &section_name)?;
                        // Only validate mask expansions when override is active.
                        // When "from rule", the stored value is a stale default and
                        // may contain arbitrary data.
                        if v.paste_mask_override {
                            check_expansion(v.paste_mask_expansion, "Via", idx, "paste_mask_expansion", &section_name)?;
                        }
                        if v.solder_mask_override {
                            check_expansion(v.solder_mask_expansion_front, "Via", idx, "solder_mask_expansion_front", &section_name)?;
                            check_expansion(v.solder_mask_expansion_back, "Via", idx, "solder_mask_expansion_back", &section_name)?;
                        }
                        for (i, d) in v.diameters_per_layer.iter().enumerate() {
                            check_dimension(*d, "Via", idx, &format!("diameters_per_layer[{i}]"), &section_name)?;
                        }
                        // Extension boolean flags (is_testpoint_top/bottom, is_assy_testpoint_top/bottom,
                        // solder_mask_override, use_separate_solder_mask_expansion,
                        // solder_mask_expansion_from_hole_edge, paste_mask_override): no range check needed
                        if let Some(tol) = v.hole_positive_tolerance {
                            check_expansion(tol, "Via", idx, "hole_positive_tolerance", &section_name)?;
                        }
                        if let Some(tol) = v.hole_negative_tolerance {
                            check_expansion(tol, "Via", idx, "hole_negative_tolerance", &section_name)?;
                        }
                        // Semantic: diameter >= hole_size when both > 0
                        if v.diameter > Coord::ZERO && v.hole_size > Coord::ZERO && v.diameter < v.hole_size {
                            return Err(AltiumFormatError::InvalidParamValue {
                                key: format!("Via[{idx}].diameter"),
                                detail: format!(
                                    "section {:?}: diameter ({}) < hole_size ({})",
                                    section_name, v.diameter, v.hole_size,
                                ),
                            });
                        }
                    }
                    primitives::PcbPrimitive::Pad(p) => {
                        check_dimension(p.size_top.x, "Pad", idx, "size_top.x", &section_name)?;
                        check_dimension(p.size_top.y, "Pad", idx, "size_top.y", &section_name)?;
                        check_dimension(p.size_mid.x, "Pad", idx, "size_mid.x", &section_name)?;
                        check_dimension(p.size_mid.y, "Pad", idx, "size_mid.y", &section_name)?;
                        check_dimension(p.size_bot.x, "Pad", idx, "size_bot.x", &section_name)?;
                        check_dimension(p.size_bot.y, "Pad", idx, "size_bot.y", &section_name)?;
                        check_dimension(p.hole_size, "Pad", idx, "hole_size", &section_name)?;
                        check_expansion(p.cache.relief_conductor_width, "Pad", idx, "cache.relief_conductor_width", &section_name)?;
                        check_expansion(p.cache.relief_air_gap, "Pad", idx, "cache.relief_air_gap", &section_name)?;
                        check_expansion(p.cache.power_plane_relief_expansion, "Pad", idx, "cache.power_plane_relief_expansion", &section_name)?;
                        check_expansion(p.cache.power_plane_clearance, "Pad", idx, "cache.power_plane_clearance", &section_name)?;
                        check_expansion(p.cache.paste_mask_expansion, "Pad", idx, "cache.paste_mask_expansion", &section_name)?;
                        check_expansion(p.cache.solder_mask_expansion, "Pad", idx, "cache.solder_mask_expansion", &section_name)?;
                        check_dimension(p.pin_package_length, "Pad", idx, "pin_package_length", &section_name)?;
                    }
                    primitives::PcbPrimitive::Arc(a) => {
                        // Arc radius is signed in Altium (IPCB_Arc.GetState_Radius
                        // returns int). Degenerate zero-sweep arcs used in unions
                        // can have negative or very large radii — no range check.
                        check_dimension(a.width, "Arc", idx, "width", &section_name)?;
                        if !a.start_angle.is_finite() || a.start_angle < 0.0 || a.start_angle > 360.0 {
                            return Err(AltiumFormatError::InvalidParamValue {
                                key: format!("Arc[{idx}].start_angle"),
                                detail: format!(
                                    "section {:?}: start_angle {} not in [0, 360]",
                                    section_name, a.start_angle,
                                ),
                            });
                        }
                        if !a.end_angle.is_finite() || a.end_angle < 0.0 || a.end_angle > 360.0 {
                            return Err(AltiumFormatError::InvalidParamValue {
                                key: format!("Arc[{idx}].end_angle"),
                                detail: format!(
                                    "section {:?}: end_angle {} not in [0, 360]",
                                    section_name, a.end_angle,
                                ),
                            });
                        }
                    }
                    primitives::PcbPrimitive::Track(t) => {
                        check_dimension(t.width, "Track", idx, "width", &section_name)?;
                    }
                    primitives::PcbPrimitive::Text(t) => {
                        check_dimension(t.height, "Text", idx, "height", &section_name)?;
                        check_dimension(t.stroke_width, "Text", idx, "stroke_width", &section_name)?;
                    }
                    primitives::PcbPrimitive::Region(r) => {
                        check_dimension(r.arc_resolution, "Region", idx, "arc_resolution", &section_name)?;
                        check_expansion(r.cavity_height, "Region", idx, "cavity_height", &section_name)?;
                    }
                    primitives::PcbPrimitive::ComponentBody(b) => {
                        check_expansion(b.standoff_height, "ComponentBody", idx, "standoff_height", &section_name)?;
                        check_expansion(b.overall_height, "ComponentBody", idx, "overall_height", &section_name)?;
                        check_dimension(b.arc_resolution, "ComponentBody", idx, "arc_resolution", &section_name)?;
                        check_expansion(b.cavity_height, "ComponentBody", idx, "cavity_height", &section_name)?;
                    }
                    primitives::PcbPrimitive::Fill(_) => {}
                }
            }
        }
    }
    Ok(())
}

fn validate_pcbdoc_invariants(doc: &PcbDoc) -> Result<()> {
    if doc.legacy_header.trim().is_empty() {
        return Err(AltiumFormatError::InvalidParamValue {
            key: FILE_HEADER.to_owned(),
            detail: format!(
                "expected non-empty legacy header, got {:?}",
                doc.legacy_header
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

    validate_pcbdoc_primitive_coords(doc)?;

    Ok(())
}

fn section_identity(section: &PcbDocSection) -> String {
    match section {
        PcbDocSection::Primitive(v) => format!("Primitive::{:?}", v.kind),
        PcbDocSection::Parameter(v) => format!("Parameter::{:?}", v.kind),
        PcbDocSection::Binary(v) => format!("Binary::{:?}", v.kind),
        PcbDocSection::UnionNames(_) => "UnionNames".to_owned(),
        PcbDocSection::SharedUnions(_) => "SharedUnions".to_owned(),
        PcbDocSection::UnionRelations(_) => "UnionRelations".to_owned(),
        PcbDocSection::PrefixedParameter(v) => format!("Prefixed::{:?}", v.kind),
        PcbDocSection::WideStrings(_) => "WideStrings6".to_owned(),
        PcbDocSection::Models(_) => "Models".to_owned(),
        PcbDocSection::EmbeddedFonts(_) => "EmbeddedFonts6".to_owned(),
        PcbDocSection::PadViaLibrary(v) => format!("PadVia::{:?}", v.section_name),
        PcbDocSection::LayerKindMapping(_) => "LayerKindMapping".to_owned(),
        PcbDocSection::PrimitiveParameters(_) => "PrimitiveParameters".to_owned(),
        PcbDocSection::UnionFeatures(_) => "UnionFeatures".to_owned(),
        PcbDocSection::SharedUnionParam(_) => "SharedUnion".to_owned(),
        PcbDocSection::ConstraintManager(_) => "ConstraintManager".to_owned(),
        PcbDocSection::PrimitiveGuids(_) => "PrimitiveGuids".to_owned(),
        PcbDocSection::DrillManager(_) => "DrillManager".to_owned(),
        PcbDocSection::LettersGeometry(_) => "LettersGeometry".to_owned(),
    }
}

fn write_primitive_section(
    cfb: &mut CfbDocument,
    name: &str,
    records: &[primitives::ParsedPrimitiveRecord],
) -> Result<()> {
    let storage = format!("/{name}");
    if !cfb.exists(&storage) {
        cfb.create_storage(&storage)?;
    }
    let mut header = BinaryWriter::new();
    header.write_u32_le(records.len() as u32);
    let mut data = BinaryWriter::new();
    for record in records {
        let payload = serialize_primitive_payload(record)?;
        data.write_u8(record.object_id as u8);
        data.write_u32_le(payload.len() as u32);
        data.write_bytes(&payload);
    }
    cfb.write_stream(&format!("{storage}/Header"), &header.finish())?;
    cfb.write_stream(&format!("{storage}/Data"), &data.finish())?;
    Ok(())
}

fn serialize_common_header(w: &mut BinaryWriter, common: &primitives::PcbPrimitiveCommon) {
    w.write_u8(common.layer as u8);
    w.write_u16_le(common.flags.raw());
    w.write_u16_le(common.net_index);
    w.write_u16_le(common.polygon_index);
    w.write_u16_le(common.component_index);
    w.write_u16_le(common.coordinate_index);
    w.write_u16_le(common.dimension_index);
}

fn serialize_primitive_payload(record: &primitives::ParsedPrimitiveRecord) -> Result<Vec<u8>> {
    match &record.primitive {
        primitives::PcbPrimitive::Track(v) => {
            let mut w = BinaryWriter::new();
            serialize_common_header(&mut w, &v.common);
            w.write_coord_point(v.start);
            w.write_coord_point(v.end);
            w.write_coord(v.width);
            w.write_u16_le(v.subpoly_index);
            w.write_u8(if v.user_routed { 1 } else { 0 });
            w.write_i32_le(v.union_index);
            w.write_u8(v.track_kind);
            w.write_u32_le(v.layer_enum_index.raw());
            if let Some(k) = v.keepout_restrictions {
                w.write_i32_le(k);
            }
            Ok(w.finish())
        }
        _ => Err(AltiumFormatError::InvalidParamValue {
            key: "PcbDoc.save".to_owned(),
            detail: "unsupported primitive for PcbDoc serialization".to_owned(),
        }),
    }
}

fn validate_record_count(section: &str, expected: usize, actual: usize) -> Result<()> {
    if expected != actual {
        return Err(AltiumFormatError::RecordCountMismatch {
            section: section.to_owned(),
            expected,
            actual,
        });
    }
    Ok(())
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
        let full_name =
            read_utf16le_len_prefixed(&mut reader, "EmbeddedFonts6.full_name")?;
        let face_name =
            read_utf16le_len_prefixed(&mut reader, "EmbeddedFonts6.face_name")?;
        let style_name =
            read_utf16le_len_prefixed(&mut reader, "EmbeddedFonts6.style_name")?;
        // Bold and italic bytes are only present when style_name is non-empty.
        // When style_name is empty (byte_len == 2, just NUL), they are omitted.
        let (bold, italic) = if !style_name.is_empty() {
            let b = reader.read_u8()? != 0;
            let i = reader.read_u8()? != 0;
            (Some(b), Some(i))
        } else {
            (None, None)
        };
        let charset = reader.read_u8()?;
        let blob_size = reader.read_u32_le()? as usize;
        let blob = reader.read_bytes(blob_size)?;
        entries.push(PcbEmbeddedFontEntry {
            full_name,
            face_name,
            style_name,
            bold,
            italic,
            charset,
            data: blob.to_vec(),
        });
        if idx + 1 == expected_count {
            break;
        }
    }
    reader.assert_exhausted()?;
    Ok(entries)
}

/// Parses DrillManager Data stream.
///
/// Format: `i32(-1)` sentinel + `u32(count)` + N records + `u32(0)` trailing.
/// Each record: `u32(text_len)` + NUL-terminated params + `u32(pad_count)` +
/// pad_indices + `u32(via_count)` + via_indices.
fn parse_drill_manager_data(data: &[u8]) -> Result<Vec<DrillManagerRecord>> {
    use crate::param_collection::ParameterCollection;

    let mut reader = BinaryReader::new(data);

    let sentinel = reader.read_i32_le()?;
    if sentinel != -1 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "DrillManager/Data".to_owned(),
            detail: format!("expected sentinel -1, got {sentinel}"),
        });
    }

    let count = reader.read_u32_le()? as usize;
    let mut records = Vec::with_capacity(count);

    for i in 0..count {
        let text_len = reader.read_u32_le()? as usize;
        let text_bytes = reader.read_bytes(text_len)?;
        let params = ParameterCollection::from_bytes(text_bytes)
            .with_context(|| format!("DrillManager record {i}"))?;

        let pad_count = reader.read_u32_le()? as usize;
        let mut pad_indices = Vec::with_capacity(pad_count);
        for _ in 0..pad_count {
            pad_indices.push(reader.read_u32_le()?);
        }

        let via_count = reader.read_u32_le()? as usize;
        let mut via_indices = Vec::with_capacity(via_count);
        for _ in 0..via_count {
            via_indices.push(reader.read_u32_le()?);
        }

        records.push(DrillManagerRecord {
            params,
            pad_indices,
            via_indices,
        });
    }

    // Trailing u32(0) — observed as PairIndex or reserved field
    let trailing = reader.read_u32_le()?;
    if trailing != 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "DrillManager/Data".to_owned(),
            detail: format!("expected trailing 0, got {trailing}"),
        });
    }

    reader.assert_exhausted().context("DrillManager/Data")?;
    Ok(records)
}

fn decode_constraint_manager_data(data: &[u8]) -> Result<String> {
    use base64::Engine;
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    let mut blocks = crate::block_stream::iter_blocks(data);
    let block = match blocks.next() {
        Some(Ok(b)) => b,
        Some(Err(e)) => return Err(e),
        None => return Ok(String::new()),
    };
    if let Some(extra) = blocks.next() {
        let _ = extra?;
        return Err(AltiumFormatError::InvalidParamValue {
            key: "ConstraintManager/Data".to_owned(),
            detail: "expected single block".to_owned(),
        });
    }

    let (base64_cow, _, had_errors) = encoding_rs::UTF_16LE.decode(&block.data);
    if had_errors {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "ConstraintManager/Data".to_owned(),
            detail: "invalid UTF-16LE encoding".to_owned(),
        });
    }
    let base64_str = base64_cow.trim_end_matches('\0');
    if base64_str.is_empty() {
        return Ok(String::new());
    }

    let compressed = base64::engine::general_purpose::STANDARD
        .decode(base64_str)
        .map_err(|e| AltiumFormatError::InvalidParamValue {
            key: "ConstraintManager/Data".to_owned(),
            detail: format!("base64 decode failed: {e}"),
        })?;

    let mut decoder = ZlibDecoder::new(&compressed[..]);
    let mut xml_bytes = Vec::new();
    decoder.read_to_end(&mut xml_bytes).map_err(|e| AltiumFormatError::InvalidParamValue {
        key: "ConstraintManager/Data".to_owned(),
        detail: format!("zlib decompress failed: {e}"),
    })?;

    String::from_utf8(xml_bytes).map_err(|e| AltiumFormatError::InvalidParamValue {
        key: "ConstraintManager/Data".to_owned(),
        detail: format!("XML is not valid UTF-8: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "proptest")]
    use super::*;
    #[cfg(feature = "proptest")]
    use proptest::prelude::*;
    #[cfg(feature = "proptest")]
    use std::fs;

    #[cfg(feature = "proptest")]
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
            if is_pcbdoc && is_cfb_file(&path) {
                out.push(path);
            }
        }
        out.sort();
        out
    }

    #[cfg(feature = "proptest")]
    fn is_cfb_file(path: &std::path::Path) -> bool {
        const CFB_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        let Ok(bytes) = fs::read(path) else {
            return false;
        };
        if bytes.len() < 8 || bytes[..8] != CFB_MAGIC {
            return false;
        }
        let cursor = std::io::Cursor::new(bytes);
        let Ok(comp) = cfb::CompoundFile::open(cursor) else {
            return false;
        };
        comp.exists("/FileHeaderSix")
    }

    #[cfg(feature = "proptest")]
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

//! PcbLib document I/O using the v2 DocumentStore architecture.
//!
//! A PcbLib file is a CFB compound file with one storage per footprint:
//! - `/<FootprintName>/Parameters` stream: footprint metadata (pipe-delimited)
//! - `/<FootprintName>/Header` stream: primitive count and version info
//! - `/<FootprintName>/Data` stream: binary primitives (type byte + length + data)
//!
//! The Data stream begins with a length-prefixed pattern name block, followed
//! by packed binary primitive records.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::{Cursor, Read, Seek, Write};
use std::rc::Rc;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::error::{AltiumError, Result};
use crate::v2::backing_store::{ParamOrigin, PcbPrimitiveRef, RecordNode, RecordOrigin};
use crate::v2::handles::PcbFootprintHandle;
use crate::v2::ids::RecordId;
use crate::v2::records::{
    parse_arc, parse_component_body, parse_fill, parse_pad, parse_region, parse_text, parse_track,
    parse_via,
};
use crate::v2::store::{DocRef, DocumentMeta, DocumentStore, GroupData, GroupMeta};
use crate::v2::traits::{DocumentQuery, HandleFamily};

use super::pcblib_streams::{
    PcbLibCountedDataStreamMeta, PcbLibFileHeaderStreamMeta, PcbLibLibraryStorageMeta,
    PcbLibFootprintSidecarStreamsMeta, PcbLibModelsStorageMeta, parse_file_header_stream,
    parse_param_table_stream, parse_primitive_guids_stream, parse_section_keys_stream,
    parse_u32_header_stream, parse_wide_strings_stream, serialize_param_table_stream,
    serialize_primitive_guids_stream, serialize_section_keys_stream,
    serialize_u32_header_stream, serialize_wide_strings_stream,
};
use super::section_keys::SectionKeyList;

const STREAM_PARAMETERS: &str = "Parameters";
const STREAM_HEADER: &str = "Header";
const STREAM_DATA: &str = "Data";
const STREAM_WIDE_STRINGS: &str = "WideStrings";
const STREAM_PRIMITIVE_GUIDS: &str = "PrimitiveGuids";
const STREAM_UNIQUE_ID_PRIMITIVE_INFORMATION: &str = "UniqueIDPrimitiveInformation";
const STREAM_EXTENDED_PRIMITIVE_INFORMATION: &str = "ExtendedPrimitiveInformation";
const TOP_LEVEL_SYSTEM_STORAGES: &[&str] =
    &["SectionKeys", "FileHeader", "Library", "FileVersionInfo"];

/// A parsed PcbLib document using the v2 DocumentStore architecture.
///
/// All records and groups are stored in a centralized `DocumentStore` accessed
/// via `Rc<RefCell<>>` handles. The `store()` method provides access for
/// reading and writing footprint data through typed handles.
pub struct PcbLib {
    store: DocRef,
}

impl PcbLib {
    /// Create a new empty PcbLib document.
    pub fn new_empty() -> Self {
        let mut store = DocumentStore::new(DocumentMeta::PcbLib {
            section_keys: SectionKeyList::new(),
            file_header_meta: PcbLibFileHeaderStreamMeta::default(),
            file_version_info_meta: PcbLibCountedDataStreamMeta {
                header_count: 1,
                data: Vec::new(),
            },
            library_meta: PcbLibLibraryStorageMeta::default(),
        });
        store.set_semantic_context("dtid:pcblib", "");
        Self {
            store: Rc::new(RefCell::new(store)),
        }
    }

    /// Returns a reference to the underlying document store.
    pub fn store(&self) -> &DocRef {
        &self.store
    }

    /// Returns typed `/SectionKeys` mapping metadata.
    pub fn section_keys(&self) -> SectionKeyList {
        let store = self.store.borrow();
        match store.meta() {
            DocumentMeta::PcbLib { section_keys, .. } => section_keys.clone(),
            _ => SectionKeyList::new(),
        }
    }

    /// Replace typed `/SectionKeys` mapping metadata.
    pub fn set_section_keys(&self, keys: SectionKeyList) {
        let mut store = self.store.borrow_mut();
        if let DocumentMeta::PcbLib { section_keys, .. } = &mut store.meta {
            *section_keys = keys;
            store.mark_semantic_ids_dirty();
        }
    }

    /// Returns typed `/FileHeader` metadata.
    pub fn file_header_meta(&self) -> PcbLibFileHeaderStreamMeta {
        let store = self.store.borrow();
        match store.meta() {
            DocumentMeta::PcbLib {
                file_header_meta, ..
            } => file_header_meta.clone(),
            _ => PcbLibFileHeaderStreamMeta::default(),
        }
    }

    /// Replace typed `/FileHeader` metadata.
    pub fn set_file_header_meta(&self, meta: PcbLibFileHeaderStreamMeta) {
        let mut store = self.store.borrow_mut();
        if let DocumentMeta::PcbLib {
            file_header_meta, ..
        } = &mut store.meta
        {
            *file_header_meta = meta;
            store.mark_semantic_ids_dirty();
        }
    }

    /// Returns typed `/FileVersionInfo/{Header,Data}` metadata.
    pub fn file_version_info_meta(&self) -> PcbLibCountedDataStreamMeta {
        let store = self.store.borrow();
        match store.meta() {
            DocumentMeta::PcbLib {
                file_version_info_meta,
                ..
            } => file_version_info_meta.clone(),
            _ => PcbLibCountedDataStreamMeta::default(),
        }
    }

    /// Replace typed `/FileVersionInfo/{Header,Data}` metadata.
    pub fn set_file_version_info_meta(&self, meta: PcbLibCountedDataStreamMeta) {
        let mut store = self.store.borrow_mut();
        if let DocumentMeta::PcbLib {
            file_version_info_meta,
            ..
        } = &mut store.meta
        {
            *file_version_info_meta = meta;
            store.mark_semantic_ids_dirty();
        }
    }

    /// Returns typed `/Library/*` metadata.
    pub fn library_meta(&self) -> PcbLibLibraryStorageMeta {
        let store = self.store.borrow();
        match store.meta() {
            DocumentMeta::PcbLib { library_meta, .. } => library_meta.clone(),
            _ => PcbLibLibraryStorageMeta::default(),
        }
    }

    /// Replace typed `/Library/*` metadata.
    pub fn set_library_meta(&self, meta: PcbLibLibraryStorageMeta) {
        let mut store = self.store.borrow_mut();
        if let DocumentMeta::PcbLib { library_meta, .. } = &mut store.meta {
            *library_meta = meta;
            store.mark_semantic_ids_dirty();
        }
    }

    /// Returns the stable document-level semantic ID, if computed.
    pub fn document_id(&self) -> Option<crate::v2::semantic_ids::SemanticId> {
        let mut store = self.store.borrow_mut();
        store.ensure_semantic_ids();
        store.document_id().cloned()
    }

    /// Open a PcbLib from a reader.
    pub fn open<R: Read + Seek>(mut reader: R) -> Result<Self> {
        let mut raw_bytes = Vec::new();
        reader
            .read_to_end(&mut raw_bytes)
            .map_err(AltiumError::Io)?;
        let doc_key = crate::v2::semantic_ids::blake3_content_hash(&raw_bytes);

        let mut cfb = cfb::CompoundFile::open(Cursor::new(raw_bytes))
            .map_err(|e| AltiumError::Cfb(format!("Failed to open CFB: {}", e)))?;

        // Enumerate top-level storages to find footprints.
        // A storage is only treated as a footprint if it has a Data stream.
        let candidate_entries: Vec<String> = cfb
            .walk()
            .filter(|e| {
                e.is_storage()
                    && e.path()
                        .parent()
                        .map_or(false, |p| p == std::path::Path::new("/"))
            })
            .filter_map(|e| {
                let name = e.path().file_name()?.to_str()?.to_string();
                if TOP_LEVEL_SYSTEM_STORAGES
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(&name))
                {
                    return None;
                }
                Some(name)
            })
            .collect();

        // Only keep storages that have a Data stream.
        let entries: Vec<String> = candidate_entries
            .into_iter()
            .filter(|name| {
                let data_path = format!("/{}/{}", name, STREAM_DATA);
                cfb.open_stream(&data_path).is_ok()
            })
            .collect();

        // Collect all stream paths upfront to avoid re-borrowing cfb.
        let all_stream_paths: Vec<String> = cfb
            .walk()
            .filter(|e| e.is_stream())
            .filter_map(|e| Some(e.path().to_str()?.to_string()))
            .collect();

        let footprint_set: std::collections::HashSet<String> =
            entries.iter().map(|s| s.to_ascii_lowercase()).collect();

        let mut path_file_header: Option<String> = None;
        let mut path_section_keys: Option<String> = None;

        let mut path_fvi_header: Option<String> = None;
        let mut path_fvi_data: Option<String> = None;

        let mut path_lib_header: Option<String> = None;
        let mut path_lib_data: Option<String> = None;
        let mut path_lib_embedded_fonts: Option<String> = None;

        let mut path_lib_cptoc_header: Option<String> = None;
        let mut path_lib_cptoc_data: Option<String> = None;

        let mut path_lib_layer_kind_header: Option<String> = None;
        let mut path_lib_layer_kind_data: Option<String> = None;

        let mut path_lib_models_header: Option<String> = None;
        let mut path_lib_models_data: Option<String> = None;
        let mut path_lib_models_entries: BTreeMap<u32, String> = BTreeMap::new();

        let mut path_lib_models_no_embed_header: Option<String> = None;
        let mut path_lib_models_no_embed_data: Option<String> = None;

        let mut path_lib_pad_via_header: Option<String> = None;
        let mut path_lib_pad_via_data: Option<String> = None;

        let mut path_lib_textures_header: Option<String> = None;
        let mut path_lib_textures_data: Option<String> = None;

        for stream_path in &all_stream_paths {
            let path_no_slash = stream_path.trim_start_matches('/');
            let mut parts = path_no_slash.split('/');
            let root = parts.next().unwrap_or("");
            let second = parts.next();
            let third = parts.next();
            let fourth = parts.next();

            if footprint_set.contains(&root.to_ascii_lowercase()) {
                continue;
            }

            match root.to_ascii_lowercase().as_str() {
                "fileheader" => {
                    if second.is_some() {
                        return Err(AltiumError::Parse(format!(
                            "pcblib FileHeader stream must be top-level, got '{}'",
                            stream_path
                        )));
                    }
                    if path_file_header.is_some() {
                        return Err(AltiumError::Parse(
                            "pcblib contains duplicate FileHeader stream".to_string(),
                        ));
                    }
                    path_file_header = Some(stream_path.clone());
                }
                "sectionkeys" => {
                    if second.is_some() {
                        return Err(AltiumError::Parse(format!(
                            "pcblib SectionKeys stream must be top-level, got '{}'",
                            stream_path
                        )));
                    }
                    if path_section_keys.is_some() {
                        return Err(AltiumError::Parse(
                            "pcblib contains duplicate SectionKeys stream".to_string(),
                        ));
                    }
                    path_section_keys = Some(stream_path.clone());
                }
                "fileversioninfo" => {
                    if fourth.is_some() || third.is_some() || second.is_none() {
                        return Err(AltiumError::Parse(format!(
                            "pcblib contains unimplemented FileVersionInfo stream '{}'",
                            stream_path
                        )));
                    }
                    match second.unwrap_or_default().to_ascii_lowercase().as_str() {
                        "header" => {
                            if path_fvi_header.is_some() {
                                return Err(AltiumError::Parse(
                                    "pcblib contains duplicate FileVersionInfo/Header stream"
                                        .to_string(),
                                ));
                            }
                            path_fvi_header = Some(stream_path.clone());
                        }
                        "data" => {
                            if path_fvi_data.is_some() {
                                return Err(AltiumError::Parse(
                                    "pcblib contains duplicate FileVersionInfo/Data stream"
                                        .to_string(),
                                ));
                            }
                            path_fvi_data = Some(stream_path.clone());
                        }
                        _ => {
                            return Err(AltiumError::Parse(format!(
                                "pcblib contains unimplemented FileVersionInfo stream '{}'",
                                stream_path
                            )));
                        }
                    }
                }
                "library" => {
                    if second.is_none() || fourth.is_some() {
                        return Err(AltiumError::Parse(format!(
                            "pcblib contains unimplemented Library stream '{}'",
                            stream_path
                        )));
                    }
                    let second_lc = second.unwrap_or_default().to_ascii_lowercase();
                    match (second_lc.as_str(), third.map(|s| s.to_ascii_lowercase())) {
                        ("header", None) => {
                            if path_lib_header.is_some() {
                                return Err(AltiumError::Parse(
                                    "pcblib contains duplicate Library/Header stream".to_string(),
                                ));
                            }
                            path_lib_header = Some(stream_path.clone());
                        }
                        ("data", None) => {
                            if path_lib_data.is_some() {
                                return Err(AltiumError::Parse(
                                    "pcblib contains duplicate Library/Data stream".to_string(),
                                ));
                            }
                            path_lib_data = Some(stream_path.clone());
                        }
                        ("embeddedfonts", None) => {
                            if path_lib_embedded_fonts.is_some() {
                                return Err(AltiumError::Parse(
                                    "pcblib contains duplicate Library/EmbeddedFonts stream"
                                        .to_string(),
                                ));
                            }
                            path_lib_embedded_fonts = Some(stream_path.clone());
                        }
                        ("componentparamstoc", Some(leaf)) => match leaf.as_str() {
                            "header" => {
                                if path_lib_cptoc_header.is_some() {
                                    return Err(AltiumError::Parse(
                                        "pcblib contains duplicate Library/ComponentParamsTOC/Header stream"
                                            .to_string(),
                                    ));
                                }
                                path_lib_cptoc_header = Some(stream_path.clone());
                            }
                            "data" => {
                                if path_lib_cptoc_data.is_some() {
                                    return Err(AltiumError::Parse(
                                        "pcblib contains duplicate Library/ComponentParamsTOC/Data stream"
                                            .to_string(),
                                    ));
                                }
                                path_lib_cptoc_data = Some(stream_path.clone());
                            }
                            _ => {
                                return Err(AltiumError::Parse(format!(
                                    "pcblib contains unimplemented Library stream '{}'",
                                    stream_path
                                )));
                            }
                        },
                        ("layerkindmapping", Some(leaf)) => match leaf.as_str() {
                            "header" => {
                                if path_lib_layer_kind_header.is_some() {
                                    return Err(AltiumError::Parse(
                                        "pcblib contains duplicate Library/LayerKindMapping/Header stream"
                                            .to_string(),
                                    ));
                                }
                                path_lib_layer_kind_header = Some(stream_path.clone());
                            }
                            "data" => {
                                if path_lib_layer_kind_data.is_some() {
                                    return Err(AltiumError::Parse(
                                        "pcblib contains duplicate Library/LayerKindMapping/Data stream"
                                            .to_string(),
                                    ));
                                }
                                path_lib_layer_kind_data = Some(stream_path.clone());
                            }
                            _ => {
                                return Err(AltiumError::Parse(format!(
                                    "pcblib contains unimplemented Library stream '{}'",
                                    stream_path
                                )));
                            }
                        },
                        ("models", Some(leaf)) => {
                            if leaf == "header" {
                                if path_lib_models_header.is_some() {
                                    return Err(AltiumError::Parse(
                                        "pcblib contains duplicate Library/Models/Header stream"
                                            .to_string(),
                                    ));
                                }
                                path_lib_models_header = Some(stream_path.clone());
                            } else if leaf == "data" {
                                if path_lib_models_data.is_some() {
                                    return Err(AltiumError::Parse(
                                        "pcblib contains duplicate Library/Models/Data stream"
                                            .to_string(),
                                    ));
                                }
                                path_lib_models_data = Some(stream_path.clone());
                            } else if let Ok(model_index) = leaf.parse::<u32>() {
                                if path_lib_models_entries.contains_key(&model_index) {
                                    return Err(AltiumError::Parse(format!(
                                        "pcblib contains duplicate Library/Models/{} stream",
                                        model_index
                                    )));
                                }
                                path_lib_models_entries.insert(model_index, stream_path.clone());
                            } else {
                                return Err(AltiumError::Parse(format!(
                                    "pcblib contains unimplemented Library/Models stream '{}'",
                                    stream_path
                                )));
                            }
                        }
                        ("modelsnoembed", Some(leaf)) => match leaf.as_str() {
                            "header" => {
                                if path_lib_models_no_embed_header.is_some() {
                                    return Err(AltiumError::Parse(
                                        "pcblib contains duplicate Library/ModelsNoEmbed/Header stream"
                                            .to_string(),
                                    ));
                                }
                                path_lib_models_no_embed_header = Some(stream_path.clone());
                            }
                            "data" => {
                                if path_lib_models_no_embed_data.is_some() {
                                    return Err(AltiumError::Parse(
                                        "pcblib contains duplicate Library/ModelsNoEmbed/Data stream"
                                            .to_string(),
                                    ));
                                }
                                path_lib_models_no_embed_data = Some(stream_path.clone());
                            }
                            _ => {
                                return Err(AltiumError::Parse(format!(
                                    "pcblib contains unimplemented Library stream '{}'",
                                    stream_path
                                )));
                            }
                        },
                        ("padvialibrary", Some(leaf)) => match leaf.as_str() {
                            "header" => {
                                if path_lib_pad_via_header.is_some() {
                                    return Err(AltiumError::Parse(
                                        "pcblib contains duplicate Library/PadViaLibrary/Header stream"
                                            .to_string(),
                                    ));
                                }
                                path_lib_pad_via_header = Some(stream_path.clone());
                            }
                            "data" => {
                                if path_lib_pad_via_data.is_some() {
                                    return Err(AltiumError::Parse(
                                        "pcblib contains duplicate Library/PadViaLibrary/Data stream"
                                            .to_string(),
                                    ));
                                }
                                path_lib_pad_via_data = Some(stream_path.clone());
                            }
                            _ => {
                                return Err(AltiumError::Parse(format!(
                                    "pcblib contains unimplemented Library stream '{}'",
                                    stream_path
                                )));
                            }
                        },
                        ("textures", Some(leaf)) => match leaf.as_str() {
                            "header" => {
                                if path_lib_textures_header.is_some() {
                                    return Err(AltiumError::Parse(
                                        "pcblib contains duplicate Library/Textures/Header stream"
                                            .to_string(),
                                    ));
                                }
                                path_lib_textures_header = Some(stream_path.clone());
                            }
                            "data" => {
                                if path_lib_textures_data.is_some() {
                                    return Err(AltiumError::Parse(
                                        "pcblib contains duplicate Library/Textures/Data stream"
                                            .to_string(),
                                    ));
                                }
                                path_lib_textures_data = Some(stream_path.clone());
                            }
                            _ => {
                                return Err(AltiumError::Parse(format!(
                                    "pcblib contains unimplemented Library stream '{}'",
                                    stream_path
                                )));
                            }
                        },
                        _ => {
                            return Err(AltiumError::Parse(format!(
                                "pcblib contains unimplemented Library stream '{}'",
                                stream_path
                            )));
                        }
                    }
                }
                _ => {
                    return Err(AltiumError::Parse(format!(
                        "pcblib contains unimplemented stream '{}'",
                        stream_path
                    )));
                }
            }
        }

        let section_keys = if let Some(path) = path_section_keys {
            parse_section_keys_stream(&read_stream_bytes(&mut cfb, &path)?)?
        } else {
            SectionKeyList::new()
        };

        let file_header_path = path_file_header.ok_or_else(|| {
            AltiumError::Parse("pcblib missing required '/FileHeader' stream".to_string())
        })?;
        let file_header_meta = parse_file_header_stream(&read_stream_bytes(&mut cfb, &file_header_path)?)?;

        let fvi_header_path = path_fvi_header.ok_or_else(|| {
            AltiumError::Parse("pcblib missing required '/FileVersionInfo/Header' stream".to_string())
        })?;
        let fvi_data_path = path_fvi_data.ok_or_else(|| {
            AltiumError::Parse("pcblib missing required '/FileVersionInfo/Data' stream".to_string())
        })?;
        let file_version_info_meta = PcbLibCountedDataStreamMeta {
            header_count: parse_u32_header_stream(
                &read_stream_bytes(&mut cfb, &fvi_header_path)?,
                "FileVersionInfo/Header",
            )?,
            data: read_stream_bytes(&mut cfb, &fvi_data_path)?,
        };

        let lib_header_path = path_lib_header.ok_or_else(|| {
            AltiumError::Parse("pcblib missing required '/Library/Header' stream".to_string())
        })?;
        let lib_data_path = path_lib_data.ok_or_else(|| {
            AltiumError::Parse("pcblib missing required '/Library/Data' stream".to_string())
        })?;
        let lib_embedded_fonts_path = path_lib_embedded_fonts.ok_or_else(|| {
            AltiumError::Parse("pcblib missing required '/Library/EmbeddedFonts' stream".to_string())
        })?;
        let lib_cptoc_header_path = path_lib_cptoc_header.ok_or_else(|| {
            AltiumError::Parse(
                "pcblib missing required '/Library/ComponentParamsTOC/Header' stream".to_string(),
            )
        })?;
        let lib_cptoc_data_path = path_lib_cptoc_data.ok_or_else(|| {
            AltiumError::Parse(
                "pcblib missing required '/Library/ComponentParamsTOC/Data' stream".to_string(),
            )
        })?;
        let lib_layer_kind_header_path = path_lib_layer_kind_header.ok_or_else(|| {
            AltiumError::Parse(
                "pcblib missing required '/Library/LayerKindMapping/Header' stream".to_string(),
            )
        })?;
        let lib_layer_kind_data_path = path_lib_layer_kind_data.ok_or_else(|| {
            AltiumError::Parse(
                "pcblib missing required '/Library/LayerKindMapping/Data' stream".to_string(),
            )
        })?;
        let lib_models_header_path = path_lib_models_header.ok_or_else(|| {
            AltiumError::Parse("pcblib missing required '/Library/Models/Header' stream".to_string())
        })?;
        let lib_models_data_path = path_lib_models_data.ok_or_else(|| {
            AltiumError::Parse("pcblib missing required '/Library/Models/Data' stream".to_string())
        })?;
        let lib_models_no_embed_header_path = path_lib_models_no_embed_header.ok_or_else(|| {
            AltiumError::Parse(
                "pcblib missing required '/Library/ModelsNoEmbed/Header' stream".to_string(),
            )
        })?;
        let lib_models_no_embed_data_path = path_lib_models_no_embed_data.ok_or_else(|| {
            AltiumError::Parse(
                "pcblib missing required '/Library/ModelsNoEmbed/Data' stream".to_string(),
            )
        })?;
        let lib_pad_via_header_path = path_lib_pad_via_header.ok_or_else(|| {
            AltiumError::Parse(
                "pcblib missing required '/Library/PadViaLibrary/Header' stream".to_string(),
            )
        })?;
        let lib_pad_via_data_path = path_lib_pad_via_data.ok_or_else(|| {
            AltiumError::Parse(
                "pcblib missing required '/Library/PadViaLibrary/Data' stream".to_string(),
            )
        })?;
        let lib_textures_header_path = path_lib_textures_header.ok_or_else(|| {
            AltiumError::Parse(
                "pcblib missing required '/Library/Textures/Header' stream".to_string(),
            )
        })?;
        let lib_textures_data_path = path_lib_textures_data.ok_or_else(|| {
            AltiumError::Parse(
                "pcblib missing required '/Library/Textures/Data' stream".to_string(),
            )
        })?;

        let models_entries = {
            let mut out: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
            for (idx, path) in path_lib_models_entries {
                out.insert(idx, read_stream_bytes(&mut cfb, &path)?);
            }
            out
        };

        let library_meta = PcbLibLibraryStorageMeta {
            header_count: parse_u32_header_stream(
                &read_stream_bytes(&mut cfb, &lib_header_path)?,
                "Library/Header",
            )?,
            data: read_stream_bytes(&mut cfb, &lib_data_path)?,
            embedded_fonts: read_stream_bytes(&mut cfb, &lib_embedded_fonts_path)?,
            component_params_toc: PcbLibCountedDataStreamMeta {
                header_count: parse_u32_header_stream(
                    &read_stream_bytes(&mut cfb, &lib_cptoc_header_path)?,
                    "Library/ComponentParamsTOC/Header",
                )?,
                data: read_stream_bytes(&mut cfb, &lib_cptoc_data_path)?,
            },
            layer_kind_mapping: PcbLibCountedDataStreamMeta {
                header_count: parse_u32_header_stream(
                    &read_stream_bytes(&mut cfb, &lib_layer_kind_header_path)?,
                    "Library/LayerKindMapping/Header",
                )?,
                data: read_stream_bytes(&mut cfb, &lib_layer_kind_data_path)?,
            },
            models: PcbLibModelsStorageMeta {
                header_count: parse_u32_header_stream(
                    &read_stream_bytes(&mut cfb, &lib_models_header_path)?,
                    "Library/Models/Header",
                )?,
                data: read_stream_bytes(&mut cfb, &lib_models_data_path)?,
                entries: models_entries,
            },
            models_no_embed: PcbLibCountedDataStreamMeta {
                header_count: parse_u32_header_stream(
                    &read_stream_bytes(&mut cfb, &lib_models_no_embed_header_path)?,
                    "Library/ModelsNoEmbed/Header",
                )?,
                data: read_stream_bytes(&mut cfb, &lib_models_no_embed_data_path)?,
            },
            pad_via_library: PcbLibCountedDataStreamMeta {
                header_count: parse_u32_header_stream(
                    &read_stream_bytes(&mut cfb, &lib_pad_via_header_path)?,
                    "Library/PadViaLibrary/Header",
                )?,
                data: read_stream_bytes(&mut cfb, &lib_pad_via_data_path)?,
            },
            textures: PcbLibCountedDataStreamMeta {
                header_count: parse_u32_header_stream(
                    &read_stream_bytes(&mut cfb, &lib_textures_header_path)?,
                    "Library/Textures/Header",
                )?,
                data: read_stream_bytes(&mut cfb, &lib_textures_data_path)?,
            },
        };

        let doc_meta = DocumentMeta::PcbLib {
            section_keys,
            file_header_meta,
            file_version_info_meta,
            library_meta,
        };
        let mut store = DocumentStore::new(doc_meta);

        for storage_name in &entries {
            // Read Parameters stream (footprint metadata)
            let params_path = format!("/{}/{}", storage_name, STREAM_PARAMETERS);
            let metadata_node = if let Ok(mut stream) = cfb.open_stream(&params_path) {
                let mut data = Vec::new();
                stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
                let mut payload =
                    super::encoding::parse_first_param_block(&data).unwrap_or_else(|| data.clone());
                // Strip trailing NUL terminators so they don't leak into
                // parameter values (encode_single_param_block re-adds the NUL).
                while payload.last() == Some(&0) {
                    payload.pop();
                }
                let param_str = super::encoding::decode_win1252(&payload);
                RecordNode::new(0, RecordOrigin::Param(ParamOrigin::new(&param_str)))
            } else {
                RecordNode::new(0, RecordOrigin::Param(ParamOrigin::new("|PATTERN=|")))
            };

            // Read Header stream
            let header_path = format!("/{}/{}", storage_name, STREAM_HEADER);
            let raw_header = if let Ok(mut stream) = cfb.open_stream(&header_path) {
                let mut data = Vec::new();
                stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
                data
            } else {
                Vec::new()
            };

            // Read Data stream (pattern name block + binary primitives)
            let data_path = format!("/{}/{}", storage_name, STREAM_DATA);
            let (primitives, primitive_order, raw_pattern_name_block) =
                if let Ok(mut stream) = cfb.open_stream(&data_path) {
                    let mut data = Vec::new();
                    stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
                    parse_pcb_data_stream(&data)?
                } else {
                    (Vec::new(), Vec::new(), Vec::new())
                };

            // Parse typed sidecar streams in this footprint storage.
            let storage_prefix = format!("{}/", storage_name);
            let mut sidecar_streams = PcbLibFootprintSidecarStreamsMeta::default();
            let mut path_primitive_guids_header: Option<String> = None;
            let mut path_primitive_guids_data: Option<String> = None;
            let mut path_unique_id_header: Option<String> = None;
            let mut path_unique_id_data: Option<String> = None;
            let mut path_extended_info_header: Option<String> = None;
            let mut path_extended_info_data: Option<String> = None;
            for stream_path in &all_stream_paths {
                let normalized_path = stream_path.trim_start_matches('/');
                if let Some(rest) = normalized_path.strip_prefix(&storage_prefix) {
                    if rest.eq_ignore_ascii_case(STREAM_PARAMETERS)
                        || rest.eq_ignore_ascii_case(STREAM_HEADER)
                        || rest.eq_ignore_ascii_case(STREAM_DATA)
                    {
                        continue;
                    }

                    let rest_lc = rest.to_ascii_lowercase();
                    match rest_lc.as_str() {
                        "widestrings" => {
                            if sidecar_streams.wide_strings.is_some() {
                                return Err(AltiumError::Parse(format!(
                                    "pcblib contains duplicate stream '{}'",
                                    stream_path
                                )));
                            }
                            let data = read_stream_bytes(&mut cfb, stream_path)?;
                            sidecar_streams.wide_strings =
                                Some(parse_wide_strings_stream(&data, stream_path)?);
                        }
                        "primitiveguids/header" => {
                            if path_primitive_guids_header.is_some() {
                                return Err(AltiumError::Parse(format!(
                                    "pcblib contains duplicate stream '{}'",
                                    stream_path
                                )));
                            }
                            path_primitive_guids_header = Some(stream_path.clone());
                        }
                        "primitiveguids/data" => {
                            if path_primitive_guids_data.is_some() {
                                return Err(AltiumError::Parse(format!(
                                    "pcblib contains duplicate stream '{}'",
                                    stream_path
                                )));
                            }
                            path_primitive_guids_data = Some(stream_path.clone());
                        }
                        "uniqueidprimitiveinformation/header" => {
                            if path_unique_id_header.is_some() {
                                return Err(AltiumError::Parse(format!(
                                    "pcblib contains duplicate stream '{}'",
                                    stream_path
                                )));
                            }
                            path_unique_id_header = Some(stream_path.clone());
                        }
                        "uniqueidprimitiveinformation/data" => {
                            if path_unique_id_data.is_some() {
                                return Err(AltiumError::Parse(format!(
                                    "pcblib contains duplicate stream '{}'",
                                    stream_path
                                )));
                            }
                            path_unique_id_data = Some(stream_path.clone());
                        }
                        "extendedprimitiveinformation/header" => {
                            if path_extended_info_header.is_some() {
                                return Err(AltiumError::Parse(format!(
                                    "pcblib contains duplicate stream '{}'",
                                    stream_path
                                )));
                            }
                            path_extended_info_header = Some(stream_path.clone());
                        }
                        "extendedprimitiveinformation/data" => {
                            if path_extended_info_data.is_some() {
                                return Err(AltiumError::Parse(format!(
                                    "pcblib contains duplicate stream '{}'",
                                    stream_path
                                )));
                            }
                            path_extended_info_data = Some(stream_path.clone());
                        }
                        _ => {
                            return Err(AltiumError::Parse(format!(
                                "pcblib contains unimplemented stream '{}'",
                                stream_path
                            )));
                        }
                    }
                }
            }

            match (path_primitive_guids_header, path_primitive_guids_data) {
                (Some(header_path), Some(data_path)) => {
                    let header_data = read_stream_bytes(&mut cfb, &header_path)?;
                    let data = read_stream_bytes(&mut cfb, &data_path)?;
                    sidecar_streams.primitive_guids = Some(parse_primitive_guids_stream(
                        &header_data,
                        &data,
                        &format!("{storage_name}/{STREAM_PRIMITIVE_GUIDS}"),
                    )?);
                }
                (None, None) => {}
                _ => {
                    return Err(AltiumError::Parse(format!(
                        "pcblib footprint '{}' has incomplete {}/{{Header,Data}} streams",
                        storage_name, STREAM_PRIMITIVE_GUIDS
                    )));
                }
            }

            match (path_unique_id_header, path_unique_id_data) {
                (Some(header_path), Some(data_path)) => {
                    let header_data = read_stream_bytes(&mut cfb, &header_path)?;
                    let data = read_stream_bytes(&mut cfb, &data_path)?;
                    sidecar_streams.unique_id_primitive_information = Some(parse_param_table_stream(
                        &header_data,
                        &data,
                        &format!("{storage_name}/{STREAM_UNIQUE_ID_PRIMITIVE_INFORMATION}"),
                    )?);
                }
                (None, None) => {}
                _ => {
                    return Err(AltiumError::Parse(format!(
                        "pcblib footprint '{}' has incomplete {}/{{Header,Data}} streams",
                        storage_name, STREAM_UNIQUE_ID_PRIMITIVE_INFORMATION
                    )));
                }
            }

            match (path_extended_info_header, path_extended_info_data) {
                (Some(header_path), Some(data_path)) => {
                    let header_data = read_stream_bytes(&mut cfb, &header_path)?;
                    let data = read_stream_bytes(&mut cfb, &data_path)?;
                    sidecar_streams.extended_primitive_information = Some(parse_param_table_stream(
                        &header_data,
                        &data,
                        &format!("{storage_name}/{STREAM_EXTENDED_PRIMITIVE_INFORMATION}"),
                    )?);
                }
                (None, None) => {}
                _ => {
                    return Err(AltiumError::Parse(format!(
                        "pcblib footprint '{}' has incomplete {}/{{Header,Data}} streams",
                        storage_name, STREAM_EXTENDED_PRIMITIVE_INFORMATION
                    )));
                }
            }

            // Insert metadata record into store
            let parent_id = store.insert_record(metadata_node);

            // Insert primitive records into store
            let mut child_ids: Vec<RecordId> = Vec::with_capacity(primitives.len());
            for prim_node in primitives {
                let id = store.insert_record(prim_node);
                child_ids.push(id);
            }

            // Build original_indices parallel to primitive_order (index within children vec)
            let original_indices: Vec<usize> = primitive_order.iter().map(|r| r.index).collect();

            let group_data = GroupData {
                parent: parent_id,
                children: child_ids,
                original_indices,
                parent_original_index: None,
                meta: GroupMeta::PcbFootprint {
                    name: storage_name.clone(),
                    raw_pattern_name_block,
                    original_primitive_order: primitive_order,
                    raw_header,
                    sidecar_streams,
                },
            };
            store.insert_group(group_data);
        }

        crate::v2::semantic_ids::compute_all_ids(&mut store, "dtid:pcblib", &doc_key);

        Ok(PcbLib {
            store: Rc::new(RefCell::new(store)),
        })
    }

    /// Open a PcbLib from a file path.
    pub fn open_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let file = std::fs::File::open(path).map_err(AltiumError::Io)?;
        Self::open(file)
    }

    /// Save a PcbLib to a writer.
    pub fn save<W: Read + Write + Seek>(&self, writer: W) -> Result<()> {
        let mut cfb = cfb::CompoundFile::create_with_version(cfb::Version::V3, writer)
            .map_err(|e| AltiumError::Cfb(format!("Failed to create CFB: {}", e)))?;

        let mut store = self.store.borrow_mut();
        store.ensure_semantic_ids();

        let (section_keys, file_header_meta, file_version_info_meta, library_meta) =
            match store.meta() {
                DocumentMeta::PcbLib {
                    section_keys,
                    file_header_meta,
                    file_version_info_meta,
                    library_meta,
                } => (
                    section_keys.clone(),
                    file_header_meta.clone(),
                    file_version_info_meta.clone(),
                    library_meta.clone(),
                ),
                _ => return Err(AltiumError::Cfb("Expected PcbLib metadata".to_string())),
            };

        // Write typed non-footprint system streams.
        {
            let mut created_storages: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            let mut write_system_stream = |rel_path: &str, data: &[u8]| -> Result<()> {
                let full_path = format!("/{}", rel_path);
                ensure_parent_storages(&mut cfb, &full_path, &mut created_storages)?;
                let mut stream = cfb.create_stream(&full_path).map_err(|e| {
                    AltiumError::Cfb(format!(
                        "Failed to create system stream {}: {}",
                        full_path, e
                    ))
                })?;
                stream.write_all(data).map_err(AltiumError::Io)?;
                Ok(())
            };

            if !section_keys.is_empty() {
                let bytes = serialize_section_keys_stream(&section_keys)?;
                write_system_stream("SectionKeys", &bytes)?;
            }

            let file_header_bytes = file_header_meta.to_stream_bytes()?;
            write_system_stream("FileHeader", &file_header_bytes)?;

            let fvi_header = serialize_u32_header_stream(file_version_info_meta.header_count);
            write_system_stream("FileVersionInfo/Header", &fvi_header)?;
            write_system_stream("FileVersionInfo/Data", &file_version_info_meta.data)?;

            let lib_header = serialize_u32_header_stream(library_meta.header_count);
            write_system_stream("Library/Header", &lib_header)?;
            write_system_stream("Library/Data", &library_meta.data)?;
            write_system_stream("Library/EmbeddedFonts", &library_meta.embedded_fonts)?;

            let toc_header = serialize_u32_header_stream(library_meta.component_params_toc.header_count);
            write_system_stream("Library/ComponentParamsTOC/Header", &toc_header)?;
            write_system_stream("Library/ComponentParamsTOC/Data", &library_meta.component_params_toc.data)?;

            let layer_header = serialize_u32_header_stream(library_meta.layer_kind_mapping.header_count);
            write_system_stream("Library/LayerKindMapping/Header", &layer_header)?;
            write_system_stream("Library/LayerKindMapping/Data", &library_meta.layer_kind_mapping.data)?;

            let models_header = serialize_u32_header_stream(library_meta.models.header_count);
            write_system_stream("Library/Models/Header", &models_header)?;
            write_system_stream("Library/Models/Data", &library_meta.models.data)?;
            for (index, blob) in &library_meta.models.entries {
                write_system_stream(&format!("Library/Models/{}", index), blob)?;
            }

            let models_no_embed_header =
                serialize_u32_header_stream(library_meta.models_no_embed.header_count);
            write_system_stream("Library/ModelsNoEmbed/Header", &models_no_embed_header)?;
            write_system_stream("Library/ModelsNoEmbed/Data", &library_meta.models_no_embed.data)?;

            let pad_via_header = serialize_u32_header_stream(library_meta.pad_via_library.header_count);
            write_system_stream("Library/PadViaLibrary/Header", &pad_via_header)?;
            write_system_stream("Library/PadViaLibrary/Data", &library_meta.pad_via_library.data)?;

            let textures_header = serialize_u32_header_stream(library_meta.textures.header_count);
            write_system_stream("Library/Textures/Header", &textures_header)?;
            write_system_stream("Library/Textures/Data", &library_meta.textures.data)?;
        }

        for &group_id in store.group_ids() {
            let group = store.group(group_id);
            let (
                name,
                raw_pattern_name_block,
                original_primitive_order,
                raw_header,
                sidecar_streams,
            ) =
                match &group.meta {
                    GroupMeta::PcbFootprint {
                        name,
                        raw_pattern_name_block,
                        original_primitive_order,
                        raw_header,
                        sidecar_streams,
                    } => (
                        name.clone(),
                        raw_pattern_name_block.clone(),
                        original_primitive_order.clone(),
                        raw_header.clone(),
                        sidecar_streams.clone(),
                    ),
                    _ => continue,
                };

            let storage_path = format!("/{}", name);
            cfb.create_storage(&storage_path)
                .map_err(|e| AltiumError::Cfb(format!("Failed to create storage: {}", e)))?;

            // Write Parameters stream
            let params_path = format!("/{}/{}", name, STREAM_PARAMETERS);
            let params_data = match &store.record(group.parent).origin {
                RecordOrigin::Param(p) => super::encoding::encode_single_param_block(&p.params),
                _ => Vec::new(),
            };
            let mut stream = cfb
                .create_stream(&params_path)
                .map_err(|e| AltiumError::Cfb(format!("Failed to create Parameters: {}", e)))?;
            stream.write_all(&params_data).map_err(AltiumError::Io)?;

            // Write Header stream
            let header_path = format!("/{}/{}", name, STREAM_HEADER);
            let mut stream = cfb
                .create_stream(&header_path)
                .map_err(|e| AltiumError::Cfb(format!("Failed to create Header: {}", e)))?;
            if raw_header.is_empty() {
                let count = group.children.len() as u32;
                stream
                    .write_all(&count.to_le_bytes())
                    .map_err(AltiumError::Io)?;
            } else {
                stream.write_all(&raw_header).map_err(AltiumError::Io)?;
            }

            // Write Data stream
            let data_path = format!("/{}/{}", name, STREAM_DATA);
            let primitives: Vec<&RecordNode> =
                group.children.iter().map(|&id| store.record(id)).collect();
            let data = build_pcb_data_stream(
                &raw_pattern_name_block,
                &original_primitive_order,
                &primitives,
            )?;
            let mut stream = cfb
                .create_stream(&data_path)
                .map_err(|e| AltiumError::Cfb(format!("Failed to create Data: {}", e)))?;
            stream.write_all(&data).map_err(AltiumError::Io)?;

            // Write typed per-footprint sidecar streams.
            {
                let mut created_storages: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                created_storages.insert(storage_path.clone());
                let mut write_sidecar = |rel_path: &str, data: &[u8]| -> Result<()> {
                    let full_path = format!("/{}/{}", name, rel_path);
                    ensure_parent_storages(&mut cfb, &full_path, &mut created_storages)?;
                    let mut stream = cfb.create_stream(&full_path).map_err(|e| {
                        AltiumError::Cfb(format!(
                            "Failed to create sidecar stream {}: {}",
                            full_path, e
                        ))
                    })?;
                    stream.write_all(data).map_err(AltiumError::Io)?;
                    Ok(())
                };

                if let Some(wide_strings) = &sidecar_streams.wide_strings {
                    let bytes = serialize_wide_strings_stream(
                        wide_strings,
                        &format!("{name}/{STREAM_WIDE_STRINGS}"),
                    )?;
                    write_sidecar(STREAM_WIDE_STRINGS, &bytes)?;
                }

                if let Some(primitive_guids) = &sidecar_streams.primitive_guids {
                    let (header_bytes, data_bytes) =
                        serialize_primitive_guids_stream(primitive_guids)?;
                    write_sidecar(&format!("{STREAM_PRIMITIVE_GUIDS}/{STREAM_HEADER}"), &header_bytes)?;
                    write_sidecar(&format!("{STREAM_PRIMITIVE_GUIDS}/{STREAM_DATA}"), &data_bytes)?;
                }

                if let Some(unique_id_info) = &sidecar_streams.unique_id_primitive_information {
                    let (header_bytes, data_bytes) = serialize_param_table_stream(
                        unique_id_info,
                        &format!("{name}/{STREAM_UNIQUE_ID_PRIMITIVE_INFORMATION}"),
                    )?;
                    write_sidecar(
                        &format!(
                            "{}/{}",
                            STREAM_UNIQUE_ID_PRIMITIVE_INFORMATION, STREAM_HEADER
                        ),
                        &header_bytes,
                    )?;
                    write_sidecar(
                        &format!(
                            "{}/{}",
                            STREAM_UNIQUE_ID_PRIMITIVE_INFORMATION, STREAM_DATA
                        ),
                        &data_bytes,
                    )?;
                }

                if let Some(extended_info) = &sidecar_streams.extended_primitive_information {
                    let (header_bytes, data_bytes) = serialize_param_table_stream(
                        extended_info,
                        &format!("{name}/{STREAM_EXTENDED_PRIMITIVE_INFORMATION}"),
                    )?;
                    write_sidecar(
                        &format!("{}/{}", STREAM_EXTENDED_PRIMITIVE_INFORMATION, STREAM_HEADER),
                        &header_bytes,
                    )?;
                    write_sidecar(
                        &format!("{}/{}", STREAM_EXTENDED_PRIMITIVE_INFORMATION, STREAM_DATA),
                        &data_bytes,
                    )?;
                }
            }
        }

        cfb.flush()
            .map_err(|e| AltiumError::Cfb(format!("CFB flush: {}", e)))?;
        Ok(())
    }

    /// Save to a file path.
    pub fn save_file(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let file = std::fs::File::create(path).map_err(AltiumError::Io)?;
        self.save(file)
    }

    /// Returns the number of footprints in the library.
    pub fn footprint_count(&self) -> usize {
        self.store.borrow().group_count()
    }

    /// Returns the footprint storage names in order.
    pub fn names(&self) -> Vec<String> {
        let store = self.store.borrow();
        store
            .group_ids()
            .iter()
            .filter_map(|&id| {
                if let GroupMeta::PcbFootprint { name, .. } = &store.group(id).meta {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Find a footprint by name (case-insensitive), returns a handle.
    pub fn find_footprint(&self, name: &str) -> Option<PcbFootprintHandle> {
        let store = self.store.borrow();
        let name_lower = name.to_lowercase();
        for &id in store.group_ids() {
            if let GroupMeta::PcbFootprint { name: fp_name, .. } = &store.group(id).meta {
                if fp_name.to_lowercase() == name_lower {
                    return Some(PcbFootprintHandle::new(self.store.clone(), id));
                }
            }
        }
        None
    }

    /// Returns a unique ID from the library (from the first footprint's UNIQUEID parameter).
    pub fn unique_id(&self) -> String {
        let store = self.store.borrow();
        for &group_id in store.group_ids() {
            let group = store.group(group_id);
            if let RecordOrigin::Param(p) = &store.record(group.parent).origin {
                if let Some(v) = p.params.get("UNIQUEID") {
                    let s = v.as_str().to_string();
                    if !s.is_empty() {
                        return s;
                    }
                }
            }
        }
        String::new()
    }

    /// Build and add a new footprint using the builder pattern.
    ///
    /// The footprint is inserted into the centralized `DocumentStore`.
    pub fn build_footprint(
        &self,
        name: &str,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut crate::v2::builders::FootprintBuilder),
    ) -> PcbFootprintHandle {
        let mut builder = crate::v2::builders::FootprintBuilder::new(template);
        build(&mut builder);
        let (metadata, primitives, primitive_refs) = builder.build();

        let mut store = self.store.borrow_mut();

        let parent_id = store.insert_record(metadata);

        let mut child_ids: Vec<RecordId> = Vec::with_capacity(primitives.len());
        for prim_node in primitives {
            let id = store.insert_record(prim_node);
            child_ids.push(id);
        }

        let original_indices: Vec<usize> = primitive_refs.iter().map(|r| r.index).collect();

        let group_data = GroupData {
            parent: parent_id,
            children: child_ids,
            original_indices,
            parent_original_index: None,
            meta: GroupMeta::PcbFootprint {
                name: name.to_string(),
                raw_pattern_name_block: Vec::new(),
                original_primitive_order: primitive_refs,
                raw_header: Vec::new(),
                sidecar_streams: PcbLibFootprintSidecarStreamsMeta::default(),
            },
        };
        let group_id = store.insert_group(group_data);
        store.mark_semantic_ids_dirty();
        PcbFootprintHandle::new(self.store.clone(), group_id)
    }
}

// ---------------------------------------------------------------------------
// DocumentQuery<PcbFootprint> for PcbLib
// ---------------------------------------------------------------------------

impl DocumentQuery<crate::v2::handles::PcbFootprint> for PcbLib {
    fn query(&self, q: &str) -> crate::error::Result<PcbFootprintHandle> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let store = self.store.borrow();
        let mut matches = Vec::new();
        for &group_id in store.group_ids() {
            let group = store.group(group_id);
            let parent_node = store.record(group.parent);
            let all = std::slice::from_ref(parent_node);
            if !evaluate(&parsed, all).is_empty() {
                matches.push(group_id);
            }
        }

        match matches.len() {
            0 => Err(crate::error::AltiumError::NoMatch(q.to_string())),
            1 => Ok(PcbFootprintHandle::new(self.store.clone(), matches[0])),
            n => Err(crate::error::AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    fn query_all(&self, q: &str) -> crate::error::Result<Vec<PcbFootprintHandle>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let store = self.store.borrow();
        let mut handles = Vec::new();
        for &group_id in store.group_ids() {
            let group = store.group(group_id);
            let parent_node = store.record(group.parent);
            let all = std::slice::from_ref(parent_node);
            if !evaluate(&parsed, all).is_empty() {
                handles.push(PcbFootprintHandle::new(self.store.clone(), group_id));
            }
        }

        Ok(handles)
    }
}

// ---------------------------------------------------------------------------
// Deep primitive queries for PcbLib
// ---------------------------------------------------------------------------

impl PcbLib {
    /// Query a single child record of type `T` across all footprint groups.
    pub fn query_child<T: HandleFamily>(&self, q: &str) -> crate::error::Result<T::Handle> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let store = self.store.borrow();
        let mut matches = Vec::new();
        for &group_id in store.group_ids() {
            let group = store.group(group_id);
            for &child_id in &group.children {
                let node = store.record(child_id);
                if node.key == T::record_id() && node.origin.is_binary() == T::is_binary() {
                    let all = std::slice::from_ref(node);
                    if !evaluate(&parsed, all).is_empty() {
                        matches.push(child_id);
                    }
                }
            }
        }

        match matches.len() {
            0 => Err(crate::error::AltiumError::NoMatch(q.to_string())),
            1 => T::try_make_handle(self.store.clone(), matches[0]),
            n => Err(crate::error::AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    /// Query all child records of type `T` across all footprint groups.
    pub fn query_all_children<T: HandleFamily>(
        &self,
        q: &str,
    ) -> crate::error::Result<Vec<T::Handle>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let store = self.store.borrow();
        let mut handles = Vec::new();
        for &group_id in store.group_ids() {
            let group = store.group(group_id);
            for &child_id in &group.children {
                let node = store.record(child_id);
                if node.key == T::record_id() && node.origin.is_binary() == T::is_binary() {
                    let all = std::slice::from_ref(node);
                    if !evaluate(&parsed, all).is_empty() {
                        handles.push(T::try_make_handle(self.store.clone(), child_id)?);
                    }
                }
            }
        }

        Ok(handles)
    }
}

// ---------------------------------------------------------------------------
// CFB storage helpers
// ---------------------------------------------------------------------------

/// Ensure all ancestor storages for a given path exist in the CFB file.
///
/// For example, given `/A/B/C/stream`, this creates `/A`, `/A/B`, and
/// `/A/B/C` if they don't already exist.
pub(crate) fn ensure_parent_storages<F: Read + Write + Seek>(
    cfb: &mut cfb::CompoundFile<F>,
    path: &str,
    created: &mut std::collections::HashSet<String>,
) -> Result<()> {
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    // Walk all ancestors (skip the final component which is the stream itself).
    let mut current = String::new();
    for part in &parts[..parts.len().saturating_sub(1)] {
        current = format!("{}/{}", current, part);
        if created.insert(current.clone()) {
            if let Err(err) = cfb.create_storage(&current) {
                if err.kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(AltiumError::Io(err));
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn read_stream_bytes<F: Read + Seek>(cfb: &mut cfb::CompoundFile<F>, path: &str) -> Result<Vec<u8>> {
    let mut stream = cfb
        .open_stream(path)
        .map_err(|e| AltiumError::Cfb(format!("Failed to open {}: {}", path, e)))?;
    let mut data = Vec::new();
    stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
    Ok(data)
}

/// Returns the number of subrecords for a given PCB primitive type.
///
/// Pad (type 2) has 6 subrecords; Text (type 5) has 2 subrecords;
/// all others have 1 subrecord.
fn subrecord_count(type_id: u8) -> usize {
    match type_id {
        2 => 6, // Pad
        5 => 2, // Text
        _ => 1,
    }
}

/// Build a `RecordOrigin` for a single-subrecord PCB primitive.
///
/// For types that have custom parse functions (Arc=1, Via=3, Track=4,
/// Fill=6, Region=11, ComponentBody=12), calls the appropriate parser to
/// populate field_spans.
fn parse_single_subrecord_origin(type_byte: u8, block_data: Vec<u8>) -> Result<RecordOrigin> {
    match type_byte {
        1 => parse_arc(&block_data),
        3 => parse_via(&block_data),
        4 => parse_track(&block_data),
        6 => parse_fill(&block_data),
        11 => parse_region(&block_data),
        12 => parse_component_body(&block_data),
        _ => Err(AltiumError::Parse(format!(
            "unimplemented PCB primitive object_id={} in single-subrecord stream",
            type_byte
        ))),
    }
}

/// Build a `RecordOrigin` for a multi-subrecord PCB primitive.
///
/// For types that have custom parse functions (Pad=2, Text=5), calls the
/// appropriate parser to populate field_spans.
fn parse_multi_subrecord_origin(type_byte: u8, block_data: Vec<u8>) -> Result<RecordOrigin> {
    match type_byte {
        2 => parse_pad(&block_data),
        5 => parse_text(&block_data),
        _ => Err(AltiumError::Parse(format!(
            "unimplemented PCB primitive object_id={} in multi-subrecord stream",
            type_byte
        ))),
    }
}

/// Parse the PCB Data stream: pattern name block + binary primitives.
///
/// Format:
/// - 4 bytes LE: pattern name length
/// - N bytes: pattern name
/// - For each primitive:
///   - 1 byte: type ID
///   - For single-subrecord types: 4 bytes LE length + data
///   - For multi-subrecord types (Pad=6, Text=2): N sequential
///     (4 bytes LE length + data) blocks stored together
fn parse_pcb_data_stream(data: &[u8]) -> Result<(Vec<RecordNode>, Vec<PcbPrimitiveRef>, Vec<u8>)> {
    let mut cursor = Cursor::new(data);
    let mut primitives = Vec::new();
    let mut primitive_order = Vec::new();

    // Read pattern name block (length-prefixed)
    let pattern_name_block = if data.is_empty() {
        Vec::new()
    } else {
        if data.len() < 4 {
            return Err(AltiumError::UnexpectedEof);
        }
        let str_len = cursor
            .read_u32::<LittleEndian>()
            .map_err(|_| AltiumError::UnexpectedEof)? as usize;
        if cursor.position() as usize + str_len > data.len() {
            return Err(AltiumError::UnexpectedEof);
        }
        let mut buf = vec![0u8; str_len];
        cursor.read_exact(&mut buf).map_err(AltiumError::Io)?;
        buf
    };

    // Read binary primitives
    while (cursor.position() as usize) < data.len() {
        let type_byte = cursor.read_u8().map_err(|_| AltiumError::UnexpectedEof)?;

        let n = subrecord_count(type_byte);

        if n == 1 {
            // Single subrecord: read u32 len + data, store data only
            let block_len = cursor
                .read_u32::<LittleEndian>()
                .map_err(|_| AltiumError::UnexpectedEof)? as usize;

            if cursor.position() as usize + block_len > data.len() {
                return Err(AltiumError::UnexpectedEof);
            }

            let mut block_data = vec![0u8; block_len];
            cursor
                .read_exact(&mut block_data)
                .map_err(AltiumError::Io)?;

            let index = primitives.len();
            let origin = parse_single_subrecord_origin(type_byte, block_data)?;
            primitives.push(RecordNode::new(type_byte, origin));
            primitive_order.push(PcbPrimitiveRef::new(type_byte, index));
        } else {
            // Multi-subrecord: read N sequential (u32 len + data) blocks,
            // store ALL bytes including u32 prefixes as one raw_block
            let start = cursor.position() as usize;
            for _ in 0..n {
                let sub_len = cursor
                    .read_u32::<LittleEndian>()
                    .map_err(|_| AltiumError::UnexpectedEof)?
                    as usize;
                if cursor.position() as usize + sub_len > data.len() {
                    return Err(AltiumError::UnexpectedEof);
                }
                cursor.set_position(cursor.position() + sub_len as u64);
            }
            let end = cursor.position() as usize;
            let block_data = data[start..end].to_vec();

            let index = primitives.len();
            let origin = parse_multi_subrecord_origin(type_byte, block_data)?;
            primitives.push(RecordNode::new(type_byte, origin));
            primitive_order.push(PcbPrimitiveRef::new(type_byte, index));
        }
    }

    Ok((primitives, primitive_order, pattern_name_block))
}

/// Build a PCB Data stream from store-level components.
///
/// Accepts the raw pattern name block, the original primitive ordering, and
/// the borrowed primitive records (indexed by position in the children vec).
fn build_pcb_data_stream(
    raw_pattern_name_block: &[u8],
    original_primitive_order: &[PcbPrimitiveRef],
    primitives: &[&RecordNode],
) -> Result<Vec<u8>> {
    let mut output = Vec::new();

    // Write pattern name block
    output
        .write_u32::<LittleEndian>(raw_pattern_name_block.len() as u32)
        .map_err(AltiumError::Io)?;
    output.extend_from_slice(raw_pattern_name_block);

    // Write primitives in original order
    for prim_ref in original_primitive_order {
        if prim_ref.index < primitives.len() {
            let prim = primitives[prim_ref.index];
            let n = subrecord_count(prim.key);

            // Get the bytes to write (from dirty origin or clean snapshot)
            let bytes: &[u8] = if prim.is_dirty() {
                match &prim.origin {
                    RecordOrigin::Binary(b) => &b.raw_block,
                    RecordOrigin::Param(_) => &[],
                }
            } else {
                &prim.original_snapshot
            };

            output.push(prim.key); // type byte

            if n == 1 {
                // Single subrecord: write u32(len) + bytes
                output
                    .write_u32::<LittleEndian>(bytes.len() as u32)
                    .map_err(AltiumError::Io)?;
                output.extend_from_slice(bytes);
            } else {
                // Multi-subrecord: bytes already contain u32 prefixes,
                // write directly after the type byte
                output.extend_from_slice(bytes);
            }
        }
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{BinaryOrigin, ParamOrigin, PcbPrimitiveRef, RecordOrigin};

    // ---------------------------------------------------------------------------
    // parse_pcb_data_stream tests
    // ---------------------------------------------------------------------------

    #[test]
    fn pcb_data_stream_roundtrip() {
        let mut data = Vec::new();
        // Pattern name: "SOT-23"
        let name = b"SOT-23";
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name);
        // A track primitive: type=4, length=49, zeros
        data.push(4); // type byte
        data.extend_from_slice(&49u32.to_le_bytes()); // length
        data.extend_from_slice(&vec![0u8; 49]); // data

        let (prims, order, pattern_name) = parse_pcb_data_stream(&data).unwrap();
        assert_eq!(pattern_name, name);
        assert_eq!(prims.len(), 1);
        assert_eq!(prims[0].key, 4);
        assert_eq!(order.len(), 1);
    }

    #[test]
    fn empty_data_stream() {
        let (prims, order, pattern_name) = parse_pcb_data_stream(&[]).unwrap();
        assert!(prims.is_empty());
        assert!(order.is_empty());
        assert!(pattern_name.is_empty());
    }

    #[test]
    fn multiple_primitives() {
        let mut data = Vec::new();
        // Pattern name: "QFP"
        let name = b"QFP";
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name);
        // Track primitive: type=4
        data.push(4);
        data.extend_from_slice(&49u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 49]);
        // Arc primitive: type=1 (single subrecord)
        data.push(1);
        data.extend_from_slice(&60u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 60]);

        let (prims, order, _) = parse_pcb_data_stream(&data).unwrap();
        assert_eq!(prims.len(), 2);
        assert_eq!(prims[0].key, 4);
        assert_eq!(prims[1].key, 1);
        assert_eq!(order.len(), 2);
        assert_eq!(order[0].type_id, 4);
        assert_eq!(order[1].type_id, 1);
    }

    #[test]
    fn known_type_parse_failure_is_error() {
        let mut data = Vec::new();
        let name = b"BAD";
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name);

        // Track payload is intentionally too short for typed parsing.
        data.push(4);
        data.extend_from_slice(&8u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 8]);

        assert!(parse_pcb_data_stream(&data).is_err());
    }

    #[test]
    fn truncated_primitive_payload_is_error() {
        let mut data = Vec::new();
        let name = b"TRUNC";
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name);

        // Unknown primitive type still must honor declared length framing.
        data.push(99);
        data.extend_from_slice(&10u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 2]);

        assert!(parse_pcb_data_stream(&data).is_err());
    }

    #[test]
    fn pad_multi_subrecord_roundtrip() {
        let mut data = Vec::new();
        // Pattern name: "PAD"
        let name = b"PAD";
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name);
        // Pad primitive: type=2 with 6 subrecords
        data.push(2);
        // Subrecords 1-4: small subrecords
        for i in 0u8..4 {
            let sub = vec![i; 2]; // 2-byte payload
            data.extend_from_slice(&(sub.len() as u32).to_le_bytes());
            data.extend_from_slice(&sub);
        }
        // Subrecord 5: core data (minimum valid size for parser = 94 bytes)
        let core = vec![0xAA; 94];
        data.extend_from_slice(&(core.len() as u32).to_le_bytes());
        data.extend_from_slice(&core);
        // Subrecord 6: stack data (8 bytes)
        let stack = vec![0xBB; 8];
        data.extend_from_slice(&(stack.len() as u32).to_le_bytes());
        data.extend_from_slice(&stack);

        let (prims, order, pattern_name) = parse_pcb_data_stream(&data).unwrap();
        assert_eq!(pattern_name, b"PAD");
        assert_eq!(prims.len(), 1);
        assert_eq!(prims[0].key, 2);
        assert_eq!(order.len(), 1);

        // The raw_block should contain all 6 subrecords with u32 prefixes
        let raw = prims[0].origin.as_binary().unwrap();
        // 4*(4+2) + (4+94) + (4+8) = 24 + 98 + 12 = 134
        assert_eq!(raw.raw_block.len(), 134);

        // Round-trip via build_pcb_data_stream
        let prim_refs: Vec<&RecordNode> = prims.iter().collect();
        let rebuilt = build_pcb_data_stream(b"PAD", &order, &prim_refs).unwrap();
        let (prims2, _, _) = parse_pcb_data_stream(&rebuilt).unwrap();
        assert_eq!(prims2.len(), 1);
        assert_eq!(prims2[0].key, 2);
        assert_eq!(prims2[0].origin.as_binary().unwrap().raw_block.len(), 134);
    }

    #[test]
    fn text_multi_subrecord_roundtrip() {
        let mut data = Vec::new();
        // Pattern name: "TXT"
        let name = b"TXT";
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name);
        // Text primitive: type=5 with 2 subrecords
        data.push(5);
        // Subrecord 1: main text data (minimum valid for parser = 111 bytes)
        let sub1 = vec![0xCC; 111];
        data.extend_from_slice(&(sub1.len() as u32).to_le_bytes());
        data.extend_from_slice(&sub1);
        // Subrecord 2: text string (10 bytes)
        let sub2 = b"Hello\0\0\0\0\0";
        data.extend_from_slice(&(sub2.len() as u32).to_le_bytes());
        data.extend_from_slice(sub2);

        let (prims, order, _) = parse_pcb_data_stream(&data).unwrap();
        assert_eq!(prims.len(), 1);
        assert_eq!(prims[0].key, 5);
        // raw_block = (4+111) + (4+10) = 129
        assert_eq!(prims[0].origin.as_binary().unwrap().raw_block.len(), 129);

        // Round-trip
        let prim_refs: Vec<&RecordNode> = prims.iter().collect();
        let rebuilt = build_pcb_data_stream(b"TXT", &order, &prim_refs).unwrap();
        let (prims2, _, _) = parse_pcb_data_stream(&rebuilt).unwrap();
        assert_eq!(prims2.len(), 1);
        assert_eq!(prims2[0].key, 5);
        assert_eq!(prims2[0].origin.as_binary().unwrap().raw_block.len(), 129);
    }

    #[test]
    fn build_stream_roundtrip() {
        let block_data = vec![0xAA; 49];
        let prim = RecordNode::new(
            4,
            RecordOrigin::Binary(BinaryOrigin::new(block_data.clone())),
        );
        let order = vec![PcbPrimitiveRef::new(4, 0)];
        let prim_refs: Vec<&RecordNode> = vec![&prim];

        let data = build_pcb_data_stream(b"DIP-8", &order, &prim_refs).unwrap();
        let (prims, out_order, pattern_name) = parse_pcb_data_stream(&data).unwrap();

        assert_eq!(pattern_name, b"DIP-8");
        assert_eq!(prims.len(), 1);
        assert_eq!(prims[0].key, 4);
        assert_eq!(out_order.len(), 1);
        assert_eq!(out_order[0].type_id, 4);
    }

    // ---------------------------------------------------------------------------
    // DocumentStore-based PcbLib construction and query tests
    // ---------------------------------------------------------------------------

    /// Helper: build a minimal PcbLib in-memory with named footprints.
    fn make_test_lib(fp_names: &[&str]) -> PcbLib {
        let doc_meta = DocumentMeta::PcbLib {
            section_keys: SectionKeyList::new(),
            file_header_meta: PcbLibFileHeaderStreamMeta::default(),
            file_version_info_meta: PcbLibCountedDataStreamMeta {
                header_count: 1,
                data: Vec::new(),
            },
            library_meta: PcbLibLibraryStorageMeta::default(),
        };
        let mut store = DocumentStore::new(doc_meta);

        for &name in fp_names {
            let param_str = format!("|PATTERN={}|DESCRIPTION={}|", name, name);
            let metadata = RecordNode::new(0, RecordOrigin::Param(ParamOrigin::new(&param_str)));
            let parent_id = store.insert_record(metadata);

            let group_data = GroupData {
                parent: parent_id,
                children: Vec::new(),
                original_indices: Vec::new(),
                parent_original_index: None,
                meta: GroupMeta::PcbFootprint {
                    name: name.to_string(),
                    raw_pattern_name_block: name.as_bytes().to_vec(),
                    original_primitive_order: Vec::new(),
                    raw_header: Vec::new(),
                    sidecar_streams: PcbLibFootprintSidecarStreamsMeta::default(),
                },
            };
            store.insert_group(group_data);
        }

        PcbLib {
            store: Rc::new(RefCell::new(store)),
        }
    }

    /// Helper: build a PcbLib with one footprint containing typed primitives.
    fn make_lib_with_primitives() -> PcbLib {
        let doc_meta = DocumentMeta::PcbLib {
            section_keys: SectionKeyList::new(),
            file_header_meta: PcbLibFileHeaderStreamMeta::default(),
            file_version_info_meta: PcbLibCountedDataStreamMeta {
                header_count: 1,
                data: Vec::new(),
            },
            library_meta: PcbLibLibraryStorageMeta::default(),
        };
        let mut store = DocumentStore::new(doc_meta);

        let metadata =
            RecordNode::new(0, RecordOrigin::Param(ParamOrigin::new("|PATTERN=SOT-23|")));
        let parent_id = store.insert_record(metadata);

        let pad_block = vec![0u8; 40];
        let pad0 = RecordNode::new(
            2,
            RecordOrigin::Binary(BinaryOrigin::new(pad_block.clone())),
        );
        let pad1 = RecordNode::new(
            2,
            RecordOrigin::Binary(BinaryOrigin::new(pad_block.clone())),
        );
        let track = RecordNode::new(4, RecordOrigin::Binary(BinaryOrigin::new(vec![0u8; 35])));

        let pad0_id = store.insert_record(pad0);
        let pad1_id = store.insert_record(pad1);
        let track_id = store.insert_record(track);

        let group_data = GroupData {
            parent: parent_id,
            children: vec![pad0_id, pad1_id, track_id],
            original_indices: vec![0, 1, 2],
            parent_original_index: None,
            meta: GroupMeta::PcbFootprint {
                name: "SOT-23".to_string(),
                raw_pattern_name_block: b"SOT-23".to_vec(),
                original_primitive_order: vec![
                    PcbPrimitiveRef::new(2, 0),
                    PcbPrimitiveRef::new(2, 1),
                    PcbPrimitiveRef::new(4, 2),
                ],
                raw_header: Vec::new(),
                sidecar_streams: PcbLibFootprintSidecarStreamsMeta::default(),
            },
        };
        store.insert_group(group_data);

        PcbLib {
            store: Rc::new(RefCell::new(store)),
        }
    }

    fn write_minimal_system_streams<W: Read + Write + Seek>(cfb: &mut cfb::CompoundFile<W>) {
        let mut created_storages: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut write_stream = |path: &str, data: &[u8]| {
            ensure_parent_storages(cfb, path, &mut created_storages).unwrap();
            cfb.create_stream(path).unwrap().write_all(data).unwrap();
        };

        let file_header = PcbLibFileHeaderStreamMeta::default().to_stream_bytes().unwrap();
        write_stream("/FileHeader", &file_header);

        write_stream("/FileVersionInfo/Header", &1u32.to_le_bytes());
        write_stream("/FileVersionInfo/Data", b"|COUNT=1|");

        write_stream("/Library/Header", &1u32.to_le_bytes());
        write_stream("/Library/Data", b"");
        write_stream("/Library/EmbeddedFonts", b"");

        write_stream("/Library/ComponentParamsTOC/Header", &1u32.to_le_bytes());
        write_stream("/Library/ComponentParamsTOC/Data", b"");

        write_stream("/Library/LayerKindMapping/Header", &1u32.to_le_bytes());
        write_stream("/Library/LayerKindMapping/Data", b"");

        write_stream("/Library/Models/Header", &0u32.to_le_bytes());
        write_stream("/Library/Models/Data", b"");

        write_stream("/Library/ModelsNoEmbed/Header", &0u32.to_le_bytes());
        write_stream("/Library/ModelsNoEmbed/Data", b"");

        write_stream("/Library/PadViaLibrary/Header", &0u32.to_le_bytes());
        write_stream("/Library/PadViaLibrary/Data", b"");

        write_stream("/Library/Textures/Header", &0u32.to_le_bytes());
        write_stream("/Library/Textures/Data", b"");
    }

    fn write_minimal_footprint<W: Read + Write + Seek>(cfb: &mut cfb::CompoundFile<W>, name: &str) {
        cfb.create_storage(format!("/{}", name)).unwrap();
        cfb.create_stream(format!("/{}/Header", name))
            .unwrap()
            .write_all(&1u32.to_le_bytes())
            .unwrap();
        cfb.create_stream(format!("/{}/Parameters", name))
            .unwrap()
            .write_all(format!("|PATTERN={}|", name).as_bytes())
            .unwrap();
        let mut data = Vec::new();
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name.as_bytes());
        cfb.create_stream(format!("/{}/Data", name))
            .unwrap()
            .write_all(&data)
            .unwrap();
    }

    #[test]
    fn pcblib_footprint_count() {
        let lib = make_test_lib(&["SOT-23", "QFP-48", "DIP-8"]);
        assert_eq!(lib.footprint_count(), 3);
    }

    #[test]
    fn pcblib_names() {
        let lib = make_test_lib(&["SOT-23", "QFP-48", "DIP-8"]);
        let names = lib.names();
        assert_eq!(names, vec!["SOT-23", "QFP-48", "DIP-8"]);
    }

    #[test]
    fn pcblib_find_footprint_found() {
        let lib = make_test_lib(&["SOT-23", "QFP-48"]);
        let handle = lib.find_footprint("sot-23");
        assert!(handle.is_some());
        assert_eq!(handle.unwrap().name(), "SOT-23");
    }

    #[test]
    fn pcblib_find_footprint_not_found() {
        let lib = make_test_lib(&["SOT-23"]);
        assert!(lib.find_footprint("DIP-8").is_none());
    }

    #[test]
    fn pcblib_query_all_footprints() {
        use crate::v2::traits::DocumentQuery;

        let lib = make_test_lib(&["SOT-23", "QFP-48", "DIP-8"]);
        let results =
            DocumentQuery::<crate::v2::handles::PcbFootprint>::query_all(&lib, "#0").unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn pcblib_query_single_footprint() {
        use crate::v2::traits::DocumentQuery;

        let lib = make_test_lib(&["SOT-23"]);
        let handle = DocumentQuery::<crate::v2::handles::PcbFootprint>::query(&lib, "#0").unwrap();
        assert_eq!(handle.name(), "SOT-23");
    }

    #[test]
    fn pcblib_query_no_match() {
        use crate::v2::traits::DocumentQuery;

        let lib = make_test_lib(&["SOT-23"]);
        let result = DocumentQuery::<crate::v2::handles::PcbFootprint>::query(&lib, "NONEXISTENT");
        assert!(matches!(result, Err(crate::error::AltiumError::NoMatch(_))));
    }

    #[test]
    fn pcblib_deep_query_pads() {
        let lib = make_lib_with_primitives();
        let results = lib
            .query_all_children::<crate::v2::handles::PcbPad>("#2")
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn pcblib_deep_query_track() {
        let lib = make_lib_with_primitives();
        let results = lib
            .query_all_children::<crate::v2::handles::PcbTrack>("#4")
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn pcblib_build_footprint() {
        use crate::v2::templates;

        let lib = PcbLib {
            store: DocumentStore::new_ref(DocumentMeta::PcbLib {
                section_keys: SectionKeyList::new(),
                file_header_meta: PcbLibFileHeaderStreamMeta::default(),
                file_version_info_meta: PcbLibCountedDataStreamMeta {
                    header_count: 1,
                    data: Vec::new(),
                },
                library_meta: PcbLibLibraryStorageMeta::default(),
            }),
        };

        assert_eq!(lib.footprint_count(), 0);
        lib.build_footprint("SOIC-8", templates::pcb_footprint_default, |_builder| {});
        assert_eq!(lib.footprint_count(), 1);
        assert_eq!(lib.names(), vec!["SOIC-8"]);
    }

    #[test]
    fn pcblib_unique_id_empty_when_absent() {
        let lib = make_test_lib(&["SOT-23"]);
        assert_eq!(lib.unique_id(), "");
    }

    #[test]
    fn pcblib_save_and_open_roundtrip() {
        use std::io::Cursor;

        let lib = make_test_lib(&["SOT-23", "DIP-8"]);
        let buf = Cursor::new(Vec::new());
        lib.save(buf).unwrap();
    }

    #[test]
    fn file_version_info_storage_not_treated_as_footprint() {
        use std::io::Cursor;

        let mut cfb =
            cfb::CompoundFile::create_with_version(cfb::Version::V3, Cursor::new(Vec::new()))
                .unwrap();

        write_minimal_system_streams(&mut cfb);
        write_minimal_footprint(&mut cfb, "SOT-23");

        cfb.flush().unwrap();
        let bytes = cfb.into_inner().into_inner();

        let lib = PcbLib::open(Cursor::new(bytes)).unwrap();
        assert_eq!(lib.footprint_count(), 1);
        assert_eq!(lib.names(), vec!["SOT-23"]);

        let file_version_info = lib.file_version_info_meta();
        assert_eq!(file_version_info.header_count, 1);
        assert_eq!(file_version_info.data, b"|COUNT=1|");
    }

    #[test]
    fn pcblib_open_rejects_unknown_system_stream() {
        use std::io::Cursor;

        let mut cfb =
            cfb::CompoundFile::create_with_version(cfb::Version::V3, Cursor::new(Vec::new()))
                .unwrap();

        write_minimal_system_streams(&mut cfb);
        write_minimal_footprint(&mut cfb, "SOT-23");

        // Unknown non-footprint stream should be rejected.
        cfb.create_storage("/Mystery").unwrap();
        cfb.create_stream("/Mystery/Meta")
            .unwrap()
            .write_all(b"x")
            .unwrap();

        cfb.flush().unwrap();
        let bytes = cfb.into_inner().into_inner();

        let err = match PcbLib::open(Cursor::new(bytes)) {
            Ok(_) => panic!("expected PcbLib::open to fail"),
            Err(err) => err,
        };
        assert!(format!("{err}")
            .to_ascii_lowercase()
            .contains("unimplemented stream"));
    }

    #[test]
    fn pcblib_open_rejects_unknown_footprint_stream() {
        use std::io::Cursor;

        let mut cfb =
            cfb::CompoundFile::create_with_version(cfb::Version::V3, Cursor::new(Vec::new()))
                .unwrap();

        write_minimal_system_streams(&mut cfb);
        write_minimal_footprint(&mut cfb, "SOT-23");

        cfb.create_stream("/SOT-23/UnknownSidecar")
            .unwrap()
            .write_all(b"x")
            .unwrap();

        cfb.flush().unwrap();
        let bytes = cfb.into_inner().into_inner();

        let err = match PcbLib::open(Cursor::new(bytes)) {
            Ok(_) => panic!("expected PcbLib::open to fail"),
            Err(err) => err,
        };
        assert!(format!("{err}")
            .to_ascii_lowercase()
            .contains("unimplemented stream"));
    }
}

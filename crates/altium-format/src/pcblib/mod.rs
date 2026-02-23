pub(crate) mod footprint;
pub(crate) mod library;
pub(crate) mod primitives;
pub(crate) mod section_keys;
pub(crate) mod sidecar;
pub(crate) mod wide_strings;

use std::collections::HashMap;
use std::path::Path;

use altium_format_types::constants::file_headers::PCB_LIBRARY_BINARY_HEADER_V6;
use altium_format_types::constants::streams::{FILE_HEADER, SECTION_KEYS};
use altium_format_types::{Coord, CoordPoint, PadShape, PadStackMode, PcbFlags, RegionKind, TextKind, V6Layer};

use crate::block_stream::iter_blocks;
use crate::pcb_binary_stream::parse_pcb_section_header;
use crate::pcb_file_header::{parse_pcb_file_header, PcbFileHeader};
use crate::pcblib::library::{
    PcbLibraryData, PcbLibComponentTocEntry, PcbLibModelEntry,
    consume_embedded_fonts, consume_header_data_substorage, parse_library_data,
    parse_component_toc, parse_model_metadata,
};
use crate::tracked_cfb::TrackedCfbDocument;
use crate::{AltiumFormatError, Result, ResultExt};

pub struct PcbLib {
    pub(crate) header: PcbFileHeader,
    pub(crate) section_keys: HashMap<String, String>,
    pub(crate) library: PcbLibraryData,
    pub(crate) component_toc: Vec<PcbLibComponentTocEntry>,
    pub(crate) model_entries: Vec<PcbLibModelEntry>,
    pub(crate) footprints: Vec<PcbFootprint>,
    pub(crate) file_version_info: Option<String>,
}

pub(crate) struct PcbFootprint {
    pub(crate) display_name: String,
    pub(crate) cfb_key: String,
    pub(crate) pattern: String,
    pub(crate) height: Coord,
    pub(crate) description: String,
    pub(crate) item_guid: String,
    pub(crate) revision_guid: String,
    pub(crate) primitives: Vec<PcbPrimitive>,
}

pub(crate) struct PcbPrimitiveCommon {
    pub(crate) layer: V6Layer,
    pub(crate) pad_byte: u8,
    pub(crate) flags: PcbFlags,
    pub(crate) net_index: i32,
    pub(crate) polygon_index: u16,
    pub(crate) component_index: u16,
    pub(crate) unknown: u8,
}

pub(crate) enum PcbPrimitive {
    Arc(PcbArc),
    Pad(PcbPad),
    Via(PcbVia),
    Track(PcbTrack),
    Text(PcbText),
    Fill(PcbFill),
    Region(PcbRegion),
    ComponentBody(PcbComponentBody),
}

pub(crate) struct PcbArc {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) center: CoordPoint,
    pub(crate) radius: Coord,
    pub(crate) start_angle: f64,
    pub(crate) end_angle: f64,
    pub(crate) width: Coord,
    pub(crate) unique_id: Option<String>,
    pub(crate) trailing_bytes: Vec<u8>,
}

pub(crate) struct PcbTrack {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) start: CoordPoint,
    pub(crate) end: CoordPoint,
    pub(crate) width: Coord,
    pub(crate) subpoly_index: u16,
    pub(crate) unique_id: Option<String>,
    pub(crate) trailing_bytes: Vec<u8>,
}

pub(crate) struct PcbVia {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) location: CoordPoint,
    pub(crate) hole_size: Coord,
    pub(crate) diameter_top: Coord,
    pub(crate) diameter_mid: Coord,
    pub(crate) diameter_bot: Coord,
    pub(crate) from_layer: V6Layer,
    pub(crate) to_layer: V6Layer,
    pub(crate) unique_id: Option<String>,
    pub(crate) trailing_bytes: Vec<u8>,
}

pub(crate) struct PcbFill {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) corner1: CoordPoint,
    pub(crate) corner2: CoordPoint,
    pub(crate) rotation: f64,
    pub(crate) unique_id: Option<String>,
    pub(crate) trailing_bytes: Vec<u8>,
}

pub(crate) struct PcbText {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) location: CoordPoint,
    pub(crate) height: Coord,
    pub(crate) rotation: f64,
    pub(crate) is_mirrored: bool,
    pub(crate) stroke_width: Coord,
    pub(crate) is_comment: bool,
    pub(crate) is_designator: bool,
    pub(crate) font_kind: TextKind,
    pub(crate) text: String,
    pub(crate) unique_id: Option<String>,
    pub(crate) trailing_bytes: Vec<u8>,
}

pub(crate) struct PcbRegion {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) kind: RegionKind,
    pub(crate) vertices: Vec<CoordPoint>,
    pub(crate) unique_id: Option<String>,
    pub(crate) trailing_bytes: Vec<u8>,
}

pub(crate) struct PcbPad {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) location: CoordPoint,
    pub(crate) size_top: CoordPoint,
    pub(crate) size_mid: CoordPoint,
    pub(crate) size_bot: CoordPoint,
    pub(crate) hole_size: Coord,
    pub(crate) shape_top: PadShape,
    pub(crate) shape_mid: PadShape,
    pub(crate) shape_bot: PadShape,
    pub(crate) rotation: f64,
    pub(crate) is_plated: bool,
    pub(crate) stack_mode: PadStackMode,
    pub(crate) unique_id: Option<String>,
    pub(crate) subrecord_trailing: [Vec<u8>; 6],
}

pub(crate) struct PcbComponentBody {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) model_guid: String,
    pub(crate) standoff_height: Coord,
    pub(crate) rotation_x: f64,
    pub(crate) rotation_y: f64,
    pub(crate) rotation_z: f64,
    pub(crate) outline: Vec<CoordPoint>,
    pub(crate) unique_id: Option<String>,
    pub(crate) trailing_bytes: Vec<u8>,
}

/// Parses the FileVersionInfo/Header and Data streams.
///
/// The Data stream contains a single text block with pipe-delimited parameters
/// (COUNT, VER0, FWDMSG0, BKMSG0, etc.). We decode the block and return the
/// raw decoded string for version identification.
fn parse_file_version_info(header_data: &[u8], data: &[u8]) -> Result<String> {
    let _count = parse_pcb_section_header(header_data)?;

    let mut result = String::new();
    for block_result in iter_blocks(data) {
        let block = block_result?;
        let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&block.data);
        result = decoded.into_owned();
    }
    Ok(result)
}

impl PcbLib {
    /// Returns the on-disk header string identifying the file format version.
    pub fn version_header(&self) -> &str {
        &self.header.version_string
    }

    /// Returns the version number from the file header (e.g. 5.01).
    pub fn minor_version(&self) -> f64 {
        self.header.version
    }

    /// Returns the optional `FileVersionInfo` string from the FileVersionInfo storage.
    pub fn file_version_info(&self) -> Option<&str> {
        self.file_version_info.as_deref()
    }

    /// Returns the number of footprints in this library.
    pub fn footprint_count(&self) -> usize {
        self.footprints.len()
    }

    /// Returns the display names of all footprints in this library.
    pub fn footprint_names(&self) -> Vec<&str> {
        self.footprints.iter().map(|fp| fp.display_name.as_str()).collect()
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let mut doc = TrackedCfbDocument::open(path)?;

        // 1. FileHeader
        let file_header_data = doc.read_stream(&format!("/{FILE_HEADER}"))?;
        let header = parse_pcb_file_header(&file_header_data)?;
        if header.version_string != PCB_LIBRARY_BINARY_HEADER_V6 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: FILE_HEADER.to_owned(),
                detail: format!(
                    "expected \"{}\", got \"{}\"",
                    PCB_LIBRARY_BINARY_HEADER_V6, header.version_string
                ),
            });
        }

        // 2. SectionKeys (optional)
        let section_keys = match doc.read_stream_optional(&format!("/{SECTION_KEYS}"))? {
            Some(data) => section_keys::parse_section_keys(&data)?,
            None => HashMap::new(),
        };

        // 3. Library/ storage
        let lib_header_data = doc.read_stream("/Library/Header")?;
        let _lib_header_count = crate::pcb_binary_stream::parse_pcb_section_header(&lib_header_data)?;

        let lib_data_raw = doc.read_stream("/Library/Data")?;
        let library = parse_library_data(&lib_data_raw)
            .context("parsing /Library/Data")?;

        let lib_toc_header = doc.read_stream("/Library/ComponentParamsTOC/Header")?;
        let lib_toc_data = doc.read_stream("/Library/ComponentParamsTOC/Data")?;
        let component_toc = parse_component_toc(&lib_toc_header, &lib_toc_data)
            .context("parsing /Library/ComponentParamsTOC")?;
        let _ = doc.list_entries("/Library/ComponentParamsTOC")?;

        let lib_models_header = doc.read_stream("/Library/Models/Header")?;
        let lib_models_data = doc.read_stream("/Library/Models/Data")?;
        let mut model_entries = parse_model_metadata(&lib_models_header, &lib_models_data)?;
        for (i, entry) in model_entries.iter_mut().enumerate() {
            let blob_path = format!("/Library/Models/{i}");
            entry.blob = doc.read_stream_optional(&blob_path)?;
        }
        let _ = doc.list_entries("/Library/Models")?;

        // Auxiliary Library sub-storages (optional)
        if doc.exists("/Library/LayerKindMapping/Header") {
            consume_header_data_substorage(
                &mut doc,
                "/Library/LayerKindMapping/Header",
                "/Library/LayerKindMapping/Data",
            )?;
            let _ = doc.list_entries("/Library/LayerKindMapping")?;
        }
        if doc.exists("/Library/PadViaLibrary/Header") {
            consume_header_data_substorage(
                &mut doc,
                "/Library/PadViaLibrary/Header",
                "/Library/PadViaLibrary/Data",
            )?;
            let _ = doc.list_entries("/Library/PadViaLibrary")?;
        }
        if doc.exists("/Library/EmbeddedFonts") {
            consume_embedded_fonts(&mut doc, "/Library/EmbeddedFonts")?;
        }
        if doc.exists("/Library/ModelsNoEmbed/Header") {
            consume_header_data_substorage(
                &mut doc,
                "/Library/ModelsNoEmbed/Header",
                "/Library/ModelsNoEmbed/Data",
            )?;
            let _ = doc.list_entries("/Library/ModelsNoEmbed")?;
        }
        if doc.exists("/Library/Textures/Header") {
            consume_header_data_substorage(
                &mut doc,
                "/Library/Textures/Header",
                "/Library/Textures/Data",
            )?;
            let _ = doc.list_entries("/Library/Textures")?;
        }

        // Mark Library storage itself as consumed.
        let _ = doc.list_entries("/Library")?;

        // 4. FileVersionInfo (optional Header/Data substorage)
        let file_version_info = if doc.exists("/FileVersionInfo/Header") {
            let fvi_header = doc.read_stream("/FileVersionInfo/Header")?;
            let fvi_data = doc.read_stream("/FileVersionInfo/Data")?;
            let _ = doc.list_entries("/FileVersionInfo")?;
            Some(parse_file_version_info(&fvi_header, &fvi_data)?)
        } else {
            let _ = doc.read_stream_optional("/FileVersionInfo/Header")?;
            let _ = doc.read_stream_optional("/FileVersionInfo/Data")?;
            None
        };

        // 5. Enumerate top-level storages (exclude system storages FileVersionInfo and Library)
        let (storages, _streams) = doc.list_entries("/")?;
        let mut footprints = Vec::new();
        for storage_name in &storages {
            let name = storage_name.trim_start_matches('/');
            if name == "FileVersionInfo" || name == "Library" {
                continue;
            }
            let data_path = format!("/{name}/Data");
            if !doc.exists(&data_path) {
                continue;
            }
            let display_name = {
                let reverse: HashMap<_, _> = section_keys
                    .iter()
                    .map(|(k, v)| (v.as_str(), k.as_str()))
                    .collect();
                reverse
                    .get(name)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| name.to_owned())
            };
            let fp = footprint::load_footprint(&mut doc, name, &display_name)
                .with_context(|| format!("loading footprint '{display_name}' (/{name})"))?;
            footprints.push(fp);
        }

        // 6. Assert all CFB entries consumed
        doc.assert_all_consumed()?;

        Ok(Self { header, section_keys, library, component_toc, model_entries, footprints, file_version_info })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altium_format_types::PcbObjectId;

    #[test]
    fn pcblib_struct_compiles() {
        let _ = PcbLib {
            header: PcbFileHeader {
                version_string: String::new(),
                version: 0.0,
                unique_id: String::new(),
            },
            section_keys: HashMap::new(),
            library: library::PcbLibraryData {
                filename: String::new(),
                kind: String::new(),
                version: String::new(),
                date: String::new(),
                time: String::new(),
            },
            component_toc: Vec::new(),
            model_entries: Vec::new(),
            footprints: Vec::new(),
            file_version_info: None,
        };
    }

    #[test]
    fn pcbprimitive_enum_all_variants() {
        let _ = PcbObjectId::Arc;
        let _ = PcbObjectId::Pad;
        let _ = PcbObjectId::Via;
        let _ = PcbObjectId::Track;
        let _ = PcbObjectId::Text;
        let _ = PcbObjectId::Fill;
        let _ = PcbObjectId::Region;
        let _ = PcbObjectId::ComponentBody;
    }
}

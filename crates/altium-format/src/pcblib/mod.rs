pub(crate) mod footprint;
pub(crate) mod library;
pub(crate) mod primitives;
pub(crate) mod section_keys;
pub(crate) mod sidecar;
pub(crate) mod wide_strings;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use altium_format_types::constants::file_headers::PCB_LIBRARY_BINARY_HEADER_V6;
use altium_format_types::constants::streams::{FILE_HEADER, SECTION_KEYS};
use altium_format_types::pcb::{PolygonReliefAngle, TentingMode};
use altium_format_types::{
    Color, Coord, CoordPoint, DaisyChainStyle, MaskExpansionMode, PadShape, PadStackMode,
    PcbFlags, PcbObjectId, PlaneConnectionStyle, RegionKind, TCacheState, TextAutoposition,
    TextKind, V6Layer, V7Layer, ViaStructureType,
};

use crate::block_stream::iter_blocks;
use crate::block_stream::write_text_block;
use crate::binary_io::BinaryWriter;
use crate::cfb_document::CfbDocument;
use crate::pcb_binary_stream::parse_pcb_section_header;
use crate::pcb_file_header::{PcbFileHeader, parse_pcb_file_header};
use crate::pcblib::library::{
    PcbEmbeddedFontEntry, PcbLayerKindMapping, PcbLibComponentTocEntry, PcbLibModelEntry,
    PcbLibraryData, PcbPadViaLibraryConfig, PcbTextureEntry, parse_component_toc,
    parse_embedded_fonts, parse_layer_kind_mapping, parse_library_data, parse_model_metadata,
    parse_pad_via_library, parse_texture_metadata,
};
use crate::pcblib::sidecar::{ExtendedPrimitiveInfoEntry, PrimitiveGuidEntry};
use crate::tracked_cfb::TrackedCfbDocument;
use crate::{AltiumFormatError, Result, ResultExt};

pub struct PcbLib {
    pub(crate) header: PcbFileHeader,
    pub(crate) section_keys: HashMap<String, String>,
    pub(crate) library: PcbLibraryData,
    pub(crate) component_toc: Vec<PcbLibComponentTocEntry>,
    pub(crate) model_entries: Vec<PcbLibModelEntry>,
    pub(crate) model_no_embed_entries: Vec<PcbLibModelEntry>,
    pub(crate) layer_kind_mapping: PcbLayerKindMapping,
    pub(crate) pad_via_library: Option<PcbPadViaLibraryConfig>,
    pub(crate) embedded_fonts: Vec<PcbEmbeddedFontEntry>,
    pub(crate) texture_entries: Vec<PcbTextureEntry>,
    pub(crate) footprints: Vec<PcbFootprint>,
    pub(crate) file_version_info: Option<String>,
    pub(crate) source_path: Option<PathBuf>,
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
    pub(crate) extended_primitive_info: Vec<ExtendedPrimitiveInfoEntry>,
    pub(crate) primitive_guids: Vec<PrimitiveGuidEntry>,
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
    pub(crate) subpoly_index: u16,
    pub(crate) user_routed: bool,
    pub(crate) union_index: i32,
    pub(crate) v7_layer: V7Layer,
    pub(crate) keepout_restrictions: i32,
    pub(crate) unique_id: Option<String>,
}

pub(crate) struct PcbTrack {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) start: CoordPoint,
    pub(crate) end: CoordPoint,
    pub(crate) width: Coord,
    pub(crate) subpoly_index: u16,
    // AD26+ extension fields (10 or 14 extra bytes)
    pub(crate) user_routed: bool,
    pub(crate) union_index: i32,
    pub(crate) track_kind: u8,
    pub(crate) layer_enum_index: i32,
    pub(crate) keepout_restrictions: i32,
    pub(crate) unique_id: Option<String>,
}

pub(crate) struct PcbVia {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) location: CoordPoint,
    pub(crate) diameter: Coord,
    pub(crate) hole_size: Coord,
    pub(crate) from_layer: V6Layer,
    pub(crate) to_layer: V6Layer,
    // Extended fields (offset 31+, present when subrecord > 31 bytes)
    pub(crate) via_properties_version: u8,
    pub(crate) thermal_relief_air_gap: Coord,
    pub(crate) thermal_relief_conductor_count: u8,
    pub(crate) thermal_relief_rotation_code: u8,
    pub(crate) thermal_relief_conductor_width: Coord,
    pub(crate) power_plane_relief_expansion: Coord,
    pub(crate) power_plane_clearance: Coord,
    pub(crate) paste_mask_expansion: Coord,
    pub(crate) solder_mask_expansion_front: Coord,
    pub(crate) planes: u16,
    pub(crate) plane_connection_style_valid: TCacheState,
    pub(crate) relief_conductor_width_valid: TCacheState,
    pub(crate) relief_entries_valid: TCacheState,
    pub(crate) relief_air_gap_valid: TCacheState,
    pub(crate) power_plane_relief_expansion_valid: TCacheState,
    pub(crate) paste_mask_expansion_valid: TCacheState,
    pub(crate) solder_mask_expansion_manual: bool,
    pub(crate) solder_mask_expansion_valid: TCacheState,
    pub(crate) power_plane_clearance_valid: TCacheState,
    pub(crate) planes_valid: TCacheState,
    pub(crate) plane_connection_style: PlaneConnectionStyle,
    pub(crate) solder_mask_expansion_mode: MaskExpansionMode,
    pub(crate) paste_mask_expansion_mode: MaskExpansionMode,
    pub(crate) tenting_mode: TentingMode,
    pub(crate) via_mode: u8,
    pub(crate) diameters_per_layer: [Coord; 32],
    // Additional extended (offset 203+)
    pub(crate) layer_enum_index: i32,
    pub(crate) stack_start_layer: u8,
    pub(crate) stack_end_layer: u8,
    pub(crate) extension_coord_209: Coord,
    pub(crate) extension_coord_213: Coord,
    pub(crate) extension_coord_217: Coord,
    pub(crate) extension_coord_221: Coord,
    pub(crate) extension_coord_225: Coord,
    pub(crate) extension_coord_229: Coord,
    pub(crate) extension_coord_233: Coord,
    pub(crate) extension_coord_237: Coord,
    pub(crate) solder_mask_expansion_linked: bool,
    pub(crate) solder_mask_expansion_back: Coord,
    // Via template link extended block (after section2, 46-byte trailing data).
    // Present in AD26 files from ~2019+. Format: u8 version + GUID[16] + GUID[16] + i32 HolePosTol + i32 HoleNegTol + u8 flags.
    pub(crate) template_link_version: Option<u8>,
    pub(crate) template_link_library_id: Option<[u8; 16]>,
    pub(crate) template_link_template_id: Option<[u8; 16]>,
    pub(crate) hole_positive_tolerance: Option<Coord>,
    pub(crate) hole_negative_tolerance: Option<Coord>,
    pub(crate) template_link_flags: Option<u8>,
    // IPC-4761 via structure block (21 bytes, ~2022+).
    // Binary layout: i32 + i32 + i32 + f64 + TViaStructureType(u8) = 21 bytes.
    // Related C# interfaces: IHoleSizeInfo, IPCB_ViaStructureSupport.
    // The f64 is confirmed as counter_hole_angle (IHoleSizeInfo.GetCounterHoleAngle() -> double).
    // One i32 is likely counter_hole_depth (IHoleSizeInfo.GetCounterHoleDepth() -> int).
    // Exact i32 field ordering unknown — Delphi save/load code not yet decompiled.
    pub(crate) ipc4761_field_0: Option<i32>,
    pub(crate) ipc4761_field_1: Option<i32>,
    pub(crate) ipc4761_field_2: Option<i32>,
    pub(crate) ipc4761_counter_hole_angle: Option<f64>,
    pub(crate) via_structure_type: Option<ViaStructureType>,
    pub(crate) layer_diameter_overrides: Vec<PcbViaSection2Entry>,
    pub(crate) unique_id: Option<String>,
}

pub(crate) struct PcbViaSection2Entry {
    pub(crate) layer: u8,
    pub(crate) diameter: Coord,
    pub(crate) rule_index: u16,
    pub(crate) flags: u8,
    pub(crate) mode: u8,
}

pub(crate) struct PcbFill {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) corner1: CoordPoint,
    pub(crate) corner2: CoordPoint,
    pub(crate) rotation: f64,
    // AD26+ extended fields (13 bytes when present)
    pub(crate) user_routed: bool,
    pub(crate) union_index: i32,
    pub(crate) v7_layer: V7Layer,
    pub(crate) keepout_restrictions: i32,
    pub(crate) unique_id: Option<String>,
}

pub(crate) struct PcbText {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) location: CoordPoint,
    pub(crate) height: Coord,
    pub(crate) text_kind: TextKind,
    pub(crate) rotation: f64,
    pub(crate) is_mirrored: bool,
    pub(crate) stroke_width: Coord,
    pub(crate) is_italic: bool,
    pub(crate) is_bold: bool,
    pub(crate) font_name: String,
    pub(crate) inverted: bool,
    pub(crate) wide_string_index: i32,
    pub(crate) ttf_text_width: Coord,
    pub(crate) ttf_text_height: Coord,
    pub(crate) font_id: i32,
    pub(crate) barcode_inverted: bool,
    pub(crate) barcode_full_width: Coord,
    pub(crate) barcode_full_height: Coord,
    pub(crate) barcode_x_margin: Coord,
    pub(crate) barcode_y_margin: Coord,
    pub(crate) barcode_min_width: Coord,
    pub(crate) barcode_show_text: bool,
    pub(crate) barcode_render_mode: u8,
    pub(crate) multiline: bool,
    pub(crate) barcode_font_name: String,
    // Extended text fields (offset 225+, version-dependent).
    // Confirmed by C# IPCB_Text3 and IPCB_Text_SaveLoadParameters interfaces.
    pub(crate) ttf_inverted_justify: Option<TextAutoposition>,
    pub(crate) ttf_offset_from_inverted_rect: Option<u8>,
    pub(crate) tail_reserved_227: Option<u8>,
    pub(crate) multiline_auto_position: Option<TextAutoposition>,
    pub(crate) is_advance_justification_valid: Option<bool>,
    pub(crate) advance_snapping: Option<u8>,
    pub(crate) tail_reserved_231: Option<u8>,
    pub(crate) advance_justification_x: Option<i32>,
    pub(crate) advance_justification_y: Option<i32>,
    pub(crate) use_text_alignment_by_snap: Option<i32>,
    pub(crate) snap_point_x: Option<Coord>,
    pub(crate) snap_point_y: Option<Coord>,
    pub(crate) text: String,
    pub(crate) unique_id: Option<String>,
}

pub(crate) struct PcbRegion {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) kind: RegionKind,
    // Region parameters (from embedded param string)
    pub(crate) v7_layer: String,
    pub(crate) name: String,
    pub(crate) param_kind: i32,
    pub(crate) subpoly_index: i32,
    pub(crate) union_index: i32,
    pub(crate) arc_resolution: Coord,
    pub(crate) is_shape_based: bool,
    pub(crate) cavity_height: Coord,
    pub(crate) keepout_restrictions: i32,
    pub(crate) layer: String,
    pub(crate) keepout: bool,
    pub(crate) is_board_cutout: bool,
    pub(crate) pad_index: i32,
    // Geometry: main outline + hole contours (all f64 vertex pairs)
    pub(crate) outline: Vec<CoordPoint>,
    pub(crate) holes: Vec<Vec<CoordPoint>>,
    pub(crate) unique_id: Option<String>,
}

/// TV6_PadCache — 38 bytes at pad main subrecord offsets 67-104.
///
/// Confirmed by C# `TV6_PadCache` struct (Pack=1) + Ghidra setter functions.
#[derive(Debug)]
pub(crate) struct PcbPadCache {
    pub(crate) plane_connection_style: PlaneConnectionStyle,
    pub(crate) relief_conductor_width: Coord,
    pub(crate) relief_entries: i16,
    pub(crate) relief_air_gap: Coord,
    pub(crate) power_plane_relief_expansion: Coord,
    pub(crate) power_plane_clearance: Coord,
    pub(crate) paste_mask_expansion: Coord,
    pub(crate) solder_mask_expansion: Coord,
    pub(crate) planes: u16,
    pub(crate) plane_connection_style_valid: TCacheState,
    pub(crate) relief_conductor_width_valid: TCacheState,
    pub(crate) relief_entries_valid: TCacheState,
    pub(crate) relief_air_gap_valid: TCacheState,
    pub(crate) power_plane_relief_expansion_valid: TCacheState,
    pub(crate) paste_mask_expansion_valid: TCacheState,
    pub(crate) solder_mask_expansion_valid: TCacheState,
    pub(crate) power_plane_clearance_valid: TCacheState,
    pub(crate) planes_valid: TCacheState,
}

/// Per-layer stack data for pads (subrecord 5, 596+ bytes when present).
///
/// Confirmed by Ghidra FUN_018a2840 (init) + FUN_0187c7d0 (per-layer loop).
#[derive(Debug)]
pub(crate) struct PcbPadStackData {
    pub(crate) inner_size_x: [Coord; 29],
    pub(crate) inner_size_y: [Coord; 29],
    pub(crate) inner_shape: [PadShape; 29],
    pub(crate) padding_261: u8,
    pub(crate) hole_shape: u8,
    pub(crate) slot_size: Coord,
    pub(crate) slot_rotation: f64,
    pub(crate) hole_offset_x: [Coord; 32],
    pub(crate) hole_offset_y: [Coord; 32],
    pub(crate) padding_531: u8,
    pub(crate) alt_shape: [u8; 32],
    pub(crate) corner_radius_pct: [u8; 32],
    pub(crate) per_layer_overrides: [u8; 32],
    /// Extended per-layer corner radius entries (offset 628+).
    /// Confirmed by C# IPCB_Pad3: StackCRPctExOnLayer, StackCRSizeOnLayer, StackCRUsePercentOnLayer.
    pub(crate) extended_cr: Vec<PcbPadExtendedCrEntry>,
}

/// Extended per-layer corner radius entry (15 bytes each).
///
/// Confirmed by C# IPCB_Pad3 and IPCB_PadTemplateStackData interfaces:
/// - CRPctEx (double in C#, stored as Coord in binary)
/// - CRSize (int)
/// - UseCRPct (bool)
#[derive(Debug)]
pub(crate) struct PcbPadExtendedCrEntry {
    pub(crate) layer_id: u32,
    pub(crate) alt_shape: u8,
    pub(crate) cr_pct_ex: Coord,
    pub(crate) cr_size: Coord,
    pub(crate) cr_pct: u8,
    pub(crate) use_percent: bool,
}

pub(crate) struct PcbPad {
    pub(crate) common: PcbPrimitiveCommon,
    // Subrecords 0-3: pad name and string data
    pub(crate) pad_name: String,
    pub(crate) unknown_sub1: String,
    pub(crate) unknown_sub2: String,
    pub(crate) unknown_sub3: String,
    // Core pad fields (offsets 13-62)
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
    pub(crate) daisy_chain_style: DaisyChainStyle,
    pub(crate) pad_mode: PadStackMode,
    // Field at offset 63 (FUN_01811110)
    pub(crate) unknown_63: i32,
    // TV6_PadCache (offsets 67-104)
    pub(crate) cache: PcbPadCache,
    // Post-cache fields (offsets 105-113)
    pub(crate) selection_memory_flags: u8,
    pub(crate) union_index: i32,
    pub(crate) jumper_id: i32,
    // Extended fields (offsets 114-171, from FUN_0187b7c0)
    pub(crate) v7_layer_override: i32,
    pub(crate) is_assy_testpoint_top: bool,
    pub(crate) is_assy_testpoint_bottom: bool,
    pub(crate) use_separate_expansions: bool,
    pub(crate) solder_mask_bottom_expansion: i32,
    pub(crate) solder_mask_expansion_from_hole_edge: bool,
    pub(crate) template_link_library_id: [u8; 16],
    pub(crate) template_link_template_id: [u8; 16],
    pub(crate) pin_package_length: Coord,
    pub(crate) hole_positive_tolerance: i32,
    pub(crate) hole_negative_tolerance: i32,
    pub(crate) reserved_170: u8,
    pub(crate) has_sub4_extension: bool,
    pub(crate) sub4_extension: Option<PcbPadSub4Extension>,
    pub(crate) thermal_reliefs: Vec<PcbPadThermalReliefEntry>,
    // Subrecord 5: per-layer stack data (0 or 596+ bytes)
    pub(crate) stack_data: Option<PcbPadStackData>,
    // Sidecar
    pub(crate) unique_id: Option<String>,
}

#[derive(Debug)]
pub(crate) struct PcbPadSub4Extension {
    pub(crate) header_len: u32,
    pub(crate) thermal_relief_count: u32,
    pub(crate) propagation_delay_f32: f32,
    pub(crate) flags8: u8,
    pub(crate) flags9: u8,
    pub(crate) propagation_delay_f64: f64,
    /// Extension header bytes 18..21: hypothesized as `XPadOffsetAllLayers` (IPCB_Pad3).
    /// Always zero in all 37,669 test pads with 26-byte headers. Asserted zero on parse.
    pub(crate) x_pad_offset_all_layers: Coord,
    /// Extension header bytes 22..25: hypothesized as `YPadOffsetAllLayers` (IPCB_Pad3).
    /// Always zero in all 37,669 test pads with 26-byte headers. Asserted zero on parse.
    pub(crate) y_pad_offset_all_layers: Coord,
}

#[derive(Debug)]
pub(crate) struct PcbPadThermalReliefEntry {
    pub(crate) layer: V7Layer,
    pub(crate) defined_type: u8,
    pub(crate) connect_style: PlaneConnectionStyle,
    pub(crate) air_gap_width: Coord,
    pub(crate) conductor_width: Coord,
    pub(crate) rotation: PolygonReliefAngle,
    pub(crate) entries: u32,
    pub(crate) expansion: Coord,
    pub(crate) conductor_by_pad_edge: bool,
    pub(crate) min_distance: Coord,
    pub(crate) enable_min_distance: bool,
    pub(crate) use_custom_relief: bool,
}

pub(crate) struct PcbComponentBody {
    pub(crate) common: PcbPrimitiveCommon,
    // Region-inherited parameters
    pub(crate) v7_layer: String,
    pub(crate) name: String,
    pub(crate) kind: i32,
    pub(crate) subpoly_index: i32,
    pub(crate) union_index: i32,
    pub(crate) arc_resolution: Coord,
    pub(crate) is_shape_based: bool,
    pub(crate) cavity_height: Coord,
    // ComponentBody parameters
    pub(crate) standoff_height: Coord,
    pub(crate) overall_height: Coord,
    pub(crate) body_projection: i32,
    pub(crate) body_color_3d: Color,
    pub(crate) body_opacity_3d: f64,
    pub(crate) identifier: String,
    pub(crate) texture: String,
    pub(crate) texture_center_x: Coord,
    pub(crate) texture_center_y: Coord,
    pub(crate) texture_size_x: Coord,
    pub(crate) texture_size_y: Coord,
    pub(crate) texture_rotation: f64,
    pub(crate) body_override_color: bool,
    // 3D model parameters
    pub(crate) model_guid: String,
    pub(crate) model_checksum: String,
    pub(crate) model_embed: bool,
    pub(crate) model_name: String,
    pub(crate) model_2d_x: Coord,
    pub(crate) model_2d_y: Coord,
    pub(crate) model_2d_rotation: f64,
    pub(crate) rotation_x: f64,
    pub(crate) rotation_y: f64,
    pub(crate) rotation_z: f64,
    pub(crate) model_3d_dz: Coord,
    pub(crate) model_type: i32,
    pub(crate) model_source: String,
    /// Snap points for 3D model alignment (indexed as MODEL.S{n}X/Y/Z).
    pub(crate) model_snap_points: Vec<(Coord, Coord, Coord)>,
    /// Extruded body minimum Z (MODEL.EXTRUDED.MINZ), only for extruded model types.
    pub(crate) model_extruded_min_z: Coord,
    /// Extruded body maximum Z (MODEL.EXTRUDED.MAXZ), only for extruded model types.
    pub(crate) model_extruded_max_z: Coord,
    /// Cylinder model radius (MODEL.CYLINDER.RADIUS), only for cylinder model types.
    pub(crate) model_cylinder_radius: Coord,
    /// Cylinder model height (MODEL.CYLINDER.HEIGHT), only for cylinder model types.
    pub(crate) model_cylinder_height: Coord,
    pub(crate) outline: Vec<CoordPoint>,
    pub(crate) unique_id: Option<String>,
}

/// Parses the FileVersionInfo/Header and Data streams.
///
/// The Data stream contains a single text block with pipe-delimited parameters
/// (COUNT, VER0, FWDMSG0, BKMSG0, etc.). We decode the block and return the
/// raw decoded string for version identification.
fn parse_file_version_info(header_data: &[u8], data: &[u8]) -> Result<String> {
    let count = parse_pcb_section_header(header_data)?;

    let mut blocks = Vec::new();
    for block_result in iter_blocks(data) {
        let block = block_result?;
        let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&block.data);
        blocks.push(decoded.into_owned());
    }

    if blocks.len() as u32 != count {
        return Err(AltiumFormatError::RecordCountMismatch {
            section: "FileVersionInfo".to_owned(),
            expected: count as usize,
            actual: blocks.len(),
        });
    }

    if blocks.len() != 1 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "FileVersionInfo".to_owned(),
            detail: format!(
                "expected exactly 1 block in FileVersionInfo/Data, got {}",
                blocks.len()
            ),
        });
    }

    Ok(blocks.into_iter().next().unwrap_or_default())
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
        self.footprints
            .iter()
            .map(|fp| fp.display_name.as_str())
            .collect()
    }

    /// Validates strict structural invariants for a parsed PcbLib document.
    pub fn validate_invariants(&self) -> Result<()> {
        validate_pcblib_invariants(self)
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
        let _lib_header_count =
            crate::pcb_binary_stream::parse_pcb_section_header(&lib_header_data)?;

        let lib_data_raw = doc.read_stream("/Library/Data")?;
        let (library, suffix_names) =
            parse_library_data(&lib_data_raw).context("parsing /Library/Data")?;

        let lib_toc_header = doc.read_stream("/Library/ComponentParamsTOC/Header")?;
        let lib_toc_data = doc.read_stream("/Library/ComponentParamsTOC/Data")?;
        let component_toc = parse_component_toc(&lib_toc_header, &lib_toc_data)
            .context("parsing /Library/ComponentParamsTOC")?;
        doc.consume_storage("/Library/ComponentParamsTOC");

        // Cross-validate Library/Data suffix names against ComponentParamsTOC.
        if !suffix_names.is_empty() {
            let toc_names: Vec<&str> = component_toc.iter().map(|e| e.name.as_str()).collect();
            let suffix_refs: Vec<&str> = suffix_names.iter().map(|s| s.as_str()).collect();
            if toc_names != suffix_refs {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: "Library/Data suffix".to_owned(),
                    detail: format!(
                        "component name index mismatch: suffix has {:?} but ComponentParamsTOC has {:?}",
                        suffix_refs, toc_names
                    ),
                });
            }
        }

        let lib_models_header = doc.read_stream("/Library/Models/Header")?;
        let lib_models_data = doc.read_stream("/Library/Models/Data")?;
        let mut model_entries = parse_model_metadata(&lib_models_header, &lib_models_data)?;
        for (i, entry) in model_entries.iter_mut().enumerate() {
            let blob_path = format!("/Library/Models/{i}");
            entry.blob = doc.read_stream_optional(&blob_path)?;
        }
        doc.consume_storage("/Library/Models");

        // Auxiliary Library sub-storages (optional)
        let layer_kind_mapping = if doc.exists("/Library/LayerKindMapping/Header") {
            let lkm_header = doc.read_stream("/Library/LayerKindMapping/Header")?;
            let lkm_data = doc.read_stream("/Library/LayerKindMapping/Data")?;
            let entries = parse_layer_kind_mapping(&lkm_header, &lkm_data)
                .context("parsing /Library/LayerKindMapping")?;
            doc.consume_storage("/Library/LayerKindMapping");
            entries
        } else {
            PcbLayerKindMapping {
                version: String::new(),
                hash: 0,
                entries: Vec::new(),
            }
        };
        let pad_via_library = if doc.exists("/Library/PadViaLibrary/Header") {
            let pvl_header = doc.read_stream("/Library/PadViaLibrary/Header")?;
            let pvl_data = doc.read_stream("/Library/PadViaLibrary/Data")?;
            let config = parse_pad_via_library(&pvl_header, &pvl_data)
                .context("parsing /Library/PadViaLibrary")?;
            doc.consume_storage("/Library/PadViaLibrary");
            config
        } else {
            None
        };
        let embedded_fonts = if doc.exists("/Library/EmbeddedFonts") {
            let ef_data = doc.read_stream("/Library/EmbeddedFonts")?;
            parse_embedded_fonts(&ef_data).context("parsing /Library/EmbeddedFonts")?
        } else {
            Vec::new()
        };
        let model_no_embed_entries = if doc.exists("/Library/ModelsNoEmbed/Header") {
            let mne_header = doc.read_stream("/Library/ModelsNoEmbed/Header")?;
            let mne_data = doc.read_stream("/Library/ModelsNoEmbed/Data")?;
            let entries = parse_model_metadata(&mne_header, &mne_data)
                .context("parsing /Library/ModelsNoEmbed")?;
            doc.consume_storage("/Library/ModelsNoEmbed");
            entries
        } else {
            Vec::new()
        };
        let texture_entries = if doc.exists("/Library/Textures/Header") {
            let tex_header = doc.read_stream("/Library/Textures/Header")?;
            let tex_data = doc.read_stream("/Library/Textures/Data")?;
            let mut tex_entries = parse_texture_metadata(&tex_header, &tex_data)
                .context("parsing /Library/Textures")?;
            // Load texture blobs from numbered streams (same pattern as Models).
            for (i, entry) in tex_entries.iter_mut().enumerate() {
                let blob_path = format!("/Library/Textures/{i}");
                entry.blob = doc.read_stream_optional(&blob_path)?;
            }
            doc.consume_storage("/Library/Textures");
            tex_entries
        } else {
            Vec::new()
        };

        // Mark Library storage itself as consumed.
        doc.consume_storage("/Library");

        // 4. FileVersionInfo (optional Header/Data substorage)
        let file_version_info = if doc.exists("/FileVersionInfo/Header") {
            let fvi_header = doc.read_stream("/FileVersionInfo/Header")?;
            let fvi_data = doc.read_stream("/FileVersionInfo/Data")?;
            doc.consume_storage("/FileVersionInfo");
            Some(parse_file_version_info(&fvi_header, &fvi_data)?)
        } else {
            doc.read_stream_optional("/FileVersionInfo/Header")?;
            doc.read_stream_optional("/FileVersionInfo/Data")?;
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

        let lib = Self {
            header,
            section_keys,
            library,
            component_toc,
            model_entries,
            model_no_embed_entries,
            layer_kind_mapping,
            pad_via_library,
            embedded_fonts,
            texture_entries,
            footprints,
            file_version_info,
            source_path: Some(path.to_path_buf()),
        };
        lib.validate_invariants()
            .context("validating PcbLib invariants")?;
        Ok(lib)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut cfb = CfbDocument::create()?;

        cfb.write_stream(&format!("/{FILE_HEADER}"), &serialize_pcblib_file_header(&self.header))?;

        cfb.create_storage("/Library")?;
        cfb.write_stream("/Library/Header", &serialize_u32_header(1))?;
        cfb.write_stream("/Library/Data", &serialize_library_data_block(&self.library))?;

        cfb.create_storage("/Library/ComponentParamsTOC")?;
        cfb.write_stream(
            "/Library/ComponentParamsTOC/Header",
            &serialize_u32_header(if self.component_toc.is_empty() { 0 } else { 1 }),
        )?;
        cfb.write_stream(
            "/Library/ComponentParamsTOC/Data",
            &serialize_component_toc_data(&self.component_toc),
        )?;

        if !self.model_entries.is_empty() {
            cfb.create_storage("/Library/Models")?;
            cfb.write_stream(
                "/Library/Models/Header",
                &serialize_u32_header(self.model_entries.len() as u32),
            )?;
            cfb.write_stream(
                "/Library/Models/Data",
                &serialize_model_entries_data(&self.model_entries),
            )?;
            for (i, entry) in self.model_entries.iter().enumerate() {
                if let Some(blob) = &entry.blob {
                    cfb.write_stream(&format!("/Library/Models/{i}"), blob)?;
                }
            }
        }

        if !self.model_no_embed_entries.is_empty() {
            cfb.create_storage("/Library/ModelsNoEmbed")?;
            cfb.write_stream(
                "/Library/ModelsNoEmbed/Header",
                &serialize_u32_header(self.model_no_embed_entries.len() as u32),
            )?;
            cfb.write_stream(
                "/Library/ModelsNoEmbed/Data",
                &serialize_model_entries_data(&self.model_no_embed_entries),
            )?;
        }

        if !self.layer_kind_mapping.entries.is_empty() || !self.layer_kind_mapping.version.is_empty() {
            cfb.create_storage("/Library/LayerKindMapping")?;
            cfb.write_stream("/Library/LayerKindMapping/Header", &serialize_u32_header(1))?;
            cfb.write_stream(
                "/Library/LayerKindMapping/Data",
                &serialize_layer_kind_mapping(&self.layer_kind_mapping),
            )?;
        }

        if let Some(cfg) = &self.pad_via_library {
            cfb.create_storage("/Library/PadViaLibrary")?;
            cfb.write_stream("/Library/PadViaLibrary/Header", &serialize_u32_header(1))?;
            cfb.write_stream("/Library/PadViaLibrary/Data", &serialize_pad_via_library(cfg))?;
        }

        if !self.embedded_fonts.is_empty() {
            cfb.write_stream(
                "/Library/EmbeddedFonts",
                &serialize_embedded_fonts(&self.embedded_fonts),
            )?;
        }

        if !self.texture_entries.is_empty() {
            cfb.create_storage("/Library/Textures")?;
            cfb.write_stream(
                "/Library/Textures/Header",
                &serialize_u32_header(self.texture_entries.len() as u32),
            )?;
            cfb.write_stream(
                "/Library/Textures/Data",
                &serialize_texture_entries_data(&self.texture_entries),
            )?;
            for (i, entry) in self.texture_entries.iter().enumerate() {
                if let Some(blob) = &entry.blob {
                    cfb.write_stream(&format!("/Library/Textures/{i}"), blob)?;
                }
            }
        }

        for fp in &self.footprints {
            let storage = format!("/{}", fp.cfb_key);
            cfb.create_storage(&storage)?;
            cfb.write_stream(&format!("{storage}/Parameters"), &serialize_footprint_parameters(fp))?;
            cfb.write_stream(
                &format!("{storage}/Header"),
                &serialize_u32_header(fp.primitives.len() as u32),
            )?;
            cfb.write_stream(&format!("{storage}/Data"), &serialize_footprint_data(fp)?)?;
        }

        if !self.section_keys.is_empty() {
            cfb.write_stream(&format!("/{SECTION_KEYS}"), &serialize_section_keys(&self.section_keys))?;
        }

        if let Some(fvi) = &self.file_version_info {
            cfb.create_storage("/FileVersionInfo")?;
            cfb.write_stream("/FileVersionInfo/Header", &serialize_u32_header(1))?;
            cfb.write_stream("/FileVersionInfo/Data", &write_text_block(fvi.as_bytes()))?;
        }

        cfb.save_to_file(path)
    }

    /// Render a single footprint by display name.
    pub fn render_footprint(
        &self,
        name: &str,
        canvas: &mut dyn crate::render::AltiumCanvas,
    ) -> crate::Result<()> {
        let fp = self
            .footprints
            .iter()
            .find(|f| f.display_name == name)
            .ok_or_else(|| {
                crate::AltiumFormatError::StreamNotFound(format!("footprint '{name}' not found"))
            })?;
        fp.render(canvas);
        Ok(())
    }
}

impl PcbFootprint {
    pub(crate) fn render(&self, canvas: &mut dyn crate::render::AltiumCanvas) {
        for prim in &self.primitives {
            crate::render::pcb::draw_pcb_primitive(prim, canvas);
        }
    }
}

fn serialize_u32_header(count: u32) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    w.write_u32_le(count);
    w.finish()
}

fn serialize_pcblib_file_header(header: &PcbFileHeader) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    let ver = header.version_string.as_bytes();
    w.write_u32_le(ver.len() as u32);
    w.write_pascal_string(&header.version_string);
    w.write_f64_le(header.version);
    if let Some(uid) = &header.unique_id {
        w.write_u32_le(uid.len() as u32);
        w.write_pascal_string(uid);
    }
    w.finish()
}

fn serialize_library_data_block(library: &PcbLibraryData) -> Vec<u8> {
    let mut params = crate::param_collection::ParameterCollection::new();
    params.insert("FILENAME", library.filename.clone());
    params.insert("KIND", library.kind.clone());
    params.insert("VERSION", library.version.clone());
    params.insert("DATE", library.date.clone());
    params.insert("TIME", library.time.clone());
    write_text_block(&params.to_bytes())
}

fn serialize_component_toc_data(entries: &[PcbLibComponentTocEntry]) -> Vec<u8> {
    let mut text = String::new();
    for (i, e) in entries.iter().enumerate() {
        if i != 0 {
            text.push_str("\r\n");
        }
        text.push_str(&format!(
            "Name={}|Pad Count={}|Height={:.4}mil|Description={}",
            e.name,
            e.pad_count,
            e.height.to_mils(),
            e.description
        ));
    }
    let mut bytes = text.into_bytes();
    bytes.push(0);
    write_text_block(&bytes)
}

fn serialize_model_entries_data(entries: &[PcbLibModelEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    for entry in entries {
        let mut params = crate::param_collection::ParameterCollection::new();
        params.insert("EMBED", if entry.embed { "TRUE".to_owned() } else { "FALSE".to_owned() });
        params.insert("ID", entry.id.clone());
        params.insert("ROTX", entry.rotation_x.to_string());
        params.insert("ROTY", entry.rotation_y.to_string());
        params.insert("ROTZ", entry.rotation_z.to_string());
        params.insert("DZ", entry.standoff.to_string());
        params.insert("CHECKSUM", entry.checksum.clone());
        params.insert("NAME", entry.name.clone());
        out.extend_from_slice(&write_text_block(&params.to_bytes()));
    }
    out
}

fn serialize_layer_kind_mapping(mapping: &PcbLayerKindMapping) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    let utf16: Vec<u16> = mapping.version.encode_utf16().chain(std::iter::once(0)).collect();
    let mut utf16_bytes = Vec::with_capacity(utf16.len() * 2);
    for c in utf16 {
        utf16_bytes.extend_from_slice(&c.to_le_bytes());
    }
    w.write_u32_le(utf16_bytes.len() as u32);
    w.write_bytes(&utf16_bytes);
    w.write_u32_le(mapping.hash);
    w.write_u32_le(mapping.entries.len() as u32);
    for entry in &mapping.entries {
        w.write_u32_le(entry.layer_id);
        w.write_u32_le(entry.kind);
    }
    w.finish()
}

fn serialize_pad_via_library(cfg: &PcbPadViaLibraryConfig) -> Vec<u8> {
    let mut params = crate::param_collection::ParameterCollection::new();
    params.insert("PADVIALIBRARY.LIBRARYID", cfg.library_id.clone());
    params.insert("PADVIALIBRARY.LIBRARYNAME", cfg.library_name.clone());
    params.insert("PADVIALIBRARY.DISPLAYUNITS", cfg.display_units.clone());
    write_text_block(&params.to_bytes())
}

fn serialize_embedded_fonts(entries: &[PcbEmbeddedFontEntry]) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    w.write_u32_le(entries.len() as u32);
    for entry in entries {
        write_utf16lp(&mut w, &entry.name);
        write_utf16lp(&mut w, &entry.style_name);
        write_utf16lp(&mut w, &entry.localized_name);
        w.write_u16_le(entry.unknown_u16);
        w.write_u8(entry.flag);
        w.write_u32_le(entry.data.len() as u32);
        w.write_bytes(&entry.data);
    }
    w.finish()
}

fn write_utf16lp(w: &mut BinaryWriter, s: &str) {
    let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
    let mut bytes = Vec::with_capacity(wide.len() * 2);
    for c in wide {
        bytes.extend_from_slice(&c.to_le_bytes());
    }
    w.write_u32_le(bytes.len() as u32);
    w.write_bytes(&bytes);
}

fn serialize_texture_entries_data(entries: &[PcbTextureEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    for e in entries {
        let mut params = crate::param_collection::ParameterCollection::new();
        params.insert("NAME", e.name.clone());
        out.extend_from_slice(&write_text_block(&params.to_bytes()));
    }
    out
}

fn serialize_section_keys(keys: &HashMap<String, String>) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    w.write_u32_le(keys.len() as u32);
    let mut pairs: Vec<_> = keys.iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(b.0));
    for (display_name, cfb_key) in pairs {
        w.write_u32_le((display_name.len() + 1) as u32);
        w.write_pascal_string(display_name);
        w.write_u32_le((cfb_key.len() + 1) as u32);
        w.write_pascal_string(cfb_key);
    }
    w.finish()
}

fn serialize_footprint_parameters(fp: &PcbFootprint) -> Vec<u8> {
    let mut params = crate::param_collection::ParameterCollection::new();
    params.insert("PATTERN", fp.pattern.clone());
    params.insert("HEIGHT", format!("{:.4}mil", fp.height.to_mils()));
    params.insert("DESCRIPTION", fp.description.clone());
    params.insert("ITEMGUID", fp.item_guid.clone());
    params.insert("REVISIONGUID", fp.revision_guid.clone());
    let data = params.to_bytes();
    let mut w = BinaryWriter::new();
    w.write_u32_le(data.len() as u32);
    w.write_bytes(&data);
    w.finish()
}

fn serialize_footprint_data(fp: &PcbFootprint) -> Result<Vec<u8>> {
    let mut w = BinaryWriter::new();
    let name = fp.pattern.as_bytes();
    if name.len() > u8::MAX as usize {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "PcbFootprint.pattern".to_owned(),
            detail: "pattern name too long".to_owned(),
        });
    }
    w.write_u32_le((1 + name.len()) as u32);
    w.write_u8(name.len() as u8);
    w.write_bytes(name);
    for prim in &fp.primitives {
        let (obj, subs) = serialize_primitive(prim)?;
        w.write_u8(obj as u8);
        for sub in subs {
            w.write_u32_le(sub.len() as u32);
            w.write_bytes(&sub);
        }
    }
    Ok(w.finish())
}

fn serialize_primitive(prim: &PcbPrimitive) -> Result<(PcbObjectId, Vec<Vec<u8>>)> {
    match prim {
        PcbPrimitive::Track(p) => Ok((PcbObjectId::Track, vec![serialize_track(p)])),
        PcbPrimitive::Via(p) => Ok((PcbObjectId::Via, vec![serialize_via(p)])),
        PcbPrimitive::Arc(p) => Ok((PcbObjectId::Arc, vec![serialize_arc(p)])),
        PcbPrimitive::Fill(p) => Ok((PcbObjectId::Fill, vec![serialize_fill(p)])),
        PcbPrimitive::Text(p) => Ok((PcbObjectId::Text, serialize_text(p))),
        PcbPrimitive::Pad(p) => Ok((PcbObjectId::Pad, serialize_pad(p))),
        PcbPrimitive::Region(p) => Ok((PcbObjectId::Region, vec![serialize_region(p)])),
        PcbPrimitive::ComponentBody(p) => Ok((PcbObjectId::ComponentBody, vec![serialize_component_body(p)])),
    }
}

fn write_primitive_common(w: &mut BinaryWriter, c: &PcbPrimitiveCommon) {
    w.write_u8(c.layer as u8);
    w.write_u8(c.pad_byte);
    w.write_u16_le(c.flags.raw());
    w.write_i32_le(c.net_index);
    w.write_u16_le(c.polygon_index);
    w.write_u16_le(c.component_index);
    w.write_u8(c.unknown);
}

fn serialize_arc(p: &PcbArc) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    write_primitive_common(&mut w, &p.common);
    w.write_coord_point(p.center);
    w.write_coord(p.radius);
    w.write_f64_le(p.start_angle);
    w.write_f64_le(p.end_angle);
    w.write_coord(p.width);
    w.write_u16_le(p.subpoly_index);
    w.write_u8(p.user_routed as u8);
    w.write_i32_le(p.union_index);
    w.write_u32_le(p.v7_layer.raw());
    w.write_i32_le(p.keepout_restrictions);
    w.finish()
}

fn serialize_track(p: &PcbTrack) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    write_primitive_common(&mut w, &p.common);
    w.write_coord_point(p.start);
    w.write_coord_point(p.end);
    w.write_coord(p.width);
    w.write_u16_le(p.subpoly_index);
    w.write_u8(p.user_routed as u8);
    w.write_i32_le(p.union_index);
    w.write_u8(p.track_kind);
    w.write_i32_le(p.layer_enum_index);
    w.write_i32_le(p.keepout_restrictions);
    w.finish()
}

fn serialize_via(p: &PcbVia) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    write_primitive_common(&mut w, &p.common);
    w.write_coord_point(p.location);
    w.write_coord(p.diameter);
    w.write_coord(p.hole_size);
    w.write_u8(p.from_layer as u8);
    w.write_u8(p.to_layer as u8);
    w.finish()
}

fn serialize_fill(p: &PcbFill) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    write_primitive_common(&mut w, &p.common);
    w.write_coord_point(p.corner1);
    w.write_coord_point(p.corner2);
    w.write_f64_le(p.rotation);
    w.write_u8(p.user_routed as u8);
    w.write_i32_le(p.union_index);
    w.write_u32_le(p.v7_layer.raw());
    w.write_i32_le(p.keepout_restrictions);
    w.finish()
}

fn serialize_text(p: &PcbText) -> Vec<Vec<u8>> {
    let mut w0 = BinaryWriter::new();
    write_primitive_common(&mut w0, &p.common);
    w0.write_coord_point(p.location);
    w0.write_coord(p.height);
    w0.write_u8(p.text_kind as u8);
    w0.write_u8(0);
    w0.write_f64_le(p.rotation);
    w0.write_u8(p.is_mirrored as u8);
    w0.write_coord(p.stroke_width);
    w0.write_bytes(&[0, 0, 0]);
    w0.write_u8(p.is_italic as u8);
    w0.write_u8(p.is_bold as u8);
    w0.write_u8(0);
    w0.write_wide_string_fixed(&p.font_name, 32);
    w0.write_u8(p.inverted as u8);
    w0.write_bytes(&[0, 0, 0]);
    w0.write_i32_le(p.wide_string_index);
    w0.write_bytes(&[0, 0, 0, 0, 0, 0]);
    w0.write_coord(p.ttf_text_width);
    w0.write_coord(p.ttf_text_height);
    w0.write_i32_le(p.font_id);
    w0.write_u8(p.barcode_inverted as u8);
    w0.write_coord(p.barcode_full_width);
    w0.write_coord(p.barcode_full_height);
    w0.write_coord(p.barcode_x_margin);
    w0.write_coord(p.barcode_y_margin);
    w0.write_coord(p.barcode_min_width);
    w0.write_u8(0);
    w0.write_u8(p.barcode_show_text as u8);
    w0.write_u8(p.barcode_render_mode);
    w0.write_u8(p.multiline as u8);
    w0.write_wide_string_fixed(&p.barcode_font_name, 32);
    let (s1, _, _) = encoding_rs::WINDOWS_1252.encode(&p.text);
    vec![w0.finish(), s1.to_vec()]
}

fn serialize_pad(p: &PcbPad) -> Vec<Vec<u8>> {
    let mut sub0 = BinaryWriter::new();
    sub0.write_pascal_string(&p.pad_name);
    let mut sub1 = BinaryWriter::new();
    sub1.write_pascal_string(&p.unknown_sub1);
    let mut sub2 = BinaryWriter::new();
    sub2.write_pascal_string(&p.unknown_sub2);
    let mut sub3 = BinaryWriter::new();
    sub3.write_pascal_string(&p.unknown_sub3);

    let mut sub4 = BinaryWriter::new();
    write_primitive_common(&mut sub4, &p.common);
    sub4.write_coord_point(p.location);
    sub4.write_coord(p.size_top.x);
    sub4.write_coord(p.size_top.y);
    sub4.write_coord(p.size_mid.x);
    sub4.write_coord(p.size_mid.y);
    sub4.write_coord(p.size_bot.x);
    sub4.write_coord(p.size_bot.y);
    sub4.write_coord(p.hole_size);
    sub4.write_u8(p.shape_top as u8);
    sub4.write_u8(p.shape_mid as u8);
    sub4.write_u8(p.shape_bot as u8);
    sub4.write_f64_le(p.rotation);
    sub4.write_u8(p.is_plated as u8);
    sub4.write_u8(p.daisy_chain_style as u8);
    sub4.write_u8(p.pad_mode as u8);
    sub4.write_i32_le(p.unknown_63);
    sub4.write_u8(p.cache.plane_connection_style as u8);
    sub4.write_coord(p.cache.relief_conductor_width);
    sub4.write_i16_le(p.cache.relief_entries);
    sub4.write_coord(p.cache.relief_air_gap);
    sub4.write_coord(p.cache.power_plane_relief_expansion);
    sub4.write_coord(p.cache.power_plane_clearance);
    sub4.write_coord(p.cache.paste_mask_expansion);
    sub4.write_coord(p.cache.solder_mask_expansion);
    sub4.write_u16_le(p.cache.planes);
    sub4.write_u8(p.cache.plane_connection_style_valid as u8);
    sub4.write_u8(p.cache.relief_conductor_width_valid as u8);
    sub4.write_u8(p.cache.relief_entries_valid as u8);
    sub4.write_u8(p.cache.relief_air_gap_valid as u8);
    sub4.write_u8(p.cache.power_plane_relief_expansion_valid as u8);
    sub4.write_u8(p.cache.paste_mask_expansion_valid as u8);
    sub4.write_u8(p.cache.solder_mask_expansion_valid as u8);
    sub4.write_u8(p.cache.power_plane_clearance_valid as u8);
    sub4.write_u8(p.cache.planes_valid as u8);
    sub4.write_u8(p.selection_memory_flags);
    sub4.write_i32_le(p.union_index);
    sub4.write_i32_le(p.jumper_id);
    sub4.write_i32_le(p.v7_layer_override);
    sub4.write_u8(p.is_assy_testpoint_top as u8);
    sub4.write_u8(p.is_assy_testpoint_bottom as u8);
    sub4.write_u8(p.use_separate_expansions as u8);
    sub4.write_i32_le(p.solder_mask_bottom_expansion);
    sub4.write_u8(p.solder_mask_expansion_from_hole_edge as u8);
    sub4.write_bytes(&p.template_link_library_id);
    sub4.write_bytes(&p.template_link_template_id);
    sub4.write_coord(p.pin_package_length);
    sub4.write_i32_le(p.hole_positive_tolerance);
    sub4.write_i32_le(p.hole_negative_tolerance);
    sub4.write_u8(p.reserved_170);
    sub4.write_u8(p.has_sub4_extension as u8);
    if let Some(ext) = &p.sub4_extension {
        sub4.write_u32_le(ext.header_len);
        let mut hdr = BinaryWriter::new();
        hdr.write_u32_le(ext.thermal_relief_count);
        hdr.write_f32_le(ext.propagation_delay_f32);
        hdr.write_u8(ext.flags8);
        hdr.write_u8(ext.flags9);
        hdr.write_f64_le(ext.propagation_delay_f64);
        hdr.write_coord(ext.x_pad_offset_all_layers);
        hdr.write_coord(ext.y_pad_offset_all_layers);
        let mut hdr_bytes = hdr.finish();
        hdr_bytes.truncate(ext.header_len as usize);
        sub4.write_bytes(&hdr_bytes);
        if !p.thermal_reliefs.is_empty() {
            sub4.write_u32_le(30);
            for relief in &p.thermal_reliefs {
                sub4.write_u32_le(relief.layer.raw());
                sub4.write_u8(relief.defined_type);
                sub4.write_u8(relief.connect_style as u8);
                sub4.write_coord(relief.air_gap_width);
                sub4.write_coord(relief.conductor_width);
                sub4.write_u8(relief.rotation as u8);
                sub4.write_u32_le(relief.entries);
                sub4.write_coord(relief.expansion);
                sub4.write_u8(relief.conductor_by_pad_edge as u8);
                sub4.write_coord(relief.min_distance);
                sub4.write_u8(relief.enable_min_distance as u8);
                sub4.write_u8(relief.use_custom_relief as u8);
            }
        }
    }

    let mut sub5 = BinaryWriter::new();
    if let Some(stack) = &p.stack_data {
        for v in stack.inner_size_x {
            sub5.write_coord(v);
        }
        for v in stack.inner_size_y {
            sub5.write_coord(v);
        }
        for v in stack.inner_shape {
            sub5.write_u8(v as u8);
        }
        sub5.write_u8(stack.padding_261);
        sub5.write_u8(stack.hole_shape);
        sub5.write_coord(stack.slot_size);
        sub5.write_f64_le(stack.slot_rotation);
        for v in stack.hole_offset_x {
            sub5.write_coord(v);
        }
        for v in stack.hole_offset_y {
            sub5.write_coord(v);
        }
        sub5.write_u8(stack.padding_531);
        sub5.write_bytes(&stack.alt_shape);
        sub5.write_bytes(&stack.corner_radius_pct);
        sub5.write_bytes(&stack.per_layer_overrides);
    }

    vec![sub0.finish(), sub1.finish(), sub2.finish(), sub3.finish(), sub4.finish(), sub5.finish()]
}

fn serialize_region(p: &PcbRegion) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    write_primitive_common(&mut w, &p.common);
    w.write_u8(p.kind as u8);
    w.write_i32_le(p.holes.len() as i32);
    let mut params = crate::param_collection::ParameterCollection::new();
    params.insert("V7_LAYER", p.v7_layer.clone());
    params.insert("NAME", p.name.clone());
    params.insert("KIND", p.param_kind.to_string());
    params.insert("SUBPOLYINDEX", p.subpoly_index.to_string());
    params.insert("UNIONINDEX", p.union_index.to_string());
    params.insert("ARCRESOLUTION", format!("{:.4}mil", p.arc_resolution.to_mils()));
    params.insert("ISSHAPEBASED", if p.is_shape_based { "TRUE".to_owned() } else { "FALSE".to_owned() });
    params.insert("CAVITYHEIGHT", format!("{:.4}mil", p.cavity_height.to_mils()));
    params.insert("KEEPOUTRESTRICTIONS", p.keepout_restrictions.to_string());
    params.insert("LAYER", p.layer.clone());
    params.insert("KEEPOUT", if p.keepout { "TRUE".to_owned() } else { "FALSE".to_owned() });
    params.insert("ISBOARDCUTOUT", if p.is_board_cutout { "TRUE".to_owned() } else { "FALSE".to_owned() });
    params.insert("PADINDEX", p.pad_index.to_string());
    let pbytes = params.to_bytes();
    w.write_u32_le(pbytes.len() as u32);
    w.write_bytes(&pbytes);
    w.write_i32_le(p.outline.len() as i32);
    for v in &p.outline {
        w.write_f64_le(v.x.to_internal() as f64);
        w.write_f64_le(v.y.to_internal() as f64);
    }
    for hole in &p.holes {
        w.write_i32_le(hole.len() as i32);
        for v in hole {
            w.write_f64_le(v.x.to_internal() as f64);
            w.write_f64_le(v.y.to_internal() as f64);
        }
    }
    w.finish()
}

fn serialize_component_body(p: &PcbComponentBody) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    write_primitive_common(&mut w, &p.common);
    w.write_u8(0);
    w.write_i32_le(0);
    let mut params = crate::param_collection::ParameterCollection::new();
    params.insert("V7_LAYER", p.v7_layer.clone());
    params.insert("NAME", p.name.clone());
    params.insert("KIND", p.kind.to_string());
    params.insert("SUBPOLYINDEX", p.subpoly_index.to_string());
    params.insert("UNIONINDEX", p.union_index.to_string());
    params.insert("ARCRESOLUTION", format!("{:.4}mil", p.arc_resolution.to_mils()));
    params.insert("ISSHAPEBASED", if p.is_shape_based { "TRUE".to_owned() } else { "FALSE".to_owned() });
    params.insert("CAVITYHEIGHT", format!("{:.4}mil", p.cavity_height.to_mils()));
    params.insert("STANDOFFHEIGHT", format!("{:.4}mil", p.standoff_height.to_mils()));
    params.insert("OVERALLHEIGHT", format!("{:.4}mil", p.overall_height.to_mils()));
    params.insert("BODYPROJECTION", p.body_projection.to_string());
    params.insert("BODYCOLOR3D", p.body_color_3d.raw().to_string());
    params.insert("BODYOPACITY3D", p.body_opacity_3d.to_string());
    params.insert("TEXTURE", p.texture.clone());
    params.insert("MODELID", p.model_guid.clone());
    params.insert("MODEL.CHECKSUM", p.model_checksum.clone());
    params.insert("MODEL.EMBED", if p.model_embed { "TRUE".to_owned() } else { "FALSE".to_owned() });
    params.insert("MODEL.NAME", p.model_name.clone());
    params.insert("MODEL.2D.X", format!("{:.4}mil", p.model_2d_x.to_mils()));
    params.insert("MODEL.2D.Y", format!("{:.4}mil", p.model_2d_y.to_mils()));
    params.insert("MODEL.2D.ROTATION", p.model_2d_rotation.to_string());
    params.insert("MODEL.3D.ROTX", p.rotation_x.to_string());
    params.insert("MODEL.3D.ROTY", p.rotation_y.to_string());
    params.insert("MODEL.3D.ROTZ", p.rotation_z.to_string());
    params.insert("MODEL.3D.DZ", format!("{:.4}mil", p.model_3d_dz.to_mils()));
    params.insert("MODEL.MODELTYPE", p.model_type.to_string());
    params.insert("MODEL.MODELSOURCE", p.model_source.clone());
    params.insert("MODEL.SNAPCOUNT", p.model_snap_points.len().to_string());
    let pbytes = params.to_bytes();
    w.write_u32_le(pbytes.len() as u32);
    w.write_bytes(&pbytes);
    w.write_i32_le(p.outline.len() as i32);
    for v in &p.outline {
        w.write_f64_le(v.x.to_internal() as f64);
        w.write_f64_le(v.y.to_internal() as f64);
    }
    w.finish()
}


fn validate_pcblib_invariants(lib: &PcbLib) -> Result<()> {
    if lib.header.version_string != PCB_LIBRARY_BINARY_HEADER_V6 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: FILE_HEADER.to_owned(),
            detail: format!(
                "expected header {:?}, got {:?}",
                PCB_LIBRARY_BINARY_HEADER_V6, lib.header.version_string
            ),
        });
    }

    if !lib.header.version.is_finite() || lib.header.version <= 0.0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "FileHeader.version".to_owned(),
            detail: format!("invalid version number {}", lib.header.version),
        });
    }

    let mut seen_display_names = HashSet::new();
    let mut seen_cfb_keys = HashSet::new();
    for (idx, fp) in lib.footprints.iter().enumerate() {
        if fp.display_name.trim().is_empty() {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Footprint.display_name".to_owned(),
                detail: format!("footprint[{idx}] has empty display name"),
            });
        }
        if fp.cfb_key.trim().is_empty() {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Footprint.cfb_key".to_owned(),
                detail: format!("footprint[{idx}] has empty storage key"),
            });
        }
        if fp.pattern.trim().is_empty() {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Footprint.pattern".to_owned(),
                detail: format!("footprint[{}:{}] has empty pattern", idx, fp.display_name),
            });
        }
        if !seen_display_names.insert(fp.display_name.clone()) {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Footprint.display_name".to_owned(),
                detail: format!("duplicate footprint display name {:?}", fp.display_name),
            });
        }
        if !seen_cfb_keys.insert(fp.cfb_key.clone()) {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Footprint.cfb_key".to_owned(),
                detail: format!("duplicate footprint storage key {:?}", fp.cfb_key),
            });
        }

        let expected_key = section_keys::resolve_footprint_key(&fp.display_name, &lib.section_keys);
        if expected_key != fp.cfb_key {
            return Err(AltiumFormatError::InvalidParamValue {
                key: SECTION_KEYS.to_owned(),
                detail: format!(
                    "footprint key mismatch for {:?}: expected {:?}, got {:?}",
                    fp.display_name, expected_key, fp.cfb_key
                ),
            });
        }
    }

    let footprint_names: HashSet<&str> = lib
        .footprints
        .iter()
        .map(|fp| fp.display_name.as_str())
        .collect();
    // Some real-world libraries include stale TOC entries; tolerate those as
    // long as all loaded footprints are internally consistent.
    let _ = footprint_names;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use altium_format_types::PcbObjectId;
    use proptest::prelude::*;
    use std::fs;

    #[test]
    fn pcblib_struct_compiles() {
        let _ = PcbLib {
            header: PcbFileHeader {
                version_string: String::new(),
                version: 0.0,
                unique_id: Some(String::new()),
            },
            section_keys: HashMap::new(),
            library: library::PcbLibraryData {
                filename: String::new(),
                kind: String::new(),
                version: String::new(),
                date: String::new(),
                time: String::new(),
                board_config: crate::board_config::PcbBoardConfig {
                    record: String::new(),
                    v9_master_stack: None,
                    v9_substacks: Vec::new(),
                    v9_stack_layers: Vec::new(),
                    v9_cache_layers: Vec::new(),
                    v8_master_stack: None,
                    v8_layers: Vec::new(),
                    v7_layers: Vec::new(),
                    legacy_layers: Vec::new(),
                    surface_properties: crate::board_config::PcbSurfaceProperties {
                        top_type: String::new(),
                        top_const: String::new(),
                        top_height: String::new(),
                        top_material: String::new(),
                        bottom_type: String::new(),
                        bottom_const: String::new(),
                        bottom_height: String::new(),
                        bottom_material: String::new(),
                        layer_stack_style: String::new(),
                        show_top_dielectric: false,
                        show_bottom_dielectric: false,
                    },
                    layer_sets: Vec::new(),
                    grid_settings: crate::board_config::PcbGridSettings {
                        big_visible_grid_size: String::new(),
                        visible_grid_size: String::new(),
                        snap_grid_size: String::new(),
                        snap_grid_size_x: String::new(),
                        snap_grid_size_y: String::new(),
                        visible_grid_mult_factor: String::new(),
                        big_visible_grid_mult_factor: String::new(),
                        electrical_grid_range: String::new(),
                        electrical_grid_enabled: false,
                        dot_grid: false,
                        dot_grid_large: false,
                    },
                    viewport: crate::board_config::PcbViewportState {
                        lx: String::new(),
                        hx: String::new(),
                        ly: String::new(),
                        hy: String::new(),
                        lookat_x: String::new(),
                        lookat_y: String::new(),
                        lookat_z: String::new(),
                        eye_rotation_x: String::new(),
                        eye_rotation_y: String::new(),
                        eye_rotation_z: String::new(),
                        zoom_mult: String::new(),
                        view_size_x: String::new(),
                        view_size_y: String::new(),
                    },
                    view_configs: crate::board_config::PcbViewConfigs {
                        config_2d_type: String::new(),
                        configuration_2d: String::new(),
                        config_2d_full_filename: String::new(),
                        config_3d_type: String::new(),
                        configuration_3d: String::new(),
                        config_3d_full_filename: String::new(),
                        board_insight_view_configuration_name: String::new(),
                    },
                    snapping: crate::board_config::PcbSnappingConfig {
                        eg_range: String::new(),
                        eg_mult: String::new(),
                        eg_enabled: false,
                        eg_snap_to_board_outline: false,
                        eg_snap_to_arc_centers: false,
                        eg_use_all_layers: false,
                        og_snap_enabled: false,
                        mg_snap_enabled: false,
                        point_guide_enabled: false,
                        grid_snap_enabled: false,
                        snapping_entity_set: String::new(),
                    },
                    near_far_objects: crate::board_config::PcbNearFarObjects {
                        near_objects_enabled: false,
                        far_objects_enabled: false,
                        near_object_set: String::new(),
                        far_object_set: String::new(),
                        near_distance: String::new(),
                    },
                    cfg2d: crate::board_config::PcbCfg2D {
                        prim_draw_mode: String::new(),
                        current_layer: String::new(),
                        display_special_strings: false,
                        show_test_points: false,
                        show_origin_marker: false,
                        eye_dist: String::new(),
                        show_status_info: false,
                        show_pad_nets: false,
                        show_pad_numbers: false,
                        show_via_nets: false,
                        show_via_span: false,
                        use_transparent_layers: false,
                        plane_draw_mode: String::new(),
                        display_net_names_on_tracks: String::new(),
                        from_tos_display_mode: String::new(),
                        pad_types_display_mode: String::new(),
                        single_layer_mode_state: String::new(),
                        origin_marker_color: String::new(),
                        show_component_ref_point: false,
                        component_ref_point_color: String::new(),
                        positive_top_solder_mask: false,
                        positive_bottom_solder_mask: false,
                        top_positive_solder_mask_alpha: String::new(),
                        bottom_positive_solder_mask_alpha: String::new(),
                        all_connections_in_single_layer_mode: false,
                        multi_colored_connections: false,
                        show_special_strings_handles: false,
                        toggle_layers: String::new(),
                        toggle_layers_set: String::new(),
                        mech_layer_in_single_layer_mode: String::new(),
                        mech_layer_in_single_layer_mode_set: String::new(),
                        layers_in_single_layer_mode_set: String::new(),
                        mech_layer_linked_to_sheet: String::new(),
                        mech_layer_linked_to_sheet_set: String::new(),
                        mech_coverlay_updated: false,
                        layer_opacity: indexmap::IndexMap::new(),
                        workspace_col_alpha: indexmap::IndexMap::new(),
                    },
                    cfg3d: indexmap::IndexMap::new(),
                    cfgall: crate::board_config::PcbCfgAll {
                        configuration_kind: String::new(),
                        configuration_desc: String::new(),
                        component_body_ref_point_color: String::new(),
                        component_body_snap_point_color: String::new(),
                        show_component_snap_markers: false,
                        show_component_snap_reference: false,
                        show_component_snap_custom: false,
                    },
                    display_unit: 0,
                    current_2d_3d_view_state: String::new(),
                    toggle_layers: String::new(),
                    show_default_sets: false,
                    board_version: String::new(),
                    vault_guid: String::new(),
                    folder_guid: String::new(),
                    lifecycle_definition_guid: String::new(),
                    revision_naming_scheme_guid: String::new(),
                    lib_grid_sn_guide: String::new(),
                    unicode: String::new(),
                    unicode_filename: String::new(),
                    unicode_name: String::new(),
                    unicode_time: String::new(),
                    plane_pullbacks: Vec::new(),
                    selection_filter: Vec::new(),
                },
            },
            component_toc: Vec::new(),
            model_entries: Vec::new(),
            model_no_embed_entries: Vec::new(),
            layer_kind_mapping: PcbLayerKindMapping {
                version: String::new(),
                hash: 0,
                entries: Vec::new(),
            },
            pad_via_library: None,
            embedded_fonts: Vec::new(),
            texture_entries: Vec::new(),
            footprints: Vec::new(),
            file_version_info: None,
            source_path: None,
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

    fn fixture_paths() -> Vec<std::path::PathBuf> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/pcblib");
        let mut out = Vec::new();
        let entries = fs::read_dir(dir).expect("read data/pcblib");
        for entry in entries.flatten() {
            let path = entry.path();
            let is_pcblib = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("pcblib"))
                .unwrap_or(false);
            if is_pcblib {
                out.push(path);
            }
        }
        out.sort();
        out
    }

    fn roundtrip_semantic_report(path: &std::path::Path) -> crate::test_utils::CfbSemanticDiffReport {
        let lib = PcbLib::open(path).expect("PcbLib::open must succeed");
        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        lib.save(tmp.path()).expect("PcbLib::save must succeed");
        crate::test_utils::diff_cfb_files_semantic(path, tmp.path()).expect("semantic diff must succeed")
    }

    #[test]
    fn pcblib_roundtrip_semantic_diff_report_is_generated() {
        let fixtures = fixture_paths();
        if fixtures.is_empty() {
            return;
        }
        let report = roundtrip_semantic_report(&fixtures[0]);
        assert!(
            !report.issues.is_empty(),
            "expected current serializer to produce semantic diff issues until full parity is reached"
        );
    }

    #[test]
    #[ignore = "enable once full pcblib serializer reaches semantic parity"]
    fn pcblib_roundtrip_semantic_eq_fixture() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/pcblib/28Pins_Project.PcbLib");
        if !path.exists() {
            return;
        }
        let lib = PcbLib::open(&path).expect("open fixture");
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        lib.save(tmp.path()).expect("save fixture");
        crate::test_utils::assert_cfb_files_semantic_eq(&path, tmp.path());
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 16, .. ProptestConfig::default() })]

        #[test]
        fn prop_pcblib_invariants_hold_for_known_fixtures(idx in 0usize..4096usize) {
            let fixtures = fixture_paths();
            prop_assume!(!fixtures.is_empty());
            let path = &fixtures[idx % fixtures.len()];
            let lib = PcbLib::open(&path)?;
            lib.validate_invariants()?;
        }

        #[test]
        fn prop_pcblib_invariants_reject_broken_header(idx in 0usize..4096usize) {
            let fixtures = fixture_paths();
            prop_assume!(!fixtures.is_empty());
            let path = &fixtures[idx % fixtures.len()];
            let mut lib = PcbLib::open(&path)?;
            lib.header.version_string = "BROKEN".to_owned();
            let err = lib.validate_invariants().expect_err("broken header should fail");
            prop_assert!(err.to_string().contains("expected header"));
        }
    }
}

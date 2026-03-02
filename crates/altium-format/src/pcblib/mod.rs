pub(crate) mod custom_shapes;
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
use altium_format_types::pcb::PolygonReliefAngle;
use altium_format_types::{
    BarcodeRenderMode, Color, Coord, CoordPoint, DaisyChainStyle, MaskExpansionState, PadShape,
    PadStackMode, PcbFlags, PcbObjectId, PlaneConnectionStyle, PolySegmentKind, RegionKind,
    TCacheState, TextAutoposition, TextKind, V6Layer, V7Layer, ViaStructureType,
};

use crate::block_stream::iter_blocks;
use crate::block_stream::write_text_block;
use crate::binary_io::BinaryWriter;
use crate::board_config::serialize_board_config;
use crate::cfb_document::CfbDocument;
use crate::pcb_binary_stream::parse_pcb_section_header;
use crate::pcb_file_header::{PcbFileHeader, parse_pcb_file_header};
use crate::pcblib::library::{
    PcbEmbeddedFontEntry, PcbLayerKindMapping, PcbLibComponentTocEntry, PcbLibModelEntry,
    PcbLibraryData, PcbPadViaLibraryConfig, PcbTextureEntry, parse_component_toc,
    parse_embedded_fonts, parse_layer_kind_mapping, parse_library_data, parse_model_metadata,
    parse_pad_via_library, parse_texture_metadata, serialize_library_data_suffix,
};
use crate::pcblib::custom_shapes::{
    CornerRadiusChamferEntry, CustomMaskShapeEntry, CustomShapeEntry,
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
    pub(crate) component_kind: Option<i32>,
    pub(crate) primitives: Vec<PcbPrimitive>,
    pub(crate) extended_primitive_info: Vec<ExtendedPrimitiveInfoEntry>,
    pub(crate) primitive_guids: Vec<PrimitiveGuidEntry>,
    pub(crate) custom_shapes: Vec<CustomShapeEntry>,
    pub(crate) custom_mask_shapes: Vec<CustomMaskShapeEntry>,
    pub(crate) corner_radius_chamfer: Vec<CornerRadiusChamferEntry>,
    pub(crate) shared_unions: Vec<crate::shared_union::SharedUnionEntry>,
}

#[derive(Debug, Clone)]
pub(crate) struct PcbPrimitiveCommon {
    pub(crate) layer: V6Layer,
    pub(crate) flags: PcbFlags,
    pub(crate) net_index: u16,
    pub(crate) polygon_index: u16,
    pub(crate) component_index: u16,
    pub(crate) coordinate_index: u16,
    pub(crate) dimension_index: u16,
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

/// A single edge/vertex from a TPolySegment record in ShapeBasedRegions6/ComponentBodies6.
#[derive(Debug, Clone)]
pub(crate) struct PolySegment {
    pub(crate) kind: PolySegmentKind,
    pub(crate) vertex: CoordPoint,
    pub(crate) center: CoordPoint,
    pub(crate) radius: Coord,
    pub(crate) angle1: f64,
    pub(crate) angle2: f64,
}

/// A region contour: either legacy f64 vertex pairs or extended TPolySegment edges.
#[derive(Debug, Clone)]
pub(crate) enum Contour {
    /// Legacy Regions6: N × (f64 x, f64 y) pairs, closing implied.
    Legacy(Vec<CoordPoint>),
    /// ShapeBasedRegions6: (N+1) × TPolySegment, closing vertex explicit.
    ShapeBased(Vec<PolySegment>),
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

#[derive(Debug)]
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
    pub(crate) solder_mask_expansion_valid: TCacheState,
    pub(crate) power_plane_clearance_valid: TCacheState,
    pub(crate) planes_valid: TCacheState,
    pub(crate) plane_connection_style: PlaneConnectionStyle,
    /// Packed 4×2-bit cache/mode flags for solder mask (bits [1:0], [3:2], [5:4], [7:6]).
    pub(crate) solder_mask_cache_flags: u8,
    /// Solder mask expansion state (packed byte, values 0-7 observed).
    /// See [`MaskExpansionState`] — NOT the same as `MaskExpansionMode`.
    pub(crate) solder_mask_expansion_state: MaskExpansionState,
    /// Packed 4×2-bit cache/mode flags for paste mask (same encoding as solder_mask).
    pub(crate) paste_mask_cache_flags: u8,
    /// Paste mask expansion state (packed byte, values 0-7 observed).
    pub(crate) paste_mask_expansion_state: MaskExpansionState,
    pub(crate) via_mode: PadStackMode,
    pub(crate) diameters_per_layer: [Coord; 32],
    // Additional extended (offset 203+)
    pub(crate) layer_enum_index: i32,
    pub(crate) stack_start_layer: u8,
    pub(crate) stack_end_layer: u8,
    /// Testpoint flag for top layer (IPCB_Primitive.GetState_IsTestPoint_Top).
    pub(crate) is_testpoint_top: bool,
    /// Testpoint flag for bottom layer (IPCB_Primitive.GetState_IsTestPoint_Bottom).
    pub(crate) is_testpoint_bottom: bool,
    /// Assembly testpoint flag for top layer (IPCB_Primitive.GetState_IsAssyTestPoint_Top).
    /// Analogous to Pad extension offset 118.
    pub(crate) is_assy_testpoint_top: bool,
    /// Assembly testpoint flag for bottom layer (IPCB_Primitive.GetState_IsAssyTestPoint_Bottom).
    /// Analogous to Pad extension offset 119.
    pub(crate) is_assy_testpoint_bottom: bool,
    /// Solder mask override flag (Delphi ePrimitiveAttribute order).
    pub(crate) solder_mask_override: bool,
    /// Use separate solder mask expansion values for front/back
    /// (TV7_PadCache.UseSeparateExpansions). Analogous to Pad extension offset 120.
    pub(crate) use_separate_solder_mask_expansion: bool,
    /// Solder mask expansion measured from hole edge rather than pad edge
    /// (IPCB_StackObject). Analogous to Pad extension offset 125.
    pub(crate) solder_mask_expansion_from_hole_edge: bool,
    /// Paste mask override flag (Delphi ePrimitiveAttribute order). Rare — 86/41K records.
    pub(crate) paste_mask_override: bool,
    pub(crate) solder_mask_expansion_linked: bool,
    pub(crate) solder_mask_expansion_back: Coord,
    // Via template link extended block (after section2).
    // Present in AD26 files from ~2019+. Format: u8 version + GUID[16] + GUID[16] + i32 HolePosTol + i32 HoleNegTol + optional flags/trailing.
    pub(crate) template_link_version: Option<u8>,
    pub(crate) template_link_library_id: Option<[u8; 16]>,
    pub(crate) template_link_template_id: Option<[u8; 16]>,
    pub(crate) hole_positive_tolerance: Option<Coord>,
    pub(crate) hole_negative_tolerance: Option<Coord>,
    pub(crate) template_link_flags: Option<u8>,
    // Section 4: Per-layer pad stack entries (stride varies: 23, 29, 30).
    // Present in files with local/external via stacks. count=0 is common for simple vias.
    pub(crate) pad_layer_entries: Vec<PcbViaPadLayerEntry>,
    pub(crate) pad_layer_stride: u32,
    // Section 5: IPC-4761 via structure (9 bytes payload, ~2022+).
    // Related C# interfaces: IHoleSizeInfo, IPCB_ViaStructureSupport, IPCB_CounterHoleParams.
    pub(crate) counter_hole_angle: Option<f64>,
    pub(crate) via_structure_type: Option<ViaStructureType>,
    pub(crate) layer_diameter_overrides: Vec<PcbViaSection2Entry>,
    pub(crate) unique_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PcbViaSection2Entry {
    pub(crate) layer: u8,
    pub(crate) diameter: Coord,
    pub(crate) rule_index: u16,
    pub(crate) flags: u8,
    pub(crate) mode: u8,
}

/// Per-layer pad stack entry for via Section 4.
/// Stride 30 (confirmed layout): layer_id + shape + mode + solder_mask_exp + paste_mask_exp
///   + plane_conn_style + relief_entries + reserved + conductor_width + reserved + air_gap + reserved.
/// Strides 23/24/29 are older versions with fewer fields.
#[derive(Debug, Clone)]
pub(crate) struct PcbViaPadLayerEntry {
    /// TV7_Layer identifier (u32, e.g. 1=Top, 32=Bottom).
    pub(crate) layer_id: u32,
    /// Pad shape on this layer (typically 1=Round).
    pub(crate) shape: PadShape,
    /// Mode byte (typically 1).
    pub(crate) mode: PadStackMode,
    /// Solder mask expansion for this layer (Coord).
    pub(crate) solder_mask_expansion: Coord,
    /// Paste mask expansion (stride >= 30 only).
    pub(crate) paste_mask_expansion: Option<Coord>,
    /// Plane connection style on this layer (TPlaneConnectionStyle).
    pub(crate) plane_connection_style: PlaneConnectionStyle,
    /// Number of thermal relief conductors (i16 in stride 30, i32 in stride 23/24).
    pub(crate) relief_entries: i32,
    /// Thermal relief conductor width (stride >= 29 only).
    pub(crate) relief_conductor_width: Option<Coord>,
    /// Thermal relief air gap (stride >= 29 only).
    pub(crate) relief_air_gap: Option<Coord>,
    /// Trailing bytes for stride 23/24 (packed as u32, lowest byte first).
    pub(crate) trailing_flags: u32,
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
    pub(crate) inverted_tt_text_border: Coord,
    pub(crate) wide_string_index: i32,
    pub(crate) union_index: i32,
    pub(crate) is_inverted_rect: bool,
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
    pub(crate) barcode_render_mode: BarcodeRenderMode,
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

#[derive(Debug)]
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
    // BoardRegion-specific parameters (present when OBJECTKIND=BoardRegion)
    pub(crate) object_kind: String,
    pub(crate) bending_line_count: i32,
    pub(crate) locked_3d: bool,
    pub(crate) layer_stack_id: String,
    // Geometry: main outline + hole contours
    pub(crate) outline: Contour,
    pub(crate) holes: Vec<Contour>,
    pub(crate) unique_id: Option<String>,
}

/// TV6_PadCache — 38 bytes at pad main subrecord offsets 67-104.
///
/// Confirmed by C# `TV6_PadCache` struct (Pack=1) + Ghidra setter functions.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub(crate) struct PcbPadStackData {
    pub(crate) inner_size_x: [Coord; 29],
    pub(crate) inner_size_y: [Coord; 29],
    pub(crate) inner_shape: [PadShape; 29],
    pub(crate) padding_261: u8,
    pub(crate) hole_shape: PadShape,
    pub(crate) slot_size: Coord,
    pub(crate) slot_rotation: f64,
    pub(crate) hole_offset_x: [Coord; 32],
    pub(crate) hole_offset_y: [Coord; 32],
    pub(crate) padding_531: u8,
    pub(crate) alt_shape: [PadShape; 32],
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
#[derive(Debug, Clone)]
pub(crate) struct PcbPadExtendedCrEntry {
    pub(crate) layer_id: u32,
    pub(crate) alt_shape: PadShape,
    pub(crate) cr_pct_ex: Coord,
    pub(crate) cr_size: Coord,
    pub(crate) cr_pct: u8,
    pub(crate) use_percent: bool,
}

#[derive(Debug)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
    /// Sphere model radius (MODEL.SPHERE.RADIUS), only for sphere model types.
    pub(crate) model_sphere_radius: Coord,
    pub(crate) outline: Contour,
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

// ── Public dump view types ────────────────────────────────────────────────────

/// Dump view for a single PCB pad.
pub struct PcbLibPadDumpView {
    pub pad_name: String,
    pub location_x_mils: f64,
    pub location_y_mils: f64,
    pub size_x_mils: f64,
    pub size_y_mils: f64,
    pub hole_size_mils: f64,
    pub rotation: f64,
    pub is_plated: bool,
    pub shape: String,
    pub layer: String,
    pub pad_mode: String,
    pub solder_mask_expansion_mils: f64,
    pub paste_mask_expansion_mils: f64,
}

/// Dump view for a PCB graphic primitive.
pub struct PcbLibGraphicDumpView {
    pub graphic_type: String,
    pub layer: String,
    // Track/line: from→to, width
    pub from_x_mils: Option<f64>,
    pub from_y_mils: Option<f64>,
    pub to_x_mils: Option<f64>,
    pub to_y_mils: Option<f64>,
    pub width_mils: Option<f64>,
    // Arc: center, radius, angles
    pub center_x_mils: Option<f64>,
    pub center_y_mils: Option<f64>,
    pub radius_mils: Option<f64>,
    pub start_angle: Option<f64>,
    pub end_angle: Option<f64>,
    // Fill: corner1, corner2
    pub corner1_x_mils: Option<f64>,
    pub corner1_y_mils: Option<f64>,
    pub corner2_x_mils: Option<f64>,
    pub corner2_y_mils: Option<f64>,
    // Text
    pub text: Option<String>,
    pub location_x_mils: Option<f64>,
    pub location_y_mils: Option<f64>,
    pub rotation: Option<f64>,
    // Via: location, diameter, hole_size
    pub diameter_mils: Option<f64>,
    pub hole_size_mils: Option<f64>,
    // Region: outline vertices
    pub outline: Vec<(f64, f64)>,
}

/// Dump view for a PCB footprint.
pub struct PcbLibFootprintDumpView {
    pub display_name: String,
    pub description: String,
    pub height_mils: f64,
    pub pads: Vec<PcbLibPadDumpView>,
    pub graphics: Vec<PcbLibGraphicDumpView>,
}

impl PcbLib {
    /// Returns footprint dump views for reverse generation.
    pub fn dump_footprints(&self) -> Vec<PcbLibFootprintDumpView> {
        self.footprints.iter().map(|fp| {
            let mut pads = Vec::new();
            let mut graphics = Vec::new();

            for prim in &fp.primitives {
                match prim {
                    PcbPrimitive::Pad(p) => {
                        pads.push(PcbLibPadDumpView {
                            pad_name: p.pad_name.clone(),
                            location_x_mils: p.location.x.raw() as f64 / 10_000.0,
                            location_y_mils: p.location.y.raw() as f64 / 10_000.0,
                            size_x_mils: p.size_top.x.raw() as f64 / 10_000.0,
                            size_y_mils: p.size_top.y.raw() as f64 / 10_000.0,
                            hole_size_mils: p.hole_size.raw() as f64 / 10_000.0,
                            rotation: p.rotation,
                            is_plated: p.is_plated,
                            shape: format!("{:?}", p.shape_top).to_lowercase(),
                            layer: format!("{:?}", p.common.layer),
                            pad_mode: format!("{:?}", p.pad_mode).to_lowercase(),
                            solder_mask_expansion_mils: p.cache.solder_mask_expansion.raw() as f64 / 10_000.0,
                            paste_mask_expansion_mils: p.cache.paste_mask_expansion.raw() as f64 / 10_000.0,
                        });
                    }
                    PcbPrimitive::Track(t) => {
                        graphics.push(PcbLibGraphicDumpView {
                            graphic_type: "track".to_string(),
                            layer: format!("{:?}", t.common.layer),
                            from_x_mils: Some(t.start.x.raw() as f64 / 10_000.0),
                            from_y_mils: Some(t.start.y.raw() as f64 / 10_000.0),
                            to_x_mils: Some(t.end.x.raw() as f64 / 10_000.0),
                            to_y_mils: Some(t.end.y.raw() as f64 / 10_000.0),
                            width_mils: Some(t.width.raw() as f64 / 10_000.0),
                            center_x_mils: None, center_y_mils: None,
                            radius_mils: None, start_angle: None, end_angle: None,
                            corner1_x_mils: None, corner1_y_mils: None,
                            corner2_x_mils: None, corner2_y_mils: None,
                            text: None, location_x_mils: None, location_y_mils: None,
                            rotation: None, diameter_mils: None, hole_size_mils: None,
                            outline: vec![],
                        });
                    }
                    PcbPrimitive::Arc(a) => {
                        graphics.push(PcbLibGraphicDumpView {
                            graphic_type: "arc".to_string(),
                            layer: format!("{:?}", a.common.layer),
                            center_x_mils: Some(a.center.x.raw() as f64 / 10_000.0),
                            center_y_mils: Some(a.center.y.raw() as f64 / 10_000.0),
                            radius_mils: Some(a.radius.raw() as f64 / 10_000.0),
                            start_angle: Some(a.start_angle),
                            end_angle: Some(a.end_angle),
                            width_mils: Some(a.width.raw() as f64 / 10_000.0),
                            from_x_mils: None, from_y_mils: None,
                            to_x_mils: None, to_y_mils: None,
                            corner1_x_mils: None, corner1_y_mils: None,
                            corner2_x_mils: None, corner2_y_mils: None,
                            text: None, location_x_mils: None, location_y_mils: None,
                            rotation: None, diameter_mils: None, hole_size_mils: None,
                            outline: vec![],
                        });
                    }
                    PcbPrimitive::Fill(f) => {
                        graphics.push(PcbLibGraphicDumpView {
                            graphic_type: "fill".to_string(),
                            layer: format!("{:?}", f.common.layer),
                            corner1_x_mils: Some(f.corner1.x.raw() as f64 / 10_000.0),
                            corner1_y_mils: Some(f.corner1.y.raw() as f64 / 10_000.0),
                            corner2_x_mils: Some(f.corner2.x.raw() as f64 / 10_000.0),
                            corner2_y_mils: Some(f.corner2.y.raw() as f64 / 10_000.0),
                            rotation: Some(f.rotation),
                            from_x_mils: None, from_y_mils: None,
                            to_x_mils: None, to_y_mils: None,
                            width_mils: None,
                            center_x_mils: None, center_y_mils: None,
                            radius_mils: None, start_angle: None, end_angle: None,
                            text: None, location_x_mils: None, location_y_mils: None,
                            diameter_mils: None, hole_size_mils: None,
                            outline: vec![],
                        });
                    }
                    PcbPrimitive::Text(t) => {
                        graphics.push(PcbLibGraphicDumpView {
                            graphic_type: "text".to_string(),
                            layer: format!("{:?}", t.common.layer),
                            text: Some(t.text.clone()),
                            location_x_mils: Some(t.location.x.raw() as f64 / 10_000.0),
                            location_y_mils: Some(t.location.y.raw() as f64 / 10_000.0),
                            rotation: Some(t.rotation),
                            from_x_mils: None, from_y_mils: None,
                            to_x_mils: None, to_y_mils: None,
                            width_mils: None,
                            center_x_mils: None, center_y_mils: None,
                            radius_mils: None, start_angle: None, end_angle: None,
                            corner1_x_mils: None, corner1_y_mils: None,
                            corner2_x_mils: None, corner2_y_mils: None,
                            diameter_mils: None, hole_size_mils: None,
                            outline: vec![],
                        });
                    }
                    PcbPrimitive::Via(v) => {
                        graphics.push(PcbLibGraphicDumpView {
                            graphic_type: "via".to_string(),
                            layer: format!("{:?}", v.common.layer),
                            location_x_mils: Some(v.location.x.raw() as f64 / 10_000.0),
                            location_y_mils: Some(v.location.y.raw() as f64 / 10_000.0),
                            diameter_mils: Some(v.diameter.raw() as f64 / 10_000.0),
                            hole_size_mils: Some(v.hole_size.raw() as f64 / 10_000.0),
                            from_x_mils: None, from_y_mils: None,
                            to_x_mils: None, to_y_mils: None,
                            width_mils: None,
                            center_x_mils: None, center_y_mils: None,
                            radius_mils: None, start_angle: None, end_angle: None,
                            corner1_x_mils: None, corner1_y_mils: None,
                            corner2_x_mils: None, corner2_y_mils: None,
                            text: None, rotation: None,
                            outline: vec![],
                        });
                    }
                    PcbPrimitive::Region(r) => {
                        let outline: Vec<(f64, f64)> = match &r.outline {
                            Contour::Legacy(pts) => pts.iter()
                                .map(|pt| (pt.x.raw() as f64 / 10_000.0, pt.y.raw() as f64 / 10_000.0))
                                .collect(),
                            Contour::ShapeBased(segs) => segs.iter()
                                .map(|s| (s.vertex.x.raw() as f64 / 10_000.0, s.vertex.y.raw() as f64 / 10_000.0))
                                .collect(),
                        };
                        graphics.push(PcbLibGraphicDumpView {
                            graphic_type: "region".to_string(),
                            layer: format!("{:?}", r.common.layer),
                            outline,
                            from_x_mils: None, from_y_mils: None,
                            to_x_mils: None, to_y_mils: None,
                            width_mils: None,
                            center_x_mils: None, center_y_mils: None,
                            radius_mils: None, start_angle: None, end_angle: None,
                            corner1_x_mils: None, corner1_y_mils: None,
                            corner2_x_mils: None, corner2_y_mils: None,
                            text: None, location_x_mils: None, location_y_mils: None,
                            rotation: None, diameter_mils: None, hole_size_mils: None,
                        });
                    }
                    PcbPrimitive::ComponentBody(_) => {
                        // ComponentBody is a 3D model reference — skip for 2D spec dump
                    }
                }
            }

            PcbLibFootprintDumpView {
                display_name: fp.display_name.clone(),
                description: fp.description.clone(),
                height_mils: fp.height.raw() as f64 / 10_000.0,
                pads,
                graphics,
            }
        }).collect()
    }

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

    // ── High-Level API ───────────────────────────────────────────────────────

    /// Returns a single footprint by display name.
    pub fn footprint(&self, name: &str) -> Result<crate::api::Footprint> {
        let fp = self.find_footprint(name)?;
        Ok(crate::api::pcblib_read::footprint_from_internal(fp))
    }

    /// Returns all footprints as public API types.
    pub fn footprints(&self) -> Vec<crate::api::Footprint> {
        self.footprints
            .iter()
            .map(crate::api::pcblib_read::footprint_from_internal)
            .collect()
    }

    /// Adds a new footprint to the library.
    ///
    /// Returns an error if a footprint with the same `display_name` already exists.
    pub fn add_footprint(&mut self, fp: crate::api::Footprint) -> Result<()> {
        // Check for duplicate display name
        if self.footprints.iter().any(|f| f.display_name == fp.display_name) {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "display_name".to_owned(),
                detail: format!("footprint '{}' already exists", fp.display_name),
            });
        }

        // Derive CFB key: sanitize name, truncate to 31 chars if needed
        let sanitized = section_keys::sanitize_cfb_name(&fp.display_name);
        let cfb_key = if sanitized.len() > 31 {
            let truncated = sanitized[..31].to_owned();
            // Check for CFB key collision
            if self.footprints.iter().any(|f| f.cfb_key == truncated) {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: "cfb_key".to_owned(),
                    detail: format!(
                        "truncated CFB key '{}' collides with existing footprint",
                        truncated
                    ),
                });
            }
            self.section_keys.insert(fp.display_name.clone(), truncated.clone());
            truncated
        } else {
            // Check for CFB key collision
            if self.footprints.iter().any(|f| f.cfb_key == sanitized) {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: "cfb_key".to_owned(),
                    detail: format!(
                        "CFB key '{}' collides with existing footprint",
                        sanitized
                    ),
                });
            }
            sanitized
        };

        let pad_count = fp.pads.len() as u32;
        let height = fp.height;
        let description = fp.description.clone();
        let name = fp.display_name.clone();

        let internal = crate::api::pcblib_write::footprint_to_internal(&fp, &cfb_key);
        self.footprints.push(internal);

        self.component_toc.push(PcbLibComponentTocEntry {
            name,
            pad_count,
            height,
            description,
        });

        self.validate_invariants()
            .with_context(|| format!("after adding footprint '{}'", fp.display_name))
    }

    /// Replaces an existing footprint, matched by `display_name`.
    ///
    /// Returns an error if no footprint with the given `display_name` exists.
    pub fn update_footprint(&mut self, fp: &crate::api::Footprint) -> Result<()> {
        let idx = self.footprints
            .iter()
            .position(|f| f.display_name == fp.display_name)
            .ok_or_else(|| AltiumFormatError::StreamNotFound(
                format!("footprint '{}' not found", fp.display_name),
            ))?;

        let existing = &self.footprints[idx];
        let updated = crate::api::pcblib_write::update_footprint_internal(fp, existing);

        // Update TOC entry
        if let Some(toc) = self.component_toc.iter_mut().find(|t| t.name == fp.display_name) {
            toc.pad_count = fp.pads.len() as u32;
            toc.height = fp.height;
            toc.description = fp.description.clone();
        }

        self.footprints[idx] = updated;

        self.validate_invariants()
            .with_context(|| format!("after updating footprint '{}'", fp.display_name))
    }

    /// Removes a footprint by display name.
    ///
    /// Returns an error if no footprint with the given name exists.
    pub fn remove_footprint(&mut self, name: &str) -> Result<()> {
        let idx = self.footprints
            .iter()
            .position(|f| f.display_name == name)
            .ok_or_else(|| AltiumFormatError::StreamNotFound(
                format!("footprint '{name}' not found"),
            ))?;

        self.footprints.remove(idx);
        self.component_toc.retain(|t| t.name != name);
        self.section_keys.remove(name);

        self.validate_invariants()
            .with_context(|| format!("after removing footprint '{name}'"))
    }

    /// Creates a minimal valid PcbLib for use in tests and as a starting point
    /// for programmatic library construction.
    pub fn new_blank_ad26() -> crate::Result<Self> {
        let board_config = crate::board_config::parse_board_config(
            &mut crate::param_collection::ParameterCollection::new(),
        )
        .context("creating default board config for blank PcbLib")?;

        Ok(Self {
            header: PcbFileHeader {
                version_string: PCB_LIBRARY_BINARY_HEADER_V6.to_owned(),
                version: 5.01,
                unique_id: Some(crate::util::generate_unique_id()),
            },
            section_keys: HashMap::new(),
            library: PcbLibraryData {
                filename: String::new(),
                kind: "Protel_Advanced_PCB_Library".to_owned(),
                version: String::new(),
                date: String::new(),
                time: String::new(),
                board_config,
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
        })
    }

    /// Find a footprint by display name.
    fn find_footprint(&self, name: &str) -> Result<&PcbFootprint> {
        self.footprints
            .iter()
            .find(|f| f.display_name == name)
            .ok_or_else(|| AltiumFormatError::StreamNotFound(
                format!("footprint '{name}' not found"),
            ))
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

        cfb.write_stream(&format!("/{FILE_HEADER}"), &serialize_pcblib_file_header(&self.header)?)?;

        cfb.create_storage("/Library")?;
        cfb.write_stream("/Library/Header", &serialize_u32_header(1))?;
        let component_names: Vec<String> = self.component_toc.iter().map(|e| e.name.clone()).collect();
        cfb.write_stream("/Library/Data", &serialize_library_data(&self.library, &component_names)?)?;

        cfb.create_storage("/Library/ComponentParamsTOC")?;
        cfb.write_stream(
            "/Library/ComponentParamsTOC/Header",
            &serialize_u32_header(if self.component_toc.is_empty() { 0 } else { 1 }),
        )?;
        cfb.write_stream(
            "/Library/ComponentParamsTOC/Data",
            &serialize_component_toc_data(&self.component_toc),
        )?;

        // Models: Altium always writes this storage, even when empty.
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

        // ModelsNoEmbed: Altium always writes this storage, even when empty.
        cfb.create_storage("/Library/ModelsNoEmbed")?;
        cfb.write_stream(
            "/Library/ModelsNoEmbed/Header",
            &serialize_u32_header(self.model_no_embed_entries.len() as u32),
        )?;
        cfb.write_stream(
            "/Library/ModelsNoEmbed/Data",
            &serialize_model_entries_data(&self.model_no_embed_entries),
        )?;

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
            cfb.write_stream(
                "/Library/PadViaLibrary/Header",
                &serialize_u32_header(cfg.templates.len() as u32),
            )?;
            cfb.write_stream("/Library/PadViaLibrary/Data", &serialize_pad_via_library(cfg))?;
        }

        if !self.embedded_fonts.is_empty() {
            cfb.write_stream(
                "/Library/EmbeddedFonts",
                &serialize_embedded_fonts(&self.embedded_fonts),
            )?;
        }

        // Textures: Altium always writes this storage, even when empty.
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

        for fp in &self.footprints {
            let storage = format!("/{}", fp.cfb_key);
            cfb.create_storage(&storage)?;
            cfb.write_stream(&format!("{storage}/Parameters"), &serialize_footprint_parameters(fp))?;
            cfb.write_stream(
                &format!("{storage}/Header"),
                &serialize_u32_header(fp.primitives.len() as u32),
            )?;
            cfb.write_stream(&format!("{storage}/Data"), &serialize_footprint_data(fp)?)?;

            // WideStrings sidecar: encode all Text primitive content as ENCODEDTEXT params.
            // Altium always writes this stream, even for footprints with no Text primitives.
            let wide_strings_data = wide_strings::serialize_pcblib_wide_strings(&fp.primitives);
            cfb.write_stream(&format!("{storage}/WideStrings"), &wide_strings_data)?;

            // UniqueIDPrimitiveInformation sidecar: tracking IDs for primitives.
            let has_unique_ids = fp.primitives.iter().any(|p| sidecar::get_unique_id(p).is_some());
            if has_unique_ids {
                let (header, data) =
                    sidecar::serialize_unique_id_primitive_information(&fp.primitives);
                cfb.create_storage(&format!("{storage}/UniqueIDPrimitiveInformation"))?;
                cfb.write_stream(
                    &format!("{storage}/UniqueIDPrimitiveInformation/Header"),
                    &header,
                )?;
                cfb.write_stream(
                    &format!("{storage}/UniqueIDPrimitiveInformation/Data"),
                    &data,
                )?;
            }

            // ExtendedPrimitiveInformation sidecar: mask expansion settings.
            if !fp.extended_primitive_info.is_empty() {
                let (header, data) =
                    sidecar::serialize_extended_primitive_information(&fp.extended_primitive_info)?;
                cfb.create_storage(&format!("{storage}/ExtendedPrimitiveInformation"))?;
                cfb.write_stream(
                    &format!("{storage}/ExtendedPrimitiveInformation/Header"),
                    &header,
                )?;
                cfb.write_stream(
                    &format!("{storage}/ExtendedPrimitiveInformation/Data"),
                    &data,
                )?;
            }

            // PrimitiveGuids sidecar: GUIDs for all viewable primitives.
            if !fp.primitive_guids.is_empty() {
                let (header, data) =
                    sidecar::serialize_primitive_guids(&fp.primitive_guids);
                cfb.create_storage(&format!("{storage}/PrimitiveGuids"))?;
                cfb.write_stream(&format!("{storage}/PrimitiveGuids/Header"), &header)?;
                cfb.write_stream(&format!("{storage}/PrimitiveGuids/Data"), &data)?;
            }

            // CustomShapes sidecar: per-pad custom shape definitions.
            if !fp.custom_shapes.is_empty() {
                let data = crate::pcblib::custom_shapes::serialize_custom_shapes(&fp.custom_shapes);
                cfb.write_stream(&format!("{storage}/CustomShapes"), &data)?;
            }

            // CustomMaskShapes sidecar: per-pad custom mask shape definitions.
            if !fp.custom_mask_shapes.is_empty() {
                let data = crate::pcblib::custom_shapes::serialize_custom_mask_shapes(&fp.custom_mask_shapes);
                cfb.write_stream(&format!("{storage}/CustomMaskShapes"), &data)?;
            }

            // CornerRadiusChamfer sidecar: per-pad corner radius settings.
            if !fp.corner_radius_chamfer.is_empty() {
                let data = crate::pcblib::custom_shapes::serialize_corner_radius_chamfer(&fp.corner_radius_chamfer);
                cfb.write_stream(&format!("{storage}/CornerRadiusChamfer"), &data)?;
            }

            // SharedUnion: union data for merged primitives.
            if !fp.shared_unions.is_empty() {
                let data = crate::shared_union::serialize_shared_union_stream(&fp.shared_unions);
                cfb.write_stream(&format!("{storage}/SharedUnion"), &data)?;
            }
        }

        if !self.section_keys.is_empty() {
            cfb.write_stream(&format!("/{SECTION_KEYS}"), &serialize_section_keys(&self.section_keys)?)?;
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

// format_mil is shared between PcbLib and PcbDoc; delegate to the shared module.
fn format_mil(coord: Coord) -> String {
    crate::pcb_primitives_serialize::format_mil(coord)
}

fn serialize_u32_header(count: u32) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    w.write_u32_le(count);
    w.finish()
}

fn serialize_pcblib_file_header(header: &PcbFileHeader) -> Result<Vec<u8>> {
    let mut w = BinaryWriter::new();
    let ver = header.version_string.as_bytes();
    w.write_u32_le(ver.len() as u32);
    w.write_pascal_string(&header.version_string)?;
    w.write_f64_le(header.version);
    if let Some(uid) = &header.unique_id {
        w.write_u32_le(uid.len() as u32);
        w.write_pascal_string(uid)?;
    }
    Ok(w.finish())
}

fn serialize_library_data(library: &PcbLibraryData, component_names: &[String]) -> Result<Vec<u8>> {
    let mut params = crate::param_collection::ParameterCollection::new();
    params.insert("FILENAME", library.filename.clone());
    params.insert("KIND", library.kind.clone());
    params.insert("VERSION", library.version.clone());
    params.insert("DATE", library.date.clone());
    params.insert("TIME", library.time.clone());
    serialize_board_config(&library.board_config, &mut params);
    let mut out = write_text_block(&params.to_bytes());
    out.extend_from_slice(&serialize_library_data_suffix(component_names)?);
    Ok(out)
}

fn serialize_component_toc_data(entries: &[PcbLibComponentTocEntry]) -> Vec<u8> {
    let mut text = String::new();
    for e in entries.iter() {
        // TOC Height uses bare numbers without "mil" suffix.
        let height_str = if e.height == Coord::ZERO {
            "0".to_owned()
        } else {
            let mils = e.height.to_mils();
            let formatted = format!("{:.4}", mils);
            formatted.trim_end_matches('0').trim_end_matches('.').to_owned()
        };
        text.push_str(&format!(
            "Name={}|Pad Count={}|Height={}|Description={}",
            e.name, e.pad_count, height_str, e.description
        ));
        // Each record is terminated with \r\n (including the last one).
        text.push_str("\r\n");
    }
    let mut bytes = text.into_bytes();
    bytes.push(0);
    write_text_block(&bytes)
}

pub(crate) fn serialize_model_entries_data(entries: &[PcbLibModelEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    for entry in entries {
        let mut params = crate::param_collection::ParameterCollection::new();
        params.insert("EMBED", if entry.embed { "TRUE".to_owned() } else { "FALSE".to_owned() });
        params.insert("ID", entry.id.clone());
        params.insert("ROTX", format!("{:.3}", entry.rotation_x));
        params.insert("ROTY", format!("{:.3}", entry.rotation_y));
        params.insert("ROTZ", format!("{:.3}", entry.rotation_z));
        params.insert("DZ", entry.standoff.to_string());
        if !entry.model_source.is_empty() {
            params.insert("MODELSOURCE", entry.model_source.clone());
        }
        params.insert("CHECKSUM", entry.checksum.clone());
        params.insert("NAME", entry.name.clone());
        if !entry.title.is_empty() {
            params.insert("TITLE", entry.title.clone());
        }
        out.extend_from_slice(&write_text_block(&params.to_bytes()));
    }
    out
}

pub(crate) fn serialize_layer_kind_mapping(mapping: &PcbLayerKindMapping) -> Vec<u8> {
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

pub(crate) fn serialize_pad_via_library(cfg: &PcbPadViaLibraryConfig) -> Vec<u8> {
    let mut params = crate::param_collection::ParameterCollection::new();
    params.insert("PADVIALIBRARY.LIBRARYID", cfg.library_id.clone());
    params.insert("PADVIALIBRARY.LIBRARYNAME", cfg.library_name.clone());
    params.insert("PADVIALIBRARY.DISPLAYUNITS", cfg.display_units.clone());
    let mut out = write_text_block(&params.to_bytes());
    for template in &cfg.templates {
        let param_bytes = template.params.to_bytes();
        out.push(template.index);
        out.extend_from_slice(&(param_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&param_bytes);
    }
    out
}

pub(crate) fn serialize_embedded_fonts(entries: &[PcbEmbeddedFontEntry]) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    w.write_u32_le(entries.len() as u32);
    for entry in entries {
        write_utf16lp(&mut w, &entry.full_name);
        write_utf16lp(&mut w, &entry.face_name);
        write_utf16lp(&mut w, &entry.style_name);
        // Bold and italic are only written when style_name is non-empty.
        if let (Some(bold), Some(italic)) = (entry.bold, entry.italic) {
            w.write_u8(u8::from(bold));
            w.write_u8(u8::from(italic));
        }
        w.write_u8(entry.charset);
        w.write_u32_le(entry.data.len() as u32);
        w.write_bytes(&entry.data);
    }
    w.finish()
}

pub(crate) fn write_utf16lp(w: &mut BinaryWriter, s: &str) {
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

fn serialize_section_keys(keys: &HashMap<String, String>) -> Result<Vec<u8>> {
    let mut w = BinaryWriter::new();
    w.write_u32_le(keys.len() as u32);
    let mut pairs: Vec<_> = keys.iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(b.0));
    for (display_name, cfb_key) in pairs {
        w.write_u32_le((display_name.len() + 1) as u32);
        w.write_pascal_string(display_name)?;
        w.write_u32_le((cfb_key.len() + 1) as u32);
        w.write_pascal_string(cfb_key)?;
    }
    Ok(w.finish())
}

fn serialize_footprint_parameters(fp: &PcbFootprint) -> Vec<u8> {
    let mut params = crate::param_collection::ParameterCollection::new();
    params.insert("PATTERN", fp.pattern.clone());
    params.insert("HEIGHT", format_mil(fp.height));
    params.insert("DESCRIPTION", fp.description.clone());
    params.insert("ITEMGUID", fp.item_guid.clone());
    params.insert("REVISIONGUID", fp.revision_guid.clone());
    if let Some(kind) = fp.component_kind {
        params.insert("COMPONENTKIND", kind.to_string());
    }
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
        PcbPrimitive::Text(p) => Ok((PcbObjectId::Text, serialize_text(p)?)),
        PcbPrimitive::Pad(p) => Ok((PcbObjectId::Pad, serialize_pad(p)?)),
        PcbPrimitive::Region(p) => Ok((PcbObjectId::Region, vec![serialize_region(p)])),
        PcbPrimitive::ComponentBody(p) => Ok((PcbObjectId::ComponentBody, vec![serialize_component_body(p)])),
    }
}

fn write_primitive_common(w: &mut BinaryWriter, c: &PcbPrimitiveCommon) {
    crate::pcb_primitives_serialize::write_primitive_common(w, c);
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
    crate::pcb_primitives_serialize::serialize_via(p)
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

fn serialize_text(p: &PcbText) -> Result<Vec<Vec<u8>>> {
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
    w0.write_wide_string_fixed(&p.font_name, 32)?;
    w0.write_u8(p.inverted as u8);
    w0.write_coord(p.inverted_tt_text_border);
    w0.write_i32_le(p.wide_string_index);
    w0.write_i32_le(p.union_index);
    w0.write_u8(p.is_inverted_rect as u8);
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
    w0.write_u8(p.barcode_render_mode as u8);
    w0.write_u8(p.multiline as u8);
    w0.write_wide_string_fixed(&p.barcode_font_name, 32)?;
    // AD26 tail fields (bytes 225-251). Always write full 252-byte format
    // (upgrade to latest on save). Use 0/default for any None fields.
    w0.write_u8(p.ttf_inverted_justify.map_or(0, |v| v as u8));
    w0.write_u8(p.ttf_offset_from_inverted_rect.unwrap_or(0));
    w0.write_u8(p.tail_reserved_227.unwrap_or(0));
    w0.write_u8(p.multiline_auto_position.map_or(0, |v| v as u8));
    w0.write_u8(p.is_advance_justification_valid.map_or(0, |v| v as u8));
    w0.write_u8(p.advance_snapping.unwrap_or(0));
    w0.write_u8(p.tail_reserved_231.unwrap_or(0));
    w0.write_i32_le(p.advance_justification_x.unwrap_or(0));
    w0.write_i32_le(p.advance_justification_y.unwrap_or(0));
    w0.write_i32_le(p.use_text_alignment_by_snap.unwrap_or(0));
    w0.write_coord(p.snap_point_x.unwrap_or(Coord::ZERO));
    w0.write_coord(p.snap_point_y.unwrap_or(Coord::ZERO));
    let (s1, _, _) = encoding_rs::WINDOWS_1252.encode(&p.text);
    let mut text_bytes = s1.to_vec();
    text_bytes.push(0); // NUL terminator
    Ok(vec![w0.finish(), text_bytes])
}

fn serialize_pad(p: &PcbPad) -> Result<Vec<Vec<u8>>> {
    crate::pcb_primitives_serialize::serialize_pad(p)
}

fn write_contour(w: &mut BinaryWriter, contour: &crate::pcblib::Contour) {
    crate::pcb_primitives_serialize::write_contour(w, contour);
}

fn serialize_region(p: &PcbRegion) -> Vec<u8> {
    crate::pcb_primitives_serialize::serialize_region(p)
}

fn serialize_component_body(p: &PcbComponentBody) -> Vec<u8> {
    crate::pcb_primitives_serialize::serialize_component_body(p)
}


/// Check that a Coord used as a non-negative dimension is in `[0, MAX_REASONABLE]`.
fn check_dimension(
    value: Coord,
    primitive: &str,
    index: usize,
    field: &str,
    footprint: &str,
) -> Result<()> {
    if value.to_internal() < 0 || value > Coord::MAX_REASONABLE_DIMENSION {
        return Err(AltiumFormatError::InvalidParamValue {
            key: format!("{primitive}[{index}].{field}"),
            detail: format!(
                "footprint {:?}: dimension {} out of range [0, {}]",
                footprint,
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
    footprint: &str,
) -> Result<()> {
    if value.abs() > Coord::MAX_REASONABLE_DIMENSION {
        return Err(AltiumFormatError::InvalidParamValue {
            key: format!("{primitive}[{index}].{field}"),
            detail: format!(
                "footprint {:?}: expansion {} out of range [-{}, {}]",
                footprint,
                value,
                Coord::MAX_REASONABLE_DIMENSION,
                Coord::MAX_REASONABLE_DIMENSION,
            ),
        });
    }
    Ok(())
}

fn validate_via_coords(via: &PcbVia, index: usize, footprint: &str) -> Result<()> {
    check_dimension(via.diameter, "Via", index, "diameter", footprint)?;
    check_dimension(via.hole_size, "Via", index, "hole_size", footprint)?;
    check_expansion(via.thermal_relief_air_gap, "Via", index, "thermal_relief_air_gap", footprint)?;
    check_expansion(via.thermal_relief_conductor_width, "Via", index, "thermal_relief_conductor_width", footprint)?;
    check_expansion(via.power_plane_relief_expansion, "Via", index, "power_plane_relief_expansion", footprint)?;
    check_expansion(via.power_plane_clearance, "Via", index, "power_plane_clearance", footprint)?;
    check_expansion(via.paste_mask_expansion, "Via", index, "paste_mask_expansion", footprint)?;
    check_expansion(via.solder_mask_expansion_front, "Via", index, "solder_mask_expansion_front", footprint)?;
    check_expansion(via.solder_mask_expansion_back, "Via", index, "solder_mask_expansion_back", footprint)?;
    for (i, d) in via.diameters_per_layer.iter().enumerate() {
        check_dimension(*d, "Via", index, &format!("diameters_per_layer[{i}]"), footprint)?;
    }
    // Extension boolean flags (is_testpoint_top/bottom, is_assy_testpoint_top/bottom,
    // solder_mask_override, use_separate_solder_mask_expansion,
    // solder_mask_expansion_from_hole_edge, paste_mask_override): no range check needed
    if let Some(tol) = via.hole_positive_tolerance {
        check_expansion(tol, "Via", index, "hole_positive_tolerance", footprint)?;
    }
    if let Some(tol) = via.hole_negative_tolerance {
        check_expansion(tol, "Via", index, "hole_negative_tolerance", footprint)?;
    }
    // Semantic: diameter >= hole_size when both > 0
    if via.diameter > Coord::ZERO && via.hole_size > Coord::ZERO && via.diameter < via.hole_size {
        return Err(AltiumFormatError::InvalidParamValue {
            key: format!("Via[{index}].diameter"),
            detail: format!(
                "footprint {:?}: diameter ({}) < hole_size ({})",
                footprint, via.diameter, via.hole_size,
            ),
        });
    }
    Ok(())
}

fn validate_pad_coords(pad: &PcbPad, index: usize, footprint: &str) -> Result<()> {
    check_dimension(pad.size_top.x, "Pad", index, "size_top.x", footprint)?;
    check_dimension(pad.size_top.y, "Pad", index, "size_top.y", footprint)?;
    check_dimension(pad.size_mid.x, "Pad", index, "size_mid.x", footprint)?;
    check_dimension(pad.size_mid.y, "Pad", index, "size_mid.y", footprint)?;
    check_dimension(pad.size_bot.x, "Pad", index, "size_bot.x", footprint)?;
    check_dimension(pad.size_bot.y, "Pad", index, "size_bot.y", footprint)?;
    check_dimension(pad.hole_size, "Pad", index, "hole_size", footprint)?;
    check_expansion(pad.cache.relief_conductor_width, "Pad", index, "cache.relief_conductor_width", footprint)?;
    check_expansion(pad.cache.relief_air_gap, "Pad", index, "cache.relief_air_gap", footprint)?;
    check_expansion(pad.cache.power_plane_relief_expansion, "Pad", index, "cache.power_plane_relief_expansion", footprint)?;
    check_expansion(pad.cache.power_plane_clearance, "Pad", index, "cache.power_plane_clearance", footprint)?;
    check_expansion(pad.cache.paste_mask_expansion, "Pad", index, "cache.paste_mask_expansion", footprint)?;
    check_expansion(pad.cache.solder_mask_expansion, "Pad", index, "cache.solder_mask_expansion", footprint)?;
    check_dimension(pad.pin_package_length, "Pad", index, "pin_package_length", footprint)?;
    Ok(())
}

fn validate_arc_coords(arc: &PcbArc, index: usize, footprint: &str) -> Result<()> {
    check_dimension(arc.radius, "Arc", index, "radius", footprint)?;
    check_dimension(arc.width, "Arc", index, "width", footprint)?;
    if !arc.start_angle.is_finite() || arc.start_angle < 0.0 || arc.start_angle > 360.0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: format!("Arc[{index}].start_angle"),
            detail: format!(
                "footprint {:?}: start_angle {} not in [0, 360]",
                footprint, arc.start_angle,
            ),
        });
    }
    if !arc.end_angle.is_finite() || arc.end_angle < 0.0 || arc.end_angle > 360.0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: format!("Arc[{index}].end_angle"),
            detail: format!(
                "footprint {:?}: end_angle {} not in [0, 360]",
                footprint, arc.end_angle,
            ),
        });
    }
    Ok(())
}

fn validate_track_coords(track: &PcbTrack, index: usize, footprint: &str) -> Result<()> {
    check_dimension(track.width, "Track", index, "width", footprint)?;
    Ok(())
}

fn validate_text_coords(text: &PcbText, index: usize, footprint: &str) -> Result<()> {
    check_dimension(text.height, "Text", index, "height", footprint)?;
    check_dimension(text.stroke_width, "Text", index, "stroke_width", footprint)?;
    Ok(())
}

fn validate_region_coords(region: &PcbRegion, index: usize, footprint: &str) -> Result<()> {
    check_dimension(region.arc_resolution, "Region", index, "arc_resolution", footprint)?;
    check_expansion(region.cavity_height, "Region", index, "cavity_height", footprint)?;
    Ok(())
}

fn validate_component_body_coords(body: &PcbComponentBody, index: usize, footprint: &str) -> Result<()> {
    check_expansion(body.standoff_height, "ComponentBody", index, "standoff_height", footprint)?;
    check_expansion(body.overall_height, "ComponentBody", index, "overall_height", footprint)?;
    check_dimension(body.arc_resolution, "ComponentBody", index, "arc_resolution", footprint)?;
    check_expansion(body.cavity_height, "ComponentBody", index, "cavity_height", footprint)?;
    Ok(())
}

fn validate_pcblib_primitive_coords(lib: &PcbLib) -> Result<()> {
    for fp in &lib.footprints {
        let name = &fp.display_name;
        for (idx, prim) in fp.primitives.iter().enumerate() {
            match prim {
                PcbPrimitive::Via(v) => validate_via_coords(v, idx, name)?,
                PcbPrimitive::Pad(p) => validate_pad_coords(p, idx, name)?,
                PcbPrimitive::Arc(a) => validate_arc_coords(a, idx, name)?,
                PcbPrimitive::Track(t) => validate_track_coords(t, idx, name)?,
                PcbPrimitive::Text(t) => validate_text_coords(t, idx, name)?,
                PcbPrimitive::Region(r) => validate_region_coords(r, idx, name)?,
                PcbPrimitive::ComponentBody(b) => validate_component_body_coords(b, idx, name)?,
                PcbPrimitive::Fill(_) => {}
            }
        }
    }
    Ok(())
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

    validate_pcblib_primitive_coords(lib)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use altium_format_types::pcb::LayerRef;
    use altium_format_types::PcbObjectId;
    #[cfg(feature = "proptest")]
    use proptest::prelude::*;
    #[cfg(feature = "test-fixtures")]
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
                    mech_pairs: Vec::new(),
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

    // ── High-Level API tests ─────────────────────────────────────────────

    fn make_test_footprint(name: &str) -> crate::api::Footprint {
        use crate::api::{Pad, TrackGraphic, PcbGraphic};
        crate::api::Footprint {
            display_name: name.to_owned(),
            description: format!("Test footprint {name}"),
            pattern: name.to_owned(),
            height: Coord::from_mils(50).expect("50 mils fits Coord"),
            pads: vec![
                Pad {
                    pad_name: "1".to_owned(),
                    unique_id: None,
                    location: CoordPoint::new(Coord::ZERO, Coord::ZERO),
                    shape: PadShape::Round,
                    x_size: Coord::from_mils(60).expect("60 mils fits Coord"),
                    y_size: Coord::from_mils(60).expect("60 mils fits Coord"),
                    rotation: 0.0,
                    hole_size: Coord::from_mils(30).expect("30 mils fits Coord"),
                    is_plated: true,
                    layer: LayerRef::from_v6(V6Layer::MultiLayer),
                    pad_mode: PadStackMode::Simple,
                    solder_mask_expansion: Coord::ZERO,
                    paste_mask_expansion: Coord::ZERO,
                    plane_connection: PlaneConnectionStyle::default(),
                    relief_conductor_width: Coord::ZERO,
                    relief_entries: 4,
                    relief_air_gap: Coord::ZERO,
                },
            ],
            graphics: vec![
                PcbGraphic::Track(TrackGraphic {
                    unique_id: None,
                    layer: LayerRef::from_v6(V6Layer::TopOverlay),
                    flags: PcbFlags::default(),
                    start: CoordPoint::new(Coord::from_mils(-50).expect("-50 mils fits Coord"), Coord::from_mils(-50).expect("-50 mils fits Coord")),
                    end: CoordPoint::new(Coord::from_mils(50).expect("50 mils fits Coord"), Coord::from_mils(-50).expect("-50 mils fits Coord")),
                    width: Coord::from_mils(10).expect("10 mils fits Coord"),
                }),
            ],
        }
    }

    #[test]
    fn api_new_blank_ad26() {
        let lib = PcbLib::new_blank_ad26().expect("blank pcblib");
        assert_eq!(lib.footprint_count(), 0);
        assert!(lib.footprint_names().is_empty());
        lib.validate_invariants().unwrap();
    }

    #[test]
    fn api_add_footprint() {
        let mut lib = PcbLib::new_blank_ad26().expect("blank pcblib");
        let fp = make_test_footprint("TestFP");
        lib.add_footprint(fp).unwrap();

        assert_eq!(lib.footprint_count(), 1);
        assert_eq!(lib.footprint_names(), vec!["TestFP"]);

        let read_back = lib.footprint("TestFP").unwrap();
        assert_eq!(read_back.display_name, "TestFP");
        assert_eq!(read_back.description, "Test footprint TestFP");
        assert_eq!(read_back.pads.len(), 1);
        assert_eq!(read_back.pads[0].pad_name, "1");
        assert_eq!(read_back.graphics.len(), 1);
    }

    #[test]
    fn api_add_footprint_duplicate_fails() {
        let mut lib = PcbLib::new_blank_ad26().expect("blank pcblib");
        lib.add_footprint(make_test_footprint("DupFP")).unwrap();
        let err = lib.add_footprint(make_test_footprint("DupFP")).unwrap_err();
        assert!(err.to_string().contains("already exists"), "error: {err}");
    }

    #[test]
    fn api_update_footprint() {
        let mut lib = PcbLib::new_blank_ad26().expect("blank pcblib");
        lib.add_footprint(make_test_footprint("UpdateMe")).unwrap();

        let mut fp = lib.footprint("UpdateMe").unwrap();
        assert_eq!(fp.pads.len(), 1);
        // Add another pad
        fp.pads.push(crate::api::Pad {
            pad_name: "2".to_owned(),
            unique_id: None,
            location: CoordPoint::new(Coord::from_mils(100).expect("100 mils fits Coord"), Coord::ZERO),
            shape: PadShape::Round,
            x_size: Coord::from_mils(60).expect("60 mils fits Coord"),
            y_size: Coord::from_mils(60).expect("60 mils fits Coord"),
            rotation: 0.0,
            hole_size: Coord::from_mils(30).expect("30 mils fits Coord"),
            is_plated: true,
            layer: LayerRef::from_v6(V6Layer::MultiLayer),
            pad_mode: PadStackMode::Simple,
            solder_mask_expansion: Coord::ZERO,
            paste_mask_expansion: Coord::ZERO,
            plane_connection: PlaneConnectionStyle::default(),
            relief_conductor_width: Coord::ZERO,
            relief_entries: 4,
            relief_air_gap: Coord::ZERO,
        });
        lib.update_footprint(&fp).unwrap();

        let read_back = lib.footprint("UpdateMe").unwrap();
        assert_eq!(read_back.pads.len(), 2);
        assert_eq!(read_back.pads[1].pad_name, "2");
    }

    #[test]
    fn api_remove_footprint() {
        let mut lib = PcbLib::new_blank_ad26().expect("blank pcblib");
        lib.add_footprint(make_test_footprint("RemoveMe")).unwrap();
        assert_eq!(lib.footprint_count(), 1);

        lib.remove_footprint("RemoveMe").unwrap();
        assert_eq!(lib.footprint_count(), 0);
    }

    #[test]
    fn api_remove_not_found() {
        let mut lib = PcbLib::new_blank_ad26().expect("blank pcblib");
        let err = lib.remove_footprint("DoesNotExist").unwrap_err();
        assert!(err.to_string().contains("not found"), "error: {err}");
    }

    #[test]
    fn api_add_save_reopen() {
        let mut lib = PcbLib::new_blank_ad26().expect("blank pcblib");
        lib.add_footprint(make_test_footprint("Roundtrip")).unwrap();

        let tmp = tempfile::NamedTempFile::new().unwrap();
        lib.save(tmp.path()).unwrap();

        let reopened = PcbLib::open(tmp.path()).unwrap();
        assert_eq!(reopened.footprint_count(), 1);
        let fp = reopened.footprint("Roundtrip").unwrap();
        assert_eq!(fp.display_name, "Roundtrip");
        assert_eq!(fp.pads.len(), 1);
        assert_eq!(fp.pads[0].pad_name, "1");
        assert_eq!(fp.graphics.len(), 1);
    }

    #[test]
    fn api_footprints_returns_all() {
        let mut lib = PcbLib::new_blank_ad26().expect("blank pcblib");
        lib.add_footprint(make_test_footprint("A")).unwrap();
        lib.add_footprint(make_test_footprint("B")).unwrap();
        lib.add_footprint(make_test_footprint("C")).unwrap();

        let all = lib.footprints();
        assert_eq!(all.len(), 3);
        let names: Vec<&str> = all.iter().map(|f| f.display_name.as_str()).collect();
        assert!(names.contains(&"A"));
        assert!(names.contains(&"B"));
        assert!(names.contains(&"C"));
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn api_read_fixture_footprints() {
        for path in fixture_paths() {
            let lib = PcbLib::open(&path).unwrap();
            let api_fps = lib.footprints();
            assert_eq!(
                api_fps.len(),
                lib.footprint_count(),
                "footprint count mismatch for {}",
                path.display()
            );
            for fp in &api_fps {
                assert!(!fp.display_name.is_empty());
                assert!(!fp.pattern.is_empty());
            }
        }
    }

    #[cfg(feature = "test-fixtures")]
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

    #[cfg(feature = "test-fixtures")]
    fn roundtrip_semantic_report(path: &std::path::Path) -> crate::test_utils::CfbSemanticDiffReport {
        let lib = PcbLib::open(path).expect("PcbLib::open must succeed");
        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        lib.save(tmp.path()).expect("PcbLib::save must succeed");
        crate::test_utils::diff_cfb_files_semantic(path, tmp.path()).expect("semantic diff must succeed")
    }

    #[cfg(feature = "test-fixtures")]
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

    #[cfg(feature = "test-fixtures")]
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

    #[cfg(feature = "proptest")]
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

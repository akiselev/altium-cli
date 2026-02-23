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
use altium_format_types::{Color, Coord, CoordPoint, HoleType, PadShape, PadStackMode, PcbFlags, PlaneConnectionStyle, RegionKind, TCacheState, TextKind, V6Layer, V7Layer};

use crate::block_stream::iter_blocks;
use crate::pcb_binary_stream::parse_pcb_section_header;
use crate::pcb_file_header::{parse_pcb_file_header, PcbFileHeader};
use crate::pcblib::library::{
    PcbEmbeddedFontEntry, PcbLayerKindMapping, PcbLibComponentTocEntry, PcbLibModelEntry,
    PcbLibraryData, PcbPadViaLibraryConfig, PcbTextureEntry, parse_component_toc,
    parse_embedded_fonts, parse_layer_kind_mapping, parse_library_data, parse_model_metadata,
    parse_pad_via_library, parse_texture_metadata,
};
use crate::tracked_cfb::TrackedCfbDocument;
use crate::{AltiumFormatError, Result, ResultExt};

pub struct PcbLib {
    pub(crate) header: PcbFileHeader,
    pub(crate) section_keys: HashMap<String, String>,
    pub(crate) library: PcbLibraryData,
    pub(crate) component_toc: Vec<PcbLibComponentTocEntry>,
    pub(crate) model_entries: Vec<PcbLibModelEntry>,
    pub(crate) layer_kind_mapping: PcbLayerKindMapping,
    pub(crate) pad_via_library: Option<PcbPadViaLibraryConfig>,
    pub(crate) embedded_fonts: Vec<PcbEmbeddedFontEntry>,
    pub(crate) texture_entries: Vec<PcbTextureEntry>,
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
    pub(crate) unique_id: Option<String>,
}

pub(crate) struct PcbVia {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) location: CoordPoint,
    pub(crate) diameter: Coord,
    pub(crate) hole_size: Coord,
    pub(crate) from_layer: V6Layer,
    pub(crate) to_layer: V6Layer,
    pub(crate) unique_id: Option<String>,
}

pub(crate) struct PcbFill {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) corner1: CoordPoint,
    pub(crate) corner2: CoordPoint,
    pub(crate) rotation: f64,
    pub(crate) unique_id: Option<String>,
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
}

pub(crate) struct PcbRegion {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) kind: RegionKind,
    pub(crate) vertices: Vec<CoordPoint>,
    pub(crate) unique_id: Option<String>,
}

/// TV6_PadCache — 38 bytes at pad main subrecord offsets 67-104.
///
/// Confirmed by C# `TV6_PadCache` struct (Pack=1) + Ghidra setter functions.
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
    pub(crate) extended_stack_data: Vec<u8>,
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
    pub(crate) hole_type: HoleType,
    pub(crate) stack_mode: PadStackMode,
    // Field at offset 63 (FUN_01811110)
    pub(crate) unknown_63: i32,
    // TV6_PadCache (offsets 67-104)
    pub(crate) cache: PcbPadCache,
    // Post-cache fields (offsets 105-113)
    pub(crate) user_routed: bool,
    pub(crate) union_index: i32,
    pub(crate) unknown_110: i32,
    // Extended fields (offsets 114-171, from FUN_0187b7c0)
    pub(crate) layer_override: i32,
    pub(crate) hole_flag_1: bool,
    pub(crate) hole_flag_2: bool,
    pub(crate) stack_flag: bool,
    pub(crate) stack_conditional: i32,
    pub(crate) unknown_125: bool,
    pub(crate) swap_id_pad: [u8; 16],
    pub(crate) swap_id_part: [u8; 16],
    pub(crate) pin_package_length: Coord,
    pub(crate) hole_positive_tolerance: i32,
    pub(crate) hole_negative_tolerance: i32,
    pub(crate) unknown_170: u8,
    pub(crate) has_stack_data: bool,
    // Post-172 variable data (stack extension, read as raw bytes)
    pub(crate) post_172_data: Vec<u8>,
    // Subrecord 5: per-layer stack data (0 or 596+ bytes)
    pub(crate) stack_data: Option<PcbPadStackData>,
    // Sidecar
    pub(crate) unique_id: Option<String>,
}

pub(crate) struct PcbComponentBody {
    pub(crate) common: PcbPrimitiveCommon,
    pub(crate) v7_layer: String,
    pub(crate) name: String,
    pub(crate) kind: i32,
    pub(crate) subpoly_index: i32,
    pub(crate) union_index: i32,
    pub(crate) standoff_height: Coord,
    pub(crate) overall_height: Coord,
    pub(crate) body_projection: i32,
    pub(crate) body_color_3d: Color,
    pub(crate) body_opacity_3d: f64,
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
        let (library, suffix_names) = parse_library_data(&lib_data_raw)
            .context("parsing /Library/Data")?;

        let lib_toc_header = doc.read_stream("/Library/ComponentParamsTOC/Header")?;
        let lib_toc_data = doc.read_stream("/Library/ComponentParamsTOC/Data")?;
        let component_toc = parse_component_toc(&lib_toc_header, &lib_toc_data)
            .context("parsing /Library/ComponentParamsTOC")?;
        let _ = doc.list_entries("/Library/ComponentParamsTOC")?;

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
        let _ = doc.list_entries("/Library/Models")?;

        // Auxiliary Library sub-storages (optional)
        let layer_kind_mapping = if doc.exists("/Library/LayerKindMapping/Header") {
            let lkm_header = doc.read_stream("/Library/LayerKindMapping/Header")?;
            let lkm_data = doc.read_stream("/Library/LayerKindMapping/Data")?;
            let entries = parse_layer_kind_mapping(&lkm_header, &lkm_data)
                .context("parsing /Library/LayerKindMapping")?;
            let _ = doc.list_entries("/Library/LayerKindMapping")?;
            entries
        } else {
            PcbLayerKindMapping { version: String::new(), hash: 0, entries: Vec::new() }
        };
        let pad_via_library = if doc.exists("/Library/PadViaLibrary/Header") {
            let pvl_header = doc.read_stream("/Library/PadViaLibrary/Header")?;
            let pvl_data = doc.read_stream("/Library/PadViaLibrary/Data")?;
            let config = parse_pad_via_library(&pvl_header, &pvl_data)
                .context("parsing /Library/PadViaLibrary")?;
            let _ = doc.list_entries("/Library/PadViaLibrary")?;
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
        if doc.exists("/Library/ModelsNoEmbed/Header") {
            let mne_header = doc.read_stream("/Library/ModelsNoEmbed/Header")?;
            let mne_count = parse_pcb_section_header(&mne_header)?;
            let mne_data = doc.read_stream("/Library/ModelsNoEmbed/Data")?;
            if mne_count > 0 || !mne_data.is_empty() {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: "/Library/ModelsNoEmbed".to_owned(),
                    detail: format!(
                        "substorage has count={mne_count} and {} data bytes; parser not yet implemented",
                        mne_data.len()
                    ),
                });
            }
            let _ = doc.list_entries("/Library/ModelsNoEmbed")?;
        }
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
            let _ = doc.list_entries("/Library/Textures")?;
            tex_entries
        } else {
            Vec::new()
        };

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

        Ok(Self {
            header,
            section_keys,
            library,
            component_toc,
            model_entries,
            layer_kind_mapping,
            pad_via_library,
            embedded_fonts,
            texture_entries,
            footprints,
            file_version_info,
        })
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
            layer_kind_mapping: PcbLayerKindMapping { version: String::new(), hash: 0, entries: Vec::new() },
            pad_via_library: None,
            embedded_fonts: Vec::new(),
            texture_entries: Vec::new(),
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

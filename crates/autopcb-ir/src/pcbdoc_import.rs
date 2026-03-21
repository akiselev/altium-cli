//! PcbDoc import adapter.
//!
//! Converts a [`PcbDocBoard`] into a [`PcbDocSpec`], which can then be
//! compiled to [`PcbIr`] via [`crate::spec_compiler::spec_to_ir`].
//!
//! Coordinates are stored as [`Coord`] values; the single Coord→mm conversion
//! happens at compile time inside `spec_to_ir()`.

use altium_format::api::{BoardContour, PcbDocBoard, RuleParams};
use altium_format_spec::model::{
    BoardLayerSpec, BoardSpec, KeepoutSpec, LayerSpec, PadGeometrySpec, PcbDocClassSpec,
    PcbDocComponentSpec, PcbDocDifferentialPairSpec, PcbDocNetSpec, PcbDocPolygonSpec,
    PcbDocRuleSpec, PcbDocSpec,
};
use altium_format_types::{Coord, CoordPoint, PadShape};

use crate::compile_error::IrCompileError;
use crate::geometry::tessellate_contour_to_coords;

/// Convert a [`PcbDocBoard`] into a [`PcbDocSpec`].
///
/// The returned spec stores coordinates as [`Coord`] values; `spec_to_ir()`
/// performs the single Coord→mm conversion at compile time.
pub fn import_pcbdoc(board: &PcbDocBoard) -> Result<PcbDocSpec, IrCompileError> {
    let board_spec = import_board(board)?;
    Ok(PcbDocSpec {
        boards: vec![board_spec],
        placement: None,
        placement_rules: vec![],
        routing: None,
    })
}

// ---------------------------------------------------------------------------
// Board
// ---------------------------------------------------------------------------

fn import_board(board: &PcbDocBoard) -> Result<BoardSpec, IrCompileError> {
    let outline = import_board_outline(board);
    let keepouts = import_keepouts(board);
    let layers = import_layer_stack(board);
    let nets = import_nets(board);
    let classes = import_classes(board);
    let differential_pairs = import_differential_pairs(board);
    let components = import_components(board)?;
    let polygons = import_polygons(board);
    let rules = import_rules(board);

    Ok(BoardSpec {
        annotation: None,
        name: board.settings.document_name.clone(),
        signal_layer_count: Some(board.settings.signal_layer_count),
        snap_grid_size: Some(board.settings.snap_grid_size),
        visible_grid_size: Some(board.settings.visible_grid_size),
        display_unit: None,
        outline,
        keepouts,
        layers,
        nets,
        components,
        tracks: vec![],
        arcs: vec![],
        vias: vec![],
        pads: vec![],
        fills: vec![],
        texts: vec![],
        regions: vec![],
        component_bodies: vec![],
        dimensions: vec![],
        polygons,
        rules,
        classes,
        differential_pairs,
    })
}

// ---------------------------------------------------------------------------
// Board outline and keepouts
// ---------------------------------------------------------------------------

fn import_board_outline(board: &PcbDocBoard) -> Option<Vec<CoordPoint>> {
    board
        .settings
        .geometry
        .outline
        .as_ref()
        .map(tessellate_contour_coords)
}

fn import_keepouts(board: &PcbDocBoard) -> Vec<KeepoutSpec> {
    board
        .settings
        .geometry
        .keepouts
        .iter()
        .map(|kz| KeepoutSpec {
            vertices: tessellate_contour_coords(&kz.outline),
            restrict_copper: true,
            restrict_components: false,
            layer: Some(LayerSpec::NamedLayer(
                kz.layer.display_name().unwrap_or("Unknown").to_string(),
            )),
        })
        .collect()
}

/// Tessellate a [`BoardContour`] into a sequence of [`CoordPoint`]s.
///
/// Arc segments are sampled at ~1° intervals to preserve curved outline geometry;
/// line segments pass through directly.
fn tessellate_contour_coords(contour: &BoardContour) -> Vec<CoordPoint> {
    tessellate_contour_to_coords(contour)
}

// ---------------------------------------------------------------------------
// Layer stack
// ---------------------------------------------------------------------------

fn import_layer_stack(board: &PcbDocBoard) -> Vec<BoardLayerSpec> {
    // `layer_stack.layers` contains ONLY copper signal layers — non-copper layers
    // (mechanical, overlay, mask, etc.) are excluded during PcbDoc parsing by
    // `extract_layer_stack_v9`, which filters using `is_copper_layer_id`. Every
    // entry here is therefore correctly marked `is_copper: true`.
    let stack = &board.settings.layer_stack;
    stack
        .layers
        .iter()
        .enumerate()
        .map(|(i, sl)| BoardLayerSpec {
            name: sl.name.clone(),
            is_copper: true,
            copper_index: Some((i + 1) as u32),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Nets
// ---------------------------------------------------------------------------

fn import_nets(board: &PcbDocBoard) -> Vec<PcbDocNetSpec> {
    board
        .nets
        .iter()
        .map(|n| PcbDocNetSpec {
            annotation: None,
            name: n.name.clone(),
            color: Some(n.color),
            visible: Some(n.visible),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Classes
// ---------------------------------------------------------------------------

fn import_classes(board: &PcbDocBoard) -> Vec<PcbDocClassSpec> {
    board
        .classes
        .iter()
        .map(|c| PcbDocClassSpec {
            annotation: None,
            name: c.name.clone(),
            kind: Some(format!("{:?}", c.kind)),
            members: c.members.clone(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Differential pairs
// ---------------------------------------------------------------------------

fn import_differential_pairs(board: &PcbDocBoard) -> Vec<PcbDocDifferentialPairSpec> {
    board
        .differential_pairs
        .iter()
        .map(|dp| PcbDocDifferentialPairSpec {
            annotation: None,
            name: dp.name.clone(),
            positive_net: Some(dp.positive_net.clone()),
            negative_net: Some(dp.negative_net.clone()),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

fn import_components(board: &PcbDocBoard) -> Result<Vec<PcbDocComponentSpec>, IrCompileError> {
    board
        .components
        .iter()
        .map(|comp| {
            let pads = import_component_pads(board, &comp.designator)?;
            Ok(PcbDocComponentSpec {
                annotation: None,
                designator: comp.designator.clone(),
                pattern: Some(comp.pattern.clone()),
                comment: Some(comp.comment.clone()),
                location: Some(comp.location),
                rotation: Some(comp.rotation),
                layer: Some(LayerSpec::NamedLayer(
                    comp.layer.display_name().unwrap_or("Unknown").to_string(),
                )),
                source_library: Some(comp.source_library.clone()),
                parameters: comp
                    .parameters
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
                pads,
            })
        })
        .collect()
}

fn import_component_pads(
    board: &PcbDocBoard,
    designator: &str,
) -> Result<Vec<PadGeometrySpec>, IrCompileError> {
    board
        .pads_for_component(designator)
        .into_iter()
        .map(|pad| {
            let shape = convert_pad_shape(pad.shape)?;
            Ok(PadGeometrySpec {
                designator: pad.pad_name.clone(),
                position: pad.location,
                shape,
                size_x: pad.x_size,
                size_y: pad.y_size,
                hole_size: if pad.hole_size == Coord::ZERO {
                    None
                } else {
                    Some(pad.hole_size)
                },
                layer: LayerSpec::NamedLayer(
                    pad.layer.display_name().unwrap_or("Unknown").to_string(),
                ),
                net: pad.net.clone(),
                rotation: pad.rotation,
            })
        })
        .collect()
}

fn convert_pad_shape(
    shape: altium_format_types::pcb::PadShape,
) -> Result<PadShape, IrCompileError> {
    use altium_format_types::pcb::PadShape as Ps;
    match shape {
        Ps::NoShape => Ok(PadShape::NoShape),
        Ps::Round => Ok(PadShape::Round),
        Ps::Rectangular => Ok(PadShape::Rectangular),
        Ps::RoundRect => Ok(PadShape::RoundRect),
        Ps::RoundedRectangular => Ok(PadShape::RoundedRectangular),
        Ps::Octagonal => Ok(PadShape::Octagonal),
        Ps::Circle => Ok(PadShape::Circle),
        Ps::RotatedRect => Ok(PadShape::RotatedRect),
        Ps::Arc | Ps::Terminator | Ps::Custom => {
            Err(IrCompileError::UnsupportedPadShape(format!("{shape:?}")))
        }
        _ => Err(IrCompileError::UnsupportedPadShape(format!("{shape:?}"))),
    }
}

// ---------------------------------------------------------------------------
// Polygons
// ---------------------------------------------------------------------------

fn import_polygons(board: &PcbDocBoard) -> Vec<PcbDocPolygonSpec> {
    board
        .polygons
        .iter()
        .map(|p| PcbDocPolygonSpec {
            annotation: None,
            name: p.name.clone(),
            net: p.net.clone(),
            layer: Some(LayerSpec::NamedLayer(
                p.layer.display_name().unwrap_or("Unknown").to_string(),
            )),
            connect_style: Some(format!("{:?}", p.connect_style)),
            pour_order: Some(p.pour_order),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

fn import_rules(board: &PcbDocBoard) -> Vec<PcbDocRuleSpec> {
    board
        .rules
        .iter()
        .map(|r| {
            let properties = rule_params_to_properties(&r.params);
            PcbDocRuleSpec {
                annotation: None,
                name: r.name.clone(),
                kind: Some(format!("{:?}", r.kind)),
                enabled: Some(r.enabled),
                priority: Some(r.priority),
                properties,
                scope: Some(r.scope.clone()),
                scope2: Some(r.scope2.clone()),
            }
        })
        .collect()
}

fn rule_params_to_properties(params: &RuleParams) -> indexmap::IndexMap<String, String> {
    let mut m = indexmap::IndexMap::new();
    match params {
        RuleParams::Clearance { gap, .. } => {
            m.insert("gap".to_string(), format_mm(*gap));
        }
        RuleParams::Width { min, max, preferred } => {
            m.insert("min".to_string(), format_mm(*min));
            m.insert("max".to_string(), format_mm(*max));
            m.insert("preferred".to_string(), format_mm(*preferred));
        }
        RuleParams::ComponentClearance { gap, .. } => {
            m.insert("gap".to_string(), format_mm(*gap));
        }
        RuleParams::BoardOutlineClearance { gap } => {
            m.insert("gap".to_string(), format_mm(*gap));
        }
        RuleParams::HoleToHoleClearance { gap } => {
            m.insert("gap".to_string(), format_mm(*gap));
        }
        RuleParams::MinimumAnnularRing { min } => {
            m.insert("min".to_string(), format_mm(*min));
        }
        RuleParams::SolderMaskExpansion { expansion, .. } => {
            m.insert("expansion".to_string(), format_mm(*expansion));
        }
        RuleParams::PasteMaskExpansion { expansion, .. } => {
            m.insert("expansion".to_string(), format_mm(*expansion));
        }
        RuleParams::RoutingTopology { topology } => {
            m.insert("topology".to_string(), format!("{topology:?}"));
        }
        RuleParams::RoutingPriority { priority } => {
            m.insert("priority".to_string(), priority.to_string());
        }
        RuleParams::RoutingLayers { layer_flags } => {
            for (name, enabled) in layer_flags {
                if *enabled {
                    m.insert(name.clone(), "true".to_string());
                }
            }
        }
        RuleParams::RoutingViaStyle {
            min_width,
            max_width,
            min_hole_width,
            max_hole_width,
            ..
        } => {
            m.insert("min_width".to_string(), format_mm(*min_width));
            m.insert("max_width".to_string(), format_mm(*max_width));
            m.insert("min_hole_width".to_string(), format_mm(*min_hole_width));
            m.insert("max_hole_width".to_string(), format_mm(*max_hole_width));
        }
        RuleParams::DiffPairsRouting {
            min_gap,
            max_gap,
            max_uncoupled_length,
            ..
        } => {
            m.insert("gap".to_string(), format_mm(*min_gap));
            m.insert("max_gap".to_string(), format_mm(*max_gap));
            m.insert(
                "max_uncoupled_length".to_string(),
                format_mm(*max_uncoupled_length),
            );
        }
        RuleParams::MatchedLengths { tolerance } => {
            m.insert("tolerance".to_string(), format_mm(*tolerance));
        }
        RuleParams::MaxMinHoleSize { min, max } => {
            m.insert("min".to_string(), format_mm(*min));
            m.insert("max".to_string(), format_mm(*max));
        }
        RuleParams::Length { min, max } => {
            m.insert("min".to_string(), format_mm(*min));
            m.insert("max".to_string(), format_mm(*max));
        }
        RuleParams::DaisyChainStubLength { max_limit } => {
            m.insert("max".to_string(), format_mm(*max_limit));
        }
        RuleParams::ParallelSegment {
            gap,
            parallel_length,
            ..
        } => {
            m.insert("check_gap".to_string(), format_mm(*gap));
            m.insert("max_run".to_string(), format_mm(*parallel_length));
        }
        RuleParams::MinimumSolderMaskSliver { min_width } => {
            m.insert("min".to_string(), format_mm(*min_width));
        }
        RuleParams::SilkToSolderMaskClearance { gap } => {
            m.insert("clearance".to_string(), format_mm(*gap));
        }
        RuleParams::SilkToSilkClearance { gap } => {
            m.insert("clearance".to_string(), format_mm(*gap));
        }
        RuleParams::SmdToCorner { distance } => {
            m.insert("clearance".to_string(), format_mm(*distance));
        }
        RuleParams::MaximumViaCount { max_via_count } => {
            m.insert("max".to_string(), max_via_count.to_string());
        }
        RuleParams::AcuteAngle { minimum } => {
            m.insert("min_angle".to_string(), minimum.to_string());
        }
        RuleParams::PowerPlaneClearance { clearance } => {
            m.insert("gap".to_string(), format_mm(*clearance));
        }
        RuleParams::MaxMinHeight {
            min_height,
            max_height,
            ..
        } => {
            m.insert("min".to_string(), format_mm(*min_height));
            m.insert("max".to_string(), format_mm(*max_height));
        }
        RuleParams::NetAntennae { tolerance } => {
            m.insert("tolerance".to_string(), format_mm(*tolerance));
        }
        RuleParams::CreepageDistance { gap } => {
            m.insert("min".to_string(), format_mm(*gap));
        }
        _ => {}
    }
    m
}

/// Format a [`Coord`] as a mm string suitable for storing in rule properties.
fn format_mm(c: Coord) -> String {
    format!("{}mm", c.to_mms())
}

// ---------------------------------------------------------------------------
// Merge helpers
// ---------------------------------------------------------------------------

/// Merge spec board mutations on top of imported board.
///
/// Strategy: spec file wins on conflict. For `Option` fields, `Some(v)` from
/// the spec file overwrites the import value; `None` preserves the import value.
/// For `Vec` fields, a non-empty spec vec replaces the import vec entirely;
/// an empty spec vec leaves the import vec untouched.
///
/// Merge strategy for Vec fields: a non-empty spec Vec replaces the import Vec
/// entirely; an empty spec Vec (no entries specified in spec file) preserves the
/// import Vec unchanged.
/// LIMITATION: there is no spec syntax to express 'clear all imported items for
/// this field.' An empty spec Vec always means 'no override' — never 'override
/// with empty list.'
fn merge_board_spec(imported: BoardSpec, spec: &BoardSpec) -> BoardSpec {
    BoardSpec {
        annotation: spec.annotation.clone().or(imported.annotation),
        name: if spec.name.is_empty() {
            imported.name
        } else {
            spec.name.clone()
        },
        signal_layer_count: spec.signal_layer_count.or(imported.signal_layer_count),
        snap_grid_size: spec.snap_grid_size.or(imported.snap_grid_size),
        visible_grid_size: spec.visible_grid_size.or(imported.visible_grid_size),
        display_unit: spec.display_unit.clone().or(imported.display_unit),
        outline: spec.outline.clone().or(imported.outline),
        keepouts: if spec.keepouts.is_empty() {
            imported.keepouts
        } else {
            spec.keepouts.clone()
        },
        layers: if spec.layers.is_empty() {
            imported.layers
        } else {
            spec.layers.clone()
        },
        nets: if spec.nets.is_empty() {
            imported.nets
        } else {
            spec.nets.clone()
        },
        components: if spec.components.is_empty() {
            imported.components
        } else {
            spec.components.clone()
        },
        tracks: if spec.tracks.is_empty() {
            imported.tracks
        } else {
            spec.tracks.clone()
        },
        arcs: if spec.arcs.is_empty() {
            imported.arcs
        } else {
            spec.arcs.clone()
        },
        vias: if spec.vias.is_empty() {
            imported.vias
        } else {
            spec.vias.clone()
        },
        pads: if spec.pads.is_empty() {
            imported.pads
        } else {
            spec.pads.clone()
        },
        fills: if spec.fills.is_empty() {
            imported.fills
        } else {
            spec.fills.clone()
        },
        texts: if spec.texts.is_empty() {
            imported.texts
        } else {
            spec.texts.clone()
        },
        regions: if spec.regions.is_empty() {
            imported.regions
        } else {
            spec.regions.clone()
        },
        component_bodies: if spec.component_bodies.is_empty() {
            imported.component_bodies
        } else {
            spec.component_bodies.clone()
        },
        dimensions: if spec.dimensions.is_empty() {
            imported.dimensions
        } else {
            spec.dimensions.clone()
        },
        polygons: if spec.polygons.is_empty() {
            imported.polygons
        } else {
            spec.polygons.clone()
        },
        rules: if spec.rules.is_empty() {
            imported.rules
        } else {
            spec.rules.clone()
        },
        classes: if spec.classes.is_empty() {
            imported.classes
        } else {
            spec.classes.clone()
        },
        differential_pairs: if spec.differential_pairs.is_empty() {
            imported.differential_pairs
        } else {
            spec.differential_pairs.clone()
        },
    }
}

/// Merge a spec-file [`PcbDocSpec`] (by reference) on top of an imported [`PcbDocSpec`].
///
/// The spec file is the source of truth. Fields present in the spec file
/// overwrite the corresponding imported values. Fields absent from the spec
/// file preserve the import value.
pub fn merge_pcbdoc_spec(mut imported: PcbDocSpec, spec: &PcbDocSpec) -> PcbDocSpec {
    // Merge first board if both have one.
    if let (Some(imported_board), Some(spec_board)) =
        (imported.boards.first_mut(), spec.boards.first())
    {
        // Take the imported board out by value, merge spec on top, put result back.
        let placeholder = BoardSpec {
            annotation: None,
            name: String::new(),
            signal_layer_count: None,
            snap_grid_size: None,
            visible_grid_size: None,
            display_unit: None,
            outline: None,
            keepouts: vec![],
            layers: vec![],
            nets: vec![],
            components: vec![],
            tracks: vec![],
            arcs: vec![],
            vias: vec![],
            pads: vec![],
            fills: vec![],
            texts: vec![],
            regions: vec![],
            component_bodies: vec![],
            dimensions: vec![],
            polygons: vec![],
            rules: vec![],
            classes: vec![],
            differential_pairs: vec![],
        };
        let taken = std::mem::replace(imported_board, placeholder);
        let merged = merge_board_spec(taken, spec_board);
        *imported_board = merged;
    }

    PcbDocSpec {
        boards: imported.boards,
        placement: spec.placement.clone().or(imported.placement),
        placement_rules: if spec.placement_rules.is_empty() {
            imported.placement_rules
        } else {
            spec.placement_rules.clone()
        },
        routing: spec.routing.clone(),
    }
}

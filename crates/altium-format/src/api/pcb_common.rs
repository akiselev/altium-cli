//! Shared types and conversion functions for PCB API modules.
//!
//! These converters are used by both PcbLib and PcbDoc API modules to convert
//! between internal `PcbPad`/`Contour` types and public API types (`PadStack`,
//! `PcbContour`, `ContourSegment`).

use crate::pcblib::{Contour, PcbPad, PolySegment};
use altium_format_types::PolySegmentKind;
use altium_format_types::coord::{Coord, CoordPoint};
use altium_format_types::pcb::{HoleType, MaskExpansionMode, PadShape, TCacheState};

// ── Pad mask expansion modes ─────────────────────────────────────────────────

/// A pad's solder/paste mask expansion mode from its cache state: `Manual`
/// is a manual expansion; `Valid` (cached rule value) and `Invalid` (not yet
/// computed) both mean the expansion comes from the design rule. Pads store
/// no "no mask" state; the test corpora have no ExtendedPrimitiveInformation
/// entries for pads (PcbDoc 94 boards, PcbLib 40 libraries).
pub(crate) fn mask_mode_from_cache(state: TCacheState) -> MaskExpansionMode {
    state.mask_expansion_mode()
}

/// The cache state to write for a pad's mask expansion mode: `Manual` for a
/// manual expansion, `Invalid` (recompute from the rule) for `Rule`. A pad
/// cannot store `NoMask`, so that is an error.
pub(crate) fn mask_cache_state_for_write(
    key: &str,
    mode: MaskExpansionMode,
) -> crate::Result<TCacheState> {
    mode.pad_cache_state()
        .ok_or_else(|| crate::AltiumFormatError::InvalidParamValue {
            key: key.to_owned(),
            detail: format!("a pad cannot store mask expansion mode {mode:?}"),
        })
}

/// Merge the written (state, value) with the old record's: the API mode
/// decides Manual vs rule; a rule-driven pad keeps the old Valid/Invalid state
/// only while its value is the old cached value (a changed value is no longer
/// the rule's result, so it becomes Invalid).
pub(crate) fn merge_mask_cache_state(
    new: (TCacheState, Coord),
    old: (TCacheState, Coord),
) -> TCacheState {
    if new.0 == TCacheState::Manual {
        TCacheState::Manual
    } else if old.0 == TCacheState::Manual || new.1 != old.1 {
        TCacheState::Invalid
    } else {
        old.0
    }
}

// ── Contour types ─────────────────────────────────────────────────────────────

/// A contour (closed path) preserving arc segments.
///
/// Used by both PcbDoc board geometry and PcbLib region/component body outlines.
///
/// Segment `i` stores vertex `i` of the path (in the field named `endpoint`) and
/// the kind of the edge from vertex `i` to vertex `i + 1`. For an
/// [`ContourSegment::Arc`], that edge is the arc around `center` with `radius`,
/// running counter-clockwise from `start_angle` to `end_angle` (degrees); either
/// end of the edge can be the one at `start_angle`, so the traversal direction
/// follows from which vertex lies there. This is Altium's `TPolySegment` layout,
/// shared by ShapeBasedRegions6, Polygons6 and the Board6 outline. Contours read
/// from those records repeat the first vertex as the last one, as stored.
#[derive(Debug, Clone)]
pub struct PcbContour {
    pub segments: Vec<ContourSegment>,
}

/// A segment in a contour — either a line or an arc.
///
/// See [`PcbContour`] for what `endpoint` and the arc fields describe.
#[derive(Debug, Clone)]
pub enum ContourSegment {
    Line {
        endpoint: CoordPoint,
    },
    Arc {
        endpoint: CoordPoint,
        center: CoordPoint,
        radius: Coord,
        start_angle: f64,
        end_angle: f64,
    },
}

impl PcbContour {
    /// Flatten this contour to a list of endpoint coordinates (losing arc data).
    pub fn to_points(&self) -> Vec<CoordPoint> {
        self.segments
            .iter()
            .map(|seg| match seg {
                ContourSegment::Line { endpoint } => *endpoint,
                ContourSegment::Arc { endpoint, .. } => *endpoint,
            })
            .collect()
    }
}

// ── Pad stack types ───────────────────────────────────────────────────────────

/// Per-layer pad shape stack describing top/mid/bottom layer shapes and
/// optional inner layer overrides.
///
/// For `PadStackMode::Simple`, `top == mid == bot` and `inner_layers` is empty.
/// For `LocalStack` or `ExternalStack`, each layer may have a different shape/size.
#[derive(Debug, Clone)]
pub struct PadStack {
    pub top: PadLayerShape,
    pub mid: PadLayerShape,
    pub bot: PadLayerShape,
    /// Inner layer shape overrides (only for non-Simple modes with stack data).
    pub inner_layers: Vec<PadInnerLayerOverride>,
    /// Hole shape (Round for most pads, Square or Slot otherwise).
    pub hole_shape: HoleType,
    /// Slot width for non-round holes.
    pub slot_size: Coord,
    /// Slot rotation for non-round holes.
    pub slot_rotation: f64,
}

/// Shape and size of a pad on a single layer.
#[derive(Debug, Clone)]
pub struct PadLayerShape {
    pub shape: PadShape,
    pub x_size: Coord,
    pub y_size: Coord,
    /// Corner radius percentage (0-100) for rounded-rectangular shapes.
    pub corner_radius_pct: u8,
}

/// An inner layer override in a pad stack.
#[derive(Debug, Clone)]
pub struct PadInnerLayerOverride {
    /// 0-based inner layer index (0 = first mid layer).
    pub inner_layer_index: usize,
    pub shape: PadLayerShape,
}

impl PadStack {
    /// Create a simple pad stack where all layers have the same shape/size.
    pub fn simple(shape: PadShape, x_size: Coord, y_size: Coord) -> Self {
        let layer_shape = PadLayerShape {
            shape,
            x_size,
            y_size,
            corner_radius_pct: 0,
        };
        PadStack {
            top: layer_shape.clone(),
            mid: layer_shape.clone(),
            bot: layer_shape,
            inner_layers: Vec::new(),
            hole_shape: HoleType::Round,
            slot_size: Coord::ZERO,
            slot_rotation: 0.0,
        }
    }
}

// ── Conversion: internal → public ─────────────────────────────────────────────

/// Extract a `PadStack` from an internal `PcbPad`.
pub(crate) fn extract_pad_stack(p: &PcbPad) -> PadStack {
    let (top_cr, mid_cr, bot_cr) = if let Some(sd) = &p.stack_data {
        // corner_radius_pct[0] = top, [1] = mid, [31] = bot
        (
            sd.corner_radius_pct[0],
            sd.corner_radius_pct[1],
            sd.corner_radius_pct[31],
        )
    } else {
        (0, 0, 0)
    };

    let top = PadLayerShape {
        shape: p.shape_top,
        x_size: p.size_top.x,
        y_size: p.size_top.y,
        corner_radius_pct: top_cr,
    };
    let mid = PadLayerShape {
        shape: p.shape_mid,
        x_size: p.size_mid.x,
        y_size: p.size_mid.y,
        corner_radius_pct: mid_cr,
    };
    let bot = PadLayerShape {
        shape: p.shape_bot,
        x_size: p.size_bot.x,
        y_size: p.size_bot.y,
        corner_radius_pct: bot_cr,
    };

    let mut inner_layers = Vec::new();
    let mut hole_shape = HoleType::Round;
    let mut slot_size = Coord::ZERO;
    let mut slot_rotation = 0.0;

    if let Some(sd) = &p.stack_data {
        hole_shape = sd.hole_shape;
        slot_size = sd.slot_size;
        slot_rotation = sd.slot_rotation;

        // Extract non-trivial inner layer overrides (inner layers are indices 0..29)
        for i in 0..29 {
            let shape = sd.inner_shape[i];
            let sx = sd.inner_size_x[i];
            let sy = sd.inner_size_y[i];
            // Only include layers that have non-zero sizes (actually used)
            if sx != Coord::ZERO || sy != Coord::ZERO {
                inner_layers.push(PadInnerLayerOverride {
                    inner_layer_index: i,
                    shape: PadLayerShape {
                        shape,
                        x_size: sx,
                        y_size: sy,
                        // corner_radius_pct[2..31] maps to inner layers 0..29
                        corner_radius_pct: sd.corner_radius_pct[i + 2],
                    },
                });
            }
        }
    }

    PadStack {
        top,
        mid,
        bot,
        inner_layers,
        hole_shape,
        slot_size,
        slot_rotation,
    }
}

/// Convert an internal `Contour` to a public `PcbContour`.
pub(crate) fn contour_to_pcb_contour(contour: &Contour) -> PcbContour {
    let segments = match contour {
        Contour::Legacy(pts) => pts
            .iter()
            .map(|pt| ContourSegment::Line { endpoint: *pt })
            .collect(),
        Contour::ShapeBased(segs) => segs
            .iter()
            .map(|s| match s.kind {
                PolySegmentKind::Line => ContourSegment::Line { endpoint: s.vertex },
                PolySegmentKind::Arc => ContourSegment::Arc {
                    endpoint: s.vertex,
                    center: s.center,
                    radius: s.radius,
                    start_angle: s.angle1,
                    end_angle: s.angle2,
                },
            })
            .collect(),
    };
    PcbContour { segments }
}

// ── Conversion: public → internal ─────────────────────────────────────────────

/// Convert a public `PcbContour` back to an internal `Contour`.
///
/// Produces `ShapeBased` if any arc segments are present, `Legacy` if all lines.
pub(crate) fn pcb_contour_to_internal(contour: &PcbContour) -> Contour {
    let has_arcs = contour
        .segments
        .iter()
        .any(|s| matches!(s, ContourSegment::Arc { .. }));

    if has_arcs {
        let segs = contour
            .segments
            .iter()
            .map(|s| match s {
                ContourSegment::Line { endpoint } => PolySegment {
                    kind: PolySegmentKind::Line,
                    vertex: *endpoint,
                    center: CoordPoint::new(Coord::ZERO, Coord::ZERO),
                    radius: Coord::ZERO,
                    angle1: 0.0,
                    angle2: 0.0,
                },
                ContourSegment::Arc {
                    endpoint,
                    center,
                    radius,
                    start_angle,
                    end_angle,
                } => PolySegment {
                    kind: PolySegmentKind::Arc,
                    vertex: *endpoint,
                    center: *center,
                    radius: *radius,
                    angle1: *start_angle,
                    angle2: *end_angle,
                },
            })
            .collect();
        Contour::ShapeBased(segs)
    } else {
        let pts = contour
            .segments
            .iter()
            .map(|s| match s {
                ContourSegment::Line { endpoint } => *endpoint,
                ContourSegment::Arc { endpoint, .. } => *endpoint,
            })
            .collect();
        Contour::Legacy(pts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_mask_modes_and_cache_states() {
        assert_eq!(
            mask_mode_from_cache(TCacheState::Manual),
            MaskExpansionMode::Manual
        );
        assert_eq!(
            mask_mode_from_cache(TCacheState::Valid),
            MaskExpansionMode::Rule
        );
        assert_eq!(
            mask_mode_from_cache(TCacheState::Invalid),
            MaskExpansionMode::Rule
        );
        assert_eq!(
            mask_cache_state_for_write("K", MaskExpansionMode::Manual).unwrap(),
            TCacheState::Manual
        );
        assert_eq!(
            mask_cache_state_for_write("K", MaskExpansionMode::Rule).unwrap(),
            TCacheState::Invalid
        );
        assert!(mask_cache_state_for_write("K", MaskExpansionMode::NoMask).is_err());
        // The API mode decides Manual vs rule; a rule pad keeps Valid/Invalid.
        let (a, b) = (Coord::from_internal(40_000), Coord::from_internal(60_000));
        for (new, old, merged) in [
            (
                (TCacheState::Manual, a),
                (TCacheState::Valid, a),
                TCacheState::Manual,
            ),
            (
                (TCacheState::Invalid, a),
                (TCacheState::Manual, a),
                TCacheState::Invalid,
            ),
            (
                (TCacheState::Invalid, a),
                (TCacheState::Valid, a),
                TCacheState::Valid,
            ),
            (
                (TCacheState::Invalid, b),
                (TCacheState::Valid, a),
                TCacheState::Invalid,
            ),
            (
                (TCacheState::Invalid, a),
                (TCacheState::Invalid, a),
                TCacheState::Invalid,
            ),
        ] {
            assert_eq!(
                merge_mask_cache_state(new, old),
                merged,
                "{new:?} over {old:?}"
            );
        }
    }
}

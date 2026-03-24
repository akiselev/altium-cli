//! Bridge between the spec pipeline and the IR.
//!
//! Provides [`load_ir_from_spec`] which compiles a [`PcbDocSpec`] directly to
//! a [`PcbIr`] and merges routing solutions.
//!
//! The spec is the sole source of truth — no PcbDoc files are opened here.

use std::path::Path;

use altium_format_spec::PcbDocSpec;
use autopcb_routes::RouteSolution;

use crate::component::IrComponent;
use crate::copper::{IrTrack, IrVia};
use crate::ir::PcbIr;
use crate::handles::{LayerId, NetId};
use crate::spec_compiler::spec_to_ir;
use crate::types::{BoundingBoxMm, PointMm};
use crate::IrError;

/// Load a [`PcbIr`] from a compiled [`PcbDocSpec`].
///
/// `footprint_libs` maps import aliases to compiled [`PcbLibSpec`]s for
/// resolving `footprint: $fp.Name` references on components.
///
/// Pipeline:
/// 1. Compile spec to IR via `spec_to_ir()`
/// 2. Apply placement `at:` overrides
/// 3. Load and merge routing solution if available
pub fn load_ir_from_spec(
    spec: &PcbDocSpec,
    footprint_libs: &std::collections::HashMap<String, altium_format_spec::model::PcbLibSpec>,
    spec_dir: &Path,
) -> crate::Result<PcbIr> {
    let mut ir = spec_to_ir(spec, footprint_libs).map_err(|e| {
        IrError::ExtractionError(format!("spec compilation failed: {e}"))
    })?;

    apply_placement_overrides(spec, &mut ir);
    merge_routes_if_available(&mut ir, spec, spec_dir);

    Ok(ir)
}

/// Apply `placement { places { ... } }` position overrides to components in the IR.
///
/// The `at:` field in a placement place spec provides position overrides that
/// are used by the autoplacer. The executor (`apply_spec_pcbdoc`) does NOT
/// handle these — they are separate from `board.components[].location`.
fn apply_placement_overrides(spec: &PcbDocSpec, ir: &mut PcbIr) {
    let placement = match &spec.placement {
        Some(p) => p,
        None => return,
    };

    for place in &placement.places {
        let at = match &place.at {
            Some(a) => a,
            None => continue,
        };
        let x_mm = at.x.to_mms();
        let y_mm = at.y.to_mms();

        for designator in &place.designators {
            for (_id, comp) in ir.components.iter_mut() {
                if comp.designator == *designator {
                    let rotation = place.rotation.unwrap_or(comp.rotation);
                    apply_component_pose(comp, x_mm, y_mm, rotation);
                }
            }
        }
    }
}

/// Reposition a component and recompute its pad world positions and bounds.
///
/// This is a general-purpose utility used by the bridge for placement overrides
/// and by the viewer for playback snapshot application.
pub fn apply_component_pose(
    comp: &mut IrComponent,
    x_mm: f64,
    y_mm: f64,
    rotation_deg: f64,
) {
    let rotation_delta = rotation_deg - comp.rotation;
    comp.position = PointMm::new(x_mm, y_mm);
    comp.rotation = rotation_deg;

    let theta = rotation_deg.to_radians();
    let (sin_t, cos_t) = theta.sin_cos();
    for pad in &mut comp.pads {
        let lx = pad.local_position.x;
        let ly = pad.local_position.y;
        pad.world_position = PointMm::new(
            x_mm + lx * cos_t - ly * sin_t,
            y_mm + lx * sin_t + ly * cos_t,
        );
        pad.shape.rotation = (pad.shape.rotation + rotation_delta).rem_euclid(360.0);
    }

    let lb = comp.local_bounds;
    let corners = [
        PointMm::new(lb.min.x, lb.min.y),
        PointMm::new(lb.min.x, lb.max.y),
        PointMm::new(lb.max.x, lb.min.y),
        PointMm::new(lb.max.x, lb.max.y),
    ];
    let mut world_pts = Vec::with_capacity(4);
    for c in corners {
        world_pts.push(PointMm::new(
            x_mm + c.x * cos_t - c.y * sin_t,
            y_mm + c.x * sin_t + c.y * cos_t,
        ));
    }
    if let Some(bb) = BoundingBoxMm::from_points(&world_pts) {
        comp.world_bounds = bb;
    }
}

/// Load a .routes file and merge its segments/vias into `PcbIr.free_copper`.
///
/// Returns `true` if a solution was loaded and merged, `false` if no routes
/// file was configured or the file could not be read.
fn merge_routes_if_available(ir: &mut PcbIr, spec: &PcbDocSpec, spec_dir: &Path) -> bool {
    let routes_path = spec
        .routing
        .as_ref()
        .and_then(|r| r.solution.as_ref())
        .map(|s| spec_dir.join(s));

    let routes_path = match routes_path {
        Some(p) if p.exists() => p,
        _ => return false,
    };

    let solution = match autopcb_routes::load_binary(&routes_path)
        .or_else(|_| autopcb_routes::load_json(&routes_path))
    {
        Ok(s) => s,
        Err(_) => return false,
    };

    merge_solution_into_ir(ir, &solution);
    true
}

/// Convert a [`RouteSolution`] into `IrTrack` and `IrVia` entries appended to
/// `ir.free_copper`. Segments and vias that reference a layer index outside the
/// IR layer stack are silently skipped (the router may have been run against a
/// different board revision).
fn merge_solution_into_ir(ir: &mut PcbIr, solution: &RouteSolution) {
    for routed_net in solution.nets.values() {
        let ir_net_id = NetId::from(routed_net.net_id.raw());
        let net = if ir.nets.get(ir_net_id).is_some() {
            Some(ir_net_id)
        } else {
            None
        };

        for seg in &routed_net.segments {
            let layer_idx = seg.layer.raw() as u32;
            let ir_layer = LayerId::from(layer_idx);
            let layer_name = ir
                .layer_stack
                .copper_layers
                .iter()
                .find(|l| l.id == ir_layer)
                .map(|l| l.name.clone())
                .unwrap_or_else(|| format!("Layer{}", layer_idx));
            ir.free_copper.tracks.push(IrTrack {
                start: PointMm::new(seg.start.x, seg.start.y),
                end: PointMm::new(seg.end.x, seg.end.y),
                width_mm: seg.width_mm,
                layer_name,
                layer: ir_layer,
                net,
                locked: false,
                pre_routed: true,
            });
        }

        for via in &routed_net.vias {
            let from_idx = via.from_layer.raw() as u32;
            let to_idx = via.to_layer.raw() as u32;
            ir.free_copper.vias.push(IrVia {
                position: PointMm::new(via.position.x, via.position.y),
                diameter_mm: via.drill_mm + 2.0 * via.annular_ring_mm,
                hole_size_mm: via.drill_mm,
                net,
                from_layer: LayerId::from(from_idx),
                to_layer: LayerId::from(to_idx),
                locked: false,
                pre_routed: true,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::{IrComponentPad, PadShapeInfo, PadShapeKind};
    use crate::handles::{ComponentId, PadId};
    use crate::types::BoardSide;

    #[test]
    fn apply_component_pose_updates_rotation_and_pad_geometry() {
        let mut comp = IrComponent {
            id: ComponentId::from(0),
            designator: "U1".into(),
            pattern: "TEST".into(),
            value: "".into(),
            position: PointMm::new(0.0, 0.0),
            rotation: 0.0,
            side: BoardSide::Top,
            local_bounds: BoundingBoxMm::new(PointMm::new(-2.0, -1.0), PointMm::new(2.0, 1.0)),
            world_bounds: BoundingBoxMm::new(PointMm::new(-2.0, -1.0), PointMm::new(2.0, 1.0)),
            pads: vec![IrComponentPad {
                id: PadId::from(0),
                name: "1".into(),
                local_position: PointMm::new(1.0, 0.0),
                world_position: PointMm::new(1.0, 0.0),
                net: None,
                shape: PadShapeInfo {
                    kind: PadShapeKind::Rectangular,
                    size_x: 1.0,
                    size_y: 2.0,
                    rotation: 0.0,
                },
                is_through_hole: false,
                hole_size_mm: 0.0,
                swap_id_pin: None,
                swap_id_part: None,
                layer_set: Vec::new(),
            }],
        };

        apply_component_pose(&mut comp, 10.0, 20.0, 90.0);

        assert_eq!(comp.position, PointMm::new(10.0, 20.0));
        assert_eq!(comp.rotation, 90.0);
        assert!((comp.pads[0].world_position.x - 10.0).abs() < 1e-6);
        assert!((comp.pads[0].world_position.y - 21.0).abs() < 1e-6);
        assert_eq!(comp.pads[0].shape.rotation, 90.0);
        assert!((comp.world_bounds.width() - 2.0).abs() < 1e-6);
        assert!((comp.world_bounds.height() - 4.0).abs() < 1e-6);
    }
}

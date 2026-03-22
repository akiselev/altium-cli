//! Shared geometry helpers used by both `extract.rs` and `spec_compiler.rs`.

use altium_format::api::{BoardContour, ContourSegment};
use altium_format_types::{Coord, CoordPoint};

use crate::component::{IrComponent, IrComponentPad};
use crate::handles::{ComponentId, IdMap};
use crate::types::{BoundingBoxMm, PointMm};

/// Tessellate a [`BoardContour`] into a sequence of [`CoordPoint`]s.
///
/// Line segments pass through directly. Arc segments are sampled at ~1°
/// intervals so that curved board outlines and keepout zones are faithfully
/// represented as polygon vertices.
pub(crate) fn tessellate_contour_to_coords(contour: &BoardContour) -> Vec<CoordPoint> {
    let mut points = Vec::new();
    for seg in &contour.segments {
        match seg {
            ContourSegment::Line { endpoint } => {
                points.push(*endpoint);
            }
            ContourSegment::Arc {
                endpoint,
                center,
                radius,
                start_angle,
                end_angle,
            } => {
                let cx = center.x.to_mms();
                let cy = center.y.to_mms();
                let r = radius.to_mms();

                let mut sweep = end_angle - start_angle;
                if sweep <= 0.0 {
                    sweep += 360.0;
                }
                let steps = (sweep.abs() as usize).max(1);
                let step_deg = sweep / steps as f64;

                for i in 1..=steps {
                    let angle_deg = start_angle + step_deg * i as f64;
                    let angle_rad = angle_deg.to_radians();
                    points.push(CoordPoint {
                        x: Coord::from_mms(cx + r * angle_rad.cos()),
                        y: Coord::from_mms(cy + r * angle_rad.sin()),
                    });
                }
                // Ensure we land exactly on the endpoint.
                if let Some(last) = points.last_mut() {
                    *last = *endpoint;
                }
            }
        }
    }
    points
}

/// Convert a world position to component-local coordinates.
pub(crate) fn world_to_local(world: PointMm, comp_pos: PointMm, rotation_deg: f64) -> PointMm {
    let dx = world.x - comp_pos.x;
    let dy = world.y - comp_pos.y;
    if rotation_deg.abs() < 1e-6 {
        return PointMm::new(dx, dy);
    }
    let angle = -rotation_deg.to_radians();
    PointMm::new(
        dx * angle.cos() - dy * angle.sin(),
        dx * angle.sin() + dy * angle.cos(),
    )
}

/// Convert a component-local position to world coordinates.
///
/// Inverse of [`world_to_local`]: rotates by `+rotation_deg` then translates by
/// `comp_pos`.
pub(crate) fn local_to_world(local: PointMm, comp_pos: PointMm, rotation_deg: f64) -> PointMm {
    if rotation_deg.abs() < 1e-6 {
        return PointMm::new(local.x + comp_pos.x, local.y + comp_pos.y);
    }
    let angle = rotation_deg.to_radians();
    PointMm::new(
        local.x * angle.cos() - local.y * angle.sin() + comp_pos.x,
        local.x * angle.sin() + local.y * angle.cos() + comp_pos.y,
    )
}

/// Compute world and local bounding boxes for all components from their pad extents.
pub(crate) fn compute_component_bounds(components: &mut IdMap<ComponentId, IrComponent>) {
    for (_id, comp) in components.iter_mut() {
        if comp.pads.is_empty() {
            comp.world_bounds =
                BoundingBoxMm::new(comp.position, comp.position).expand(0.5);
            comp.local_bounds = BoundingBoxMm::new(
                PointMm::new(0.0, 0.0),
                PointMm::new(0.0, 0.0),
            )
            .expand(0.5);
            continue;
        }

        let world_points: Vec<PointMm> = pad_corner_points(&comp.pads, |p| p.world_position);
        comp.world_bounds = BoundingBoxMm::from_points(&world_points)
            .unwrap_or_else(|| BoundingBoxMm::new(comp.position, comp.position));

        let local_points: Vec<PointMm> = pad_corner_points(&comp.pads, |p| p.local_position);
        comp.local_bounds = BoundingBoxMm::from_points(&local_points).unwrap_or_else(|| {
            BoundingBoxMm::new(PointMm::new(0.0, 0.0), PointMm::new(0.0, 0.0))
        });
    }
}

fn pad_corner_points(pads: &[IrComponentPad], center: impl Fn(&IrComponentPad) -> PointMm) -> Vec<PointMm> {
    pads.iter()
        .flat_map(|p| {
            let hx = p.shape.size_x / 2.0;
            let hy = p.shape.size_y / 2.0;
            let c = center(p);
            [
                PointMm::new(c.x - hx, c.y - hy),
                PointMm::new(c.x + hx, c.y + hy),
            ]
        })
        .collect()
}

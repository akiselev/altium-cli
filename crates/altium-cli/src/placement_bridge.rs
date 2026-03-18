//! Bridge: translates a [`PlacementSpec`] + [`PcbIr`] into solver [`UserConstraint`]s.
//!
//! This module lives in `altium-cli` because it depends on both `altium-format-spec`
//! (for the spec model types) and `autopcb-placement` (for the constraint types).
//! Neither crate depends on the other, so the bridge must sit at a layer that imports both.

use std::collections::HashSet;

use altium_format_spec::{PlacementConstraintSpec, PlacementSpec, UnplacedStrategy};
use autopcb_ir::PcbIr;
use autopcb_placement::{Direction, PlacementEdge, RectRegion, UserConstraint, named_region_from_board};

/// Translate a [`PlacementSpec`] into solver constraints and an autoplace designator list.
///
/// Returns `(constraints, autoplace_designators)` where `autoplace_designators` lists the
/// component designators that the solver should treat as free variables.
///
/// # Constraint mapping
///
/// - Component with `at:` and `autoplace: false` → `UserConstraint::FixedPosition`
/// - Component with `autoplace: true`:
///   - If has `edge:` → `UserConstraint::EdgePlacement`
///   - If has `near:` + `max_distance:` → `UserConstraint::Near`
///   - If has `region_rect:` → `UserConstraint::RegionContainment`
///   - If has `region_name:` (named preset) → `UserConstraint::RegionContainment`
/// - `spec.constraints` directional entries → `UserConstraint::Directional`
///
/// # Unplaced strategy
///
/// Components present in `ir` but not mentioned in `spec.places` are handled
/// according to `spec.unplaced`:
/// - `Autoplace` (default): added to the autoplace set
/// - `Ignore`: added as `FixedPosition` at their current IR position
/// - `Error`: returns an error listing the unmentioned designators
///
/// # Designator validation
///
/// If a designator in `spec.places` is NOT found in `ir.components`:
/// - If `unplaced: error`, returns an error listing all unknown designators
/// - Otherwise, emits a warning to stderr and skips (no constraint added)
pub fn placement_spec_to_constraints(
    spec: &PlacementSpec,
    ir: &PcbIr,
) -> anyhow::Result<(Vec<UserConstraint>, Vec<String>)> {
    let mut constraints: Vec<UserConstraint> = Vec::new();
    let mut autoplace_designators: Vec<String> = Vec::new();

    // Build lookup: designator → IR position (mm)
    let ir_designators: HashSet<String> = ir
        .components
        .iter()
        .map(|(_, c)| c.designator.clone())
        .collect();

    // Track which designators are mentioned in the spec.
    let mut spec_designators: HashSet<String> = HashSet::new();

    // Collect unknown designators for error reporting.
    let mut unknown_in_ir: Vec<String> = Vec::new();

    for place in &spec.places {
        for designator in &place.designators {
            spec_designators.insert(designator.clone());

            if !ir_designators.contains(designator) {
                unknown_in_ir.push(designator.clone());
                // Will handle below after collecting all unknowns.
                continue;
            }

            if place.autoplace {
                // Component is a solver variable.
                autoplace_designators.push(designator.clone());

                if let Some(edge_str) = &place.edge {
                    let edge = parse_edge(edge_str).ok_or_else(|| {
                        anyhow::anyhow!(
                            "invalid edge value '{}' for designator {}",
                            edge_str,
                            designator
                        )
                    })?;
                    constraints.push(UserConstraint::EdgePlacement {
                        designator: designator.clone(),
                        edge,
                        inset_mm: place.inset.map(|v| v.to_mms()).unwrap_or(0.0),
                    });
                }

                if let (Some(near), Some(max_dist)) = (&place.near, place.max_distance) {
                    constraints.push(UserConstraint::Near {
                        a: designator.clone(),
                        b: near.clone(),
                        max_distance_mm: max_dist.to_mms(),
                    });
                }

                if let Some(region_name) = &place.region_name {
                    if let Some(rr) = named_region_from_board(ir, region_name) {
                        constraints.push(UserConstraint::RegionContainment {
                            designator: designator.clone(),
                            region: rr,
                        });
                    }
                }

                if let Some((from, to)) = place.region_rect {
                    constraints.push(UserConstraint::RegionContainment {
                        designator: designator.clone(),
                        region: RectRegion {
                            min_x: from.x.to_mms(),
                            min_y: from.y.to_mms(),
                            max_x: to.x.to_mms(),
                            max_y: to.y.to_mms(),
                        },
                    });
                }
            } else {
                // Locked component: has at: without autoplace, or explicit fixed: true.
                if let Some(at) = place.at {
                    constraints.push(UserConstraint::FixedPosition {
                        designator: designator.clone(),
                        x_mm: at.x.to_mms(),
                        y_mm: at.y.to_mms(),
                    });
                }
            }
        }
    }

    // Handle unknown designators (in spec but not in IR).
    if !unknown_in_ir.is_empty() {
        if spec.unplaced == UnplacedStrategy::Error {
            anyhow::bail!(
                "designator(s) in spec not found in PcbIr: {}",
                unknown_in_ir.join(", ")
            );
        } else {
            for d in &unknown_in_ir {
                eprintln!(
                    "warning: designator '{}' in spec not found in PcbIr — skipping",
                    d
                );
            }
        }
    }

    // Translate directional constraints from spec.constraints.
    for c in &spec.constraints {
        match c {
            PlacementConstraintSpec::LeftOf { a, b, gap } => {
                constraints.push(UserConstraint::Directional {
                    a: a.clone(),
                    b: b.clone(),
                    direction: Direction::LeftOf,
                    gap_mm: gap.map(|v| v.to_mms()).unwrap_or(0.0),
                });
            }
            PlacementConstraintSpec::RightOf { a, b, gap } => {
                constraints.push(UserConstraint::Directional {
                    a: a.clone(),
                    b: b.clone(),
                    direction: Direction::RightOf,
                    gap_mm: gap.map(|v| v.to_mms()).unwrap_or(0.0),
                });
            }
            PlacementConstraintSpec::Above { a, b, gap } => {
                constraints.push(UserConstraint::Directional {
                    a: a.clone(),
                    b: b.clone(),
                    direction: Direction::Above,
                    gap_mm: gap.map(|v| v.to_mms()).unwrap_or(0.0),
                });
            }
            PlacementConstraintSpec::Below { a, b, gap } => {
                constraints.push(UserConstraint::Directional {
                    a: a.clone(),
                    b: b.clone(),
                    direction: Direction::Below,
                    gap_mm: gap.map(|v| v.to_mms()).unwrap_or(0.0),
                });
            }
        }
    }

    // Handle components in PcbIr not mentioned in spec.places (unplaced strategy).
    let unmentioned: Vec<_> = ir
        .components
        .iter()
        .filter(|(_, c)| !spec_designators.contains(&c.designator))
        .map(|(_, c)| c)
        .collect();

    if !unmentioned.is_empty() {
        match spec.unplaced {
            UnplacedStrategy::Error => {
                let names: Vec<&str> =
                    unmentioned.iter().map(|c| c.designator.as_str()).collect();
                anyhow::bail!(
                    "PcbIr component(s) not mentioned in spec (unplaced: error): {}",
                    names.join(", ")
                );
            }
            UnplacedStrategy::Ignore => {
                for comp in &unmentioned {
                    constraints.push(UserConstraint::FixedPosition {
                        designator: comp.designator.clone(),
                        x_mm: comp.position.x,
                        y_mm: comp.position.y,
                    });
                }
            }
            UnplacedStrategy::Autoplace => {
                for comp in &unmentioned {
                    autoplace_designators.push(comp.designator.clone());
                }
            }
        }
    }

    Ok((constraints, autoplace_designators))
}

fn parse_edge(s: &str) -> Option<PlacementEdge> {
    match s {
        "top" => Some(PlacementEdge::Top),
        "bottom" => Some(PlacementEdge::Bottom),
        "left" => Some(PlacementEdge::Left),
        "right" => Some(PlacementEdge::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altium_format_spec::{
        PlacementClearanceSpec, PlacementOptimizeSpec, PlacementPlaceSpec,
    };
    use altium_format_types::coord::{Coord, CoordPoint};
    use autopcb_ir::{
        BoardSide, BoundingBoxMm, ComponentId, FreeCopperGeometry, IrBoardGeometry, IrComponent,
        IrLayerStack, IdMap, PointMm,
    };

    fn make_coord(mils: f64) -> Coord {
        Coord::from_mils_f64(mils)
    }

    fn make_point(x_mils: f64, y_mils: f64) -> CoordPoint {
        CoordPoint { x: make_coord(x_mils), y: make_coord(y_mils) }
    }

    fn minimal_ir(designators: &[(&str, f64, f64)]) -> PcbIr {
        let mut components: IdMap<ComponentId, IrComponent> = IdMap::new();
        for (i, &(d, x, y)) in designators.iter().enumerate() {
            let zero_bb =
                BoundingBoxMm::new(PointMm::new(0.0, 0.0), PointMm::new(1.0, 1.0));
            let id = ComponentId::from(i as u32);
            components.push(IrComponent {
                id,
                designator: d.to_string(),
                pattern: String::new(),
                value: String::new(),
                position: PointMm::new(x, y),
                rotation: 0.0,
                side: BoardSide::Top,
                local_bounds: zero_bb,
                world_bounds: zero_bb,
                pads: vec![],
            });
        }

        let board_bounds =
            BoundingBoxMm::new(PointMm::new(0.0, 0.0), PointMm::new(100.0, 100.0));
        PcbIr {
            board: IrBoardGeometry {
                outline: vec![
                    PointMm::new(0.0, 0.0),
                    PointMm::new(100.0, 0.0),
                    PointMm::new(100.0, 100.0),
                    PointMm::new(0.0, 100.0),
                ],
                cutouts: vec![],
                bounds: board_bounds,
                keepouts: vec![],
            },
            layer_stack: IrLayerStack { copper_layers: vec![], copper_layer_count: 0 },
            components,
            nets: IdMap::new(),
            rules: IdMap::new(),
            free_copper: FreeCopperGeometry::default(),
            polygons: IdMap::new(),
        }
    }

    fn empty_spec(
        places: Vec<PlacementPlaceSpec>,
        unplaced: UnplacedStrategy,
    ) -> PlacementSpec {
        PlacementSpec {
            target: None,
            places,
            constraints: vec![],
            optimize: PlacementOptimizeSpec { ratsnest: true, ratsnest_weight: 0.01 },
            clearance: PlacementClearanceSpec { all: None, edge: None },
            autoplace_config: None,
            unplaced,
            allow_pin_swap: false,
            allow_part_swap: false,
            allow_gate_swap: false,
            groups: vec![],
        }
    }

    fn locked_place(designator: &str, x_mils: f64, y_mils: f64) -> PlacementPlaceSpec {
        PlacementPlaceSpec {
            designators: vec![designator.to_string()],
            region_name: None,
            region_rect: None,
            edge: None,
            inset: None,
            near: None,
            max_distance: None,
            rotation_options: vec![],
            fixed: true,
            at: Some(make_point(x_mils, y_mils)),
            side: None,
            autoplace: false,
            no_pin_swap: vec![],
            no_part_swap: false,
        }
    }

    fn autoplace_place(designator: &str) -> PlacementPlaceSpec {
        PlacementPlaceSpec {
            designators: vec![designator.to_string()],
            region_name: None,
            region_rect: None,
            edge: None,
            inset: None,
            near: None,
            max_distance: None,
            rotation_options: vec![],
            fixed: false,
            at: None,
            side: None,
            autoplace: true,
            no_pin_swap: vec![],
            no_part_swap: false,
        }
    }

    #[test]
    fn locked_place_produces_fixed_position() {
        let ir = minimal_ir(&[("U1", 5.0, 10.0)]);
        let spec = empty_spec(
            vec![locked_place("U1", 1000.0, 2000.0)],
            UnplacedStrategy::Ignore,
        );
        let (constraints, autoplace) = placement_spec_to_constraints(&spec, &ir).unwrap();
        assert!(autoplace.is_empty(), "locked component must not be in autoplace set");
        assert_eq!(constraints.len(), 1);
        match &constraints[0] {
            UserConstraint::FixedPosition { designator, x_mm, y_mm } => {
                assert_eq!(designator, "U1");
                assert!((x_mm - make_coord(1000.0).to_mms()).abs() < 1e-6);
                assert!((y_mm - make_coord(2000.0).to_mms()).abs() < 1e-6);
            }
            other => panic!("expected FixedPosition, got {:?}", other),
        }
    }

    #[test]
    fn all_components_locked_empty_autoplace_set() {
        let ir = minimal_ir(&[("U1", 0.0, 0.0), ("U2", 10.0, 0.0)]);
        let spec = empty_spec(
            vec![locked_place("U1", 0.0, 0.0), locked_place("U2", 393.7, 0.0)],
            UnplacedStrategy::Ignore,
        );
        let (constraints, autoplace) = placement_spec_to_constraints(&spec, &ir).unwrap();
        assert!(autoplace.is_empty());
        assert_eq!(constraints.len(), 2);
        assert!(
            constraints.iter().all(|c| matches!(c, UserConstraint::FixedPosition { .. }))
        );
    }

    #[test]
    fn autoplace_component_added_to_autoplace_set() {
        let ir = minimal_ir(&[("C1", 0.0, 0.0)]);
        let spec = empty_spec(vec![autoplace_place("C1")], UnplacedStrategy::Ignore);
        let (constraints, autoplace) = placement_spec_to_constraints(&spec, &ir).unwrap();
        assert_eq!(autoplace, vec!["C1"]);
        assert!(constraints.is_empty(), "no sub-constraints for plain autoplace");
    }

    #[test]
    fn autoplace_with_edge_produces_edge_placement() {
        let ir = minimal_ir(&[("C1", 0.0, 0.0)]);
        let mut place = autoplace_place("C1");
        place.edge = Some("top".to_string());
        // ~2.0 mm: 2.0 / 0.0254 ≈ 78.74 mils
        place.inset = Some(make_coord(78.74));
        let spec = empty_spec(vec![place], UnplacedStrategy::Ignore);
        let (constraints, autoplace) = placement_spec_to_constraints(&spec, &ir).unwrap();
        assert_eq!(autoplace, vec!["C1"]);
        assert_eq!(constraints.len(), 1);
        match &constraints[0] {
            UserConstraint::EdgePlacement { designator, edge, inset_mm } => {
                assert_eq!(designator, "C1");
                assert!(matches!(edge, PlacementEdge::Top));
                assert!((inset_mm - make_coord(78.74).to_mms()).abs() < 1e-3);
            }
            other => panic!("expected EdgePlacement, got {:?}", other),
        }
    }

    #[test]
    fn autoplace_with_near_produces_near_constraint() {
        let ir = minimal_ir(&[("C1", 0.0, 0.0), ("U1", 10.0, 0.0)]);
        let mut place = autoplace_place("C1");
        place.near = Some("U1".to_string());
        // ~5 mm: 5 / 0.0254 ≈ 196.85 mils
        place.max_distance = Some(make_coord(196.85));
        let spec = empty_spec(vec![place], UnplacedStrategy::Autoplace);
        let (constraints, autoplace) = placement_spec_to_constraints(&spec, &ir).unwrap();
        assert!(autoplace.contains(&"C1".to_string()));
        let near = constraints.iter().find(|c| matches!(c, UserConstraint::Near { .. }));
        assert!(near.is_some(), "expected Near constraint");
        match near.unwrap() {
            UserConstraint::Near { a, b, max_distance_mm } => {
                assert_eq!(a, "C1");
                assert_eq!(b, "U1");
                assert!((max_distance_mm - make_coord(196.85).to_mms()).abs() < 1e-3);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn designator_in_spec_not_in_ir_warning_no_constraint() {
        let ir = minimal_ir(&[("U1", 0.0, 0.0)]);
        // R99 is not in the IR — should warn but not fail (unplaced: ignore)
        let spec = empty_spec(
            vec![locked_place("R99", 0.0, 0.0)],
            UnplacedStrategy::Ignore,
        );
        let (constraints, autoplace) = placement_spec_to_constraints(&spec, &ir).unwrap();
        // U1 not in spec + unplaced:ignore → FixedPosition for U1
        // R99 not in IR → warning, no constraint
        let r99_constraints: Vec<_> = constraints
            .iter()
            .filter(|c| matches!(c, UserConstraint::FixedPosition { designator, .. } if designator == "R99"))
            .collect();
        assert!(r99_constraints.is_empty(), "R99 not in IR should not produce a constraint");
        assert!(autoplace.is_empty());
    }

    #[test]
    fn unplaced_error_with_missing_component_returns_error() {
        let ir = minimal_ir(&[("U1", 0.0, 0.0), ("U2", 5.0, 0.0)]);
        // Only U1 in spec; unplaced: error → U2 triggers error.
        let spec = empty_spec(
            vec![locked_place("U1", 0.0, 0.0)],
            UnplacedStrategy::Error,
        );
        let result = placement_spec_to_constraints(&spec, &ir);
        assert!(result.is_err(), "expected error for unmentioned component with unplaced: error");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("U2"), "error message should name the missing component");
    }

    #[test]
    fn unplaced_error_with_unknown_spec_designator_returns_error() {
        let ir = minimal_ir(&[("U1", 0.0, 0.0)]);
        // R99 in spec but not in IR, with unplaced: error → returns error.
        let spec = empty_spec(
            vec![locked_place("R99", 0.0, 0.0)],
            UnplacedStrategy::Error,
        );
        let result = placement_spec_to_constraints(&spec, &ir);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("R99"));
    }

    #[test]
    fn unplaced_ignore_adds_fixed_position_for_unmentioned() {
        let ir = minimal_ir(&[("U1", 0.0, 0.0), ("U2", 10.0, 20.0)]);
        // Only U1 in spec; unplaced: ignore → U2 gets FixedPosition at IR coords.
        let spec = empty_spec(
            vec![locked_place("U1", 0.0, 0.0)],
            UnplacedStrategy::Ignore,
        );
        let (constraints, autoplace) = placement_spec_to_constraints(&spec, &ir).unwrap();
        assert!(autoplace.is_empty());
        // 2 constraints: U1 locked + U2 fixed at current pos
        assert_eq!(constraints.len(), 2);
        let u2_fixed = constraints.iter().find(|c| {
            matches!(c, UserConstraint::FixedPosition { designator, .. } if designator == "U2")
        });
        assert!(u2_fixed.is_some());
        match u2_fixed.unwrap() {
            UserConstraint::FixedPosition { x_mm, y_mm, .. } => {
                assert!((x_mm - 10.0).abs() < 1e-6);
                assert!((y_mm - 20.0).abs() < 1e-6);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn unplaced_autoplace_adds_unmentioned_to_autoplace_set() {
        let ir = minimal_ir(&[("U1", 0.0, 0.0), ("U2", 10.0, 20.0)]);
        let spec = empty_spec(
            vec![locked_place("U1", 0.0, 0.0)],
            UnplacedStrategy::Autoplace,
        );
        let (_, autoplace) = placement_spec_to_constraints(&spec, &ir).unwrap();
        assert!(autoplace.contains(&"U2".to_string()));
    }

    #[test]
    fn directional_constraints_translated() {
        let ir = minimal_ir(&[("U1", 0.0, 0.0), ("U2", 10.0, 0.0)]);
        let mut spec = empty_spec(vec![], UnplacedStrategy::Autoplace);
        spec.constraints = vec![PlacementConstraintSpec::LeftOf {
            a: "U1".to_string(),
            b: "U2".to_string(),
            gap: Some(make_coord(394.0)), // ~10 mm
        }];
        let (constraints, _) = placement_spec_to_constraints(&spec, &ir).unwrap();
        let dir = constraints.iter().find(|c| matches!(c, UserConstraint::Directional { .. }));
        assert!(dir.is_some());
        match dir.unwrap() {
            UserConstraint::Directional { a, b, direction, gap_mm } => {
                assert_eq!(a, "U1");
                assert_eq!(b, "U2");
                assert!(matches!(direction, Direction::LeftOf));
                assert!(*gap_mm > 0.0);
            }
            _ => unreachable!(),
        }
    }
}

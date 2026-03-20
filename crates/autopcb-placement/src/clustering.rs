use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};

use autopcb_ir::PcbIr;
use tracing::{debug, info};

use crate::{PlacementConfig, PlacementError, RectRegion, UserConstraint};

#[derive(Debug, Clone)]
pub struct ClusterLeafPlan {
    pub members: Vec<String>,
    pub region: RectRegion,
}

#[derive(Debug, Clone)]
pub struct ClusterPlan {
    pub leaves: Vec<ClusterLeafPlan>,
}

#[derive(Debug, Clone)]
struct Unit {
    members: Vec<String>,
    area: f64,
    degree: f64,
    edge_affinity: EdgeAffinity,
}

#[derive(Debug, Clone, Copy, Default)]
struct EdgeAffinity {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

#[derive(Debug, Clone, Copy)]
enum SplitAxis {
    Horizontal,
    Vertical,
}

impl EdgeAffinity {
    fn score_for_region(&self, axis: SplitAxis, primary: bool) -> f64 {
        match axis {
            SplitAxis::Horizontal => {
                if primary {
                    self.left
                } else {
                    self.right
                }
            }
            SplitAxis::Vertical => {
                if primary {
                    self.bottom
                } else {
                    self.top
                }
            }
        }
    }
}

pub fn build_cluster_plan(
    ir: &PcbIr,
    user_constraints: &[UserConstraint],
    placement_groups: &[Vec<String>],
    config: &PlacementConfig,
) -> Result<Option<ClusterPlan>, PlacementError> {
    info!(
        target: "autopcb_placement::clustering",
        component_count = ir.components.len(),
        user_constraint_count = user_constraints.len(),
        placement_group_count = placement_groups.len(),
        cluster_target_size = config.cluster_target_size,
        cluster_max_depth = config.cluster_max_depth,
        "placement_clustering_started"
    );
    if ir.components.len() <= config.cluster_target_size.max(2) {
        debug!(
            target: "autopcb_placement::clustering",
            "placement_clustering_skipped_small_design"
        );
        return Ok(None);
    }

    let designators: Vec<String> = ir
        .components
        .iter()
        .map(|(_, comp)| comp.designator.clone())
        .collect();
    if designators.len() < 2 {
        return Ok(None);
    }

    let mut desig_to_idx = HashMap::new();
    for (idx, designator) in designators.iter().enumerate() {
        desig_to_idx.insert(designator.clone(), idx);
    }

    let locked_groups = locked_groups(
        &designators,
        &desig_to_idx,
        user_constraints,
        placement_groups,
    )?;
    debug!(
        target: "autopcb_placement::clustering",
        locked_group_count = locked_groups.len(),
        "placement_clustering_locked_groups_built"
    );
    let units = build_units(ir, &locked_groups, &desig_to_idx);
    if units.len() < 2 {
        debug!(
            target: "autopcb_placement::clustering",
            unit_count = units.len(),
            "placement_clustering_skipped_insufficient_units"
        );
        return Ok(None);
    }

    let adjacency = build_adjacency(ir, &units, &desig_to_idx);
    let mut units = units;
    for (idx, unit) in units.iter_mut().enumerate() {
        unit.degree = adjacency[idx].iter().copied().sum();
    }

    let all_units: Vec<usize> = (0..units.len()).collect();
    let mut leaves = Vec::new();
    partition_units(
        &all_units,
        0,
        RectRegion {
            min_x: ir.board.bounds.min.x,
            min_y: ir.board.bounds.min.y,
            max_x: ir.board.bounds.max.x,
            max_y: ir.board.bounds.max.y,
        },
        &units,
        &adjacency,
        config.cluster_target_size.max(2),
        config.cluster_max_depth.max(1),
        &mut leaves,
    );

    if leaves.len() < 2 {
        debug!(
            target: "autopcb_placement::clustering",
            leaf_count = leaves.len(),
            "placement_clustering_skipped_single_leaf"
        );
        return Ok(None);
    }

    info!(
        target: "autopcb_placement::clustering",
        unit_count = units.len(),
        leaf_count = leaves.len(),
        "placement_clustering_finished"
    );
    Ok(Some(ClusterPlan { leaves }))
}

fn locked_groups(
    designators: &[String],
    desig_to_idx: &HashMap<String, usize>,
    user_constraints: &[UserConstraint],
    placement_groups: &[Vec<String>],
) -> Result<Vec<Vec<String>>, PlacementError> {
    let mut uf = UnionFind::new(designators.len());

    for group in placement_groups {
        union_designator_group(&mut uf, group, desig_to_idx)?;
    }

    for constraint in user_constraints {
        match constraint {
            UserConstraint::Directional { a, b, .. } | UserConstraint::Near { a, b, .. } => {
                let ia = *desig_to_idx
                    .get(a)
                    .ok_or_else(|| PlacementError::UnknownComponent(a.clone()))?;
                let ib = *desig_to_idx
                    .get(b)
                    .ok_or_else(|| PlacementError::UnknownComponent(b.clone()))?;
                uf.union(ia, ib);
            }
            UserConstraint::EdgePlacement { .. }
            | UserConstraint::RegionContainment { .. }
            | UserConstraint::FixedPosition { .. } => {}
        }
    }

    let mut groups = BTreeMap::<usize, Vec<String>>::new();
    for designator in designators {
        let idx = *desig_to_idx.get(designator).expect("designator index");
        groups
            .entry(uf.find(idx))
            .or_default()
            .push(designator.clone());
    }

    Ok(groups.into_values().collect())
}

fn union_designator_group(
    uf: &mut UnionFind,
    group: &[String],
    desig_to_idx: &HashMap<String, usize>,
) -> Result<(), PlacementError> {
    let Some(first) = group.first() else {
        return Ok(());
    };
    let first_idx = *desig_to_idx
        .get(first)
        .ok_or_else(|| PlacementError::UnknownComponent(first.clone()))?;
    for designator in &group[1..] {
        let idx = *desig_to_idx
            .get(designator)
            .ok_or_else(|| PlacementError::UnknownComponent(designator.clone()))?;
        uf.union(first_idx, idx);
    }
    Ok(())
}

fn build_units(
    ir: &PcbIr,
    locked_groups: &[Vec<String>],
    desig_to_idx: &HashMap<String, usize>,
) -> Vec<Unit> {
    let bounds = ir.board.bounds;
    locked_groups
        .iter()
        .map(|members| {
            let mut area = 0.0;
            let mut affinity = EdgeAffinity::default();
            for designator in members {
                let comp = ir
                    .components
                    .values()
                    .find(|c| c.designator == *designator)
                    .expect("component in locked group");
                area += comp.local_bounds.width().max(0.5) * comp.local_bounds.height().max(0.5);
                if is_connector(&comp.designator, &comp.pattern) {
                    let left = (comp.position.x - bounds.min.x).abs();
                    let right = (bounds.max.x - comp.position.x).abs();
                    let bottom = (comp.position.y - bounds.min.y).abs();
                    let top = (bounds.max.y - comp.position.y).abs();
                    let min_dist = left.min(right.min(bottom.min(top)));
                    if (left - min_dist).abs() < 1e-6 {
                        affinity.left += 1.0;
                    }
                    if (right - min_dist).abs() < 1e-6 {
                        affinity.right += 1.0;
                    }
                    if (bottom - min_dist).abs() < 1e-6 {
                        affinity.bottom += 1.0;
                    }
                    if (top - min_dist).abs() < 1e-6 {
                        affinity.top += 1.0;
                    }
                }
            }
            let mut sorted_members = members.clone();
            sorted_members.sort();
            let _ = desig_to_idx;
            Unit {
                members: sorted_members,
                area,
                degree: 0.0,
                edge_affinity: affinity,
            }
        })
        .collect()
}

fn build_adjacency(
    ir: &PcbIr,
    units: &[Unit],
    desig_to_idx: &HashMap<String, usize>,
) -> Vec<Vec<f64>> {
    let mut comp_to_unit = HashMap::<usize, usize>::new();
    for (unit_idx, unit) in units.iter().enumerate() {
        for designator in &unit.members {
            if let Some(comp_idx) = desig_to_idx.get(designator) {
                comp_to_unit.insert(*comp_idx, unit_idx);
            }
        }
    }

    let designator_order: Vec<String> = ir
        .components
        .iter()
        .map(|(_, comp)| comp.designator.clone())
        .collect();
    let mut order_lookup = HashMap::new();
    for (idx, designator) in designator_order.iter().enumerate() {
        order_lookup.insert(designator.clone(), idx);
    }

    let mut adjacency = vec![vec![0.0; units.len()]; units.len()];
    for (_, net) in ir.nets.iter() {
        if net.pins.len() < 2 || net.component_count > 8 || is_power_net(&net.name) {
            continue;
        }
        let mut connected_units = Vec::<usize>::new();
        for pin in &net.pins {
            if let Some(comp) = ir.components.get(pin.component) {
                if let Some(comp_order) = order_lookup.get(&comp.designator) {
                    if let Some(&unit_idx) = comp_to_unit.get(comp_order) {
                        if !connected_units.contains(&unit_idx) {
                            connected_units.push(unit_idx);
                        }
                    }
                }
            }
        }
        if connected_units.len() < 2 {
            continue;
        }
        let weight = 1.0 / (connected_units.len() as f64 - 1.0).max(1.0);
        for i in 0..connected_units.len() {
            for j in (i + 1)..connected_units.len() {
                let a = connected_units[i];
                let b = connected_units[j];
                adjacency[a][b] += weight;
                adjacency[b][a] += weight;
            }
        }
    }
    adjacency
}

#[allow(clippy::too_many_arguments)]
fn partition_units(
    unit_indices: &[usize],
    depth: usize,
    region: RectRegion,
    units: &[Unit],
    adjacency: &[Vec<f64>],
    target_size: usize,
    max_depth: usize,
    out: &mut Vec<ClusterLeafPlan>,
) {
    let member_count: usize = unit_indices
        .iter()
        .map(|&idx| units[idx].members.len())
        .sum();
    if member_count <= target_size || depth >= max_depth || unit_indices.len() <= 1 {
        let mut members = Vec::new();
        for &idx in unit_indices {
            members.extend(units[idx].members.iter().cloned());
        }
        members.sort();
        out.push(ClusterLeafPlan { members, region });
        return;
    }

    let axis = if (region.max_x - region.min_x) >= (region.max_y - region.min_y) {
        SplitAxis::Horizontal
    } else {
        SplitAxis::Vertical
    };

    let (mut part_a, mut part_b) = grow_bipartition(unit_indices, units, adjacency);
    if part_a.is_empty() || part_b.is_empty() {
        let mut sorted = unit_indices.to_vec();
        sorted.sort_by(|&lhs, &rhs| unit_sort_key(lhs, rhs, units));
        let mid = sorted.len() / 2;
        part_a = sorted[..mid].to_vec();
        part_b = sorted[mid..].to_vec();
    }
    if part_a.is_empty() || part_b.is_empty() {
        let mut members = Vec::new();
        for &idx in unit_indices {
            members.extend(units[idx].members.iter().cloned());
        }
        members.sort();
        out.push(ClusterLeafPlan { members, region });
        return;
    }

    let area_a: f64 = part_a.iter().map(|&idx| units[idx].area).sum();
    let area_b: f64 = part_b.iter().map(|&idx| units[idx].area).sum();
    let (region_a, region_b) = split_region(region, axis, area_a, area_b);

    let direct_score = partition_affinity(&part_a, units, axis, true)
        + partition_affinity(&part_b, units, axis, false);
    let swapped_score = partition_affinity(&part_a, units, axis, false)
        + partition_affinity(&part_b, units, axis, true);
    let (left_partition, right_partition) = if swapped_score > direct_score {
        (part_b, part_a)
    } else {
        (part_a, part_b)
    };

    partition_units(
        &left_partition,
        depth + 1,
        region_a,
        units,
        adjacency,
        target_size,
        max_depth,
        out,
    );
    partition_units(
        &right_partition,
        depth + 1,
        region_b,
        units,
        adjacency,
        target_size,
        max_depth,
        out,
    );
}

fn unit_sort_key(lhs: usize, rhs: usize, units: &[Unit]) -> Ordering {
    units[lhs].members.first().cmp(&units[rhs].members.first())
}

fn grow_bipartition(
    unit_indices: &[usize],
    units: &[Unit],
    adjacency: &[Vec<f64>],
) -> (Vec<usize>, Vec<usize>) {
    let mut sorted = unit_indices.to_vec();
    sorted.sort_by(|&lhs, &rhs| {
        units[rhs]
            .degree
            .partial_cmp(&units[lhs].degree)
            .unwrap_or(Ordering::Equal)
            .then_with(|| unit_sort_key(lhs, rhs, units))
    });
    let seed_a = sorted[0];
    let seed_b = sorted
        .iter()
        .copied()
        .skip(1)
        .min_by(|&lhs, &rhs| {
            adjacency[seed_a][lhs]
                .partial_cmp(&adjacency[seed_a][rhs])
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    units[rhs]
                        .degree
                        .partial_cmp(&units[lhs].degree)
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| unit_sort_key(lhs, rhs, units))
        })
        .unwrap_or(seed_a);

    let mut part_a = vec![seed_a];
    let mut part_b = if seed_b != seed_a {
        vec![seed_b]
    } else {
        Vec::new()
    };
    let mut unassigned: HashSet<usize> = unit_indices.iter().copied().collect();
    unassigned.remove(&seed_a);
    unassigned.remove(&seed_b);
    let mut area_a = units[seed_a].area;
    let mut area_b = if seed_b != seed_a {
        units[seed_b].area
    } else {
        0.0
    };

    while !unassigned.is_empty() {
        let prefer_a = area_a <= area_b || part_b.is_empty();
        let target = if prefer_a { &part_a } else { &part_b };
        let choice = pick_best_unit(&unassigned, target, units, adjacency);
        if prefer_a {
            part_a.push(choice);
            area_a += units[choice].area;
        } else {
            part_b.push(choice);
            area_b += units[choice].area;
        }
        unassigned.remove(&choice);
    }

    (part_a, part_b)
}

fn pick_best_unit(
    candidates: &HashSet<usize>,
    target_partition: &[usize],
    units: &[Unit],
    adjacency: &[Vec<f64>],
) -> usize {
    candidates
        .iter()
        .copied()
        .max_by(|&lhs, &rhs| {
            let lhs_link = partition_link(lhs, target_partition, adjacency);
            let rhs_link = partition_link(rhs, target_partition, adjacency);
            lhs_link
                .partial_cmp(&rhs_link)
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    units[lhs]
                        .degree
                        .partial_cmp(&units[rhs].degree)
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| {
                    units[rhs]
                        .area
                        .partial_cmp(&units[lhs].area)
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| unit_sort_key(lhs, rhs, units))
        })
        .expect("at least one clustering candidate")
}

fn partition_link(unit_idx: usize, target_partition: &[usize], adjacency: &[Vec<f64>]) -> f64 {
    target_partition
        .iter()
        .map(|&other| adjacency[unit_idx][other])
        .sum()
}

fn split_region(
    region: RectRegion,
    axis: SplitAxis,
    area_a: f64,
    area_b: f64,
) -> (RectRegion, RectRegion) {
    let ratio = if (area_a + area_b) > 0.0 {
        area_a / (area_a + area_b)
    } else {
        0.5
    }
    .clamp(0.25, 0.75);

    match axis {
        SplitAxis::Horizontal => {
            let split_x = region.min_x + (region.max_x - region.min_x) * ratio;
            (
                RectRegion {
                    min_x: region.min_x,
                    min_y: region.min_y,
                    max_x: split_x,
                    max_y: region.max_y,
                },
                RectRegion {
                    min_x: split_x,
                    min_y: region.min_y,
                    max_x: region.max_x,
                    max_y: region.max_y,
                },
            )
        }
        SplitAxis::Vertical => {
            let split_y = region.min_y + (region.max_y - region.min_y) * ratio;
            (
                RectRegion {
                    min_x: region.min_x,
                    min_y: region.min_y,
                    max_x: region.max_x,
                    max_y: split_y,
                },
                RectRegion {
                    min_x: region.min_x,
                    min_y: split_y,
                    max_x: region.max_x,
                    max_y: region.max_y,
                },
            )
        }
    }
}

fn partition_affinity(partition: &[usize], units: &[Unit], axis: SplitAxis, primary: bool) -> f64 {
    partition
        .iter()
        .map(|&idx| units[idx].edge_affinity.score_for_region(axis, primary))
        .sum()
}

fn is_power_net(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    upper.contains("GND")
        || upper.contains("VCC")
        || upper.contains("VDD")
        || upper.contains("VSS")
        || upper.contains("VBUS")
        || upper.contains("VIN")
        || upper.contains("3V3")
        || upper.contains("5V")
}

fn is_connector(designator: &str, pattern: &str) -> bool {
    let pattern_upper = pattern.to_ascii_uppercase();
    designator.starts_with('J')
        || designator.starts_with('P')
        || pattern_upper.contains("USB")
        || pattern_upper.contains("CONN")
        || pattern_upper.contains("HDR")
        || pattern_upper.contains("GCT")
        || pattern_upper.contains(" DF")
}

#[derive(Debug, Clone)]
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl UnionFind {
    fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
            rank: vec![0; size],
        }
    }

    fn find(&mut self, idx: usize) -> usize {
        if self.parent[idx] != idx {
            let parent = self.parent[idx];
            self.parent[idx] = self.find(parent);
        }
        self.parent[idx]
    }

    fn union(&mut self, lhs: usize, rhs: usize) {
        let lhs_root = self.find(lhs);
        let rhs_root = self.find(rhs);
        if lhs_root == rhs_root {
            return;
        }
        match self.rank[lhs_root].cmp(&self.rank[rhs_root]) {
            Ordering::Less => self.parent[lhs_root] = rhs_root,
            Ordering::Greater => self.parent[rhs_root] = lhs_root,
            Ordering::Equal => {
                self.parent[rhs_root] = lhs_root;
                self.rank[lhs_root] += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use autopcb_ir::{
        BoardSide, BoundingBoxMm, ComponentId, IdMap, IrBoardGeometry, IrComponent, IrComponentPad,
        IrCopperLayer, IrLayerStack, IrNet, IrNetPin, PadId, PadShapeInfo, PadShapeKind, PcbIr,
        PointMm,
    };

    fn simple_ir() -> PcbIr {
        let mut components = IdMap::<ComponentId, IrComponent>::new();
        let mut next_pad = 0u32;
        for (designator, x, y) in [
            ("J1", 5.0, 40.0),
            ("U1", 20.0, 40.0),
            ("U2", 80.0, 40.0),
            ("J2", 95.0, 40.0),
        ] {
            let pad_id = PadId::from(next_pad);
            next_pad += 1;
            let comp_id = components.push(IrComponent {
                id: ComponentId::from(0),
                designator: designator.to_string(),
                pattern: designator.to_string(),
                value: String::new(),
                position: PointMm::new(x, y),
                rotation: 0.0,
                side: BoardSide::Top,
                local_bounds: BoundingBoxMm::new(PointMm::new(-2.0, -1.0), PointMm::new(2.0, 1.0)),
                world_bounds: BoundingBoxMm::new(
                    PointMm::new(x - 2.0, y - 1.0),
                    PointMm::new(x + 2.0, y + 1.0),
                ),
                pads: vec![IrComponentPad {
                    id: pad_id,
                    name: "1".into(),
                    local_position: PointMm::new(0.0, 0.0),
                    world_position: PointMm::new(x, y),
                    net: None,
                    shape: PadShapeInfo {
                        kind: PadShapeKind::Rectangular,
                        size_x: 1.0,
                        size_y: 1.0,
                        rotation: 0.0,
                    },
                    is_through_hole: false,
                    hole_size_mm: 0.0,
                    swap_id_pin: None,
                    swap_id_part: None,
                }],
            });
            components[comp_id].id = comp_id;
        }

        let mut nets = IdMap::new();
        let net0 = nets.push(IrNet {
            id: autopcb_ir::NetId::from(0),
            name: "SIG_A".into(),
            pins: vec![
                IrNetPin {
                    pad: components[ComponentId::from(0)].pads[0].id,
                    component: ComponentId::from(0),
                    position: components[ComponentId::from(0)].pads[0].world_position,
                },
                IrNetPin {
                    pad: components[ComponentId::from(1)].pads[0].id,
                    component: ComponentId::from(1),
                    position: components[ComponentId::from(1)].pads[0].world_position,
                },
            ],
            component_count: 2,
        });
        nets[net0].id = net0;
        let net1 = nets.push(IrNet {
            id: autopcb_ir::NetId::from(0),
            name: "SIG_B".into(),
            pins: vec![
                IrNetPin {
                    pad: components[ComponentId::from(2)].pads[0].id,
                    component: ComponentId::from(2),
                    position: components[ComponentId::from(2)].pads[0].world_position,
                },
                IrNetPin {
                    pad: components[ComponentId::from(3)].pads[0].id,
                    component: ComponentId::from(3),
                    position: components[ComponentId::from(3)].pads[0].world_position,
                },
            ],
            component_count: 2,
        });
        nets[net1].id = net1;

        PcbIr {
            board: IrBoardGeometry {
                outline: vec![
                    PointMm::new(0.0, 0.0),
                    PointMm::new(100.0, 0.0),
                    PointMm::new(100.0, 80.0),
                    PointMm::new(0.0, 80.0),
                ],
                cutouts: Vec::new(),
                bounds: BoundingBoxMm::new(PointMm::new(0.0, 0.0), PointMm::new(100.0, 80.0)),
                keepouts: Vec::new(),
            },
            layer_stack: IrLayerStack {
                copper_layers: vec![IrCopperLayer {
                    id: autopcb_ir::LayerId::from(0),
                    name: "Top".into(),
                    is_top: true,
                    is_bottom: false,
                }],
                copper_layer_count: 2,
            },
            components,
            nets,
            rules: IdMap::new(),
            free_copper: Default::default(),
            polygons: IdMap::new(),
        }
    }

    #[test]
    fn cluster_plan_preserves_user_groups() {
        let ir = simple_ir();
        let config = PlacementConfig {
            auto_cluster: true,
            cluster_target_size: 2,
            cluster_max_depth: 2,
            ..PlacementConfig::default()
        };
        let plan = build_cluster_plan(&ir, &[], &[vec!["U1".into(), "U2".into()]], &config)
            .expect("cluster plan")
            .expect("non-trivial clustering");

        let containing_leaves: Vec<&ClusterLeafPlan> = plan
            .leaves
            .iter()
            .filter(|leaf| {
                leaf.members.contains(&"U1".to_string()) || leaf.members.contains(&"U2".to_string())
            })
            .collect();
        assert_eq!(
            containing_leaves.len(),
            1,
            "group members should share one leaf"
        );
        let leaf = containing_leaves[0];
        assert!(leaf.members.contains(&"U1".to_string()));
        assert!(leaf.members.contains(&"U2".to_string()));
    }
}

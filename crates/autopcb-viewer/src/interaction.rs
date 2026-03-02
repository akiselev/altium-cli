//! Hit-testing and hover interaction.

use autopcb_ir::{BoardSide, ComponentId, NetId, PcbIr, PointMm};

/// Find which component (if any) contains `world_pos` using AABB hit-test.
pub fn find_component_at(ir: &PcbIr, world_pos: PointMm) -> Option<ComponentId> {
    for (id, comp) in ir.components.iter() {
        if comp.world_bounds.contains(&world_pos) {
            return Some(id);
        }
    }
    None
}

/// Collect the unique `NetId`s connected to a given component (via its pads).
pub fn nets_for_component(ir: &PcbIr, id: ComponentId) -> Vec<NetId> {
    let comp = &ir.components[id];
    let mut nets: Vec<NetId> = comp.pads.iter().filter_map(|p| p.net).collect();
    nets.sort_by_key(|n| n.raw());
    nets.dedup();
    nets
}

/// Build a tooltip string for a hovered component.
pub fn component_tooltip(ir: &PcbIr, id: ComponentId) -> String {
    let comp = &ir.components[id];
    let side = match comp.side {
        BoardSide::Top => "Top",
        BoardSide::Bottom => "Bottom",
    };
    format!(
        "{} ({})\nPattern: {}\nValue: {}\nSide: {}\nPads: {}\nPos: ({:.2}, {:.2}) mm",
        comp.designator,
        comp.id,
        comp.pattern,
        comp.value,
        side,
        comp.pads.len(),
        comp.position.x,
        comp.position.y
    )
}

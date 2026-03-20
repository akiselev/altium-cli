// via_check.wgsl — hole size and annular ring check for vias.
// Dispatch: one thread per via.
// Input:  vias buffer (drill_mm, annular_ring_mm), via_bounds uniform.
// Output: violation buffer.

struct ViaBounds {
    hole_min_mm: f32,
    hole_max_mm: f32,
    annular_ring_min_mm: f32,
}

struct Via {
    position_x: f32,
    position_y: f32,
    drill_mm: f32,
    annular_ring_mm: f32,
}

@group(0) @binding(0) var<storage, read>       vias: array<Via>;
@group(0) @binding(1) var<uniform>             via_bounds: ViaBounds;
@group(0) @binding(2) var<storage, read_write> violations: array<u32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // STUB: not yet implemented
    let idx = gid.x;
    _ = vias[idx].drill_mm;
    _ = via_bounds.hole_min_mm;
}

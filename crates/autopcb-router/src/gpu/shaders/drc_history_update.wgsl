// drc_history_update.wgsl — history cost increment for DRC violations.
// For each violation, increments the history cost of the affected cells/edges
// so the PathFinder penalizes repeatedly-violated locations.
// Dispatch: one thread per compacted violation.
// Input:  compact violations buffer, penalty uniform.
// Output: history cost buffer (read_write, atomic adds).

@group(0) @binding(0) var<storage, read>       violations: array<u32>;
@group(0) @binding(1) var<uniform>             penalty: f32;
@group(0) @binding(2) var<storage, read_write> history_costs: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // STUB: not yet implemented
    let idx = gid.x;
    _ = violations[idx];
    _ = penalty;
}

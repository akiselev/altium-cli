// segment_extract.wgsl — per-layer segment filtering.
// Dispatch: one workgroup per layer.
// Input:  all_segments buffer, layer_id uniform.
// Output: filtered segments buffer for the target layer.

@group(0) @binding(0) var<storage, read>       all_segments: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> layer_segments: array<vec4<f32>>;
@group(0) @binding(2) var<uniform>             layer_id: u32;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // STUB: not yet implemented
    let idx = gid.x;
    _ = all_segments[idx];
    _ = layer_id;
}

// short_check.wgsl — occupancy overlap detection (short-circuit check).
// Dispatch: one thread per segment.
// Input:  occupancy grid buffer, segments buffer.
// Output: violation buffer.

@group(0) @binding(0) var<storage, read>       segments: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read>       occupancy: array<u32>;
@group(0) @binding(2) var<storage, read_write> violations: array<u32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // STUB: not yet implemented
    let idx = gid.x;
    _ = segments[idx];
    _ = occupancy[idx];
}

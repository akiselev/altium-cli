// width_check.wgsl — per-segment width bounds check.
// Dispatch: one thread per segment.
// Input:  segments buffer (vec4: x0,y0,x1,y1; width packed elsewhere),
//         width_bounds uniform (min_mm, max_mm).
// Output: violation buffer.

struct WidthBounds {
    min_mm: f32,
    max_mm: f32,
}

@group(0) @binding(0) var<storage, read>       segments: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read>       segment_widths: array<f32>;
@group(0) @binding(2) var<uniform>             width_bounds: WidthBounds;
@group(0) @binding(3) var<storage, read_write> violations: array<u32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // STUB: not yet implemented
    let idx = gid.x;
    _ = segments[idx];
    _ = segment_widths[idx];
    _ = width_bounds.min_mm;
}

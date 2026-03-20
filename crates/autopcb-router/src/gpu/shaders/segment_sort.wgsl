// segment_sort.wgsl — y-coordinate radix sort for segment list.
// Dispatch: multiple passes, one workgroup per 256-element block.
// Input:  unsorted segments buffer, pass uniform (digit position).
// Output: sorted segments buffer, histogram buffer.

@group(0) @binding(0) var<storage, read>       input_segments: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> output_segments: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> histogram: array<u32>;
@group(0) @binding(3) var<uniform>             sort_pass: u32;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // STUB: not yet implemented
    let idx = gid.x;
    _ = input_segments[idx];
    _ = sort_pass;
}

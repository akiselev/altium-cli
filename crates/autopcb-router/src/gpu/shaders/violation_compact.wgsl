// violation_compact.wgsl — stream compaction of sparse violation buffer.
// Uses parallel prefix sum to pack non-zero violation entries into a dense output.
// Dispatch: one workgroup per 256-element block of the input.
// Input:  sparse violations buffer (0 = no violation, non-zero = violation index).
// Output: compact violations buffer, count atomic.

@group(0) @binding(0) var<storage, read>       sparse: array<u32>;
@group(0) @binding(1) var<storage, read_write> compact: array<u32>;
@group(0) @binding(2) var<storage, read_write> count: atomic<u32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // STUB: not yet implemented
    let idx = gid.x;
    _ = sparse[idx];
}

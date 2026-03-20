# GPU DRC Architecture

## Overview

The `gpu` module provides a GPU-accelerated DRC backend that runs the full set of
clearance and short-circuit checks in parallel on the graphics card via `wgpu`.
The module is compiled only when the `gpu` Cargo feature is enabled.

At runtime, `DrcEngineSelector` automatically picks the GPU backend when the
segment count exceeds the **gpu_threshold** (default: **5000 segments**) and a
compatible GPU adapter is available.  Below this threshold, or when no adapter is
found, the CPU fallback (`CpuDrcEngine`) is always used.

## X-Check Parallel Sweepline Algorithm

The GPU clearance check uses an **X-Check** parallel sweepline:

1. **Segment extract** (`segment_extract.wgsl`): filter all segments to a single
   layer, producing a per-layer active list.
2. **Segment sort** (`segment_sort.wgsl`): radix-sort the layer list by Y
   coordinate so that the sweepline can advance monotonically.
3. **Sweepline check** (`sweepline_check.wgsl`): each thread processes one segment
   from the sorted list as the "query" and scans a narrow active window for
   candidates within the bounding-box heuristic.  Actual Euclidean distance is
   computed only for candidates that pass the bounding test.
4. **Short check** (`short_check.wgsl`): occupancy-grid overlap test for
   short-circuit detection; runs independently of the sweepline.
5. **Width check** (`width_check.wgsl`): per-segment width-bounds validation.
6. **Via check** (`via_check.wgsl`): hole-size and annular-ring bounds per via.
7. **Violation compact** (`violation_compact.wgsl`): parallel prefix-sum stream
   compaction to pack sparse violation flags into a dense output buffer.
8. **DRC history update** (`drc_history_update.wgsl`): increments PathFinder
   history costs for cells/edges associated with violations so that repeatedly
   violated locations are penalised in subsequent routing iterations.

## Current Status

GPU shaders are **stubs** — each `.wgsl` file contains the correct binding
declarations and workgroup size but the compute body is not yet implemented.
The CPU fallback (`CpuDrcEngine`) is always used at runtime regardless of the
segment count.

## gpu_threshold Auto-Selection

```
if segment_count >= gpu_threshold (default 5000) AND gpu adapter available:
    use GpuDrcEngine
else:
    use CpuDrcEngine
```

The threshold can be overridden via `RouterConfig::gpu_threshold`.

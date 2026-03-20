//! Serpentine/accordion insertion for matched-length constraints.
//!
//! Inserts meander segments in the longest straight segment of a routed net to
//! pad its total length to `target_length_mm`. If the net is already at or
//! above the target length no segments are inserted.
//!
//! The serpentine pattern is a series of alternating perpendicular excursions
//! (accordion bends) inserted into the middle of the selected segment. Each
//! full meander loop adds approximately `2 × amplitude_mm + pitch_mm` to the
//! path length.

use autopcb_routes::{NetId, Point, TraceSegment};

/// Parameters for serpentine insertion.
pub struct SerpentineParams {
    /// Half-width of each meander excursion in mm.
    pub amplitude_mm: f64,
    /// Distance between successive meander centres in mm (along the base trace).
    pub pitch_mm: f64,
    /// Target total trace length in mm.
    pub target_length_mm: f64,
}

/// Tolerance for treating a coordinate difference as zero (mm).
const EPS: f64 = 1e-9;

/// Segment length in mm.
fn seg_len(s: &TraceSegment) -> f64 {
    let dx = s.end.x - s.start.x;
    let dy = s.end.y - s.start.y;
    (dx * dx + dy * dy).sqrt()
}

/// Total path length of all segments.
fn total_length(segments: &[TraceSegment]) -> f64 {
    segments.iter().map(seg_len).sum()
}

/// Unit vector along a segment.
fn unit_dir(s: &TraceSegment) -> (f64, f64) {
    let dx = s.end.x - s.start.x;
    let dy = s.end.y - s.start.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < EPS {
        (1.0, 0.0)
    } else {
        (dx / len, dy / len)
    }
}

/// Perpendicular (left-rotated) unit vector.
fn perp(ux: f64, uy: f64) -> (f64, f64) {
    (-uy, ux)
}

/// Find the index of the longest segment in `segments` that belongs to
/// `net_id` and has enough length to accommodate the insertion.
///
/// Returns `None` if no suitable segment is found.
fn find_longest_segment(segments: &[TraceSegment], net_id: NetId) -> Option<usize> {
    segments
        .iter()
        .enumerate()
        .filter(|(_, s)| s.net_id == net_id)
        .max_by(|(_, a), (_, b)| seg_len(a).partial_cmp(&seg_len(b)).unwrap())
        .map(|(i, _)| i)
}

/// Generate the accordion meander segments to be inserted in place of the
/// middle portion of `base`.
///
/// `loops` is the number of complete meander loops.  Each loop consists of 4
/// segments forming a rectangular U-shape laid along the base axis:
///
/// ```text
/// base axis ───────────────────────────────────→
///                  ┌──────pitch──────┐
///                  │                 │
/// ──pitch/2──→ excursion        excursion back ──pitch/2──→
/// ```
///
/// Each loop adds `2×amplitude + pitch` in extra length beyond the `pitch`
/// of base axis it consumes.
fn build_serpentine(
    base: &TraceSegment,
    loops: u32,
    amplitude: f64,
    pitch: f64,
) -> Vec<TraceSegment> {
    if loops == 0 {
        return vec![base.clone()];
    }

    let (ux, uy) = unit_dir(base);
    let (px, py) = perp(ux, uy);

    // We insert the serpentine in the centre of the base segment, keeping
    // a small margin at each end equal to `pitch/2`.
    let base_len = seg_len(base);
    let margin = (pitch / 2.0).min(base_len / 4.0);
    let available = base_len - 2.0 * margin;

    // Each loop occupies `pitch` along the base direction.
    let total_pitch_needed = loops as f64 * pitch;
    if total_pitch_needed > available {
        // Not enough room — return the segment unchanged.
        return vec![base.clone()];
    }

    let mut result: Vec<TraceSegment> = Vec::with_capacity(loops as usize * 4 + 2);

    // Lead-in: from base.start to the first meander start.
    let lead_in_end = Point {
        x: base.start.x + ux * margin,
        y: base.start.y + uy * margin,
    };
    result.push(TraceSegment {
        net_id: base.net_id,
        layer: base.layer,
        start: base.start,
        end: lead_in_end,
        width_mm: base.width_mm,
    });

    let mut cursor = lead_in_end;

    for loop_idx in 0..loops {
        // Alternate direction of the perpendicular excursion.
        let sign = if loop_idx % 2 == 0 { 1.0 } else { -1.0 };
        let exc_x = px * amplitude * sign;
        let exc_y = py * amplitude * sign;

        // Each loop consumes exactly `pitch` along the base axis and adds
        // `2×amplitude` in perpendicular travel:
        //
        //   cursor ──pitch/2──→ p1 ──amplitude⊥──→ p2
        //                                           │
        //            p4 ←──amplitude⊥─── p3  ←──pitch/2──
        //
        // cursor→p1: half pitch on axis
        // p1→p2:     amplitude perpendicular out
        // p2→p3:     nothing — p3 = p2 shifted half pitch on axis
        // p3→p4:     amplitude perpendicular back to axis
        // (p4 = cursor + pitch, the start of the next loop)

        let p1 = Point {
            x: cursor.x + ux * (pitch / 2.0),
            y: cursor.y + uy * (pitch / 2.0),
        };
        let p2 = Point {
            x: p1.x + exc_x,
            y: p1.y + exc_y,
        };
        // p3 is directly above/below where p4 will be (half pitch further on axis).
        let p4_on_axis = Point {
            x: cursor.x + ux * pitch,
            y: cursor.y + uy * pitch,
        };
        let p3 = Point {
            x: p4_on_axis.x + exc_x,
            y: p4_on_axis.y + exc_y,
        };

        // Emit 4 segments: cursor→p1, p1→p2, p2→p3, p3→p4_on_axis.
        result.push(TraceSegment {
            net_id: base.net_id,
            layer: base.layer,
            start: cursor,
            end: p1,
            width_mm: base.width_mm,
        });
        result.push(TraceSegment {
            net_id: base.net_id,
            layer: base.layer,
            start: p1,
            end: p2,
            width_mm: base.width_mm,
        });
        result.push(TraceSegment {
            net_id: base.net_id,
            layer: base.layer,
            start: p2,
            end: p3,
            width_mm: base.width_mm,
        });
        result.push(TraceSegment {
            net_id: base.net_id,
            layer: base.layer,
            start: p3,
            end: p4_on_axis,
            width_mm: base.width_mm,
        });

        cursor = p4_on_axis;
    }

    // Lead-out: from end of serpentine to base.end.
    result.push(TraceSegment {
        net_id: base.net_id,
        layer: base.layer,
        start: cursor,
        end: base.end,
        width_mm: base.width_mm,
    });

    result
}

/// Insert serpentine meanders into `segments` for `net_id` to reach
/// `params.target_length_mm`.
///
/// If the net's current total length already meets or exceeds the target,
/// no modification is made.
pub fn insert_serpentine(
    segments: &mut Vec<TraceSegment>,
    params: &SerpentineParams,
    net_id: NetId,
) {
    let current_length = total_length(segments);
    if current_length + EPS >= params.target_length_mm {
        return;
    }

    let shortage = params.target_length_mm - current_length;

    // Each loop consumes `pitch` mm of the base segment's length but adds
    // `pitch/2 + amplitude + pitch/2 + amplitude = pitch + 2×amplitude` mm
    // of new path length.  The net gain per loop is therefore `2×amplitude`.
    let gain_per_loop = 2.0 * params.amplitude_mm;
    if gain_per_loop < EPS {
        return;
    }
    let loops = (shortage / gain_per_loop).ceil() as u32;

    // Find the best segment to insert into.
    let idx = match find_longest_segment(segments, net_id) {
        Some(i) => i,
        None => return,
    };

    let base = segments[idx].clone();
    let replacement = build_serpentine(&base, loops, params.amplitude_mm, params.pitch_mm);

    // Replace segment at idx with the serpentine expansion.
    segments.splice(idx..=idx, replacement);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use autopcb_routes::{LayerId, NetId, Point, TraceSegment};

    fn seg(x0: f64, y0: f64, x1: f64, y1: f64) -> TraceSegment {
        TraceSegment {
            net_id: NetId(0),
            layer: LayerId(0),
            start: Point { x: x0, y: y0 },
            end: Point { x: x1, y: y1 },
            width_mm: 0.2,
        }
    }

    fn make_params(amplitude: f64, pitch: f64, target: f64) -> SerpentineParams {
        SerpentineParams {
            amplitude_mm: amplitude,
            pitch_mm: pitch,
            target_length_mm: target,
        }
    }

    /// A net shorter than the target gets serpentine loops added, increasing
    /// total length toward the target.
    ///
    /// Base: 50mm horizontal. amplitude=1mm, pitch=2mm, target=60mm.
    /// shortage=10mm, gain_per_loop=2×amplitude=2mm, loops=5.
    /// Each loop: pitch/2 + amplitude + pitch/2 + amplitude = 4mm segment length,
    /// consuming pitch=2mm of base, so gain = 4-2 = 2mm per loop.
    /// 5 loops × 2mm gain = 10mm extra → total ≈ 60mm.
    #[test]
    fn short_net_gets_padded_toward_target() {
        // Single 50 mm horizontal segment.
        let mut segs = vec![seg(0.0, 0.0, 50.0, 0.0)];
        let params = make_params(1.0, 2.0, 60.0);
        insert_serpentine(&mut segs, &params, NetId(0));

        let after = total_length(&segs);
        assert!(
            after > 50.0,
            "expected length > 50 after serpentine, got {after}"
        );
        assert!(
            after >= 58.0,
            "expected close to 60mm target, got {after}"
        );
    }

    /// A net that already meets the target: no modification.
    #[test]
    fn already_long_enough_unchanged() {
        let mut segs = vec![seg(0.0, 0.0, 10.0, 0.0)];
        let before_len = segs.len();
        let params = make_params(1.0, 2.0, 8.0); // target < current length
        insert_serpentine(&mut segs, &params, NetId(0));
        assert_eq!(segs.len(), before_len, "no serpentine should be added");
    }

    /// A net exactly at the target: no modification.
    #[test]
    fn exactly_at_target_unchanged() {
        let mut segs = vec![seg(0.0, 0.0, 10.0, 0.0)];
        let before_len = segs.len();
        let params = make_params(1.0, 2.0, 10.0);
        insert_serpentine(&mut segs, &params, NetId(0));
        assert_eq!(segs.len(), before_len, "no serpentine at exact target");
    }

    /// After insertion, segment count increases (serpentine adds segments).
    ///
    /// Base: 100mm, target=110mm. shortage=10mm, gain=2×amplitude=2mm/loop,
    /// loops=5. pitch=2mm, total_pitch=10mm. available=100-2×1=98mm. Fits.
    #[test]
    fn segment_count_increases() {
        let mut segs = vec![seg(0.0, 0.0, 100.0, 0.0)];
        let original_count = segs.len();
        let params = make_params(1.0, 2.0, 110.0);
        insert_serpentine(&mut segs, &params, NetId(0));
        assert!(
            segs.len() > original_count,
            "expected more segments after serpentine, got {}",
            segs.len()
        );
    }

    /// Serpentine for a different net_id does not modify a single-net trace.
    #[test]
    fn different_net_id_not_modified() {
        let mut segs = vec![seg(0.0, 0.0, 10.0, 0.0)];
        let before_len = segs.len();
        let params = make_params(1.0, 2.0, 30.0);
        insert_serpentine(&mut segs, &params, NetId(99)); // wrong net
        assert_eq!(segs.len(), before_len, "different net must not be modified");
    }
}

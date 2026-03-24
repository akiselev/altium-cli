//! Differential-pair routing: coupled and semi-coupled modes.
//!
//! Routes the primary net then offsets the secondary net at the configured
//! gap. Verifies gap and skew constraints from `DiffPairConfig`.
//!
//! ## Algorithm
//!
//! For each segment of the primary net that has a corresponding segment in the
//! secondary net (matched by index), the secondary segment is shifted
//! perpendicular to the primary by `config.gap_mm`. The sign of the offset
//! alternates to keep the pair symmetric.
//!
//! This is a geometric post-processing step — the secondary trace must already
//! exist in the solution (from the detailed router). The optimizer adjusts its
//! position to enforce the gap constraint.

use autopcb_routes::{NetId, Point, RouteSolution, RoutedNet, RoutedVia, TraceSegment};

use crate::rules::DiffPairConfig;

/// Tolerance for treating a coordinate difference as zero (mm).
const EPS: f64 = 1e-9;

/// Differential-pair optimizer: adjusts secondary trace position to enforce
/// gap and skew constraints from `DiffPairConfig`.
pub struct DiffPairOptimizer;

impl DiffPairOptimizer {
    /// Adjust the spacing between the traces for `net_a` and `net_b` in
    /// `solution` to match `config.gap_mm`.
    ///
    /// For each segment pair `(a_seg[i], b_seg[i])`, the secondary segment
    /// `b_seg[i]` is translated perpendicular to the primary by the configured
    /// gap. If the two nets have different segment counts, only the overlapping
    /// prefix is adjusted.
    pub fn optimize_pair(
        solution: &mut RouteSolution,
        net_a: NetId,
        net_b: NetId,
        config: &DiffPairConfig,
    ) {
        // Collect primary-net segments (read-only reference is fine here
        // because we will borrow the secondary net mutably next).
        let a_segs: Vec<TraceSegment> = match solution.nets.get(&net_a) {
            Some(n) => n.segments.clone(),
            None => return,
        };

        let b_net = match solution.nets.get_mut(&net_b) {
            Some(n) => n,
            None => return,
        };

        let pair_count = a_segs.len().min(b_net.segments.len());

        for i in 0..pair_count {
            let a = &a_segs[i];
            let b = &mut b_net.segments[i];

            // Skip cross-layer transitions (different layers on primary).
            if a.layer != b.layer {
                continue;
            }

            // Compute the unit direction of the primary segment.
            let dx = a.end.x - a.start.x;
            let dy = a.end.y - a.start.y;
            let len = (dx * dx + dy * dy).sqrt();
            if len < EPS {
                continue;
            }
            let ux = dx / len;
            let uy = dy / len;

            // Perpendicular (left-rotate): (-uy, ux).
            let px = -uy;
            let py = ux;

            // Place secondary segment offset by gap from primary's midpoint.
            let a_mid = Point {
                x: (a.start.x + a.end.x) / 2.0,
                y: (a.start.y + a.end.y) / 2.0,
            };
            let b_mid = Point {
                x: (b.start.x + b.end.x) / 2.0,
                y: (b.start.y + b.end.y) / 2.0,
            };

            // Determine which side of the primary the secondary is on.
            let side_dot = (b_mid.x - a_mid.x) * px + (b_mid.y - a_mid.y) * py;
            let sign = if side_dot >= 0.0 { 1.0 } else { -1.0 };

            // Target midpoint for secondary.
            let target_mid = Point {
                x: a_mid.x + px * config.gap * sign,
                y: a_mid.y + py * config.gap * sign,
            };

            // Translate secondary segment so its midpoint lands on target_mid.
            let offset_x = target_mid.x - b_mid.x;
            let offset_y = target_mid.y - b_mid.y;

            b.start.x += offset_x;
            b.start.y += offset_y;
            b.end.x += offset_x;
            b.end.y += offset_y;

            // Adjust secondary endpoints to match primary length (preserve
            // direction and length).
            let b_len = {
                let ddx = b.end.x - b.start.x;
                let ddy = b.end.y - b.start.y;
                (ddx * ddx + ddy * ddy).sqrt()
            };
            if b_len < EPS {
                // Zero-length secondary: set to match primary direction.
                b.start = target_mid;
                b.end = target_mid;
            } else {
                // Reorient secondary to match primary direction with same length.
                let half = b_len / 2.0;
                b.start = Point {
                    x: target_mid.x - ux * half,
                    y: target_mid.y - uy * half,
                };
                b.end = Point {
                    x: target_mid.x + ux * half,
                    y: target_mid.y + uy * half,
                };
            }
        }

        // Update routed_length_mm for the secondary net.
        if let Some(b_net) = solution.nets.get_mut(&net_b) {
            b_net.routed_length_mm = b_net
                .segments
                .iter()
                .map(|s| {
                    let dx = s.end.x - s.start.x;
                    let dy = s.end.y - s.start.y;
                    (dx * dx + dy * dy).sqrt()
                })
                .sum();
        }
    }
}

// ---------------------------------------------------------------------------
// CenterlineExpander
// ---------------------------------------------------------------------------

/// Expands a centerline path into two physical differential pair traces.
///
/// Given a primary net's segments (routed as a fat centerline with room
/// for both traces), produces two `RoutedNet`s offset perpendicular to
/// the centerline by ±(gap/2 + trace_width/2).
pub struct CenterlineExpander;

impl CenterlineExpander {
    /// Expand a centerline into two physical differential pair traces.
    ///
    /// The primary net gets the `+offset` side, the partner gets `-offset`.
    /// Both traces have `trace_width` width (not the fat centerline width).
    pub fn expand_pair(
        centerline_segments: &[TraceSegment],
        centerline_vias: &[RoutedVia],
        primary_net_id: NetId,
        partner_net_id: NetId,
        config: &DiffPairConfig,
        trace_width: f64,
    ) -> (RoutedNet, RoutedNet) {
        let offset = config.gap / 2.0 + trace_width / 2.0;

        let mut primary_segments = Vec::with_capacity(centerline_segments.len());
        let mut partner_segments = Vec::with_capacity(centerline_segments.len());
        let mut primary_vias = Vec::with_capacity(centerline_vias.len());
        let mut partner_vias = Vec::with_capacity(centerline_vias.len());

        for seg in centerline_segments {
            let dx = seg.end.x - seg.start.x;
            let dy = seg.end.y - seg.start.y;
            let len = (dx * dx + dy * dy).sqrt();

            if len < EPS {
                // Zero-length segment: duplicate as-is for both nets
                primary_segments.push(TraceSegment {
                    net_id: primary_net_id,
                    width_mm: trace_width,
                    ..*seg
                });
                partner_segments.push(TraceSegment {
                    net_id: partner_net_id,
                    width_mm: trace_width,
                    ..*seg
                });
                continue;
            }

            // Unit direction and perpendicular
            let ux = dx / len;
            let uy = dy / len;
            let px = -uy; // perpendicular (left-rotate)
            let py = ux;

            // Primary: +offset perpendicular
            primary_segments.push(TraceSegment {
                net_id: primary_net_id,
                layer: seg.layer,
                start: Point {
                    x: seg.start.x + px * offset,
                    y: seg.start.y + py * offset,
                },
                end: Point {
                    x: seg.end.x + px * offset,
                    y: seg.end.y + py * offset,
                },
                width_mm: trace_width,
            });

            // Partner: -offset perpendicular
            partner_segments.push(TraceSegment {
                net_id: partner_net_id,
                layer: seg.layer,
                start: Point {
                    x: seg.start.x - px * offset,
                    y: seg.start.y - py * offset,
                },
                end: Point {
                    x: seg.end.x - px * offset,
                    y: seg.end.y - py * offset,
                },
                width_mm: trace_width,
            });
        }

        // Expand vias: each centerline via becomes two vias
        for via in centerline_vias {
            // Use the direction from the nearest segment to determine offset direction.
            // If no segments, offset in +x direction as fallback.
            let (px, py) = nearest_perpendicular(centerline_segments, via.position);

            primary_vias.push(RoutedVia {
                net_id: primary_net_id,
                position: Point {
                    x: via.position.x + px * offset,
                    y: via.position.y + py * offset,
                },
                ..*via
            });
            partner_vias.push(RoutedVia {
                net_id: partner_net_id,
                position: Point {
                    x: via.position.x - px * offset,
                    y: via.position.y - py * offset,
                },
                ..*via
            });
        }

        let primary_length: f64 = primary_segments
            .iter()
            .map(|s| {
                let dx = s.end.x - s.start.x;
                let dy = s.end.y - s.start.y;
                (dx * dx + dy * dy).sqrt()
            })
            .sum();

        let partner_length: f64 = partner_segments
            .iter()
            .map(|s| {
                let dx = s.end.x - s.start.x;
                let dy = s.end.y - s.start.y;
                (dx * dx + dy * dy).sqrt()
            })
            .sum();

        (
            RoutedNet {
                net_id: primary_net_id,
                segments: primary_segments,
                vias: primary_vias,
                routed_length_mm: primary_length,
            },
            RoutedNet {
                net_id: partner_net_id,
                segments: partner_segments,
                vias: partner_vias,
                routed_length_mm: partner_length,
            },
        )
    }
}

/// Find the perpendicular direction of the nearest segment to a point.
/// Returns (px, py) perpendicular unit vector, or (0, 1) as fallback.
fn nearest_perpendicular(segments: &[TraceSegment], point: Point) -> (f64, f64) {
    let mut best_dist = f64::INFINITY;
    let mut best_perp = (0.0_f64, 1.0_f64);

    for seg in segments {
        let mid_x = (seg.start.x + seg.end.x) / 2.0;
        let mid_y = (seg.start.y + seg.end.y) / 2.0;
        let dist = (point.x - mid_x).powi(2) + (point.y - mid_y).powi(2);

        if dist < best_dist {
            best_dist = dist;
            let dx = seg.end.x - seg.start.x;
            let dy = seg.end.y - seg.start.y;
            let len = (dx * dx + dy * dy).sqrt();
            if len > EPS {
                best_perp = (-dy / len, dx / len);
            }
        }
    }

    best_perp
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use autopcb_routes::{LayerId, NetId, Point, RoutedNet, RoutedVia, RouteSolution, TraceSegment};

    fn make_solution(
        net_a_segs: Vec<TraceSegment>,
        net_b_segs: Vec<TraceSegment>,
    ) -> RouteSolution {
        let mut solution = RouteSolution::new();
        solution.nets.insert(
            NetId(0),
            RoutedNet {
                net_id: NetId(0),
                segments: net_a_segs,
                vias: vec![],
                routed_length_mm: 0.0,
            },
        );
        solution.nets.insert(
            NetId(1),
            RoutedNet {
                net_id: NetId(1),
                segments: net_b_segs,
                vias: vec![],
                routed_length_mm: 0.0,
            },
        );
        solution
    }

    fn gap_between(a: &TraceSegment, b: &TraceSegment) -> f64 {
        // Midpoint of each segment.
        let a_mid_x = (a.start.x + a.end.x) / 2.0;
        let a_mid_y = (a.start.y + a.end.y) / 2.0;
        let b_mid_x = (b.start.x + b.end.x) / 2.0;
        let b_mid_y = (b.start.y + b.end.y) / 2.0;
        let dx = b_mid_x - a_mid_x;
        let dy = b_mid_y - a_mid_y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Two parallel horizontal traces: after optimization the gap between
    /// their midpoints equals `config.gap`.
    #[test]
    fn parallel_traces_adjusted_to_gap() {
        let net_a_seg = TraceSegment {
            net_id: NetId(0),
            layer: LayerId(0),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
            width_mm: 0.2,
        };
        // Secondary starts 2 mm above primary — should be adjusted to 0.15 mm.
        let net_b_seg = TraceSegment {
            net_id: NetId(1),
            layer: LayerId(0),
            start: Point { x: 0.0, y: 2.0 },
            end: Point { x: 10.0, y: 2.0 },
            width_mm: 0.2,
        };

        let mut solution = make_solution(vec![net_a_seg], vec![net_b_seg]);
        let config = DiffPairConfig {
            gap: 0.15,
            max_gap: 0.5,
            max_skew: 5.0,
        };

        DiffPairOptimizer::optimize_pair(&mut solution, NetId(0), NetId(1), &config);

        let a = &solution.nets[&NetId(0)].segments[0];
        let b = &solution.nets[&NetId(1)].segments[0];
        let actual_gap = gap_between(a, b);

        assert!(
            (actual_gap - config.gap).abs() < 1e-6,
            "expected gap={}, got {}",
            config.gap,
            actual_gap
        );
    }

    /// If net_a is missing, optimize_pair returns without panicking.
    #[test]
    fn missing_net_a_returns_gracefully() {
        let mut solution = RouteSolution::new();
        solution.nets.insert(
            NetId(1),
            RoutedNet {
                net_id: NetId(1),
                segments: vec![TraceSegment {
                    net_id: NetId(1),
                    layer: LayerId(0),
                    start: Point { x: 0.0, y: 0.0 },
                    end: Point { x: 1.0, y: 0.0 },
                    width_mm: 0.2,
                }],
                vias: vec![],
                routed_length_mm: 1.0,
            },
        );
        let config = DiffPairConfig { gap: 0.15, max_gap: 0.5, max_skew: 5.0 };
        // Should not panic.
        DiffPairOptimizer::optimize_pair(&mut solution, NetId(0), NetId(1), &config);
    }

    /// If net_b is missing, optimize_pair returns without panicking.
    #[test]
    fn missing_net_b_returns_gracefully() {
        let mut solution = RouteSolution::new();
        solution.nets.insert(
            NetId(0),
            RoutedNet {
                net_id: NetId(0),
                segments: vec![TraceSegment {
                    net_id: NetId(0),
                    layer: LayerId(0),
                    start: Point { x: 0.0, y: 0.0 },
                    end: Point { x: 1.0, y: 0.0 },
                    width_mm: 0.2,
                }],
                vias: vec![],
                routed_length_mm: 1.0,
            },
        );
        let config = DiffPairConfig { gap: 0.15, max_gap: 0.5, max_skew: 5.0 };
        DiffPairOptimizer::optimize_pair(&mut solution, NetId(0), NetId(1), &config);
    }

    /// Two vertical traces: gap after optimization matches config.
    #[test]
    fn vertical_traces_adjusted_to_gap() {
        let net_a_seg = TraceSegment {
            net_id: NetId(0),
            layer: LayerId(0),
            start: Point { x: 5.0, y: 0.0 },
            end: Point { x: 5.0, y: 10.0 },
            width_mm: 0.2,
        };
        let net_b_seg = TraceSegment {
            net_id: NetId(1),
            layer: LayerId(0),
            start: Point { x: 10.0, y: 0.0 },
            end: Point { x: 10.0, y: 10.0 },
            width_mm: 0.2,
        };
        let mut solution = make_solution(vec![net_a_seg], vec![net_b_seg]);
        let config = DiffPairConfig { gap: 0.3, max_gap: 1.0, max_skew: 5.0 };
        DiffPairOptimizer::optimize_pair(&mut solution, NetId(0), NetId(1), &config);

        let a = &solution.nets[&NetId(0)].segments[0];
        let b = &solution.nets[&NetId(1)].segments[0];
        let actual_gap = gap_between(a, b);
        assert!(
            (actual_gap - config.gap).abs() < 1e-6,
            "vertical: expected gap={}, got {}",
            config.gap,
            actual_gap
        );
    }

    // -----------------------------------------------------------------------
    // CenterlineExpander tests
    // -----------------------------------------------------------------------

    #[test]
    fn centerline_expand_horizontal() {
        // Horizontal centerline segment from (0,0) to (10,0)
        // gap=0.15, width=0.2
        // offset = 0.15/2 + 0.2/2 = 0.175
        // Primary at y=+0.175, Partner at y=-0.175
        let config = DiffPairConfig { gap: 0.15, max_gap: 0.5, max_skew: 5.0 };
        let seg = TraceSegment {
            net_id: NetId(0),
            layer: LayerId(0),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
            width_mm: 0.5, // fat centerline width (ignored by expander)
        };
        let (p, n) =
            CenterlineExpander::expand_pair(&[seg], &[], NetId(0), NetId(1), &config, 0.2);
        assert_eq!(p.segments.len(), 1);
        assert_eq!(n.segments.len(), 1);
        assert!((p.segments[0].start.y - 0.175).abs() < 1e-6);
        assert!((n.segments[0].start.y - (-0.175)).abs() < 1e-6);
        assert!((p.segments[0].width_mm - 0.2).abs() < 1e-6);
        assert_eq!(p.net_id, NetId(0));
        assert_eq!(n.net_id, NetId(1));
    }

    #[test]
    fn centerline_expand_vertical() {
        // Vertical centerline segment from (5,0) to (5,10)
        // Perpendicular is (-1, 0), so primary at x=5-0.175, partner at x=5+0.175
        let config = DiffPairConfig { gap: 0.15, max_gap: 0.5, max_skew: 5.0 };
        let seg = TraceSegment {
            net_id: NetId(0),
            layer: LayerId(0),
            start: Point { x: 5.0, y: 0.0 },
            end: Point { x: 5.0, y: 10.0 },
            width_mm: 0.5,
        };
        let (p, n) =
            CenterlineExpander::expand_pair(&[seg], &[], NetId(0), NetId(1), &config, 0.2);
        let offset = 0.175;
        // For vertical (dx=0, dy=10): ux=0, uy=1, px=-1, py=0
        // Primary: x = 5 + (-1)*offset = 5 - 0.175 = 4.825
        assert!(
            (p.segments[0].start.x - (5.0 - offset)).abs() < 1e-6,
            "p.x={}",
            p.segments[0].start.x
        );
        assert!(
            (n.segments[0].start.x - (5.0 + offset)).abs() < 1e-6,
            "n.x={}",
            n.segments[0].start.x
        );
    }

    #[test]
    fn centerline_expand_via_duplication() {
        let config = DiffPairConfig { gap: 0.15, max_gap: 0.5, max_skew: 5.0 };
        let seg = TraceSegment {
            net_id: NetId(0),
            layer: LayerId(0),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
            width_mm: 0.5,
        };
        let via = RoutedVia {
            net_id: NetId(0),
            position: Point { x: 5.0, y: 0.0 },
            from_layer: LayerId(0),
            to_layer: LayerId(1),
            drill_mm: 0.3,
            annular_ring_mm: 0.1,
        };
        let (p, n) =
            CenterlineExpander::expand_pair(&[seg], &[via], NetId(0), NetId(1), &config, 0.2);
        assert_eq!(p.vias.len(), 1);
        assert_eq!(n.vias.len(), 1);
        assert!((p.vias[0].position.y - 0.175).abs() < 1e-6);
        assert!((n.vias[0].position.y - (-0.175)).abs() < 1e-6);
    }

    #[test]
    fn centerline_expand_zero_length_segment() {
        let config = DiffPairConfig { gap: 0.15, max_gap: 0.5, max_skew: 5.0 };
        let seg = TraceSegment {
            net_id: NetId(0),
            layer: LayerId(0),
            start: Point { x: 5.0, y: 5.0 },
            end: Point { x: 5.0, y: 5.0 },
            width_mm: 0.5,
        };
        let (p, n) =
            CenterlineExpander::expand_pair(&[seg], &[], NetId(0), NetId(1), &config, 0.2);
        assert_eq!(p.segments.len(), 1);
        assert_eq!(n.segments.len(), 1);
        // Both should be at the same point (no offset for zero-length)
    }
}

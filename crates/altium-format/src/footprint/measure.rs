//! Footprint measurement utilities.
//!
//! Provides functions for measuring distances and clearances between PCB
//! footprint features. Uses the `geo` crate for robust 2D geometry calculations.
//!
//! These measurements are designed to help LLM agents verify the correctness
//! of generated PCB footprints by checking dimensions against datasheets.
//!
//! # V2 Migration Note
//! This module uses v1 types (PcbComponent, PcbPad, PcbRecord).
//! TODO: Migrate to v2 PCB types when fully available.

#![allow(deprecated)]

use geo::{Coord as GeoCoord, EuclideanDistance, Line, Point, Polygon, Rect, point};

use crate::records::pcb::{PcbComponent, PcbPad, PcbPadShape, PcbRecord};
use crate::types::{Coord, CoordPoint};

/// Measurement result with value and unit.
#[derive(Debug, Clone)]
pub struct Measurement {
    /// Value in millimeters.
    pub mm: f64,
    /// Value in mils.
    pub mils: f64,
}

impl Measurement {
    /// Create a measurement from internal coordinate units.
    pub fn from_coord(coord: Coord) -> Self {
        Self {
            mm: coord.to_mms(),
            mils: coord.to_mils(),
        }
    }

    /// Create a measurement from millimeters.
    pub fn from_mm(mm: f64) -> Self {
        Self {
            mm,
            mils: mm / 0.0254,
        }
    }

    /// Format for display.
    pub fn display(&self) -> String {
        format!("{:.3}mm ({:.1}mil)", self.mm, self.mils)
    }
}

/// Result of measuring distance between two pads.
#[derive(Debug, Clone)]
pub struct PadDistance {
    /// First pad designator.
    pub pad1: String,
    /// Second pad designator.
    pub pad2: String,
    /// Center-to-center distance.
    pub center_to_center: Measurement,
    /// Edge-to-edge distance (gap between pads).
    pub edge_to_edge: Measurement,
}

/// Result of pad pitch analysis.
#[derive(Debug, Clone)]
pub struct PitchAnalysis {
    /// Detected pitch value (most common spacing).
    pub pitch: Measurement,
    /// Number of pad pairs with this pitch.
    pub count: usize,
    /// Direction: "horizontal", "vertical", or "diagonal".
    pub direction: String,
    /// Individual pad-to-pad distances in sequence.
    pub pad_pairs: Vec<(String, String, Measurement)>,
}

/// Overall footprint dimensions.
#[derive(Debug, Clone)]
pub struct FootprintDimensions {
    /// Total width (x-axis span).
    pub width: Measurement,
    /// Total height (y-axis span).
    pub height: Measurement,
    /// Minimum X coordinate.
    pub min_x: Measurement,
    /// Maximum X coordinate.
    pub max_x: Measurement,
    /// Minimum Y coordinate.
    pub min_y: Measurement,
    /// Maximum Y coordinate.
    pub max_y: Measurement,
}

/// Pad dimensions and position.
#[derive(Debug, Clone)]
pub struct PadInfo {
    /// Pad designator.
    pub designator: String,
    /// X position (center).
    pub x: Measurement,
    /// Y position (center).
    pub y: Measurement,
    /// Pad width.
    pub width: Measurement,
    /// Pad height.
    pub height: Measurement,
    /// Hole diameter (0 for SMD).
    pub hole: Option<Measurement>,
    /// Shape name.
    pub shape: String,
}

/// Clearance measurement result.
#[derive(Debug, Clone)]
pub struct ClearanceResult {
    /// Feature type 1.
    pub feature1: String,
    /// Feature type 2.
    pub feature2: String,
    /// Minimum clearance found.
    pub clearance: Measurement,
    /// Location description.
    pub location: String,
}

/// Convert internal CoordPoint to geo Point.
fn to_geo_point(p: CoordPoint) -> Point<f64> {
    point!(x: p.x.to_mms(), y: p.y.to_mms())
}

/// Get pad center as geo Point.
fn pad_center(pad: &PcbPad) -> Point<f64> {
    to_geo_point(pad.location)
}

/// Get pad bounds as a geo Rect.
#[allow(dead_code)] // Reserved for future pad collision detection
fn pad_rect(pad: &PcbPad) -> Rect<f64> {
    let size = pad.size_top();
    let half_w = size.x.to_mms() / 2.0;
    let half_h = size.y.to_mms() / 2.0;
    let cx = pad.location.x.to_mms();
    let cy = pad.location.y.to_mms();

    Rect::new(
        GeoCoord {
            x: cx - half_w,
            y: cy - half_h,
        },
        GeoCoord {
            x: cx + half_w,
            y: cy + half_h,
        },
    )
}

/// Get pad outline as a polygon (for rotated pads).
#[allow(dead_code)] // Reserved for future rotated pad collision detection
fn pad_polygon(pad: &PcbPad) -> Polygon<f64> {
    let size = pad.size_top();
    let half_w = size.x.to_mms() / 2.0;
    let half_h = size.y.to_mms() / 2.0;
    let cx = pad.location.x.to_mms();
    let cy = pad.location.y.to_mms();

    // Treats pad as axis-aligned rectangle; rotation not applied to bounds calculation
    let corners = vec![
        (cx - half_w, cy - half_h),
        (cx + half_w, cy - half_h),
        (cx + half_w, cy + half_h),
        (cx - half_w, cy + half_h),
        (cx - half_w, cy - half_h), // Close polygon
    ];

    Polygon::new(geo::LineString::from(corners), vec![])
}

/// Calculate minimum distance between two pad edges.
fn pad_edge_distance(pad1: &PcbPad, pad2: &PcbPad) -> f64 {
    let center_dist = pad_center(pad1).euclidean_distance(&pad_center(pad2));

    // Calculate effective radii along the axis connecting the pads
    let dx = pad2.location.x.to_mms() - pad1.location.x.to_mms();
    let dy = pad2.location.y.to_mms() - pad1.location.y.to_mms();

    if center_dist < 1e-9 {
        return 0.0; // Same location
    }

    // Normalize direction
    let nx = dx / center_dist;
    let ny = dy / center_dist;

    // Calculate extent of each pad along the line connecting centers
    let size1 = pad1.size_top();
    let size2 = pad2.size_top();

    let extent1 = effective_extent(size1.x.to_mms(), size1.y.to_mms(), nx, ny, pad1.shape_top());
    let extent2 = effective_extent(size2.x.to_mms(), size2.y.to_mms(), nx, ny, pad2.shape_top());

    (center_dist - extent1 - extent2).max(0.0)
}

/// Calculate the extent of a pad shape along a given direction.
fn effective_extent(width: f64, height: f64, nx: f64, ny: f64, shape: PcbPadShape) -> f64 {
    match shape {
        PcbPadShape::Round | PcbPadShape::Circle | PcbPadShape::NoShape => {
            // For round pads, use radius
            width.min(height) / 2.0
        }
        PcbPadShape::Rectangular
        | PcbPadShape::RoundedRectangle
        | PcbPadShape::Octagonal
        | PcbPadShape::RoundRect
        | PcbPadShape::RotatedRect
        | PcbPadShape::Arc
        | PcbPadShape::Terminator => {
            // For rectangular shapes, calculate intersection with rectangle edge
            let half_w = width / 2.0;
            let half_h = height / 2.0;

            // Extent along the direction (nx, ny)
            let abs_nx = nx.abs();
            let abs_ny = ny.abs();

            if abs_nx < 1e-9 {
                half_h
            } else if abs_ny < 1e-9 {
                half_w
            } else {
                // Calculate where ray intersects rectangle
                let t_x = half_w / abs_nx;
                let t_y = half_h / abs_ny;
                t_x.min(t_y) * (abs_nx * abs_nx + abs_ny * abs_ny).sqrt()
            }
        }
    }
}

/// Measure distance between two specific pads.
pub fn measure_pad_distance(
    component: &PcbComponent,
    pad1_des: &str,
    pad2_des: &str,
) -> Option<PadDistance> {
    let pad1 = component.pads().find(|p| p.designator == pad1_des)?;
    let pad2 = component.pads().find(|p| p.designator == pad2_des)?;

    let center_dist = pad_center(pad1).euclidean_distance(&pad_center(pad2));
    let edge_dist = pad_edge_distance(pad1, pad2);

    Some(PadDistance {
        pad1: pad1_des.to_string(),
        pad2: pad2_des.to_string(),
        center_to_center: Measurement::from_mm(center_dist),
        edge_to_edge: Measurement::from_mm(edge_dist),
    })
}

/// Analyze pad pitch (spacing between adjacent pads).
pub fn analyze_pitch(component: &PcbComponent) -> Vec<PitchAnalysis> {
    let pads: Vec<&PcbPad> = component.pads().collect();
    if pads.len() < 2 {
        return vec![];
    }

    let mut horizontal_pitches: Vec<(String, String, f64)> = vec![];
    let mut vertical_pitches: Vec<(String, String, f64)> = vec![];

    // Group pads by approximate Y coordinate (for horizontal pitch)
    // and by approximate X coordinate (for vertical pitch)
    let tolerance = 0.1; // mm

    for i in 0..pads.len() {
        for j in (i + 1)..pads.len() {
            let p1 = pads[i];
            let p2 = pads[j];

            let dx = (p2.location.x.to_mms() - p1.location.x.to_mms()).abs();
            let dy = (p2.location.y.to_mms() - p1.location.y.to_mms()).abs();

            // Check if pads are aligned horizontally (same Y)
            if dy < tolerance && dx > tolerance {
                horizontal_pitches.push((p1.designator.clone(), p2.designator.clone(), dx));
            }

            // Check if pads are aligned vertically (same X)
            if dx < tolerance && dy > tolerance {
                vertical_pitches.push((p1.designator.clone(), p2.designator.clone(), dy));
            }
        }
    }

    let mut results = vec![];

    // Analyze horizontal pitch
    if !horizontal_pitches.is_empty() {
        if let Some(analysis) = analyze_pitch_group(horizontal_pitches, "horizontal") {
            results.push(analysis);
        }
    }

    // Analyze vertical pitch
    if !vertical_pitches.is_empty() {
        if let Some(analysis) = analyze_pitch_group(vertical_pitches, "vertical") {
            results.push(analysis);
        }
    }

    results
}

/// Analyze a group of pitch measurements to find the most common pitch.
fn analyze_pitch_group(
    mut pitches: Vec<(String, String, f64)>,
    direction: &str,
) -> Option<PitchAnalysis> {
    if pitches.is_empty() {
        return None;
    }

    // Sort by distance
    pitches.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());

    // Find the minimum pitch (adjacent pads)
    let min_pitch = pitches[0].2;

    // Filter to only adjacent pads (those with approximately minimum pitch)
    let tolerance = min_pitch * 0.1; // 10% tolerance
    let adjacent: Vec<_> = pitches
        .into_iter()
        .filter(|(_, _, d)| (*d - min_pitch).abs() < tolerance)
        .collect();

    if adjacent.is_empty() {
        return None;
    }

    let avg_pitch = adjacent.iter().map(|(_, _, d)| d).sum::<f64>() / adjacent.len() as f64;

    Some(PitchAnalysis {
        pitch: Measurement::from_mm(avg_pitch),
        count: adjacent.len(),
        direction: direction.to_string(),
        pad_pairs: adjacent
            .into_iter()
            .map(|(p1, p2, d)| (p1, p2, Measurement::from_mm(d)))
            .collect(),
    })
}

/// Get overall footprint dimensions.
pub fn measure_dimensions(component: &PcbComponent) -> FootprintDimensions {
    let bounds = component.calculate_bounds();

    FootprintDimensions {
        width: Measurement::from_coord(bounds.width()),
        height: Measurement::from_coord(bounds.height()),
        min_x: Measurement::from_coord(bounds.location1.x),
        max_x: Measurement::from_coord(bounds.location2.x),
        min_y: Measurement::from_coord(bounds.location1.y),
        max_y: Measurement::from_coord(bounds.location2.y),
    }
}

/// Get detailed information about a specific pad.
pub fn measure_pad(component: &PcbComponent, designator: &str) -> Option<PadInfo> {
    let pad = component.pads().find(|p| p.designator == designator)?;

    let size = pad.size_top();
    let shape_name = match pad.shape_top() {
        PcbPadShape::NoShape => "None",
        PcbPadShape::Round => "Round",
        PcbPadShape::Rectangular => "Rectangular",
        PcbPadShape::Octagonal => "Octagonal",
        PcbPadShape::Circle => "Circle",
        PcbPadShape::Arc => "Arc",
        PcbPadShape::Terminator => "Terminator",
        PcbPadShape::RoundRect => "Round Rect",
        PcbPadShape::RotatedRect => "Rotated Rect",
        PcbPadShape::RoundedRectangle => "Rounded Rectangle",
    };

    Some(PadInfo {
        designator: designator.to_string(),
        x: Measurement::from_coord(pad.location.x),
        y: Measurement::from_coord(pad.location.y),
        width: Measurement::from_coord(size.x),
        height: Measurement::from_coord(size.y),
        hole: if pad.has_hole() {
            Some(Measurement::from_coord(pad.hole_size))
        } else {
            None
        },
        shape: shape_name.to_string(),
    })
}

/// Measure all pads and return their info.
pub fn measure_all_pads(component: &PcbComponent) -> Vec<PadInfo> {
    component
        .pads()
        .map(|pad| {
            let size = pad.size_top();
            let shape_name = match pad.shape_top() {
                PcbPadShape::NoShape => "None",
                PcbPadShape::Round => "Round",
                PcbPadShape::Rectangular => "Rectangular",
                PcbPadShape::Octagonal => "Octagonal",
                PcbPadShape::Circle => "Circle",
                PcbPadShape::Arc => "Arc",
                PcbPadShape::Terminator => "Terminator",
                PcbPadShape::RoundRect => "Round Rect",
                PcbPadShape::RotatedRect => "Rotated Rect",
                PcbPadShape::RoundedRectangle => "Rounded Rectangle",
            };

            PadInfo {
                designator: pad.designator.clone(),
                x: Measurement::from_coord(pad.location.x),
                y: Measurement::from_coord(pad.location.y),
                width: Measurement::from_coord(size.x),
                height: Measurement::from_coord(size.y),
                hole: if pad.has_hole() {
                    Some(Measurement::from_coord(pad.hole_size))
                } else {
                    None
                },
                shape: shape_name.to_string(),
            }
        })
        .collect()
}

/// Find minimum pad-to-pad clearance (edge-to-edge gap).
pub fn minimum_pad_clearance(component: &PcbComponent) -> Option<ClearanceResult> {
    let pads: Vec<&PcbPad> = component.pads().collect();
    if pads.len() < 2 {
        return None;
    }

    let mut min_clearance = f64::MAX;
    let mut min_pair = ("", "");

    for i in 0..pads.len() {
        for j in (i + 1)..pads.len() {
            let dist = pad_edge_distance(pads[i], pads[j]);
            if dist < min_clearance {
                min_clearance = dist;
                min_pair = (&pads[i].designator, &pads[j].designator);
            }
        }
    }

    if min_clearance == f64::MAX {
        return None;
    }

    Some(ClearanceResult {
        feature1: format!("Pad {}", min_pair.0),
        feature2: format!("Pad {}", min_pair.1),
        clearance: Measurement::from_mm(min_clearance),
        location: format!("Between pads {} and {}", min_pair.0, min_pair.1),
    })
}

/// Measure pad-to-silkscreen clearance.
pub fn pad_to_silkscreen_clearance(component: &PcbComponent) -> Option<ClearanceResult> {
    let pads: Vec<&PcbPad> = component.pads().collect();

    // Collect silkscreen tracks
    let tracks: Vec<_> = component
        .primitives
        .iter()
        .filter_map(|p| {
            if let PcbRecord::Track(t) = p {
                if t.common.layer.is_overlay() {
                    return Some(t);
                }
            }
            None
        })
        .collect();

    // Collect silkscreen arcs
    let arcs: Vec<_> = component
        .primitives
        .iter()
        .filter_map(|p| {
            if let PcbRecord::Arc(a) = p {
                if a.common.layer.is_overlay() {
                    return Some(a);
                }
            }
            None
        })
        .collect();

    if pads.is_empty() || (tracks.is_empty() && arcs.is_empty()) {
        return None;
    }

    let mut min_clearance = f64::MAX;
    let mut min_pad = "";
    let mut min_type = "";

    // Check pad-to-track clearance
    for pad in &pads {
        let pad_center = pad_center(pad);
        let size = pad.size_top();
        let pad_radius = (size.x.to_mms().max(size.y.to_mms())) / 2.0;

        for track in &tracks {
            let start = to_geo_point(track.start);
            let end = to_geo_point(track.end);
            let line = Line::new(start.0, end.0);
            let line_width = track.width.to_mms() / 2.0;

            // Distance from pad center to line
            let dist = pad_center.euclidean_distance(&line);
            let clearance = dist - pad_radius - line_width;

            if clearance < min_clearance {
                min_clearance = clearance;
                min_pad = &pad.designator;
                min_type = "track";
            }
        }

        for arc in &arcs {
            // Distance from pad center to arc center
            let arc_center = to_geo_point(arc.location);
            let dist_to_center = pad_center.euclidean_distance(&arc_center);
            let arc_radius = arc.radius.to_mms();
            let arc_width = arc.width.to_mms() / 2.0;

            // Distance to arc is distance to center minus radius
            let dist_to_arc = (dist_to_center - arc_radius).abs();
            let clearance = dist_to_arc - pad_radius - arc_width;

            if clearance < min_clearance {
                min_clearance = clearance;
                min_pad = &pad.designator;
                min_type = "arc";
            }
        }
    }

    if min_clearance == f64::MAX {
        return None;
    }

    Some(ClearanceResult {
        feature1: format!("Pad {}", min_pad),
        feature2: format!("Silkscreen {}", min_type),
        clearance: Measurement::from_mm(min_clearance.max(0.0)),
        location: format!("Pad {} to silkscreen", min_pad),
    })
}

/// Measure span between two pad rows (e.g., for IC packages).
/// Returns the distance between pad centers on opposite sides.
pub fn measure_row_span(component: &PcbComponent) -> Option<Measurement> {
    let pads: Vec<&PcbPad> = component.pads().collect();
    if pads.len() < 2 {
        return None;
    }

    // Find min and max Y coordinates (for horizontal IC)
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;

    for pad in &pads {
        let y = pad.location.y.to_mms();
        if y < min_y {
            min_y = y;
        }
        if y > max_y {
            max_y = y;
        }
    }

    let y_span = max_y - min_y;

    // Find min and max X coordinates (for vertical IC)
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;

    for pad in &pads {
        let x = pad.location.x.to_mms();
        if x < min_x {
            min_x = x;
        }
        if x > max_x {
            max_x = x;
        }
    }

    let x_span = max_x - min_x;

    // Return the smaller span (distance between rows)
    if y_span < x_span && y_span > 0.1 {
        Some(Measurement::from_mm(y_span))
    } else if x_span > 0.1 {
        Some(Measurement::from_mm(x_span))
    } else {
        None
    }
}

/// Comprehensive measurement report for a footprint.
#[derive(Debug)]
pub struct MeasurementReport {
    /// Footprint name.
    pub name: String,
    /// Overall dimensions.
    pub dimensions: FootprintDimensions,
    /// All pad info.
    pub pads: Vec<PadInfo>,
    /// Pitch analysis results.
    pub pitch: Vec<PitchAnalysis>,
    /// Minimum pad clearance.
    pub min_pad_clearance: Option<ClearanceResult>,
    /// Pad-to-silkscreen clearance.
    pub silkscreen_clearance: Option<ClearanceResult>,
    /// Row span (for dual-row packages).
    pub row_span: Option<Measurement>,
}

/// Generate a comprehensive measurement report for a footprint.
pub fn generate_report(component: &PcbComponent) -> MeasurementReport {
    MeasurementReport {
        name: component.pattern.clone(),
        dimensions: measure_dimensions(component),
        pads: measure_all_pads(component),
        pitch: analyze_pitch(component),
        min_pad_clearance: minimum_pad_clearance(component),
        silkscreen_clearance: pad_to_silkscreen_clearance(component),
        row_span: measure_row_span(component),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_measurement_display() {
        let m = Measurement::from_mm(1.27);
        assert!(m.display().contains("1.27"));
        assert!(m.display().contains("mil"));
    }
}

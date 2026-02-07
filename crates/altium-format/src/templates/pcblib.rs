//! PcbLib footprint template for creating PCB footprints.
//!
//! The [`PcbFootprintTemplate`] supports creating footprints from:
//! - Explicit pad positions
//! - Pad array patterns (rows, dual rows, quad, grid)
//! - Silkscreen outlines
//! - Courtyard and assembly layers

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::MmInput;
use crate::error::Result;
use crate::footprint::{FootprintBuilder, PadRowDirection};
use crate::records::pcb::{PcbComponent, PcbPadShape};

// ═══════════════════════════════════════════════════════════════════════════
// Template Input Types
// ═══════════════════════════════════════════════════════════════════════════

/// Template for creating a PCB footprint.
///
/// All dimensions are in millimeters by default (standard for PCB work).
///
/// # Pad creation modes
///
/// Pads can be created in multiple ways (combined freely):
///
/// 1. **Explicit pads**: Individual pad positions via `pads` array
/// 2. **Pad rows**: Linear arrangements via `pad_rows` array
/// 3. **Dual rows**: SOIC/DIP-style via `dual_rows` array
/// 4. **Quad arrangement**: QFP-style via `quad_pads` array
/// 5. **Grid**: BGA/LGA-style via `pad_grids` array
///
/// # Example (SOIC-8)
/// ```json
/// {
///   "name": "SOIC-8",
///   "description": "8-pin SOIC package",
///   "dual_rows": [{
///     "pads_per_side": 4,
///     "pitch": 1.27,
///     "row_spacing": 5.3,
///     "pad_width": 1.5,
///     "pad_height": 0.6
///   }],
///   "silkscreen": { "width": 3.9, "height": 4.9 }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PcbFootprintTemplate {
    /// Footprint pattern name. Required.
    pub name: String,

    /// Footprint description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Component height for 3D (mm).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<MmInput>,

    /// Explicit individual pad definitions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pads: Vec<PadTemplate>,

    /// Pad row patterns (linear arrangements).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pad_rows: Vec<PadRowTemplate>,

    /// Dual-row patterns (SOIC, DIP, SOP, TSSOP).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dual_rows: Vec<DualRowTemplate>,

    /// Quad-pad patterns (QFP, LQFP, TQFP).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub quad_pads: Vec<QuadPadTemplate>,

    /// Grid patterns (BGA, LGA).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pad_grids: Vec<PadGridTemplate>,

    /// Silkscreen rectangle outline (auto-generated from body dimensions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub silkscreen: Option<SilkscreenTemplate>,

    /// Explicit silkscreen lines.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub silkscreen_lines: Vec<SilkLineTemplate>,

    /// Silkscreen arcs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub silkscreen_arcs: Vec<SilkArcTemplate>,

    /// Courtyard rectangle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub courtyard: Option<CourtyardTemplate>,

    /// Whether to add a pin 1 indicator dot. Default: true when pads are present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin1_indicator: Option<bool>,
}

impl Default for PcbFootprintTemplate {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            height: None,
            pads: Vec::new(),
            pad_rows: Vec::new(),
            dual_rows: Vec::new(),
            quad_pads: Vec::new(),
            pad_grids: Vec::new(),
            silkscreen: None,
            silkscreen_lines: Vec::new(),
            silkscreen_arcs: Vec::new(),
            courtyard: None,
            pin1_indicator: None,
        }
    }
}

/// Template for a single pad.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PadTemplate {
    /// Pad designator (e.g., "1", "A1"). Auto-numbered if omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub designator: Option<String>,

    /// X position (mm).
    pub x: MmInput,
    /// Y position (mm).
    pub y: MmInput,

    /// Pad width (mm). For through-hole, this is the pad diameter.
    pub width: MmInput,
    /// Pad height (mm). For through-hole round pads, same as width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<MmInput>,

    /// Pad shape. Default: "rectangular" for SMD, "round" for through-hole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,

    /// Hole diameter (mm). If present, creates a through-hole pad.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hole: Option<MmInput>,
}

/// Template for a row of pads.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PadRowTemplate {
    /// Number of pads.
    pub count: usize,
    /// Pitch: center-to-center distance (mm).
    pub pitch: MmInput,
    /// Pad width (mm).
    pub pad_width: MmInput,
    /// Pad height (mm).
    pub pad_height: MmInput,
    /// Start X position (mm). Default: 0.
    #[serde(default)]
    pub start_x: MmInput,
    /// Start Y position (mm). Default: 0.
    #[serde(default)]
    pub start_y: MmInput,
    /// Direction: "horizontal" or "vertical". Default: "horizontal".
    #[serde(default = "default_direction")]
    pub direction: String,
    /// Starting designator number. Default: 1.
    #[serde(default = "default_start_designator")]
    pub start_designator: u32,
    /// Pad shape: "rectangular", "round", "octagonal", "roundrect". Default: "rectangular".
    #[serde(default = "default_pad_shape")]
    pub shape: String,
    /// Hole diameter (mm) for through-hole pads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hole: Option<MmInput>,
}

/// Template for dual-row pads (SOIC, DIP, SOP, TSSOP).
///
/// Creates two parallel rows of pads, numbered sequentially down one side
/// then up the other (standard IC numbering).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DualRowTemplate {
    /// Number of pads per side.
    pub pads_per_side: usize,
    /// Pitch: center-to-center distance between adjacent pads (mm).
    pub pitch: MmInput,
    /// Row spacing: distance between row centers (mm).
    pub row_spacing: MmInput,
    /// Pad width: perpendicular to package body (mm).
    pub pad_width: MmInput,
    /// Pad height: along package body (mm).
    pub pad_height: MmInput,
    /// Pad shape. Default: "rectangular".
    #[serde(default = "default_pad_shape")]
    pub shape: String,
    /// Hole diameter (mm) for through-hole pads (e.g., DIP).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hole: Option<MmInput>,
}

/// Template for quad-pad arrangement (QFP, LQFP, TQFP).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QuadPadTemplate {
    /// Number of pads per side.
    pub pads_per_side: usize,
    /// Pitch (mm).
    pub pitch: MmInput,
    /// Span: distance between opposite row centers (mm).
    pub span: MmInput,
    /// Pad width: perpendicular to body edge (mm).
    pub pad_width: MmInput,
    /// Pad height: along body edge (mm).
    pub pad_height: MmInput,
    /// Pad shape. Default: "rectangular".
    #[serde(default = "default_pad_shape")]
    pub shape: String,
}

/// Template for a grid of pads (BGA, LGA).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PadGridTemplate {
    /// Number of rows (A, B, C, ...).
    pub rows: usize,
    /// Number of columns (1, 2, 3, ...).
    pub cols: usize,
    /// Pitch (mm), same for X and Y.
    pub pitch: MmInput,
    /// Pad diameter (mm).
    pub pad_diameter: MmInput,
    /// Pad shape. Default: "round".
    #[serde(default = "default_grid_shape")]
    pub shape: String,
    /// Skip pads within this radius (mm) from center (for thermal pad). Default: 0.
    #[serde(default)]
    pub skip_center: MmInput,
}

/// Template for silkscreen rectangle outline.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SilkscreenTemplate {
    /// Body width (mm).
    pub width: MmInput,
    /// Body height (mm).
    pub height: MmInput,
    /// Center X (mm). Default: 0.
    #[serde(default)]
    pub x: MmInput,
    /// Center Y (mm). Default: 0.
    #[serde(default)]
    pub y: MmInput,
    /// Line width (mm). Default: 0.2.
    #[serde(default = "default_silk_width")]
    pub line_width: MmInput,
}

/// Template for a silkscreen line.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SilkLineTemplate {
    /// Start X (mm).
    pub x1: MmInput,
    /// Start Y (mm).
    pub y1: MmInput,
    /// End X (mm).
    pub x2: MmInput,
    /// End Y (mm).
    pub y2: MmInput,
    /// Line width (mm). Default: 0.2.
    #[serde(default = "default_silk_width")]
    pub width: MmInput,
}

/// Template for a silkscreen arc.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SilkArcTemplate {
    /// Center X (mm).
    pub x: MmInput,
    /// Center Y (mm).
    pub y: MmInput,
    /// Radius (mm).
    pub radius: MmInput,
    /// Start angle in degrees (0 = right, 90 = up).
    #[serde(default)]
    pub start_angle: f64,
    /// End angle in degrees. Default: 360.
    #[serde(default = "default_end_angle")]
    pub end_angle: f64,
    /// Line width (mm). Default: 0.2.
    #[serde(default = "default_silk_width")]
    pub width: MmInput,
}

/// Template for courtyard rectangle.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CourtyardTemplate {
    /// Width (mm).
    pub width: MmInput,
    /// Height (mm).
    pub height: MmInput,
    /// Center X (mm). Default: 0.
    #[serde(default)]
    pub x: MmInput,
    /// Center Y (mm). Default: 0.
    #[serde(default)]
    pub y: MmInput,
    /// Line width (mm). Default: 0.05.
    #[serde(default = "default_courtyard_width")]
    pub line_width: MmInput,
}

fn default_direction() -> String {
    "horizontal".to_string()
}
fn default_start_designator() -> u32 {
    1
}
fn default_pad_shape() -> String {
    "rectangular".to_string()
}
fn default_grid_shape() -> String {
    "round".to_string()
}
fn default_silk_width() -> MmInput {
    MmInput(0.2)
}
fn default_end_angle() -> f64 {
    360.0
}
fn default_courtyard_width() -> MmInput {
    MmInput(0.05)
}

// ═══════════════════════════════════════════════════════════════════════════
// Template Application
// ═══════════════════════════════════════════════════════════════════════════

impl PcbFootprintTemplate {
    /// Compute the approximate position of pad 1 from template inputs.
    ///
    /// Returns `(x_mm, y_mm)` for the first pad, or `None` if no pads are defined.
    fn first_pad_position(&self) -> Option<(f64, f64)> {
        // Check explicit pads first
        if let Some(pad) = self.pads.first() {
            return Some((pad.x.to_mm(), pad.y.to_mm()));
        }
        // Dual rows: pad 1 is at left side, bottom of left column
        if let Some(dual) = self.dual_rows.first() {
            let half_span = dual.row_spacing.to_mm() / 2.0;
            let row_length = (dual.pads_per_side - 1) as f64 * dual.pitch.to_mm();
            return Some((-half_span, -row_length / 2.0));
        }
        // Pad rows: pad 1 is at the start position
        if let Some(row) = self.pad_rows.first() {
            return Some((row.start_x.to_mm(), row.start_y.to_mm()));
        }
        // Quad pads: pad 1 is at the bottom of the left side
        if let Some(quad) = self.quad_pads.first() {
            let half_span = quad.span.to_mm() / 2.0;
            let row_length = (quad.pads_per_side - 1) as f64 * quad.pitch.to_mm();
            return Some((-half_span, -row_length / 2.0));
        }
        // Grid pads: pad A1 is at top-left
        if let Some(grid) = self.pad_grids.first() {
            let grid_width = (grid.cols - 1) as f64 * grid.pitch.to_mm();
            let grid_height = (grid.rows - 1) as f64 * grid.pitch.to_mm();
            return Some((-grid_width / 2.0, grid_height / 2.0));
        }
        None
    }

    /// Apply this template to produce a `PcbComponent`.
    pub fn apply(&self) -> Result<PcbComponent> {
        let mut builder = FootprintBuilder::new(&self.name);

        if let Some(ref desc) = self.description {
            builder = builder.description(desc.clone());
        }
        if let Some(ref h) = self.height {
            builder = builder.height_mm(h.to_mm());
        }

        // Track whether we added any pads (for pin1 indicator)
        let mut has_pads = false;

        // Explicit pads
        let mut auto_num = 1u32;
        for pad in &self.pads {
            let designator = pad
                .designator
                .clone()
                .unwrap_or_else(|| {
                    let d = auto_num.to_string();
                    auto_num += 1;
                    d
                });
            let width = pad.width.to_mm();
            let height = pad.height.as_ref().map(|h| h.to_mm()).unwrap_or(width);
            let shape = pad
                .shape
                .as_deref()
                .map(parse_pad_shape)
                .unwrap_or_else(|| {
                    if pad.hole.is_some() {
                        PcbPadShape::Round
                    } else {
                        PcbPadShape::Rectangular
                    }
                });

            if let Some(ref hole) = pad.hole {
                builder.add_th_pad(
                    &designator,
                    pad.x.to_mm(),
                    pad.y.to_mm(),
                    width,
                    hole.to_mm(),
                    shape,
                );
            } else {
                builder.add_smd_pad(
                    &designator,
                    pad.x.to_mm(),
                    pad.y.to_mm(),
                    width,
                    height,
                    shape,
                );
            }
            has_pads = true;
        }

        // Pad rows
        for row in &self.pad_rows {
            let shape = parse_pad_shape(&row.shape);
            let direction = PadRowDirection::try_parse(&row.direction)
                .unwrap_or(PadRowDirection::Horizontal);

            if let Some(ref hole) = row.hole {
                builder.add_th_pad_row(
                    row.count,
                    row.pitch.to_mm(),
                    row.pad_width.to_mm(),
                    hole.to_mm(),
                    row.start_x.to_mm(),
                    row.start_y.to_mm(),
                    direction,
                    row.start_designator,
                    shape,
                );
            } else {
                builder.add_pad_row(
                    row.count,
                    row.pitch.to_mm(),
                    row.pad_width.to_mm(),
                    row.pad_height.to_mm(),
                    row.start_x.to_mm(),
                    row.start_y.to_mm(),
                    direction,
                    row.start_designator,
                    shape,
                );
            }
            has_pads = true;
        }

        // Dual rows
        for dual in &self.dual_rows {
            let shape = parse_pad_shape(&dual.shape);
            if let Some(ref hole) = dual.hole {
                builder.add_dual_row_th(
                    dual.pads_per_side,
                    dual.pitch.to_mm(),
                    dual.row_spacing.to_mm(),
                    dual.pad_width.to_mm(),
                    hole.to_mm(),
                    shape,
                );
            } else {
                builder.add_dual_row_smd(
                    dual.pads_per_side,
                    dual.pitch.to_mm(),
                    dual.row_spacing.to_mm(),
                    dual.pad_width.to_mm(),
                    dual.pad_height.to_mm(),
                    shape,
                );
            }
            has_pads = true;
        }

        // Quad pads
        for quad in &self.quad_pads {
            let shape = parse_pad_shape(&quad.shape);
            builder.add_quad_pads_smd(
                quad.pads_per_side,
                quad.pitch.to_mm(),
                quad.span.to_mm(),
                quad.pad_width.to_mm(),
                quad.pad_height.to_mm(),
                shape,
            );
            has_pads = true;
        }

        // Pad grids
        for grid in &self.pad_grids {
            let shape = parse_pad_shape(&grid.shape);
            builder.add_pad_grid(
                grid.rows,
                grid.cols,
                grid.pitch.to_mm(),
                grid.pad_diameter.to_mm(),
                shape,
                grid.skip_center.to_mm(),
            );
            has_pads = true;
        }

        // Silkscreen rectangle
        if let Some(ref silk) = self.silkscreen {
            builder.add_silkscreen_rect(
                silk.x.to_mm(),
                silk.y.to_mm(),
                silk.width.to_mm(),
                silk.height.to_mm(),
                silk.line_width.to_mm(),
            );
        }

        // Silkscreen lines
        for line in &self.silkscreen_lines {
            builder.add_silkscreen_line(
                line.x1.to_mm(),
                line.y1.to_mm(),
                line.x2.to_mm(),
                line.y2.to_mm(),
                line.width.to_mm(),
            );
        }

        // Silkscreen arcs
        for arc in &self.silkscreen_arcs {
            builder.add_silkscreen_arc(
                arc.x.to_mm(),
                arc.y.to_mm(),
                arc.radius.to_mm(),
                arc.start_angle,
                arc.end_angle,
                arc.width.to_mm(),
            );
        }

        // Courtyard
        if let Some(ref court) = self.courtyard {
            builder.add_courtyard_rect(
                court.x.to_mm(),
                court.y.to_mm(),
                court.width.to_mm(),
                court.height.to_mm(),
                court.line_width.to_mm(),
            );
        }

        // Pin 1 indicator
        let show_indicator = self.pin1_indicator.unwrap_or(has_pads);
        if show_indicator && has_pads {
            // Compute indicator position near pin 1
            let (ind_x, ind_y) = if let Some(ref silk) = self.silkscreen {
                // Place outside silkscreen, near bottom-left corner
                let x = silk.x.to_mm() - silk.width.to_mm() / 2.0 - 0.5;
                let y = silk.y.to_mm() - silk.height.to_mm() / 2.0;
                (x, y)
            } else {
                // Infer position from pad geometry: find first pad position
                self.first_pad_position()
                    .map(|(x, y)| (x - 0.5, y - 0.5))
                    .unwrap_or((-0.5, -0.5))
            };
            builder.add_pin1_indicator(ind_x, ind_y, 0.3);
        }

        let component = builder.build_deterministic(&mut ());
        Ok(component)
    }
}

fn parse_pad_shape(s: &str) -> PcbPadShape {
    match s.to_lowercase().as_str() {
        "round" | "circle" => PcbPadShape::Round,
        "rectangular" | "rect" | "rectangle" => PcbPadShape::Rectangular,
        "octagonal" | "octagon" | "oct" => PcbPadShape::Octagonal,
        "roundrect" | "round_rect" | "rounded_rectangle" => PcbPadShape::RoundRect,
        _ => PcbPadShape::Rectangular,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soic8_template() {
        let template = PcbFootprintTemplate {
            name: "SOIC-8".to_string(),
            description: Some("8-pin SOIC package".to_string()),
            dual_rows: vec![DualRowTemplate {
                pads_per_side: 4,
                pitch: MmInput(1.27),
                row_spacing: MmInput(5.3),
                pad_width: MmInput(1.5),
                pad_height: MmInput(0.6),
                shape: "rectangular".to_string(),
                hole: None,
            }],
            silkscreen: Some(SilkscreenTemplate {
                width: MmInput(3.9),
                height: MmInput(4.9),
                x: MmInput(0.0),
                y: MmInput(0.0),
                line_width: MmInput(0.2),
            }),
            ..Default::default()
        };

        let result = template.apply().unwrap();
        assert_eq!(result.pattern, "SOIC-8");
        assert_eq!(result.pad_count(), 8);
    }

    #[test]
    fn test_dip16_template() {
        let template = PcbFootprintTemplate {
            name: "DIP-16".to_string(),
            dual_rows: vec![DualRowTemplate {
                pads_per_side: 8,
                pitch: MmInput(2.54),
                row_spacing: MmInput(7.62),
                pad_width: MmInput(1.6),
                pad_height: MmInput(1.6),
                shape: "round".to_string(),
                hole: Some(MmInput(0.9)),
            }],
            ..Default::default()
        };

        let result = template.apply().unwrap();
        assert_eq!(result.pad_count(), 16);
        // Verify through-hole
        let pads: Vec<_> = result.pads().collect();
        assert!(pads[0].has_hole());
    }

    #[test]
    fn test_bga_template() {
        let template = PcbFootprintTemplate {
            name: "BGA-64".to_string(),
            pad_grids: vec![PadGridTemplate {
                rows: 8,
                cols: 8,
                pitch: MmInput(0.8),
                pad_diameter: MmInput(0.4),
                shape: "round".to_string(),
                skip_center: MmInput(0.0),
            }],
            ..Default::default()
        };

        let result = template.apply().unwrap();
        assert_eq!(result.pad_count(), 64);
    }

    #[test]
    fn test_explicit_pads() {
        let template = PcbFootprintTemplate {
            name: "SOT23-3".to_string(),
            pads: vec![
                PadTemplate {
                    designator: Some("1".to_string()),
                    x: MmInput(-0.95),
                    y: MmInput(-1.0),
                    width: MmInput(0.6),
                    height: Some(MmInput(0.7)),
                    shape: Some("rectangular".to_string()),
                    hole: None,
                },
                PadTemplate {
                    designator: Some("2".to_string()),
                    x: MmInput(0.95),
                    y: MmInput(-1.0),
                    width: MmInput(0.6),
                    height: Some(MmInput(0.7)),
                    shape: Some("rectangular".to_string()),
                    hole: None,
                },
                PadTemplate {
                    designator: Some("3".to_string()),
                    x: MmInput(0.0),
                    y: MmInput(1.0),
                    width: MmInput(0.6),
                    height: Some(MmInput(0.7)),
                    shape: Some("rectangular".to_string()),
                    hole: None,
                },
            ],
            ..Default::default()
        };

        let result = template.apply().unwrap();
        assert_eq!(result.pad_count(), 3);
    }

    #[test]
    fn test_json_roundtrip() {
        let template = PcbFootprintTemplate {
            name: "TEST".to_string(),
            dual_rows: vec![DualRowTemplate {
                pads_per_side: 4,
                pitch: MmInput(1.27),
                row_spacing: MmInput(5.3),
                pad_width: MmInput(1.5),
                pad_height: MmInput(0.6),
                shape: "rectangular".to_string(),
                hole: None,
            }],
            ..Default::default()
        };

        let json = serde_json::to_string(&template).unwrap();
        let parsed: PcbFootprintTemplate = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "TEST");
        assert_eq!(parsed.dual_rows.len(), 1);
    }

    #[test]
    fn test_json_schema_generation() {
        let schema = schemars::schema_for!(PcbFootprintTemplate);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(json.contains("PcbFootprintTemplate"));
        assert!(json.contains("DualRowTemplate"));
    }
}

//! Footprint builder for programmatic footprint creation.
//!
//! # V2 Migration Note
//! This module uses v1 types (PcbComponent, PcbRecord, PcbPad, PcbTrack, etc.).
//! TODO: Migrate to v2 PCB types when fully available.

#![allow(deprecated)]

use crate::records::pcb::{
    PcbArc, PcbComponent, PcbFlags, PcbPad, PcbPadHoleShape, PcbPadShape, PcbPrimitiveCommon,
    PcbRecord, PcbStackMode, PcbTrack,
};
use crate::types::{Coord, CoordPoint, Layer, MaskExpansion};

/// Direction for a row of pads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadRowDirection {
    /// Pads arranged horizontally (along X axis).
    Horizontal,
    /// Pads arranged vertically (along Y axis).
    Vertical,
}

impl PadRowDirection {
    /// Parse from string (accepts "horizontal", "h", "x", "vertical", "v", "y").
    pub fn try_parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "horizontal" | "h" | "x" | "horiz" => Some(PadRowDirection::Horizontal),
            "vertical" | "v" | "y" | "vert" => Some(PadRowDirection::Vertical),
            _ => None,
        }
    }
}

/// Builder for creating PCB footprints.
#[derive(Debug)]
pub struct FootprintBuilder {
    /// Footprint pattern name.
    name: String,
    /// Footprint description.
    description: String,
    /// Component height (for 3D).
    height: Coord,
    /// Primitives in the footprint.
    primitives: Vec<PcbRecord>,
    /// Next pad designator number.
    next_pad_num: u32,
}

impl FootprintBuilder {
    /// Create a new footprint builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            height: Coord::default(),
            primitives: Vec::new(),
            next_pad_num: 1,
        }
    }

    /// Set the footprint description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set the component height.
    pub fn height_mm(mut self, height: f64) -> Self {
        self.height = Coord::from_mms(height);
        self
    }

    /// Add an SMD pad.
    pub fn add_smd_pad(
        &mut self,
        designator: impl Into<String>,
        x_mm: f64,
        y_mm: f64,
        width_mm: f64,
        height_mm: f64,
        shape: PcbPadShape,
    ) -> &mut Self {
        let pad = self.create_smd_pad(
            designator.into(),
            Coord::from_mms(x_mm),
            Coord::from_mms(y_mm),
            Coord::from_mms(width_mm),
            Coord::from_mms(height_mm),
            shape,
            Layer::TOP_LAYER,
        );
        self.primitives.push(PcbRecord::Pad(Box::new(pad)));
        self
    }

    /// Add an SMD pad with auto-generated designator.
    pub fn add_smd_pad_auto(
        &mut self,
        x_mm: f64,
        y_mm: f64,
        width_mm: f64,
        height_mm: f64,
        shape: PcbPadShape,
    ) -> &mut Self {
        let designator = self.next_pad_num.to_string();
        self.next_pad_num += 1;
        self.add_smd_pad(designator, x_mm, y_mm, width_mm, height_mm, shape)
    }

    /// Add a through-hole pad.
    pub fn add_th_pad(
        &mut self,
        designator: impl Into<String>,
        x_mm: f64,
        y_mm: f64,
        pad_diameter_mm: f64,
        hole_diameter_mm: f64,
        shape: PcbPadShape,
    ) -> &mut Self {
        let pad = self.create_th_pad(
            designator.into(),
            Coord::from_mms(x_mm),
            Coord::from_mms(y_mm),
            Coord::from_mms(pad_diameter_mm),
            Coord::from_mms(hole_diameter_mm),
            shape,
        );
        self.primitives.push(PcbRecord::Pad(Box::new(pad)));
        self
    }

    /// Add a through-hole pad with auto-generated designator.
    pub fn add_th_pad_auto(
        &mut self,
        x_mm: f64,
        y_mm: f64,
        pad_diameter_mm: f64,
        hole_diameter_mm: f64,
        shape: PcbPadShape,
    ) -> &mut Self {
        let designator = self.next_pad_num.to_string();
        self.next_pad_num += 1;
        self.add_th_pad(
            designator,
            x_mm,
            y_mm,
            pad_diameter_mm,
            hole_diameter_mm,
            shape,
        )
    }

    /// Add a rectangular through-hole pad (for pin 1 marking, etc.).
    pub fn add_th_rect_pad(
        &mut self,
        designator: impl Into<String>,
        x_mm: f64,
        y_mm: f64,
        width_mm: f64,
        height_mm: f64,
        hole_diameter_mm: f64,
    ) -> &mut Self {
        let mut pad = self.create_th_pad(
            designator.into(),
            Coord::from_mms(x_mm),
            Coord::from_mms(y_mm),
            Coord::from_mms(width_mm.max(height_mm)),
            Coord::from_mms(hole_diameter_mm),
            PcbPadShape::Rectangular,
        );
        // Set different X/Y sizes for rectangular pad
        let size = CoordPoint::from_mms(width_mm, height_mm);
        for i in 0..32 {
            pad.size_layers[i] = size;
        }
        self.primitives.push(PcbRecord::Pad(Box::new(pad)));
        self
    }

    /// Add a silkscreen line (track on top overlay).
    pub fn add_silkscreen_line(
        &mut self,
        x1_mm: f64,
        y1_mm: f64,
        x2_mm: f64,
        y2_mm: f64,
        width_mm: f64,
    ) -> &mut Self {
        let track = PcbTrack {
            common: PcbPrimitiveCommon {
                layer: Layer::TOP_OVERLAY,
                flags: PcbFlags::UNLOCKED | PcbFlags::UNKNOWN8,
                unique_id: None,
            },
            start: CoordPoint::from_mms(x1_mm, y1_mm),
            end: CoordPoint::from_mms(x2_mm, y2_mm),
            width: Coord::from_mms(width_mm),
            unknown: vec![0u8; 16],
        };
        self.primitives.push(PcbRecord::Track(track));
        self
    }

    /// Add a silkscreen rectangle outline.
    pub fn add_silkscreen_rect(
        &mut self,
        x_mm: f64,
        y_mm: f64,
        width_mm: f64,
        height_mm: f64,
        line_width_mm: f64,
    ) -> &mut Self {
        let half_w = width_mm / 2.0;
        let half_h = height_mm / 2.0;

        // Draw four lines for rectangle
        self.add_silkscreen_line(
            x_mm - half_w,
            y_mm - half_h,
            x_mm + half_w,
            y_mm - half_h,
            line_width_mm,
        );
        self.add_silkscreen_line(
            x_mm + half_w,
            y_mm - half_h,
            x_mm + half_w,
            y_mm + half_h,
            line_width_mm,
        );
        self.add_silkscreen_line(
            x_mm + half_w,
            y_mm + half_h,
            x_mm - half_w,
            y_mm + half_h,
            line_width_mm,
        );
        self.add_silkscreen_line(
            x_mm - half_w,
            y_mm + half_h,
            x_mm - half_w,
            y_mm - half_h,
            line_width_mm,
        );
        self
    }

    /// Add a silkscreen arc.
    pub fn add_silkscreen_arc(
        &mut self,
        center_x_mm: f64,
        center_y_mm: f64,
        radius_mm: f64,
        start_angle: f64,
        end_angle: f64,
        width_mm: f64,
    ) -> &mut Self {
        let arc = PcbArc {
            common: PcbPrimitiveCommon {
                layer: Layer::TOP_OVERLAY,
                flags: PcbFlags::UNLOCKED | PcbFlags::UNKNOWN8,
                unique_id: None,
            },
            location: CoordPoint::from_mms(center_x_mm, center_y_mm),
            radius: Coord::from_mms(radius_mm),
            start_angle,
            end_angle,
            width: Coord::from_mms(width_mm),
        };
        self.primitives.push(PcbRecord::Arc(arc));
        self
    }

    /// Add a silkscreen circle.
    pub fn add_silkscreen_circle(
        &mut self,
        center_x_mm: f64,
        center_y_mm: f64,
        radius_mm: f64,
        width_mm: f64,
    ) -> &mut Self {
        self.add_silkscreen_arc(center_x_mm, center_y_mm, radius_mm, 0.0, 360.0, width_mm)
    }

    /// Add pin 1 indicator (small dot on silkscreen).
    pub fn add_pin1_indicator(&mut self, x_mm: f64, y_mm: f64, radius_mm: f64) -> &mut Self {
        // Small filled circle - use a thick arc
        self.add_silkscreen_circle(x_mm, y_mm, radius_mm / 2.0, radius_mm)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // HIGH-LEVEL PAD CREATION (matches datasheet terminology)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Add a row of SMD pads with specified pitch (center-to-center distance).
    ///
    /// This is the most fundamental high-level operation - creates equally-spaced
    /// pads in a line, like you'd see on one side of a SOIC or connector.
    ///
    /// # Arguments
    /// * `count` - Number of pads to create
    /// * `pitch_mm` - Center-to-center distance between adjacent pads
    /// * `pad_width_mm` - Width of each pad (perpendicular to row)
    /// * `pad_height_mm` - Height of each pad (along row direction)
    /// * `start_x_mm` - X position of first pad center
    /// * `start_y_mm` - Y position of first pad center
    /// * `direction` - "horizontal" (pads extend in +X) or "vertical" (pads extend in +Y)
    /// * `start_designator` - First pad number (subsequent pads increment from this)
    /// * `shape` - Pad shape
    #[allow(clippy::too_many_arguments)]
    pub fn add_pad_row(
        &mut self,
        count: usize,
        pitch_mm: f64,
        pad_width_mm: f64,
        pad_height_mm: f64,
        start_x_mm: f64,
        start_y_mm: f64,
        direction: PadRowDirection,
        start_designator: u32,
        shape: PcbPadShape,
    ) -> &mut Self {
        for i in 0..count {
            let designator = (start_designator + i as u32).to_string();
            let offset = i as f64 * pitch_mm;
            let (x, y) = match direction {
                PadRowDirection::Horizontal => (start_x_mm + offset, start_y_mm),
                PadRowDirection::Vertical => (start_x_mm, start_y_mm + offset),
            };
            self.add_smd_pad(&designator, x, y, pad_width_mm, pad_height_mm, shape);
        }
        self
    }

    /// Add a row of through-hole pads with specified pitch.
    #[allow(clippy::too_many_arguments)]
    pub fn add_th_pad_row(
        &mut self,
        count: usize,
        pitch_mm: f64,
        pad_diameter_mm: f64,
        hole_diameter_mm: f64,
        start_x_mm: f64,
        start_y_mm: f64,
        direction: PadRowDirection,
        start_designator: u32,
        shape: PcbPadShape,
    ) -> &mut Self {
        for i in 0..count {
            let designator = (start_designator + i as u32).to_string();
            let offset = i as f64 * pitch_mm;
            let (x, y) = match direction {
                PadRowDirection::Horizontal => (start_x_mm + offset, start_y_mm),
                PadRowDirection::Vertical => (start_x_mm, start_y_mm + offset),
            };
            self.add_th_pad(&designator, x, y, pad_diameter_mm, hole_diameter_mm, shape);
        }
        self
    }

    /// Add dual rows of SMD pads (like SOIC, SOP, TSSOP packages).
    ///
    /// Creates two parallel rows of pads, numbered sequentially down one side
    /// then up the other (standard IC numbering).
    ///
    /// # Arguments
    /// * `pads_per_side` - Number of pads on each side
    /// * `pitch_mm` - Center-to-center distance between adjacent pads (along each row)
    /// * `row_spacing_mm` - Distance between row centers (lead span / center-to-center)
    /// * `pad_width_mm` - Pad width (perpendicular to package body)
    /// * `pad_height_mm` - Pad height (along package body)
    /// * `shape` - Pad shape
    ///
    /// The package is centered at origin. Left row is pads 1 to N, right row is N+1 to 2N
    /// (numbered bottom-to-top on left, top-to-bottom on right).
    pub fn add_dual_row_smd(
        &mut self,
        pads_per_side: usize,
        pitch_mm: f64,
        row_spacing_mm: f64,
        pad_width_mm: f64,
        pad_height_mm: f64,
        shape: PcbPadShape,
    ) -> &mut Self {
        let half_span = row_spacing_mm / 2.0;
        let row_length = (pads_per_side - 1) as f64 * pitch_mm;
        let start_y = -row_length / 2.0;

        // Left row (pins 1 to N, bottom to top)
        for i in 0..pads_per_side {
            let designator = (i + 1).to_string();
            let y = start_y + i as f64 * pitch_mm;
            self.add_smd_pad(
                &designator,
                -half_span,
                y,
                pad_width_mm,
                pad_height_mm,
                shape,
            );
        }

        // Right row (pins N+1 to 2N, top to bottom)
        for i in 0..pads_per_side {
            let designator = (pads_per_side + i + 1).to_string();
            let y = start_y + (pads_per_side - 1 - i) as f64 * pitch_mm;
            self.add_smd_pad(
                &designator,
                half_span,
                y,
                pad_width_mm,
                pad_height_mm,
                shape,
            );
        }

        self
    }

    /// Add dual rows of through-hole pads (like DIP packages).
    ///
    /// Same numbering as `add_dual_row_smd`.
    pub fn add_dual_row_th(
        &mut self,
        pads_per_side: usize,
        pitch_mm: f64,
        row_spacing_mm: f64,
        pad_diameter_mm: f64,
        hole_diameter_mm: f64,
        shape: PcbPadShape,
    ) -> &mut Self {
        let half_span = row_spacing_mm / 2.0;
        let row_length = (pads_per_side - 1) as f64 * pitch_mm;
        let start_y = -row_length / 2.0;

        // Left row (pins 1 to N, bottom to top)
        for i in 0..pads_per_side {
            let designator = (i + 1).to_string();
            let y = start_y + i as f64 * pitch_mm;
            self.add_th_pad(
                &designator,
                -half_span,
                y,
                pad_diameter_mm,
                hole_diameter_mm,
                shape,
            );
        }

        // Right row (pins N+1 to 2N, top to bottom)
        for i in 0..pads_per_side {
            let designator = (pads_per_side + i + 1).to_string();
            let y = start_y + (pads_per_side - 1 - i) as f64 * pitch_mm;
            self.add_th_pad(
                &designator,
                half_span,
                y,
                pad_diameter_mm,
                hole_diameter_mm,
                shape,
            );
        }

        self
    }

    /// Add quad arrangement of SMD pads (like QFP, LQFP, TQFP packages).
    ///
    /// Creates four rows of pads around a square/rectangular body.
    /// Numbering starts at bottom-left corner of left side, goes counter-clockwise.
    ///
    /// # Arguments
    /// * `pads_per_side` - Number of pads on each side
    /// * `pitch_mm` - Center-to-center distance between adjacent pads
    /// * `span_mm` - Distance between opposite row centers (lead span)
    /// * `pad_width_mm` - Pad width (perpendicular to body edge)
    /// * `pad_height_mm` - Pad height (along body edge)
    /// * `shape` - Pad shape
    pub fn add_quad_pads_smd(
        &mut self,
        pads_per_side: usize,
        pitch_mm: f64,
        span_mm: f64,
        pad_width_mm: f64,
        pad_height_mm: f64,
        shape: PcbPadShape,
    ) -> &mut Self {
        let half_span = span_mm / 2.0;
        let row_length = (pads_per_side - 1) as f64 * pitch_mm;
        let start_offset = -row_length / 2.0;
        let mut pin = 1u32;

        // Left side (bottom to top)
        for i in 0..pads_per_side {
            let y = start_offset + i as f64 * pitch_mm;
            self.add_smd_pad(
                pin.to_string(),
                -half_span,
                y,
                pad_width_mm,
                pad_height_mm,
                shape,
            );
            pin += 1;
        }

        // Bottom side (left to right)
        for i in 0..pads_per_side {
            let x = start_offset + i as f64 * pitch_mm;
            // Rotated 90 degrees, so swap width/height
            self.add_smd_pad(
                pin.to_string(),
                x,
                -half_span,
                pad_height_mm,
                pad_width_mm,
                shape,
            );
            pin += 1;
        }

        // Right side (bottom to top) - numbered in reverse
        for i in 0..pads_per_side {
            let y = start_offset + (pads_per_side - 1 - i) as f64 * pitch_mm;
            self.add_smd_pad(
                pin.to_string(),
                half_span,
                y,
                pad_width_mm,
                pad_height_mm,
                shape,
            );
            pin += 1;
        }

        // Top side (right to left) - numbered in reverse
        for i in 0..pads_per_side {
            let x = start_offset + (pads_per_side - 1 - i) as f64 * pitch_mm;
            self.add_smd_pad(
                pin.to_string(),
                x,
                half_span,
                pad_height_mm,
                pad_width_mm,
                shape,
            );
            pin += 1;
        }

        self
    }

    /// Add a grid of SMD pads (like BGA, LGA packages).
    ///
    /// Creates a matrix of pads with alphanumeric designators (A1, A2, ..., B1, B2, ...).
    ///
    /// # Arguments
    /// * `rows` - Number of rows (letters A, B, C, ...)
    /// * `cols` - Number of columns (numbers 1, 2, 3, ...)
    /// * `pitch_mm` - Center-to-center distance (same for X and Y)
    /// * `pad_diameter_mm` - Pad diameter
    /// * `shape` - Pad shape (typically Round for BGA)
    /// * `skip_center` - If > 0, skip pads within this radius from center (for thermal pad)
    pub fn add_pad_grid(
        &mut self,
        rows: usize,
        cols: usize,
        pitch_mm: f64,
        pad_diameter_mm: f64,
        shape: PcbPadShape,
        skip_center_mm: f64,
    ) -> &mut Self {
        let grid_width = (cols - 1) as f64 * pitch_mm;
        let grid_height = (rows - 1) as f64 * pitch_mm;
        let start_x = -grid_width / 2.0;
        let start_y = grid_height / 2.0; // Start from top

        for row in 0..rows {
            let row_letter = (b'A' + row as u8) as char;
            let y = start_y - row as f64 * pitch_mm;

            for col in 0..cols {
                let x = start_x + col as f64 * pitch_mm;

                // Skip if within center exclusion zone
                if skip_center_mm > 0.0 {
                    let dist = (x * x + y * y).sqrt();
                    if dist < skip_center_mm {
                        continue;
                    }
                }

                let designator = format!("{}{}", row_letter, col + 1);
                self.add_smd_pad(&designator, x, y, pad_diameter_mm, pad_diameter_mm, shape);
            }
        }

        self
    }

    /// Add a grid of SMD pads with separate X and Y pitches.
    #[allow(clippy::too_many_arguments)]
    pub fn add_pad_grid_xy(
        &mut self,
        rows: usize,
        cols: usize,
        pitch_x_mm: f64,
        pitch_y_mm: f64,
        pad_width_mm: f64,
        pad_height_mm: f64,
        shape: PcbPadShape,
    ) -> &mut Self {
        let grid_width = (cols - 1) as f64 * pitch_x_mm;
        let grid_height = (rows - 1) as f64 * pitch_y_mm;
        let start_x = -grid_width / 2.0;
        let start_y = grid_height / 2.0;

        for row in 0..rows {
            let row_letter = (b'A' + row as u8) as char;
            let y = start_y - row as f64 * pitch_y_mm;

            for col in 0..cols {
                let x = start_x + col as f64 * pitch_x_mm;
                let designator = format!("{}{}", row_letter, col + 1);
                self.add_smd_pad(&designator, x, y, pad_width_mm, pad_height_mm, shape);
            }
        }

        self
    }

    /// Add pads using "spacing" (edge-to-edge) rather than pitch (center-to-center).
    ///
    /// Useful when datasheets specify gap between pads rather than pitch.
    ///
    /// # Arguments
    /// * `count` - Number of pads
    /// * `spacing_mm` - Edge-to-edge distance between adjacent pads
    /// * `pad_width_mm` - Pad width (in the direction of the row)
    /// * `pad_height_mm` - Pad height (perpendicular to row)
    /// * `start_x_mm`, `start_y_mm` - Position of first pad center
    /// * `direction` - Row direction
    /// * `start_designator` - First pad number
    /// * `shape` - Pad shape
    #[allow(clippy::too_many_arguments)]
    pub fn add_pad_row_with_spacing(
        &mut self,
        count: usize,
        spacing_mm: f64,
        pad_width_mm: f64,
        pad_height_mm: f64,
        start_x_mm: f64,
        start_y_mm: f64,
        direction: PadRowDirection,
        start_designator: u32,
        shape: PcbPadShape,
    ) -> &mut Self {
        // Convert spacing to pitch: pitch = spacing + pad_dimension_along_row
        let pad_along_row = match direction {
            PadRowDirection::Horizontal => pad_width_mm,
            PadRowDirection::Vertical => pad_height_mm,
        };
        let pitch_mm = spacing_mm + pad_along_row;
        self.add_pad_row(
            count,
            pitch_mm,
            pad_width_mm,
            pad_height_mm,
            start_x_mm,
            start_y_mm,
            direction,
            start_designator,
            shape,
        )
    }

    /// Add a courtyard line (mechanical layer).
    pub fn add_courtyard_line(
        &mut self,
        x1_mm: f64,
        y1_mm: f64,
        x2_mm: f64,
        y2_mm: f64,
        width_mm: f64,
    ) -> &mut Self {
        let track = PcbTrack {
            common: PcbPrimitiveCommon {
                layer: Layer::MECHANICAL_15, // Courtyard layer
                flags: PcbFlags::UNLOCKED | PcbFlags::UNKNOWN8,
                unique_id: None,
            },
            start: CoordPoint::from_mms(x1_mm, y1_mm),
            end: CoordPoint::from_mms(x2_mm, y2_mm),
            width: Coord::from_mms(width_mm),
            unknown: vec![0u8; 16],
        };
        self.primitives.push(PcbRecord::Track(track));
        self
    }

    /// Add a courtyard rectangle.
    pub fn add_courtyard_rect(
        &mut self,
        x_mm: f64,
        y_mm: f64,
        width_mm: f64,
        height_mm: f64,
        line_width_mm: f64,
    ) -> &mut Self {
        let half_w = width_mm / 2.0;
        let half_h = height_mm / 2.0;

        self.add_courtyard_line(
            x_mm - half_w,
            y_mm - half_h,
            x_mm + half_w,
            y_mm - half_h,
            line_width_mm,
        );
        self.add_courtyard_line(
            x_mm + half_w,
            y_mm - half_h,
            x_mm + half_w,
            y_mm + half_h,
            line_width_mm,
        );
        self.add_courtyard_line(
            x_mm + half_w,
            y_mm + half_h,
            x_mm - half_w,
            y_mm + half_h,
            line_width_mm,
        );
        self.add_courtyard_line(
            x_mm - half_w,
            y_mm + half_h,
            x_mm - half_w,
            y_mm - half_h,
            line_width_mm,
        );
        self
    }

    /// Add assembly layer outline (for fabrication drawings).
    pub fn add_assembly_rect(
        &mut self,
        x_mm: f64,
        y_mm: f64,
        width_mm: f64,
        height_mm: f64,
        line_width_mm: f64,
    ) -> &mut Self {
        let half_w = width_mm / 2.0;
        let half_h = height_mm / 2.0;

        let add_line = |builder: &mut Self, x1: f64, y1: f64, x2: f64, y2: f64| {
            let track = PcbTrack {
                common: PcbPrimitiveCommon {
                    layer: Layer::MECHANICAL_13, // Assembly layer
                    flags: PcbFlags::UNLOCKED | PcbFlags::UNKNOWN8,
                    unique_id: None,
                },
                start: CoordPoint::from_mms(x1, y1),
                end: CoordPoint::from_mms(x2, y2),
                width: Coord::from_mms(line_width_mm),
                unknown: vec![0u8; 16],
            };
            builder.primitives.push(PcbRecord::Track(track));
        };

        add_line(
            self,
            x_mm - half_w,
            y_mm - half_h,
            x_mm + half_w,
            y_mm - half_h,
        );
        add_line(
            self,
            x_mm + half_w,
            y_mm - half_h,
            x_mm + half_w,
            y_mm + half_h,
        );
        add_line(
            self,
            x_mm + half_w,
            y_mm + half_h,
            x_mm - half_w,
            y_mm + half_h,
        );
        add_line(
            self,
            x_mm - half_w,
            y_mm + half_h,
            x_mm - half_w,
            y_mm - half_h,
        );
        self
    }

    /// Build the footprint component (non-deterministic).
    ///
    /// **Prefer using `build_deterministic()` for reproducible execution.**
    #[deprecated(
        since = "0.1.0",
        note = "Use build_deterministic() with a DeterminismContext for reproducible execution"
    )]
    pub fn build(self) -> PcbComponent {
        PcbComponent {
            pattern: self.name,
            description: self.description,
            height: self.height,
            item_guid: uuid::Uuid::new_v4().to_string(),
            revision_guid: uuid::Uuid::new_v4().to_string(),
            primitives: self.primitives,
        }
    }

    /// Build component with standard UUID generation.
    ///
    /// Standalone library uses standard UUIDs; Cadatomic fork replaces with deterministic context.
    pub fn build_deterministic(self, _det: &mut ()) -> PcbComponent {
        PcbComponent {
            pattern: self.name,
            description: self.description,
            height: self.height,
            item_guid: uuid::Uuid::new_v4().to_string(),
            revision_guid: uuid::Uuid::new_v4().to_string(),
            primitives: self.primitives,
        }
    }

    // Internal helper methods

    #[allow(clippy::too_many_arguments)]
    fn create_smd_pad(
        &self,
        designator: String,
        x: Coord,
        y: Coord,
        width: Coord,
        height: Coord,
        shape: PcbPadShape,
        layer: Layer,
    ) -> PcbPad {
        let size = CoordPoint::new(width, height);
        let shape_layers = [shape; 32];
        let mut size_layers = [size; 32];

        // SMD pad only on specified layer
        let active_layer_index = layer.to_byte() as usize - 1;
        for (index, size_layer) in size_layers.iter_mut().enumerate() {
            if index != active_layer_index {
                *size_layer = CoordPoint::default();
            }
        }

        PcbPad {
            common: PcbPrimitiveCommon {
                layer,
                flags: PcbFlags::UNLOCKED | PcbFlags::UNKNOWN8,
                unique_id: None,
            },
            designator,
            location: CoordPoint::new(x, y),
            rotation: 0.0,
            is_plated: true,
            jumper_id: 0,
            stack_mode: PcbStackMode::Simple,
            hole_size: Coord::default(),
            hole_shape: PcbPadHoleShape::Round,
            hole_rotation: 0.0,
            hole_slot_length: Coord::default(),
            paste_mask_expansion: MaskExpansion::Auto,
            solder_mask_expansion: MaskExpansion::Auto,
            size_layers,
            shape_layers,
            corner_radius_percentage: [50; 32],
            offsets_from_hole_center: [CoordPoint::default(); 32],
        }
    }

    fn create_th_pad(
        &self,
        designator: String,
        x: Coord,
        y: Coord,
        pad_size: Coord,
        hole_size: Coord,
        shape: PcbPadShape,
    ) -> PcbPad {
        let size = CoordPoint::new(pad_size, pad_size);

        PcbPad {
            common: PcbPrimitiveCommon {
                layer: Layer::multi_layer(),
                flags: PcbFlags::UNLOCKED | PcbFlags::UNKNOWN8,
                unique_id: None,
            },
            designator,
            location: CoordPoint::new(x, y),
            rotation: 0.0,
            is_plated: true,
            jumper_id: 0,
            stack_mode: PcbStackMode::Simple,
            hole_size,
            hole_shape: PcbPadHoleShape::Round,
            hole_rotation: 0.0,
            hole_slot_length: Coord::default(),
            paste_mask_expansion: MaskExpansion::Auto,
            solder_mask_expansion: MaskExpansion::Auto,
            size_layers: [size; 32],
            shape_layers: [shape; 32],
            corner_radius_percentage: [50; 32],
            offsets_from_hole_center: [CoordPoint::default(); 32],
        }
    }
}

/// Extension methods for PcbComponent to support editing.
impl PcbComponent {
    /// Create a new empty component (non-deterministic).
    ///
    /// **Prefer using `new_deterministic()` for reproducible execution.**
    #[deprecated(
        since = "0.1.0",
        note = "Use new_deterministic() with a DeterminismContext for reproducible execution"
    )]
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            description: String::new(),
            height: Coord::default(),
            item_guid: uuid::Uuid::new_v4().to_string(),
            revision_guid: uuid::Uuid::new_v4().to_string(),
            primitives: Vec::new(),
        }
    }

    /// Create new component with standard UUID generation.
    ///
    /// Standalone library uses standard UUIDs; Cadatomic fork replaces with deterministic context.
    pub fn new_deterministic(pattern: impl Into<String>, _det: &mut ()) -> Self {
        Self {
            pattern: pattern.into(),
            description: String::new(),
            height: Coord::default(),
            item_guid: uuid::Uuid::new_v4().to_string(),
            revision_guid: uuid::Uuid::new_v4().to_string(),
            primitives: Vec::new(),
        }
    }

    /// Set the description.
    pub fn set_description(&mut self, desc: impl Into<String>) {
        self.description = desc.into();
    }

    /// Add a primitive.
    pub fn add_primitive(&mut self, record: PcbRecord) {
        self.primitives.push(record);
    }

    /// Remove a primitive by index.
    pub fn remove_primitive(&mut self, index: usize) -> Option<PcbRecord> {
        if index < self.primitives.len() {
            Some(self.primitives.remove(index))
        } else {
            None
        }
    }

    /// Find a pad by designator.
    pub fn find_pad(&self, designator: &str) -> Option<&PcbPad> {
        self.pads().find(|p| p.designator == designator)
    }

    /// Find a pad by designator (mutable).
    pub fn find_pad_mut(&mut self, designator: &str) -> Option<&mut PcbPad> {
        for prim in &mut self.primitives {
            if let PcbRecord::Pad(pad) = prim {
                if pad.designator == designator {
                    return Some(pad);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_footprint_builder_basic() {
        let mut det = ();
        let footprint = FootprintBuilder::new("TEST-FOOTPRINT")
            .description("Test footprint")
            .height_mm(1.0)
            .build_deterministic(&mut det);

        assert_eq!(footprint.pattern, "TEST-FOOTPRINT");
        assert_eq!(footprint.description, "Test footprint");
    }

    #[test]
    fn test_footprint_builder_smd_pads() {
        let mut det = ();
        let mut builder = FootprintBuilder::new("SOT23");
        builder
            .add_smd_pad("1", -0.95, -1.0, 0.6, 0.7, PcbPadShape::Rectangular)
            .add_smd_pad("2", 0.95, -1.0, 0.6, 0.7, PcbPadShape::Rectangular)
            .add_smd_pad("3", 0.0, 1.0, 0.6, 0.7, PcbPadShape::Rectangular);

        let footprint = builder.build_deterministic(&mut det);
        assert_eq!(footprint.pad_count(), 3);
    }

    #[test]
    fn test_footprint_builder_th_pads() {
        let mut det = ();
        let mut builder = FootprintBuilder::new("DIP8");
        for i in 0..8 {
            let x = if i < 4 { -3.81 } else { 3.81 };
            let y = (i % 4) as f64 * 2.54 - 3.81;
            builder.add_th_pad_auto(x, y, 1.6, 0.9, PcbPadShape::Round);
        }

        let footprint = builder.build_deterministic(&mut det);
        assert_eq!(footprint.pad_count(), 8);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // HIGH-LEVEL PAD CREATION TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_pad_row_horizontal() {
        let mut det = ();
        let mut builder = FootprintBuilder::new("CONN-8");
        builder.add_pad_row(
            8,    // count
            2.54, // pitch (mm)
            1.5,  // pad_width (mm)
            0.6,  // pad_height (mm)
            0.0,  // start_x (mm)
            0.0,  // start_y (mm)
            PadRowDirection::Horizontal,
            1, // start_designator
            PcbPadShape::Rectangular,
        );

        let footprint = builder.build_deterministic(&mut det);
        assert_eq!(footprint.pad_count(), 8);

        // Verify positions (pads should be at 0, 2.54, 5.08, ... mm)
        let pads: Vec<_> = footprint.pads().collect();
        assert_eq!(pads[0].designator, "1");
        assert_eq!(pads[7].designator, "8");

        // Check spacing: pad 2 should be at ~2.54mm from pad 1
        let x1 = pads[0].location.x.to_mms();
        let x2 = pads[1].location.x.to_mms();
        assert!((x2 - x1 - 2.54).abs() < 0.01);
    }

    #[test]
    fn test_pad_row_vertical() {
        let mut det = ();
        let mut builder = FootprintBuilder::new("VERT-4");
        builder.add_pad_row(
            4,
            1.0, // 1mm pitch
            0.5,
            0.5,
            0.0,
            0.0,
            PadRowDirection::Vertical,
            1,
            PcbPadShape::Round,
        );

        let footprint = builder.build_deterministic(&mut det);
        assert_eq!(footprint.pad_count(), 4);

        // Verify Y spacing
        let pads: Vec<_> = footprint.pads().collect();
        let y1 = pads[0].location.y.to_mms();
        let y2 = pads[1].location.y.to_mms();
        assert!((y2 - y1 - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_dual_row_smd() {
        let mut det = ();
        let mut builder = FootprintBuilder::new("SOIC-8");
        builder.add_dual_row_smd(
            4,    // pads_per_side
            1.27, // pitch (mm)
            5.3,  // row_spacing (mm)
            1.5,  // pad_width (mm)
            0.6,  // pad_height (mm)
            PcbPadShape::Rectangular,
        );

        let footprint = builder.build_deterministic(&mut det);
        assert_eq!(footprint.pad_count(), 8); // 4 * 2 = 8

        // Verify pin numbering: 1-4 on left, 5-8 on right
        let pads: Vec<_> = footprint.pads().collect();
        assert_eq!(pads[0].designator, "1");
        assert_eq!(pads[3].designator, "4");
        assert_eq!(pads[4].designator, "5");
        assert_eq!(pads[7].designator, "8");

        // Verify row spacing: left pads at -2.65mm, right at +2.65mm
        let left_x = pads[0].location.x.to_mms();
        let right_x = pads[4].location.x.to_mms();
        assert!((right_x - left_x - 5.3).abs() < 0.01);
    }

    #[test]
    fn test_dual_row_th() {
        let mut det = ();
        let mut builder = FootprintBuilder::new("DIP-16");
        builder.add_dual_row_th(
            8,    // pads_per_side
            2.54, // pitch (mm) - 100mil
            7.62, // row_spacing (mm) - 300mil
            1.6,  // pad_diameter (mm)
            0.9,  // hole_diameter (mm)
            PcbPadShape::Round,
        );

        let footprint = builder.build_deterministic(&mut det);
        assert_eq!(footprint.pad_count(), 16); // 8 * 2 = 16

        // Verify it's through-hole (has holes)
        let pads: Vec<_> = footprint.pads().collect();
        assert!(pads[0].has_hole());
        assert!((pads[0].hole_size.to_mms() - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_quad_pads() {
        let mut det = ();
        let mut builder = FootprintBuilder::new("QFP-48");
        builder.add_quad_pads_smd(
            12,  // pads_per_side
            0.5, // pitch (mm)
            9.0, // span (mm)
            1.5, // pad_width (mm)
            0.3, // pad_height (mm)
            PcbPadShape::Rectangular,
        );

        let footprint = builder.build_deterministic(&mut det);
        assert_eq!(footprint.pad_count(), 48); // 12 * 4 = 48

        // Verify sequential numbering
        let pads: Vec<_> = footprint.pads().collect();
        assert_eq!(pads[0].designator, "1");
        assert_eq!(pads[11].designator, "12");
        assert_eq!(pads[12].designator, "13");
        assert_eq!(pads[47].designator, "48");
    }

    #[test]
    fn test_pad_grid() {
        let mut det = ();
        let mut builder = FootprintBuilder::new("BGA-64");
        builder.add_pad_grid(
            8,   // rows (A-H)
            8,   // cols (1-8)
            0.8, // pitch (mm)
            0.4, // pad_diameter (mm)
            PcbPadShape::Round,
            0.0, // skip_center (no center skip)
        );

        let footprint = builder.build_deterministic(&mut det);
        assert_eq!(footprint.pad_count(), 64); // 8 * 8 = 64

        // Verify alphanumeric designators
        let pads: Vec<_> = footprint.pads().collect();
        assert_eq!(pads[0].designator, "A1");
        assert_eq!(pads[7].designator, "A8");
        assert_eq!(pads[8].designator, "B1");
        assert_eq!(pads[63].designator, "H8");
    }

    #[test]
    fn test_pad_grid_with_center_skip() {
        let mut det = ();
        let mut builder = FootprintBuilder::new("BGA-CENTER-SKIP");
        builder.add_pad_grid(
            6,   // rows
            6,   // cols
            1.0, // pitch (mm)
            0.5, // pad_diameter (mm)
            PcbPadShape::Round,
            1.5, // skip_center (skip pads within 1.5mm of center)
        );

        let footprint = builder.build_deterministic(&mut det);
        // Should have less than 36 pads due to center skip
        assert!(footprint.pad_count() < 36);
        assert!(footprint.pad_count() > 30); // But not too many skipped
    }

    #[test]
    fn test_pad_row_with_spacing() {
        let mut det = ();
        let mut builder = FootprintBuilder::new("SPACED-PADS");
        builder.add_pad_row_with_spacing(
            3,   // count
            0.5, // spacing (mm) - edge-to-edge
            1.0, // pad_width (mm)
            0.5, // pad_height (mm)
            0.0, // start_x
            0.0, // start_y
            PadRowDirection::Horizontal,
            1,
            PcbPadShape::Rectangular,
        );

        let footprint = builder.build_deterministic(&mut det);
        assert_eq!(footprint.pad_count(), 3);

        // With 0.5mm spacing and 1.0mm pad width, pitch should be 1.5mm
        let pads: Vec<_> = footprint.pads().collect();
        let x1 = pads[0].location.x.to_mms();
        let x2 = pads[1].location.x.to_mms();
        assert!((x2 - x1 - 1.5).abs() < 0.01); // pitch = spacing + pad_width
    }

    #[test]
    fn test_pad_row_direction_parse() {
        assert_eq!(
            PadRowDirection::try_parse("horizontal"),
            Some(PadRowDirection::Horizontal)
        );
        assert_eq!(
            PadRowDirection::try_parse("h"),
            Some(PadRowDirection::Horizontal)
        );
        assert_eq!(
            PadRowDirection::try_parse("x"),
            Some(PadRowDirection::Horizontal)
        );
        assert_eq!(
            PadRowDirection::try_parse("vertical"),
            Some(PadRowDirection::Vertical)
        );
        assert_eq!(
            PadRowDirection::try_parse("v"),
            Some(PadRowDirection::Vertical)
        );
        assert_eq!(
            PadRowDirection::try_parse("y"),
            Some(PadRowDirection::Vertical)
        );
        assert_eq!(PadRowDirection::try_parse("invalid"), None);
    }
}

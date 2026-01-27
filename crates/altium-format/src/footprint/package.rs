//! Package type definitions and IPC land pattern calculations.

use super::FootprintBuilder;
use crate::records::pcb::PcbPadShape;

/// Common package types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageType {
    /// Chip resistor/capacitor (0201, 0402, 0603, etc.)
    Chip,
    /// Small Outline Transistor
    Sot,
    /// Small Outline Package
    Sop,
    /// Small Outline Integrated Circuit
    Soic,
    /// Thin Small Outline Package
    Tssop,
    /// Quad Flat Package
    Qfp,
    /// Quad Flat No-lead
    Qfn,
    /// Ball Grid Array
    Bga,
    /// Dual In-line Package
    Dip,
    /// Pin Grid Array
    Pga,
    /// Through-Hole connector
    Connector,
}

/// Lead style for packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeadStyle {
    /// Gull-wing leads (SOIC, QFP, TSSOP)
    GullWing,
    /// J-lead (PLCC)
    JLead,
    /// No-lead / Flat lead (QFN, DFN)
    Flat,
    /// Ball (BGA)
    Ball,
    /// Through-hole
    ThroughHole,
    /// Chip termination
    Chip,
}

/// IPC-7351 density levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcDensity {
    /// Most Dense (Least) - Level A
    MostDense,
    /// Nominal (Normal) - Level B
    Nominal,
    /// Least Dense (Most) - Level C
    LeastDense,
}

impl IpcDensity {
    /// Get courtyard excess for this density level.
    pub fn courtyard_excess_mm(&self) -> f64 {
        match self {
            IpcDensity::MostDense => 0.10,
            IpcDensity::Nominal => 0.25,
            IpcDensity::LeastDense => 0.50,
        }
    }

    /// Get toe fillet for gull-wing leads.
    pub fn toe_fillet_mm(&self, pitch_mm: f64) -> f64 {
        if pitch_mm <= 0.625 {
            match self {
                IpcDensity::MostDense => 0.15,
                IpcDensity::Nominal => 0.35,
                IpcDensity::LeastDense => 0.55,
            }
        } else {
            match self {
                IpcDensity::MostDense => 0.25,
                IpcDensity::Nominal => 0.45,
                IpcDensity::LeastDense => 0.65,
            }
        }
    }

    /// Get heel fillet for gull-wing leads.
    pub fn heel_fillet_mm(&self, pitch_mm: f64) -> f64 {
        if pitch_mm <= 0.625 {
            match self {
                IpcDensity::MostDense => 0.25,
                IpcDensity::Nominal => 0.35,
                IpcDensity::LeastDense => 0.45,
            }
        } else {
            match self {
                IpcDensity::MostDense => 0.35,
                IpcDensity::Nominal => 0.45,
                IpcDensity::LeastDense => 0.55,
            }
        }
    }

    /// Get side fillet for gull-wing leads.
    pub fn side_fillet_mm(&self, pitch_mm: f64) -> f64 {
        if pitch_mm <= 0.625 {
            match self {
                IpcDensity::MostDense => -0.02,
                IpcDensity::Nominal => 0.01,
                IpcDensity::LeastDense => 0.05,
            }
        } else {
            match self {
                IpcDensity::MostDense => 0.01,
                IpcDensity::Nominal => 0.05,
                IpcDensity::LeastDense => 0.07,
            }
        }
    }
}

/// Chip component specification.
#[derive(Debug, Clone)]
pub struct ChipSpec {
    /// Imperial size code (e.g., "0402", "0603", "0805")
    pub size_code: String,
    /// Body length (mm)
    pub body_length_mm: f64,
    /// Body width (mm)
    pub body_width_mm: f64,
    /// Terminal length (mm)
    pub terminal_length_mm: f64,
    /// Component height (mm)
    pub height_mm: f64,
}

impl ChipSpec {
    /// Standard 0201 chip.
    pub fn chip_0201() -> Self {
        Self {
            size_code: "0201".to_string(),
            body_length_mm: 0.60,
            body_width_mm: 0.30,
            terminal_length_mm: 0.15,
            height_mm: 0.30,
        }
    }

    /// Standard 0402 chip.
    pub fn chip_0402() -> Self {
        Self {
            size_code: "0402".to_string(),
            body_length_mm: 1.00,
            body_width_mm: 0.50,
            terminal_length_mm: 0.25,
            height_mm: 0.50,
        }
    }

    /// Standard 0603 chip.
    pub fn chip_0603() -> Self {
        Self {
            size_code: "0603".to_string(),
            body_length_mm: 1.60,
            body_width_mm: 0.80,
            terminal_length_mm: 0.30,
            height_mm: 0.80,
        }
    }

    /// Standard 0805 chip.
    pub fn chip_0805() -> Self {
        Self {
            size_code: "0805".to_string(),
            body_length_mm: 2.00,
            body_width_mm: 1.25,
            terminal_length_mm: 0.40,
            height_mm: 1.00,
        }
    }

    /// Standard 1206 chip.
    pub fn chip_1206() -> Self {
        Self {
            size_code: "1206".to_string(),
            body_length_mm: 3.20,
            body_width_mm: 1.60,
            terminal_length_mm: 0.50,
            height_mm: 1.10,
        }
    }

    /// Create a footprint for this chip spec.
    pub fn to_footprint(&self, density: IpcDensity) -> FootprintBuilder {
        // IPC-7351B calculations for chip components
        let toe = 0.55; // Toe extension
        let heel = 0.00; // Heel extension (negative for chips)
        let side = 0.05; // Side extension

        let courtyard_excess = density.courtyard_excess_mm();

        // Calculate pad dimensions
        let pad_length = self.terminal_length_mm + toe - heel;
        let pad_width = self.body_width_mm + 2.0 * side;

        // Calculate pad center position
        let pad_center_x = (self.body_length_mm - self.terminal_length_mm + pad_length) / 2.0;

        let mut builder = FootprintBuilder::new(format!("CHIP_{}", self.size_code))
            .description(format!("{} Chip Component", self.size_code))
            .height_mm(self.height_mm);

        // Add pads
        builder.add_smd_pad(
            "1",
            -pad_center_x,
            0.0,
            pad_length,
            pad_width,
            PcbPadShape::Rectangular,
        );
        builder.add_smd_pad(
            "2",
            pad_center_x,
            0.0,
            pad_length,
            pad_width,
            PcbPadShape::Rectangular,
        );

        // Add silkscreen (avoid pads)
        let silk_width = 0.15;
        let silk_y = pad_width / 2.0 + silk_width;
        let silk_x = self.body_length_mm / 2.0;

        builder.add_silkscreen_line(-silk_x, silk_y, silk_x, silk_y, silk_width);
        builder.add_silkscreen_line(-silk_x, -silk_y, silk_x, -silk_y, silk_width);

        // Add courtyard
        let cy_x = pad_center_x + pad_length / 2.0 + courtyard_excess;
        let cy_y = pad_width / 2.0 + courtyard_excess;
        builder.add_courtyard_rect(0.0, 0.0, 2.0 * cy_x, 2.0 * cy_y, 0.05);

        builder
    }
}

/// Gull-wing IC specification (SOIC, TSSOP, QFP).
#[derive(Debug, Clone)]
pub struct GullWingSpec {
    /// Package name (e.g., "SOIC-8", "TSSOP-20")
    pub name: String,
    /// Number of pins.
    pub pin_count: u32,
    /// Pitch between pins (mm).
    pub pitch_mm: f64,
    /// Lead span (tip to tip, mm).
    pub lead_span_mm: f64,
    /// Lead width (mm).
    pub lead_width_mm: f64,
    /// Lead length (mm).
    pub lead_length_mm: f64,
    /// Body width (mm) - parallel to leads.
    pub body_width_mm: f64,
    /// Body length (mm) - perpendicular to leads.
    pub body_length_mm: f64,
    /// Component height (mm).
    pub height_mm: f64,
    /// Number of sides with pins (2 for SOIC, 4 for QFP).
    pub sides: u8,
}

impl GullWingSpec {
    /// Standard SOIC-8 narrow body.
    pub fn soic_8() -> Self {
        Self {
            name: "SOIC-8".to_string(),
            pin_count: 8,
            pitch_mm: 1.27,
            lead_span_mm: 6.0,
            lead_width_mm: 0.40,
            lead_length_mm: 0.70,
            body_width_mm: 3.9,
            body_length_mm: 4.9,
            height_mm: 1.75,
            sides: 2,
        }
    }

    /// Standard TSSOP-20.
    pub fn tssop_20() -> Self {
        Self {
            name: "TSSOP-20".to_string(),
            pin_count: 20,
            pitch_mm: 0.65,
            lead_span_mm: 6.4,
            lead_width_mm: 0.25,
            lead_length_mm: 0.60,
            body_width_mm: 4.4,
            body_length_mm: 6.5,
            height_mm: 1.1,
            sides: 2,
        }
    }

    /// Standard LQFP-48.
    pub fn lqfp_48() -> Self {
        Self {
            name: "LQFP-48".to_string(),
            pin_count: 48,
            pitch_mm: 0.5,
            lead_span_mm: 9.0,
            lead_width_mm: 0.22,
            lead_length_mm: 0.50,
            body_width_mm: 7.0,
            body_length_mm: 7.0,
            height_mm: 1.4,
            sides: 4,
        }
    }

    /// Create a footprint for this IC spec.
    pub fn to_footprint(&self, density: IpcDensity) -> FootprintBuilder {
        let toe = density.toe_fillet_mm(self.pitch_mm);
        let heel = density.heel_fillet_mm(self.pitch_mm);
        let side = density.side_fillet_mm(self.pitch_mm);
        let courtyard_excess = density.courtyard_excess_mm();

        // Calculate pad dimensions
        let pad_length = self.lead_length_mm + toe + heel;
        let pad_width = self.lead_width_mm + 2.0 * side;

        // Pad center distance from origin
        let pad_center = (self.lead_span_mm - self.lead_length_mm + pad_length) / 2.0;

        let mut builder = FootprintBuilder::new(&self.name)
            .description(format!("{} package", self.name))
            .height_mm(self.height_mm);

        if self.sides == 2 {
            // Two-sided package (SOIC, TSSOP)
            let pins_per_side = self.pin_count / 2;
            let total_span = (pins_per_side - 1) as f64 * self.pitch_mm;
            let start_y = -total_span / 2.0;

            // Left side (pins 1 to N/2)
            for i in 0..pins_per_side {
                let y = start_y + i as f64 * self.pitch_mm;
                builder.add_smd_pad(
                    (i + 1).to_string(),
                    -pad_center,
                    y,
                    pad_length,
                    pad_width,
                    PcbPadShape::Rectangular,
                );
            }

            // Right side (pins N/2+1 to N, going down)
            for i in 0..pins_per_side {
                let y = -start_y - i as f64 * self.pitch_mm;
                builder.add_smd_pad(
                    (pins_per_side + i + 1).to_string(),
                    pad_center,
                    y,
                    pad_length,
                    pad_width,
                    PcbPadShape::Rectangular,
                );
            }
        } else {
            // Four-sided package (QFP)
            let pins_per_side = self.pin_count / 4;
            let total_span = (pins_per_side - 1) as f64 * self.pitch_mm;
            let start_pos = -total_span / 2.0;

            // Bottom side (starting from pin 1)
            for i in 0..pins_per_side {
                let x = start_pos + i as f64 * self.pitch_mm;
                builder.add_smd_pad(
                    (i + 1).to_string(),
                    x,
                    -pad_center,
                    pad_width,
                    pad_length,
                    PcbPadShape::Rectangular,
                );
            }

            // Right side
            for i in 0..pins_per_side {
                let y = start_pos + i as f64 * self.pitch_mm;
                builder.add_smd_pad(
                    (pins_per_side + i + 1).to_string(),
                    pad_center,
                    y,
                    pad_length,
                    pad_width,
                    PcbPadShape::Rectangular,
                );
            }

            // Top side (going right to left)
            for i in 0..pins_per_side {
                let x = -start_pos - i as f64 * self.pitch_mm;
                builder.add_smd_pad(
                    (2 * pins_per_side + i + 1).to_string(),
                    x,
                    pad_center,
                    pad_width,
                    pad_length,
                    PcbPadShape::Rectangular,
                );
            }

            // Left side (going down)
            for i in 0..pins_per_side {
                let y = -start_pos - i as f64 * self.pitch_mm;
                builder.add_smd_pad(
                    (3 * pins_per_side + i + 1).to_string(),
                    -pad_center,
                    y,
                    pad_length,
                    pad_width,
                    PcbPadShape::Rectangular,
                );
            }
        }

        // Add silkscreen (body outline, avoiding pads)
        let silk_width = 0.15;
        let body_half_w = self.body_width_mm / 2.0;
        let body_half_l = self.body_length_mm / 2.0;

        // For two-sided, draw top and bottom lines
        if self.sides == 2 {
            builder.add_silkscreen_line(
                -body_half_w,
                body_half_l,
                body_half_w,
                body_half_l,
                silk_width,
            );
            builder.add_silkscreen_line(
                -body_half_w,
                -body_half_l,
                body_half_w,
                -body_half_l,
                silk_width,
            );
        }

        // Pin 1 indicator
        let pin1_x = if self.sides == 2 {
            -pad_center - pad_length
        } else {
            0.0
        };
        let pin1_y = if self.sides == 2 {
            -(self.pin_count as f64 / 4.0 - 0.5) * self.pitch_mm
        } else {
            -pad_center - pad_length
        };
        builder.add_pin1_indicator(pin1_x - 0.3, pin1_y - 0.3, 0.3);

        // Courtyard
        let cy_extent = pad_center + pad_length / 2.0 + courtyard_excess;
        builder.add_courtyard_rect(0.0, 0.0, 2.0 * cy_extent, 2.0 * cy_extent, 0.05);

        builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chip_0805_footprint() {
        let mut det = ();
        let spec = ChipSpec::chip_0805();
        let footprint = spec
            .to_footprint(IpcDensity::Nominal)
            .build_deterministic(&mut det);

        assert_eq!(footprint.pad_count(), 2);
        assert!(footprint.pattern.contains("0805"));
    }

    #[test]
    fn test_soic8_footprint() {
        let mut det = ();
        let spec = GullWingSpec::soic_8();
        let footprint = spec
            .to_footprint(IpcDensity::Nominal)
            .build_deterministic(&mut det);

        assert_eq!(footprint.pad_count(), 8);
        assert!(footprint.pattern.contains("SOIC"));
    }
}

//! SchLib component template for creating schematic library components.
//!
//! The [`SchComponentTemplate`] is the primary input type for creating
//! schematic components. It supports:
//!
//! - Inline pin definitions with smart layout
//! - Automatic body rectangle sizing
//! - Designator prefix inference from component name
//! - Optional explicit graphics (rectangles, lines, polygons)

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{CoordInput, HexColor};
use crate::error::Result;
use crate::io::SchLibComponent;
use crate::records::sch::{
    LineWidth, PinConglomerateFlags, PinElectricalType, PinSymbol, SchComponent, SchDesignator,
    SchImplementationList, SchLabel, SchLine, SchParameter, SchPin, SchPrimitiveBase, SchRectangle,
    SchRecord, SchGraphicalBase,
};

// ═══════════════════════════════════════════════════════════════════════════
// Template Input Types (JSON Schema-compatible)
// ═══════════════════════════════════════════════════════════════════════════

/// Template for creating a schematic library component.
///
/// Most fields are optional - the template system infers sensible defaults:
/// - **name** (required): Component name / library reference
/// - **pins**: Array of pin definitions; auto-positioned if coordinates omitted
/// - **body_width**: Auto-sized to fit pin names if not specified
/// - **designator_prefix**: Inferred from name (e.g., "LM358" → "U")
///
/// # Example (minimal)
/// ```json
/// {
///   "name": "LM358",
///   "description": "Dual Op-Amp",
///   "pins": [
///     { "designator": "1", "name": "OUT_A", "electrical": "output", "side": "left" },
///     { "designator": "2", "name": "IN-_A", "electrical": "input", "side": "left" },
///     { "designator": "3", "name": "IN+_A", "electrical": "input", "side": "left" },
///     { "designator": "4", "name": "GND", "electrical": "power", "side": "bottom" },
///     { "designator": "8", "name": "VCC", "electrical": "power", "side": "top" }
///   ]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SchComponentTemplate {
    /// Component name (LIBREFERENCE). This is the primary identifier.
    pub name: String,

    /// Component description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Designator prefix (e.g., "U?", "R?", "C?"). Inferred from name if not provided.
    /// The "?" suffix is Altium's auto-numbering placeholder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub designator_prefix: Option<String>,

    /// Number of parts (for multi-part components like quad op-amps). Default: 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_count: Option<i32>,

    /// Pin definitions. Pins are auto-positioned if x/y are omitted.
    #[serde(default)]
    pub pins: Vec<PinTemplate>,

    /// Body rectangle width in mils (auto-sized if omitted).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_width: Option<CoordInput>,

    /// Pin length in mils. Default: 200mil.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin_length: Option<CoordInput>,

    /// Pin spacing in mils (vertical distance between pins). Default: 100mil.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin_spacing: Option<CoordInput>,

    /// Explicit rectangles (body outlines, etc.). If omitted and pins are
    /// provided, a body rectangle is auto-generated.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rectangles: Vec<RectangleTemplate>,

    /// Explicit lines.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<LineTemplate>,

    /// Border color for auto-generated body (hex RRGGBB). Default: "800000" (dark red).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_color: Option<HexColor>,

    /// Fill color for auto-generated body (hex RRGGBB). Default: "FFFFB0" (light yellow).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<HexColor>,

    /// If true, skip auto-generating the body rectangle even when no explicit
    /// rectangles are provided.
    #[serde(default)]
    pub no_auto_body: bool,
}

impl Default for SchComponentTemplate {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            designator_prefix: None,
            part_count: None,
            pins: Vec::new(),
            body_width: None,
            pin_length: None,
            pin_spacing: None,
            rectangles: Vec::new(),
            lines: Vec::new(),
            border_color: None,
            fill_color: None,
            no_auto_body: false,
        }
    }
}

/// Template for a single pin.
///
/// All fields except `designator` and `name` are optional with smart defaults.
///
/// # Pin positioning
/// If `x` and `y` are omitted, pins are auto-positioned based on `side`:
/// - **left** pins: placed at the left edge, pointing right
/// - **right** pins: placed at the right edge, pointing left
/// - **top** pins: placed at the top, pointing down
/// - **bottom** pins: placed at the bottom, pointing up
///
/// Pins without a `side` default to "left" for inputs and "right" for outputs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PinTemplate {
    /// Pin designator (e.g., "1", "2", "A1"). Required.
    pub designator: String,

    /// Pin name (e.g., "VCC", "GND", "DATA0"). Required.
    pub name: String,

    /// Electrical type. Default: "passive".
    ///
    /// Accepted values: "input", "output", "io", "passive", "power",
    /// "oc" (open collector), "oe" (open emitter), "hiz" (high impedance).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub electrical: Option<String>,

    /// Which side of the body the pin connects to.
    ///
    /// Accepted values: "left", "right", "top", "bottom".
    /// Default: inferred from electrical type (inputs→left, outputs→right,
    /// power→top/bottom).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,

    /// Explicit X position (mils or string with unit). Overrides auto-positioning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<CoordInput>,

    /// Explicit Y position (mils or string with unit). Overrides auto-positioning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<CoordInput>,

    /// Pin orientation: "left", "right", "up", "down".
    /// Default: inferred from `side`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<String>,

    /// Pin length (mils). Default: inherits from component template.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<CoordInput>,

    /// Hide the pin. Default: false.
    #[serde(default)]
    pub hidden: bool,

    /// Pin description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Symbol on inner edge. Default: "none".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_inner: Option<String>,

    /// Symbol on outer edge (e.g., "dot" for active-low). Default: "none".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_outer: Option<String>,

    /// Symbol inside (e.g., "clock"). Default: "none".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_inside: Option<String>,

    /// Symbol outside. Default: "none".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_outside: Option<String>,
}

impl Default for PinTemplate {
    fn default() -> Self {
        Self {
            designator: String::new(),
            name: String::new(),
            electrical: None,
            side: None,
            x: None,
            y: None,
            orientation: None,
            length: None,
            hidden: false,
            description: None,
            symbol_inner: None,
            symbol_outer: None,
            symbol_inside: None,
            symbol_outside: None,
        }
    }
}

/// Template for an explicit rectangle.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RectangleTemplate {
    /// Corner 1 X (mils or string with unit).
    pub x1: CoordInput,
    /// Corner 1 Y.
    pub y1: CoordInput,
    /// Corner 2 X.
    pub x2: CoordInput,
    /// Corner 2 Y.
    pub y2: CoordInput,
    /// Whether the rectangle is filled. Default: true.
    #[serde(default = "default_true")]
    pub filled: bool,
    /// Fill color (hex RRGGBB). Default: "FFFFB0".
    #[serde(default = "default_fill_color")]
    pub fill_color: HexColor,
    /// Border color (hex RRGGBB). Default: "800000".
    #[serde(default = "default_border_color")]
    pub border_color: HexColor,
}

/// Template for an explicit line.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LineTemplate {
    /// Start X (mils or string with unit).
    pub x1: CoordInput,
    /// Start Y.
    pub y1: CoordInput,
    /// End X.
    pub x2: CoordInput,
    /// End Y.
    pub y2: CoordInput,
    /// Line color (hex RRGGBB). Default: "800000".
    #[serde(default = "default_border_color")]
    pub color: HexColor,
}

fn default_true() -> bool {
    true
}
fn default_fill_color() -> HexColor {
    HexColor("FFFFB0".to_string())
}
fn default_border_color() -> HexColor {
    HexColor("800000".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// Template Application Logic
// ═══════════════════════════════════════════════════════════════════════════

impl SchComponentTemplate {
    /// Apply this template to produce an `SchLibComponent` ready to insert
    /// into a SchLib file.
    pub fn apply(&self) -> Result<SchLibComponent> {
        let pin_length_mils = self.pin_length.as_ref().map(|c| c.to_mils()).unwrap_or(200.0);
        let pin_spacing_mils = self.pin_spacing.as_ref().map(|c| c.to_mils()).unwrap_or(100.0);

        // Classify pins by side
        let mut left_pins = Vec::new();
        let mut right_pins = Vec::new();
        let mut top_pins = Vec::new();
        let mut bottom_pins = Vec::new();

        for pin in &self.pins {
            let side = pin.effective_side();
            match side.as_str() {
                "left" => left_pins.push(pin),
                "right" => right_pins.push(pin),
                "top" => top_pins.push(pin),
                "bottom" => bottom_pins.push(pin),
                _ => left_pins.push(pin), // fallback
            }
        }

        let max_vertical = left_pins.len().max(right_pins.len());
        let max_horizontal = top_pins.len().max(bottom_pins.len());

        // Calculate body dimensions
        let body_height_mils = (max_vertical + 1) as f64 * pin_spacing_mils;
        let min_width_for_tb = if max_horizontal > 0 {
            (max_horizontal + 1) as f64 * pin_spacing_mils
        } else {
            0.0
        };
        let default_body_width = 800.0_f64.max(min_width_for_tb);
        let body_width_mils = self
            .body_width
            .as_ref()
            .map(|c| c.to_mils().max(min_width_for_tb))
            .unwrap_or(default_body_width);

        // Create component record
        let component = SchComponent {
            lib_reference: self.name.clone(),
            component_description: self.description.clone().unwrap_or_default(),
            part_count: self.part_count.unwrap_or(1),
            display_mode_count: 1,
            current_part_id: 1,
            ..Default::default()
        };

        let mut primitives = vec![SchRecord::Component(component.clone())];

        // Designator record
        let designator_text = self
            .designator_prefix
            .clone()
            .unwrap_or_else(|| infer_designator_prefix(&self.name, self.description.as_deref()));
        let designator = SchDesignator {
            param: SchParameter {
                label: SchLabel {
                    text: designator_text,
                    font_id: 1,
                    ..Default::default()
                },
                name: "Designator".to_string(),
                read_only_state: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        primitives.push(SchRecord::Designator(designator));

        // Implementation list
        let impl_list = SchImplementationList {
            base: SchPrimitiveBase {
                owner_index: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        primitives.push(SchRecord::ImplementationList(impl_list));

        // Body rectangle (auto-generated or explicit)
        if !self.no_auto_body && self.rectangles.is_empty() && !self.pins.is_empty() {
            let border = self
                .border_color
                .as_ref()
                .and_then(|c| c.to_altium_color().ok())
                .unwrap_or(0x000080); // Dark red in BGR
            let fill = self
                .fill_color
                .as_ref()
                .and_then(|c| c.to_altium_color().ok())
                .unwrap_or(0xB0FFFF); // Light yellow in BGR

            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(1);
            graphical.location_x = CoordInput::from_mils(0.0).to_raw();
            graphical.location_y = CoordInput::from_mils(0.0).to_raw();
            graphical.color = border;
            graphical.area_color = fill;

            let rect = SchRectangle {
                graphical,
                corner_x: CoordInput::from_mils(body_width_mils).to_raw(),
                corner_y: CoordInput::from_mils(body_height_mils).to_raw(),
                line_width: LineWidth::Small,
                is_solid: true,
                transparent: false,
                ..Default::default()
            };
            primitives.push(SchRecord::Rectangle(rect));
        }

        // Explicit rectangles
        for rect_tmpl in &self.rectangles {
            let border = rect_tmpl
                .border_color
                .to_altium_color()
                .unwrap_or(0x000080);
            let fill = rect_tmpl
                .fill_color
                .to_altium_color()
                .unwrap_or(0xB0FFFF);

            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(1);
            graphical.location_x = rect_tmpl.x1.to_raw();
            graphical.location_y = rect_tmpl.y1.to_raw();
            graphical.color = border;
            graphical.area_color = fill;

            let rect = SchRectangle {
                graphical,
                corner_x: rect_tmpl.x2.to_raw(),
                corner_y: rect_tmpl.y2.to_raw(),
                line_width: LineWidth::Small,
                is_solid: rect_tmpl.filled,
                transparent: !rect_tmpl.filled,
                ..Default::default()
            };
            primitives.push(SchRecord::Rectangle(rect));
        }

        // Explicit lines
        for line_tmpl in &self.lines {
            let color = line_tmpl.color.to_altium_color().unwrap_or(0x000080);

            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(1);
            graphical.location_x = line_tmpl.x1.to_raw();
            graphical.location_y = line_tmpl.y1.to_raw();
            graphical.color = color;

            let line = SchLine {
                graphical,
                corner_x: line_tmpl.x2.to_raw(),
                corner_y: line_tmpl.y2.to_raw(),
                ..Default::default()
            };
            primitives.push(SchRecord::Line(line));
        }

        // Create pins
        let create_pin = |pin_tmpl: &PinTemplate,
                          x_mils: f64,
                          y_mils: f64,
                          conglomerate: PinConglomerateFlags|
         -> Result<SchPin> {
            let length = pin_tmpl
                .length
                .as_ref()
                .map(|c| c.to_mils())
                .unwrap_or(pin_length_mils);

            let electrical = match pin_tmpl.electrical.as_deref() {
                Some(s) => parse_electrical_type(s).map_err(|e| {
                    crate::error::AltiumError::Template(format!(
                        "pin '{}': {}", pin_tmpl.designator, e
                    ))
                })?,
                None => PinElectricalType::Passive,
            };

            // Use explicit orientation if set, otherwise use side-inferred conglomerate
            let orientation_flags = pin_tmpl
                .orientation
                .as_deref()
                .map(|o| match o.to_lowercase().as_str() {
                    "right" => PinConglomerateFlags::NONE,
                    "left" => PinConglomerateFlags::FLIPPED,
                    "down" => PinConglomerateFlags::ROTATED,
                    "up" => PinConglomerateFlags::ROTATED | PinConglomerateFlags::FLIPPED,
                    _ => conglomerate,
                })
                .unwrap_or(conglomerate);

            let mut flags = orientation_flags;
            flags |= PinConglomerateFlags::DISPLAY_NAME_VISIBLE;
            flags |= PinConglomerateFlags::DESIGNATOR_VISIBLE;
            if pin_tmpl.hidden {
                flags |= PinConglomerateFlags::HIDE;
            }

            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(1);
            graphical.location_x = CoordInput::from_mils(x_mils).to_raw();
            graphical.location_y = CoordInput::from_mils(y_mils).to_raw();
            graphical.color = 0x000080;

            Ok(SchPin {
                graphical,
                designator: pin_tmpl.designator.clone(),
                name: pin_tmpl.name.clone(),
                electrical,
                pin_conglomerate: flags,
                pin_length: CoordInput::from_mils(length).to_raw(),
                description: pin_tmpl.description.clone().unwrap_or_default(),
                symbol_inner_edge: pin_tmpl
                    .symbol_inner
                    .as_deref()
                    .map(parse_pin_symbol)
                    .unwrap_or(PinSymbol::None),
                symbol_outer_edge: pin_tmpl
                    .symbol_outer
                    .as_deref()
                    .map(parse_pin_symbol)
                    .unwrap_or(PinSymbol::None),
                symbol_inside: pin_tmpl
                    .symbol_inside
                    .as_deref()
                    .map(parse_pin_symbol)
                    .unwrap_or(PinSymbol::None),
                symbol_outside: pin_tmpl
                    .symbol_outside
                    .as_deref()
                    .map(parse_pin_symbol)
                    .unwrap_or(PinSymbol::None),
                ..Default::default()
            })
        };

        // Left pins (pointing right into body)
        for (i, pin_tmpl) in left_pins.iter().enumerate() {
            let (x, y) = if let (Some(px), Some(py)) = (&pin_tmpl.x, &pin_tmpl.y) {
                (px.to_mils(), py.to_mils())
            } else {
                let y = body_height_mils - (i + 1) as f64 * pin_spacing_mils;
                (-pin_length_mils, y)
            };
            let pin = create_pin(pin_tmpl, x, y, PinConglomerateFlags::NONE)?;
            primitives.push(SchRecord::Pin(pin));
        }

        // Right pins (pointing left into body)
        for (i, pin_tmpl) in right_pins.iter().enumerate() {
            let (x, y) = if let (Some(px), Some(py)) = (&pin_tmpl.x, &pin_tmpl.y) {
                (px.to_mils(), py.to_mils())
            } else {
                let y = body_height_mils - (i + 1) as f64 * pin_spacing_mils;
                (body_width_mils + pin_length_mils, y)
            };
            let pin = create_pin(pin_tmpl, x, y, PinConglomerateFlags::FLIPPED)?;
            primitives.push(SchRecord::Pin(pin));
        }

        // Top pins (pointing down into body)
        for (i, pin_tmpl) in top_pins.iter().enumerate() {
            let (x, y) = if let (Some(px), Some(py)) = (&pin_tmpl.x, &pin_tmpl.y) {
                (px.to_mils(), py.to_mils())
            } else {
                let x = (i + 1) as f64 * pin_spacing_mils;
                (x, body_height_mils + pin_length_mils)
            };
            let pin = create_pin(pin_tmpl, x, y, PinConglomerateFlags::ROTATED)?;
            primitives.push(SchRecord::Pin(pin));
        }

        // Bottom pins (pointing up into body)
        for (i, pin_tmpl) in bottom_pins.iter().enumerate() {
            let (x, y) = if let (Some(px), Some(py)) = (&pin_tmpl.x, &pin_tmpl.y) {
                (px.to_mils(), py.to_mils())
            } else {
                let x = (i + 1) as f64 * pin_spacing_mils;
                (x, -pin_length_mils)
            };
            let pin = create_pin(
                pin_tmpl,
                x,
                y,
                PinConglomerateFlags::ROTATED | PinConglomerateFlags::FLIPPED,
            )?;
            primitives.push(SchRecord::Pin(pin));
        }

        Ok(SchLibComponent {
            component,
            primitives,
        })
    }
}

impl PinTemplate {
    /// Determine the effective side for this pin, inferring from electrical type
    /// when not explicitly set.
    pub fn effective_side(&self) -> String {
        if let Some(ref side) = self.side {
            return side.to_lowercase();
        }

        // Infer from electrical type
        let electrical = self.electrical.as_deref().unwrap_or("passive");
        match electrical.to_lowercase().as_str() {
            "input" => "left".to_string(),
            "output" => "right".to_string(),
            "io" | "inputoutput" => "left".to_string(),
            "power" => {
                // Power pins: VCC/VDD-like go to top, GND/VSS-like go to bottom
                let name_upper = self.name.to_uppercase();
                if name_upper.contains("GND")
                    || name_upper.contains("VSS")
                    || name_upper.contains("VEE")
                {
                    "bottom".to_string()
                } else {
                    "top".to_string()
                }
            }
            _ => "left".to_string(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Helper Functions
// ═══════════════════════════════════════════════════════════════════════════

fn parse_electrical_type(s: &str) -> std::result::Result<PinElectricalType, String> {
    match s.to_lowercase().as_str() {
        "input" | "in" => Ok(PinElectricalType::Input),
        "output" | "out" => Ok(PinElectricalType::Output),
        "io" | "inputoutput" | "bidirectional" => Ok(PinElectricalType::InputOutput),
        "passive" | "pas" => Ok(PinElectricalType::Passive),
        "power" | "pwr" => Ok(PinElectricalType::Power),
        "oc" | "opencollector" | "open_collector" => Ok(PinElectricalType::OpenCollector),
        "oe" | "openemitter" | "open_emitter" => Ok(PinElectricalType::OpenEmitter),
        "hiz" | "hi_z" | "tristate" => Ok(PinElectricalType::HiZ),
        _ => Err(format!(
            "unknown electrical type '{}'. Valid values: input, output, io, passive, power, oc, oe, hiz",
            s
        )),
    }
}

fn parse_pin_symbol(s: &str) -> PinSymbol {
    match s.to_lowercase().as_str() {
        "none" | "" => PinSymbol::None,
        "dot" => PinSymbol::Dot,
        "clock" | "clk" => PinSymbol::Clock,
        "active_low_input" | "activellowinput" => PinSymbol::ActiveLowInput,
        "active_low_output" | "activelowoutput" => PinSymbol::ActiveLowOutput,
        "schmitt" => PinSymbol::Schmitt,
        "open_collector" | "opencollector" => PinSymbol::OpenCollector,
        "open_emitter" | "openemitter" => PinSymbol::OpenEmitter,
        _ => PinSymbol::None,
    }
}

/// Infer a designator prefix from a component name and description.
///
/// Uses common naming conventions:
/// - Names containing "resistor", "R_" → "R"
/// - Names containing "capacitor", "C_" → "C"
/// - Names containing op-amp patterns → "U"
/// - Generic ICs → "U"
/// - Connectors → "J"
/// - etc.
fn infer_designator_prefix(name: &str, description: Option<&str>) -> String {
    let n = name.to_uppercase();
    let d = description.unwrap_or("").to_uppercase();

    // Check for common passive prefixes
    if n.starts_with("R_") || n.starts_with("R ") || d.contains("RESISTOR") {
        return "R?".to_string();
    }
    if n.starts_with("C_") || n.starts_with("C ") || d.contains("CAPACITOR") {
        return "C?".to_string();
    }
    if n.starts_with("L_") || n.starts_with("L ") || d.contains("INDUCTOR") {
        return "L?".to_string();
    }
    if n.starts_with("D_") || n.starts_with("D ") || d.contains("DIODE") || d.contains("LED") {
        return "D?".to_string();
    }
    if d.contains("TRANSISTOR") || n.starts_with("Q_") || d.contains("MOSFET") || d.contains("BJT")
    {
        return "Q?".to_string();
    }
    if d.contains("CONNECTOR") || n.starts_with("CONN") || n.starts_with("J_") {
        return "J?".to_string();
    }
    if d.contains("CRYSTAL") || d.contains("OSCILLATOR") || n.starts_with("Y_") {
        return "Y?".to_string();
    }
    if d.contains("FUSE") || n.starts_with("F_") {
        return "F?".to_string();
    }
    if d.contains("TRANSFORMER") || n.starts_with("T_") {
        return "T?".to_string();
    }

    // Default to U for ICs
    "U?".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_template() {
        let template = SchComponentTemplate {
            name: "TestComponent".to_string(),
            ..Default::default()
        };
        let result = template.apply().unwrap();
        assert_eq!(result.component.lib_reference, "TestComponent");
    }

    #[test]
    fn test_template_with_pins() {
        let template = SchComponentTemplate {
            name: "LM358".to_string(),
            description: Some("Dual Op-Amp".to_string()),
            pins: vec![
                PinTemplate {
                    designator: "1".to_string(),
                    name: "OUT_A".to_string(),
                    electrical: Some("output".to_string()),
                    side: Some("right".to_string()),
                    ..Default::default()
                },
                PinTemplate {
                    designator: "2".to_string(),
                    name: "IN-_A".to_string(),
                    electrical: Some("input".to_string()),
                    side: Some("left".to_string()),
                    ..Default::default()
                },
                PinTemplate {
                    designator: "4".to_string(),
                    name: "GND".to_string(),
                    electrical: Some("power".to_string()),
                    side: Some("bottom".to_string()),
                    ..Default::default()
                },
                PinTemplate {
                    designator: "8".to_string(),
                    name: "VCC".to_string(),
                    electrical: Some("power".to_string()),
                    side: Some("top".to_string()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let result = template.apply().unwrap();
        assert_eq!(result.component.lib_reference, "LM358");

        // Count pins
        let pin_count = result
            .primitives
            .iter()
            .filter(|p| matches!(p, SchRecord::Pin(_)))
            .count();
        assert_eq!(pin_count, 4);

        // Should have auto-generated body rectangle
        let rect_count = result
            .primitives
            .iter()
            .filter(|p| matches!(p, SchRecord::Rectangle(_)))
            .count();
        assert_eq!(rect_count, 1);
    }

    #[test]
    fn test_pin_side_inference() {
        let pin = PinTemplate {
            designator: "1".to_string(),
            name: "DATA".to_string(),
            electrical: Some("input".to_string()),
            ..Default::default()
        };
        assert_eq!(pin.effective_side(), "left");

        let pin = PinTemplate {
            designator: "2".to_string(),
            name: "OUT".to_string(),
            electrical: Some("output".to_string()),
            ..Default::default()
        };
        assert_eq!(pin.effective_side(), "right");

        let pin = PinTemplate {
            designator: "3".to_string(),
            name: "GND".to_string(),
            electrical: Some("power".to_string()),
            ..Default::default()
        };
        assert_eq!(pin.effective_side(), "bottom");

        let pin = PinTemplate {
            designator: "4".to_string(),
            name: "VCC".to_string(),
            electrical: Some("power".to_string()),
            ..Default::default()
        };
        assert_eq!(pin.effective_side(), "top");
    }

    #[test]
    fn test_designator_inference() {
        assert_eq!(infer_designator_prefix("R_100K", None), "R?");
        assert_eq!(infer_designator_prefix("C_100nF", None), "C?");
        assert_eq!(infer_designator_prefix("LM358", Some("Dual Op-Amp")), "U?");
        assert_eq!(
            infer_designator_prefix("CONN_01x04", Some("Connector")),
            "J?"
        );
    }

    #[test]
    fn test_json_roundtrip() {
        let template = SchComponentTemplate {
            name: "TEST".to_string(),
            pins: vec![PinTemplate {
                designator: "1".to_string(),
                name: "VCC".to_string(),
                electrical: Some("power".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };

        let json = serde_json::to_string(&template).unwrap();
        let parsed: SchComponentTemplate = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "TEST");
        assert_eq!(parsed.pins.len(), 1);
    }

    #[test]
    fn test_json_schema_generation() {
        let schema = schemars::schema_for!(SchComponentTemplate);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(json.contains("SchComponentTemplate"));
        assert!(json.contains("PinTemplate"));
    }
}

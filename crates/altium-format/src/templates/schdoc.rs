//! SchDoc placement template for adding elements to schematic documents.
//!
//! The [`SchDocPlacementTemplate`] supports placing multiple elements
//! in a single operation:
//! - Components (from library or inline definitions)
//! - Wires with vertices
//! - Net labels
//! - Power ports
//! - Junctions
//! - Ports

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::CoordInput;

// ═══════════════════════════════════════════════════════════════════════════
// Template Input Types
// ═══════════════════════════════════════════════════════════════════════════

/// Template for placing elements on a schematic document.
///
/// All coordinate values are in mils by default (standard for schematics).
/// This template allows placing multiple elements in a single operation,
/// which is useful for agents that need to build up a schematic incrementally.
///
/// # Example
/// ```json
/// {
///   "components": [
///     {
///       "lib_reference": "LM358",
///       "designator": "U1",
///       "x": 2000,
///       "y": 1500
///     }
///   ],
///   "wires": [
///     { "vertices": [[1000, 1500], [2000, 1500]] }
///   ],
///   "net_labels": [
///     { "name": "VCC", "x": 1500, "y": 2000 }
///   ],
///   "power_ports": [
///     { "name": "GND", "x": 2000, "y": 500, "style": "ground" }
///   ]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SchDocPlacementTemplate {
    /// Components to place on the schematic.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<ComponentPlacement>,

    /// Wires to add.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wires: Vec<WirePlacement>,

    /// Net labels to place.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub net_labels: Vec<NetLabelPlacement>,

    /// Power ports to place.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub power_ports: Vec<PowerPortPlacement>,

    /// Junctions to place.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub junctions: Vec<JunctionPlacement>,

    /// Ports to place.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<PortPlacement>,

    /// Whether to auto-add junctions where wires cross. Default: false.
    #[serde(default)]
    pub auto_junctions: bool,
}

impl Default for SchDocPlacementTemplate {
    fn default() -> Self {
        Self {
            components: Vec::new(),
            wires: Vec::new(),
            net_labels: Vec::new(),
            power_ports: Vec::new(),
            junctions: Vec::new(),
            ports: Vec::new(),
            auto_junctions: false,
        }
    }
}

/// Place a component on the schematic.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComponentPlacement {
    /// Library reference name (must exist in a loaded library).
    pub lib_reference: String,

    /// Designator (e.g., "U1", "R3"). Auto-assigned if omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub designator: Option<String>,

    /// X position (mils).
    pub x: CoordInput,
    /// Y position (mils).
    pub y: CoordInput,

    /// Orientation: "normal", "rotated_90", "rotated_180", "rotated_270".
    /// Default: "normal".
    #[serde(default = "default_orientation")]
    pub orientation: String,

    /// Library path to load the component from (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_path: Option<String>,
}

/// Place a wire on the schematic.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WirePlacement {
    /// Wire vertices as [x, y] pairs (mils). Minimum 2 points.
    pub vertices: Vec<[CoordInput; 2]>,
}

/// Place a net label.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NetLabelPlacement {
    /// Net name.
    pub name: String,
    /// X position (mils).
    pub x: CoordInput,
    /// Y position (mils).
    pub y: CoordInput,
    /// Orientation: "horizontal", "vertical". Default: "horizontal".
    #[serde(default = "default_orientation")]
    pub orientation: String,
}

/// Place a power port.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PowerPortPlacement {
    /// Net name (e.g., "VCC", "GND", "+3.3V").
    pub name: String,
    /// X position (mils).
    pub x: CoordInput,
    /// Y position (mils).
    pub y: CoordInput,
    /// Power port style: "arrow", "bar", "wave", "ground", "power_ground",
    /// "signal_ground", "earth_ground", "circle".
    /// Default: inferred from name (GND→"ground", VCC→"arrow").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Orientation: "up", "down", "left", "right". Default: inferred from style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<String>,
}

/// Place a junction.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JunctionPlacement {
    /// X position (mils).
    pub x: CoordInput,
    /// Y position (mils).
    pub y: CoordInput,
}

/// Place a port.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PortPlacement {
    /// Port name.
    pub name: String,
    /// X position (mils).
    pub x: CoordInput,
    /// Y position (mils).
    pub y: CoordInput,
    /// IO type: "unspecified", "input", "output", "io". Default: "unspecified".
    #[serde(default = "default_io_type")]
    pub io_type: String,
}

fn default_orientation() -> String {
    "normal".to_string()
}

fn default_io_type() -> String {
    "unspecified".to_string()
}

impl PowerPortPlacement {
    /// Infer the power port style from the net name.
    pub fn effective_style(&self) -> String {
        if let Some(ref style) = self.style {
            return style.clone();
        }

        let name_upper = self.name.to_uppercase();
        if name_upper.contains("GND") || name_upper.contains("VSS") || name_upper.contains("VEE")
        {
            "ground".to_string()
        } else {
            "arrow".to_string()
        }
    }

    /// Infer orientation from style.
    pub fn effective_orientation(&self) -> String {
        if let Some(ref orient) = self.orientation {
            return orient.clone();
        }

        let style = self.effective_style();
        match style.as_str() {
            "ground" | "power_ground" | "signal_ground" | "earth_ground" => "down".to_string(),
            _ => "up".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_port_inference() {
        let pp = PowerPortPlacement {
            name: "GND".to_string(),
            x: CoordInput(100.0),
            y: CoordInput(200.0),
            style: None,
            orientation: None,
        };
        assert_eq!(pp.effective_style(), "ground");
        assert_eq!(pp.effective_orientation(), "down");

        let pp = PowerPortPlacement {
            name: "VCC".to_string(),
            x: CoordInput(100.0),
            y: CoordInput(200.0),
            style: None,
            orientation: None,
        };
        assert_eq!(pp.effective_style(), "arrow");
        assert_eq!(pp.effective_orientation(), "up");
    }

    #[test]
    fn test_json_roundtrip() {
        let template = SchDocPlacementTemplate {
            components: vec![ComponentPlacement {
                lib_reference: "LM358".to_string(),
                designator: Some("U1".to_string()),
                x: CoordInput(2000.0),
                y: CoordInput(1500.0),
                orientation: "normal".to_string(),
                library_path: None,
            }],
            wires: vec![WirePlacement {
                vertices: vec![
                    [CoordInput(1000.0), CoordInput(1500.0)],
                    [CoordInput(2000.0), CoordInput(1500.0)],
                ],
            }],
            ..Default::default()
        };

        let json = serde_json::to_string(&template).unwrap();
        let parsed: SchDocPlacementTemplate = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.components.len(), 1);
        assert_eq!(parsed.wires.len(), 1);
    }

    #[test]
    fn test_json_schema_generation() {
        let schema = schemars::schema_for!(SchDocPlacementTemplate);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(json.contains("SchDocPlacementTemplate"));
        assert!(json.contains("ComponentPlacement"));
        assert!(json.contains("WirePlacement"));
    }
}

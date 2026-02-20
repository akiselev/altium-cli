// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Schematic document operations (v2).
//!
//! Provides high-level operations for exploring and analyzing Altium schematic
//! document (.SchDoc) files using the v2 public API.
//!
//! This module uses ONLY the public API of `altium_format::documents::schdoc::SchDoc`.
//! No internal backing-store types (`ParamOrigin`, `BinaryOrigin`, `ComponentGroup`)
//! are accessed directly.

mod browse;
mod detail;
mod json;

pub use browse::*;
pub use detail::*;
pub use json::*;

use std::collections::HashMap;
use std::path::Path;

use altium_format::coord::{AltiumCoord, SchCoord};
use altium_format::documents::schdoc::SchDoc;
use altium_format::records::{SchNetLabelRecord, SchPowerRecord};

use crate::helpers::*;

/// Opens and parses a SchDoc file from the given path.
pub(super) fn open_schdoc(path: &Path) -> Result<SchDoc, Box<dyn std::error::Error>> {
    Ok(SchDoc::open_file(path).map_err(|e| e.to_string())?)
}

/// Format a coordinate pair as a display string.
pub(super) fn format_location(x: SchCoord, y: SchCoord) -> String {
    format!("({:.1}, {:.1})", x.to_mils(), y.to_mils())
}

/// Decode sheet size style integer to a human-readable name.
pub(super) fn sheet_size_name(style: i32) -> &'static str {
    match style {
        0 => "A4",
        1 => "A3",
        2 => "A2",
        3 => "A1",
        4 => "A0",
        5 => "A",
        6 => "B",
        7 => "C",
        8 => "D",
        9 => "E",
        10 => "Letter",
        11 => "Legal",
        12 => "Tabloid",
        13 => "OrCAD A",
        14 => "OrCAD B",
        15 => "OrCAD C",
        16 => "OrCAD D",
        17 => "OrCAD E",
        _ => "Custom",
    }
}

/// Decode power port style integer to a human-readable name.
pub(super) fn power_style_name(style: i32) -> &'static str {
    match style {
        0 => "Circle",
        1 => "Arrow",
        2 => "Bar",
        3 => "Wave",
        4 => "Power Ground",
        5 => "Signal Ground",
        6 => "Earth",
        7 => "GND Power",
        _ => "Unknown",
    }
}

/// Decode port I/O type integer to a human-readable name.
pub(super) fn port_io_type_name(io_type: i32) -> &'static str {
    match io_type {
        0 => "Unspecified",
        1 => "Output",
        2 => "Input",
        3 => "Bidirectional",
        _ => "Unspecified",
    }
}

/// Extract the sheet size string from the document.
pub(super) fn get_sheet_size(doc: &SchDoc) -> String {
    if let Some(rec) = doc.sheet_record() {
        sheet_size_name(rec.sheet_style() as i32).to_string()
    } else {
        "Unknown".to_string()
    }
}

/// Collect all unique net names from net labels (RECORD=25).
pub(super) fn collect_net_names(doc: &SchDoc) -> Vec<String> {
    let mut nets: HashMap<String, bool> = HashMap::new();
    doc.for_each_record_of_type(25, |node| {
        let rec = SchNetLabelRecord::from_origin(node.origin.clone());
        let text = rec.text();
        if !text.is_empty() {
            nets.insert(text, true);
        }
    });
    let mut result: Vec<String> = nets.into_keys().collect();
    result.sort_by(|a, b| alphanumeric_sort(a, b));
    result
}

/// Collect all unique power net names from power port records (RECORD=17).
pub(super) fn collect_power_nets(doc: &SchDoc) -> Vec<String> {
    let mut nets: HashMap<String, bool> = HashMap::new();
    doc.for_each_record_of_type(17, |node| {
        let rec = SchPowerRecord::from_origin(node.origin.clone());
        let text = rec.text();
        if !text.is_empty() {
            nets.insert(text, true);
        }
    });
    let mut result: Vec<String> = nets.into_keys().collect();
    result.sort_by(|a, b| alphanumeric_sort(a, b));
    result
}

/// Check if a net name looks like a power rail.
pub(super) fn is_power_rail(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("vcc")
        || lower.starts_with("vdd")
        || lower.starts_with("v3")
        || lower.starts_with("v5")
        || lower.starts_with("v1")
        || lower.starts_with('+')
        || lower.contains("pwr")
        || lower.contains("supply")
        || lower.contains("vin")
        || lower.contains("vout")
        || lower.contains("vbat")
        || lower.contains("vbus")
        || lower.contains("avcc")
        || lower.contains("dvcc")
        || lower.contains("avdd")
        || lower.contains("dvdd")
}

/// Check if a net name looks like a ground reference.
pub(super) fn is_ground_net(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower == "gnd"
        || lower == "agnd"
        || lower == "dgnd"
        || lower == "pgnd"
        || lower == "vss"
        || lower == "avss"
        || lower == "dvss"
        || lower.starts_with("gnd")
        || lower.contains("ground")
}

/// Check if a net name looks like a data bus signal.
pub(super) fn is_data_bus(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("d0")
        || lower.starts_with("d[")
        || lower.starts_with("data")
        || lower.starts_with("sd")
        || lower.starts_with("sda")
        || lower.starts_with("mosi")
        || lower.starts_with("miso")
}

/// Check if a net name looks like an address bus signal.
pub(super) fn is_address_bus(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("a0") || lower.starts_with("a[") || lower.starts_with("addr")
}

/// Check if a net name looks like a control signal.
pub(super) fn is_control_signal(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("cs")
        || lower.contains("ce")
        || lower.contains("we")
        || lower.contains("oe")
        || lower.contains("rd")
        || lower.contains("wr")
        || lower.contains("sel")
        || lower.contains("strobe")
}

#[cfg(test)]
mod tests {
    use super::*;
    use altium_format::coord::SchCoord;

    #[test]
    fn test_schcoord_to_mils() {
        assert!((SchCoord::from_raw(10000000).to_mils() - 100.0).abs() < 0.001);
        assert!((SchCoord::from_raw(0).to_mils() - 0.0).abs() < 0.001);
        assert!((SchCoord::from_raw(-5000000).to_mils() - (-50.0)).abs() < 0.001);
    }

    #[test]
    fn test_sheet_size_name() {
        assert_eq!(sheet_size_name(0), "A4");
        assert_eq!(sheet_size_name(5), "A");
        assert_eq!(sheet_size_name(10), "Letter");
        assert_eq!(sheet_size_name(99), "Custom");
    }

    #[test]
    fn test_power_style_name() {
        assert_eq!(power_style_name(0), "Circle");
        assert_eq!(power_style_name(4), "Power Ground");
        assert_eq!(power_style_name(99), "Unknown");
    }

    #[test]
    fn test_port_io_type_name() {
        assert_eq!(port_io_type_name(0), "Unspecified");
        assert_eq!(port_io_type_name(1), "Output");
        assert_eq!(port_io_type_name(2), "Input");
        assert_eq!(port_io_type_name(3), "Bidirectional");
    }

    #[test]
    fn test_is_power_rail() {
        assert!(is_power_rail("VCC"));
        assert!(is_power_rail("VDD"));
        assert!(is_power_rail("+3V3"));
        assert!(is_power_rail("VBAT"));
        assert!(!is_power_rail("GND"));
        assert!(!is_power_rail("SDA"));
    }

    #[test]
    fn test_is_ground_net() {
        assert!(is_ground_net("GND"));
        assert!(is_ground_net("AGND"));
        assert!(is_ground_net("DGND"));
        assert!(is_ground_net("VSS"));
        assert!(!is_ground_net("VCC"));
        assert!(!is_ground_net("CLK"));
    }

    #[test]
    fn test_is_data_bus() {
        assert!(is_data_bus("D0"));
        assert!(is_data_bus("D[7:0]"));
        assert!(is_data_bus("DATA_IN"));
        assert!(is_data_bus("SDA"));
        assert!(is_data_bus("MOSI"));
        assert!(!is_data_bus("CLK"));
    }

    #[test]
    fn test_is_address_bus() {
        assert!(is_address_bus("A0"));
        assert!(is_address_bus("A[15:0]"));
        assert!(is_address_bus("ADDR0"));
        assert!(!is_address_bus("DATA"));
    }

    #[test]
    fn test_is_control_signal() {
        assert!(is_control_signal("CS"));
        assert!(is_control_signal("nWE"));
        assert!(is_control_signal("STROBE"));
        assert!(!is_control_signal("VCC"));
    }

    #[test]
    fn test_format_location() {
        assert_eq!(
            format_location(SchCoord::from_raw(10000000), SchCoord::from_raw(20000000)),
            "(100.0, 200.0)"
        );
        assert_eq!(
            format_location(SchCoord::from_raw(0), SchCoord::from_raw(0)),
            "(0.0, 0.0)"
        );
    }
}

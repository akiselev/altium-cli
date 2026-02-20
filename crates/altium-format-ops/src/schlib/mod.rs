// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Schematic library operations (v2).
//!
//! Provides high-level operations for exploring and manipulating Altium schematic
//! library (.SchLib) files using only the public API of `altium_format`.

mod browse;
mod detail;
mod json;
mod mutate;

pub use browse::*;
pub use detail::*;
pub use json::*;
pub use mutate::*;

use std::collections::HashMap;
use std::path::Path;

use altium_format::coord::{AltiumCoord, SchCoord};
use altium_format::documents::schlib::SchLib;
use altium_format::handles::SchComponentHandle;

use crate::helpers::*;

/// Opens and parses a SchLib file from the given path.
pub(super) fn open_schlib(path: &Path) -> Result<SchLib, Box<dyn std::error::Error>> {
    Ok(SchLib::open_file(path).map_err(|e| e.to_string())?)
}

/// Convert schematic coordinates to mils for display.
pub(super) fn coord_to_mils(value: SchCoord) -> String {
    format!("{:.1}", value.to_mils())
}

/// Count primitives by type name using the component handle.
pub(super) fn count_primitives(comp: &SchComponentHandle) -> HashMap<&'static str, usize> {
    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    for (type_id, _) in comp.all_children() {
        let name = sch_record_type_name(type_id);
        *counts.entry(name).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use crate::categorization::categorize_component;
    use crate::helpers::*;
    use altium_format::records::enums::PinElectricalType;

    #[test]
    fn test_categorize_component() {
        assert_eq!(
            categorize_component("STM32F4_MCU", "ARM Microcontroller"),
            "Microcontroller"
        );
        assert_eq!(
            categorize_component("LPC1768", "Cortex-M3 MCU"),
            "Microcontroller"
        );
        assert_eq!(categorize_component("Resistor_0603", ""), "Resistor");
        assert_eq!(categorize_component("Capacitor_100nF", ""), "Capacitor");
        assert_eq!(
            categorize_component("HEADER_2x5", "2x5 Pin Header"),
            "Connector"
        );
        assert_eq!(categorize_component("LED_0603", "SMD LED"), "LED");
    }

    #[test]
    fn test_electrical_type_name() {
        assert_eq!(electrical_type_name(PinElectricalType::Input), "Input");
        assert_eq!(electrical_type_name(PinElectricalType::Output), "Output");
        assert_eq!(electrical_type_name(PinElectricalType::IO), "Bidirectional");
        assert_eq!(electrical_type_name(PinElectricalType::Passive), "Passive");
        assert_eq!(electrical_type_name(PinElectricalType::Power), "Power");
    }

    #[test]
    fn test_parse_electrical_type() {
        assert!(matches!(
            parse_electrical_type("input"),
            PinElectricalType::Input
        ));
        assert!(matches!(
            parse_electrical_type("Output"),
            PinElectricalType::Output
        ));
        assert!(matches!(parse_electrical_type("IO"), PinElectricalType::IO));
        assert!(matches!(
            parse_electrical_type("bidirectional"),
            PinElectricalType::IO
        ));
        assert!(matches!(
            parse_electrical_type("passive"),
            PinElectricalType::Passive
        ));
        assert!(matches!(
            parse_electrical_type("power"),
            PinElectricalType::Power
        ));
        assert!(matches!(
            parse_electrical_type("unknown"),
            PinElectricalType::Passive
        )); // default
    }

    #[test]
    fn test_sch_record_type_name() {
        assert_eq!(sch_record_type_name(1), "Component");
        assert_eq!(sch_record_type_name(2), "Pin");
        assert_eq!(sch_record_type_name(14), "Rectangle");
        assert_eq!(sch_record_type_name(12), "Arc");
        assert_eq!(sch_record_type_name(41), "Parameter");
        assert_eq!(sch_record_type_name(46), "MapDefinerList");
        assert_eq!(sch_record_type_name(47), "MapDefiner");
        assert_eq!(sch_record_type_name(48), "ImplementationParameters");
        assert_eq!(sch_record_type_name(200), "Unknown");
    }
}

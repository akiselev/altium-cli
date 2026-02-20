// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Shared helper functions for ops modules.

/// Sorts strings with embedded numbers naturally (e.g., "A2" < "A10").
pub fn alphanumeric_sort(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(&ac), Some(&bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    let a_num: String = a_chars
                        .by_ref()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    let b_num: String = b_chars
                        .by_ref()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    let a_val: u64 = a_num.parse().unwrap_or(0);
                    let b_val: u64 = b_num.parse().unwrap_or(0);
                    match a_val.cmp(&b_val) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    a_chars.next();
                    b_chars.next();
                    match ac.cmp(&bc) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }
            }
        }
    }
}

/// Map a schematic record key (u8) to a human-readable type name.
pub fn sch_record_type_name(key: u8) -> &'static str {
    match key {
        1 => "Component",
        2 => "Pin",
        3 => "Symbol",
        4 => "Label",
        5 => "Bezier",
        6 => "Polyline",
        7 => "Polygon",
        8 => "Ellipse",
        9 => "Pie",
        10 => "RoundRectangle",
        11 => "EllipticalArc",
        12 => "Arc",
        13 => "Line",
        14 => "Rectangle",
        17 => "Power",
        18 => "Port",
        22 => "NoERC",
        25 => "NetLabel",
        26 => "Bus",
        27 => "Wire",
        28 => "TextFrame",
        29 => "Junction",
        30 => "Image",
        31 => "Sheet",
        32 => "SheetName",
        33 => "SheetFileName",
        34 => "Designator",
        37 => "BusEntry",
        39 => "SheetSymbol",
        40 => "SheetEntry",
        41 => "Parameter",
        44 => "ImplementationList",
        45 => "Implementation",
        46 => "MapDefinerList",
        47 => "MapDefiner",
        48 => "ImplementationParameters",
        209 => "Note",
        255 => "Blanket",
        _ => "Unknown",
    }
}

/// Map a PCB primitive type byte to a human-readable name.
pub fn pcb_primitive_type_name(key: u8) -> &'static str {
    match key {
        1 => "Arc",
        2 => "Pad",
        3 => "Via",
        4 => "Track",
        5 => "Text",
        6 => "Fill",
        7 => "Connection",
        8 => "Net",
        9 => "Component",
        10 => "Polygon",
        11 => "Region",
        12 => "ComponentBody",
        13 => "Dimension",
        14 => "Coordinate",
        _ => "Unknown",
    }
}

/// Map a TShape value to a human-readable shape name.
pub fn pad_shape_name(shape: u8) -> &'static str {
    match shape {
        0 => "NoShape",
        1 => "Round",
        2 => "Rectangular",
        3 => "Octagonal",
        4 => "Circle",
        5 => "Arc",
        6 => "Terminator",
        7 => "RoundedRect",
        8 => "RotatedRect",
        9 => "RoundedRectangular",
        _ => "Unknown",
    }
}

/// Map a layer byte to a display name.
pub fn pcb_layer_name(layer: u8) -> &'static str {
    match layer {
        0 => "NoLayer",
        1 => "TopLayer",
        32 => "BottomLayer",
        33 => "TopOverlay",
        34 => "BottomOverlay",
        35 => "TopPaste",
        36 => "BottomPaste",
        37 => "TopSolder",
        38 => "BottomSolder",
        74 => "MultiLayer",
        _ => {
            if layer >= 2 && layer <= 31 {
                "MidLayer"
            } else if layer >= 39 && layer <= 54 {
                "InternalPlane"
            } else if layer >= 57 && layer <= 72 {
                "Mechanical"
            } else {
                "Other"
            }
        }
    }
}

/// Get the electrical type name for display.
pub fn electrical_type_name(
    electrical: altium_format::v2::records::enums::PinElectricalType,
) -> &'static str {
    use altium_format::v2::records::enums::PinElectricalType;
    match electrical {
        PinElectricalType::Input => "Input",
        PinElectricalType::Output => "Output",
        PinElectricalType::IO => "Bidirectional",
        PinElectricalType::Passive => "Passive",
        PinElectricalType::Power => "Power",
        PinElectricalType::HiZ => "Hi-Z",
        PinElectricalType::OpenCollector => "Open Collector",
        PinElectricalType::OpenEmitter => "Open Emitter",
    }
}

/// Parse electrical type from string.
pub fn parse_electrical_type(s: &str) -> altium_format::v2::records::enums::PinElectricalType {
    use altium_format::v2::records::enums::PinElectricalType;
    match s.to_lowercase().as_str() {
        "input" | "in" => PinElectricalType::Input,
        "output" | "out" => PinElectricalType::Output,
        "io" | "bidirectional" | "bidir" => PinElectricalType::IO,
        "passive" | "pass" => PinElectricalType::Passive,
        "power" | "pwr" => PinElectricalType::Power,
        "hiz" | "hi-z" | "tristate" => PinElectricalType::HiZ,
        "opencollector" | "open_collector" | "oc" => PinElectricalType::OpenCollector,
        "openemitter" | "open_emitter" | "oe" => PinElectricalType::OpenEmitter,
        _ => PinElectricalType::Passive, // default
    }
}

/// Parse a dimension string with unit suffix (e.g., "2.54mm", "100mil") into
/// internal PCB coordinate units.
pub fn parse_dimension(s: &str) -> Result<i32, Box<dyn std::error::Error>> {
    use altium_format::v2::coord::{AltiumCoord, PcbCoord};
    let s = s.trim();
    if let Some(val) = s.strip_suffix("mm") {
        let mm: f64 = val
            .parse()
            .map_err(|_| format!("Invalid dimension: {}", s))?;
        Ok(PcbCoord::from_mm(mm).to_raw())
    } else if let Some(val) = s.strip_suffix("mil") {
        let mils: f64 = val
            .parse()
            .map_err(|_| format!("Invalid dimension: {}", s))?;
        Ok(PcbCoord::from_mils(mils).to_raw())
    } else if let Some(val) = s.strip_suffix("in") {
        let inches: f64 = val
            .parse()
            .map_err(|_| format!("Invalid dimension: {}", s))?;
        Ok(PcbCoord::from_mils(inches * 1000.0).to_raw())
    } else {
        // Default to mm
        let mm: f64 = s.parse().map_err(|_| format!("Invalid dimension: {}", s))?;
        Ok(PcbCoord::from_mm(mm).to_raw())
    }
}

/// Parse shape name to TShape byte value.
pub fn parse_shape(s: &str) -> u8 {
    match s.to_lowercase().as_str() {
        "round" | "circular" => 1,
        "rectangular" | "rect" | "rectangle" => 2,
        "octagonal" | "oct" => 3,
        "rounded_rect" | "roundedrect" | "rounded_rectangular" => 9,
        _ => 2, // default to rectangular
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alphanumeric_sort() {
        let mut items = vec!["A10", "A2", "A1", "B1"];
        items.sort_by(|a, b| alphanumeric_sort(a, b));
        assert_eq!(items, vec!["A1", "A2", "A10", "B1"]);
    }

    #[test]
    fn test_alphanumeric_sort_mixed() {
        let mut items = vec!["PIN10", "PIN2", "PIN1", "PIN20", "VCC", "GND"];
        items.sort_by(|a, b| alphanumeric_sort(a, b));
        assert_eq!(items, vec!["GND", "PIN1", "PIN2", "PIN10", "PIN20", "VCC"]);
    }

    #[test]
    fn test_alphanumeric_sort_pure_numbers() {
        let mut items = vec!["100", "2", "1", "20"];
        items.sort_by(|a, b| alphanumeric_sort(a, b));
        assert_eq!(items, vec!["1", "2", "20", "100"]);
    }

    #[test]
    fn test_parse_dimension() {
        use altium_format::v2::coord::{AltiumCoord, PcbCoord};
        let raw_mm = parse_dimension("1.0mm").unwrap();
        let coord = PcbCoord::from_raw(raw_mm);
        assert!((coord.to_mm() - 1.0).abs() < 0.01);

        let raw_mil = parse_dimension("100mil").unwrap();
        let coord = PcbCoord::from_raw(raw_mil);
        assert!((coord.to_mils() - 100.0).abs() < 0.1);

        let raw_default = parse_dimension("2.54").unwrap();
        let coord = PcbCoord::from_raw(raw_default);
        assert!((coord.to_mm() - 2.54).abs() < 0.01);
    }

    #[test]
    fn test_parse_shape() {
        assert_eq!(parse_shape("round"), 1);
        assert_eq!(parse_shape("rectangular"), 2);
        assert_eq!(parse_shape("rect"), 2);
        assert_eq!(parse_shape("octagonal"), 3);
        assert_eq!(parse_shape("rounded_rect"), 9);
        assert_eq!(parse_shape("unknown"), 2);
    }
}

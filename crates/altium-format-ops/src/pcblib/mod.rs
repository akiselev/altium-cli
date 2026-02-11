// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! PCB footprint library operations.
//!
//! Provides high-level operations for exploring and manipulating Altium PCB
//! footprint library (.PcbLib) files. Uses only the public API from
//! `altium_format` — no internal backing-store types are accessed.

mod browse;
mod detail;
mod json;
mod measure;
mod mutate;
mod render;

pub use browse::*;
pub use detail::*;
pub use json::*;
pub use measure::*;
pub use mutate::*;
pub use render::*;

use std::collections::HashMap;
use std::path::Path;

use altium_format::v2::coord::{AltiumCoord, PcbCoord};
use altium_format::v2::documents::pcblib::PcbLib;

use crate::helpers::*;
use crate::output::*;

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/// PCB primitive type IDs (from the binary framing byte).
#[allow(dead_code)]
pub(super) const TYPE_ARC: u8 = 1;
pub(super) const TYPE_PAD: u8 = 2;
#[allow(dead_code)]
pub(super) const TYPE_VIA: u8 = 3;
#[allow(dead_code)]
pub(super) const TYPE_TRACK: u8 = 4;
#[allow(dead_code)]
pub(super) const TYPE_TEXT: u8 = 5;
#[allow(dead_code)]
pub(super) const TYPE_FILL: u8 = 6;
#[allow(dead_code)]
pub(super) const TYPE_REGION: u8 = 11;
#[allow(dead_code)]
pub(super) const TYPE_COMPONENT_BODY: u8 = 12;

// ═══════════════════════════════════════════════════════════════════════════
// PAD DATA EXTRACTION
// ═══════════════════════════════════════════════════════════════════════════

/// Pad data extracted using the typed `PcbPadRecord` public API.
pub(super) struct PadData {
    pub(super) designator: String,
    pub(super) layer: u8,
    pub(super) record: altium_format::v2::records::PcbPadRecord,
}

impl PadData {
    /// Extract pad data from a cloned `PcbPadRecord`.
    pub(super) fn from_record(record: altium_format::v2::records::PcbPadRecord) -> Self {
        Self {
            designator: record.designator(),
            layer: record.layer(),
            record,
        }
    }

    /// Returns true if this is an SMD pad (no through-hole).
    pub(super) fn is_smd(&self) -> bool {
        self.record.hole_size().to_raw() == 0
    }

    /// Returns a human-readable shape name.
    pub(super) fn shape_name(&self) -> &'static str {
        pad_shape_name(self.record.top_shape())
    }

    /// Returns the layer name for display.
    pub(super) fn layer_name(&self) -> &'static str {
        pcb_layer_name(self.layer)
    }

    /// Returns the size formatted as a string in mm.
    pub(super) fn size_string(&self) -> String {
        let x_mm = self.record.top_size_x().to_mm();
        let y_mm = self.record.top_size_y().to_mm();
        if (x_mm - y_mm).abs() < 0.001 {
            format!("{:.3}mm", x_mm)
        } else {
            format!("{:.3}mm x {:.3}mm", x_mm, y_mm)
        }
    }

    /// Returns the hole size formatted as a string in mm, or None for SMD.
    pub(super) fn hole_string(&self) -> Option<String> {
        if self.record.hole_size().to_raw() == 0 {
            None
        } else {
            Some(format!("{:.3}mm", self.record.hole_size().to_mm()))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Opens and parses a PcbLib file from the given path.
pub(super) fn open_pcblib(path: &Path) -> Result<PcbLib, Box<dyn std::error::Error>> {
    Ok(PcbLib::open_file(path).map_err(|e| e.to_string())?)
}

/// Convert mm to internal PCB coordinate units.
pub(super) fn mm_to_raw(mm: f64) -> i32 {
    PcbCoord::from_mm(mm).to_raw()
}

/// Find a footprint by name (case-insensitive). Returns (index, name).
pub(super) fn find_footprint_by_name<'a>(
    lib: &'a PcbLib,
    name: &str,
) -> Result<(usize, &'a str), Box<dyn std::error::Error>> {
    let idx = lib
        .find_footprint(name)
        .ok_or_else(|| format!("Footprint '{}' not found in library", name))?;
    Ok((idx, &lib.names()[idx]))
}

/// Extract all pad data from a footprint via the read-only view.
pub(super) fn extract_pads_from_view(view: &altium_format::v2::documents::pcblib::PcbFootprintReadView<'_>) -> Vec<PadData> {
    let mut pads = Vec::new();
    view.for_each_pad(|record| {
        pads.push(PadData::from_record(record));
    });
    pads
}

/// Count primitives by type using the read-only view.
pub(super) fn count_primitives_from_view(
    view: &altium_format::v2::documents::pcblib::PcbFootprintReadView<'_>,
) -> HashMap<&'static str, usize> {
    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    view.for_each_primitive(|child| {
        let name = pcb_primitive_type_name(child.type_id());
        *counts.entry(name).or_insert(0) += 1;
    });
    counts
}

/// Categorize a footprint by its name and description.
pub(super) fn categorize_footprint(name: &str, description: &str) -> &'static str {
    let name_lower = name.to_lowercase();
    let desc_lower = description.to_lowercase();

    if name_lower.contains("bga") || desc_lower.contains("bga") {
        return "BGA";
    }
    if name_lower.contains("qfp")
        || name_lower.contains("tqfp")
        || name_lower.contains("lqfp")
    {
        return "QFP";
    }
    if name_lower.contains("qfn") || name_lower.contains("dfn") {
        return "QFN/DFN";
    }
    if name_lower.contains("soic")
        || name_lower.contains("sop")
        || name_lower.contains("ssop")
        || name_lower.contains("tssop")
        || name_lower.contains("msop")
    {
        return "SOIC/SOP";
    }
    if name_lower.contains("sot") {
        return "SOT";
    }
    if name_lower.contains("dip") || name_lower.contains("pdip") {
        return "DIP";
    }
    if name_lower.starts_with("0201")
        || name_lower.starts_with("0402")
        || name_lower.starts_with("0603")
        || name_lower.starts_with("0805")
        || name_lower.starts_with("1206")
        || name_lower.starts_with("1210")
        || name_lower.starts_with("2010")
        || name_lower.starts_with("2512")
    {
        return "Chip/SMD";
    }
    if name_lower.contains("header")
        || name_lower.contains("connector")
        || name_lower.contains("socket")
        || name_lower.contains("terminal")
        || name_lower.contains("usb")
        || name_lower.contains("rj45")
    {
        return "Connector";
    }
    if name_lower.contains("axial")
        || name_lower.contains("radial")
        || name_lower.contains("through")
        || name_lower.contains("th_")
    {
        return "Through-Hole";
    }
    if name_lower.contains("cap_elec") || name_lower.contains("electrolytic") {
        return "Electrolytic";
    }
    if name_lower.contains("inductor")
        || name_lower.contains("choke")
        || name_lower.contains("ferrite")
    {
        return "Inductor";
    }
    if name_lower.contains("xtal")
        || name_lower.contains("crystal")
        || name_lower.contains("oscillator")
    {
        return "Crystal/Oscillator";
    }
    if name_lower.contains("led") {
        return "LED";
    }
    if name_lower.contains("test") || name_lower.contains("tp_") {
        return "Test Point";
    }
    if name_lower.contains("mount") || name_lower.contains("standoff") {
        return "Mounting Hole";
    }

    "Other"
}

/// Compute bounding box for a footprint's pads.
pub(super) fn compute_bounding_box(pads: &[PadData]) -> BoundingBox {
    if pads.is_empty() {
        return BoundingBox {
            width: "0mm".to_string(),
            height: "0mm".to_string(),
        };
    }

    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;

    for pad in pads {
        let pos_x = pad.record.position_x().to_raw();
        let pos_y = pad.record.position_y().to_raw();
        let half_x = pad.record.top_size_x().to_raw() / 2;
        let half_y = pad.record.top_size_y().to_raw() / 2;
        min_x = min_x.min(pos_x - half_x);
        max_x = max_x.max(pos_x + half_x);
        min_y = min_y.min(pos_y - half_y);
        max_y = max_y.max(pos_y + half_y);
    }

    let width_mm = PcbCoord::from_raw(max_x - min_x).to_mm();
    let height_mm = PcbCoord::from_raw(max_y - min_y).to_mm();

    BoundingBox {
        width: format!("{:.3}mm", width_mm),
        height: format!("{:.3}mm", height_mm),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_footprint() {
        assert_eq!(categorize_footprint("BGA-256", ""), "BGA");
        assert_eq!(categorize_footprint("TQFP-100", ""), "QFP");
        assert_eq!(categorize_footprint("QFN-32", ""), "QFN/DFN");
        assert_eq!(categorize_footprint("SOIC-8", ""), "SOIC/SOP");
        assert_eq!(categorize_footprint("SOT-23", ""), "SOT");
        assert_eq!(categorize_footprint("DIP-8", ""), "DIP");
        assert_eq!(categorize_footprint("0603", ""), "Chip/SMD");
        assert_eq!(categorize_footprint("USB_TypeC", ""), "Connector");
        assert_eq!(categorize_footprint("LED_0805", ""), "LED");
        assert_eq!(categorize_footprint("TestPoint_1mm", ""), "Test Point");
        assert_eq!(categorize_footprint("CustomFP", ""), "Other");
    }

    #[test]
    fn test_pad_data_from_record() {
        let origin = altium_format::v2::templates::pcb_pad_default();
        let mut record = altium_format::v2::records::PcbPadRecord::from_origin(origin);
        record.set_position_x(PcbCoord::from_mm(1.0));
        record.set_position_y(PcbCoord::from_mm(2.0));
        record.set_top_size_x(PcbCoord::from_mm(0.5));
        record.set_top_size_y(PcbCoord::from_mm(0.8));
        record.set_top_shape(2); // Rectangular
        record.set_layer(1); // TopLayer

        let pad = PadData::from_record(record);
        assert!((pad.record.position_x().to_mm() - 1.0).abs() < 0.01);
        assert!((pad.record.position_y().to_mm() - 2.0).abs() < 0.01);
        assert_eq!(pad.record.top_shape(), 2);
        assert!(pad.is_smd());
        assert_eq!(pad.shape_name(), "Rectangular");
        assert_eq!(pad.layer_name(), "TopLayer");
    }

    #[test]
    fn test_compute_bounding_box_from_records() {
        let origin1 = altium_format::v2::templates::pcb_pad_default();
        let mut rec1 = altium_format::v2::records::PcbPadRecord::from_origin(origin1);
        rec1.set_position_x(PcbCoord::from_mm(-1.0));
        rec1.set_position_y(PcbCoord::from_mm(0.0));
        rec1.set_top_size_x(PcbCoord::from_mm(0.5));
        rec1.set_top_size_y(PcbCoord::from_mm(0.5));
        rec1.set_layer(1);

        let origin2 = altium_format::v2::templates::pcb_pad_default();
        let mut rec2 = altium_format::v2::records::PcbPadRecord::from_origin(origin2);
        rec2.set_position_x(PcbCoord::from_mm(1.0));
        rec2.set_position_y(PcbCoord::from_mm(0.0));
        rec2.set_top_size_x(PcbCoord::from_mm(0.5));
        rec2.set_top_size_y(PcbCoord::from_mm(0.5));
        rec2.set_layer(1);

        let pads = vec![
            PadData::from_record(rec1),
            PadData::from_record(rec2),
        ];

        let bb = compute_bounding_box(&pads);
        assert!(bb.width.contains("2.5"));
    }
}

//! PCB Component (footprint) container.
//!
//! **DEPRECATED**: Use `v2::pcb::PcbComponentV2` with `v2::pcb::io` instead.

use super::{PcbPad, PcbRecord};
use crate::types::{Coord, CoordRect, ParameterCollection};

/// A PCB component (footprint) containing primitives.
///
/// **DEPRECATED**: Use `v2::pcb::PcbComponentV2` instead.
#[deprecated(note = "Use v2::pcb::PcbComponentV2")]
#[derive(Debug, Clone, Default)]
pub struct PcbComponent {
    /// Footprint pattern name.
    pub pattern: String,
    /// Description.
    pub description: String,
    /// Height.
    pub height: Coord,
    /// Item GUID.
    pub item_guid: String,
    /// Revision GUID.
    pub revision_guid: String,
    /// Primitives belonging to this component.
    pub primitives: Vec<PcbRecord>,
}

impl PcbComponent {
    /// Import component data from parameters.
    pub fn import_from_parameters(&mut self, p: &ParameterCollection) {
        self.pattern = p
            .get("PATTERN")
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        self.height = Coord::from_raw(p.get("HEIGHT").map(|v| v.as_int_or(0)).unwrap_or(0));
        self.description = p
            .get("DESCRIPTION")
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        self.item_guid = p
            .get("ITEMGUID")
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        self.revision_guid = p
            .get("REVISIONGUID")
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
    }

    /// Export component data to parameters.
    pub fn export_to_parameters(&self) -> ParameterCollection {
        let mut params = ParameterCollection::new();
        params.add("PATTERN", &self.pattern);
        params.add_int("HEIGHT", self.height.to_raw());
        if !self.description.is_empty() {
            params.add("DESCRIPTION", &self.description);
        }
        if !self.item_guid.is_empty() {
            params.add("ITEMGUID", &self.item_guid);
        }
        if !self.revision_guid.is_empty() {
            params.add("REVISIONGUID", &self.revision_guid);
        }
        params
    }

    /// Get the component name (pattern).
    pub fn name(&self) -> &str {
        &self.pattern
    }

    /// Count the number of pads.
    pub fn pad_count(&self) -> usize {
        self.primitives
            .iter()
            .filter(|p| matches!(p, PcbRecord::Pad(_)))
            .count()
    }

    /// Get all pads.
    pub fn pads(&self) -> impl Iterator<Item = &PcbPad> {
        self.primitives.iter().filter_map(|p| {
            if let PcbRecord::Pad(pad) = p {
                Some(pad.as_ref())
            } else {
                None
            }
        })
    }

    /// Total primitive count.
    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }

    /// Calculate the bounding rectangle of all primitives.
    pub fn calculate_bounds(&self) -> CoordRect {
        let mut bounds = CoordRect::default();
        let mut first = true;

        for prim in &self.primitives {
            let prim_bounds = match prim {
                PcbRecord::Arc(r) => r.calculate_bounds(),
                PcbRecord::Pad(r) => r.calculate_bounds(),
                PcbRecord::Via(r) => r.calculate_bounds(),
                PcbRecord::Track(r) => r.calculate_bounds(),
                PcbRecord::Text(r) => r.calculate_bounds(),
                PcbRecord::Fill(r) => r.calculate_bounds(),
                PcbRecord::Region(r) => r.calculate_bounds(),
                PcbRecord::ComponentBody(r) => r.calculate_bounds(),
                PcbRecord::Polygon(_)
                | PcbRecord::Dimension(_)
                | PcbRecord::Coordinate(_) => continue, // These don't belong to footprint components
                PcbRecord::Unknown { .. } => continue,
            };

            if first {
                bounds = prim_bounds;
                first = false;
            } else {
                bounds = bounds.union(prim_bounds);
            }
        }

        bounds
    }
}

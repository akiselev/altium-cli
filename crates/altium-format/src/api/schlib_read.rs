//! Read path: convert internal SchLib types → public API types.

use crate::api::schlib_types::*;
use crate::api::sch_common::{process_records, build_footprint_maps};
use crate::sch_records::SchLibComponent;
use crate::schlib::SchLibComponentIndex;
use crate::Result;

/// Convert an internal `SchLibComponent` + index entry into a public `Component`.
pub(crate) fn component_from_internal(
    comp: &SchLibComponent,
    index: &SchLibComponentIndex,
) -> Result<Component> {
    let mut designator = None;
    let mut pins = Vec::new();
    let mut parameters = Vec::new();
    let mut graphics = Vec::new();

    // Process main records
    process_records(&comp.records, &mut designator, &mut pins, &mut parameters, &mut graphics)?;
    // Process additional records (same logic)
    process_records(&comp.additional_records, &mut designator, &mut pins, &mut parameters, &mut graphics)?;

    // Build footprint maps from the implementation chain in main records
    let footprints = build_footprint_maps(&comp.records)?;

    Ok(Component {
        lib_reference: index.lib_ref.clone(),
        designator,
        description: if index.description.is_empty() {
            None
        } else {
            Some(index.description.clone())
        },
        component_kind: if comp.component.component_kind == altium_format_types::common::ComponentKind::Standard {
            None
        } else {
            Some(comp.component.component_kind)
        },
        part_count: comp.component.part_count,
        show_hidden_pins: comp.component.show_hidden_pins,
        pins,
        parameters,
        footprints,
        graphics,
        aliases: index.aliases.clone(),
    })
}

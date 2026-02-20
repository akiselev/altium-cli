// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! SchLib mutation commands: create, add_component, add_pin.

use std::path::Path;

use crate::helpers::*;

use altium_format::v2::handles::SchComponentHandle;

use super::open_schlib;

/// Embedded blank SchLib template.
const BLANK_SCHLIB_TEMPLATE: &[u8] =
    include_bytes!("../../../altium-format/data/blank/Schlib1.SchLib");

/// Creates an empty SchLib file at the given path.
pub fn cmd_create(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        return Err(format!("File already exists: {}", path.display()).into());
    }

    std::fs::write(path, BLANK_SCHLIB_TEMPLATE)
        .map_err(|e| format!("Error creating file: {}", e))?;

    println!("Created empty SchLib: {}", path.display());
    Ok(())
}

/// Adds a new component to an existing library.
pub fn cmd_add_component(
    path: &Path,
    name: &str,
    description: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    // Check if component already exists
    if lib.find_component(name).is_some() {
        return Err(format!("Component '{}' already exists in library", name).into());
    }

    // Add component using the builder API
    use altium_format::v2::newtypes::LibReference;
    use altium_format::v2::templates;
    lib.build_component(templates::sch_component_default, |builder| {
        builder.with_component(|comp| {
            comp.set_lib_reference(LibReference::from(name));
            comp.set_component_description(description.unwrap_or_default());
        });
    });

    // Clear raw header to force rebuild
    {
        let mut store = lib.store().borrow_mut();
        if let altium_format::v2::store::DocumentMeta::SchLib { raw_header, .. } = store.meta_mut()
        {
            *raw_header = None;
        }
    }

    // Write back
    lib.save_file(path).map_err(|e| e.to_string())?;

    println!("Added component '{}' to {}", name, path.display());
    Ok(())
}

/// Adds a pin to an existing component in the library.
pub fn cmd_add_pin(
    path: &Path,
    component: &str,
    designator: &str,
    name: &str,
    electrical_type: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    // Find component
    let group_id = lib
        .find_component(component)
        .ok_or_else(|| format!("Component '{}' not found", component))?;

    // Parse electrical type
    let electrical = parse_electrical_type(electrical_type);

    // Build pin record from template
    use altium_format::v2::backing_store::RecordNode;
    use altium_format::v2::newtypes::{Designator, PinName};
    use altium_format::v2::records::SchPinRecord;
    use altium_format::v2::templates;
    use altium_format::v2::traits::{FromOrigin, RecordType};

    let pin_origin = templates::sch_pin_default();
    let mut pin_rec = SchPinRecord::from_origin(pin_origin.clone());
    pin_rec.set_designator(Designator::from(designator));
    pin_rec.set_name(PinName::from(name));
    pin_rec.set_electrical(electrical);
    let node = RecordNode::new(SchPinRecord::RECORD_ID, pin_rec.into_origin());

    let comp = SchComponentHandle::new(lib.store().clone(), group_id);
    comp.add_record(node);

    // Clear raw header to force rebuild
    {
        let mut store = lib.store().borrow_mut();
        if let altium_format::v2::store::DocumentMeta::SchLib { raw_header, .. } = store.meta_mut()
        {
            *raw_header = None;
        }
    }

    // Write back
    lib.save_file(path).map_err(|e| e.to_string())?;

    println!(
        "Added pin '{}' ({}) to component '{}' in {}",
        designator,
        name,
        component,
        path.display()
    );
    Ok(())
}

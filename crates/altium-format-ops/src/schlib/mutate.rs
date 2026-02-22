// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! SchLib mutation commands: create, add_component, add_pin.

use std::path::Path;

use crate::helpers::*;

use super::open_schlib;

/// Embedded blank SchLib template.
const BLANK_SCHLIB_TEMPLATE: &[u8] =
    include_bytes!("../../../altium-format/data/blank/Schlib1.SchLib");

/// Creates an empty SchLib file at the given path.
pub fn cmd_create(path: &Path) -> crate::Result<()> {
    if path.exists() {
        return Err(crate::AltiumOpsError::AlreadyExists(format!("File already exists: {}", path.display())));
    }

    std::fs::write(path, BLANK_SCHLIB_TEMPLATE)?;

    println!("Created empty SchLib: {}", path.display());
    Ok(())
}

/// Adds a new component to an existing library.
pub fn cmd_add_component(
    path: &Path,
    name: &str,
    description: Option<String>,
) -> crate::Result<()> {
    let lib = open_schlib(path)?;

    // Check if component already exists
    if lib.find_component_handle(name).is_some() {
        return Err(crate::AltiumOpsError::AlreadyExists(format!("Component '{}' already exists in library", name)));
    }

    // Add component using the builder API
    use altium_format::newtypes::LibReference;
    use altium_format::templates;
    lib.build_component(templates::sch_component_default, |builder| {
        builder.with_component(|comp| {
            comp.set_lib_reference(LibReference::from(name));
            comp.set_component_description(description.unwrap_or_default());
        });
    })?;

    lib.invalidate_cached_header();

    // Write back
    lib.save_file(path)?;

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
) -> crate::Result<()> {
    let lib = open_schlib(path)?;

    // Find component
    let comp = lib
        .find_component_handle(component)
        .ok_or_else(|| crate::AltiumOpsError::NotFound(format!("Component '{}' not found", component)))?;

    // Parse electrical type
    let electrical = parse_electrical_type(electrical_type);

    // Build pin record from template
    use altium_format::newtypes::{Designator, PinName};
    use altium_format::records::SchPinRecord;
    use altium_format::templates;

    let pin_origin = templates::sch_pin_default();
    let mut pin_rec = SchPinRecord::from_origin(pin_origin);
    pin_rec.set_designator(Designator::from(designator));
    pin_rec.set_name(PinName::from(name));
    pin_rec.set_electrical(electrical);

    comp.add_child_record(pin_rec);

    lib.invalidate_cached_header();

    // Write back
    lib.save_file(path)?;

    println!(
        "Added pin '{}' ({}) to component '{}' in {}",
        designator,
        name,
        component,
        path.display()
    );
    Ok(())
}

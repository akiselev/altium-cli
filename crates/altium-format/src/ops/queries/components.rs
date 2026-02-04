// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Component query operations.
//!
//! # V2 Migration Note
//! This module uses v1 types (SchDoc, SchRecord, SchComponent).
//! TODO: Migrate to v2 types when SchDocV2 provides equivalent functionality.

#![allow(deprecated)]

use crate::io::SchDoc;
use crate::ops::categorization::categorize_component;
use crate::records::sch::{SchComponent, SchRecord};
use std::collections::HashMap;

/// Group components by category.
pub fn components_by_category(
    doc: &SchDoc,
    get_designator: impl Fn(&SchComponent) -> String,
) -> HashMap<&'static str, Vec<(String, String, String)>> {
    let mut groups: HashMap<&'static str, Vec<(String, String, String)>> = HashMap::new();

    for record in &doc.primitives {
        if let SchRecord::Component(comp) = record {
            let designator = get_designator(comp);
            let category = categorize_component(&comp.lib_reference, &comp.component_description);
            groups.entry(category).or_default().push((
                designator,
                comp.lib_reference.clone(),
                comp.component_description.clone(),
            ));
        }
    }

    groups
}

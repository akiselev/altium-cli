// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! SchLib browse commands: overview, list, search, info.

use std::collections::HashMap;
use std::path::Path;

use altium_format::v2::traits::DocumentQuery;
use altium_format::v2::views::{SchComponent, SchPin};

use crate::categorization::categorize_component;
use crate::helpers::*;
use crate::output::*;

use super::{count_primitives_via_view, open_schlib};

/// Returns library overview with statistics and component category breakdown.
pub fn cmd_overview(path: &Path) -> Result<SchLibOverview, Box<dyn std::error::Error>> {
    let mut lib = open_schlib(path)?;

    // 1. COMPONENTS BY CATEGORY
    let mut categories: HashMap<&'static str, Vec<ComponentSummary>> = HashMap::new();

    DocumentQuery::<SchComponent>::query_all(&mut lib, "#1")?.for_each_mut(|entry, view| {
        let category = categorize_component(entry.lib_ref(), entry.description());
        let pin_count = view.child_keys::<SchPin>().count();
        categories
            .entry(category)
            .or_default()
            .push(ComponentSummary {
                name: entry.lib_ref().to_string(),
                description: entry.description().to_string(),
                pin_count,
                part_count: entry.part_count(),
            });
    });

    let category_order = [
        "Microcontroller",
        "FPGA/CPLD",
        "Memory",
        "ADC",
        "DAC",
        "Transceiver/PHY",
        "Clock/Oscillator",
        "Power Supply",
        "Amplifier",
        "Mux/Switch",
        "Buffer/Driver",
        "Capacitor",
        "Resistor",
        "Inductor/Ferrite",
        "Diode/Protection",
        "Transistor",
        "LED",
        "Connector",
        "Test Point",
        "Other IC",
    ];

    let mut components_by_category = Vec::new();
    for category in category_order.iter() {
        if let Some(mut comps) = categories.remove(*category) {
            comps.sort_by(|a, b| alphanumeric_sort(&a.name, &b.name));
            components_by_category.push((category.to_string(), comps));
        }
    }
    for (category, mut comps) in categories {
        if !comps.is_empty() {
            comps.sort_by(|a, b| alphanumeric_sort(&a.name, &b.name));
            components_by_category.push((category.to_string(), comps));
        }
    }

    // 2. PIN STATISTICS
    let mut total_pins = 0;
    let mut pin_types: HashMap<&'static str, usize> = HashMap::new();

    DocumentQuery::<SchComponent>::query_all(&mut lib, "#1")?.for_each_mut(|_entry, mut view| {
        let keys: Vec<_> = view.child_keys::<SchPin>().collect();
        for key in keys {
            view.with_child_mut(key, |pin| {
                total_pins += 1;
                let type_name = electrical_type_name(pin.electrical());
                *pin_types.entry(type_name).or_insert(0) += 1;
            });
        }
    });

    let mut pin_types_vec: Vec<_> = pin_types
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    pin_types_vec.sort_by(|a, b| b.1.cmp(&a.1));

    // 3. MULTI-PART COMPONENTS
    let mut multi_part_components: Vec<ComponentSummary> = Vec::new();

    DocumentQuery::<SchComponent>::query_all(&mut lib, "#1")?.for_each_mut(|entry, view| {
        if entry.part_count() > 1 {
            multi_part_components.push(ComponentSummary {
                name: entry.lib_ref().to_string(),
                description: entry.description().to_string(),
                pin_count: view.child_keys::<SchPin>().count(),
                part_count: entry.part_count(),
            });
        }
    });
    multi_part_components.sort_by(|a, b| b.part_count.cmp(&a.part_count));

    // 4. LARGEST COMPONENTS (by pin count)
    let mut by_pins: Vec<ComponentSummary> = Vec::new();

    DocumentQuery::<SchComponent>::query_all(&mut lib, "#1")?.for_each_mut(|entry, view| {
        by_pins.push(ComponentSummary {
            name: entry.lib_ref().to_string(),
            description: entry.description().to_string(),
            pin_count: view.child_keys::<SchPin>().count(),
            part_count: entry.part_count(),
        });
    });
    by_pins.sort_by_key(|c| std::cmp::Reverse(c.pin_count));

    let largest_components = by_pins.into_iter().take(10).collect();

    Ok(SchLibOverview {
        path: path.display().to_string(),
        total_components: lib.component_count(),
        components_by_category,
        pin_statistics: PinStatistics {
            total_pins,
            pin_types: pin_types_vec,
        },
        multi_part_components,
        largest_components,
        component_details: None,
    })
}

/// Lists all components in the library sorted alphanumerically.
pub fn cmd_list(path: &Path) -> Result<SchLibComponentList, Box<dyn std::error::Error>> {
    let mut lib = open_schlib(path)?;

    let mut components: Vec<ComponentSummary> = Vec::new();

    DocumentQuery::<SchComponent>::query_all(&mut lib, "#1")?.for_each_mut(|entry, view| {
        components.push(ComponentSummary {
            name: entry.lib_ref().to_string(),
            description: entry.description().to_string(),
            pin_count: view.child_keys::<SchPin>().count(),
            part_count: entry.part_count(),
        });
    });

    components.sort_by(|a, b| alphanumeric_sort(&a.name, &b.name));

    Ok(SchLibComponentList {
        path: path.display().to_string(),
        total_components: lib.component_count(),
        components,
    })
}

/// Searches for components matching the query in name or description.
pub fn cmd_search(
    path: &Path,
    query: &str,
    limit: Option<usize>,
) -> Result<SchLibSearchResults, Box<dyn std::error::Error>> {
    let mut lib = open_schlib(path)?;

    let query_lower = query.to_lowercase();
    let has_wildcard = query.contains('*');

    let mut matches: Vec<ComponentSummary> = Vec::new();

    DocumentQuery::<SchComponent>::query_all(&mut lib, "#1")?.for_each_mut(|entry, view| {
        let name = entry.lib_ref().to_lowercase();
        let desc = entry.description().to_lowercase();

        let is_match = if has_wildcard {
            let pattern = query_lower.replace('*', "");
            name.contains(&pattern) || desc.contains(&pattern)
        } else {
            name.contains(&query_lower) || desc.contains(&query_lower)
        };

        if is_match {
            matches.push(ComponentSummary {
                name: entry.lib_ref().to_string(),
                description: entry.description().to_string(),
                pin_count: view.child_keys::<SchPin>().count(),
                part_count: entry.part_count(),
            });
        }
    });

    // Sort by relevance (exact name match first, then by name)
    matches.sort_by(|a, b| {
        let a_exact = a.name.to_lowercase() == query_lower;
        let b_exact = b.name.to_lowercase() == query_lower;
        match (a_exact, b_exact) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => alphanumeric_sort(&a.name, &b.name),
        }
    });

    if let Some(max) = limit {
        matches.truncate(max);
    }

    let total_matches = matches.len();

    Ok(SchLibSearchResults {
        query: query.to_string(),
        total_matches,
        results: matches,
    })
}

/// Returns detailed library metadata including file info and header data.
pub fn cmd_info(path: &Path) -> Result<SchLibInfo, Box<dyn std::error::Error>> {
    let mut lib = open_schlib(path)?;

    let mut primitive_counts: HashMap<&'static str, usize> = HashMap::new();
    let mut total_primitives = 0;
    let mut multi_part_count = 0;

    DocumentQuery::<SchComponent>::query_all(&mut lib, "#1")?.for_each_mut(|entry, view| {
        let counts = count_primitives_via_view(&view);
        for (name, count) in counts {
            *primitive_counts.entry(name).or_insert(0) += count;
            total_primitives += count;
        }
        if entry.part_count() > 1 {
            multi_part_count += 1;
        }
    });

    let mut primitive_types: Vec<_> = primitive_counts
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    primitive_types.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(SchLibInfo {
        path: path.display().to_string(),
        component_count: lib.component_count(),
        total_primitives,
        primitive_types,
        multi_part_count,
    })
}

// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Power net analysis.
//!
//! # V2 Migration Note
//! This module uses v1 types (SchDoc, SchRecord).
//! TODO: Migrate to v2 types when SchDocV2 provides equivalent functionality.

#![allow(deprecated)]

use crate::io::SchDoc;
use crate::records::sch::SchRecord;
use std::collections::{HashMap, HashSet};

/// Extract power net names.
pub fn power_nets(doc: &SchDoc) -> HashSet<String> {
    let mut nets = HashSet::new();

    for record in &doc.primitives {
        if let SchRecord::PowerObject(pwr) = record {
            nets.insert(pwr.text.clone());
        }
    }

    nets
}

/// Map power nets to their connection counts.
pub fn power_map(doc: &SchDoc) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();

    for record in &doc.primitives {
        if let SchRecord::PowerObject(pwr) = record {
            *counts.entry(pwr.text.clone()).or_insert(0) += 1;
        }
    }

    counts
}

/// Type alias for power net statistics (name, connection count).
pub type PowerNetStats = Vec<(String, usize)>;

/// Separate power rails from ground nets.
#[allow(clippy::type_complexity)]
pub fn separate_power_and_ground(
    power_nets: HashMap<String, usize>,
) -> (PowerNetStats, PowerNetStats) {
    let mut rails: Vec<_> = power_nets
        .iter()
        .filter(|(name, _)| {
            !name.to_uppercase().contains("GND") && !name.to_uppercase().contains("VSS")
        })
        .map(|(name, count)| (name.clone(), *count))
        .collect();
    rails.sort_by(|a, b| b.1.cmp(&a.1));

    let mut grounds: Vec<_> = power_nets
        .iter()
        .filter(|(name, _)| {
            name.to_uppercase().contains("GND") || name.to_uppercase().contains("VSS")
        })
        .map(|(name, count)| (name.clone(), *count))
        .collect();
    grounds.sort_by(|a, b| b.1.cmp(&a.1));

    (rails, grounds)
}

// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Net connectivity queries.
//!
//! # V2 Migration Note
//! This module uses v1 types (SchDoc, SchRecord).
//! TODO: Migrate to v2 types when SchDocV2 provides equivalent functionality.

#![allow(deprecated)]

use crate::io::SchDoc;
use crate::records::sch::SchRecord;
use std::collections::HashMap;

/// Build net name to location mapping.
pub fn net_locations(doc: &SchDoc) -> HashMap<(i32, i32), String> {
    let mut net_at_location: HashMap<(i32, i32), String> = HashMap::new();

    for record in &doc.primitives {
        match record {
            SchRecord::NetLabel(nl) => {
                net_at_location.insert(
                    (nl.label.graphical.location_x, nl.label.graphical.location_y),
                    nl.label.text.clone(),
                );
            }
            SchRecord::PowerObject(p) => {
                net_at_location.insert(
                    (p.graphical.location_x, p.graphical.location_y),
                    p.text.clone(),
                );
            }
            _ => {}
        }
    }

    net_at_location
}

/// Group connections by net name using proximity matching.
pub fn connections_by_net(
    net_locations: &HashMap<(i32, i32), String>,
    pin_locations: &HashMap<(i32, i32), Vec<String>>,
    proximity_threshold: i32,
) -> HashMap<String, Vec<String>> {
    let mut nets: HashMap<String, Vec<String>> = HashMap::new();

    for ((net_x, net_y), net_name) in net_locations {
        for ((pin_x, pin_y), pins) in pin_locations {
            if (net_x - pin_x).abs() < proximity_threshold
                && (net_y - pin_y).abs() < proximity_threshold
            {
                for pin_ref in pins {
                    nets.entry(net_name.clone())
                        .or_default()
                        .push(pin_ref.clone());
                }
            }
        }
    }

    nets
}

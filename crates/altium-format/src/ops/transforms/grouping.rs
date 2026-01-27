// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Spatial grouping transforms.

use crate::types::Coord;

/// Group items by proximity using squared distance for threshold comparison.
///
/// Avoids sqrt (expensive) by comparing squared distances.
/// Uses saturating arithmetic to prevent overflow with large coordinate values.
pub fn group_by_proximity<T>(
    items: &[T],
    get_location: impl Fn(&T) -> Option<(i32, i32)>,
    threshold: Coord,
) -> Vec<Vec<&T>> {
    let mut groups: Vec<Vec<&T>> = Vec::new();

    for item in items {
        let Some(loc) = get_location(item) else {
            continue;
        };

        let mut assigned = false;
        for group in &mut groups {
            if let Some(first) = group.first() {
                if let Some(first_loc) = get_location(first) {
                    let threshold_raw = threshold.to_raw();
                    let threshold_squared = threshold_raw.saturating_mul(threshold_raw);

                    let dx = loc.0 - first_loc.0;
                    let dy = loc.1 - first_loc.1;
                    let distance_squared =
                        dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy));

                    if distance_squared < threshold_squared {
                        group.push(item);
                        assigned = true;
                        break;
                    }
                }
            }
        }

        if !assigned {
            groups.push(vec![item]);
        }
    }

    groups
}

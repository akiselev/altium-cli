// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! PcbLib measurement commands.

use std::path::Path;

use altium_format::v2::coord::{AltiumCoord, PcbCoord};
use altium_format::v2::traits::DocumentQuery;
use altium_format::v2::views::PcbFootprint;

use super::{compute_bounding_box, extract_pads_from_view, find_footprint_by_name, open_pcblib};

/// Measure footprint dimensions and clearances.
pub fn cmd_measure(
    path: &Path,
    footprint: &str,
    _measure_type: &str,
    _pad1: Option<&str>,
    _pad2: Option<&str>,
    _axis: Option<&str>,
    as_json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lib = open_pcblib(path)?;
    let (idx, _) = find_footprint_by_name(&lib, footprint)?;

    let mut output: Option<Result<(), Box<dyn std::error::Error>>> = None;
    let mut current_idx = 0;
    lib.query_all::<PcbFootprint>("#0")?.for_each_mut(|fp_name, mut view| {
        if current_idx == idx {
            let pads = extract_pads_from_view(&mut view);
            let bb = compute_bounding_box(&pads);

            // Calculate pad pitch (min center-to-center distance)
            let mut min_pitch = f64::MAX;
            for i in 0..pads.len() {
                for j in (i + 1)..pads.len() {
                    let dx = (pads[i].record.position_x().to_raw()
                        - pads[j].record.position_x().to_raw()) as f64;
                    let dy = (pads[i].record.position_y().to_raw()
                        - pads[j].record.position_y().to_raw()) as f64;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist > 0.0 && dist < min_pitch {
                        min_pitch = dist;
                    }
                }
            }

            let pitch_mm = if min_pitch < f64::MAX {
                PcbCoord::from_raw(min_pitch as i32).to_mm()
            } else {
                0.0
            };

            if as_json {
                let result = serde_json::json!({
                    "footprint": fp_name,
                    "pad_count": pads.len(),
                    "bounding_box": {
                        "width": bb.width,
                        "height": bb.height,
                    },
                    "min_pitch_mm": format!("{:.3}", pitch_mm),
                    "smd_pads": pads.iter().filter(|p| p.is_smd()).count(),
                    "th_pads": pads.iter().filter(|p| !p.is_smd()).count(),
                });
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
            } else {
                println!("Footprint: {}", fp_name);
                println!("Pad count: {}", pads.len());
                println!("Bounding box: {} x {}", bb.width, bb.height);
                if pitch_mm > 0.0 {
                    println!("Min pad pitch: {:.3}mm", pitch_mm);
                }
                println!(
                    "SMD: {}, Through-hole: {}",
                    pads.iter().filter(|p| p.is_smd()).count(),
                    pads.iter().filter(|p| !p.is_smd()).count()
                );
            }

            output = Some(Ok(()));
        }
        current_idx += 1;
    });

    match output {
        Some(r) => r,
        None => Err(format!("Footprint '{}' not found", footprint).into()),
    }
}

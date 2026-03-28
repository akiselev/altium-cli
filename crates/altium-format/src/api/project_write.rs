//! Write path: serializes AltiumProject internal storage to INI text format.

use indexmap::IndexMap;

use crate::project::AltiumProject;

/// Known output-level key prefixes, in the order they should be written.
const OUTPUT_KEY_ORDER: &[&str] = &[
    "OutputType",
    "OutputName",
    "OutputDocumentPath",
    "OutputVariantName",
    "OutputDefault",
    "PageOptions",
];

/// Serialize the internal project representation to INI text.
pub(crate) fn write_ini(proj: &AltiumProject) -> String {
    let mut out = String::new();

    // Sections written in canonical order matching Altium's output
    write_section(&mut out, "Design", proj.design());
    write_section(&mut out, "Preferences", proj.preferences());

    // Numbered sections
    for (i, doc) in proj.documents().iter().enumerate() {
        write_section(&mut out, &format!("Document{}", i + 1), doc);
    }

    for (i, cfg) in proj.configurations().iter().enumerate() {
        write_section(&mut out, &format!("Configuration{}", i + 1), cfg);
    }

    // Output groups need special handling for indexed output keys
    for (i, group) in proj.output_groups().iter().enumerate() {
        write_output_group(&mut out, &format!("OutputGroup{}", i + 1), group);
    }

    write_section(&mut out, "Modification Levels", proj.modification_levels());
    write_section(&mut out, "Difference Levels", proj.difference_levels());
    write_section(&mut out, "Electrical Rules Check", proj.erc_levels());
    write_section(&mut out, "ERC Connection Matrix", proj.erc_matrix());
    write_section(&mut out, "Annotate", proj.annotate());
    write_section(&mut out, "PrjClassGen", proj.class_gen());
    write_section(
        &mut out,
        "LibraryUpdateOptions",
        proj.library_update_options(),
    );
    write_section(
        &mut out,
        "DatabaseUpdateOptions",
        proj.database_update_options(),
    );
    write_section(&mut out, "Comparison Options", proj.comparison_options());
    write_section(&mut out, "SmartPDF", proj.smart_pdf());

    // Optional numbered sections (only if present)
    for (i, var) in proj.variants().iter().enumerate() {
        write_section(&mut out, &format!("Variant{}", i + 1), var);
    }
    for (i, param) in proj.parameters_sections().iter().enumerate() {
        write_section(&mut out, &format!("Parameter{}", i + 1), param);
    }
    for (i, dp) in proj.diff_pair_suffixes().iter().enumerate() {
        write_section(&mut out, &format!("DiffPairSuffix{}", i + 1), dp);
    }

    if !proj.net_infos().is_empty() {
        write_section(&mut out, "Net Info", proj.net_infos());
    }
    if !proj.unique_ids_mappings().is_empty() {
        write_section(&mut out, "UniqueIDMappings", proj.unique_ids_mappings());
    }

    out
}

/// Write a simple section with key=value pairs.
fn write_section(out: &mut String, name: &str, map: &IndexMap<String, String>) {
    out.push_str(&format!("[{}]\n", name));
    for (key, value) in map {
        out.push_str(&format!("{}={}\n", key, value));
    }
    out.push('\n');
}

/// Write an output group section with group-level keys followed by indexed output keys.
fn write_output_group(out: &mut String, name: &str, group: &crate::project::OutputGroupRaw) {
    out.push_str(&format!("[{}]\n", name));

    // Group-level keys first
    for (key, value) in group.keys() {
        out.push_str(&format!("{}={}\n", key, value));
    }

    // Per-output indexed keys: reconstruct OutputType1, OutputName1, etc.
    for (i, output) in group.outputs().iter().enumerate() {
        let idx = i + 1; // 1-based in the file
        for prefix in OUTPUT_KEY_ORDER {
            if let Some(value) = output.get(*prefix) {
                out.push_str(&format!("{}{idx}={}\n", prefix, value));
            }
        }
    }

    out.push('\n');
}

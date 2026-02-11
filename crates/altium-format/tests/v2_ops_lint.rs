// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Architecture enforcement lint for v2 ops modules.
//!
//! Ensures ops code uses the typed v2 record API (Record::from_origin() +
//! typed getters) instead of raw backing-store access (as_param()/as_binary()
//! + manual field extraction).
//!
//! Allowed exceptions must be annotated with `// LINT-ALLOW:` followed by
//! a justification. Common exceptions:
//! - Vertex data (SchWireRecord, SchPolylineRecord vertices are #[altium(skip)])
//! - Pad designator/layer (not yet in PcbPadRecord typed API)
//! - Footprint metadata (no typed record for generic metadata nodes)
//! - Component DESIGNATOR on SchDoc (stored on node but not in typed API)
//! - Record construction in manipulation commands (writing, not reading)

use std::path::Path;

/// Patterns that indicate raw backing-store access, bypassing the typed API.
const BANNED_PATTERNS: &[&str] = &[
    ".as_param()",
    ".params.get(",
    ".as_binary()",
    ".raw_block",
    "as_int_or(",
    "as_str()",
];

/// Marker comment that exempts a line from the lint.
const LINT_ALLOW_MARKER: &str = "LINT-ALLOW:";

/// Source files to check.
const OPS_FILES: &[&str] = &[
    "crates/altium-format/src/v2/ops/schlib.rs",
    "crates/altium-format/src/v2/ops/schdoc.rs",
    "crates/altium-format/src/v2/ops/pcblib.rs",
    "crates/altium-format/src/v2/ops/pcbdoc.rs",
];

/// Sections where raw access is expected (construction/write-back code).
/// Lines within these functions are excluded from the lint.
const WRITE_FUNCTIONS: &[&str] = &[
    "fn cmd_create(",
    "fn cmd_add_component(",
    "fn cmd_add_pin(",
    "fn cmd_add_footprint(",
    "fn cmd_add_pad(",
    "fn cmd_add_silkscreen(",
    "fn cmd_add_arc(",
    "fn cmd_add_json(",
    "fn cmd_add_pad_row(",
    "fn cmd_add_dual_row(",
    "fn cmd_add_quad_pads(",
    "fn cmd_add_pad_grid(",
    "fn cmd_gen_chip(",
    "fn build_pad_binary(",
    "fn build_track_binary(",
    "fn build_arc_binary(",
];

struct LintViolation {
    file: String,
    line_num: usize,
    line: String,
    pattern: String,
}

fn find_workspace_root() -> std::path::PathBuf {
    let mut dir = std::env::current_dir().unwrap();
    loop {
        if dir.join("Cargo.toml").exists() && dir.join("crates").exists() {
            return dir;
        }
        if !dir.pop() {
            panic!("Could not find workspace root");
        }
    }
}

fn is_in_write_function(lines: &[&str], current_line: usize) -> bool {
    // Walk backwards to find the enclosing function signature
    let mut brace_depth: i32 = 0;
    for i in (0..=current_line).rev() {
        let line = lines[i].trim();

        // Count braces to track scope
        for ch in line.chars() {
            match ch {
                '}' => brace_depth += 1,
                '{' => brace_depth -= 1,
                _ => {}
            }
        }

        // If we've gone above our current scope, stop
        if brace_depth < 0 {
            // Check if this line contains a write function signature
            for func in WRITE_FUNCTIONS {
                if line.contains(func) {
                    return true;
                }
            }
            return false;
        }

        // Also check function signatures at our scope level
        if brace_depth <= 0 {
            for func in WRITE_FUNCTIONS {
                if line.contains(func) {
                    return true;
                }
            }
        }
    }
    false
}

fn is_in_test_block(lines: &[&str], current_line: usize) -> bool {
    // Walk backwards to check if we're inside #[cfg(test)] or #[test]
    for i in (0..=current_line).rev() {
        let line = lines[i].trim();
        if line == "#[cfg(test)]" || line == "#[test]" {
            return true;
        }
        if line.starts_with("mod tests") {
            return true;
        }
        // Don't search too far back
        if current_line - i > 200 {
            break;
        }
    }
    false
}

fn lint_file(path: &Path) -> Vec<LintViolation> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let lines: Vec<&str> = content.lines().collect();
    let mut violations = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("//") {
            continue;
        }

        // Skip lines with LINT-ALLOW marker
        if line.contains(LINT_ALLOW_MARKER) {
            continue;
        }

        // Skip test code
        if is_in_test_block(&lines, i) {
            continue;
        }

        // Check each banned pattern
        for pattern in BANNED_PATTERNS {
            if trimmed.contains(pattern) {
                // Skip write/construction functions
                if is_in_write_function(&lines, i) {
                    continue;
                }

                violations.push(LintViolation {
                    file: path.display().to_string(),
                    line_num: i + 1,
                    line: trimmed.to_string(),
                    pattern: pattern.to_string(),
                });
            }
        }
    }

    violations
}

#[test]
fn v2_ops_no_raw_backing_store_access() {
    let root = find_workspace_root();
    let mut all_violations = Vec::new();

    for rel_path in OPS_FILES {
        let full_path = root.join(rel_path);
        if full_path.exists() {
            let violations = lint_file(&full_path);
            all_violations.extend(violations);
        }
    }

    if !all_violations.is_empty() {
        let mut msg = String::new();
        msg.push_str(&format!(
            "\n\n=== V2 OPS ARCHITECTURE LINT FAILED ===\n\
             Found {} raw backing-store access(es) in ops modules.\n\
             Use typed Record::from_origin() + getters instead.\n\
             To exempt a line, add a comment: // LINT-ALLOW: <reason>\n\n",
            all_violations.len()
        ));

        for v in &all_violations {
            msg.push_str(&format!(
                "  {}:{}: [{}]\n    {}\n\n",
                v.file, v.line_num, v.pattern, v.line
            ));
        }

        panic!("{}", msg);
    }
}

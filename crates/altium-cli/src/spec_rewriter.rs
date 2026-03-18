//! Spec rewriter: updates a `.pcbdoc-spec` file in-place after autoplace runs.
//!
//! Strategy: text-based rewriting. For each autoplace component in the solver result,
//! find the corresponding `place DESIGNATOR { ... }` block in the source text and replace
//! `autoplace: true` with `at: (x_mm, y_mm)` while preserving all other properties.
//! Multi-designator `place C1, C2, C3 { autoplace: true }` blocks are expanded to
//! individual blocks. Components not mentioned in any place block are appended at the
//! end of the `placement { }` block.

use std::collections::{HashMap, HashSet};

use autopcb_placement::{PlacementComponentState, PlacementResult};

/// Result of the spec rewrite operation.
pub struct RewriteResult {
    /// The updated spec text.
    pub text: String,
    /// Designators that were rewritten in-place (found a matching place block).
    pub rewritten_in_place: Vec<String>,
    /// Designators that were appended as new blocks.
    pub appended: Vec<String>,
}

/// Rewrite a `.pcbdoc-spec` source text to replace `autoplace: true` with concrete positions.
///
/// For each component in `result.components` whose designator is in `autoplace_designators`:
/// - If the component is mentioned in a `place` block that has `autoplace: true`:
///   - Replace the `autoplace: true` line with `at: (x_mm, y_mm)`
///   - Insert `rotation: N` after the `at:` line
///   - Add `// autoplace: solved` comment after the block opening brace
///   - Multi-designator blocks are expanded to individual single-designator blocks
/// - If the component is not mentioned in any place block:
///   - Append a new `place DESIGNATOR { at: (x_mm, y_mm), rotation: N }` block before
///     the closing `}` of the outermost `placement { }` block.
///
/// Non-autoplace content (constraints, groups, rules, clearance, optimize) is preserved
/// verbatim. Locked components (with `at:` and no `autoplace: true`) are unchanged.
pub fn rewrite_spec_with_placement(
    original_spec_text: &str,
    result: &PlacementResult,
    autoplace_designators: &[String],
) -> RewriteResult {
    let autoplace_set: HashSet<&str> =
        autoplace_designators.iter().map(|s| s.as_str()).collect();

    // Build a map from designator → PlacementComponentState for quick lookup.
    let state_map: HashMap<&str, &PlacementComponentState> =
        result.components.iter().map(|c| (c.designator.as_str(), c)).collect();

    // Only process components that are both in autoplace_designators and in result.components.
    let solvable: HashSet<&str> =
        autoplace_set.iter().copied().filter(|d| state_map.contains_key(d)).collect();

    // Find all place blocks in the source text.
    let place_blocks = find_place_blocks(original_spec_text);

    // Track which designators we handle via in-place rewrite vs append.
    let mut rewritten_in_place: Vec<String> = Vec::new();
    let mut appended: Vec<String> = Vec::new();

    // Categorize place blocks:
    // - blocks that contain at least one autoplace designator → need rewriting
    // - all other blocks → preserved verbatim
    let mut blocks_to_rewrite: Vec<&PlaceBlock> = Vec::new();
    let mut autoplace_handled: HashSet<&str> = HashSet::new();

    for block in &place_blocks {
        let has_autoplace_designator = block.designators.iter().any(|d| solvable.contains(d.as_str()));
        if has_autoplace_designator && block.has_autoplace {
            blocks_to_rewrite.push(block);
            for d in &block.designators {
                if solvable.contains(d.as_str()) {
                    autoplace_handled.insert(d.as_str());
                }
            }
        }
    }

    // Which solvable designators have no matching place block → need appending.
    let to_append: Vec<&str> = solvable
        .iter()
        .copied()
        .filter(|d| !autoplace_handled.contains(d))
        .collect();

    // Build the rewritten text by replacing blocks in reverse order (to preserve offsets).
    let mut replacements: Vec<(usize, usize, String)> = Vec::new();

    for block in &blocks_to_rewrite {
        let replacement = rewrite_block(block, &state_map, original_spec_text);
        for designator in &block.designators {
            if solvable.contains(designator.as_str()) {
                rewritten_in_place.push(designator.clone());
            }
        }
        replacements.push((block.start, block.end, replacement));
    }

    // Sort replacements in reverse order so we can apply them without offset drift.
    replacements.sort_by(|a, b| b.0.cmp(&a.0));

    let mut output = original_spec_text.to_owned();
    for (start, end, replacement) in replacements {
        output.replace_range(start..end, &replacement);
    }

    // Append new blocks for designators not found in any place block.
    if !to_append.is_empty() {
        let mut sorted_appended: Vec<&str> = to_append;
        sorted_appended.sort_unstable();

        // Find the closing `}` of the `placement { ... }` block.
        // We look for the last `}` that closes the top-level placement block.
        let append_text = build_append_text(&sorted_appended, &state_map);

        if let Some(insert_pos) = find_placement_block_close(&output) {
            output.insert_str(insert_pos, &append_text);
        } else {
            // No placement block found; append at end.
            output.push_str("\nplacement {\n");
            output.push_str(&append_text);
            output.push_str("}\n");
        }

        for d in &sorted_appended {
            appended.push(d.to_string());
        }
    }

    RewriteResult { text: output, rewritten_in_place, appended }
}

// ── Block scanner ─────────────────────────────────────────────────────────────

/// A parsed `place DESIGNATOR(S) { ... }` block found in the source text.
#[derive(Debug)]
struct PlaceBlock {
    /// Byte offset of the start of the `place` keyword.
    start: usize,
    /// Byte offset of the character *after* the closing `}`.
    end: usize,
    /// Designators listed in `place D1, D2, ... { ... }`.
    designators: Vec<String>,
    /// Whether the block body contains `autoplace: true` or `autoplace:true`.
    has_autoplace: bool,
    /// Byte offset of the `{` that opens the block body.
    brace_open: usize,
}

/// Scan `text` for `place ...  { ... }` blocks at the top level of a `placement { }` block.
///
/// Uses a simple brace-depth scanner — does not handle nested `placement {}` blocks.
fn find_place_blocks(text: &str) -> Vec<PlaceBlock> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut result = Vec::new();
    let mut i = 0;

    while i < len {
        // Skip whitespace.
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= len {
            break;
        }

        // Skip line comments: `//` through end of line.
        if text[i..].starts_with("//") {
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Look for the keyword `place` followed by whitespace or a designator character.
        if text[i..].starts_with("place") {
            let after_kw = i + 5;
            if after_kw < len && (bytes[after_kw].is_ascii_whitespace() || bytes[after_kw] == b',') {
                // Parse designators until `{`.
                let kw_start = i;
                let mut j = after_kw;

                // Skip whitespace after `place`.
                while j < len && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }

                // Collect designators (comma-separated identifiers).
                let mut designators: Vec<String> = Vec::new();
                while j < len && bytes[j] != b'{' && bytes[j] != b'\n' {
                    // Skip whitespace and commas.
                    while j < len && (bytes[j].is_ascii_whitespace() || bytes[j] == b',') {
                        j += 1;
                    }
                    if j >= len || bytes[j] == b'{' || bytes[j] == b'\n' {
                        break;
                    }
                    // Read a designator token (alphanumeric + _ + . + / + -).
                    let start_tok = j;
                    while j < len
                        && !bytes[j].is_ascii_whitespace()
                        && bytes[j] != b','
                        && bytes[j] != b'{'
                    {
                        j += 1;
                    }
                    if j > start_tok {
                        designators.push(text[start_tok..j].to_owned());
                    }
                }

                // Skip to `{`.
                while j < len && bytes[j] != b'{' {
                    j += 1;
                }
                if j >= len {
                    break;
                }
                let brace_open = j;
                j += 1; // consume `{`

                // Now scan for the matching `}` handling nested braces.
                let body_start = j;
                let mut depth = 1usize;
                while j < len && depth > 0 {
                    if bytes[j] == b'{' {
                        depth += 1;
                    } else if bytes[j] == b'}' {
                        depth -= 1;
                    }
                    j += 1;
                }
                let block_end = j; // char after closing `}`

                if !designators.is_empty() {
                    let has_autoplace =
                        find_autoplace_line(text, body_start, block_end.saturating_sub(1));

                    result.push(PlaceBlock {
                        start: kw_start,
                        end: block_end,
                        designators,
                        has_autoplace,
                        brace_open,
                    });
                    i = block_end;
                    continue;
                }
            }
        }

        // Skip to end of line.
        while i < len && bytes[i] != b'\n' {
            i += 1;
        }
        if i < len {
            i += 1;
        }
    }

    result
}

/// Search for `autoplace: true` or `autoplace:true` in the block body.
///
/// Returns `true` if found.
fn find_autoplace_line(text: &str, body_start: usize, body_end: usize) -> bool {
    let body = &text[body_start..body_end];
    let bytes = text.as_bytes();

    for (rel, _) in body.match_indices("autoplace") {
        let abs = body_start + rel;
        let after_kw = abs + "autoplace".len();
        let mut k = after_kw;
        while k < body_end && bytes[k].is_ascii_whitespace() && bytes[k] != b'\n' {
            k += 1;
        }
        if k < body_end && bytes[k] == b':' {
            k += 1;
            while k < body_end && bytes[k].is_ascii_whitespace() && bytes[k] != b'\n' {
                k += 1;
            }
            if text[k..].starts_with("true") {
                return true;
            }
        }
    }

    false
}

// ── Block rewriter ────────────────────────────────────────────────────────────

/// Rewrite a single `place` block.
///
/// - If block has a single designator and it's in `state_map`: replace `autoplace: true`
///   with `at: (x, y)` + `rotation: N`, add `// autoplace: solved` comment.
/// - If block has multiple designators, split into one block per designator.
///   Locked designators (not in `state_map`) get their own block with unchanged content.
fn rewrite_block(
    block: &PlaceBlock,
    state_map: &HashMap<&str, &PlacementComponentState>,
    original: &str,
) -> String {
    // Extract the block body (between `{` and `}`).
    let body_start = block.brace_open + 1;
    let body_end = block.end.saturating_sub(1);
    let body = &original[body_start..body_end];

    // Detect indentation from the `place` keyword line.
    let line_start = original[..block.start].rfind('\n').map(|p| p + 1).unwrap_or(0);
    let indent = detect_indent(&original[line_start..block.start]);
    let inner_indent = format!("{}    ", indent);

    if block.designators.len() == 1 {
        let d = &block.designators[0];
        if let Some(state) = state_map.get(d.as_str()) {
            rewrite_single_block(d, body, state, &indent, &inner_indent)
        } else {
            // Designator not in solver result — preserve verbatim.
            original[block.start..block.end].to_owned()
        }
    } else {
        // Multi-designator block: expand to individual blocks.
        let mut out = String::new();
        for (i, d) in block.designators.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            if let Some(state) = state_map.get(d.as_str()) {
                let new_block = rewrite_single_block(d, body, state, &indent, &inner_indent);
                out.push_str(&new_block);
            } else {
                // Designator not yet solved — preserve autoplace: true so it remains
                // schedulable for the next solver run.
                out.push_str(&format!(
                    "{}place {} {{\n{}// autoplace: unsolved\n{}}}\n",
                    indent,
                    d,
                    body,
                    indent
                ));
            }
        }
        out
    }
}

/// Rewrite the body of a single-designator place block that has `autoplace: true`.
fn rewrite_single_block(
    designator: &str,
    body: &str,
    state: &PlacementComponentState,
    indent: &str,
    inner_indent: &str,
) -> String {
    let at_line = format!(
        "{}at: ({:.4}mm, {:.4}mm)",
        inner_indent, state.x_mm, state.y_mm
    );
    let rotation_line = format!("{}rotation: {:.1}", inner_indent, state.rotation_deg);

    // Build new body: replace autoplace line with at + rotation, keep everything else.
    let new_body = replace_autoplace_with_position(body, &at_line, &rotation_line, inner_indent);

    format!(
        "{}place {} {{\n{}{}// autoplace: solved\n{}}}\n",
        indent,
        designator,
        if new_body.is_empty() || new_body.ends_with('\n') {
            new_body
        } else {
            format!("{}\n", new_body)
        },
        inner_indent,
        indent
    )
}

/// Replace the `autoplace: true` line in `body` with `at:` and `rotation:` lines.
/// Other lines are preserved verbatim (with leading/trailing blank lines stripped).
fn replace_autoplace_with_position(body: &str, at_line: &str, rotation_line: &str, _inner_indent: &str) -> String {
    let mut lines_out: Vec<String> = Vec::new();
    let mut inserted = false;

    for line in body.lines() {
        let trimmed = line.trim();
        if is_autoplace_true_line(trimmed) {
            if !inserted {
                lines_out.push(at_line.to_owned());
                lines_out.push(rotation_line.to_owned());
                inserted = true;
            }
            // Drop the autoplace: true line.
        } else {
            lines_out.push(line.to_owned());
        }
    }

    // If no autoplace line was found (shouldn't happen), append at the end.
    if !inserted {
        lines_out.push(at_line.to_owned());
        lines_out.push(rotation_line.to_owned());
    }

    // Remove leading blank lines.
    while lines_out.first().map(|l| l.trim().is_empty()).unwrap_or(false) {
        lines_out.remove(0);
    }

    if lines_out.is_empty() {
        String::new()
    } else {
        let mut out = lines_out.join("\n");
        out.push('\n');
        out
    }
}

/// Returns true if the (trimmed) line is `autoplace: true` or `autoplace:true`.
fn is_autoplace_true_line(trimmed: &str) -> bool {
    if let Some(rest) = trimmed.strip_prefix("autoplace") {
        let rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace());
        if let Some(rest) = rest.strip_prefix(':') {
            let rest = rest.trim();
            return rest == "true" || rest.starts_with("true,") || rest.starts_with("true//") || rest.starts_with("true ");
        }
    }
    false
}

/// Detect the leading whitespace (indentation) of a string.
fn detect_indent(s: &str) -> String {
    s.chars()
        .take_while(|c| c.is_ascii_whitespace())
        .collect()
}

// ── Appender ──────────────────────────────────────────────────────────────────

/// Build the text to append for new place blocks (for designators not found in source).
fn build_append_text(
    designators: &[&str],
    state_map: &HashMap<&str, &PlacementComponentState>,
) -> String {
    let mut out = String::new();
    for &d in designators {
        if let Some(state) = state_map.get(d) {
            out.push_str(&format!(
                "    place {} {{\n        at: ({:.4}mm, {:.4}mm)\n        rotation: {:.1}\n        // autoplace: solved\n    }}\n",
                d, state.x_mm, state.y_mm, state.rotation_deg
            ));
        }
    }
    out
}

/// Find the byte offset of the closing `}` of the outermost `placement { }` block.
///
/// Returns the position just *before* the closing `}` so we can insert text there.
fn find_placement_block_close(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let len = bytes.len();

    // Find `placement` keyword.
    let mut i = 0;
    while i < len {
        if text[i..].starts_with("placement") {
            let after_kw = i + "placement".len();
            if after_kw >= len {
                break;
            }
            // Must be followed by whitespace or `{`.
            if bytes[after_kw].is_ascii_whitespace() || bytes[after_kw] == b'{' {
                // Skip to `{`.
                let mut j = after_kw;
                while j < len && bytes[j] != b'{' {
                    j += 1;
                }
                if j >= len {
                    break;
                }
                j += 1; // consume `{`

                // Scan for matching `}`.
                let mut depth = 1usize;
                while j < len && depth > 0 {
                    if bytes[j] == b'{' {
                        depth += 1;
                    } else if bytes[j] == b'}' {
                        depth -= 1;
                        if depth == 0 {
                            return Some(j);
                        }
                    }
                    j += 1;
                }
            }
        }
        i += 1;
    }
    None
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use autopcb_placement::{PlacementComponentState, PlacementResult};

    fn make_result(components: Vec<(&str, f64, f64, f64)>) -> PlacementResult {
        PlacementResult {
            status: "ok".to_string(),
            total_iterations: 10,
            duration_ms: 100,
            hpwl_estimate_mm: 50.0,
            overlap_violations: 0,
            snapshots: vec![],
            components: components
                .into_iter()
                .map(|(d, x, y, r)| PlacementComponentState {
                    designator: d.to_string(),
                    x_mm: x,
                    y_mm: y,
                    rotation_deg: r,
                })
                .collect(),
        }
    }

    #[test]
    fn autoplace_true_replaced_with_position() {
        let spec = r#"
placement {
    place U1 {
        autoplace: true
        region: center
    }
}
"#;
        let result = make_result(vec![("U1", 10.5, 20.3, 0.0)]);
        let autoplace = vec!["U1".to_string()];

        let rw = rewrite_spec_with_placement(spec, &result, &autoplace);
        assert!(rw.rewritten_in_place.contains(&"U1".to_string()));
        assert!(rw.appended.is_empty());
        assert!(
            rw.text.contains("at: (10.5000mm, 20.3000mm)"),
            "expected at: line, got:\n{}",
            rw.text
        );
        assert!(rw.text.contains("rotation: 0.0"), "expected rotation line");
        assert!(!rw.text.contains("autoplace: true"), "autoplace: true must be removed");
        assert!(rw.text.contains("region: center"), "other properties must be preserved");
        assert!(rw.text.contains("// autoplace: solved"), "solved comment must be present");
    }

    #[test]
    fn locked_components_unchanged() {
        let spec = r#"
placement {
    place U1 {
        at: (5.0mm, 5.0mm)
        rotation: 90.0
    }
    place U2 {
        autoplace: true
    }
}
"#;
        let result = make_result(vec![("U2", 15.0, 25.0, 0.0)]);
        let autoplace = vec!["U2".to_string()];

        let rw = rewrite_spec_with_placement(spec, &result, &autoplace);
        // U1 locked block preserved verbatim.
        assert!(rw.text.contains("at: (5.0mm, 5.0mm)"), "U1 at: must be preserved");
        assert!(rw.text.contains("rotation: 90.0"), "U1 rotation must be preserved");
        // U2 solved.
        assert!(rw.text.contains("at: (15.0000mm, 25.0000mm)"), "U2 must have solved position");
        assert!(rw.rewritten_in_place.contains(&"U2".to_string()));
    }

    #[test]
    fn unmentioned_autoplace_components_appended() {
        let spec = r#"
placement {
    place U1 {
        at: (5.0mm, 5.0mm)
    }
}
"#;
        // R1 is in the autoplace set but has no place block in spec.
        let result = make_result(vec![("U1", 5.0, 5.0, 0.0), ("R1", 30.0, 40.0, 90.0)]);
        let autoplace = vec!["R1".to_string()];

        let rw = rewrite_spec_with_placement(spec, &result, &autoplace);
        assert!(rw.appended.contains(&"R1".to_string()));
        assert!(rw.text.contains("place R1"), "R1 block must be appended");
        assert!(rw.text.contains("at: (30.0000mm, 40.0000mm)"), "R1 position must be correct");
        assert!(rw.text.contains("rotation: 90.0"), "R1 rotation must be correct");
    }

    #[test]
    fn multi_designator_block_expanded_to_individual() {
        let spec = r#"
placement {
    place C1, C2, C3 {
        autoplace: true
    }
}
"#;
        let result = make_result(vec![
            ("C1", 10.0, 10.0, 0.0),
            ("C2", 20.0, 10.0, 0.0),
            ("C3", 30.0, 10.0, 0.0),
        ]);
        let autoplace = vec!["C1".to_string(), "C2".to_string(), "C3".to_string()];

        let rw = rewrite_spec_with_placement(spec, &result, &autoplace);
        assert!(rw.text.contains("place C1"), "C1 individual block required");
        assert!(rw.text.contains("place C2"), "C2 individual block required");
        assert!(rw.text.contains("place C3"), "C3 individual block required");
        assert!(!rw.text.contains("C1, C2"), "multi-designator block must not remain");
        assert!(rw.rewritten_in_place.contains(&"C1".to_string()));
        assert!(rw.rewritten_in_place.contains(&"C2".to_string()));
        assert!(rw.rewritten_in_place.contains(&"C3".to_string()));
    }

    #[test]
    fn constraints_and_clearance_blocks_preserved() {
        let spec = r#"
placement {
    clearance {
        all: 0.5mm
    }
    optimize {
        ratsnest: true
    }
    left_of U1, U2 {
        gap: 2mm
    }
    place U1 {
        autoplace: true
    }
}
"#;
        let result = make_result(vec![("U1", 10.0, 10.0, 0.0)]);
        let autoplace = vec!["U1".to_string()];

        let rw = rewrite_spec_with_placement(spec, &result, &autoplace);
        assert!(rw.text.contains("clearance {"), "clearance block must be preserved");
        assert!(rw.text.contains("all: 0.5mm"), "clearance value must be preserved");
        assert!(rw.text.contains("optimize {"), "optimize block must be preserved");
        assert!(rw.text.contains("ratsnest: true"), "optimize content must be preserved");
        assert!(rw.text.contains("left_of"), "directional constraint must be preserved");
    }

    #[test]
    fn rotation_nonzero_included_in_output() {
        let spec = r#"
placement {
    place Q1 {
        autoplace: true
    }
}
"#;
        let result = make_result(vec![("Q1", 5.0, 8.0, 270.0)]);
        let autoplace = vec!["Q1".to_string()];

        let rw = rewrite_spec_with_placement(spec, &result, &autoplace);
        assert!(rw.text.contains("rotation: 270.0"), "rotation must be 270.0");
        assert!(rw.text.contains("at: (5.0000mm, 8.0000mm)"));
    }
}

//! Spec rewriter: updates a `.pcbdoc-spec` file in-place after autoplace runs.
//!
//! Strategy: AST-based rewriting with trivia (comment) preservation. Parses the
//! source with `parse_with_trivia()` to obtain the typed AST and a `TriviaMap`
//! that associates comments with `PlaceDecl` nodes. For each autoplace component
//! in the solver result, finds the corresponding `PlaceDecl` in the AST and
//! replaces its byte span with formatter-generated text that re-attaches leading
//! and trailing trivia. All source text outside replacement spans is preserved
//! verbatim.

use std::collections::{HashMap, HashSet};

use altium_format_spec::ast::{Expr, ObjectItem, PlaceDecl, PlacementItem, SpecItem};
use altium_format_spec::diagnostic::Span;
use altium_format_spec::trivia::{TriviaMap, parse_with_trivia};
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
///   - Replace the entire `PlaceDecl` span with a solved block containing `at:` and `rotation:`
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
) -> anyhow::Result<RewriteResult> {
    let autoplace_set: HashSet<&str> = autoplace_designators.iter().map(|s| s.as_str()).collect();

    let state_map: HashMap<&str, &PlacementComponentState> = result
        .components
        .iter()
        .map(|c| (c.designator.as_str(), c))
        .collect();

    let solvable: HashSet<&str> = autoplace_set
        .iter()
        .copied()
        .filter(|d| state_map.contains_key(d))
        .collect();

    let (ast, trivia_map) = parse_with_trivia(original_spec_text)
        .map_err(|e| anyhow::anyhow!("failed to parse spec for rewriting: {e}"))?;

    // Find the PlacementDecl in the AST.
    let placement_item = ast
        .items
        .iter()
        .find(|item| matches!(&item.node, SpecItem::Placement(_)));

    let placement_item = match placement_item {
        Some(p) => p,
        None => {
            // No placement block → output identical to input.
            return Ok(RewriteResult {
                text: original_spec_text.to_owned(),
                rewritten_in_place: Vec::new(),
                appended: Vec::new(),
            });
        }
    };

    let placement_decl = match &placement_item.node {
        SpecItem::Placement(p) => p,
        _ => unreachable!(),
    };

    // Collect replacements: (span_start, span_end, replacement_text).
    let mut replacements: Vec<(u32, u32, String)> = Vec::new();
    let mut rewritten_in_place: Vec<String> = Vec::new();
    let mut autoplace_handled: HashSet<String> = HashSet::new();

    for pitem in &placement_decl.body {
        let PlacementItem::Place(place) = &pitem.node else {
            continue;
        };

        let block_desigs: Vec<String> = place.designators.iter().map(|d| d.node.as_str()).collect();

        let has_any_autoplace_desig = block_desigs
            .iter()
            .any(|d| solvable.contains(d.as_str()) && has_autoplace(place));

        if !has_any_autoplace_desig {
            continue;
        }

        let place_span = pitem.span;
        let indent = detect_indent_from_span(original_spec_text, place_span.start);
        let inner_indent = format!("{}    ", indent);

        let replacement_text = if block_desigs.len() == 1 {
            let d = &block_desigs[0];
            let state = state_map.get(d.as_str());
            build_replacement_text(
                d,
                place,
                state,
                original_spec_text,
                &indent,
                &inner_indent,
                &trivia_map,
                place_span,
            )
        } else {
            // Multi-designator: expand to individual blocks.
            let mut out = String::new();
            for (i, d) in block_desigs.iter().enumerate() {
                if i > 0 {
                    out.push('\n');
                }
                let state = state_map.get(d.as_str());
                // Span::new(0,0) is a sentinel meaning "suppress trivia for this block".
                // A real PlaceDecl cannot start at byte 0 — the `placement { }` wrapper
                // always precedes it. Guards in build_*_replacement detect this via
                // `if place_span.start > 0`.
                let block_span = if i == 0 { place_span } else { Span::new(0, 0) };
                let block_text = build_replacement_text(
                    d,
                    place,
                    state,
                    original_spec_text,
                    &indent,
                    &inner_indent,
                    &trivia_map,
                    block_span,
                );
                out.push_str(&block_text);
            }
            out
        };

        for d in &block_desigs {
            if solvable.contains(d.as_str()) {
                rewritten_in_place.push(d.clone());
                autoplace_handled.insert(d.clone());
            }
        }
        replacements.push((place_span.start, place_span.end, replacement_text));
    }

    // Which solvable designators have no matching place block → need appending.
    let mut to_append: Vec<&str> = solvable
        .iter()
        .copied()
        .filter(|d| !autoplace_handled.contains(*d))
        .collect();
    to_append.sort_unstable();

    // Sort replacements in reverse byte order and apply.
    replacements.sort_by(|a, b| b.0.cmp(&a.0));

    let mut output = original_spec_text.to_owned();
    for (start, end, replacement) in replacements {
        output.replace_range(start as usize..end as usize, &replacement);
    }

    // Append new blocks for designators not found in any place block.
    let mut appended: Vec<String> = Vec::new();
    if !to_append.is_empty() {
        let append_text = build_append_text(&to_append, &state_map);

        if let Some(insert_pos) = find_placement_block_close(&output) {
            output.insert_str(insert_pos, &append_text);
        } else {
            output.push_str("\nplacement {\n");
            output.push_str(&append_text);
            output.push_str("}\n");
        }

        for d in &to_append {
            appended.push(d.to_string());
        }
    }

    Ok(RewriteResult {
        text: output,
        rewritten_in_place,
        appended,
    })
}

// ── AST helpers ───────────────────────────────────────────────────────────────

/// Returns true if the `PlaceDecl` body contains `autoplace: true`.
fn has_autoplace(place: &PlaceDecl) -> bool {
    place.body.node.items.iter().any(|item| {
        if let ObjectItem::Property(prop) = &item.node {
            prop.key.node == "autoplace" && matches!(prop.value.node, Expr::Bool(true))
        } else {
            false
        }
    })
}

// ── Replacement text builders ─────────────────────────────────────────────────

/// Build the replacement text for a single designator's place block.
///
/// If the component is solved, emits a block with `at:` and `rotation:` and a
/// `// autoplace: solved` annotation. If not solved (missing from state_map),
/// emits the original body with `// autoplace: unsolved` annotation.
fn build_replacement_text(
    designator: &str,
    place: &PlaceDecl,
    state: Option<&&PlacementComponentState>,
    source: &str,
    indent: &str,
    inner_indent: &str,
    trivia: &TriviaMap,
    place_span: Span,
) -> String {
    match state {
        Some(state) => build_solved_replacement(
            designator,
            place,
            state,
            source,
            indent,
            inner_indent,
            trivia,
            place_span,
        ),
        None => build_unsolved_replacement(
            designator,
            place,
            source,
            indent,
            inner_indent,
            trivia,
            place_span,
        ),
    }
}

/// Build replacement text for a solved (positioned) component.
fn build_solved_replacement(
    designator: &str,
    place: &PlaceDecl,
    state: &PlacementComponentState,
    source: &str,
    indent: &str,
    inner_indent: &str,
    trivia: &TriviaMap,
    place_span: Span,
) -> String {
    let mut out = String::new();

    // Emit leading trivia (comments before this block).
    // place_span.start == 0 is the sentinel from multi-designator expansion
    // for non-first blocks; skip trivia attachment for those.
    if place_span.start > 0 {
        for comment in trivia.leading(place_span) {
            out.push_str(indent);
            out.push_str(&comment.text);
            out.push('\n');
        }
    }

    // Place header.
    out.push_str(indent);
    out.push_str(&format!("place {} {{\n", designator));

    // Emit non-autoplace properties verbatim from source, with intra-body comments.
    let body_start = place.body.span.start + 1; // byte after `{`
    let mut prev_end = body_start;
    for item in &place.body.node.items {
        // Always emit comments between the previous item and this one,
        // even if we're about to skip `autoplace: true`.
        emit_intra_body_comments(&mut out, trivia, prev_end, item.span.start, inner_indent);
        prev_end = item.span.end;
        if let ObjectItem::Property(prop) = &item.node {
            if prop.key.node == "autoplace" {
                continue;
            }
        }
        let item_text = &source[item.span.start as usize..item.span.end as usize];
        out.push_str(inner_indent);
        out.push_str(item_text.trim());
        out.push('\n');
    }
    // Emit any trailing intra-body comments after the last item.
    let body_end = place.body.span.end.saturating_sub(1); // byte before `}`
    emit_intra_body_comments(&mut out, trivia, prev_end, body_end, inner_indent);

    // Add placement result.
    out.push_str(&format!(
        "{}at: ({:.4}mm, {:.4}mm)\n",
        inner_indent, state.x_mm, state.y_mm
    ));
    out.push_str(&format!(
        "{}rotation: {:.1}\n",
        inner_indent, state.rotation_deg
    ));

    // Solved annotation.
    out.push_str(&format!("{}// autoplace: solved\n", inner_indent));

    // Closing brace.
    out.push_str(indent);
    out.push('}');

    // Trailing trivia.
    if place_span.start > 0 {
        if let Some(trailing) = trivia.trailing(place_span) {
            out.push(' ');
            out.push_str(&trailing.text);
        }
    }
    out.push('\n');

    out
}

/// Build replacement text for an unsolved (not in state_map) component.
///
/// Preserves the original body properties but adds `// autoplace: unsolved` annotation.
fn build_unsolved_replacement(
    designator: &str,
    place: &PlaceDecl,
    source: &str,
    indent: &str,
    inner_indent: &str,
    trivia: &TriviaMap,
    place_span: Span,
) -> String {
    let mut out = String::new();

    // place_span.start == 0 is the sentinel from multi-designator expansion
    // for non-first blocks; skip trivia attachment for those.
    if place_span.start > 0 {
        for comment in trivia.leading(place_span) {
            out.push_str(indent);
            out.push_str(&comment.text);
            out.push('\n');
        }
    }

    out.push_str(indent);
    out.push_str(&format!("place {} {{\n", designator));

    // Preserve all original properties verbatim, with intra-body comments.
    let body_start = place.body.span.start + 1;
    let mut prev_end = body_start;
    for item in &place.body.node.items {
        emit_intra_body_comments(&mut out, trivia, prev_end, item.span.start, inner_indent);
        let item_text = &source[item.span.start as usize..item.span.end as usize];
        out.push_str(inner_indent);
        out.push_str(item_text.trim());
        out.push('\n');
        prev_end = item.span.end;
    }
    let body_end = place.body.span.end.saturating_sub(1);
    emit_intra_body_comments(&mut out, trivia, prev_end, body_end, inner_indent);

    out.push_str(&format!("{}// autoplace: unsolved\n", inner_indent));
    out.push_str(indent);
    out.push('}');

    if place_span.start > 0 {
        if let Some(trailing) = trivia.trailing(place_span) {
            out.push(' ');
            out.push_str(&trailing.text);
        }
    }
    out.push('\n');

    out
}

/// Emit any comments that fall within the byte range `[prev_end, next_start)`.
fn emit_intra_body_comments(
    out: &mut String,
    trivia: &TriviaMap,
    prev_end: u32,
    next_start: u32,
    inner_indent: &str,
) {
    for comment in trivia.in_range(prev_end, next_start) {
        out.push_str(inner_indent);
        out.push_str(&comment.text);
        out.push('\n');
    }
}

// ── Indentation detection ─────────────────────────────────────────────────────

/// Detect the base indentation of the `place` keyword by scanning backward to the
/// most recent newline.
fn detect_indent_from_span(source: &str, span_start: u32) -> String {
    let before = &source[..span_start as usize];
    let line_start = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
    before[line_start..span_start as usize]
        .chars()
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

    let mut i = 0;
    while i < len {
        if text[i..].starts_with("placement") {
            let after_kw = i + "placement".len();
            if after_kw >= len {
                break;
            }
            if bytes[after_kw].is_ascii_whitespace() || bytes[after_kw] == b'{' {
                let mut j = after_kw;
                while j < len && bytes[j] != b'{' {
                    j += 1;
                }
                if j >= len {
                    break;
                }
                j += 1; // consume `{`

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

    fn rewrite(spec: &str, result: PlacementResult, autoplace: Vec<&str>) -> RewriteResult {
        let autoplace: Vec<String> = autoplace.into_iter().map(|s| s.to_string()).collect();
        rewrite_spec_with_placement(spec, &result, &autoplace)
            .expect("rewrite_spec_with_placement failed")
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
        let rw = rewrite(spec, result, vec!["U1"]);
        assert!(rw.rewritten_in_place.contains(&"U1".to_string()));
        assert!(rw.appended.is_empty());
        assert!(
            rw.text.contains("at: (10.5000mm, 20.3000mm)"),
            "expected at: line, got:\n{}",
            rw.text
        );
        assert!(rw.text.contains("rotation: 0.0"), "expected rotation line");
        assert!(
            !rw.text.contains("autoplace: true"),
            "autoplace: true must be removed"
        );
        assert!(
            rw.text.contains("region: center"),
            "other properties must be preserved"
        );
        assert!(
            rw.text.contains("// autoplace: solved"),
            "solved comment must be present"
        );
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
        let rw = rewrite(spec, result, vec!["U2"]);
        // U1 locked block preserved verbatim.
        assert!(
            rw.text.contains("at: (5.0mm, 5.0mm)"),
            "U1 at: must be preserved"
        );
        assert!(
            rw.text.contains("rotation: 90.0"),
            "U1 rotation must be preserved"
        );
        // U2 solved.
        assert!(
            rw.text.contains("at: (15.0000mm, 25.0000mm)"),
            "U2 must have solved position"
        );
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
        let result = make_result(vec![("U1", 5.0, 5.0, 0.0), ("R1", 30.0, 40.0, 90.0)]);
        let rw = rewrite(spec, result, vec!["R1"]);
        assert!(rw.appended.contains(&"R1".to_string()));
        assert!(rw.text.contains("place R1"), "R1 block must be appended");
        assert!(
            rw.text.contains("at: (30.0000mm, 40.0000mm)"),
            "R1 position must be correct"
        );
        assert!(
            rw.text.contains("rotation: 90.0"),
            "R1 rotation must be correct"
        );
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
        let rw = rewrite(spec, result, vec!["C1", "C2", "C3"]);
        assert!(rw.text.contains("place C1"), "C1 individual block required");
        assert!(rw.text.contains("place C2"), "C2 individual block required");
        assert!(rw.text.contains("place C3"), "C3 individual block required");
        assert!(
            !rw.text.contains("C1, C2"),
            "multi-designator block must not remain"
        );
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
    place U1 {
        autoplace: true
    }
}
"#;
        let result = make_result(vec![("U1", 10.0, 10.0, 0.0)]);
        let rw = rewrite(spec, result, vec!["U1"]);
        assert!(
            rw.text.contains("clearance {"),
            "clearance block must be preserved"
        );
        assert!(
            rw.text.contains("all: 0.5mm"),
            "clearance value must be preserved"
        );
        assert!(
            rw.text.contains("optimize {"),
            "optimize block must be preserved"
        );
        assert!(
            rw.text.contains("ratsnest: true"),
            "optimize content must be preserved"
        );
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
        let rw = rewrite(spec, result, vec!["Q1"]);
        assert!(
            rw.text.contains("rotation: 270.0"),
            "rotation must be 270.0"
        );
        assert!(rw.text.contains("at: (5.0000mm, 8.0000mm)"));
    }

    #[test]
    fn comment_before_place_block_preserved() {
        let spec = r#"placement {
// this is U1
place U1 {
    autoplace: true
}
}"#;
        let result = make_result(vec![("U1", 1.0, 2.0, 0.0)]);
        let rw = rewrite(spec, result, vec!["U1"]);
        assert!(
            rw.text.contains("// this is U1"),
            "comment before block must be preserved:\n{}",
            rw.text
        );
    }

    #[test]
    fn comment_inside_place_body_preserved() {
        let spec = r#"placement {
    place U1 {
        region: center
        // this resistor must be near U2
        autoplace: true
    }
}"#;
        let result = make_result(vec![("U1", 1.0, 2.0, 0.0)]);
        let rw = rewrite(spec, result, vec!["U1"]);
        assert!(
            rw.text.contains("region: center"),
            "region property must be preserved"
        );
        assert!(
            rw.text.contains("// this resistor must be near U2"),
            "intra-body comment must be preserved:\n{}",
            rw.text
        );
    }

    #[test]
    fn trailing_comment_on_closing_brace_preserved() {
        let spec = "placement {\nplace U1 {\n    autoplace: true\n} // my comment\n}";
        let result = make_result(vec![("U1", 1.0, 2.0, 0.0)]);
        let rw = rewrite(spec, result, vec!["U1"]);
        assert!(
            rw.text.contains("// my comment"),
            "trailing comment must be preserved:\n{}",
            rw.text
        );
    }

    #[test]
    fn no_placement_block_output_identical_to_input() {
        let spec = "component FOO {\n    value: \"bar\"\n}\n";
        let result = make_result(vec![("U1", 1.0, 2.0, 0.0)]);
        let rw = rewrite(spec, result, vec!["U1"]);
        assert_eq!(
            rw.text, spec,
            "output must equal input when no placement block"
        );
        assert!(rw.rewritten_in_place.is_empty());
        assert!(rw.appended.is_empty());
    }

    #[test]
    fn all_components_locked_output_identical_to_input() {
        let spec = r#"
placement {
    place U1 {
        at: (5.0mm, 5.0mm)
        rotation: 0.0
    }
}
"#;
        let result = make_result(vec![]);
        let rw = rewrite(spec, result, vec![]);
        assert_eq!(
            rw.text, spec,
            "output must equal input when all components locked"
        );
        assert!(rw.rewritten_in_place.is_empty());
        assert!(rw.appended.is_empty());
    }

    #[test]
    fn roundtrip_rewrite_then_reparse() {
        let spec = r#"placement {
    // leading comment
    place C1 {
        autoplace: true
    } // trailing
}
"#;
        let result = make_result(vec![("C1", 3.0, 4.0, 90.0)]);
        let rw = rewrite(spec, result, vec!["C1"]);
        // Re-parse the output to ensure no parse errors.
        let reparse = parse_with_trivia(&rw.text);
        assert!(
            reparse.is_ok(),
            "rewritten spec failed to parse: {:?}\nSpec was:\n{}",
            reparse.err(),
            rw.text
        );
    }

    #[test]
    fn comment_between_two_place_blocks_preserved() {
        let spec = r#"placement {
place U1 {
    autoplace: true
}
// between
place U2 {
    autoplace: true
}
}"#;
        let result = make_result(vec![("U1", 1.0, 2.0, 0.0), ("U2", 3.0, 4.0, 0.0)]);
        let rw = rewrite(spec, result, vec!["U1", "U2"]);
        assert!(
            rw.text.contains("// between"),
            "comment between blocks must be preserved:\n{}",
            rw.text
        );
    }
}

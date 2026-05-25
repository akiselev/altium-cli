//! Nondestructive spec dump merge: preserves comments and annotation IDs from
//! an existing spec file when re-dumping from an Altium document.
//!
//! The merge works at the text level using AST span information:
//! 1. Parse both old and new spec texts to get ASTs
//! 2. Extract trivia (comments) from the old file
//! 3. Match blocks between old and new by source_id (UniqueID) or natural key (name)
//! 4. Emit new content in old ordering, reattaching old comments and annotation IDs

use std::collections::HashMap;

use altium_format_spec::ast::{BlockAnnotation, EntityName, SpecItem};
use altium_format_spec::diagnostic::Spanned;
use altium_format_spec::extract_top_level_trivia;
use altium_format_spec::parser::parse_spec;
use altium_format_spec::trivia::{ItemTrivia, TriviaLine};

// ── Public API ────────────────────────────────────────────────────────────────

/// Merge a fresh dump with an existing spec file, preserving comments and
/// annotation IDs from the old file.
///
/// Returns the merged text, or `None` if the old file couldn't be parsed
/// (caller should fall back to overwriting).
pub fn merge_spec(old_text: &str, new_text: &str) -> Option<String> {
    let old_ast = parse_spec(old_text).ok()?;
    let new_ast = match parse_spec(new_text) {
        Ok(ast) => ast,
        Err(_) => return None, // fresh dump should always parse; bail if not
    };

    let old_trivia = extract_top_level_trivia(old_text, &old_ast);

    // Build identity index from old AST.
    let old_blocks: Vec<OldBlock> = old_ast
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let identity = extract_identity(&item.node);
            let trivia = old_trivia.get(i).cloned().unwrap_or_default();
            OldBlock {
                identity,
                trivia,
                index: i,
            }
        })
        .collect();

    // Build lookup maps from old blocks.
    let mut source_id_map: HashMap<(ItemType, &str), usize> = HashMap::new();
    let mut natural_key_map: HashMap<(ItemType, &str), usize> = HashMap::new();

    for (i, block) in old_blocks.iter().enumerate() {
        if let Some(ref sid) = block.identity.source_id {
            source_id_map
                .entry((block.identity.item_type, sid.as_str()))
                .or_insert(i);
        }
        if let Some(ref key) = block.identity.natural_key {
            natural_key_map
                .entry((block.identity.item_type, key.as_str()))
                .or_insert(i);
        }
    }

    // Match new blocks to old blocks.
    let new_identities: Vec<BlockIdentity> = new_ast
        .items
        .iter()
        .map(|item| extract_identity(&item.node))
        .collect();

    // For each new item, find its match in the old file (if any).
    let mut new_to_old: Vec<Option<usize>> = Vec::with_capacity(new_ast.items.len());
    let mut old_consumed: Vec<bool> = vec![false; old_blocks.len()];

    for new_id in &new_identities {
        let matched = find_match(
            new_id,
            &source_id_map,
            &natural_key_map,
            &old_blocks,
            &old_consumed,
        );
        if let Some(old_idx) = matched {
            old_consumed[old_idx] = true;
        }
        new_to_old.push(matched);
    }

    // Build output order: old ordering for matched blocks, then new-only blocks.
    let mut output_items: Vec<OutputItem> = Vec::new();

    // Phase 1: Walk old items in order. For matched ones, emit new content + old trivia.
    for (old_idx, block) in old_blocks.iter().enumerate() {
        if !old_consumed[old_idx] {
            continue; // deleted block — silently drop
        }
        // Find which new item matched this old block.
        let new_idx = new_to_old
            .iter()
            .position(|m| *m == Some(old_idx))
            .expect("consumed old block must have a matching new item");

        output_items.push(OutputItem {
            new_index: new_idx,
            trivia: Some(&block.trivia),
            old_annotation_id: block.identity.annotation_id.as_deref(),
        });
    }

    // Phase 2: Append new-only items (no match in old file).
    for (new_idx, matched) in new_to_old.iter().enumerate() {
        if matched.is_none() {
            output_items.push(OutputItem {
                new_index: new_idx,
                trivia: None,
                old_annotation_id: None,
            });
        }
    }

    // Handle EOF text: anything after the last item in the old file.
    let eof_text = if !old_ast.items.is_empty() {
        let last_end = old_ast.items.last().unwrap().span.end as usize;
        let tail = &old_text[last_end..];
        if tail.trim().is_empty() {
            None
        } else {
            Some(tail)
        }
    } else {
        None
    };

    // Emit merged output.
    //
    // Each surviving old block carries its own trivia (leading comments).
    // If the first old block is deleted, its comments are dropped per decision #2
    // (spec files are in version control). Header comments survive naturally when
    // the first old block survives, since it's emitted first with its trivia.
    let mut out = String::new();

    for (i, output) in output_items.iter().enumerate() {
        // Blank line separator between items (not before the first item).
        if i > 0 && !out.is_empty() && !out.ends_with("\n\n") {
            out.push('\n');
        }

        // Emit trivia (comment lines) from old file.
        if let Some(trivia) = output.trivia {
            emit_trivia(&mut out, trivia);
        }

        // Extract the new block's text from new_text.
        let new_span = new_ast.items[output.new_index].span;
        let mut block_text = new_text[new_span.start as usize..new_span.end as usize].to_string();

        // Replace annotation ID if we have an old one to preserve.
        if let Some(old_id) = output.old_annotation_id {
            block_text = replace_annotation_id(&block_text, old_id);
        }

        out.push_str(&block_text);
        out.push('\n');

        // Emit trailing comment from old file.
        if let Some(trivia) = output.trivia {
            if let Some(ref trailing) = trivia.trailing {
                // Trailing comment goes on the same line as the block's closing.
                // Remove the last newline we just added, append the comment, re-add newline.
                if out.ends_with('\n') {
                    out.pop();
                }
                out.push(' ');
                out.push_str(trailing);
                out.push('\n');
            }
        }
    }

    // EOF text from old file.
    if let Some(eof) = eof_text {
        out.push_str(eof);
    }

    // Ensure file ends with newline.
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }

    Some(out)
}

// ── Identity extraction ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ItemType {
    Import,
    LetBinding,
    Component,
    Footprint,
    Project,
    SwapGroup,
    Sheet,
    Net,
    Power,
    SchDocObject,
    Board,
    Placement,
    PcbDocPrimitive,
    Polygon,
    Rule,
    Class,
    DifferentialPair,
    Routing,
}

#[derive(Debug, Default)]
struct BlockIdentity {
    item_type: ItemType,
    natural_key: Option<String>,
    source_id: Option<String>,
    annotation_id: Option<String>,
}

impl Default for ItemType {
    fn default() -> Self {
        ItemType::LetBinding // arbitrary; overridden in extract_identity
    }
}

struct OldBlock {
    identity: BlockIdentity,
    trivia: ItemTrivia,
    #[allow(dead_code)]
    index: usize,
}

struct OutputItem<'a> {
    new_index: usize,
    trivia: Option<&'a ItemTrivia>,
    old_annotation_id: Option<&'a str>,
}

fn extract_annotation_fields(
    ann: &Option<Spanned<BlockAnnotation>>,
) -> (Option<String>, Option<String>) {
    match ann {
        Some(spanned) => {
            let id = spanned.node.id.as_ref().map(|s| s.node.clone());
            let source_id = spanned.node.source_id.as_ref().map(|s| s.node.clone());
            (id, source_id)
        }
        None => (None, None),
    }
}

fn entity_name_key(name: &EntityName) -> String {
    name.as_str()
}

fn extract_identity(item: &SpecItem) -> BlockIdentity {
    match item {
        SpecItem::Import(decl) => BlockIdentity {
            item_type: ItemType::Import,
            natural_key: Some(decl.path.node.clone()),
            ..Default::default()
        },
        SpecItem::LetBinding(decl) => BlockIdentity {
            item_type: ItemType::LetBinding,
            natural_key: Some(decl.name.node.clone()),
            ..Default::default()
        },
        SpecItem::Component(decl) => {
            let (ann_id, source_id) = extract_annotation_fields(&decl.annotation);
            BlockIdentity {
                item_type: ItemType::Component,
                natural_key: Some(entity_name_key(&decl.name.node)),
                annotation_id: ann_id,
                source_id,
            }
        }
        SpecItem::Footprint(decl) => {
            let (ann_id, source_id) = extract_annotation_fields(&decl.annotation);
            BlockIdentity {
                item_type: ItemType::Footprint,
                natural_key: Some(entity_name_key(&decl.name.node)),
                annotation_id: ann_id,
                source_id,
            }
        }
        SpecItem::Project(decl) => BlockIdentity {
            item_type: ItemType::Project,
            natural_key: Some(entity_name_key(&decl.name.node)),
            ..Default::default()
        },
        SpecItem::SwapGroup(decl) => BlockIdentity {
            item_type: ItemType::SwapGroup,
            natural_key: Some(entity_name_key(&decl.name.node)),
            ..Default::default()
        },
        SpecItem::Sheet(decl) => {
            let (ann_id, source_id) = extract_annotation_fields(&decl.annotation);
            BlockIdentity {
                item_type: ItemType::Sheet,
                natural_key: None, // sheet has no name
                annotation_id: ann_id,
                source_id,
            }
        }
        SpecItem::Net(decl) => {
            let (ann_id, source_id) = extract_annotation_fields(&decl.annotation);
            BlockIdentity {
                item_type: ItemType::Net,
                natural_key: Some(entity_name_key(&decl.name.node)),
                annotation_id: ann_id,
                source_id,
            }
        }
        SpecItem::Power(decl) => {
            let (ann_id, source_id) = extract_annotation_fields(&decl.annotation);
            BlockIdentity {
                item_type: ItemType::Power,
                natural_key: Some(entity_name_key(&decl.name.node)),
                annotation_id: ann_id,
                source_id,
            }
        }
        SpecItem::SchDocObject(decl) => BlockIdentity {
            item_type: ItemType::SchDocObject,
            natural_key: decl.name.as_ref().map(|n| entity_name_key(&n.node)),
            ..Default::default()
        },
        SpecItem::Board(decl) => {
            let (ann_id, source_id) = extract_annotation_fields(&decl.annotation);
            BlockIdentity {
                item_type: ItemType::Board,
                natural_key: Some(entity_name_key(&decl.name.node)),
                annotation_id: ann_id,
                source_id,
            }
        }
        SpecItem::Placement(decl) => {
            let (ann_id, source_id) = extract_annotation_fields(&decl.annotation);
            BlockIdentity {
                item_type: ItemType::Placement,
                natural_key: None, // placement has no name
                annotation_id: ann_id,
                source_id,
            }
        }
        SpecItem::PcbDocPrimitive(decl) => BlockIdentity {
            item_type: ItemType::PcbDocPrimitive,
            natural_key: decl.name.as_ref().map(|n| entity_name_key(&n.node)),
            ..Default::default()
        },
        SpecItem::Polygon(decl) => {
            let (ann_id, source_id) = extract_annotation_fields(&decl.annotation);
            BlockIdentity {
                item_type: ItemType::Polygon,
                natural_key: Some(entity_name_key(&decl.name.node)),
                annotation_id: ann_id,
                source_id,
            }
        }
        SpecItem::Rule(decl) => {
            let (ann_id, source_id) = extract_annotation_fields(&decl.annotation);
            BlockIdentity {
                item_type: ItemType::Rule,
                natural_key: Some(entity_name_key(&decl.name.node)),
                annotation_id: ann_id,
                source_id,
            }
        }
        SpecItem::Class(decl) => {
            let (ann_id, source_id) = extract_annotation_fields(&decl.annotation);
            BlockIdentity {
                item_type: ItemType::Class,
                natural_key: Some(entity_name_key(&decl.name.node)),
                annotation_id: ann_id,
                source_id,
            }
        }
        SpecItem::DifferentialPair(decl) => BlockIdentity {
            item_type: ItemType::DifferentialPair,
            natural_key: Some(entity_name_key(&decl.name.node)),
            ..Default::default()
        },
        SpecItem::Routing(_) => BlockIdentity {
            item_type: ItemType::Routing,
            natural_key: None,
            ..Default::default()
        },
    }
}

// ── Matching ──────────────────────────────────────────────────────────────────

fn find_match(
    new_id: &BlockIdentity,
    source_id_map: &HashMap<(ItemType, &str), usize>,
    natural_key_map: &HashMap<(ItemType, &str), usize>,
    old_blocks: &[OldBlock],
    old_consumed: &[bool],
) -> Option<usize> {
    // Primary: match by source_id (Altium UniqueID).
    if let Some(ref sid) = new_id.source_id {
        if let Some(&old_idx) = source_id_map.get(&(new_id.item_type, sid.as_str())) {
            if !old_consumed[old_idx] {
                return Some(old_idx);
            }
        }
    }

    // Fallback: match by natural key (entity name).
    if let Some(ref key) = new_id.natural_key {
        if let Some(&old_idx) = natural_key_map.get(&(new_id.item_type, key.as_str())) {
            if !old_consumed[old_idx] {
                return Some(old_idx);
            }
        }
    }

    // Singleton types (Sheet, Placement) — match by type alone.
    // These have no natural key, so we find the first unconsumed old item of the same type.
    if new_id.natural_key.is_none()
        && matches!(new_id.item_type, ItemType::Sheet | ItemType::Placement)
    {
        for (i, block) in old_blocks.iter().enumerate() {
            if block.identity.item_type == new_id.item_type && !old_consumed[i] {
                return Some(i);
            }
        }
    }

    None
}

// ── Annotation ID replacement ─────────────────────────────────────────────────

/// Replace the annotation `id = "XXXXXXXX"` in a block's text with a different ID.
///
/// The dump format always emits `#[annotation(id = "XXXXXXXX"` as the first line
/// of an annotated block. We find and replace the 8-char ID.
fn replace_annotation_id(block_text: &str, old_id: &str) -> String {
    // Find `id = "` pattern and replace the 8 chars after it.
    const PREFIX: &str = "id = \"";
    if let Some(pos) = block_text.find(PREFIX) {
        let id_start = pos + PREFIX.len();
        let id_end = id_start + 8;
        if id_end <= block_text.len() {
            let mut result = String::with_capacity(block_text.len());
            result.push_str(&block_text[..id_start]);
            result.push_str(old_id);
            result.push_str(&block_text[id_end..]);
            return result;
        }
    }
    block_text.to_string()
}

// ── Trivia emission ───────────────────────────────────────────────────────────

fn emit_trivia(out: &mut String, trivia: &ItemTrivia) {
    // Strip leading and trailing Blank lines — they're inter-block formatting
    // that we control ourselves. Only preserve comment lines (and inner blanks
    // between comments, which are intentional spacing like section headers).
    let lines = &trivia.leading;

    // Find first non-blank and last non-blank.
    let first_comment = lines.iter().position(|l| !matches!(l, TriviaLine::Blank));
    let last_comment = lines.iter().rposition(|l| !matches!(l, TriviaLine::Blank));

    if let (Some(first), Some(last)) = (first_comment, last_comment) {
        for line in &lines[first..=last] {
            match line {
                TriviaLine::Blank => out.push('\n'),
                TriviaLine::LineComment(text) => {
                    out.push_str(text);
                    out.push('\n');
                }
                TriviaLine::BlockComment(text) => {
                    out.push_str(text);
                    out.push('\n');
                }
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_old_file_returns_none_on_parse_error() {
        // Garbage old text should return None (fall back to overwrite).
        let result = merge_spec(
            "this is not valid spec syntax {{{",
            "component \"X\" {\n}\n",
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_empty_old_file_returns_new_content() {
        let new_text = "#[annotation(id = \"AB12CD34\")]\ncomponent \"X\" {\n}\n";
        let result = merge_spec("", new_text).unwrap();
        // All new items appended. The new content should appear.
        assert!(result.contains("component \"X\""));
    }

    #[test]
    fn test_comment_preservation() {
        let old_text = "\
// This is a capacitor bank
#[annotation(id = \"OLDID001\")]
component \"CAP100\" {
    description: \"100nF capacitor\"
}
";
        let new_text = "\
#[annotation(id = \"NEWID999\")]
component \"CAP100\" {
    description: \"100nF capacitor updated\"
}
";
        let result = merge_spec(old_text, new_text).unwrap();
        // Comment should be preserved.
        assert!(
            result.contains("// This is a capacitor bank"),
            "comment lost: {result}"
        );
        // Old annotation ID should be preserved.
        assert!(
            result.contains("OLDID001"),
            "old annotation ID lost: {result}"
        );
        // New content should be used.
        assert!(
            result.contains("100nF capacitor updated"),
            "new content missing: {result}"
        );
    }

    #[test]
    fn test_annotation_id_preservation() {
        let old_text = "\
#[annotation(id = \"AAAAAAAA\")]
component \"R1\" {
    description: \"resistor\"
}
";
        let new_text = "\
#[annotation(id = \"ZZZZZZZZ\")]
component \"R1\" {
    description: \"resistor v2\"
}
";
        let result = merge_spec(old_text, new_text).unwrap();
        assert!(
            result.contains("AAAAAAAA"),
            "old ID not preserved: {result}"
        );
        assert!(
            !result.contains("ZZZZZZZZ"),
            "new random ID leaked: {result}"
        );
    }

    #[test]
    fn test_new_block_appended() {
        let old_text = "\
#[annotation(id = \"AAAAAAAA\")]
component \"R1\" {
    description: \"resistor\"
}
";
        let new_text = "\
#[annotation(id = \"BBBBBBBB\")]
component \"R1\" {
    description: \"resistor\"
}

#[annotation(id = \"CCCCCCCC\")]
component \"C1\" {
    description: \"capacitor\"
}
";
        let result = merge_spec(old_text, new_text).unwrap();
        // R1 should keep old ID.
        assert!(result.contains("AAAAAAAA"), "old ID lost: {result}");
        // C1 should appear with its fresh ID.
        assert!(
            result.contains("component \"C1\""),
            "new block missing: {result}"
        );
        assert!(
            result.contains("CCCCCCCC"),
            "new block ID missing: {result}"
        );
    }

    #[test]
    fn test_deleted_block_dropped() {
        let old_text = "\
// comment on R1
#[annotation(id = \"AAAAAAAA\")]
component \"R1\" {
    description: \"resistor\"
}

// comment on C1
#[annotation(id = \"BBBBBBBB\")]
component \"C1\" {
    description: \"capacitor\"
}
";
        let new_text = "\
#[annotation(id = \"CCCCCCCC\")]
component \"C1\" {
    description: \"capacitor v2\"
}
";
        let result = merge_spec(old_text, new_text).unwrap();
        // R1 should be gone.
        assert!(
            !result.contains("component \"R1\""),
            "deleted block survived: {result}"
        );
        // C1 should exist with old ID.
        assert!(result.contains("BBBBBBBB"), "old ID for C1 lost: {result}");
        // Comment on C1 preserved.
        assert!(
            result.contains("// comment on C1"),
            "comment on C1 lost: {result}"
        );
    }

    #[test]
    fn test_ordering_preserved() {
        let old_text = "\
#[annotation(id = \"AAAAAAAA\")]
component \"A\" {
}

#[annotation(id = \"BBBBBBBB\")]
component \"B\" {
}

#[annotation(id = \"CCCCCCCC\")]
component \"C\" {
}
";
        // New dump reverses the order.
        let new_text = "\
#[annotation(id = \"XXXXXXXX\")]
component \"C\" {
}

#[annotation(id = \"YYYYYYYY\")]
component \"A\" {
}

#[annotation(id = \"ZZZZZZZZ\")]
component \"B\" {
}
";
        let result = merge_spec(old_text, new_text).unwrap();
        // Old ordering should be preserved: A, B, C.
        let pos_a = result.find("component \"A\"").unwrap();
        let pos_b = result.find("component \"B\"").unwrap();
        let pos_c = result.find("component \"C\"").unwrap();
        assert!(pos_a < pos_b, "A should come before B: {result}");
        assert!(pos_b < pos_c, "B should come before C: {result}");
    }

    #[test]
    fn test_replace_annotation_id() {
        let text = "#[annotation(id = \"NEWID123\")]\ncomponent \"X\" {\n}";
        let result = replace_annotation_id(text, "OLDID456");
        assert!(result.contains("OLDID456"));
        assert!(!result.contains("NEWID123"));
    }

    #[test]
    fn test_replace_annotation_id_with_source_id() {
        let text = "#[annotation(id = \"NEWID123\", source_id = \"abc\")]\ncomponent \"X\" {\n}";
        let result = replace_annotation_id(text, "OLDID456");
        assert!(result.contains("OLDID456"));
        assert!(result.contains("source_id = \"abc\""));
    }

    #[test]
    fn test_trailing_comment_preservation() {
        let old_text = "\
#[annotation(id = \"AAAAAAAA\")]
component \"X\" {
    description: \"test\"
} // power section
";
        let new_text = "\
#[annotation(id = \"NEWID123\")]
component \"X\" {
    description: \"test v2\"
}
";
        let result = merge_spec(old_text, new_text).unwrap();
        assert!(
            result.contains("// power section"),
            "trailing comment lost: {result}"
        );
    }

    #[test]
    fn test_header_comment_preservation() {
        let old_text = "\
// Auto-generated spec file
// Do not edit the annotations

#[annotation(id = \"AAAAAAAA\")]
component \"X\" {
}
";
        let new_text = "\
#[annotation(id = \"NEWID123\")]
component \"X\" {
    description: \"updated\"
}
";
        let result = merge_spec(old_text, new_text).unwrap();
        assert!(
            result.contains("// Auto-generated spec file"),
            "header comment lost: {result}"
        );
        assert!(
            result.contains("// Do not edit the annotations"),
            "second header comment lost: {result}"
        );
    }

    #[test]
    fn test_source_id_matching() {
        // Old has source_id, new also has matching source_id but different name.
        let old_text = "\
// important comment
#[annotation(id = \"OLDID001\", source_id = \"UNIQ123\")]
component \"OldName\" {
}
";
        let new_text = "\
#[annotation(id = \"NEWRANDO\", source_id = \"UNIQ123\")]
component \"NewName\" {
}
";
        let result = merge_spec(old_text, new_text).unwrap();
        // Should match by source_id despite name change.
        assert!(
            result.contains("OLDID001"),
            "old annotation ID lost on source_id match: {result}"
        );
        assert!(
            result.contains("// important comment"),
            "comment lost: {result}"
        );
        // Content should be from new dump (has NewName).
        assert!(
            result.contains("component \"NewName\""),
            "new content missing: {result}"
        );
    }

    #[test]
    fn test_pcbdoc_primitives_not_matched() {
        // Primitives have no stable identity — they should always use fresh content.
        let old_text = "\
board \"test\" {
}

track { layer: TopLayer, from: (0mm, 0mm), to: (1mm, 1mm), width: 0.254mm }
";
        let new_text = "\
board \"test\" {
    signal_layer_count: 2
}

track { layer: TopLayer, from: (0mm, 0mm), to: (2mm, 2mm), width: 0.5mm }
";
        let result = merge_spec(old_text, new_text).unwrap();
        // New track content should be used.
        assert!(
            result.contains("to: (2mm, 2mm)"),
            "new primitive content missing: {result}"
        );
    }
}

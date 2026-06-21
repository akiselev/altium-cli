//! Comment-preserving reconciliation of a fresh document dump into an existing
//! authored spec, expressed entirely as typed structured edits.

use std::collections::{BTreeMap, HashMap, HashSet};

use thiserror::Error;

use super::access::{BlockKind, BlockRef, SourceId, SpecTree, body_node};
use super::edit::{
    ExprSource, IntentBlock, PropertyKey, SpecEdit, StructuredEditError, apply_edits,
};
use super::lexer::lex_lossless;
use super::syntax::SyntaxKind as K;
use crate::diagnostic::ParseError;

#[derive(Debug, Error)]
pub enum DumpMergeError {
    #[error("existing spec is malformed: {0}")]
    ExistingParse(ParseError),
    #[error("fresh dump is malformed: {0}")]
    DumpParse(ParseError),
    #[error("structured dump edit failed: {0}")]
    Edit(#[from] StructuredEditError),
    #[error("ambiguous {kind:?} identity '{key}' under {parent}")]
    AmbiguousIdentity {
        parent: SourceId,
        kind: BlockKind,
        key: String,
    },
    #[error("duplicate property '{key}' in {kind:?} block")]
    DuplicateProperty { kind: BlockKind, key: String },
    #[error("cannot preserve authored source while changing a non-name {kind:?} block header")]
    UnsupportedHeaderChange { kind: BlockKind },
}

/// Merge a canonical fresh dump into existing source using typed CST edits.
///
/// Matched blocks retain ordering, comments, whitespace, unchanged property
/// bytes, and their prior annotation ID. Malformed input is always an error.
pub fn merge_dump(existing: &str, fresh_dump: &str) -> Result<String, DumpMergeError> {
    let existing = SpecTree::parse(existing.to_owned()).map_err(DumpMergeError::ExistingParse)?;
    let fresh = SpecTree::parse(fresh_dump.to_owned()).map_err(DumpMergeError::DumpParse)?;
    let mut edits = Vec::new();
    reconcile_children(
        existing.root_id(),
        existing.top_level_blocks(),
        fresh.top_level_blocks(),
        &mut edits,
    )?;
    Ok(apply_edits(&existing, &edits)?.source().to_owned())
}

fn reconcile_children(
    existing_parent: SourceId,
    existing: Vec<BlockRef<'_>>,
    fresh: Vec<BlockRef<'_>>,
    edits: &mut Vec<SpecEdit>,
) -> Result<(), DumpMergeError> {
    let matches = match_blocks(&existing_parent, &existing, &fresh)?;
    let matched_existing: HashSet<usize> = matches.iter().flatten().copied().collect();

    for (index, block) in existing.iter().enumerate() {
        if !matched_existing.contains(&index) {
            edits.push(SpecEdit::DeleteBlock {
                id: block.id().clone(),
            });
        }
    }

    for (fresh_index, old_index) in matches.into_iter().enumerate() {
        let fresh_block = &fresh[fresh_index];
        let Some(old_index) = old_index else {
            edits.push(SpecEdit::InsertBlock {
                parent: existing_parent.clone(),
                block: IntentBlock::from_block(fresh_block),
            });
            continue;
        };
        reconcile_block(&existing_parent, &existing[old_index], fresh_block, edits)?;
    }
    Ok(())
}

fn reconcile_block(
    existing_parent: &SourceId,
    existing: &BlockRef<'_>,
    fresh: &BlockRef<'_>,
    edits: &mut Vec<SpecEdit>,
) -> Result<(), DumpMergeError> {
    let header_changed = header_signature(existing) != header_signature(fresh);
    let name_only = header_changed
        && existing.name_source().is_some()
        && fresh.name_source().is_some()
        && header_signature_without_name(existing) == header_signature_without_name(fresh);
    if header_changed && !name_only {
        return Err(DumpMergeError::UnsupportedHeaderChange {
            kind: existing.kind(),
        });
    }
    let old_pattern = generated_pattern_comment(existing);
    let fresh_pattern = generated_pattern_comment(fresh);
    let replace_pattern_comment = match existing.trailing_line_comment() {
        Some(_) if old_pattern.is_none() => false,
        _ => old_pattern != fresh_pattern,
    };
    if name_only || replace_pattern_comment {
        let mut replacement_binding = fresh.annotation().map(|annotation| annotation.metadata());
        if let Some(old_id) = existing
            .annotation()
            .and_then(|annotation| annotation.metadata().id)
            && let Some(binding) = &mut replacement_binding
        {
            binding.id = Some(old_id);
        }
        edits.push(SpecEdit::DeleteBlock {
            id: existing.id().clone(),
        });
        edits.push(SpecEdit::InsertBlock {
            parent: existing_parent.clone(),
            block: IntentBlock::replacement_from_block(
                fresh,
                replacement_binding.as_ref(),
                name_only,
                replace_pattern_comment,
            ),
        });
    }

    reconcile_annotation(existing, fresh, edits);
    reconcile_properties(existing, fresh, edits)?;
    reconcile_children(
        existing.id().clone(),
        existing.child_blocks(),
        fresh.child_blocks(),
        edits,
    )
}

fn generated_pattern_comment(block: &BlockRef<'_>) -> Option<String> {
    block
        .trailing_line_comment()
        .filter(|comment| comment.trim_start().starts_with("// pattern:"))
        .map(str::to_owned)
}

fn reconcile_annotation(existing: &BlockRef<'_>, fresh: &BlockRef<'_>, edits: &mut Vec<SpecEdit>) {
    let Some(fresh_annotation) = fresh.annotation() else {
        return;
    };
    let mut desired = fresh_annotation.metadata();
    if let Some(old_id) = existing
        .annotation()
        .and_then(|annotation| annotation.metadata().id)
    {
        desired.id = Some(old_id);
    }
    if existing
        .annotation()
        .map(|annotation| annotation.metadata())
        != Some(desired.clone())
    {
        edits.push(SpecEdit::SetAnnotation {
            id: existing.id().clone(),
            binding: desired,
        });
    }
}

fn reconcile_properties(
    existing: &BlockRef<'_>,
    fresh: &BlockRef<'_>,
    edits: &mut Vec<SpecEdit>,
) -> Result<(), DumpMergeError> {
    let old = property_map(existing)?;
    let new = property_map(fresh)?;
    for (key, value) in &new {
        if old.get(key).is_none_or(|old_value| old_value != value) {
            edits.push(SpecEdit::SetProperty {
                id: existing.id().clone(),
                key: PropertyKey::new(key.clone())?,
                value: ExprSource::parse(value.clone())?,
            });
        }
    }
    for key in old.keys() {
        if !new.contains_key(key) {
            edits.push(SpecEdit::RemoveProperty {
                id: existing.id().clone(),
                key: PropertyKey::new(key.clone())?,
            });
        }
    }
    Ok(())
}

fn property_map(block: &BlockRef<'_>) -> Result<BTreeMap<String, String>, DumpMergeError> {
    let mut result = BTreeMap::new();
    for property in block.properties() {
        let key = property.key().to_owned();
        if result
            .insert(key.clone(), property.value_text().to_owned())
            .is_some()
        {
            return Err(DumpMergeError::DuplicateProperty {
                kind: block.kind(),
                key,
            });
        }
    }
    Ok(result)
}

#[derive(Debug)]
struct Identity {
    kind: BlockKind,
    source_id: Option<String>,
    natural_key: Option<String>,
}

fn identity(block: &BlockRef<'_>) -> Identity {
    Identity {
        kind: block.kind(),
        source_id: block
            .annotation()
            .and_then(|annotation| annotation.metadata().source_id),
        natural_key: block.natural_key(),
    }
}

fn match_blocks(
    parent: &SourceId,
    existing: &[BlockRef<'_>],
    fresh: &[BlockRef<'_>],
) -> Result<Vec<Option<usize>>, DumpMergeError> {
    let mut by_source: HashMap<(BlockKind, String), Vec<usize>> = HashMap::new();
    let mut by_natural: HashMap<(BlockKind, String), Vec<usize>> = HashMap::new();
    for (index, block) in existing.iter().enumerate() {
        let id = identity(block);
        if let Some(source_id) = id.source_id {
            by_source
                .entry((id.kind, source_id))
                .or_default()
                .push(index);
        }
        if let Some(natural_key) = id.natural_key {
            by_natural
                .entry((id.kind, natural_key))
                .or_default()
                .push(index);
        }
    }

    let mut consumed = HashSet::new();
    let mut result = vec![None; fresh.len()];
    for (fresh_index, block) in fresh.iter().enumerate() {
        let id = identity(block);
        let mut candidate = None;
        if let Some(source_id) = &id.source_id {
            candidate = unique_candidate(
                parent,
                id.kind,
                source_id,
                by_source.get(&(id.kind, source_id.clone())),
            )?;
        } else if candidate.is_none()
            && let Some(natural_key) = &id.natural_key
        {
            candidate = unique_candidate(
                parent,
                id.kind,
                natural_key,
                by_natural.get(&(id.kind, natural_key.clone())),
            )?;
        }
        if let Some(index) = candidate
            && consumed.insert(index)
        {
            result[fresh_index] = Some(index);
        }
    }

    // Identityless records cannot safely use names. First preserve exact
    // semantic matches, then use same-header ordinal matching only when the
    // unmatched collection cardinalities agree. A count change remains a
    // delete/add, preventing middle insertion/deletion from shifting identity.
    for (fresh_index, block) in fresh.iter().enumerate() {
        if result[fresh_index].is_some() || !is_identityless(block) {
            continue;
        }
        let signature = semantic_signature(block);
        let candidates: Vec<_> = existing
            .iter()
            .enumerate()
            .filter(|(index, candidate)| {
                !consumed.contains(index)
                    && is_identityless(candidate)
                    && semantic_signature(candidate) == signature
            })
            .map(|(index, _)| index)
            .collect();
        if let [index] = candidates.as_slice() {
            consumed.insert(*index);
            result[fresh_index] = Some(*index);
        }
    }

    let mut groups: HashMap<Vec<(K, String)>, (Vec<usize>, Vec<usize>)> = HashMap::new();
    for (index, block) in existing.iter().enumerate() {
        if !consumed.contains(&index) && is_identityless(block) {
            groups
                .entry(header_signature(block))
                .or_default()
                .0
                .push(index);
        }
    }
    for (index, block) in fresh.iter().enumerate() {
        if result[index].is_none() && is_identityless(block) {
            groups
                .entry(header_signature(block))
                .or_default()
                .1
                .push(index);
        }
    }
    for (_, (old, new)) in groups {
        if old.len() == new.len() {
            for (old_index, new_index) in old.into_iter().zip(new) {
                consumed.insert(old_index);
                result[new_index] = Some(old_index);
            }
        }
    }
    Ok(result)
}

fn unique_candidate(
    parent: &SourceId,
    kind: BlockKind,
    key: &str,
    candidates: Option<&Vec<usize>>,
) -> Result<Option<usize>, DumpMergeError> {
    match candidates.map(Vec::as_slice).unwrap_or_default() {
        [] => Ok(None),
        [index] => Ok(Some(*index)),
        _ => Err(DumpMergeError::AmbiguousIdentity {
            parent: parent.clone(),
            kind,
            key: key.to_owned(),
        }),
    }
}

fn is_identityless(block: &BlockRef<'_>) -> bool {
    let identity = identity(block);
    identity.source_id.is_none() && identity.natural_key.is_none()
}

fn semantic_signature(block: &BlockRef<'_>) -> Vec<(K, String)> {
    tokens_without_trivia(block, false, false)
}

fn header_signature(block: &BlockRef<'_>) -> Vec<(K, String)> {
    tokens_without_trivia(block, true, false)
}

fn header_signature_without_name(block: &BlockRef<'_>) -> Vec<(K, String)> {
    tokens_without_trivia(block, true, true)
}

fn tokens_without_trivia(
    block: &BlockRef<'_>,
    omit_body: bool,
    omit_name: bool,
) -> Vec<(K, String)> {
    let node_range = byte_range(block.node().text_range());
    let body_range = body_node(block.node()).map(|body| byte_range(body.text_range()));
    let annotation_range = block
        .annotation()
        .map(|annotation| byte_range(annotation.node().text_range()));
    let name_range = block.name_node().map(|name| byte_range(name.text_range()));
    lex_lossless(block.source())
        .expect("SpecTree source has already been lexed successfully")
        .into_iter()
        .filter(|token| node_range.start <= token.range.start && token.range.end <= node_range.end)
        .filter(|token| {
            (!omit_body
                || !body_range
                    .as_ref()
                    .is_some_and(|range| contains(range, &token.range)))
                && !annotation_range
                    .as_ref()
                    .is_some_and(|range| contains(range, &token.range))
                && (!omit_name
                    || !name_range
                        .as_ref()
                        .is_some_and(|range| contains(range, &token.range)))
        })
        .filter(|token| {
            !matches!(
                token.kind,
                K::Whitespace | K::Newline | K::LineComment | K::BlockComment
            )
        })
        .map(|token| (token.kind, block.source()[token.range].to_owned()))
        .collect()
}

fn contains(outer: &std::ops::Range<usize>, inner: &std::ops::Range<usize>) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn byte_range(range: cstree::text::TextRange) -> std::ops::Range<usize> {
    u32::from(range.start()) as usize..u32::from(range.end()) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_updates_only_changed_property_and_keeps_comments_and_id() {
        let old = "// component docs\n#[annotation(id = \"OLDID001\", source_id = \"U1\")]\ncomponent \"R1\" {\n    description : \"old\" // keep\n    value: \"10k\"\n}\n";
        let fresh = "#[annotation(id = \"RANDOM99\", source_id = \"U1\")]\ncomponent \"R1\" {\n    description: \"new\"\n    value: \"10k\"\n}\n";
        let merged = merge_dump(old, fresh).unwrap();
        assert_eq!(
            merged,
            "// component docs\n#[annotation(id = \"OLDID001\", source_id = \"U1\")]\ncomponent \"R1\" {\n    description : \"new\" // keep\n    value: \"10k\"\n}\n"
        );
    }

    #[test]
    fn merge_recurses_into_named_children() {
        let old =
            "component X {\n    // pin docs\n    pin 1 { electrical: passive, name: \"A\" }\n}\n";
        let fresh = "component X {\n    pin 1 { electrical: input, name: \"A\" }\n}\n";
        let merged = merge_dump(old, fresh).unwrap();
        assert!(merged.contains("// pin docs\n    pin 1 { electrical: input, name: \"A\" }"));
    }

    #[test]
    fn malformed_existing_source_is_a_hard_error() {
        assert!(matches!(
            merge_dump("component {", "component X {}"),
            Err(DumpMergeError::ExistingParse(_))
        ));
    }

    #[test]
    fn unchanged_source_is_byte_identical_despite_fresh_annotation_id() {
        let old = "// docs\n#[annotation(id = \"AAAAAAAA\")]\ncomponent X { value : \"10k\" }\n";
        let fresh = "#[annotation(id = \"BBBBBBBB\")]\ncomponent X { value: \"10k\" }\n";
        assert_eq!(merge_dump(old, fresh).unwrap(), old);
    }

    #[test]
    fn identityless_primitives_preserve_comments_and_update_by_ordinal() {
        let old = "// route note\ntrack { width : 6mil, layer: top }\ntrack { width: 8mil, layer: bottom }\n";
        let fresh = "track { width: 10mil, layer: top }\ntrack { width: 8mil, layer: bottom }\n";
        let merged = merge_dump(old, fresh).unwrap();
        assert_eq!(
            merged,
            "// route note\ntrack { width : 10mil, layer: top }\ntrack { width: 8mil, layer: bottom }\n"
        );
    }

    #[test]
    fn mismatched_source_ids_do_not_fall_back_to_natural_key() {
        let old = "#[annotation(id = \"OLDID001\", source_id = \"U1\")]\ncomponent R1 {}\n";
        let fresh = "#[annotation(id = \"NEWID002\", source_id = \"U2\")]\ncomponent R1 {}\n";
        let merged = merge_dump(old, fresh).unwrap();
        assert!(!merged.contains("OLDID001"));
        assert!(merged.contains("NEWID002"));
    }

    #[test]
    fn source_id_rename_keeps_position_comments_and_old_annotation_id() {
        let old = "// renamed component\n#[annotation(id = \"OLDID001\", source_id = \"U1\")]\ncomponent Old {\n    // body docs\n    value : \"old\"\n} // authored trailing note\n\n#[annotation(id = \"KEEP0001\")]\ncomponent Keep {}\n";
        let fresh = "#[annotation(id = \"RANDOM99\", source_id = \"U1\")]\ncomponent New {\n    value: \"new\"\n} // generated fresh note\n\n#[annotation(id = \"RANDOM88\")]\ncomponent Keep {}\n";
        let merged = merge_dump(old, fresh).unwrap();
        assert!(merged.contains("// renamed component\n#[annotation(id = \"OLDID001\", source_id = \"U1\")]\ncomponent New {\n    // body docs\n    value : \"new\"\n} // authored trailing note"));
        assert!(!merged.contains("generated fresh note"));
        assert!(merged.find("component New").unwrap() < merged.find("component Keep").unwrap());
    }

    #[test]
    fn inserted_block_keeps_fresh_same_line_comment() {
        let merged = merge_dump(
            "board B {}\n",
            "board B {}\ncomponent U1 { at: (0mil, 0mil) } // pattern: \"QFN\"\n",
        )
        .unwrap();
        assert!(merged.contains("component U1 { at: (0mil, 0mil) } // pattern: \"QFN\""));
    }

    #[test]
    fn non_name_header_change_fails_closed() {
        let old = "x = component R {\n    // authored body\n    value: \"10k\"\n}\n";
        let fresh = "component R {\n    value: \"22k\"\n}\n";
        assert!(matches!(
            merge_dump(old, fresh),
            Err(DumpMergeError::UnsupportedHeaderChange {
                kind: BlockKind::Component
            })
        ));
    }

    #[test]
    fn generated_pattern_comment_is_updated_without_rewriting_block() {
        let old = "// component docs\ncomponent U1 { at : (0mil, 0mil) } // pattern: \"OLD\"\n";
        let fresh = "component U1 { at: (0mil, 0mil) } // pattern: \"NEW\"\n";
        assert_eq!(
            merge_dump(old, fresh).unwrap(),
            "// component docs\ncomponent U1 { at : (0mil, 0mil) } // pattern: \"NEW\"\n"
        );
    }
}

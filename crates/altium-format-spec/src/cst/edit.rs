//! Structured edits for lossless spec sources.

use std::collections::HashSet;

use thiserror::Error;

use super::access::{BindingMetadata, SourceId, SpecTree, body_node};
use super::lexer::lex_lossless;
use super::syntax::SyntaxKind as K;
use crate::diagnostic::ParseError;

/// A validated property name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PropertyKey(String);

impl PropertyKey {
    pub fn new(key: impl Into<String>) -> Result<Self, StructuredEditError> {
        let key = key.into();
        let probe = format!("component Probe {{ {key}: null }}");
        let tree = SpecTree::parse(probe).map_err(StructuredEditError::InvalidFragment)?;
        let valid = tree.top_level_blocks()[0].property(&key).is_some();
        if !valid {
            return Err(StructuredEditError::InvalidPropertyKey(key));
        }
        Ok(Self(key))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validated expression source fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprSource(String);

impl ExprSource {
    pub fn parse(source: impl Into<String>) -> Result<Self, StructuredEditError> {
        let source = source.into();
        let probe = SpecTree::parse(format!("component Probe {{ value: {source} }}"))
            .map_err(StructuredEditError::InvalidFragment)?;
        let blocks = probe.top_level_blocks();
        let valid = blocks.len() == 1
            && blocks[0]
                .property("value")
                .is_some_and(|property| property.value_text() == source.trim());
        if !valid {
            return Err(StructuredEditError::ExpectedSingleExpression);
        }
        Ok(Self(source))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validated declaration/item source fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentBlock {
    source: String,
    replacement: bool,
    replacement_name: Option<String>,
    replacement_trailing_comment: Option<Option<String>>,
}

impl IntentBlock {
    /// Validate one top-level block. The same grammar is used for nested items;
    /// callers constructing nested edits normally obtain fragments from a dump
    /// tree through [`IntentBlock::from_block`].
    pub fn parse(source: impl Into<String>) -> Result<Self, StructuredEditError> {
        let source = source.into();
        let tree = SpecTree::parse(source.clone()).map_err(StructuredEditError::InvalidFragment)?;
        if tree.top_level_blocks().len() != 1 {
            return Err(StructuredEditError::ExpectedSingleBlock);
        }
        Ok(Self {
            source,
            replacement: false,
            replacement_name: None,
            replacement_trailing_comment: None,
        })
    }

    /// Construct a validated fragment from an existing typed CST block.
    /// This is the constructor used for nested declarations, whose grammar is
    /// defined by their parent rather than accepted at file scope.
    pub fn from_block(block: &super::access::BlockRef<'_>) -> Self {
        let start = u32::from(block.node().text_range().start()) as usize;
        let line_start = block.source()[..start]
            .rfind('\n')
            .map_or(0, |position| position + 1);
        let outer_indent = &block.source()[line_start..start];
        let text = normalize_newlines(block.text(), "\n");
        let mut lines = text.lines();
        let mut relative = lines.next().unwrap_or_default().to_owned();
        for line in lines {
            relative.push('\n');
            relative.push_str(line.strip_prefix(outer_indent).unwrap_or(line));
        }
        if text.ends_with('\n') {
            relative.push('\n');
        }
        if let Some(trailing) = block.trailing_line_comment() {
            relative.push_str(trailing);
        }
        Self {
            source: relative,
            replacement: false,
            replacement_name: None,
            replacement_trailing_comment: None,
        }
    }

    pub(crate) fn replacement_from_block(
        block: &super::access::BlockRef<'_>,
        binding: Option<&BindingMetadata>,
        name_only: bool,
        replace_trailing_comment: bool,
    ) -> Self {
        let mut result = Self::from_block(block);
        if let Some(binding) = binding {
            let rendered = render_annotation(binding);
            if let Some(annotation) = block.annotation() {
                let block_start = u32::from(block.node().text_range().start()) as usize;
                let annotation_range = byte_range(annotation.node().text_range());
                result.source.replace_range(
                    annotation_range.start - block_start..annotation_range.end - block_start,
                    &rendered,
                );
            } else {
                result.source = format!("{rendered}\n{}", result.source);
            }
        }
        result.replacement = true;
        if name_only {
            result.replacement_name = block.name_source().map(str::to_owned);
        }
        if replace_trailing_comment {
            result.replacement_trailing_comment =
                Some(block.trailing_line_comment().map(str::to_owned));
        }
        result
    }

    pub fn as_str(&self) -> &str {
        &self.source
    }
}

/// One typed edit against a [`SpecTree`] snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecEdit {
    InsertBlock {
        parent: SourceId,
        block: IntentBlock,
    },
    DeleteBlock {
        id: SourceId,
    },
    SetProperty {
        id: SourceId,
        key: PropertyKey,
        value: ExprSource,
    },
    RemoveProperty {
        id: SourceId,
        key: PropertyKey,
    },
    SetAnnotation {
        id: SourceId,
        binding: BindingMetadata,
    },
}

#[derive(Debug, Error)]
pub enum StructuredEditError {
    #[error("invalid spec source: {0}")]
    Parse(#[from] ParseError),
    #[error("invalid structured-edit fragment: {0}")]
    InvalidFragment(ParseError),
    #[error("property key '{0}' is not valid in the spec grammar")]
    InvalidPropertyKey(String),
    #[error("intent block must contain exactly one declaration")]
    ExpectedSingleBlock,
    #[error("expression fragment must contain exactly one expression")]
    ExpectedSingleExpression,
    #[error("source id {0} does not exist in this source snapshot")]
    UnknownSourceId(SourceId),
    #[error("source id {0} does not identify an editable block")]
    NotABlock(SourceId),
    #[error("source id {0} does not identify a container block")]
    NotAContainer(SourceId),
    #[error("property '{key}' occurs more than once in block {id}")]
    DuplicateProperty { id: SourceId, key: String },
    #[error("structured edits overlap at byte ranges {first:?} and {second:?}")]
    OverlappingEdits {
        first: std::ops::Range<usize>,
        second: std::ops::Range<usize>,
    },
    #[error("multiple structured edits target block {0} incompatibly")]
    ConflictingEdits(SourceId),
}

#[derive(Debug)]
struct Replacement {
    range: std::ops::Range<usize>,
    text: String,
    order: usize,
}

/// Apply typed edits in memory, preserving every byte outside the edited ranges,
/// then reparse the result before returning it.
pub fn apply_edits(tree: &SpecTree, edits: &[SpecEdit]) -> Result<SpecTree, StructuredEditError> {
    let mut replacements = Vec::new();
    let mut property_targets = HashSet::new();
    let mut deleted_blocks = HashSet::new();
    let mut annotation_targets = HashSet::new();

    let mut replacement_deletes = HashSet::new();
    let mut replacement_inserts = std::collections::HashMap::new();
    for (index, pair) in edits.windows(2).enumerate() {
        if let [
            SpecEdit::DeleteBlock { id },
            SpecEdit::InsertBlock { parent, block },
        ] = pair
            && block.replacement
            && tree.parent_block_id(id).as_ref() == Some(parent)
        {
            replacement_deletes.insert(index);
            replacement_inserts.insert(index + 1, id.clone());
        }
    }

    for (index, edit) in edits.iter().enumerate() {
        match edit {
            SpecEdit::DeleteBlock { id } => {
                if replacement_deletes.contains(&index) {
                    continue;
                }
                if !deleted_blocks.insert(id.clone()) {
                    return Err(StructuredEditError::ConflictingEdits(id.clone()));
                }
            }
            SpecEdit::SetProperty { id, .. }
            | SpecEdit::RemoveProperty { id, .. }
            | SpecEdit::SetAnnotation { id, .. } => {
                if deleted_blocks.contains(id) {
                    return Err(StructuredEditError::ConflictingEdits(id.clone()));
                }
                if matches!(edit, SpecEdit::SetAnnotation { .. })
                    && !annotation_targets.insert(id.clone())
                {
                    return Err(StructuredEditError::ConflictingEdits(id.clone()));
                }
            }
            SpecEdit::InsertBlock { .. } => {}
        }
    }
    // The first pass only catches delete-before-mutate. Catch the reverse order.
    for (index, edit) in edits.iter().enumerate() {
        if let SpecEdit::DeleteBlock { id } = edit
            && !replacement_deletes.contains(&index)
            && (annotation_targets.contains(id)
                || edits.iter().any(|candidate| {
                    matches!(candidate,
                    SpecEdit::SetProperty { id: other, .. }
                    | SpecEdit::RemoveProperty { id: other, .. } if other == id)
                }))
        {
            return Err(StructuredEditError::ConflictingEdits(id.clone()));
        }
    }

    for (order, edit) in edits.iter().enumerate() {
        match edit {
            SpecEdit::InsertBlock { parent, block } => {
                if let Some(replaced) = replacement_inserts.get(&order) {
                    let target = tree
                        .block(replaced)
                        .ok_or_else(|| classify_missing(tree, replaced))?;
                    let mut handled = false;
                    if let (Some(new_name), Some(old_name)) =
                        (&block.replacement_name, target.name_node())
                    {
                        replacements.push(Replacement {
                            range: byte_range(old_name.text_range()),
                            text: new_name.clone(),
                            order,
                        });
                        handled = true;
                    }
                    if let Some(desired) = &block.replacement_trailing_comment {
                        let offset = u32::from(target.node().text_range().end()) as usize;
                        replacements.push(Replacement {
                            range: target
                                .trailing_line_comment_range()
                                .unwrap_or(offset..offset),
                            text: desired.clone().unwrap_or_default(),
                            order,
                        });
                        handled = true;
                    }
                    if !handled {
                        replacements.push(Replacement {
                            range: byte_range(target.node().text_range()),
                            text: replacement_text(tree, &target, block),
                            order,
                        });
                    }
                    continue;
                }
                let (offset, text) = insertion(tree, parent, block)?;
                replacements.push(Replacement {
                    range: offset..offset,
                    text,
                    order,
                });
            }
            SpecEdit::DeleteBlock { id } => {
                if replacement_deletes.contains(&order) {
                    continue;
                }
                let block = tree.block(id).ok_or_else(|| classify_missing(tree, id))?;
                push_removal(
                    &mut replacements,
                    tree.source(),
                    byte_range(block.node().text_range()),
                    order,
                )?;
            }
            SpecEdit::SetProperty { id, key, value } => {
                if !property_targets.insert((id.clone(), key.clone())) {
                    return Err(StructuredEditError::DuplicateProperty {
                        id: id.clone(),
                        key: key.0.clone(),
                    });
                }
                let block = tree.block(id).ok_or_else(|| classify_missing(tree, id))?;
                let matches: Vec<_> = block
                    .properties()
                    .into_iter()
                    .filter(|property| property.key() == key.as_str())
                    .collect();
                if matches.len() > 1 {
                    return Err(StructuredEditError::DuplicateProperty {
                        id: id.clone(),
                        key: key.0.clone(),
                    });
                }
                if let Some(property) = matches.first() {
                    let range = value_range(property.text(), property.node().text_range());
                    replacements.push(Replacement {
                        range,
                        text: value.0.clone(),
                        order,
                    });
                } else {
                    let (offset, text) = property_insertion(tree, &block, key, value)?;
                    replacements.push(Replacement {
                        range: offset..offset,
                        text,
                        order,
                    });
                }
            }
            SpecEdit::RemoveProperty { id, key } => {
                if !property_targets.insert((id.clone(), key.clone())) {
                    return Err(StructuredEditError::DuplicateProperty {
                        id: id.clone(),
                        key: key.0.clone(),
                    });
                }
                let block = tree.block(id).ok_or_else(|| classify_missing(tree, id))?;
                let matches: Vec<_> = block
                    .properties()
                    .into_iter()
                    .filter(|property| property.key() == key.as_str())
                    .collect();
                if matches.len() > 1 {
                    return Err(StructuredEditError::DuplicateProperty {
                        id: id.clone(),
                        key: key.0.clone(),
                    });
                }
                if let Some(property) = matches.first() {
                    let range =
                        line_aware_removal(tree.source(), byte_range(property.node().text_range()));
                    push_removal(&mut replacements, tree.source(), range, order)?;
                }
            }
            SpecEdit::SetAnnotation { id, binding } => {
                let block = tree.block(id).ok_or_else(|| classify_missing(tree, id))?;
                if let Some(annotation) = block.annotation() {
                    push_annotation_edits(
                        &mut replacements,
                        tree.source(),
                        &annotation,
                        binding,
                        order,
                    )?;
                } else {
                    let rendered = render_annotation(binding);
                    let offset = byte_range(block.node().text_range()).start;
                    let line_start = tree.source()[..offset]
                        .rfind('\n')
                        .map_or(0, |position| position + 1);
                    let indent = &tree.source()[line_start..offset];
                    let indent = if indent.chars().all(|ch| matches!(ch, ' ' | '\t')) {
                        indent
                    } else {
                        ""
                    };
                    let newline = newline_style(tree.source());
                    replacements.push(Replacement {
                        range: offset..offset,
                        text: format!("{rendered}{newline}{indent}"),
                        order,
                    });
                }
            }
        }
    }

    replacements.sort_by(|a, b| {
        a.range
            .start
            .cmp(&b.range.start)
            .then(a.range.end.cmp(&b.range.end))
    });
    replacements.dedup_by(|a, b| a.range == b.range && a.text.is_empty() && b.text.is_empty());
    for pair in replacements.windows(2) {
        if pair[0].range.end > pair[1].range.start {
            return Err(StructuredEditError::OverlappingEdits {
                first: pair[0].range.clone(),
                second: pair[1].range.clone(),
            });
        }
    }

    // Descending byte order keeps all ranges valid. At the same insertion point,
    // reverse requested order so repeated `insert_str` produces caller order.
    replacements.sort_by(|a, b| {
        b.range
            .start
            .cmp(&a.range.start)
            .then(b.order.cmp(&a.order))
    });
    let mut source = tree.source().to_owned();
    for replacement in replacements {
        source.replace_range(replacement.range, &replacement.text);
    }
    SpecTree::parse(source).map_err(StructuredEditError::Parse)
}

fn replacement_text(
    tree: &SpecTree,
    target: &super::access::BlockRef<'_>,
    block: &IntentBlock,
) -> String {
    let start = u32::from(target.node().text_range().start()) as usize;
    let line_start = tree.source()[..start]
        .rfind('\n')
        .map_or(0, |position| position + 1);
    let indent = &tree.source()[line_start..start];
    let newline = newline_style(tree.source());
    let normalized = normalize_newlines(block.as_str(), newline);
    let mut lines = normalized.split(newline);
    let mut result = lines.next().unwrap_or_default().to_owned();
    for line in lines {
        result.push_str(newline);
        if !line.is_empty() {
            result.push_str(indent);
        }
        result.push_str(line);
    }
    result
}

fn classify_missing(tree: &SpecTree, id: &SourceId) -> StructuredEditError {
    if tree.node(id).is_some() {
        StructuredEditError::NotABlock(id.clone())
    } else {
        StructuredEditError::UnknownSourceId(id.clone())
    }
}

fn insertion(
    tree: &SpecTree,
    parent: &SourceId,
    block: &IntentBlock,
) -> Result<(usize, String), StructuredEditError> {
    if parent.is_root() {
        let offset = tree.source().len();
        let newline = newline_style(tree.source());
        let prefix = if tree.source().is_empty() {
            ""
        } else if tree.source().ends_with(&format!("{newline}{newline}")) {
            ""
        } else if tree.source().ends_with(newline) {
            newline
        } else {
            // This branch needs an owned value, handled below.
            ""
        };
        let owned_prefix;
        let prefix = if !tree.source().is_empty()
            && !tree.source().ends_with(newline)
            && prefix.is_empty()
        {
            owned_prefix = format!("{newline}{newline}");
            owned_prefix.as_str()
        } else {
            prefix
        };
        let normalized = normalize_newlines(block.as_str(), newline);
        let suffix = if normalized.ends_with('\n') {
            ""
        } else {
            newline
        };
        return Ok((offset, format!("{prefix}{normalized}{suffix}")));
    }

    let block_ref = tree
        .block(parent)
        .ok_or_else(|| classify_missing(tree, parent))?;
    let body = body_node(block_ref.node())
        .ok_or_else(|| StructuredEditError::NotAContainer(parent.clone()))?;
    let close = body
        .last_token()
        .filter(|token| token.kind() == K::RBrace)
        .expect("structured container body ends in '}'");
    let offset = u32::from(close.text_range().start()) as usize;
    let indent = child_indent(tree.source(), body);
    let newline = newline_style(tree.source());
    let rendered = indent_fragment(block.as_str().trim_end(), &indent, newline);
    let needs_leading_newline = !tree.source()[..offset].ends_with('\n');
    Ok((
        offset,
        format!(
            "{}{rendered}{newline}",
            if needs_leading_newline { newline } else { "" }
        ),
    ))
}

fn property_insertion(
    tree: &SpecTree,
    block: &super::access::BlockRef<'_>,
    key: &PropertyKey,
    value: &ExprSource,
) -> Result<(usize, String), StructuredEditError> {
    let body = body_node(block.node())
        .ok_or_else(|| StructuredEditError::NotAContainer(block.id().clone()))?;
    let close = body
        .last_token()
        .filter(|token| token.kind() == K::RBrace)
        .expect("structured container body ends in '}'");
    let offset = u32::from(close.text_range().start()) as usize;
    let indent = child_indent(tree.source(), body);
    let newline = newline_style(tree.source());
    let leading = if tree.source()[..offset].ends_with('\n') {
        ""
    } else {
        newline
    };
    Ok((
        offset,
        format!(
            "{leading}{indent}{}: {}{newline}",
            key.as_str(),
            value.as_str()
        ),
    ))
}

fn child_indent(source: &str, body: &super::syntax::ResolvedNode) -> String {
    if let Some(first) = body.children().find(|node| node.kind() != K::Annotation) {
        let start = u32::from(first.text_range().start()) as usize;
        let line_start = source[..start].rfind('\n').map_or(0, |pos| pos + 1);
        let indent = &source[line_start..start];
        if indent.chars().all(|ch| matches!(ch, ' ' | '\t')) {
            return indent.to_owned();
        }
    }
    let body_start = u32::from(body.text_range().start()) as usize;
    let line_start = source[..body_start].rfind('\n').map_or(0, |pos| pos + 1);
    let parent_indent: String = source[line_start..body_start]
        .chars()
        .take_while(|ch| matches!(ch, ' ' | '\t'))
        .collect();
    format!("{parent_indent}    ")
}

fn indent_fragment(fragment: &str, indent: &str, newline: &str) -> String {
    fragment
        .lines()
        .map(|line| format!("{indent}{line}"))
        .collect::<Vec<_>>()
        .join(newline)
}

fn value_range(property_text: &str, absolute: cstree::text::TextRange) -> std::ops::Range<usize> {
    let colon = property_text.find(':').expect("property has ':'");
    let after_colon = &property_text[colon + 1..];
    let leading = after_colon.len() - after_colon.trim_start().len();
    let trailing = after_colon.len() - after_colon.trim_end().len();
    let start = u32::from(absolute.start()) as usize + colon + 1 + leading;
    let end = u32::from(absolute.end()) as usize - trailing;
    start..end
}

fn line_aware_removal(source: &str, range: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let line_start = source[..range.start].rfind('\n').map_or(0, |pos| pos + 1);
    let line_end = source[range.end..]
        .find('\n')
        .map(|pos| range.end + pos + 1)
        .unwrap_or(range.end);
    let before = &source[line_start..range.start];
    let after = &source[range.end..line_end];
    if before.chars().all(|ch| matches!(ch, ' ' | '\t'))
        && after
            .trim_matches(|ch: char| matches!(ch, ' ' | '\t' | ',' | '\r' | '\n'))
            .is_empty()
    {
        line_start..line_end
    } else {
        range
    }
}

fn push_removal(
    replacements: &mut Vec<Replacement>,
    source: &str,
    range: std::ops::Range<usize>,
    order: usize,
) -> Result<(), StructuredEditError> {
    let separator = adjacent_comma(source, &range)?;
    replacements.push(Replacement {
        range,
        text: String::new(),
        order,
    });
    if let Some(range) = separator {
        replacements.push(Replacement {
            range,
            text: String::new(),
            order,
        });
    }
    Ok(())
}

/// Find a comma separated from a node only by lossless trivia. The comma is
/// returned separately so comments between the node and separator survive.
fn adjacent_comma(
    source: &str,
    removed: &std::ops::Range<usize>,
) -> Result<Option<std::ops::Range<usize>>, StructuredEditError> {
    let tokens = lex_lossless(source).map_err(StructuredEditError::Parse)?;
    let is_trivia = |kind: K| {
        matches!(
            kind,
            K::Whitespace | K::Newline | K::LineComment | K::BlockComment
        )
    };
    if let Some(token) = tokens
        .iter()
        .filter(|token| token.range.start >= removed.end)
        .find(|token| !is_trivia(token.kind))
        && token.kind == K::Comma
    {
        return Ok(Some(token.range.clone()));
    }
    if let Some(token) = tokens
        .iter()
        .rev()
        .filter(|token| token.range.end <= removed.start)
        .find(|token| !is_trivia(token.kind))
        && token.kind == K::Comma
    {
        return Ok(Some(token.range.clone()));
    }
    Ok(None)
}

fn push_annotation_edits(
    replacements: &mut Vec<Replacement>,
    source: &str,
    annotation: &super::access::AnnotationRef<'_>,
    binding: &BindingMetadata,
    order: usize,
) -> Result<(), StructuredEditError> {
    let desired = [
        ("id", binding.id.as_ref().map(|value| quote(value))),
        ("stable", binding.stable.map(|value| value.to_string())),
        ("group", binding.group.as_ref().map(|value| quote(value))),
        (
            "source_id",
            binding.source_id.as_ref().map(|value| quote(value)),
        ),
    ];
    let args: Vec<_> = annotation
        .node()
        .children()
        .filter(|node| node.kind() == K::AnnotationArg)
        .collect();
    let mut present = HashSet::new();
    let mut remaining = args.len();
    for arg in args {
        let text = super::access::slice_node(source, arg);
        let (key, _) = text.split_once('=').expect("annotation arg contains '='");
        let key = key.trim();
        present.insert(key.to_owned());
        let wanted = desired
            .iter()
            .find_map(|(candidate, value)| (*candidate == key).then_some(value))
            .expect("structured parser rejects unknown annotation keys");
        if let Some(value) = wanted {
            let range = annotation_value_range(text, arg.text_range());
            if &source[range.clone()] != value {
                replacements.push(Replacement {
                    range,
                    text: value.clone(),
                    order,
                });
            }
        } else {
            remaining -= 1;
            push_removal(replacements, source, byte_range(arg.text_range()), order)?;
        }
    }

    let additions: Vec<_> = desired
        .iter()
        .filter_map(|(key, value)| {
            (!present.contains(*key))
                .then(|| value.as_ref().map(|value| format!("{key} = {value}")))?
        })
        .collect();
    if !additions.is_empty() {
        let annotation_range = byte_range(annotation.node().text_range());
        let relative_close = annotation
            .text()
            .rfind(')')
            .expect("annotation closes with ')'");
        let offset = annotation_range.start + relative_close;
        let has_trailing_comma = lex_lossless(source)
            .map_err(StructuredEditError::Parse)?
            .into_iter()
            .rev()
            .filter(|token| token.range.end <= offset)
            .find(|token| {
                !matches!(
                    token.kind,
                    K::Whitespace | K::Newline | K::LineComment | K::BlockComment
                )
            })
            .is_some_and(|token| token.kind == K::Comma);
        let prefix = if remaining > 0 && !has_trailing_comma {
            ", "
        } else {
            ""
        };
        replacements.push(Replacement {
            range: offset..offset,
            text: format!("{prefix}{}", additions.join(", ")),
            order,
        });
    }
    Ok(())
}

fn annotation_value_range(
    arg_text: &str,
    absolute: cstree::text::TextRange,
) -> std::ops::Range<usize> {
    let equals = arg_text.find('=').expect("annotation arg contains '='");
    let after_equals = &arg_text[equals + 1..];
    let leading = after_equals.len() - after_equals.trim_start().len();
    let trailing = after_equals.len() - after_equals.trim_end().len();
    let start = u32::from(absolute.start()) as usize + equals + 1 + leading;
    let end = u32::from(absolute.end()) as usize - trailing;
    start..end
}

fn render_annotation(binding: &BindingMetadata) -> String {
    let mut args = Vec::new();
    if let Some(value) = &binding.id {
        args.push(format!("id = {}", quote(value)));
    }
    if let Some(value) = binding.stable {
        args.push(format!("stable = {value}"));
    }
    if let Some(value) = &binding.group {
        args.push(format!("group = {}", quote(value)));
    }
    if let Some(value) = &binding.source_id {
        args.push(format!("source_id = {}", quote(value)));
    }
    format!("#[annotation({})]", args.join(", "))
}

fn newline_style(source: &str) -> &'static str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn normalize_newlines(source: &str, newline: &str) -> String {
    let lf = source.replace("\r\n", "\n");
    if newline == "\n" {
        lf
    } else {
        lf.replace('\n', newline)
    }
}

fn quote(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a Rust string cannot fail")
}

fn byte_range(range: cstree::text::TextRange) -> std::ops::Range<usize> {
    u32::from(range.start()) as usize..u32::from(range.end()) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(value: &str) -> PropertyKey {
        PropertyKey::new(value).unwrap()
    }

    fn expr(value: &str) -> ExprSource {
        ExprSource::parse(value).unwrap()
    }

    #[test]
    fn set_existing_property_changes_only_value_bytes() {
        let tree =
            SpecTree::parse("// header\ncomponent R {\n\tdescription  :  \"old\" // keep\n}\n")
                .unwrap();
        let id = tree.top_level_blocks()[0].id().clone();
        let edited = apply_edits(
            &tree,
            &[SpecEdit::SetProperty {
                id,
                key: key("description"),
                value: expr("\"new\""),
            }],
        )
        .unwrap();
        assert_eq!(
            edited.source(),
            "// header\ncomponent R {\n\tdescription  :  \"new\" // keep\n}\n"
        );
    }

    #[test]
    fn insert_remove_property_and_set_annotation_preserve_surroundings() {
        let tree =
            SpecTree::parse("// header\ncomponent R {\n    old: 1\n    // body comment\n}\n")
                .unwrap();
        let id = tree.top_level_blocks()[0].id().clone();
        let edited = apply_edits(
            &tree,
            &[
                SpecEdit::RemoveProperty {
                    id: id.clone(),
                    key: key("old"),
                },
                SpecEdit::SetProperty {
                    id: id.clone(),
                    key: key("value"),
                    value: expr("\"10k\""),
                },
                SpecEdit::SetAnnotation {
                    id,
                    binding: BindingMetadata {
                        id: Some("AAAA1111".into()),
                        ..Default::default()
                    },
                },
            ],
        )
        .unwrap();
        assert!(
            edited
                .source()
                .starts_with("// header\n#[annotation(id = \"AAAA1111\")]\ncomponent R")
        );
        assert!(
            edited
                .source()
                .contains("    // body comment\n    value: \"10k\"\n}")
        );
        assert!(!edited.source().contains("old: 1"));
    }

    #[test]
    fn insert_and_delete_blocks_leave_unrelated_bytes_unchanged() {
        let tree = SpecTree::parse("// A\ncomponent A {}\n\n// B\ncomponent B {}\n").unwrap();
        let a = tree.top_level_blocks()[0].id().clone();
        let edited = apply_edits(
            &tree,
            &[
                SpecEdit::DeleteBlock { id: a },
                SpecEdit::InsertBlock {
                    parent: SourceId::root(),
                    block: IntentBlock::parse("component C {}").unwrap(),
                },
            ],
        )
        .unwrap();
        assert!(edited.source().contains("// A\n\n\n// B\ncomponent B {}"));
        assert!(edited.source().ends_with("\ncomponent C {}\n"));
    }

    #[test]
    fn expression_fragment_cannot_break_out_of_wrapper() {
        assert!(matches!(
            ExprSource::parse("1 } component X {").unwrap_err(),
            StructuredEditError::ExpectedSingleExpression
        ));
    }

    #[test]
    fn inline_property_removal_consumes_a_separator() {
        let tree = SpecTree::parse("component A { a: 1, b: 2 }").unwrap();
        let id = tree.top_level_blocks()[0].id().clone();
        let edited = apply_edits(&tree, &[SpecEdit::RemoveProperty { id, key: key("a") }]).unwrap();
        assert_eq!(edited.source(), "component A {  b: 2 }");
    }

    #[test]
    fn removal_preserves_comment_between_property_and_separator() {
        let tree = SpecTree::parse("component A { a: 1 /* keep */, b: 2 }").unwrap();
        let id = tree.top_level_blocks()[0].id().clone();
        let edited = apply_edits(&tree, &[SpecEdit::RemoveProperty { id, key: key("a") }]).unwrap();
        assert_eq!(edited.source(), "component A {  /* keep */ b: 2 }");
    }

    #[test]
    fn set_annotation_preserves_internal_comments_and_formatting() {
        let tree = SpecTree::parse(
            "#[annotation(id = \"AAAAAAAA\", // keep\n             stable = true)]\ncomponent A {}\n",
        )
        .unwrap();
        let id = tree.top_level_blocks()[0].id().clone();
        let edited = apply_edits(
            &tree,
            &[SpecEdit::SetAnnotation {
                id,
                binding: BindingMetadata {
                    id: Some("BBBBBBBB".into()),
                    stable: Some(true),
                    ..Default::default()
                },
            }],
        )
        .unwrap();
        assert_eq!(
            edited.source(),
            "#[annotation(id = \"BBBBBBBB\", // keep\n             stable = true)]\ncomponent A {}\n"
        );
    }

    #[test]
    fn adjacent_removals_share_separator_without_conflict() {
        let tree = SpecTree::parse("component A { a: 1, b: 2 }").unwrap();
        let id = tree.top_level_blocks()[0].id().clone();
        let edited = apply_edits(
            &tree,
            &[
                SpecEdit::RemoveProperty {
                    id: id.clone(),
                    key: key("a"),
                },
                SpecEdit::RemoveProperty { id, key: key("b") },
            ],
        )
        .unwrap();
        assert_eq!(edited.source(), "component A {   }");
    }

    #[test]
    fn clearing_annotation_args_shares_separator_without_conflict() {
        let tree =
            SpecTree::parse("#[annotation(id = \"AAAAAAAA\", stable = true)]\ncomponent A {}\n")
                .unwrap();
        let id = tree.top_level_blocks()[0].id().clone();
        let edited = apply_edits(
            &tree,
            &[SpecEdit::SetAnnotation {
                id,
                binding: BindingMetadata::default(),
            }],
        )
        .unwrap();
        assert_eq!(
            edited.top_level_blocks()[0]
                .annotation()
                .unwrap()
                .metadata(),
            BindingMetadata::default()
        );
        assert!(!edited.source().contains("AAAAAAAA"));
    }

    #[test]
    fn annotation_addition_reuses_trailing_comma() {
        let tree = SpecTree::parse("#[annotation(id = \"AAAAAAAA\",)]\ncomponent A {}\n").unwrap();
        let id = tree.top_level_blocks()[0].id().clone();
        let edited = apply_edits(
            &tree,
            &[SpecEdit::SetAnnotation {
                id,
                binding: BindingMetadata {
                    id: Some("AAAAAAAA".into()),
                    stable: Some(true),
                    ..Default::default()
                },
            }],
        )
        .unwrap();
        assert!(edited.source().contains("id = \"AAAAAAAA\",stable = true"));
    }

    #[test]
    fn root_insertion_uses_existing_newline_style() {
        let tree = SpecTree::parse("component A {}\r\n").unwrap();
        let edited = apply_edits(
            &tree,
            &[SpecEdit::InsertBlock {
                parent: tree.root_id(),
                block: IntentBlock::parse("component B {\n    value: 1\n}\n").unwrap(),
            }],
        )
        .unwrap();
        assert!(!edited.source().replace("\r\n", "").contains('\n'));
    }

    #[test]
    fn nested_intent_block_can_be_built_from_typed_accessor() {
        let tree = SpecTree::parse("component A { pin 1 {} }").unwrap();
        let pin = tree.top_level_blocks()[0].child_blocks()[0].clone();
        assert_eq!(IntentBlock::from_block(&pin).as_str(), "pin 1 {}");
    }

    #[test]
    fn stale_source_id_does_not_retarget_a_sibling() {
        let tree = SpecTree::parse("component A {}\ncomponent B {}\n").unwrap();
        let stale_b = tree.top_level_blocks()[1].id().clone();
        let a = tree.top_level_blocks()[0].id().clone();
        let edited = apply_edits(&tree, &[SpecEdit::DeleteBlock { id: a }]).unwrap();
        assert!(matches!(
            apply_edits(
                &edited,
                &[SpecEdit::SetAnnotation {
                    id: stale_b,
                    binding: BindingMetadata::default(),
                }]
            ),
            Err(StructuredEditError::UnknownSourceId(_))
        ));
    }

    #[test]
    fn nested_annotation_preserves_indent_and_crlf() {
        let tree = SpecTree::parse("sheet {\r\n    constraint near {}\r\n}\r\n").unwrap();
        let constraint = tree.top_level_blocks()[0].child_blocks()[0].id().clone();
        let edited = apply_edits(
            &tree,
            &[SpecEdit::SetAnnotation {
                id: constraint,
                binding: BindingMetadata {
                    id: Some("AAAA1111".into()),
                    ..Default::default()
                },
            }],
        )
        .unwrap();
        assert!(
            edited
                .source()
                .contains("    #[annotation(id = \"AAAA1111\")]\r\n    constraint near {}")
        );
    }
}

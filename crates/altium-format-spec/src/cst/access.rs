//! Typed, read-only accessors over the lossless spec CST.
//!
//! The raw `cstree` nodes remain available for parser development, but callers
//! that inspect or edit specs should use this layer. It deliberately exposes
//! only grammar concepts and source identities, rather than token indexes or
//! green-tree implementation details.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use super::parse_structured;
use super::syntax::{ResolvedNode, SyntaxKind as K};
use crate::diagnostic::ParseError;

/// Identity of a node within one parsed source snapshot.
///
/// Each segment is a child-*node* index (tokens and trivia are excluded). This
/// makes the identity independent of formatting, while keeping stale IDs from
/// accidentally selecting a different byte offset after an edit. IDs are only
/// valid for the [`SpecTree`] that produced them.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceId {
    snapshot: u64,
    path: Vec<u32>,
}

impl SourceId {
    /// The source-file root.
    pub fn root() -> Self {
        Self {
            snapshot: 0,
            path: Vec::new(),
        }
    }

    pub(crate) fn child(&self, index: usize) -> Self {
        let mut path = self.path.clone();
        path.push(index as u32);
        Self {
            snapshot: self.snapshot,
            path,
        }
    }

    pub(crate) fn segments(&self) -> &[u32] {
        &self.path
    }

    pub(crate) fn is_root(&self) -> bool {
        self.path.is_empty()
    }
}

impl fmt::Display for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            return f.write_str("root");
        }
        write!(f, "{}:", self.snapshot)?;
        for (index, segment) in self.path.iter().enumerate() {
            if index > 0 {
                f.write_str("/")?;
            }
            write!(f, "{segment}")?;
        }
        Ok(())
    }
}

/// Grammar-level kind of an editable declaration or item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BlockKind {
    Import,
    LetBinding,
    Component,
    Footprint,
    SwapGroup,
    Pin,
    Parameter,
    Part,
    Alias,
    FootprintMap,
    Graphic,
    PinConnection,
    PadNet,
    PinPadPair,
    Pad,
    Row,
    Column,
    Grid,
    Project,
    Sheet,
    Net,
    Power,
    SchDocObject,
    Board,
    Placement,
    Routing,
    PcbPrimitive,
    Polygon,
    Rule,
    Class,
    DiffPair,
    Document,
    AnnotationBlock,
    MatchParameter,
    ErcMatrix,
    ErcMatrixEntry,
    ErcLevels,
    ErcLevelEntry,
    OutputGroup,
    Output,
    Comparison,
    ComparisonRule,
    ClassGen,
    LibraryUpdate,
    Variant,
    Variation,
    ParamVariation,
    Place,
    PlacementConstraint,
    Minimize,
    PlacementGroup,
    PlacementSeparate,
    Optimize,
    Clearance,
    Autoplace,
    Entry,
    Constraint,
    FontBlock,
    Font,
}

impl BlockKind {
    pub(crate) fn from_syntax(kind: K) -> Option<Self> {
        Some(match kind {
            K::Import => Self::Import,
            K::LetBinding => Self::LetBinding,
            K::Component => Self::Component,
            K::Footprint => Self::Footprint,
            K::SwapGroup => Self::SwapGroup,
            K::Pin => Self::Pin,
            K::Parameter => Self::Parameter,
            K::Part => Self::Part,
            K::Alias => Self::Alias,
            K::FootprintMap => Self::FootprintMap,
            K::Graphic => Self::Graphic,
            K::PinConnection => Self::PinConnection,
            K::PadNet => Self::PadNet,
            K::PinPadPair => Self::PinPadPair,
            K::Pad => Self::Pad,
            K::Row => Self::Row,
            K::Column => Self::Column,
            K::Grid => Self::Grid,
            K::Project => Self::Project,
            K::Sheet => Self::Sheet,
            K::Net => Self::Net,
            K::Power => Self::Power,
            K::SchDocObject => Self::SchDocObject,
            K::Board => Self::Board,
            K::Placement => Self::Placement,
            K::Routing => Self::Routing,
            K::PcbPrimitive => Self::PcbPrimitive,
            K::Polygon => Self::Polygon,
            K::Rule => Self::Rule,
            K::Class => Self::Class,
            K::DiffPair => Self::DiffPair,
            K::DocumentBlock => Self::Document,
            K::AnnotationBlock => Self::AnnotationBlock,
            K::MatchParameter => Self::MatchParameter,
            K::ErcMatrix => Self::ErcMatrix,
            K::ErcMatrixEntry => Self::ErcMatrixEntry,
            K::ErcLevels => Self::ErcLevels,
            K::ErcLevelEntry => Self::ErcLevelEntry,
            K::OutputGroup => Self::OutputGroup,
            K::Output => Self::Output,
            K::Comparison => Self::Comparison,
            K::ComparisonRule => Self::ComparisonRule,
            K::ClassGen => Self::ClassGen,
            K::LibraryUpdate => Self::LibraryUpdate,
            K::Variant => Self::Variant,
            K::Variation => Self::Variation,
            K::ParamVariation => Self::ParamVariation,
            K::Place => Self::Place,
            K::PlacementConstraint => Self::PlacementConstraint,
            K::Minimize => Self::Minimize,
            K::PlacementGroup => Self::PlacementGroup,
            K::PlacementSeparate => Self::PlacementSeparate,
            K::Optimize => Self::Optimize,
            K::Clearance => Self::Clearance,
            K::Autoplace => Self::Autoplace,
            K::Entry => Self::Entry,
            K::Constraint => Self::Constraint,
            K::FontBlock => Self::FontBlock,
            K::Font => Self::Font,
            _ => return None,
        })
    }
}

/// Typed values supported by `#[annotation(...)]`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BindingMetadata {
    pub id: Option<String>,
    pub stable: Option<bool>,
    pub group: Option<String>,
    pub source_id: Option<String>,
}

/// A parsed lossless spec source plus typed accessors over its tree.
pub struct SpecTree {
    source: String,
    root: ResolvedNode,
    snapshot: u64,
}

static NEXT_SNAPSHOT: AtomicU64 = AtomicU64::new(1);

impl SpecTree {
    pub fn parse(source: impl Into<String>) -> Result<Self, ParseError> {
        let source = source.into();
        let root = parse_structured(&source)?;
        let snapshot = NEXT_SNAPSHOT.fetch_add(1, Ordering::Relaxed);
        Ok(Self {
            source,
            root,
            snapshot,
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn root_id(&self) -> SourceId {
        SourceId {
            snapshot: self.snapshot,
            path: Vec::new(),
        }
    }

    pub fn top_level_blocks(&self) -> Vec<BlockRef<'_>> {
        child_blocks(&self.root, &self.source, &self.root_id())
    }

    pub fn block(&self, id: &SourceId) -> Option<BlockRef<'_>> {
        let node = self.node(id)?;
        BlockKind::from_syntax(node.kind())?;
        Some(BlockRef {
            source: &self.source,
            node,
            id: id.clone(),
        })
    }

    pub(crate) fn node(&self, id: &SourceId) -> Option<&ResolvedNode> {
        if !id.is_root() && id.snapshot != self.snapshot {
            return None;
        }
        let mut node = &self.root;
        for segment in id.segments() {
            node = node.children().nth(*segment as usize)?;
        }
        Some(node)
    }

    pub(crate) fn parent_block_id(&self, id: &SourceId) -> Option<SourceId> {
        if !id.is_root() && id.snapshot != self.snapshot {
            return None;
        }
        let mut node = &self.root;
        let mut node_id = self.root_id();
        let mut parent_block = self.root_id();
        for segment in id
            .segments()
            .iter()
            .take(id.segments().len().saturating_sub(1))
        {
            node = node.children().nth(*segment as usize)?;
            node_id = node_id.child(*segment as usize);
            if BlockKind::from_syntax(node.kind()).is_some() {
                parent_block = node_id.clone();
            }
        }
        Some(parent_block)
    }
}

/// Typed view of one declaration/item node.
#[derive(Clone)]
pub struct BlockRef<'a> {
    source: &'a str,
    node: &'a ResolvedNode,
    id: SourceId,
}

impl<'a> BlockRef<'a> {
    pub fn id(&self) -> &SourceId {
        &self.id
    }

    pub fn kind(&self) -> BlockKind {
        BlockKind::from_syntax(self.node.kind()).expect("BlockRef always wraps a block kind")
    }

    pub fn text(&self) -> &'a str {
        slice_node(self.source, self.node)
    }

    /// Declared name/path/binding used as the natural identity fallback.
    pub fn natural_key(&self) -> Option<String> {
        if let Some(name) = self.node.children().find(|node| node.kind() == K::Name) {
            return decode_string_or_ident(slice_node(self.source, name));
        }
        if self.node.kind() == K::Import {
            return direct_token_text(self.node, self.source, K::String).and_then(decode_string);
        }
        if let Some(binding) = self.node.children().find(|node| node.kind() == K::Binding) {
            return direct_token_text(binding, self.source, K::Ident).map(str::to_owned);
        }
        if self.node.kind() == K::LetBinding {
            return direct_token_text(self.node, self.source, K::Ident).map(str::to_owned);
        }
        None
    }

    pub(crate) fn name_node(&self) -> Option<&'a ResolvedNode> {
        self.node.children().find(|node| node.kind() == K::Name)
    }

    pub(crate) fn name_source(&self) -> Option<&'a str> {
        self.name_node().map(|node| slice_node(self.source, node))
    }

    pub fn annotation(&self) -> Option<AnnotationRef<'a>> {
        let node = self
            .node
            .children()
            .find(|node| node.kind() == K::Annotation)?;
        Some(AnnotationRef {
            source: self.source,
            node,
        })
    }

    pub fn properties(&self) -> Vec<PropertyRef<'a>> {
        let Some(body) = body_node(self.node) else {
            return Vec::new();
        };
        body.children()
            .filter(|node| node.kind() == K::Property)
            .map(|node| PropertyRef {
                source: self.source,
                node,
            })
            .collect()
    }

    pub fn property(&self, key: &str) -> Option<PropertyRef<'a>> {
        self.properties()
            .into_iter()
            .find(|property| property.key() == key)
    }

    pub fn child_blocks(&self) -> Vec<BlockRef<'a>> {
        let Some(body) = body_node(self.node) else {
            return Vec::new();
        };
        let body_index = self
            .node
            .children()
            .position(|node| std::ptr::eq(node, body))
            .expect("body is a direct child");
        let body_id = self.id.child(body_index);
        child_blocks(body, self.source, &body_id)
    }

    pub(crate) fn node(&self) -> &'a ResolvedNode {
        self.node
    }

    pub(crate) fn source(&self) -> &'a str {
        self.source
    }

    pub(crate) fn trailing_line_comment(&self) -> Option<&'a str> {
        let range = self.trailing_line_comment_range()?;
        Some(&self.source[range])
    }

    pub(crate) fn trailing_line_comment_range(&self) -> Option<std::ops::Range<usize>> {
        let end = u32::from(self.node.text_range().end()) as usize;
        let line_end = self.source[end..]
            .find('\n')
            .map_or(self.source.len(), |offset| end + offset);
        let trailing = &self.source[end..line_end];
        trailing
            .trim_start()
            .starts_with("//")
            .then_some(end..line_end)
    }
}

/// Typed view of one `key: value` property.
#[derive(Clone, Copy)]
pub struct PropertyRef<'a> {
    source: &'a str,
    node: &'a ResolvedNode,
}

impl<'a> PropertyRef<'a> {
    pub fn key(&self) -> &'a str {
        let token = self.node.first_token().expect("property has a key token");
        slice_range(self.source, token.text_range())
    }

    pub fn value_text(&self) -> &'a str {
        let text = slice_node(self.source, self.node);
        let colon = text.find(':').expect("property has a colon");
        text[colon + 1..].trim()
    }

    pub fn text(&self) -> &'a str {
        slice_node(self.source, self.node)
    }

    pub(crate) fn node(&self) -> &'a ResolvedNode {
        self.node
    }
}

/// Typed view of a sync annotation.
pub struct AnnotationRef<'a> {
    source: &'a str,
    node: &'a ResolvedNode,
}

impl AnnotationRef<'_> {
    pub fn metadata(&self) -> BindingMetadata {
        let mut result = BindingMetadata::default();
        for arg in self
            .node
            .children()
            .filter(|node| node.kind() == K::AnnotationArg)
        {
            let text = slice_node(self.source, arg);
            let Some((key, value)) = text.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            match key {
                "id" => result.id = decode_string(value),
                "stable" => {
                    result.stable = match value {
                        "true" => Some(true),
                        "false" => Some(false),
                        _ => None,
                    }
                }
                "group" => result.group = decode_string(value),
                "source_id" => result.source_id = decode_string(value),
                _ => continue,
            }
        }
        result
    }

    pub fn text(&self) -> &str {
        slice_node(self.source, self.node)
    }

    pub(crate) fn node(&self) -> &ResolvedNode {
        self.node
    }
}

fn child_blocks<'a>(
    parent: &'a ResolvedNode,
    source: &'a str,
    parent_id: &SourceId,
) -> Vec<BlockRef<'a>> {
    parent
        .children()
        .enumerate()
        .filter_map(|(index, node)| {
            BlockKind::from_syntax(node.kind())?;
            Some(BlockRef {
                source,
                node,
                id: parent_id.child(index),
            })
        })
        .collect()
}

pub(crate) fn body_node(node: &ResolvedNode) -> Option<&ResolvedNode> {
    node.children()
        .find(|child| matches!(child.kind(), K::Block | K::Object))
}

pub(crate) fn slice_node<'a>(source: &'a str, node: &ResolvedNode) -> &'a str {
    slice_range(source, node.text_range())
}

pub(crate) fn slice_range(source: &str, range: cstree::text::TextRange) -> &str {
    &source[u32::from(range.start()) as usize..u32::from(range.end()) as usize]
}

fn direct_token_text<'a>(node: &ResolvedNode, source: &'a str, kind: K) -> Option<&'a str> {
    node.children_with_tokens().find_map(|element| {
        if element.kind() == kind {
            Some(slice_range(source, element.text_range()))
        } else {
            None
        }
    })
}

fn decode_string_or_ident(value: &str) -> Option<String> {
    let value = value.trim();
    if value.starts_with('"') {
        decode_string(value)
    } else {
        Some(value.to_owned())
    }
}

fn decode_string(value: &str) -> Option<String> {
    serde_json::from_str(value.trim()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_accessors_expose_blocks_properties_and_annotations() {
        let tree = SpecTree::parse(
            "#[annotation(id = \"AAAA1111\", stable = true, source_id = \"U1\")]\n\
             component \"R1\" {\n    description: \"resistor\"\n    pin 1 { electrical: passive }\n}\n",
        ).unwrap();
        let component = tree.top_level_blocks().pop().unwrap();
        assert_eq!(component.kind(), BlockKind::Component);
        assert_eq!(component.natural_key().as_deref(), Some("R1"));
        assert_eq!(
            component.property("description").unwrap().value_text(),
            "\"resistor\""
        );
        assert_eq!(component.child_blocks()[0].kind(), BlockKind::Pin);
        assert_eq!(
            component.annotation().unwrap().metadata(),
            BindingMetadata {
                id: Some("AAAA1111".into()),
                stable: Some(true),
                group: None,
                source_id: Some("U1".into()),
            }
        );
        assert!(tree.block(component.id()).is_some());
    }

    #[test]
    fn typed_accessors_decode_escaped_strings() {
        let tree = SpecTree::parse(
            "#[annotation(id = \"AA\\\"AA111\", source_id = \"A\\\\B\")]\ncomponent \"R\\\"1\" {}\n",
        )
        .unwrap();
        let component = &tree.top_level_blocks()[0];
        assert_eq!(component.natural_key().as_deref(), Some("R\"1"));
        let metadata = component.annotation().unwrap().metadata();
        assert_eq!(metadata.id.as_deref(), Some("AA\"AA111"));
        assert_eq!(metadata.source_id.as_deref(), Some("A\\B"));
    }
}

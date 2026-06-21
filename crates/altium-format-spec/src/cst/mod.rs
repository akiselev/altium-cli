//! Lossless concrete syntax tree (CST) for the spec language.
//!
//! Milestone 1: a lossless lexer plus a flat green-tree builder, with the
//! defining invariant proven by test — `parse_lossless(src).text() == src`,
//! byte for byte. Tree *structure* (typed blocks, properties, expressions) and
//! the structured-edit API are layered on top of this in later milestones; the
//! token stream the structured parser emits is exactly the one produced here, so
//! losslessness is preserved by construction.

pub mod access;
pub mod edit;
pub mod lexer;
pub mod merge;
pub mod parser;
pub mod syntax;

pub use access::{
    AnnotationRef, BindingMetadata, BlockKind, BlockRef, PropertyRef, SourceId, SpecTree,
};
pub use edit::{ExprSource, IntentBlock, PropertyKey, SpecEdit, StructuredEditError, apply_edits};
pub use merge::{DumpMergeError, merge_dump};
pub use parser::parse_structured;

use cstree::build::GreenNodeBuilder;

use crate::cst::syntax::{ResolvedNode, SyntaxKind, SyntaxNode};
use crate::diagnostic::ParseError;

/// Lex `source` and build a lossless green tree: a single `Root` node whose
/// children are every leaf token (including whitespace and comments) in order.
///
/// Returns a resolved root from which `.text()` reproduces `source` exactly.
pub fn parse_lossless(source: &str) -> Result<ResolvedNode, ParseError> {
    let tokens = lexer::lex_lossless(source)?;

    let mut builder: GreenNodeBuilder<SyntaxKind> = GreenNodeBuilder::new();
    builder.start_node(SyntaxKind::Root);
    for tok in &tokens {
        builder.token(tok.kind, &source[tok.range.clone()]);
    }
    builder.finish_node();

    let (green, cache) = builder.finish();
    let interner = cache
        .expect("fresh GreenNodeBuilder always owns its interner")
        .into_interner()
        .expect("owned interner is recoverable");
    Ok(SyntaxNode::new_root_with_resolver(green, interner))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The core invariant: a parsed tree reproduces its source byte-for-byte.
    fn assert_roundtrip(src: &str) {
        let root = parse_lossless(src).expect("lex should succeed");
        assert_eq!(root.text(), src, "CST text must equal source byte-for-byte");
    }

    #[test]
    fn roundtrip_empty() {
        assert_roundtrip("");
    }

    #[test]
    fn roundtrip_whitespace_only() {
        assert_roundtrip("   \t \r  ");
        assert_roundtrip("\n\n");
        assert_roundtrip("  \n\t\n ");
    }

    #[test]
    fn roundtrip_minimal_component() {
        assert_roundtrip("component R_0603 {}");
    }

    #[test]
    fn roundtrip_preserves_comments_and_layout() {
        let src = "\
// leading comment
component R_0603 {
    designator: \"R\"   // inline comment
    /* block
       comment */
    pin 1 { at: (100mil, 0mil), electrical: passive }
    parameter \"Value\" { text: \"10k\" }
}

// trailing comment
";
        assert_roundtrip(src);
    }

    #[test]
    fn roundtrip_irregular_whitespace() {
        // Tabs, trailing spaces, CRLF, blank lines, odd indentation.
        let src = "component\tR {\r\n  \t designator:  \"R\"  \r\n\n}\r\n";
        assert_roundtrip(src);
    }

    #[test]
    fn roundtrip_expressions_and_literals() {
        let src = "let x = 3.140 + #FF00AA\nfootprint F { pad 1 { at: (-1.5mm, 0mil) } }\n";
        assert_roundtrip(src);
    }

    #[test]
    fn roundtrip_annotation_and_arrow() {
        let src = "#[annotation(id = \"AB12CD34\")]\nsheet { pin A -> #NET ... }\n";
        assert_roundtrip(src);
    }
}

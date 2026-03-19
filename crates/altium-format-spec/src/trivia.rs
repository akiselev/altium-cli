use std::collections::BTreeMap;

use crate::ast::{PlacementItem, SpecFile, SpecItem};
use crate::diagnostic::{ParseError, Span};
use crate::eval::{SpecError, SpecErrorCode};
use crate::lexer::lex;
use crate::parser::parse_spec_from_tokens;

// ── Shared trivia types (used by both formatter and rewriter) ─────────────────

/// Trivia attached to a top-level item (used by the formatter).
#[derive(Debug, Default, Clone)]
pub struct ItemTrivia {
    /// Lines (or blank lines) that appear before the item in the source.
    pub leading: Vec<TriviaLine>,
    /// A single trailing comment on the same line as the item (if any).
    pub trailing: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TriviaLine {
    Blank,
    LineComment(String),
    BlockComment(String),
}

/// Scan a gap string (between two top-level items) for trivia lines.
pub fn scan_trivia_lines(gap: &str) -> Vec<TriviaLine> {
    let mut result = Vec::new();
    let mut i = 0;
    let bytes = gap.as_bytes();

    while i < bytes.len() {
        // Skip horizontal whitespace.
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\r') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'\n' {
            result.push(TriviaLine::Blank);
            i += 1;
            continue;
        }
        // Line comment.
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            let start = i;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            let text = gap[start..i].trim_end().to_string();
            result.push(TriviaLine::LineComment(text));
            // Consume the trailing newline so it doesn't become a Blank.
            if i < bytes.len() && bytes[i] == b'\n' {
                i += 1;
            }
            continue;
        }
        // Block comment.
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            let start = i;
            i += 2;
            let mut depth = 1u32;
            while i < bytes.len() {
                if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                    depth += 1;
                    i += 2;
                } else if i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            let text = gap[start..i].to_string();
            result.push(TriviaLine::BlockComment(text));
            // Skip trailing whitespace and newline after block comment.
            while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\r') {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'\n' {
                i += 1;
            }
            continue;
        }
        // Non-whitespace, non-comment character (shouldn't happen in a gap).
        i += 1;
    }

    result
}

// ── CommentToken ──────────────────────────────────────────────────────────────

/// A comment captured during lexing.
#[derive(Debug, Clone, PartialEq)]
pub struct CommentToken {
    /// Byte span of the comment in the source, including delimiters.
    pub span: Span,
    /// Raw text of the comment including delimiters (`//…` or `/*…*/`).
    pub text: String,
    /// `true` for block comments (`/* … */`), `false` for line comments (`// …`).
    pub is_block: bool,
}

// ── TriviaMap ─────────────────────────────────────────────────────────────────

/// Associates comments with PlaceDecl AST node spans.
///
/// Leading trivia: comments immediately before a node, keyed by the node's
/// start byte. Trailing trivia: an inline comment on the same line as the
/// node's closing token, keyed by the node's end byte.
pub struct TriviaMap {
    /// Comments preceding a node. Key = node start byte.
    leading: BTreeMap<u32, Vec<CommentToken>>,
    /// Trailing inline comment after a node. Key = node end byte.
    trailing: BTreeMap<u32, CommentToken>,
    /// All comments, in source order.
    all: Vec<CommentToken>,
}

impl TriviaMap {
    /// Build a TriviaMap from lexer comments and AST PlaceDecl spans.
    ///
    /// Only PlaceDecl spans are indexed (the rewriter only replaces those
    /// nodes). Comments not near any PlaceDecl are preserved in gap ranges.
    pub fn build(comments: Vec<CommentToken>, ast: &SpecFile, source: &str) -> Self {
        let place_spans = collect_place_spans(ast);

        let mut leading: BTreeMap<u32, Vec<CommentToken>> = BTreeMap::new();
        let mut trailing: BTreeMap<u32, CommentToken> = BTreeMap::new();

        'outer: for comment in &comments {
            // Check if comment is a trailing comment for a preceding node
            // (same line as the node's closing token).
            for &node_span in &place_spans {
                if comment.span.start >= node_span.end {
                    // Check no newline between node end and comment start.
                    let between = &source[node_span.end as usize..comment.span.start as usize];
                    if !between.contains('\n') {
                        trailing.insert(node_span.end, comment.clone());
                        continue 'outer;
                    }
                }
            }

            // Find the nearest following PlaceDecl by start byte.
            let following_start = place_spans
                .iter()
                .filter(|s| s.start >= comment.span.end)
                .map(|s| s.start)
                .next();

            if let Some(node_start) = following_start {
                leading.entry(node_start).or_default().push(comment.clone());
            }
            // Comments with no following PlaceDecl are orphans — not attached.
        }

        Self {
            leading,
            trailing,
            all: comments,
        }
    }

    /// Returns leading trivia comments for a node identified by its span.
    ///
    /// These are comments immediately before the node in source order.
    pub fn leading(&self, span: Span) -> &[CommentToken] {
        self.leading
            .get(&span.start)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Returns the trailing inline comment for a node identified by its span,
    /// if any. A trailing comment appears on the same line as the node's end.
    pub fn trailing(&self, span: Span) -> Option<&CommentToken> {
        self.trailing.get(&span.end)
    }

    /// Returns all comments whose spans fall within the byte range `[start, end)`.
    pub fn in_range(&self, start: u32, end: u32) -> Vec<&CommentToken> {
        self.all
            .iter()
            .filter(|c| c.span.start >= start && c.span.end <= end)
            .collect()
    }
}

// ── Span collection ───────────────────────────────────────────────────────────

/// Collect all PlaceDecl spans from the AST, in source order.
fn collect_place_spans(ast: &SpecFile) -> Vec<Span> {
    let mut spans = Vec::new();
    for item in &ast.items {
        if let SpecItem::Placement(placement) = &item.node {
            for placement_item in &placement.body {
                if let PlacementItem::Place(_) = &placement_item.node {
                    spans.push(placement_item.span);
                }
            }
        }
    }
    // Sort by start byte so binary search / iteration is in source order.
    spans.sort_by_key(|s| s.start);
    spans
}

// ── Public parse entrypoint ───────────────────────────────────────────────────

/// Parse a spec file and build the associated TriviaMap in one pass.
///
/// Calls `lex()` once to get both the token stream and comment side channel,
/// passes the tokens to the parser, and builds the TriviaMap from comments
/// and the resulting AST.
pub fn parse_with_trivia(source: &str) -> Result<(SpecFile, TriviaMap), SpecError> {
    let (tokens, comments) = lex(source).map_err(|e| parse_error_to_spec_error(e))?;
    let ast = parse_spec_from_tokens(source, tokens)
        .map_err(|e| parse_error_to_spec_error(e))?;
    let trivia = TriviaMap::build(comments, &ast, source);
    Ok((ast, trivia))
}

fn parse_error_to_spec_error(e: ParseError) -> SpecError {
    SpecError::new(
        SpecErrorCode::ParseError,
        e.message,
        Some(e.span),
    )
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn lex_comments(input: &str) -> Vec<CommentToken> {
        lex(input).unwrap().1
    }

    #[test]
    fn test_line_comment_captured() {
        let comments = lex_comments("a // this is a comment\nb");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, "// this is a comment");
        assert!(!comments[0].is_block);
        // span starts at byte 2 (after "a ")
        assert_eq!(comments[0].span.start, 2);
        assert_eq!(comments[0].span.end, 22);
    }

    #[test]
    fn test_block_comment_captured() {
        let comments = lex_comments("a /* block comment */ b");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, "/* block comment */");
        assert!(comments[0].is_block);
    }

    #[test]
    fn test_nested_block_comment_captured_as_single() {
        let comments = lex_comments("a /* outer /* inner */ outer */ b");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, "/* outer /* inner */ outer */");
        assert!(comments[0].is_block);
    }

    #[test]
    fn test_multiple_comments_captured() {
        let comments = lex_comments("// first\n// second\n");
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].text, "// first");
        assert_eq!(comments[1].text, "// second");
    }

    #[test]
    fn test_empty_spec_empty_trivia_map() {
        let (ast, trivia) = parse_with_trivia("").unwrap();
        assert!(ast.items.is_empty());
        assert!(trivia.all.is_empty());
    }

    #[test]
    fn test_parse_with_trivia_basic() {
        let source = r#"
placement {
    // a component
    place U1 {
        autoplace: true
    }
}
"#;
        let (ast, trivia) = parse_with_trivia(source).unwrap();
        assert!(!ast.items.is_empty());
        // There should be one comment
        assert_eq!(trivia.all.len(), 1);
        assert_eq!(trivia.all[0].text, "// a component");
    }

    #[test]
    fn test_leading_trivia_for_place_block() {
        let source = r#"placement {
// before place
place U1 {
    autoplace: true
}
}"#;
        let (ast, trivia) = parse_with_trivia(source).unwrap();

        // Find the PlaceDecl span
        let place_span = {
            let mut span = None;
            for item in &ast.items {
                if let SpecItem::Placement(placement) = &item.node {
                    for pitem in &placement.body {
                        if let PlacementItem::Place(_) = &pitem.node {
                            span = Some(pitem.span);
                        }
                    }
                }
            }
            span.expect("no place decl found")
        };

        let leading = trivia.leading(place_span);
        assert_eq!(leading.len(), 1);
        assert_eq!(leading[0].text, "// before place");
    }

    #[test]
    fn test_trailing_trivia_for_place_block() {
        let source = "placement {\nplace U1 {\n    autoplace: true\n} // trailing\n}";
        let (ast, trivia) = parse_with_trivia(source).unwrap();

        let place_span = {
            let mut span = None;
            for item in &ast.items {
                if let SpecItem::Placement(placement) = &item.node {
                    for pitem in &placement.body {
                        if let PlacementItem::Place(_) = &pitem.node {
                            span = Some(pitem.span);
                        }
                    }
                }
            }
            span.expect("no place decl found")
        };

        let trailing = trivia.trailing(place_span);
        assert!(trailing.is_some());
        assert_eq!(trailing.unwrap().text, "// trailing");
    }

    #[test]
    fn test_comment_between_two_place_blocks_attaches_to_second() {
        let source = r#"placement {
place U1 {
    autoplace: true
}
// between blocks
place U2 {
    autoplace: true
}
}"#;
        let (ast, trivia) = parse_with_trivia(source).unwrap();

        let place_spans: Vec<Span> = {
            let mut spans = Vec::new();
            for item in &ast.items {
                if let SpecItem::Placement(placement) = &item.node {
                    for pitem in &placement.body {
                        if let PlacementItem::Place(_) = &pitem.node {
                            spans.push(pitem.span);
                        }
                    }
                }
            }
            spans
        };
        assert_eq!(place_spans.len(), 2);

        // Comment should be leading trivia for the second place block
        let leading_u2 = trivia.leading(place_spans[1]);
        assert_eq!(leading_u2.len(), 1);
        assert_eq!(leading_u2[0].text, "// between blocks");

        // First block should have no leading trivia
        let leading_u1 = trivia.leading(place_spans[0]);
        assert!(leading_u1.is_empty());
    }

    #[test]
    fn test_orphan_comment_at_eof_not_attached() {
        let source = "placement {\n}\n// orphan at eof";
        let (_ast, trivia) = parse_with_trivia(source).unwrap();
        // Comment exists in all
        assert_eq!(trivia.all.len(), 1);
        // But leading is empty (no following place decl)
        assert!(trivia.leading.is_empty());
        // And trailing is empty (no place decl ends before it on same line)
        assert!(trivia.trailing.is_empty());
    }

    #[test]
    fn test_in_range() {
        let source = "placement {\n// c1\nplace U1 {\n    autoplace: true\n}\n// c2\n}";
        let (_ast, trivia) = parse_with_trivia(source).unwrap();
        // Get the comment that's within the placement block
        let all_in_file = trivia.in_range(0, source.len() as u32);
        assert_eq!(all_in_file.len(), trivia.all.len());
    }

    #[test]
    fn test_parse_with_trivia_roundtrip() {
        let source = r#"placement {
    // leading comment
    place C1 {
        autoplace: true
    } // trailing
}
"#;
        let result = parse_with_trivia(source);
        assert!(result.is_ok(), "parse_with_trivia failed: {:?}", result.err());
        let (ast, _trivia) = result.unwrap();
        assert!(!ast.items.is_empty());
    }
}

//! Lossless lexer for the CST.
//!
//! Rather than re-implement tokenization, this reuses the existing, tested
//! [`crate::lexer::lex`] (which already produces byte-accurate token spans and a
//! separate comment list) and reconstructs a *gapless* token sequence covering
//! every byte of the source. The only bytes the underlying lexer does not emit as
//! tokens are runs of `[ \t\r]` (it drops them) — those become `Whitespace`
//! tokens here. `\n` is already emitted as a token, and comments come from the
//! comment list. The result is therefore guaranteed to match the existing
//! tokenizer's boundaries exactly, plus trivia.

use std::ops::Range;

use crate::cst::syntax::SyntaxKind;
use crate::diagnostic::{ParseError, ParseErrorCode, Span};
use crate::lexer::{TokenKind, lex};

/// A single lossless leaf token: its kind and its byte range in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LosslessToken {
    pub kind: SyntaxKind,
    pub range: Range<usize>,
}

/// Tokenize `source` into a gapless sequence of leaf tokens covering `0..len`.
///
/// Errors only when the underlying lexer errors (e.g. an unterminated block
/// comment or a genuinely invalid character) or when a gap between tokens
/// unexpectedly contains a non-`[ \t\r]` byte (which would indicate a tokenizer
/// bug rather than malformed input).
pub fn lex_lossless(source: &str) -> Result<Vec<LosslessToken>, ParseError> {
    let (tokens, comments) = lex(source)?;

    // Merge real tokens (excluding the zero-width EOF) and comments into one
    // span-ordered list.
    let mut items: Vec<(usize, usize, SyntaxKind)> =
        Vec::with_capacity(tokens.len() + comments.len());
    for t in &tokens {
        if matches!(t.kind, TokenKind::Eof) {
            continue;
        }
        items.push((
            t.span.start as usize,
            t.span.end as usize,
            map_token_kind(&t.kind),
        ));
    }
    for c in &comments {
        let kind = if c.is_block {
            SyntaxKind::BlockComment
        } else {
            SyntaxKind::LineComment
        };
        items.push((c.span.start as usize, c.span.end as usize, kind));
    }
    items.sort_by_key(|(start, _, _)| *start);

    let mut out: Vec<LosslessToken> = Vec::with_capacity(items.len() * 2 + 1);
    let mut pos = 0usize;
    for (start, end, kind) in items {
        if start > pos {
            push_whitespace_gap(&mut out, source, pos..start)?;
        }
        out.push(LosslessToken {
            kind,
            range: start..end,
        });
        pos = end;
    }
    if pos < source.len() {
        push_whitespace_gap(&mut out, source, pos..source.len())?;
    }

    Ok(out)
}

/// Emit a single `Whitespace` token for a gap, verifying it contains only
/// `[ \t\r]` (newlines are tokenized, so they never appear in a gap).
fn push_whitespace_gap(
    out: &mut Vec<LosslessToken>,
    source: &str,
    range: Range<usize>,
) -> Result<(), ParseError> {
    let slice = &source.as_bytes()[range.clone()];
    if let Some(bad) = slice
        .iter()
        .position(|&b| !matches!(b, b' ' | b'\t' | b'\r'))
    {
        let off = range.start + bad;
        return Err(ParseError::new(
            ParseErrorCode::E1001,
            format!(
                "internal: unexpected byte {:#04x} in inter-token gap (not whitespace)",
                slice[bad]
            ),
            Span::new(off as u32, (off + 1) as u32),
        ));
    }
    out.push(LosslessToken {
        kind: SyntaxKind::Whitespace,
        range,
    });
    Ok(())
}

/// Map an existing tokenizer kind to its CST leaf kind. Payloads are ignored —
/// the CST stores the raw source text, not parsed values.
fn map_token_kind(kind: &TokenKind) -> SyntaxKind {
    use SyntaxKind as S;
    match kind {
        TokenKind::Ident(_) => S::Ident,
        TokenKind::DollarIdent(_) => S::DollarIdent,
        TokenKind::String(_) => S::String,
        TokenKind::Template(_) => S::Template,
        TokenKind::Integer(_) => S::Int,
        TokenKind::Float(_) => S::Float,
        TokenKind::Dim(_, _) => S::Dim,
        TokenKind::Color(_, _, _) => S::Color,

        TokenKind::Import => S::ImportKw,
        TokenKind::As => S::AsKw,
        TokenKind::Component => S::ComponentKw,
        TokenKind::Footprint => S::FootprintKw,
        TokenKind::Project => S::ProjectKw,
        TokenKind::Sheet => S::SheetKw,
        TokenKind::Net => S::NetKw,
        TokenKind::Power => S::PowerKw,
        TokenKind::Pin => S::PinKw,
        TokenKind::Pad => S::PadKw,
        TokenKind::Part => S::PartKw,
        TokenKind::Parameter => S::ParameterKw,
        TokenKind::Alias => S::AliasKw,
        TokenKind::Row => S::RowKw,
        TokenKind::Column => S::ColumnKw,
        TokenKind::Grid => S::GridKw,
        TokenKind::Board => S::BoardKw,
        TokenKind::SwapGroup => S::SwapGroupKw,
        TokenKind::Group => S::GroupKw,
        TokenKind::Separate => S::SeparateKw,
        TokenKind::Autoplace => S::AutoplaceKw,
        TokenKind::PadNet => S::PadNetKw,
        TokenKind::Let => S::LetKw,
        TokenKind::True => S::TrueKw,
        TokenKind::False => S::FalseKw,
        TokenKind::Null => S::NullKw,

        TokenKind::LBrace => S::LBrace,
        TokenKind::RBrace => S::RBrace,
        TokenKind::LParen => S::LParen,
        TokenKind::RParen => S::RParen,
        TokenKind::LBracket => S::LBracket,
        TokenKind::RBracket => S::RBracket,
        TokenKind::Colon => S::Colon,
        TokenKind::Comma => S::Comma,
        TokenKind::Dot => S::Dot,
        TokenKind::DotDotDot => S::DotDotDot,
        TokenKind::Eq => S::Eq,
        TokenKind::Arrow => S::Arrow,
        TokenKind::Plus => S::Plus,
        TokenKind::Minus => S::Minus,
        TokenKind::Star => S::Star,
        TokenKind::Slash => S::Slash,
        TokenKind::Semi => S::Semi,
        TokenKind::Hash => S::Hash,
        // EOF is filtered out before mapping.
        TokenKind::Newline => S::Newline,
        TokenKind::Eof => S::Error,
    }
}

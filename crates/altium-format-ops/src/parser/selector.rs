use std::str::FromStr;

use super::ast::{
    SelectorAttrOp, SelectorAttribute, SelectorChain, SelectorCombinator, SelectorCompound,
    SelectorExpr, SelectorFilter, SelectorLink, SelectorSimple, SelectorStringMode, SelectorValue,
    SelectorWildcard, Span, Spanned, Unit,
};
use super::diagnostic::{ParseError, ParseErrorCode};

#[derive(Debug, Clone, PartialEq)]
enum TokKind {
    Ident(String),
    DollarIdent(String),
    String(String),
    Integer(i32),
    Float(f64),
    Dim(f64, Unit),
    Regex(String),

    LBracket,
    RBracket,
    Colon,
    Dot,
    Comma,

    Eq,
    Ne,
    Contains,
    StartsWith,
    EndsWith,
    WordMatch,
    Gt,
    Lt,
    Ge,
    Le,

    Plus,
    Tilde,
    At,
    Percent,
    Hash,
    Star,
    Question,

    And,
    Or,
    Not,
    Ws,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
struct Tok {
    kind: TokKind,
    span: Span,
}

pub fn parse_selector(source: &str, base_offset: u32) -> Result<Spanned<SelectorExpr>, ParseError> {
    let tokens = lex_selector(source, base_offset)?;
    let mut p = SelectorParser { tokens, pos: 0 };
    let expr = p.parse_or()?;
    p.skip_ws();
    if !p.at(&TokKind::Eof) {
        return Err(ParseError::new(
            ParseErrorCode::E1006,
            "unexpected token at end of selector",
            p.current().span,
        )
        .with_help("remove trailing tokens or join selector fragments with AND/OR/combinators"));
    }
    Ok(expr)
}

struct SelectorParser {
    tokens: Vec<Tok>,
    pos: usize,
}

impl SelectorParser {
    fn parse_or(&mut self) -> Result<Spanned<SelectorExpr>, ParseError> {
        let mut items = vec![self.parse_and()?];
        loop {
            self.skip_ws();
            if self.consume_if(&TokKind::Or) || self.consume_if(&TokKind::Comma) {
                self.skip_ws();
                items.push(self.parse_and()?);
                continue;
            }
            break;
        }

        if items.len() == 1 {
            Ok(items.pop().expect("len checked"))
        } else {
            let span = merge_list_span(&items);
            Ok(Spanned::new(SelectorExpr::Or(items), span))
        }
    }

    fn parse_and(&mut self) -> Result<Spanned<SelectorExpr>, ParseError> {
        let mut items = vec![self.parse_not()?];
        loop {
            let had_ws = self.skip_ws();
            if self.consume_if(&TokKind::And) {
                self.skip_ws();
                items.push(self.parse_not()?);
                continue;
            }

            if had_ws && self.starts_compound() {
                items.push(self.parse_not()?);
                continue;
            }

            break;
        }

        if items.len() == 1 {
            Ok(items.pop().expect("len checked"))
        } else {
            let span = merge_list_span(&items);
            Ok(Spanned::new(SelectorExpr::And(items), span))
        }
    }

    fn parse_not(&mut self) -> Result<Spanned<SelectorExpr>, ParseError> {
        self.skip_ws();
        if self.consume_if(&TokKind::Not) {
            let start = self.prev().span;
            self.skip_ws();
            let inner = self.parse_not()?;
            let span = start.merge(inner.span);
            Ok(Spanned::new(SelectorExpr::Not(Box::new(inner)), span))
        } else {
            self.parse_chain()
        }
    }

    fn parse_chain(&mut self) -> Result<Spanned<SelectorExpr>, ParseError> {
        let first = self.parse_compound()?;
        let mut rest = Vec::new();

        loop {
            let ws_before = self.skip_ws();
            let (comb, explicit) = if self.consume_if(&TokKind::Gt) {
                (SelectorCombinator::Child, true)
            } else if self.consume_if(&TokKind::Plus) {
                (SelectorCombinator::Adjacent, true)
            } else if self.consume_if(&TokKind::Tilde) {
                (SelectorCombinator::Sibling, true)
            } else if ws_before && self.starts_compound() {
                (SelectorCombinator::Descendant, false)
            } else {
                break;
            };

            let comb_span = if explicit {
                self.prev().span
            } else {
                Span::new(first.span.end, first.span.end.saturating_add(1))
            };

            self.skip_ws();
            if !self.starts_compound() {
                return Err(ParseError::new(
                    ParseErrorCode::E1006,
                    "expected selector after combinator",
                    self.current().span,
                )
                .with_help("example: component > pin:power"));
            }
            let right = self.parse_compound()?;
            let link_span = comb_span.merge(right.span);
            rest.push(Spanned::new(
                SelectorLink {
                    combinator: Spanned::new(comb, comb_span),
                    right,
                },
                link_span,
            ));
        }

        let span = if let Some(last) = rest.last() {
            first.span.merge(last.span)
        } else {
            first.span
        };

        Ok(Spanned::new(
            SelectorExpr::Chain(SelectorChain { first, rest }),
            span,
        ))
    }

    fn parse_compound(&mut self) -> Result<Spanned<SelectorCompound>, ParseError> {
        self.skip_ws();
        let start = self.current().span;

        let mut head = None;
        let mut filters = Vec::new();

        if self.starts_simple_head() {
            head = Some(self.parse_simple()?);
        }

        while self.at(&TokKind::LBracket) || self.at(&TokKind::Colon) {
            if self.at(&TokKind::LBracket) {
                filters.push(self.parse_attr_filter()?);
            } else {
                filters.push(self.parse_pseudo_filter()?);
            }
        }

        let head = head.unwrap_or_else(|| Spanned::new(SelectorSimple::Any, start));
        if matches!(head.node, SelectorSimple::Any) && filters.is_empty() {
            return Err(
                ParseError::new(ParseErrorCode::E1006, "expected selector term", start)
                    .with_help("examples: R*, component[designator=R1], pin:power"),
            );
        }

        let end = filters.last().map(|v| v.span).unwrap_or(head.span);
        Ok(Spanned::new(
            SelectorCompound { head, filters },
            start.merge(end),
        ))
    }

    fn parse_simple(&mut self) -> Result<Spanned<SelectorSimple>, ParseError> {
        let tok = self.bump().clone();
        match tok.kind {
            TokKind::DollarIdent(v) => Ok(Spanned::new(SelectorSimple::DollarRef(v), tok.span)),
            TokKind::Tilde => {
                let ident = self.expect_ident("expected net name after '~'")?;
                let span = tok.span.merge(ident.span);
                Ok(Spanned::new(SelectorSimple::NetPattern(ident.node), span))
            }
            TokKind::At => {
                let value = self.parse_value(false)?;
                let span = tok.span.merge(value.span);
                Ok(Spanned::new(SelectorSimple::ValuePattern(value.node), span))
            }
            TokKind::Percent => {
                let ident = self.expect_ident("expected part number after '%'")?;
                let span = tok.span.merge(ident.span);
                Ok(Spanned::new(SelectorSimple::PartPattern(ident.node), span))
            }
            TokKind::Hash => {
                let n = self.expect_integer("expected integer after '#'")?;
                let span = tok.span.merge(n.span);
                Ok(Spanned::new(SelectorSimple::IdPattern(n.node), span))
            }
            TokKind::Ident(v) => {
                if self.at(&TokKind::Colon) && !is_type_keyword(&v) {
                    self.bump();
                    let rhs = self.expect_ident("expected identifier after ':'")?;
                    let span = tok.span.merge(rhs.span);
                    return Ok(Spanned::new(
                        SelectorSimple::ComponentPin {
                            component: v,
                            pin: rhs.node,
                        },
                        span,
                    ));
                }

                let wildcard = if self.consume_if(&TokKind::Star) {
                    Some(SelectorWildcard::AnySuffix)
                } else if self.consume_if(&TokKind::Question) {
                    if self.consume_if(&TokKind::Question) {
                        Some(SelectorWildcard::TwoChars)
                    } else {
                        Some(SelectorWildcard::OneChar)
                    }
                } else {
                    None
                };

                if is_type_keyword(&v) && wildcard.is_none() {
                    return Ok(Spanned::new(SelectorSimple::Type(v), tok.span));
                }

                let end = self.prev().span;
                Ok(Spanned::new(
                    SelectorSimple::DesignatorPattern { ident: v, wildcard },
                    tok.span.merge(end),
                ))
            }
            _ => Err(
                ParseError::new(ParseErrorCode::E1006, "expected selector head", tok.span)
                    .with_help("examples: component, R*, ~VCC, @10K, $caps"),
            ),
        }
    }

    fn parse_attr_filter(&mut self) -> Result<Spanned<SelectorFilter>, ParseError> {
        let lb = self.expect(TokKind::LBracket, "expected '['")?.span;

        let mut field = Vec::new();
        let first = self.expect_ident("expected field name in attribute selector")?;
        field.push(first.clone());
        while self.consume_if(&TokKind::Dot) {
            field.push(self.expect_ident("expected field segment after '.'")?);
        }

        let op_tok = self.bump().clone();
        let op = match op_tok.kind {
            TokKind::Eq => SelectorAttrOp::Eq,
            TokKind::Ne => SelectorAttrOp::Ne,
            TokKind::Contains => SelectorAttrOp::Contains,
            TokKind::StartsWith => SelectorAttrOp::StartsWith,
            TokKind::EndsWith => SelectorAttrOp::EndsWith,
            TokKind::WordMatch => SelectorAttrOp::WordMatch,
            TokKind::Gt => SelectorAttrOp::Gt,
            TokKind::Lt => SelectorAttrOp::Lt,
            TokKind::Ge => SelectorAttrOp::Ge,
            TokKind::Le => SelectorAttrOp::Le,
            _ => {
                return Err(ParseError::new(
                    ParseErrorCode::E1006,
                    "expected attribute operator",
                    op_tok.span,
                )
                .with_help("valid operators: = != *= ^= $= ~= > < >= <=")
                .with_note("example: [designator^=R]"));
            }
        };

        let value = self.parse_value(true)?;
        let rb_span = self
            .expect(
                TokKind::RBracket,
                "expected ']' to close attribute selector",
            )?
            .span;

        let mode = if let TokKind::Ident(v) = &self.current().kind {
            if v == "i" {
                let span = self.bump().span;
                Some(Spanned::new(SelectorStringMode::CaseInsensitive, span))
            } else if v == "s" {
                let span = self.bump().span;
                Some(Spanned::new(SelectorStringMode::CaseSensitive, span))
            } else {
                None
            }
        } else {
            None
        };

        let attr = SelectorAttribute {
            field,
            op: Spanned::new(op, op_tok.span),
            value,
            mode,
        };
        let span = lb.merge(rb_span);
        Ok(Spanned::new(SelectorFilter::Attribute(attr), span))
    }

    fn parse_pseudo_filter(&mut self) -> Result<Spanned<SelectorFilter>, ParseError> {
        let colon = self.expect(TokKind::Colon, "expected ':'")?.span;
        let ident = self.expect_ident("expected pseudo-class name after ':'")?;
        let span = colon.merge(ident.span);
        Ok(Spanned::new(
            SelectorFilter::Pseudo(Spanned::new(ident.node, ident.span)),
            span,
        ))
    }

    fn parse_value(
        &mut self,
        allow_bare_ident: bool,
    ) -> Result<Spanned<SelectorValue>, ParseError> {
        self.skip_ws();
        let tok = self.bump().clone();
        match tok.kind {
            TokKind::String(v) => Ok(Spanned::new(SelectorValue::String(v), tok.span)),
            TokKind::Integer(v) => Ok(Spanned::new(SelectorValue::Integer(v), tok.span)),
            TokKind::Float(v) => Ok(Spanned::new(SelectorValue::Float(v), tok.span)),
            TokKind::Dim(v, unit) => Ok(Spanned::new(SelectorValue::Dim(v, unit), tok.span)),
            TokKind::Regex(v) => Ok(Spanned::new(SelectorValue::Regex(v), tok.span)),
            TokKind::Ident(v) if allow_bare_ident => match v.as_str() {
                "true" => Ok(Spanned::new(SelectorValue::Bool(true), tok.span)),
                "false" => Ok(Spanned::new(SelectorValue::Bool(false), tok.span)),
                _ => Ok(Spanned::new(SelectorValue::Ident(v), tok.span)),
            },
            _ => Err(
                ParseError::new(ParseErrorCode::E1006, "expected selector value", tok.span)
                    .with_help(
                        "valid values: string, number, dim (10mil), bool, ident, regex /.../",
                    ),
            ),
        }
    }

    fn starts_compound(&self) -> bool {
        matches!(
            self.current().kind,
            TokKind::Ident(_)
                | TokKind::DollarIdent(_)
                | TokKind::LBracket
                | TokKind::Colon
                | TokKind::Tilde
                | TokKind::At
                | TokKind::Percent
                | TokKind::Hash
                | TokKind::Not
        )
    }

    fn starts_simple_head(&self) -> bool {
        matches!(
            self.current().kind,
            TokKind::Ident(_)
                | TokKind::DollarIdent(_)
                | TokKind::Tilde
                | TokKind::At
                | TokKind::Percent
                | TokKind::Hash
        )
    }

    fn expect_ident(&mut self, message: &str) -> Result<Spanned<String>, ParseError> {
        self.skip_ws();
        let tok = self.bump().clone();
        if let TokKind::Ident(v) = tok.kind {
            Ok(Spanned::new(v, tok.span))
        } else {
            Err(ParseError::new(ParseErrorCode::E1006, message, tok.span))
        }
    }

    fn expect_integer(&mut self, message: &str) -> Result<Spanned<i32>, ParseError> {
        self.skip_ws();
        let tok = self.bump().clone();
        if let TokKind::Integer(v) = tok.kind {
            Ok(Spanned::new(v, tok.span))
        } else {
            Err(ParseError::new(ParseErrorCode::E1006, message, tok.span))
        }
    }

    fn expect(&mut self, kind: TokKind, message: &str) -> Result<&Tok, ParseError> {
        self.skip_ws();
        if self.at(&kind) {
            Ok(self.bump())
        } else {
            Err(ParseError::new(
                ParseErrorCode::E1006,
                message,
                self.current().span,
            ))
        }
    }

    fn skip_ws(&mut self) -> bool {
        let mut had = false;
        while self.at(&TokKind::Ws) {
            had = true;
            self.bump();
        }
        had
    }

    fn consume_if(&mut self, kind: &TokKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn at(&self, kind: &TokKind) -> bool {
        same_variant(&self.current().kind, kind)
    }

    fn current(&self) -> &Tok {
        &self.tokens[self.pos]
    }

    fn prev(&self) -> &Tok {
        if self.pos == 0 {
            &self.tokens[0]
        } else {
            &self.tokens[self.pos - 1]
        }
    }

    fn bump(&mut self) -> &Tok {
        let idx = self.pos;
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        &self.tokens[idx]
    }
}

fn merge_list_span<T>(items: &[Spanned<T>]) -> Span {
    let first = items.first().expect("non-empty").span;
    let last = items.last().expect("non-empty").span;
    first.merge(last)
}

fn same_variant(a: &TokKind, b: &TokKind) -> bool {
    matches!(
        (a, b),
        (TokKind::Ident(_), TokKind::Ident(_))
            | (TokKind::DollarIdent(_), TokKind::DollarIdent(_))
            | (TokKind::String(_), TokKind::String(_))
            | (TokKind::Integer(_), TokKind::Integer(_))
            | (TokKind::Float(_), TokKind::Float(_))
            | (TokKind::Dim(_, _), TokKind::Dim(_, _))
            | (TokKind::Regex(_), TokKind::Regex(_))
            | (TokKind::LBracket, TokKind::LBracket)
            | (TokKind::RBracket, TokKind::RBracket)
            | (TokKind::Colon, TokKind::Colon)
            | (TokKind::Dot, TokKind::Dot)
            | (TokKind::Comma, TokKind::Comma)
            | (TokKind::Eq, TokKind::Eq)
            | (TokKind::Ne, TokKind::Ne)
            | (TokKind::Contains, TokKind::Contains)
            | (TokKind::StartsWith, TokKind::StartsWith)
            | (TokKind::EndsWith, TokKind::EndsWith)
            | (TokKind::WordMatch, TokKind::WordMatch)
            | (TokKind::Gt, TokKind::Gt)
            | (TokKind::Lt, TokKind::Lt)
            | (TokKind::Ge, TokKind::Ge)
            | (TokKind::Le, TokKind::Le)
            | (TokKind::Plus, TokKind::Plus)
            | (TokKind::Tilde, TokKind::Tilde)
            | (TokKind::At, TokKind::At)
            | (TokKind::Percent, TokKind::Percent)
            | (TokKind::Hash, TokKind::Hash)
            | (TokKind::Star, TokKind::Star)
            | (TokKind::Question, TokKind::Question)
            | (TokKind::And, TokKind::And)
            | (TokKind::Or, TokKind::Or)
            | (TokKind::Not, TokKind::Not)
            | (TokKind::Ws, TokKind::Ws)
            | (TokKind::Eof, TokKind::Eof)
    )
}

fn lex_selector(input: &str, base_offset: u32) -> Result<Vec<Tok>, ParseError> {
    let mut out = Vec::new();
    let mut i = 0usize;
    let bytes = input.as_bytes();

    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b' ' | b'\t' | b'\n' | b'\r' => {
                let start = i;
                i += 1;
                while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
                    i += 1;
                }
                out.push(tok(TokKind::Ws, base_offset, start, i));
            }
            b'[' => {
                out.push(tok(TokKind::LBracket, base_offset, i, i + 1));
                i += 1;
            }
            b']' => {
                out.push(tok(TokKind::RBracket, base_offset, i, i + 1));
                i += 1;
            }
            b':' => {
                out.push(tok(TokKind::Colon, base_offset, i, i + 1));
                i += 1;
            }
            b'.' => {
                out.push(tok(TokKind::Dot, base_offset, i, i + 1));
                i += 1;
            }
            b',' => {
                out.push(tok(TokKind::Comma, base_offset, i, i + 1));
                i += 1;
            }
            b'=' => {
                out.push(tok(TokKind::Eq, base_offset, i, i + 1));
                i += 1;
            }
            b'!' if peek(bytes, i + 1) == Some(b'=') => {
                out.push(tok(TokKind::Ne, base_offset, i, i + 2));
                i += 2;
            }
            b'*' if peek(bytes, i + 1) == Some(b'=') => {
                out.push(tok(TokKind::Contains, base_offset, i, i + 2));
                i += 2;
            }
            b'^' if peek(bytes, i + 1) == Some(b'=') => {
                out.push(tok(TokKind::StartsWith, base_offset, i, i + 2));
                i += 2;
            }
            b'$' if peek(bytes, i + 1) == Some(b'=') => {
                out.push(tok(TokKind::EndsWith, base_offset, i, i + 2));
                i += 2;
            }
            b'~' if peek(bytes, i + 1) == Some(b'=') => {
                out.push(tok(TokKind::WordMatch, base_offset, i, i + 2));
                i += 2;
            }
            b'>' if peek(bytes, i + 1) == Some(b'=') => {
                out.push(tok(TokKind::Ge, base_offset, i, i + 2));
                i += 2;
            }
            b'<' if peek(bytes, i + 1) == Some(b'=') => {
                out.push(tok(TokKind::Le, base_offset, i, i + 2));
                i += 2;
            }
            b'>' => {
                out.push(tok(TokKind::Gt, base_offset, i, i + 1));
                i += 1;
            }
            b'<' => {
                out.push(tok(TokKind::Lt, base_offset, i, i + 1));
                i += 1;
            }
            b'+' => {
                out.push(tok(TokKind::Plus, base_offset, i, i + 1));
                i += 1;
            }
            b'~' => {
                out.push(tok(TokKind::Tilde, base_offset, i, i + 1));
                i += 1;
            }
            b'@' => {
                out.push(tok(TokKind::At, base_offset, i, i + 1));
                i += 1;
            }
            b'%' => {
                out.push(tok(TokKind::Percent, base_offset, i, i + 1));
                i += 1;
            }
            b'#' => {
                out.push(tok(TokKind::Hash, base_offset, i, i + 1));
                i += 1;
            }
            b'*' => {
                out.push(tok(TokKind::Star, base_offset, i, i + 1));
                i += 1;
            }
            b'?' => {
                out.push(tok(TokKind::Question, base_offset, i, i + 1));
                i += 1;
            }
            b'"' => {
                let start = i;
                let (value, end) = read_quoted(input, i)?;
                out.push(tok(TokKind::String(value), base_offset, start, end));
                i = end;
            }
            b'$' => {
                let start = i;
                i += 1;
                let (ident, end) = read_ident(input, i)?;
                i = end;
                out.push(tok(TokKind::DollarIdent(ident), base_offset, start, end));
            }
            b'/' => {
                let start = i;
                let (pat, end) = read_regex(input, i)?;
                out.push(tok(TokKind::Regex(pat), base_offset, start, end));
                i = end;
            }
            _ if (b as char).is_ascii_digit() => {
                let start = i;
                let (kind, end) = read_number_or_dim(input, i, base_offset)?;
                out.push(tok(kind, base_offset, start, end));
                i = end;
            }
            _ => {
                let ch = input[i..].chars().next().expect("utf8");
                if is_ident_start(ch) {
                    let start = i;
                    let (ident, end) = read_ident_fast(input, i);
                    i = end;
                    let kind = match ident.as_str() {
                        "AND" | "and" => TokKind::And,
                        "OR" | "or" => TokKind::Or,
                        "NOT" | "not" => TokKind::Not,
                        _ => TokKind::Ident(ident),
                    };
                    out.push(tok(kind, base_offset, start, end));
                } else {
                    return Err(ParseError::new(
                        ParseErrorCode::E1006,
                        format!("unexpected selector character '{}'", ch),
                        Span::new(
                            base_offset + start_u32(i),
                            base_offset + start_u32(i + ch.len_utf8()),
                        ),
                    ));
                }
            }
        }
    }

    out.push(tok(TokKind::Eof, base_offset, input.len(), input.len()));
    Ok(out)
}

fn tok(kind: TokKind, base_offset: u32, start: usize, end: usize) -> Tok {
    Tok {
        kind,
        span: Span::new(base_offset + start_u32(start), base_offset + start_u32(end)),
    }
}

fn start_u32(v: usize) -> u32 {
    v as u32
}

fn peek(bytes: &[u8], idx: usize) -> Option<u8> {
    bytes.get(idx).copied()
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

fn read_ident(input: &str, start: usize) -> Result<(String, usize), ParseError> {
    let Some(ch) = input[start..].chars().next() else {
        return Err(ParseError::new(
            ParseErrorCode::E1006,
            "expected identifier",
            Span::new(start as u32, start as u32),
        ));
    };
    if !is_ident_start(ch) {
        return Err(ParseError::new(
            ParseErrorCode::E1006,
            "expected identifier",
            Span::new(start as u32, (start + ch.len_utf8()) as u32),
        ));
    }
    Ok(read_ident_fast(input, start))
}

fn read_ident_fast(input: &str, start: usize) -> (String, usize) {
    for (off, ch) in input[start..].char_indices() {
        if !is_ident_continue(ch) {
            let end = start + off;
            return (input[start..end].to_string(), end);
        }
    }
    (input[start..].to_string(), input.len())
}

fn read_quoted(input: &str, start: usize) -> Result<(String, usize), ParseError> {
    let mut i = start + 1;
    let mut out = String::new();
    while i < input.len() {
        let ch = input[i..].chars().next().expect("char");
        match ch {
            '"' => return Ok((out, i + 1)),
            '\\' => {
                i += 1;
                let Some(esc) = input[i..].chars().next() else {
                    return Err(ParseError::new(
                        ParseErrorCode::E1006,
                        "unterminated escape in selector string",
                        Span::new(start as u32, input.len() as u32),
                    ));
                };
                let mapped = match esc {
                    '"' => '"',
                    '\\' => '\\',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    _ => esc,
                };
                out.push(mapped);
                i += esc.len_utf8();
            }
            _ => {
                out.push(ch);
                i += ch.len_utf8();
            }
        }
    }

    Err(ParseError::new(
        ParseErrorCode::E1006,
        "unterminated selector string literal",
        Span::new(start as u32, input.len() as u32),
    ))
}

fn read_regex(input: &str, start: usize) -> Result<(String, usize), ParseError> {
    let mut i = start + 1;
    let mut out = String::new();
    let mut escaped = false;
    while i < input.len() {
        let ch = input[i..].chars().next().expect("char");
        if escaped {
            out.push(ch);
            escaped = false;
            i += ch.len_utf8();
            continue;
        }
        if ch == '\\' {
            out.push(ch);
            escaped = true;
            i += 1;
            continue;
        }
        if ch == '/' {
            return Ok((out, i + 1));
        }
        out.push(ch);
        i += ch.len_utf8();
    }

    Err(ParseError::new(
        ParseErrorCode::E1006,
        "unterminated regex literal",
        Span::new(start as u32, input.len() as u32),
    ))
}

fn read_number_or_dim(
    input: &str,
    start: usize,
    base_offset: u32,
) -> Result<(TokKind, usize), ParseError> {
    let mut end = start;
    while end < input.len() && input.as_bytes()[end].is_ascii_digit() {
        end += 1;
    }

    let mut is_float = false;
    if end < input.len()
        && input.as_bytes()[end] == b'.'
        && end + 1 < input.len()
        && input.as_bytes()[end + 1].is_ascii_digit()
    {
        is_float = true;
        end += 1;
        while end < input.len() && input.as_bytes()[end].is_ascii_digit() {
            end += 1;
        }
    }

    let number = &input[start..end];
    if end < input.len() {
        let ch = input[end..].chars().next().expect("char");
        if is_ident_start(ch) {
            let (unit, unit_end) = read_ident_fast(input, end);
            let unit = parse_unit(&unit).ok_or_else(|| {
                ParseError::new(
                    ParseErrorCode::E1003,
                    format!("unknown unit suffix '{}'", unit),
                    Span::new(
                        base_offset + start_u32(end),
                        base_offset + start_u32(unit_end),
                    ),
                )
                .with_help("valid units: mm, mil, in, dxp, raw")
            })?;
            let value = f64::from_str(number).map_err(|_| {
                ParseError::new(
                    ParseErrorCode::E1006,
                    "invalid numeric literal",
                    Span::new(base_offset + start_u32(start), base_offset + start_u32(end)),
                )
            })?;
            return Ok((TokKind::Dim(value, unit), unit_end));
        }
    }

    if is_float {
        let value = f64::from_str(number).map_err(|_| {
            ParseError::new(
                ParseErrorCode::E1006,
                "invalid float literal",
                Span::new(base_offset + start_u32(start), base_offset + start_u32(end)),
            )
        })?;
        return Ok((TokKind::Float(value), end));
    }

    let value = i32::from_str(number).map_err(|_| {
        ParseError::new(
            ParseErrorCode::E1006,
            "invalid integer literal",
            Span::new(base_offset + start_u32(start), base_offset + start_u32(end)),
        )
    })?;
    Ok((TokKind::Integer(value), end))
}

fn parse_unit(unit: &str) -> Option<Unit> {
    match unit {
        "mil" => Some(Unit::Mil),
        "mm" => Some(Unit::Mm),
        "in" => Some(Unit::Inch),
        "dxp" => Some(Unit::Dxp),
        "raw" => Some(Unit::Raw),
        _ => None,
    }
}

fn is_type_keyword(v: &str) -> bool {
    matches!(
        v,
        "component"
            | "pin"
            | "wire"
            | "bus"
            | "port"
            | "power"
            | "label"
            | "netlabel"
            | "junction"
            | "sheet"
            | "parameter"
            | "line"
            | "arc"
            | "text"
            | "polygon"
            | "rectangle"
            | "pad"
            | "via"
            | "track"
            | "fill"
            | "region"
            | "rule"
            | "net"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::string::string_regex;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    fn ok(src: &str) -> Spanned<SelectorExpr> {
        parse_selector(src, 0).unwrap_or_else(|e| panic!("{}", e.render("sel", src)))
    }

    fn shape(expr: &SelectorExpr) -> String {
        match expr {
            SelectorExpr::Or(v) => format!(
                "or({})",
                v.iter().map(|x| shape(&x.node)).collect::<Vec<_>>().join(",")
            ),
            SelectorExpr::And(v) => format!(
                "and({})",
                v.iter().map(|x| shape(&x.node)).collect::<Vec<_>>().join(",")
            ),
            SelectorExpr::Not(v) => format!("not({})", shape(&v.node)),
            SelectorExpr::Chain(chain) => {
                let mut s = format!("chain({})", simple_shape(&chain.first.node.head.node));
                for link in &chain.rest {
                    let c = match link.node.combinator.node {
                        SelectorCombinator::Descendant => "desc",
                        SelectorCombinator::Child => "child",
                        SelectorCombinator::Adjacent => "adj",
                        SelectorCombinator::Sibling => "sib",
                    };
                    s.push_str(&format!(
                        "-{}-{}",
                        c,
                        simple_shape(&link.node.right.node.head.node)
                    ));
                }
                s
            }
        }
    }

    fn simple_shape(v: &SelectorSimple) -> &'static str {
        match v {
            SelectorSimple::Any => "any",
            SelectorSimple::DollarRef(_) => "dollar",
            SelectorSimple::DesignatorPattern { .. } => "designator",
            SelectorSimple::NetPattern(_) => "net",
            SelectorSimple::ValuePattern(_) => "value",
            SelectorSimple::PartPattern(_) => "part",
            SelectorSimple::IdPattern(_) => "id",
            SelectorSimple::ComponentPin { .. } => "comppin",
            SelectorSimple::Type(_) => "type",
        }
    }

    #[test]
    fn parse_logical_and_combinators() {
        let expr = ok("component[designator^=R] AND NOT pin:power OR R*, C*");
        match expr.node {
            SelectorExpr::Or(v) => assert!(v.len() >= 2),
            _ => panic!("expected OR root"),
        }
    }

    #[test]
    fn parse_pseudo_and_component_pin() {
        let a = ok("pin:power");
        let b = ok("U1:VCC");
        assert!(matches!(a.node, SelectorExpr::Chain(_)));
        assert!(matches!(b.node, SelectorExpr::Chain(_)));
    }

    #[test]
    fn parse_attr_modes_regex_dims() {
        let expr = ok("component[name=/^U\\d+$/][x>=10mil]s");
        assert!(matches!(expr.node, SelectorExpr::Chain(_)));
    }

    #[test]
    fn parse_invalid_reports_help() {
        let err = parse_selector("component[foo]", 0).expect_err("should fail");
        let rendered = err.render("s", "component[foo]");
        assert!(rendered.contains("expected attribute operator"));
    }

    proptest! {
        #[test]
        fn prop_selector_parser_never_panics(s in string_regex(r"(?s).{0,180}").expect("regex")) {
            let result = catch_unwind(AssertUnwindSafe(|| parse_selector(&s, 0)));
            prop_assert!(result.is_ok(), "selector parser panicked for {:?}", s);
        }

        #[test]
        fn prop_selector_whitespace_metamorphic(
            lhs in "[A-Za-z_][A-Za-z0-9_]{0,5}",
            rhs in "[A-Za-z_][A-Za-z0-9_]{0,5}"
        ) {
            let a = format!("{lhs}[designator^=R]AND NOT pin:power OR {rhs}*");
            let b = format!("  {lhs}[designator^=R]  AND   NOT   pin:power   OR   {rhs}*  ");
            let pa = ok(&a);
            let pb = ok(&b);
            prop_assert_eq!(shape(&pa.node), shape(&pb.node));
        }

        #[test]
        fn prop_selector_precedence_not_and_or(
            x in "[A-Za-z_][A-Za-z0-9_]{0,4}",
            y in "[A-Za-z_][A-Za-z0-9_]{0,4}",
            z in "[A-Za-z_][A-Za-z0-9_]{0,4}"
        ) {
            let src = format!("{x}* OR {y}* AND NOT {z}*");
            let parsed = ok(&src);
            match parsed.node {
                SelectorExpr::Or(ref terms) => {
                    prop_assert_eq!(terms.len(), 2);
                    prop_assert!(matches!(terms[1].node, SelectorExpr::And(_)));
                }
                _ => prop_assert!(false, "expected OR at root"),
            }
        }
    }
}

use std::str::FromStr;

use crate::diagnostic::{ParseError, ParseErrorCode, Span, Unit};

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplatePart {
    Literal(String),
    Expr(Vec<Token>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Ident(String),
    DollarIdent(String),
    String(String),
    Template(Vec<TemplatePart>),
    Integer(i32),
    Float(f64),
    Dim(f64, Unit),
    Color(u8, u8, u8),

    // Keywords
    Import,
    As,
    Component,
    Footprint,
    Pin,
    Pad,
    Part,
    Parameter,
    Alias,
    Map,
    Row,
    Column,
    Grid,

    // Shared keywords
    Let,
    True,
    False,
    Null,

    // Punctuation
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Colon,
    Comma,
    Dot,
    DotDotDot,
    Eq,
    Plus,
    Minus,
    Star,
    Slash,
    Semi,
    Newline,

    Eof,
}

impl TokenKind {
    pub fn same_variant(&self, other: &TokenKind) -> bool {
        use TokenKind::*;
        matches!(
            (self, other),
            (Ident(_), Ident(_))
                | (DollarIdent(_), DollarIdent(_))
                | (String(_), String(_))
                | (Template(_), Template(_))
                | (Integer(_), Integer(_))
                | (Float(_), Float(_))
                | (Dim(_, _), Dim(_, _))
                | (Color(_, _, _), Color(_, _, _))
                | (Import, Import)
                | (As, As)
                | (Component, Component)
                | (Footprint, Footprint)
                | (Pin, Pin)
                | (Pad, Pad)
                | (Part, Part)
                | (Parameter, Parameter)
                | (Alias, Alias)
                | (Map, Map)
                | (Row, Row)
                | (Column, Column)
                | (Grid, Grid)
                | (Let, Let)
                | (True, True)
                | (False, False)
                | (Null, Null)
                | (LBrace, LBrace)
                | (RBrace, RBrace)
                | (LParen, LParen)
                | (RParen, RParen)
                | (LBracket, LBracket)
                | (RBracket, RBracket)
                | (Colon, Colon)
                | (Comma, Comma)
                | (Dot, Dot)
                | (DotDotDot, DotDotDot)
                | (Eq, Eq)
                | (Plus, Plus)
                | (Minus, Minus)
                | (Star, Star)
                | (Slash, Slash)
                | (Semi, Semi)
                | (Newline, Newline)
                | (Eof, Eof)
        )
    }
}

pub fn lex(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut out = Vec::new();
    let mut i = 0usize;
    let bytes = input.as_bytes();

    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b' ' | b'\t' | b'\r' => {
                i += 1;
            }
            b'\n' => {
                out.push(tok(TokenKind::Newline, i, i + 1));
                i += 1;
            }
            b'/' if peek_byte(bytes, i + 1) == Some(b'/') => {
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if peek_byte(bytes, i + 1) == Some(b'*') => {
                let start = i;
                i += 2;
                let mut depth = 1u32;
                while i < bytes.len() {
                    if bytes[i] == b'/' && peek_byte(bytes, i + 1) == Some(b'*') {
                        depth += 1;
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'*' && peek_byte(bytes, i + 1) == Some(b'/') {
                        depth -= 1;
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                        continue;
                    }
                    i += 1;
                }
                if depth != 0 {
                    return Err(ParseError::new(
                        ParseErrorCode::E1001,
                        "unterminated block comment",
                        Span::new(start as u32, input.len() as u32),
                    ));
                }
            }
            b'(' => {
                out.push(tok(TokenKind::LParen, i, i + 1));
                i += 1;
            }
            b')' => {
                out.push(tok(TokenKind::RParen, i, i + 1));
                i += 1;
            }
            b'[' => {
                out.push(tok(TokenKind::LBracket, i, i + 1));
                i += 1;
            }
            b']' => {
                out.push(tok(TokenKind::RBracket, i, i + 1));
                i += 1;
            }
            b'{' => {
                out.push(tok(TokenKind::LBrace, i, i + 1));
                i += 1;
            }
            b'}' => {
                out.push(tok(TokenKind::RBrace, i, i + 1));
                i += 1;
            }
            b':' => {
                out.push(tok(TokenKind::Colon, i, i + 1));
                i += 1;
            }
            b',' => {
                out.push(tok(TokenKind::Comma, i, i + 1));
                i += 1;
            }
            b'.' if peek_byte(bytes, i + 1) == Some(b'.')
                && peek_byte(bytes, i + 2) == Some(b'.') =>
            {
                out.push(tok(TokenKind::DotDotDot, i, i + 3));
                i += 3;
            }
            b'.' => {
                out.push(tok(TokenKind::Dot, i, i + 1));
                i += 1;
            }
            b'=' => {
                out.push(tok(TokenKind::Eq, i, i + 1));
                i += 1;
            }
            b'+' => {
                out.push(tok(TokenKind::Plus, i, i + 1));
                i += 1;
            }
            b'-' => {
                out.push(tok(TokenKind::Minus, i, i + 1));
                i += 1;
            }
            b'*' => {
                out.push(tok(TokenKind::Star, i, i + 1));
                i += 1;
            }
            b'/' => {
                out.push(tok(TokenKind::Slash, i, i + 1));
                i += 1;
            }
            b';' => {
                out.push(tok(TokenKind::Semi, i, i + 1));
                i += 1;
            }
            b'$' => {
                let start = i;
                i += 1;
                let Some(ch) = input[i..].chars().next() else {
                    return Err(ParseError::new(
                        ParseErrorCode::E1001,
                        "expected identifier after '$'",
                        Span::new(start as u32, (start + 1) as u32),
                    ));
                };
                if !is_ident_start(ch) {
                    return Err(ParseError::new(
                        ParseErrorCode::E1001,
                        "expected identifier after '$'",
                        Span::new(start as u32, (start + 1) as u32),
                    ));
                }
                let (ident, end) = read_ident(input, i);
                i = end;
                out.push(tok(TokenKind::DollarIdent(ident.to_string()), start, i));
            }
            b'"' => {
                let start = i;
                let (value, end) = read_quoted_string(input, i)?;
                i = end;
                out.push(tok(TokenKind::String(value), start, end));
            }
            b'`' => {
                let start = i;
                let (parts, end) = read_template(input, i)?;
                i = end;
                out.push(tok(TokenKind::Template(parts), start, end));
            }
            b'#' => {
                let start = i;
                let end = i + 7;
                if end <= bytes.len() {
                    let lit_bytes = &bytes[i + 1..end];
                    if lit_bytes.iter().all(|b| b.is_ascii_hexdigit()) {
                        if let Ok(lit) = std::str::from_utf8(lit_bytes) {
                            let r = u8::from_str_radix(&lit[0..2], 16).unwrap_or(0);
                            let g = u8::from_str_radix(&lit[2..4], 16).unwrap_or(0);
                            let b = u8::from_str_radix(&lit[4..6], 16).unwrap_or(0);
                            out.push(tok(TokenKind::Color(r, g, b), start, end));
                            i = end;
                            continue;
                        }
                    }
                }
                return Err(ParseError::new(
                    ParseErrorCode::E1001,
                    "expected 6 hex digits after '#' for color literal",
                    Span::new(start as u32, (start + 1) as u32),
                )
                .with_help("example: #FF0000 for red"));
            }
            _ if is_digit(b as char) => {
                let start = i;
                let (kind, end) = read_number_or_dim(input, i)?;
                i = end;
                out.push(tok(kind, start, end));
            }
            _ => {
                let ch = input[i..].chars().next().expect("valid utf-8 boundary");
                if is_ident_start(ch) {
                    let start = i;
                    let (ident, end) = read_ident(input, i);
                    i = end;
                    let kind = match ident {
                        "import" => TokenKind::Import,
                        "as" => TokenKind::As,
                        "component" => TokenKind::Component,
                        "footprint" => TokenKind::Footprint,
                        "pin" => TokenKind::Pin,
                        "pad" => TokenKind::Pad,
                        "part" => TokenKind::Part,
                        "parameter" => TokenKind::Parameter,
                        "alias" => TokenKind::Alias,
                        "map" => TokenKind::Map,
                        "row" => TokenKind::Row,
                        "column" => TokenKind::Column,
                        "grid" => TokenKind::Grid,
                        "let" => TokenKind::Let,
                        "true" => TokenKind::True,
                        "false" => TokenKind::False,
                        "null" => TokenKind::Null,
                        _ => TokenKind::Ident(ident.to_string()),
                    };
                    out.push(tok(kind, start, end));
                } else {
                    return Err(ParseError::new(
                        ParseErrorCode::E1001,
                        format!("unexpected character '{}'", ch),
                        Span::new(i as u32, (i + ch.len_utf8()) as u32),
                    ));
                }
            }
        }
    }

    out.push(tok(TokenKind::Eof, input.len(), input.len()));
    Ok(out)
}

fn tok(kind: TokenKind, start: usize, end: usize) -> Token {
    Token {
        kind,
        span: Span::new(start as u32, end as u32),
    }
}

fn peek_byte(bytes: &[u8], idx: usize) -> Option<u8> {
    bytes.get(idx).copied()
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn is_digit(ch: char) -> bool {
    ch.is_ascii_digit()
}

fn read_ident(input: &str, start: usize) -> (&str, usize) {
    for (offset, ch) in input[start..].char_indices() {
        if !is_ident_continue(ch) {
            let end = start + offset;
            return (&input[start..end], end);
        }
    }
    (&input[start..], input.len())
}

fn read_quoted_string(input: &str, start_quote: usize) -> Result<(String, usize), ParseError> {
    let mut i = start_quote + 1;
    let mut out = String::new();
    while i < input.len() {
        let ch = input[i..].chars().next().expect("char boundary");
        match ch {
            '"' => return Ok((out, i + 1)),
            '\\' => {
                let esc_start = i;
                i += 1;
                let Some(esc) = input[i..].chars().next() else {
                    return Err(ParseError::new(
                        ParseErrorCode::E1001,
                        "unterminated escape sequence",
                        Span::new(esc_start as u32, input.len() as u32),
                    ));
                };
                let mapped = match esc {
                    '"' => '"',
                    '\\' => '\\',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    _ => {
                        return Err(ParseError::new(
                            ParseErrorCode::E1001,
                            format!("invalid escape sequence \\{}", esc),
                            Span::new(esc_start as u32, (i + esc.len_utf8()) as u32),
                        )
                        .with_help("valid escapes: \\\\, \\\", \\n, \\r, \\t"));
                    }
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
        ParseErrorCode::E1001,
        "unterminated string literal",
        Span::new(start_quote as u32, input.len() as u32),
    ))
}

fn read_template(input: &str, start_tick: usize) -> Result<(Vec<TemplatePart>, usize), ParseError> {
    let mut i = start_tick + 1;
    let mut parts = Vec::new();
    let mut literal = String::new();

    while i < input.len() {
        let ch = input[i..].chars().next().expect("char boundary");
        match ch {
            '`' => {
                if !literal.is_empty() {
                    parts.push(TemplatePart::Literal(std::mem::take(&mut literal)));
                }
                return Ok((parts, i + 1));
            }
            '\\' => {
                let esc_start = i;
                i += 1;
                let Some(esc) = input[i..].chars().next() else {
                    return Err(ParseError::new(
                        ParseErrorCode::E1001,
                        "unterminated escape sequence in template string",
                        Span::new(esc_start as u32, input.len() as u32),
                    ));
                };
                let mapped = match esc {
                    '`' => '`',
                    '\\' => '\\',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '{' => '{',
                    '}' => '}',
                    _ => {
                        return Err(ParseError::new(
                            ParseErrorCode::E1001,
                            format!("invalid escape sequence \\{}", esc),
                            Span::new(esc_start as u32, (i + esc.len_utf8()) as u32),
                        )
                        .with_help("valid escapes in templates: \\\\, \\`, \\n, \\r, \\t, \\{, \\}"));
                    }
                };
                literal.push(mapped);
                i += esc.len_utf8();
            }
            '{' if peek_byte(input.as_bytes(), i + 1) == Some(b'{') => {
                literal.push('{');
                i += 2;
            }
            '}' if peek_byte(input.as_bytes(), i + 1) == Some(b'}') => {
                literal.push('}');
                i += 2;
            }
            '{' => {
                if !literal.is_empty() {
                    parts.push(TemplatePart::Literal(std::mem::take(&mut literal)));
                }
                i += 1;
                let (expr_tokens, new_i) = read_template_expr(input, i, start_tick)?;
                i = new_i;
                parts.push(TemplatePart::Expr(expr_tokens));
            }
            _ => {
                literal.push(ch);
                i += ch.len_utf8();
            }
        }
    }

    Err(ParseError::new(
        ParseErrorCode::E1001,
        "unterminated template string",
        Span::new(start_tick as u32, input.len() as u32),
    ))
}

fn read_template_expr(
    input: &str,
    start: usize,
    template_start: usize,
) -> Result<(Vec<Token>, usize), ParseError> {
    let mut i = start;
    let mut depth = 1usize;
    let expr_start = i;

    while i < input.len() {
        let b = input.as_bytes()[i];
        match b {
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    let expr_src = &input[expr_start..i];
                    let tokens = lex(expr_src).map_err(|e| {
                        ParseError::new(
                            ParseErrorCode::E1001,
                            format!("error in template interpolation: {}", e.message),
                            Span::new(
                                (expr_start as u32) + e.span.start,
                                (expr_start as u32) + e.span.end,
                            ),
                        )
                    })?;
                    let tokens: Vec<Token> = tokens
                        .into_iter()
                        .filter(|t| !matches!(t.kind, TokenKind::Eof))
                        .map(|t| Token {
                            kind: t.kind,
                            span: Span::new(
                                t.span.start + expr_start as u32,
                                t.span.end + expr_start as u32,
                            ),
                        })
                        .collect();
                    return Ok((tokens, i + 1));
                }
                i += 1;
            }
            b'"' => {
                i += 1;
                while i < input.len() {
                    let ch = input[i..].chars().next().unwrap();
                    if ch == '"' {
                        i += 1;
                        break;
                    }
                    if ch == '\\' {
                        i += 2;
                    } else {
                        i += ch.len_utf8();
                    }
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    Err(ParseError::new(
        ParseErrorCode::E1001,
        "unterminated interpolation in template string",
        Span::new(template_start as u32, input.len() as u32),
    ))
}

fn read_number_or_dim(input: &str, start: usize) -> Result<(TokenKind, usize), ParseError> {
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

    let num_str = &input[start..end];
    if end < input.len() {
        let next = input[end..].chars().next().expect("utf-8 boundary");
        if is_ident_start(next) {
            let (unit_str, unit_end) = read_ident(input, end);
            if let Some(unit) = parse_unit(unit_str) {
                let value = f64::from_str(num_str).map_err(|_| {
                    ParseError::new(
                        ParseErrorCode::E1001,
                        "invalid numeric literal",
                        Span::new(start as u32, end as u32),
                    )
                })?;
                return Ok((TokenKind::Dim(value, unit), unit_end));
            }
            return Err(ParseError::new(
                ParseErrorCode::E1003,
                format!("unknown unit suffix '{}'", unit_str),
                Span::new(end as u32, unit_end as u32),
            )
            .with_help("valid units: mm, mil, in, dxp, raw")
            .with_note(format!("numeric literal: {}", num_str)));
        }
    }

    if is_float {
        let value = f64::from_str(num_str).map_err(|_| {
            ParseError::new(
                ParseErrorCode::E1001,
                "invalid float literal",
                Span::new(start as u32, end as u32),
            )
        })?;
        return Ok((TokenKind::Float(value), end));
    }

    let value = i32::from_str(num_str).map_err(|_| {
        ParseError::new(
            ParseErrorCode::E1001,
            "invalid integer literal",
            Span::new(start as u32, end as u32),
        )
    })?;
    Ok((TokenKind::Integer(value), end))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Unit;

    fn lex_kinds(input: &str) -> Vec<TokenKind> {
        lex(input)
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    fn lex_ok(input: &str) -> Vec<Token> {
        lex(input).unwrap()
    }

    #[test]
    fn test_basic_keywords() {
        let kinds = lex_kinds("import as component footprint pin pad part parameter alias map row column grid let true false null");
        assert_eq!(kinds, vec![
            TokenKind::Import,
            TokenKind::As,
            TokenKind::Component,
            TokenKind::Footprint,
            TokenKind::Pin,
            TokenKind::Pad,
            TokenKind::Part,
            TokenKind::Parameter,
            TokenKind::Alias,
            TokenKind::Map,
            TokenKind::Row,
            TokenKind::Column,
            TokenKind::Grid,
            TokenKind::Let,
            TokenKind::True,
            TokenKind::False,
            TokenKind::Null,
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_graphic_types_are_idents() {
        // Graphic types are NOT keywords — they lex as Ident
        let kinds = lex_kinds("line rectangle arc elliptical_arc ellipse polyline polygon bezier pie round_rectangle label text_frame image track fill region text via component_body");
        for kind in &kinds[..kinds.len() - 1] {
            assert!(matches!(kind, TokenKind::Ident(_)), "expected Ident, got {:?}", kind);
        }
    }

    #[test]
    fn test_integer() {
        let kinds = lex_kinds("42 0 100");
        assert_eq!(kinds, vec![
            TokenKind::Integer(42),
            TokenKind::Integer(0),
            TokenKind::Integer(100),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_float() {
        let kinds = lex_kinds("3.14 0.5");
        assert_eq!(kinds, vec![
            TokenKind::Float(3.14),
            TokenKind::Float(0.5),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_dim_tokens() {
        let kinds = lex_kinds("20mm 100mil 2.54mm 1in 50dxp 100raw");
        assert_eq!(kinds, vec![
            TokenKind::Dim(20.0, Unit::Mm),
            TokenKind::Dim(100.0, Unit::Mil),
            TokenKind::Dim(2.54, Unit::Mm),
            TokenKind::Dim(1.0, Unit::Inch),
            TokenKind::Dim(50.0, Unit::Dxp),
            TokenKind::Dim(100.0, Unit::Raw),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_dim_disambiguation_space() {
        // "20 mm" should be Integer + Ident, not Dim
        let kinds = lex_kinds("20 mm");
        assert_eq!(kinds, vec![
            TokenKind::Integer(20),
            TokenKind::Ident("mm".to_string()),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_unknown_unit_suffix() {
        assert!(lex("20xyz").is_err());
    }

    #[test]
    fn test_color() {
        let kinds = lex_kinds("#FF0000 #00ff00 #aAbBcC");
        assert_eq!(kinds, vec![
            TokenKind::Color(0xFF, 0x00, 0x00),
            TokenKind::Color(0x00, 0xFF, 0x00),
            TokenKind::Color(0xAA, 0xBB, 0xCC),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_invalid_color() {
        assert!(lex("#FFGG00").is_err());
        assert!(lex("#FFF").is_err());
    }

    #[test]
    fn test_dollar_ident() {
        let kinds = lex_kinds("$body $fp $p2");
        assert_eq!(kinds, vec![
            TokenKind::DollarIdent("body".to_string()),
            TokenKind::DollarIdent("fp".to_string()),
            TokenKind::DollarIdent("p2".to_string()),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_dollar_no_ident_error() {
        assert!(lex("$").is_err());
        assert!(lex("$ foo").is_err());
    }

    #[test]
    fn test_string() {
        let kinds = lex_kinds(r#""hello" "world""#);
        assert_eq!(kinds, vec![
            TokenKind::String("hello".to_string()),
            TokenKind::String("world".to_string()),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_string_escapes() {
        let kinds = lex_kinds(r#""\n\t\r\"\\""#);
        assert_eq!(kinds, vec![
            TokenKind::String("\n\t\r\"\\".to_string()),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_string_unterminated() {
        assert!(lex(r#""hello"#).is_err());
    }

    #[test]
    fn test_template_literal_only() {
        let tokens = lex_ok("`hello world`");
        assert_eq!(tokens.len(), 2); // template + eof
        if let TokenKind::Template(parts) = &tokens[0].kind {
            assert_eq!(parts.len(), 1);
            assert!(matches!(&parts[0], TemplatePart::Literal(s) if s == "hello world"));
        } else {
            panic!("expected Template token");
        }
    }

    #[test]
    fn test_template_with_expr() {
        let tokens = lex_ok("`prefix {$body.width} suffix`");
        if let TokenKind::Template(parts) = &tokens[0].kind {
            assert_eq!(parts.len(), 3);
            assert!(matches!(&parts[0], TemplatePart::Literal(s) if s == "prefix "));
            assert!(matches!(&parts[1], TemplatePart::Expr(_)));
            assert!(matches!(&parts[2], TemplatePart::Literal(s) if s == " suffix"));
        } else {
            panic!("expected Template token");
        }
    }

    #[test]
    fn test_template_double_brace_escape() {
        let tokens = lex_ok("`{{literal}}`");
        if let TokenKind::Template(parts) = &tokens[0].kind {
            assert_eq!(parts.len(), 1);
            assert!(matches!(&parts[0], TemplatePart::Literal(s) if s == "{literal}"));
        } else {
            panic!("expected Template token");
        }
    }

    #[test]
    fn test_template_unterminated() {
        assert!(lex("`hello").is_err());
    }

    #[test]
    fn test_newline_emitted() {
        let kinds = lex_kinds("a\nb");
        assert_eq!(kinds, vec![
            TokenKind::Ident("a".to_string()),
            TokenKind::Newline,
            TokenKind::Ident("b".to_string()),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_line_comment_consumed() {
        let kinds = lex_kinds("a // this is a comment\nb");
        assert_eq!(kinds, vec![
            TokenKind::Ident("a".to_string()),
            TokenKind::Newline,
            TokenKind::Ident("b".to_string()),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_block_comment_consumed() {
        let kinds = lex_kinds("a /* block comment */ b");
        assert_eq!(kinds, vec![
            TokenKind::Ident("a".to_string()),
            TokenKind::Ident("b".to_string()),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_nested_block_comment() {
        let kinds = lex_kinds("a /* outer /* inner */ outer */ b");
        assert_eq!(kinds, vec![
            TokenKind::Ident("a".to_string()),
            TokenKind::Ident("b".to_string()),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_unterminated_block_comment() {
        assert!(lex("/* not closed").is_err());
    }

    #[test]
    fn test_dotdotdot() {
        let kinds = lex_kinds("...smd");
        assert_eq!(kinds, vec![
            TokenKind::DotDotDot,
            TokenKind::Ident("smd".to_string()),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_punctuation() {
        let kinds = lex_kinds("{ } ( ) [ ] : , . = + - * /");
        assert_eq!(kinds, vec![
            TokenKind::LBrace,
            TokenKind::RBrace,
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LBracket,
            TokenKind::RBracket,
            TokenKind::Colon,
            TokenKind::Comma,
            TokenKind::Dot,
            TokenKind::Eq,
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash,
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_semicolon() {
        let kinds = lex_kinds("a; b");
        assert_eq!(kinds, vec![
            TokenKind::Ident("a".to_string()),
            TokenKind::Semi,
            TokenKind::Ident("b".to_string()),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_span_correctness() {
        let tokens = lex_ok("hello 42");
        assert_eq!(tokens[0].span.start, 0);
        assert_eq!(tokens[0].span.end, 5);
        assert_eq!(tokens[1].span.start, 6);
        assert_eq!(tokens[1].span.end, 8);
    }

    #[test]
    fn test_unexpected_char() {
        assert!(lex("@").is_err());
    }
}

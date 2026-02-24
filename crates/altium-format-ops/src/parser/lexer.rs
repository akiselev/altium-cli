use std::str::FromStr;

use super::ast::{Span, Unit};
use super::diagnostic::{ParseError, ParseErrorCode};

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident(String),
    DollarIdent(String),
    String(String),
    Template(String),
    Integer(i32),
    Float(f64),
    Dim(f64, Unit),
    Color(u8, u8, u8),

    Assert,
    Let,
    True,
    False,
    Null,

    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Colon,
    Comma,
    Dot,
    DotDotDot,
    Caret,
    Tilde,
    At,
    Percent,
    Hash,
    Question,
    Dollar,
    Eq,
    EqEq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
    Plus,
    Minus,
    Star,
    Slash,
    Semicolon,
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
                | (Assert, Assert)
                | (Let, Let)
                | (True, True)
                | (False, False)
                | (Null, Null)
                | (LParen, LParen)
                | (RParen, RParen)
                | (LBracket, LBracket)
                | (RBracket, RBracket)
                | (LBrace, LBrace)
                | (RBrace, RBrace)
                | (Colon, Colon)
                | (Comma, Comma)
                | (Dot, Dot)
                | (DotDotDot, DotDotDot)
                | (Caret, Caret)
                | (Tilde, Tilde)
                | (At, At)
                | (Percent, Percent)
                | (Hash, Hash)
                | (Question, Question)
                | (Dollar, Dollar)
                | (Eq, Eq)
                | (EqEq, EqEq)
                | (Ne, Ne)
                | (Gt, Gt)
                | (Lt, Lt)
                | (Ge, Ge)
                | (Le, Le)
                | (Plus, Plus)
                | (Minus, Minus)
                | (Star, Star)
                | (Slash, Slash)
                | (Semicolon, Semicolon)
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
            b'^' => {
                out.push(tok(TokenKind::Caret, i, i + 1));
                i += 1;
            }
            b'~' => {
                out.push(tok(TokenKind::Tilde, i, i + 1));
                i += 1;
            }
            b'@' => {
                out.push(tok(TokenKind::At, i, i + 1));
                i += 1;
            }
            b'%' => {
                out.push(tok(TokenKind::Percent, i, i + 1));
                i += 1;
            }
            b'?' => {
                out.push(tok(TokenKind::Question, i, i + 1));
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
            b'=' if peek_byte(bytes, i + 1) == Some(b'=') => {
                out.push(tok(TokenKind::EqEq, i, i + 2));
                i += 2;
            }
            b'=' => {
                out.push(tok(TokenKind::Eq, i, i + 1));
                i += 1;
            }
            b'!' if peek_byte(bytes, i + 1) == Some(b'=') => {
                out.push(tok(TokenKind::Ne, i, i + 2));
                i += 2;
            }
            b'>' if peek_byte(bytes, i + 1) == Some(b'=') => {
                out.push(tok(TokenKind::Ge, i, i + 2));
                i += 2;
            }
            b'>' => {
                out.push(tok(TokenKind::Gt, i, i + 1));
                i += 1;
            }
            b'<' if peek_byte(bytes, i + 1) == Some(b'=') => {
                out.push(tok(TokenKind::Le, i, i + 2));
                i += 2;
            }
            b'<' => {
                out.push(tok(TokenKind::Lt, i, i + 1));
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
                out.push(tok(TokenKind::Semicolon, i, i + 1));
                i += 1;
            }
            b'$' if peek_byte(bytes, i + 1) == Some(b'=') => {
                out.push(tok(TokenKind::Dollar, i, i + 1));
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
                let (value, end) = read_template(input, i)?;
                i = end;
                out.push(tok(TokenKind::Template(value), start, end));
            }
            b'#' => {
                let end = i + 7;
                if end <= bytes.len() {
                    let lit_bytes = &bytes[i + 1..end];
                    if lit_bytes.iter().all(|b| b.is_ascii_hexdigit()) {
                        if let Ok(lit) = std::str::from_utf8(lit_bytes) {
                            let r = u8::from_str_radix(&lit[0..2], 16).unwrap_or(0);
                            let g = u8::from_str_radix(&lit[2..4], 16).unwrap_or(0);
                            let b = u8::from_str_radix(&lit[4..6], 16).unwrap_or(0);
                            out.push(tok(TokenKind::Color(r, g, b), i, end));
                            i = end;
                            continue;
                        }
                    }
                }
                out.push(tok(TokenKind::Hash, i, i + 1));
                i += 1;
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
                        "assert" => TokenKind::Assert,
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
                i += ch.len_utf8();
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
                        .with_help("valid escapes: \\, \", \\n, \\r, \\t"));
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

fn read_template(input: &str, start_tick: usize) -> Result<(String, usize), ParseError> {
    let mut i = start_tick + 1;
    let mut out = String::new();
    while i < input.len() {
        let ch = input[i..].chars().next().expect("char boundary");
        match ch {
            '`' => return Ok((out, i + 1)),
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
                out.push('\\');
                out.push(esc);
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
        "unterminated template string",
        Span::new(start_tick as u32, input.len() as u32),
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

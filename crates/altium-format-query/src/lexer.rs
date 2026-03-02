use crate::diagnostic::{Span, Unit};
use crate::error::{QueryError, QueryErrorCode};

/// Token kind produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Ident(String),
    String(String),
    Integer(i64),
    Float(f64),
    Dim(f64, Unit),
    Regex(String),
    Bool(bool),

    // Pattern prefixes (value follows as next token or is part of the ident)
    Dollar,  // $
    At,      // @
    Tilde,   // ~
    Percent, // %
    Hash,    // #

    // Wildcards
    Star,      // *
    Question,  // ?

    // Comparison operators
    Eq,           // =
    NotEq,        // !=
    Contains,     // *=
    StartsWith,   // ^=
    EndsWith,     // $=
    WordMatch,    // ~=
    Gt,           // > (inside [...])
    Lt,           // <
    Gte,          // >=
    Lte,          // <=

    // Structural
    LBracket,     // [
    RBracket,     // ]
    LParen,       // (
    RParen,       // )
    Colon,        // :
    Comma,        // ,
    Dot,          // .

    // Combinators
    ChildCombinator,     // > (outside [...])

    // Keywords
    And,
    Or,
    Not,
}

impl TokenKind {
    /// Returns true if both tokens are the same variant (ignoring payload).
    pub fn same_variant(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

/// A lexer token with its kind and source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// Tokenize a query string into a sequence of tokens.
///
/// The lexer is context-sensitive for `>`: outside `[...]` brackets it produces
/// `ChildCombinator`, inside brackets it produces `Gt` (greater-than operator).
pub fn lex(input: &str) -> Result<Vec<Token>, QueryError> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0usize;
    let mut bracket_depth: u32 = 0;

    while i < bytes.len() {
        // Skip whitespace (but don't skip it — whitespace is significant for
        // descendant combinator detection, which the parser handles)
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }

        let start = i;

        match bytes[i] {
            // Brackets
            b'[' => {
                bracket_depth += 1;
                tokens.push(Token {
                    kind: TokenKind::LBracket,
                    span: Span::new(start as u32, (i + 1) as u32),
                });
                i += 1;
            }
            b']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                tokens.push(Token {
                    kind: TokenKind::RBracket,
                    span: Span::new(start as u32, (i + 1) as u32),
                });
                i += 1;
            }

            // Parens
            b'(' => {
                tokens.push(Token {
                    kind: TokenKind::LParen,
                    span: Span::new(start as u32, (i + 1) as u32),
                });
                i += 1;
            }
            b')' => {
                tokens.push(Token {
                    kind: TokenKind::RParen,
                    span: Span::new(start as u32, (i + 1) as u32),
                });
                i += 1;
            }

            // Colon, comma, dot
            b':' => {
                tokens.push(Token {
                    kind: TokenKind::Colon,
                    span: Span::new(start as u32, (i + 1) as u32),
                });
                i += 1;
            }
            b',' => {
                tokens.push(Token {
                    kind: TokenKind::Comma,
                    span: Span::new(start as u32, (i + 1) as u32),
                });
                i += 1;
            }
            b'.' => {
                tokens.push(Token {
                    kind: TokenKind::Dot,
                    span: Span::new(start as u32, (i + 1) as u32),
                });
                i += 1;
            }

            // Pattern prefixes
            b'$' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    // $= ends-with operator
                    tokens.push(Token {
                        kind: TokenKind::EndsWith,
                        span: Span::new(start as u32, (i + 2) as u32),
                    });
                    i += 2;
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Dollar,
                        span: Span::new(start as u32, (i + 1) as u32),
                    });
                    i += 1;
                }
            }
            b'@' => {
                tokens.push(Token {
                    kind: TokenKind::At,
                    span: Span::new(start as u32, (i + 1) as u32),
                });
                i += 1;
            }
            b'#' => {
                tokens.push(Token {
                    kind: TokenKind::Hash,
                    span: Span::new(start as u32, (i + 1) as u32),
                });
                i += 1;
            }

            // Tilde: ~= (word match) or ~ (standalone, currently unused as prefix)
            b'~' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    tokens.push(Token {
                        kind: TokenKind::WordMatch,
                        span: Span::new(start as u32, (i + 2) as u32),
                    });
                    i += 2;
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Tilde,
                        span: Span::new(start as u32, (i + 1) as u32),
                    });
                    i += 1;
                }
            }

            // Percent: % (net name prefix)
            b'%' => {
                tokens.push(Token {
                    kind: TokenKind::Percent,
                    span: Span::new(start as u32, (i + 1) as u32),
                });
                i += 1;
            }

            // Star: *= (contains) or * (wildcard)
            b'*' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    tokens.push(Token {
                        kind: TokenKind::Contains,
                        span: Span::new(start as u32, (i + 2) as u32),
                    });
                    i += 2;
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Star,
                        span: Span::new(start as u32, (i + 1) as u32),
                    });
                    i += 1;
                }
            }

            // Question mark (wildcard)
            b'?' => {
                tokens.push(Token {
                    kind: TokenKind::Question,
                    span: Span::new(start as u32, (i + 1) as u32),
                });
                i += 1;
            }

            // Operators starting with !
            b'!' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    tokens.push(Token {
                        kind: TokenKind::NotEq,
                        span: Span::new(start as u32, (i + 2) as u32),
                    });
                    i += 2;
                } else {
                    return Err(QueryError::new(
                        QueryErrorCode::UnexpectedCharacter,
                        "expected '!=' after '!'",
                    )
                    .with_span(Span::new(start as u32, (i + 1) as u32)));
                }
            }

            // = (exact match)
            b'=' => {
                tokens.push(Token {
                    kind: TokenKind::Eq,
                    span: Span::new(start as u32, (i + 1) as u32),
                });
                i += 1;
            }

            // ^ : ^= (starts-with)
            b'^' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    tokens.push(Token {
                        kind: TokenKind::StartsWith,
                        span: Span::new(start as u32, (i + 2) as u32),
                    });
                    i += 2;
                } else {
                    return Err(QueryError::new(
                        QueryErrorCode::UnexpectedCharacter,
                        "expected '^=' after '^'",
                    )
                    .with_span(Span::new(start as u32, (i + 1) as u32)));
                }
            }

            // > : >= or > (context-sensitive: child combinator vs greater-than)
            b'>' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    tokens.push(Token {
                        kind: TokenKind::Gte,
                        span: Span::new(start as u32, (i + 2) as u32),
                    });
                    i += 2;
                } else if bracket_depth > 0 {
                    tokens.push(Token {
                        kind: TokenKind::Gt,
                        span: Span::new(start as u32, (i + 1) as u32),
                    });
                    i += 1;
                } else {
                    tokens.push(Token {
                        kind: TokenKind::ChildCombinator,
                        span: Span::new(start as u32, (i + 1) as u32),
                    });
                    i += 1;
                }
            }

            // < : <= or <
            b'<' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    tokens.push(Token {
                        kind: TokenKind::Lte,
                        span: Span::new(start as u32, (i + 2) as u32),
                    });
                    i += 2;
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Lt,
                        span: Span::new(start as u32, (i + 1) as u32),
                    });
                    i += 1;
                }
            }

            // Double-quoted string
            b'"' => {
                i += 1; // skip opening quote
                let mut s = String::new();
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        match bytes[i + 1] {
                            b'"' => {
                                s.push('"');
                                i += 2;
                            }
                            b'\\' => {
                                s.push('\\');
                                i += 2;
                            }
                            b'n' => {
                                s.push('\n');
                                i += 2;
                            }
                            _ => {
                                s.push(bytes[i] as char);
                                i += 1;
                            }
                        }
                    } else {
                        s.push(bytes[i] as char);
                        i += 1;
                    }
                }
                if i >= bytes.len() {
                    return Err(QueryError::new(
                        QueryErrorCode::UnterminatedString,
                        "unterminated string literal",
                    )
                    .with_span(Span::new(start as u32, i as u32)));
                }
                i += 1; // skip closing quote
                tokens.push(Token {
                    kind: TokenKind::String(s),
                    span: Span::new(start as u32, i as u32),
                });
            }

            // Regex literal: /pattern/
            b'/' => {
                i += 1; // skip opening /
                let mut pattern = String::new();
                while i < bytes.len() && bytes[i] != b'/' {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        // Escaped character in regex
                        pattern.push(bytes[i] as char);
                        pattern.push(bytes[i + 1] as char);
                        i += 2;
                    } else {
                        pattern.push(bytes[i] as char);
                        i += 1;
                    }
                }
                if i >= bytes.len() {
                    return Err(QueryError::new(
                        QueryErrorCode::UnterminatedRegex,
                        "unterminated regex literal",
                    )
                    .with_span(Span::new(start as u32, i as u32)));
                }
                i += 1; // skip closing /
                tokens.push(Token {
                    kind: TokenKind::Regex(pattern),
                    span: Span::new(start as u32, i as u32),
                });
            }

            // Numbers (integers, floats, dimensional values)
            b'0'..=b'9' | b'-' if {
                // Negative numbers: '-' followed by digit
                bytes[i] == b'-' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit()
            } || bytes[i].is_ascii_digit() =>
            {
                let num_start = i;
                if bytes[i] == b'-' {
                    i += 1;
                }
                // Integer part
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                let mut is_float = false;
                // Fractional part
                if i < bytes.len() && bytes[i] == b'.' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
                    is_float = true;
                    i += 1; // skip '.'
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                let num_str = &input[num_start..i];

                // Check for known unit suffix (mil, mm, in).
                // If the suffix is NOT a known unit, backtrack and emit just the number,
                // letting the alphabetic characters be tokenized as an identifier.
                if i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                    let unit_start = i;
                    let saved_i = i;
                    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    let unit_str = &input[unit_start..i];
                    match unit_str {
                        "mil" | "mm" | "in" => {
                            let unit = match unit_str {
                                "mil" => Unit::Mil,
                                "mm" => Unit::Mm,
                                "in" => Unit::Inch,
                                _ => unreachable!(),
                            };
                            let value: f64 = num_str.parse().map_err(|_| {
                                QueryError::new(
                                    QueryErrorCode::InvalidNumber,
                                    format!("invalid number '{num_str}'"),
                                )
                                .with_span(Span::new(num_start as u32, i as u32))
                            })?;
                            tokens.push(Token {
                                kind: TokenKind::Dim(value, unit),
                                span: Span::new(start as u32, i as u32),
                            });
                        }
                        _ => {
                            // Not a known unit — backtrack and emit just the number
                            i = saved_i;
                            if is_float {
                                let value: f64 = num_str.parse().map_err(|_| {
                                    QueryError::new(
                                        QueryErrorCode::InvalidNumber,
                                        format!("invalid float '{num_str}'"),
                                    )
                                    .with_span(Span::new(num_start as u32, i as u32))
                                })?;
                                tokens.push(Token {
                                    kind: TokenKind::Float(value),
                                    span: Span::new(start as u32, i as u32),
                                });
                            } else {
                                let value: i64 = num_str.parse().map_err(|_| {
                                    QueryError::new(
                                        QueryErrorCode::InvalidNumber,
                                        format!("invalid integer '{num_str}'"),
                                    )
                                    .with_span(Span::new(num_start as u32, i as u32))
                                })?;
                                tokens.push(Token {
                                    kind: TokenKind::Integer(value),
                                    span: Span::new(start as u32, i as u32),
                                });
                            }
                        }
                    }
                } else if is_float {
                    let value: f64 = num_str.parse().map_err(|_| {
                        QueryError::new(
                            QueryErrorCode::InvalidNumber,
                            format!("invalid float '{num_str}'"),
                        )
                        .with_span(Span::new(num_start as u32, i as u32))
                    })?;
                    tokens.push(Token {
                        kind: TokenKind::Float(value),
                        span: Span::new(start as u32, i as u32),
                    });
                } else {
                    let value: i64 = num_str.parse().map_err(|_| {
                        QueryError::new(
                            QueryErrorCode::InvalidNumber,
                            format!("invalid integer '{num_str}'"),
                        )
                        .with_span(Span::new(num_start as u32, i as u32))
                    })?;
                    tokens.push(Token {
                        kind: TokenKind::Integer(value),
                        span: Span::new(start as u32, i as u32),
                    });
                }
            }

            // Identifiers and keywords
            c if c.is_ascii_alphabetic() || c == b'_' => {
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-')
                {
                    i += 1;
                }
                let word = &input[start..i];
                let kind = match word.to_ascii_lowercase().as_str() {
                    "and" => TokenKind::And,
                    "or" => TokenKind::Or,
                    "not" => TokenKind::Not,
                    "true" => TokenKind::Bool(true),
                    "false" => TokenKind::Bool(false),
                    _ => TokenKind::Ident(word.to_string()),
                };
                tokens.push(Token {
                    kind,
                    span: Span::new(start as u32, i as u32),
                });
            }

            _ => {
                return Err(QueryError::new(
                    QueryErrorCode::UnexpectedCharacter,
                    format!("unexpected character '{}'", bytes[i] as char),
                )
                .with_span(Span::new(start as u32, (i + 1) as u32)));
            }
        }
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(input: &str) -> Result<Vec<TokenKind>, QueryError> {
        Ok(lex(input)?.into_iter().map(|t| t.kind).collect())
    }

    #[test]
    fn test_simple_ident() {
        assert_eq!(kinds("component").unwrap(), vec![TokenKind::Ident("component".into())]);
    }

    #[test]
    fn test_keywords() {
        assert_eq!(
            kinds("AND OR NOT").unwrap(),
            vec![TokenKind::And, TokenKind::Or, TokenKind::Not]
        );
        // Case-insensitive keywords
        assert_eq!(
            kinds("and or not").unwrap(),
            vec![TokenKind::And, TokenKind::Or, TokenKind::Not]
        );
    }

    #[test]
    fn test_booleans() {
        assert_eq!(
            kinds("true false").unwrap(),
            vec![TokenKind::Bool(true), TokenKind::Bool(false)]
        );
    }

    #[test]
    fn test_numbers() {
        assert_eq!(kinds("42").unwrap(), vec![TokenKind::Integer(42)]);
        assert_eq!(kinds("-10").unwrap(), vec![TokenKind::Integer(-10)]);
        assert_eq!(kinds("3.14").unwrap(), vec![TokenKind::Float(3.14)]);
    }

    #[test]
    fn test_dimensional_values() {
        assert_eq!(kinds("100mil").unwrap(), vec![TokenKind::Dim(100.0, Unit::Mil)]);
        assert_eq!(kinds("2.54mm").unwrap(), vec![TokenKind::Dim(2.54, Unit::Mm)]);
        assert_eq!(kinds("0.1in").unwrap(), vec![TokenKind::Dim(0.1, Unit::Inch)]);
    }

    #[test]
    fn test_unknown_unit_backtracks() {
        // Unknown suffixes after numbers are NOT errors — the lexer backtracks
        // and emits the number, then lexes the suffix as an identifier.
        assert_eq!(
            kinds("100foo").unwrap(),
            vec![TokenKind::Integer(100), TokenKind::Ident("foo".into())]
        );
        // Known units produce Dim tokens
        assert_eq!(
            kinds("100mil").unwrap(),
            vec![TokenKind::Dim(100.0, Unit::Mil)]
        );
    }

    #[test]
    fn test_string_literal() {
        assert_eq!(
            kinds(r#""hello world""#).unwrap(),
            vec![TokenKind::String("hello world".into())]
        );
    }

    #[test]
    fn test_unterminated_string() {
        assert!(kinds(r#""hello"#).is_err());
    }

    #[test]
    fn test_regex_literal() {
        assert_eq!(
            kinds(r#"/^U[0-9]+$/"#).unwrap(),
            vec![TokenKind::Regex("^U[0-9]+$".into())]
        );
    }

    #[test]
    fn test_unterminated_regex() {
        assert!(kinds("/abc").is_err());
    }

    #[test]
    fn test_comparison_operators() {
        assert_eq!(kinds("=").unwrap(), vec![TokenKind::Eq]);
        assert_eq!(kinds("!=").unwrap(), vec![TokenKind::NotEq]);
        assert_eq!(kinds("*=").unwrap(), vec![TokenKind::Contains]);
        assert_eq!(kinds("^=").unwrap(), vec![TokenKind::StartsWith]);
        assert_eq!(kinds("$=").unwrap(), vec![TokenKind::EndsWith]);
        assert_eq!(kinds("~=").unwrap(), vec![TokenKind::WordMatch]);
        assert_eq!(kinds("<=").unwrap(), vec![TokenKind::Lte]);
    }

    #[test]
    fn test_context_sensitive_gt() {
        // Outside brackets: child combinator
        let toks = lex("component > pin").unwrap();
        assert!(toks.iter().any(|t| t.kind == TokenKind::ChildCombinator));

        // Inside brackets: greater-than
        let toks = lex("[x>100]").unwrap();
        assert!(toks.iter().any(|t| t.kind == TokenKind::Gt));

        // >= always produces Gte regardless of context
        let toks = lex("[x>=100]").unwrap();
        assert!(toks.iter().any(|t| t.kind == TokenKind::Gte));
    }

    #[test]
    fn test_pattern_prefixes() {
        assert_eq!(
            kinds("$LM358").unwrap(),
            vec![TokenKind::Dollar, TokenKind::Ident("LM358".into())]
        );
        assert_eq!(
            kinds("@10K").unwrap(),
            vec![TokenKind::At, TokenKind::Integer(10), TokenKind::Ident("K".into())]
        );
        assert_eq!(
            kinds("%VCC").unwrap(),
            vec![TokenKind::Percent, TokenKind::Ident("VCC".into())]
        );
        assert_eq!(
            kinds("#42").unwrap(),
            vec![TokenKind::Hash, TokenKind::Integer(42)]
        );
    }

    #[test]
    fn test_wildcards() {
        assert_eq!(
            kinds("R*").unwrap(),
            vec![TokenKind::Ident("R".into()), TokenKind::Star]
        );
        assert_eq!(
            kinds("U?").unwrap(),
            vec![TokenKind::Ident("U".into()), TokenKind::Question]
        );
    }

    #[test]
    fn test_compound_query() {
        // Use a quoted string for "10K" to avoid lexing as Integer + Ident
        let toks = kinds(r#"component[value="10K"] > pin:power"#).unwrap();
        assert_eq!(
            toks,
            vec![
                TokenKind::Ident("component".into()),
                TokenKind::LBracket,
                TokenKind::Ident("value".into()),
                TokenKind::Eq,
                TokenKind::String("10K".into()),
                TokenKind::RBracket,
                TokenKind::ChildCombinator,
                TokenKind::Ident("pin".into()),
                TokenKind::Colon,
                TokenKind::Ident("power".into()),
            ]
        );
    }

    #[test]
    fn test_ident_with_hyphens() {
        assert_eq!(
            kinds("open-collector").unwrap(),
            vec![TokenKind::Ident("open-collector".into())]
        );
    }

    #[test]
    fn test_negative_number_not_ident_minus() {
        // '-' followed by digit -> negative number
        assert_eq!(kinds("-5").unwrap(), vec![TokenKind::Integer(-5)]);
    }
}

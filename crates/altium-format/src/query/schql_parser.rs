//! Query parser - tokenizes and parses SchQL query strings.

use super::ast::*;

/// Query parsing error
#[derive(Debug, Clone)]
pub enum QueryError {
    /// Syntax error in query
    ParseError { position: usize, message: String },
    /// Unknown element type
    UnknownElement(String),
    /// Unknown attribute
    UnknownAttribute(String),
    /// Unknown pseudo-selector
    UnknownPseudo(String),
    /// Invalid combinator
    InvalidCombinator(String),
    /// Type mismatch (e.g., pin selector on net)
    TypeMismatch { expected: String, got: String },
    /// Empty query
    EmptyQuery,
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryError::ParseError { position, message } => {
                write!(f, "Parse error at position {}: {}", position, message)
            }
            QueryError::UnknownElement(e) => write!(f, "Unknown element type: {}", e),
            QueryError::UnknownAttribute(a) => write!(f, "Unknown attribute: {}", a),
            QueryError::UnknownPseudo(p) => write!(f, "Unknown pseudo-selector: :{}", p),
            QueryError::InvalidCombinator(c) => write!(f, "Invalid combinator: {}", c),
            QueryError::TypeMismatch { expected, got } => {
                write!(f, "Type mismatch: expected {}, got {}", expected, got)
            }
            QueryError::EmptyQuery => write!(f, "Empty query"),
        }
    }
}

impl std::error::Error for QueryError {}

/// Token types for the query lexer
#[derive(Debug, Clone, PartialEq)]
enum Token {
    // Identifiers and values
    Ident(String),
    String(String),
    Number(i64),
    Float(f64),

    // Symbols
    Hash,          // #
    Dot,           // .
    Comma,         // ,
    Colon,         // :
    DoubleColon,   // ::
    Star,          // *
    Greater,       // >
    DoubleGreater, // >>
    Plus,          // +
    Tilde,         // ~
    LBracket,      // [
    RBracket,      // ]
    LParen,        // (
    RParen,        // )
    Equals,        // =
    NotEquals,     // !=
    TildeEquals,   // ~=
    CaretEquals,   // ^=
    DollarEquals,  // $=
    StarEquals,    // *=
    GreaterEquals, // >=
    LessEquals,    // <=
    Less,          // <

    // End of input
    Eof,
}

/// Query parser
pub struct QueryParser {
    // Configuration could go here
}

impl QueryParser {
    /// Create a new parser
    pub fn new() -> Self {
        Self {}
    }

    /// Parse a query string into a Selector AST
    pub fn parse(&self, input: &str) -> Result<Selector, QueryError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(QueryError::EmptyQuery);
        }

        let tokens = self.tokenize(input)?;
        let mut parser = SelectorParser::new(tokens);
        parser.parse_selector_list()
    }

    /// Tokenize input string
    fn tokenize(&self, input: &str) -> Result<Vec<Token>, QueryError> {
        let mut tokens = Vec::new();
        let mut chars = input.chars().peekable();
        let mut pos = 0;

        while let Some(&c) = chars.peek() {
            match c {
                // Whitespace - skip but track position
                ' ' | '\t' | '\n' | '\r' => {
                    chars.next();
                    pos += 1;
                }

                // Single-char tokens
                '#' => {
                    chars.next();
                    pos += 1;
                    tokens.push(Token::Hash);
                }
                '.' => {
                    chars.next();
                    pos += 1;
                    tokens.push(Token::Dot);
                }
                ',' => {
                    chars.next();
                    pos += 1;
                    tokens.push(Token::Comma);
                }
                '[' => {
                    chars.next();
                    pos += 1;
                    tokens.push(Token::LBracket);
                }
                ']' => {
                    chars.next();
                    pos += 1;
                    tokens.push(Token::RBracket);
                }
                '(' => {
                    chars.next();
                    pos += 1;
                    tokens.push(Token::LParen);
                }
                ')' => {
                    chars.next();
                    pos += 1;
                    tokens.push(Token::RParen);
                }
                '+' => {
                    chars.next();
                    pos += 1;
                    tokens.push(Token::Plus);
                }

                // Multi-char tokens starting with *
                '*' => {
                    chars.next();
                    pos += 1;
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        pos += 1;
                        tokens.push(Token::StarEquals);
                    } else {
                        tokens.push(Token::Star);
                    }
                }

                // Multi-char tokens starting with :
                ':' => {
                    chars.next();
                    pos += 1;
                    if chars.peek() == Some(&':') {
                        chars.next();
                        pos += 1;
                        tokens.push(Token::DoubleColon);
                    } else {
                        tokens.push(Token::Colon);
                    }
                }

                // Multi-char tokens starting with >
                '>' => {
                    chars.next();
                    pos += 1;
                    if chars.peek() == Some(&'>') {
                        chars.next();
                        pos += 1;
                        tokens.push(Token::DoubleGreater);
                    } else if chars.peek() == Some(&'=') {
                        chars.next();
                        pos += 1;
                        tokens.push(Token::GreaterEquals);
                    } else {
                        tokens.push(Token::Greater);
                    }
                }

                // Multi-char tokens starting with <
                '<' => {
                    chars.next();
                    pos += 1;
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        pos += 1;
                        tokens.push(Token::LessEquals);
                    } else {
                        tokens.push(Token::Less);
                    }
                }

                // Multi-char tokens starting with ~
                '~' => {
                    chars.next();
                    pos += 1;
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        pos += 1;
                        tokens.push(Token::TildeEquals);
                    } else {
                        tokens.push(Token::Tilde);
                    }
                }

                // ^= (starts with)
                '^' => {
                    chars.next();
                    pos += 1;
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        pos += 1;
                        tokens.push(Token::CaretEquals);
                    } else {
                        return Err(QueryError::ParseError {
                            position: pos,
                            message: "Expected '=' after '^'".to_string(),
                        });
                    }
                }

                // $= (ends with)
                '$' => {
                    chars.next();
                    pos += 1;
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        pos += 1;
                        tokens.push(Token::DollarEquals);
                    } else {
                        return Err(QueryError::ParseError {
                            position: pos,
                            message: "Expected '=' after '$'".to_string(),
                        });
                    }
                }

                // != (not equals)
                '!' => {
                    chars.next();
                    pos += 1;
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        pos += 1;
                        tokens.push(Token::NotEquals);
                    } else {
                        return Err(QueryError::ParseError {
                            position: pos,
                            message: "Expected '=' after '!'".to_string(),
                        });
                    }
                }

                '=' => {
                    chars.next();
                    pos += 1;
                    tokens.push(Token::Equals);
                }

                // String literals
                '"' | '\'' => {
                    let quote = c;
                    chars.next();
                    pos += 1;
                    let mut s = String::new();
                    loop {
                        match chars.peek() {
                            Some(&c) if c == quote => {
                                chars.next();
                                pos += 1;
                                break;
                            }
                            Some(&'\\') => {
                                chars.next();
                                pos += 1;
                                if let Some(&escaped) = chars.peek() {
                                    chars.next();
                                    pos += 1;
                                    s.push(escaped);
                                }
                            }
                            Some(&c) => {
                                s.push(c);
                                chars.next();
                                pos += 1;
                            }
                            None => {
                                return Err(QueryError::ParseError {
                                    position: pos,
                                    message: "Unterminated string".to_string(),
                                });
                            }
                        }
                    }
                    tokens.push(Token::String(s));
                }

                // Numbers (including negative)
                '0'..='9' => {
                    let mut num_str = String::new();
                    let mut has_dot = false;
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_digit() {
                            num_str.push(c);
                            chars.next();
                            pos += 1;
                        } else if c == '.' && !has_dot {
                            has_dot = true;
                            num_str.push(c);
                            chars.next();
                            pos += 1;
                        } else {
                            break;
                        }
                    }
                    if has_dot {
                        tokens.push(Token::Float(num_str.parse().unwrap_or(0.0)));
                    } else {
                        tokens.push(Token::Number(num_str.parse().unwrap_or(0)));
                    }
                }

                // Negative numbers
                '-' => {
                    chars.next();
                    pos += 1;
                    if chars.peek().is_some_and(|c| c.is_ascii_digit()) {
                        let mut num_str = String::from("-");
                        let mut has_dot = false;
                        while let Some(&c) = chars.peek() {
                            if c.is_ascii_digit() {
                                num_str.push(c);
                                chars.next();
                                pos += 1;
                            } else if c == '.' && !has_dot {
                                has_dot = true;
                                num_str.push(c);
                                chars.next();
                                pos += 1;
                            } else {
                                break;
                            }
                        }
                        if has_dot {
                            tokens.push(Token::Float(num_str.parse().unwrap_or(0.0)));
                        } else {
                            tokens.push(Token::Number(num_str.parse().unwrap_or(0)));
                        }
                    } else {
                        // Treat as part of an identifier
                        let mut ident = String::from("-");
                        while let Some(&c) = chars.peek() {
                            if c.is_alphanumeric() || c == '_' || c == '-' {
                                ident.push(c);
                                chars.next();
                                pos += 1;
                            } else {
                                break;
                            }
                        }
                        tokens.push(Token::Ident(ident));
                    }
                }

                // Identifiers
                _ if c.is_alphabetic() || c == '_' => {
                    let mut ident = String::new();
                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' || c == '-' {
                            ident.push(c);
                            chars.next();
                            pos += 1;
                        } else {
                            break;
                        }
                    }
                    tokens.push(Token::Ident(ident));
                }

                _ => {
                    return Err(QueryError::ParseError {
                        position: pos,
                        message: format!("Unexpected character: '{}'", c),
                    });
                }
            }
        }

        tokens.push(Token::Eof);
        Ok(tokens)
    }
}

impl Default for QueryParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Recursive descent parser for selectors
struct SelectorParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl SelectorParser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn _peek_next(&self) -> &Token {
        self.tokens.get(self.pos + 1).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.current().clone();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), QueryError> {
        if self.current() == expected {
            self.advance();
            Ok(())
        } else {
            Err(QueryError::ParseError {
                position: self.pos,
                message: format!("Expected {:?}, got {:?}", expected, self.current()),
            })
        }
    }

    /// Parse comma-separated selector list
    fn parse_selector_list(&mut self) -> Result<Selector, QueryError> {
        let mut selectors = vec![self.parse_combinator_chain()?];

        while self.current() == &Token::Comma {
            self.advance();
            selectors.push(self.parse_combinator_chain()?);
        }

        if selectors.len() == 1 {
            Ok(selectors.pop().unwrap())
        } else {
            Ok(Selector::Union(selectors))
        }
    }

    /// Parse combinators: A B, A > B, A >> B, A ~ B, A + B, A :: B
    fn parse_combinator_chain(&mut self) -> Result<Selector, QueryError> {
        let mut left = self.parse_compound_selector()?;

        loop {
            let combinator = match self.current() {
                Token::Greater => {
                    self.advance();
                    CombinatorType::Child
                }
                Token::DoubleGreater => {
                    self.advance();
                    CombinatorType::Connected
                }
                Token::Tilde => {
                    self.advance();
                    CombinatorType::Sibling
                }
                Token::Plus => {
                    self.advance();
                    CombinatorType::Adjacent
                }
                Token::DoubleColon => {
                    self.advance();
                    CombinatorType::OnNet
                }
                // Check for implicit descendant combinator (when next token starts a selector)
                // These tokens can start a new selector, indicating an implicit descendant relationship
                Token::Ident(_) | Token::Hash | Token::Star | Token::LBracket | Token::Colon => {
                    CombinatorType::Descendant
                }
                _ => break,
            };

            let right = self.parse_compound_selector()?;
            left = Selector::Combinator {
                left: Box::new(left),
                combinator,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse compound selector (multiple simple selectors on same element)
    ///
    /// In CSS-style selectors, a compound can have at most one type selector or ID selector,
    /// followed by any number of attribute selectors and pseudo-selectors.
    /// Examples:
    /// - `component` - just a type selector
    /// - `#U1` - just an ID selector
    /// - `pin[type=input]` - type + attribute
    /// - `component:first` - type + pseudo
    ///
    /// But NOT: `component pin` (that's a descendant, not a compound)
    fn parse_compound_selector(&mut self) -> Result<Selector, QueryError> {
        let mut parts = Vec::new();
        let mut has_primary = false; // Track if we have an element/ID/universal selector

        loop {
            match self.current() {
                Token::Ident(name) if !has_primary => {
                    let name = name.clone();
                    self.advance();
                    if let Some(elem_type) = ElementType::try_parse(&name) {
                        parts.push(Selector::Element(elem_type));
                        has_primary = true;
                    } else {
                        return Err(QueryError::UnknownElement(name));
                    }
                }
                Token::Hash if !has_primary => {
                    self.advance();
                    match self.current() {
                        Token::Ident(id) => {
                            let id = id.clone();
                            self.advance();
                            parts.push(Selector::Id(id));
                            has_primary = true;
                        }
                        Token::Number(n) => {
                            // Allow numeric IDs like #1, #2
                            let id = n.to_string();
                            self.advance();
                            parts.push(Selector::Id(id));
                            has_primary = true;
                        }
                        _ => {
                            return Err(QueryError::ParseError {
                                position: self.pos,
                                message: "Expected identifier after #".to_string(),
                            });
                        }
                    }
                }
                Token::Star if !has_primary => {
                    self.advance();
                    parts.push(Selector::Universal);
                    has_primary = true;
                }
                Token::LBracket => {
                    parts.push(self.parse_attribute_selector()?);
                }
                Token::Colon => {
                    parts.push(self.parse_pseudo_selector()?);
                }
                _ => break,
            }
        }

        if parts.is_empty() {
            Err(QueryError::ParseError {
                position: self.pos,
                message: format!("Expected selector, got {:?}", self.current()),
            })
        } else if parts.len() == 1 {
            Ok(parts.pop().unwrap())
        } else {
            Ok(Selector::Compound(parts))
        }
    }

    /// Parse [attr=value] selector
    fn parse_attribute_selector(&mut self) -> Result<Selector, QueryError> {
        self.expect(&Token::LBracket)?;

        let name = match self.current() {
            Token::Ident(name) => {
                let n = name.clone().to_lowercase();
                self.advance();
                n
            }
            _ => {
                return Err(QueryError::ParseError {
                    position: self.pos,
                    message: "Expected attribute name".to_string(),
                });
            }
        };

        // Check for operator
        let (op, value) = match self.current() {
            Token::RBracket => (AttributeOp::Exists, None),
            Token::Equals => {
                self.advance();
                (AttributeOp::Equals, Some(self.parse_value()?))
            }
            Token::NotEquals => {
                self.advance();
                (AttributeOp::NotEquals, Some(self.parse_value()?))
            }
            Token::TildeEquals => {
                self.advance();
                (AttributeOp::WordMatch, Some(self.parse_value()?))
            }
            Token::CaretEquals => {
                self.advance();
                (AttributeOp::StartsWith, Some(self.parse_value()?))
            }
            Token::DollarEquals => {
                self.advance();
                (AttributeOp::EndsWith, Some(self.parse_value()?))
            }
            Token::StarEquals => {
                self.advance();
                (AttributeOp::Contains, Some(self.parse_value()?))
            }
            Token::Greater => {
                self.advance();
                (AttributeOp::GreaterThan, Some(self.parse_value()?))
            }
            Token::Less => {
                self.advance();
                (AttributeOp::LessThan, Some(self.parse_value()?))
            }
            Token::GreaterEquals => {
                self.advance();
                (AttributeOp::GreaterOrEqual, Some(self.parse_value()?))
            }
            Token::LessEquals => {
                self.advance();
                (AttributeOp::LessOrEqual, Some(self.parse_value()?))
            }
            _ => {
                return Err(QueryError::ParseError {
                    position: self.pos,
                    message: format!("Expected operator or ], got {:?}", self.current()),
                });
            }
        };

        // Check for case-insensitive flag
        let case_insensitive = if let Token::Ident(flag) = self.current() {
            if flag.to_lowercase() == "i" {
                self.advance();
                true
            } else {
                false
            }
        } else {
            false
        };

        self.expect(&Token::RBracket)?;

        Ok(Selector::Attribute(AttributeSelector {
            name,
            op,
            value,
            case_insensitive,
        }))
    }

    /// Parse :pseudo or :pseudo(arg)
    fn parse_pseudo_selector(&mut self) -> Result<Selector, QueryError> {
        self.expect(&Token::Colon)?;

        let name = match self.current() {
            Token::Ident(name) => {
                let n = name.clone().to_lowercase();
                self.advance();
                n
            }
            _ => {
                return Err(QueryError::ParseError {
                    position: self.pos,
                    message: "Expected pseudo-selector name".to_string(),
                });
            }
        };

        // Handle functional pseudo-selectors: :not() and :has()
        if name == "not" {
            self.expect(&Token::LParen)?;
            let inner = self.parse_selector_list()?;
            self.expect(&Token::RParen)?;
            return Ok(Selector::Not(Box::new(inner)));
        }

        if name == "has" {
            self.expect(&Token::LParen)?;
            let inner = self.parse_selector_list()?;
            self.expect(&Token::RParen)?;
            return Ok(Selector::Has(Box::new(inner)));
        }

        // Parse optional argument for pseudo-selectors like :nth(3), :limit(10)
        let arg = if self.current() == &Token::LParen {
            self.advance();
            let arg = self.parse_value()?;
            self.expect(&Token::RParen)?;
            Some(arg)
        } else {
            None
        };

        if let Some(pseudo) = PseudoSelector::try_parse(&name, arg.as_deref()) {
            Ok(Selector::Pseudo(pseudo))
        } else {
            Err(QueryError::UnknownPseudo(name))
        }
    }

    /// Parse a value (string, number, or identifier)
    fn parse_value(&mut self) -> Result<String, QueryError> {
        match self.current().clone() {
            Token::String(s) => {
                self.advance();
                Ok(s)
            }
            Token::Number(n) => {
                self.advance();
                Ok(n.to_string())
            }
            Token::Float(f) => {
                self.advance();
                Ok(f.to_string())
            }
            Token::Ident(s) => {
                self.advance();
                Ok(s)
            }
            _ => Err(QueryError::ParseError {
                position: self.pos,
                message: format!("Expected value, got {:?}", self.current()),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_element() {
        let parser = QueryParser::new();
        let sel = parser.parse("component").unwrap();
        assert_eq!(sel, Selector::Element(ElementType::Component));
    }

    #[test]
    fn test_parse_id() {
        let parser = QueryParser::new();
        let sel = parser.parse("#U1").unwrap();
        assert_eq!(sel, Selector::Id("U1".to_string()));
    }

    #[test]
    fn test_parse_attribute() {
        let parser = QueryParser::new();
        let sel = parser.parse("[part=LM7805]").unwrap();
        match sel {
            Selector::Attribute(attr) => {
                assert_eq!(attr.name, "part");
                assert_eq!(attr.op, AttributeOp::Equals);
                assert_eq!(attr.value, Some("LM7805".to_string()));
            }
            _ => panic!("Expected attribute selector"),
        }
    }

    #[test]
    fn test_parse_attribute_contains() {
        let parser = QueryParser::new();
        let sel = parser.parse("[part*=7805]").unwrap();
        match sel {
            Selector::Attribute(attr) => {
                assert_eq!(attr.name, "part");
                assert_eq!(attr.op, AttributeOp::Contains);
                assert_eq!(attr.value, Some("7805".to_string()));
            }
            _ => panic!("Expected attribute selector"),
        }
    }

    #[test]
    fn test_parse_pseudo() {
        let parser = QueryParser::new();
        let sel = parser.parse(":connected").unwrap();
        assert_eq!(sel, Selector::Pseudo(PseudoSelector::Connected));
    }

    #[test]
    fn test_parse_pseudo_with_arg() {
        let parser = QueryParser::new();
        let sel = parser.parse(":limit(10)").unwrap();
        assert_eq!(sel, Selector::Pseudo(PseudoSelector::Limit(10)));
    }

    #[test]
    fn test_parse_compound() {
        let parser = QueryParser::new();
        let sel = parser.parse("pin[type=input]").unwrap();
        match sel {
            Selector::Compound(parts) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0], Selector::Element(ElementType::Pin));
            }
            _ => panic!("Expected compound selector"),
        }
    }

    #[test]
    fn test_parse_child_combinator() {
        let parser = QueryParser::new();
        let sel = parser.parse("#U1 > pin").unwrap();
        match sel {
            Selector::Combinator {
                left,
                combinator,
                right,
            } => {
                assert_eq!(*left, Selector::Id("U1".to_string()));
                assert_eq!(combinator, CombinatorType::Child);
                assert_eq!(*right, Selector::Element(ElementType::Pin));
            }
            _ => panic!("Expected combinator selector"),
        }
    }

    #[test]
    fn test_parse_descendant_combinator() {
        let parser = QueryParser::new();
        let sel = parser.parse("#U1 pin").unwrap();
        match sel {
            Selector::Combinator { combinator, .. } => {
                assert_eq!(combinator, CombinatorType::Descendant);
            }
            _ => panic!("Expected combinator selector"),
        }
    }

    #[test]
    fn test_parse_union() {
        let parser = QueryParser::new();
        let sel = parser.parse("component, port").unwrap();
        match sel {
            Selector::Union(selectors) => {
                assert_eq!(selectors.len(), 2);
            }
            _ => panic!("Expected union selector"),
        }
    }

    #[test]
    fn test_parse_not() {
        let parser = QueryParser::new();
        let sel = parser.parse(":not(pin)").unwrap();
        match sel {
            Selector::Not(inner) => {
                assert_eq!(*inner, Selector::Element(ElementType::Pin));
            }
            _ => panic!("Expected not selector"),
        }
    }

    #[test]
    fn test_parse_complex() {
        let parser = QueryParser::new();
        // Complex query: components with 7805 in part name, get their pins
        let sel = parser
            .parse("component[part*=7805] > pin[type=input]")
            .unwrap();
        match sel {
            Selector::Combinator {
                left,
                combinator,
                right,
            } => {
                assert_eq!(combinator, CombinatorType::Child);
                match *left {
                    Selector::Compound(ref parts) => {
                        assert_eq!(parts.len(), 2);
                    }
                    _ => panic!("Expected compound on left"),
                }
                match *right {
                    Selector::Compound(ref parts) => {
                        assert_eq!(parts.len(), 2);
                    }
                    _ => panic!("Expected compound on right"),
                }
            }
            _ => panic!("Expected combinator"),
        }
    }

    #[test]
    fn test_parse_on_net() {
        let parser = QueryParser::new();
        let sel = parser.parse("#VCC :: pin").unwrap();
        match sel {
            Selector::Combinator { combinator, .. } => {
                assert_eq!(combinator, CombinatorType::OnNet);
            }
            _ => panic!("Expected on-net combinator"),
        }
    }

    #[test]
    fn test_parse_string_value() {
        let parser = QueryParser::new();
        let sel = parser.parse("[name=\"CLK IN\"]").unwrap();
        match sel {
            Selector::Attribute(attr) => {
                assert_eq!(attr.value, Some("CLK IN".to_string()));
            }
            _ => panic!("Expected attribute selector"),
        }
    }
}

use std::fmt;

use crate::diagnostic::{Span, locate_line};

/// Error code categories for query language errors.
///
/// - Q1xxx: Lexer errors (tokenization)
/// - Q2xxx: Parser errors (grammar)
/// - Q3xxx: Schema errors (field validation)
/// - Q4xxx: Evaluation errors (runtime)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryErrorCode {
    // Lexer (Q1xxx)
    UnexpectedCharacter,
    UnterminatedString,
    UnterminatedRegex,
    InvalidNumber,
    UnknownUnit,

    // Parser (Q2xxx)
    UnexpectedToken,
    ExpectedSelector,
    UnknownTypeSelector,
    UnknownPseudoClass,
    ExpectedExpression,
    ExpectedCloseBracket,
    ExpectedCloseParen,

    // Schema (Q3xxx)
    UnknownField,
    IncompatibleOperator,

    // Eval (Q4xxx)
    DocumentError,
    Unsupported,
}

impl QueryErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnexpectedCharacter => "Q1001",
            Self::UnterminatedString => "Q1002",
            Self::UnterminatedRegex => "Q1003",
            Self::InvalidNumber => "Q1004",
            Self::UnknownUnit => "Q1005",
            Self::UnexpectedToken => "Q2001",
            Self::ExpectedSelector => "Q2002",
            Self::UnknownTypeSelector => "Q2003",
            Self::UnknownPseudoClass => "Q2004",
            Self::ExpectedExpression => "Q2005",
            Self::ExpectedCloseBracket => "Q2006",
            Self::ExpectedCloseParen => "Q2007",
            Self::UnknownField => "Q3001",
            Self::IncompatibleOperator => "Q3002",
            Self::DocumentError => "Q4001",
            Self::Unsupported => "Q4002",
        }
    }
}

/// A query language error with optional source span and help text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryError {
    pub code: QueryErrorCode,
    pub message: String,
    pub span: Option<Span>,
    pub help: Option<String>,
}

impl QueryError {
    pub fn new(code: QueryErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            span: None,
            help: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Render a rich error diagnostic (rustc-style) given the query source text.
    pub fn render(&self, source: &str) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "error[{}]: {}\n",
            self.code.as_str(),
            self.message
        ));

        if let Some(span) = self.span {
            let (line_no, col_no, line_text) = locate_line(source, span.start as usize);
            out.push_str(&format!(" --> query:{}:{}\n", line_no, col_no));
            out.push_str("  |\n");
            out.push_str(&format!("{:>2} | {}\n", line_no, line_text));
            out.push_str("  | ");

            let caret_count = if span.end > span.start {
                let start_byte = span.start as usize;
                let end_byte = (span.end as usize).min(source.len());
                if end_byte > start_byte {
                    source[start_byte..end_byte].chars().count().max(1)
                } else {
                    1
                }
            } else {
                1
            };

            out.push_str(&" ".repeat(col_no.saturating_sub(1)));
            out.push_str(&"^".repeat(caret_count));
            out.push('\n');
        }

        if let Some(help) = &self.help {
            out.push_str(&format!("  = help: {}\n", help));
        }
        out
    }
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for QueryError {}

/// Result type for query operations.
pub type QueryResult<T> = Result<T, QueryError>;

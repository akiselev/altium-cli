use std::fmt;

/// A byte-offset span within query source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn merge(self, other: Span) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// An AST node annotated with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub const fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}

/// Coordinate unit for dimensional values in queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Mil,
    Mm,
    Inch,
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unit::Mil => write!(f, "mil"),
            Unit::Mm => write!(f, "mm"),
            Unit::Inch => write!(f, "in"),
        }
    }
}

/// Locate the line number, column number (1-based, char-aware), and line text
/// for a byte offset within source text.
pub fn locate_line(source: &str, pos: usize) -> (usize, usize, String) {
    let bounded = pos.min(source.len());
    let mut line_no = 1usize;
    let mut line_start = 0usize;
    for (idx, ch) in source.char_indices() {
        if idx >= bounded {
            break;
        }
        if ch == '\n' {
            line_no += 1;
            line_start = idx + ch.len_utf8();
        }
    }

    let line_end = source[line_start..]
        .find('\n')
        .map(|x| x + line_start)
        .unwrap_or(source.len());
    let line_text = source[line_start..line_end].to_string();
    let col_no = source[line_start..bounded].chars().count() + 1;
    (line_no, col_no, line_text)
}

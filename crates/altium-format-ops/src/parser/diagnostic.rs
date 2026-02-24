use std::fmt;

use super::ast::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorCode {
    E1001,
    E1002,
    E1003,
    E1004,
    E1005,
    E1006,
    E1007,
    E1008,
    E2001,
    E2002,
    E2003,
    E2004,
    E2005,
    E2006,
    E2007,
    E2008,
}

impl ParseErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::E1001 => "E1001",
            Self::E1002 => "E1002",
            Self::E1003 => "E1003",
            Self::E1004 => "E1004",
            Self::E1005 => "E1005",
            Self::E1006 => "E1006",
            Self::E1007 => "E1007",
            Self::E1008 => "E1008",
            Self::E2001 => "E2001",
            Self::E2002 => "E2002",
            Self::E2003 => "E2003",
            Self::E2004 => "E2004",
            Self::E2005 => "E2005",
            Self::E2006 => "E2006",
            Self::E2007 => "E2007",
            Self::E2008 => "E2008",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub code: ParseErrorCode,
    pub message: String,
    pub span: Span,
    pub help: Option<String>,
    pub notes: Vec<String>,
}

impl ParseError {
    pub fn new(code: ParseErrorCode, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            message: message.into(),
            span,
            help: None,
            notes: Vec::new(),
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn render(&self, source_name: &str, source: &str) -> String {
        let (line_no, col_no, line_text) = locate_line(source, self.span.start as usize);
        let mut out = String::new();
        out.push_str(&format!(
            "error[{}]: {}\n",
            self.code.as_str(),
            self.message
        ));
        out.push_str(&format!(" --> {}:{}:{}\n", source_name, line_no, col_no));
        out.push_str("  |\n");
        out.push_str(&format!("{:>2} | {}\n", line_no, line_text));
        out.push_str("  | ");
        let caret_count = caret_len(self.span, source, line_no, col_no);
        out.push_str(&" ".repeat(col_no.saturating_sub(1)));
        out.push_str(&"^".repeat(caret_count));
        out.push('\n');
        if let Some(help) = &self.help {
            out.push_str(&format!("  = help: {}\n", help));
        }
        for note in &self.notes {
            out.push_str(&format!("  = note: {}\n", note));
        }
        out
    }
}

fn locate_line(source: &str, pos: usize) -> (usize, usize, String) {
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

fn caret_len(span: Span, source: &str, line_no: usize, col_no: usize) -> usize {
    if span.end <= span.start {
        return 1;
    }

    let mut cur_line = 1usize;
    let mut line_start = 0usize;
    for (idx, ch) in source.char_indices() {
        if cur_line == line_no {
            line_start = idx;
            break;
        }
        if ch == '\n' {
            cur_line += 1;
        }
    }

    let start_byte = byte_at_column(source, line_start, col_no);
    let end_byte = span.end as usize;
    if end_byte <= start_byte {
        return 1;
    }
    source[start_byte..end_byte].chars().count().max(1)
}

fn byte_at_column(source: &str, line_start: usize, column: usize) -> usize {
    let mut col = 1usize;
    for (idx, _) in source[line_start..].char_indices() {
        if col == column {
            return line_start + idx;
        }
        col += 1;
    }
    source.len()
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for ParseError {}

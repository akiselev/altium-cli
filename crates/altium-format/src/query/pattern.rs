//! Glob-style pattern matching for selectors.
//!
//! Supports:
//! - `*` - Match any characters (zero or more)
//! - `?` - Match exactly one character
//! - `[abc]` - Match any character in the set
//! - `[a-z]` - Match any character in the range
//! - `[!abc]` - Match any character NOT in the set

use regex::Regex;
use std::fmt;

use crate::error::{AltiumError, Result};

/// A compiled glob-style pattern for matching strings.
#[derive(Clone)]
pub struct Pattern {
    /// Original pattern string
    raw: String,
    /// Compiled regex for matching
    regex: Regex,
    /// Whether this pattern is a literal (no wildcards)
    is_literal: bool,
}

impl Pattern {
    /// Create a new pattern from a glob string.
    ///
    /// # Examples
    ///
    /// ```
    /// use altium_format::query::Pattern;
    ///
    /// let p = Pattern::new("R*").unwrap();
    /// assert!(p.matches("R1"));
    /// assert!(p.matches("R100"));
    /// assert!(!p.matches("C1"));
    ///
    /// let p = Pattern::new("R?").unwrap();
    /// assert!(p.matches("R1"));
    /// assert!(!p.matches("R10"));
    /// ```
    pub fn new(pattern: &str) -> Result<Self> {
        let is_literal = !pattern.contains(['*', '?', '[']);
        let regex_str = if is_literal {
            format!("(?i)^{}$", regex::escape(pattern))
        } else {
            Self::glob_to_regex(pattern)?
        };

        let regex = Regex::new(&regex_str)
            .map_err(|e| AltiumError::Parse(format!("Invalid pattern '{}': {}", pattern, e)))?;

        Ok(Self {
            raw: pattern.to_string(),
            regex,
            is_literal,
        })
    }

    /// Create a pattern that matches a literal string exactly.
    pub fn literal(s: &str) -> Self {
        let regex_str = format!("(?i)^{}$", regex::escape(s));
        Self {
            raw: s.to_string(),
            regex: Regex::new(&regex_str).unwrap(),
            is_literal: true,
        }
    }

    /// Create a pattern that matches anything.
    pub fn any() -> Self {
        Self {
            raw: "*".to_string(),
            regex: Regex::new(".*").unwrap(),
            is_literal: false,
        }
    }

    /// Returns the original pattern string.
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// Returns true if this pattern has no wildcards.
    pub fn is_literal(&self) -> bool {
        self.is_literal
    }

    /// Test if a string matches this pattern (case-insensitive).
    pub fn matches(&self, text: &str) -> bool {
        self.regex.is_match(text)
    }

    /// Test if a string matches this pattern (case-sensitive).
    pub fn matches_case_sensitive(&self, text: &str) -> bool {
        if self.is_literal {
            self.raw == text
        } else {
            // For case-sensitive, we'd need a separate regex
            // For now, fall back to case-insensitive
            self.regex.is_match(text)
        }
    }

    /// Convert a glob pattern to a regex string.
    fn glob_to_regex(pattern: &str) -> Result<String> {
        let mut result = String::from("(?i)^"); // Case-insensitive
        let mut chars = pattern.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '*' => result.push_str(".*"),
                '?' => result.push('.'),
                '[' => {
                    result.push('[');
                    // Handle negation [!...]
                    if chars.peek() == Some(&'!') {
                        chars.next();
                        result.push('^');
                    }
                    // Copy characters until closing ]
                    // Don't escape '-' as it's used for ranges like [1-4]
                    let mut found_close = false;
                    for c in chars.by_ref() {
                        if c == ']' {
                            result.push(']');
                            found_close = true;
                            break;
                        }
                        // Only escape backslash and caret inside character class
                        if c == '\\' || c == '^' {
                            result.push('\\');
                        }
                        result.push(c);
                    }
                    if !found_close {
                        return Err(AltiumError::Parse(format!(
                            "Unclosed '[' in pattern '{}'",
                            pattern
                        )));
                    }
                }
                // Escape regex metacharacters
                '.' | '+' | '^' | '$' | '(' | ')' | '{' | '}' | '|' | '\\' => {
                    result.push('\\');
                    result.push(c);
                }
                _ => result.push(c),
            }
        }

        result.push('$');
        Ok(result)
    }
}

impl fmt::Debug for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pattern")
            .field("raw", &self.raw)
            .field("is_literal", &self.is_literal)
            .finish()
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl PartialEq for Pattern {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl Eq for Pattern {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_pattern() {
        let p = Pattern::new("R1").unwrap();
        assert!(p.is_literal());
        assert!(p.matches("R1"));
        assert!(p.matches("r1")); // Case-insensitive
        assert!(!p.matches("R2"));
        assert!(!p.matches("R10"));
    }

    #[test]
    fn test_star_wildcard() {
        let p = Pattern::new("R*").unwrap();
        assert!(!p.is_literal());
        assert!(p.matches("R"));
        assert!(p.matches("R1"));
        assert!(p.matches("R10"));
        assert!(p.matches("R100"));
        assert!(p.matches("RESISTOR"));
        assert!(!p.matches("C1"));
    }

    #[test]
    fn test_question_wildcard() {
        let p = Pattern::new("R?").unwrap();
        assert!(p.matches("R1"));
        assert!(p.matches("R9"));
        assert!(p.matches("Ra"));
        assert!(!p.matches("R"));
        assert!(!p.matches("R10"));
    }

    #[test]
    fn test_double_question() {
        let p = Pattern::new("R??").unwrap();
        assert!(p.matches("R10"));
        assert!(p.matches("R99"));
        assert!(!p.matches("R1"));
        assert!(!p.matches("R100"));
    }

    #[test]
    fn test_character_class() {
        let p = Pattern::new("[RC]*").unwrap();
        assert!(p.matches("R1"));
        assert!(p.matches("C1"));
        assert!(p.matches("R100"));
        assert!(!p.matches("U1"));
        assert!(!p.matches("L1"));
    }

    #[test]
    fn test_character_range() {
        let p = Pattern::new("U[1-4]").unwrap();
        assert!(p.matches("U1"));
        assert!(p.matches("U2"));
        assert!(p.matches("U3"));
        assert!(p.matches("U4"));
        assert!(!p.matches("U5"));
        assert!(!p.matches("U0"));
    }

    #[test]
    fn test_negated_class() {
        let p = Pattern::new("[!RC]*").unwrap();
        assert!(p.matches("U1"));
        assert!(p.matches("L1"));
        assert!(!p.matches("R1"));
        assert!(!p.matches("C1"));
    }

    #[test]
    fn test_complex_pattern() {
        let p = Pattern::new("*CLK*").unwrap();
        assert!(p.matches("CLK"));
        assert!(p.matches("SPI_CLK"));
        assert!(p.matches("CLK_OUT"));
        assert!(p.matches("SYS_CLK_IN"));
        assert!(!p.matches("CLOCK"));
    }

    #[test]
    fn test_case_insensitive() {
        let p = Pattern::new("VCC").unwrap();
        assert!(p.matches("VCC"));
        assert!(p.matches("vcc"));
        assert!(p.matches("Vcc"));
    }

    #[test]
    fn test_special_chars() {
        let p = Pattern::new("R1.5K").unwrap();
        assert!(p.matches("R1.5K"));
        assert!(!p.matches("R15K"));
    }
}

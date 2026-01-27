//! Parser for the ergonomic selector syntax.
//!
//! # Syntax Overview
//!
//! - `U1` - Component by designator
//! - `R*` - Components matching designator pattern
//! - `U1:3` - Pin 3 of component U1
//! - `U1:VCC` - VCC pin of U1
//! - `$LM358` - Component by part number
//! - `~VCC` - Net by name
//! - `@10K` - Component by value
//! - `#Power` - Sheet by name
//! - `R*@10K` - Resistors with value 10K
//! - `component[x>1000]` - Type with property filter
//! - `U*:output:unconnected` - Output pins on ICs that are unconnected

use crate::error::{AltiumError, Result};

use super::pattern::Pattern;
use super::selector::{
    Combinator, FilterOperator, FilterValue, NetConnectedTarget, PropertyFilter, PseudoSelector,
    RecordMatcher, RecordType, Selector, SelectorChain, SelectorSegment,
};

/// Parser for selector strings.
pub struct SelectorParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> SelectorParser<'a> {
    /// Create a new parser for the given input.
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.trim(),
            pos: 0,
        }
    }

    /// Parse the input into a Selector.
    pub fn parse(mut self) -> Result<Selector> {
        if self.input.is_empty() {
            return Ok(Selector::any());
        }

        let mut alternatives = vec![];

        loop {
            self.skip_whitespace();
            if self.is_at_end() {
                break;
            }

            let chain = self.parse_chain()?;
            alternatives.push(chain);

            self.skip_whitespace();
            if self.peek() == Some(',') {
                self.advance();
                self.skip_whitespace();
            } else {
                break;
            }
        }

        if alternatives.is_empty() {
            Ok(Selector::any())
        } else {
            Ok(Selector { alternatives })
        }
    }

    /// Parse a selector chain (segments connected by combinators).
    fn parse_chain(&mut self) -> Result<SelectorChain> {
        let mut segments = vec![];

        loop {
            self.skip_whitespace();
            if self.is_at_end() || self.peek() == Some(',') {
                break;
            }

            let mut segment = self.parse_segment()?;

            // Check for combinator
            self.skip_whitespace();
            if let Some(combinator) = self.try_parse_combinator() {
                segment.combinator = Some(combinator);
                self.skip_whitespace();
            }

            segments.push(segment);

            // If no combinator was found and we're not at a comma/end, we're done
            if segments.last().unwrap().combinator.is_none() {
                // Check if next char could start a new segment
                let next = self.peek();
                let could_continue = next
                    .map(|c| {
                        c.is_alphanumeric()
                            || c == '$'
                            || c == '~'
                            || c == '@'
                            || c == '#'
                            || c == '*'
                            || c == '['
                            || c == ':'
                    })
                    .unwrap_or(false);

                if could_continue {
                    // Implicit descendant combinator
                    segments.last_mut().unwrap().combinator = Some(Combinator::Descendant);
                } else {
                    break;
                }
            }
        }

        Ok(SelectorChain { segments })
    }

    /// Parse a single selector segment.
    fn parse_segment(&mut self) -> Result<SelectorSegment> {
        self.skip_whitespace();

        // Determine matcher type from prefix
        let matcher = self.parse_matcher()?;

        // Parse property filters [prop op value]
        let mut filters = vec![];
        while self.peek() == Some('[') {
            filters.push(self.parse_property_filter()?);
        }

        // Parse pseudo-selectors :name
        let mut pseudo = vec![];
        while self.peek() == Some(':') {
            // Look ahead to see if this is a pseudo-selector or part of matcher
            let saved_pos = self.pos;
            self.advance(); // skip ':'

            // Check if this looks like a pseudo-selector
            if let Some(c) = self.peek() {
                if c.is_alphabetic() {
                    let name = self.parse_identifier_chars();
                    if let Some(ps) = PseudoSelector::from_name(&name) {
                        pseudo.push(ps);
                        continue;
                    } else if name == "not" || name == "has" || name == "is" {
                        // Parse nested selector
                        self.expect('(')?;
                        let inner = self.parse_until_char(')')?;
                        self.expect(')')?;
                        let inner_selector = SelectorParser::new(&inner).parse()?;
                        let ps = match name.as_str() {
                            "not" => PseudoSelector::Not(Box::new(inner_selector)),
                            "has" => PseudoSelector::Has(Box::new(inner_selector)),
                            "is" => PseudoSelector::Is(Box::new(inner_selector)),
                            _ => unreachable!(),
                        };
                        pseudo.push(ps);
                        continue;
                    } else if name == "nth-child" || name == "nthchild" {
                        self.expect('(')?;
                        let n_str = self.parse_until_char(')')?;
                        self.expect(')')?;
                        let n: usize = n_str.trim().parse().map_err(|_| {
                            AltiumError::Parse(format!("Invalid nth-child value: {}", n_str))
                        })?;
                        pseudo.push(PseudoSelector::NthChild(n));
                        continue;
                    }
                }
            }

            // Not a pseudo-selector, restore position
            self.pos = saved_pos;
            break;
        }

        Ok(SelectorSegment {
            matcher,
            filters,
            pseudo,
            combinator: None,
        })
    }

    /// Parse the record matcher part of a segment.
    fn parse_matcher(&mut self) -> Result<RecordMatcher> {
        match self.peek() {
            Some('$') => {
                // Part number: $LM358
                self.advance();
                let pattern = self.parse_pattern()?;
                Ok(RecordMatcher::PartNumber(pattern))
            }
            Some('~') => {
                // Net reference: ~VCC or ~VCC:pins
                self.advance();
                let net_pattern = self.parse_pattern()?;

                // Check for :pins or :components suffix
                if self.peek() == Some(':') {
                    let saved_pos = self.pos;
                    self.advance();
                    let target_name = self.parse_identifier_chars();

                    match target_name.to_lowercase().as_str() {
                        "pins" => Ok(RecordMatcher::NetConnected {
                            net: net_pattern,
                            target: NetConnectedTarget::Pins,
                        }),
                        "components" => Ok(RecordMatcher::NetConnected {
                            net: net_pattern,
                            target: NetConnectedTarget::Components,
                        }),
                        _ => {
                            // Not a net target, restore and return plain net
                            self.pos = saved_pos;
                            Ok(RecordMatcher::Net(net_pattern))
                        }
                    }
                } else {
                    Ok(RecordMatcher::Net(net_pattern))
                }
            }
            Some('@') => {
                // Value: @10K
                self.advance();
                let pattern = self.parse_pattern()?;
                Ok(RecordMatcher::Value(pattern))
            }
            Some('#') => {
                // Sheet: #Power
                self.advance();
                let pattern = self.parse_pattern()?;
                Ok(RecordMatcher::Sheet(pattern))
            }
            Some('*') => {
                // Any record
                self.advance();
                Ok(RecordMatcher::Any)
            }
            Some('[') | Some(':') => {
                // Property filter or pseudo-selector without type specifier
                Ok(RecordMatcher::Any)
            }
            _ => {
                // Could be: designator, type name, or designator:pin
                let ident = self.parse_identifier()?;

                // Check if it's a type keyword
                if let Some(record_type) = RecordType::try_parse(&ident) {
                    return Ok(RecordMatcher::Type(record_type));
                }

                // It's a designator pattern
                let designator_pattern = Pattern::new(&ident)?;

                // Check for :pin suffix
                if self.peek() == Some(':') {
                    let saved_pos = self.pos;
                    self.advance();

                    // Check if this is a pin pattern or a pseudo-selector
                    if let Some(c) = self.peek() {
                        if c.is_alphanumeric() || c == '*' || c == '?' || c == '[' {
                            let pin_ident = self.parse_identifier()?;

                            // Check if it's actually a pseudo-selector
                            if PseudoSelector::from_name(&pin_ident).is_some()
                                || pin_ident == "not"
                                || pin_ident == "has"
                                || pin_ident == "is"
                                || pin_ident.starts_with("nth")
                            {
                                // Restore and let pseudo-selector parsing handle it
                                self.pos = saved_pos;
                                return Ok(RecordMatcher::Designator(designator_pattern));
                            }

                            let pin_pattern = Pattern::new(&pin_ident)?;
                            return Ok(RecordMatcher::Pin {
                                component: designator_pattern,
                                pin: pin_pattern,
                            });
                        }
                    }
                    // Restore position if not a pin pattern
                    self.pos = saved_pos;
                }

                // Check for @value suffix (R*@10K)
                if self.peek() == Some('@') {
                    self.advance();
                    let value_pattern = self.parse_pattern()?;
                    return Ok(RecordMatcher::DesignatorWithValue {
                        designator: designator_pattern,
                        value: value_pattern,
                    });
                }

                Ok(RecordMatcher::Designator(designator_pattern))
            }
        }
    }

    /// Parse a property filter like [rotation=90].
    fn parse_property_filter(&mut self) -> Result<PropertyFilter> {
        self.expect('[')?;
        self.skip_whitespace();

        // Parse property name
        let property = self.parse_identifier()?;
        self.skip_whitespace();

        // Parse operator
        let operator = self.parse_operator()?;
        self.skip_whitespace();

        // Parse value
        let value = self.parse_filter_value()?;
        self.skip_whitespace();

        self.expect(']')?;

        Ok(PropertyFilter {
            property,
            operator,
            value,
        })
    }

    /// Parse a filter operator.
    fn parse_operator(&mut self) -> Result<FilterOperator> {
        // Try two-character operators first
        let two_char = self.peek_n(2);
        if let Some(op) = two_char.and_then(|s| FilterOperator::try_parse(&s)) {
            self.advance();
            self.advance();
            return Ok(op);
        }

        // Try single-character operators
        if let Some(c) = self.peek() {
            let s = c.to_string();
            if let Some(op) = FilterOperator::try_parse(&s) {
                self.advance();
                return Ok(op);
            }
        }

        Err(AltiumError::Parse(format!(
            "Expected operator at position {}",
            self.pos
        )))
    }

    /// Parse a filter value.
    fn parse_filter_value(&mut self) -> Result<FilterValue> {
        match self.peek() {
            Some('"') | Some('\'') => {
                // Quoted string
                let quote = self.advance().unwrap();
                let value = self.parse_until_char(quote)?;
                self.expect(quote)?;
                Ok(FilterValue::String(value))
            }
            Some(c) if c.is_ascii_digit() || c == '-' || c == '.' => {
                // Number
                let num_str = self.parse_number_str();
                let num: f64 = num_str
                    .parse()
                    .map_err(|_| AltiumError::Parse(format!("Invalid number: {}", num_str)))?;
                Ok(FilterValue::Number(num))
            }
            _ => {
                // Unquoted string or pattern
                let value = self.parse_until_chars(&[']', ' ', '\t']);
                if value.contains(['*', '?', '[']) {
                    Ok(FilterValue::Pattern(Pattern::new(&value)?))
                } else if value.eq_ignore_ascii_case("true") {
                    Ok(FilterValue::Bool(true))
                } else if value.eq_ignore_ascii_case("false") {
                    Ok(FilterValue::Bool(false))
                } else {
                    Ok(FilterValue::String(value))
                }
            }
        }
    }

    /// Try to parse a combinator.
    fn try_parse_combinator(&mut self) -> Option<Combinator> {
        match self.peek() {
            Some('>') => {
                self.advance();
                Some(Combinator::DirectChild)
            }
            Some('/') => {
                self.advance();
                Some(Combinator::DirectChild)
            }
            _ => None,
        }
    }

    /// Parse a pattern (identifier with wildcards).
    fn parse_pattern(&mut self) -> Result<Pattern> {
        let ident = self.parse_identifier()?;
        Pattern::new(&ident)
    }

    /// Parse an identifier (alphanumeric + wildcards).
    fn parse_identifier(&mut self) -> Result<String> {
        let ident = self.parse_identifier_chars();
        if ident.is_empty() {
            Err(AltiumError::Parse(format!(
                "Expected identifier at position {}",
                self.pos
            )))
        } else {
            Ok(ident)
        }
    }

    /// Parse identifier characters.
    /// Handles glob-style character classes [abc] properly.
    fn parse_identifier_chars(&mut self) -> String {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == '*' || c == '?' || c == '.' {
                self.advance();
            } else if c == '[' {
                // Check if this is a character class [abc] or a property filter [prop=val]
                // Look ahead to see if we find ] before any = or other filter operators
                let saved_pos = self.pos;
                self.advance(); // consume '['

                let mut found_close = false;
                let mut looks_like_filter = false;

                while let Some(inner_c) = self.peek() {
                    if inner_c == ']' {
                        self.advance();
                        found_close = true;
                        break;
                    } else if inner_c == '=' || inner_c == '>' || inner_c == '<' {
                        // Looks like a property filter, not a character class
                        looks_like_filter = true;
                        break;
                    } else if inner_c == '[' || inner_c == ' ' || inner_c == '\t' {
                        // Nested bracket or whitespace - not a character class
                        looks_like_filter = true;
                        break;
                    }
                    self.advance();
                }

                if !found_close || looks_like_filter {
                    // Not a character class, restore position and stop
                    self.pos = saved_pos;
                    break;
                }
                // Otherwise we consumed a valid character class, continue
            } else {
                break;
            }
        }
        self.input[start..self.pos].to_string()
    }

    /// Parse a number string.
    fn parse_number_str(&mut self) -> String {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E' {
                self.advance();
            } else {
                break;
            }
        }
        self.input[start..self.pos].to_string()
    }

    /// Parse until a specific character.
    fn parse_until_char(&mut self, end: char) -> Result<String> {
        let start = self.pos;
        let mut depth = 0;
        while let Some(c) = self.peek() {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                if depth == 0 && end == ')' {
                    break;
                }
                depth -= 1;
            } else if c == end && depth == 0 {
                break;
            }
            self.advance();
        }
        Ok(self.input[start..self.pos].to_string())
    }

    /// Parse until any of the specified characters.
    fn parse_until_chars(&mut self, ends: &[char]) -> String {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if ends.contains(&c) {
                break;
            }
            self.advance();
        }
        self.input[start..self.pos].to_string()
    }

    /// Expect a specific character.
    fn expect(&mut self, expected: char) -> Result<()> {
        match self.peek() {
            Some(c) if c == expected => {
                self.advance();
                Ok(())
            }
            Some(c) => Err(AltiumError::Parse(format!(
                "Expected '{}' but found '{}' at position {}",
                expected, c, self.pos
            ))),
            None => Err(AltiumError::Parse(format!(
                "Expected '{}' but reached end of input",
                expected
            ))),
        }
    }

    /// Skip whitespace characters.
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Peek at the current character.
    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    /// Peek at the next n characters.
    fn peek_n(&self, n: usize) -> Option<String> {
        let remaining = &self.input[self.pos..];
        if remaining.len() >= n {
            Some(remaining[..n].to_string())
        } else {
            None
        }
    }

    /// Advance to the next character.
    fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        if let Some(ch) = c {
            self.pos += ch.len_utf8();
        }
        c
    }

    /// Check if at end of input.
    fn is_at_end(&self) -> bool {
        self.pos >= self.input.len()
    }
}

/// Parse a selector string.
pub fn parse(input: &str) -> Result<Selector> {
    SelectorParser::new(input).parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_designator() {
        let sel = parse("U1").unwrap();
        assert_eq!(sel.alternatives.len(), 1);
        let chain = &sel.alternatives[0];
        assert_eq!(chain.segments.len(), 1);
        match &chain.segments[0].matcher {
            RecordMatcher::Designator(p) => assert_eq!(p.as_str(), "U1"),
            _ => panic!("Expected Designator"),
        }
    }

    #[test]
    fn test_parse_designator_wildcard() {
        let sel = parse("R*").unwrap();
        match &sel.alternatives[0].segments[0].matcher {
            RecordMatcher::Designator(p) => {
                assert_eq!(p.as_str(), "R*");
                assert!(!p.is_literal());
            }
            _ => panic!("Expected Designator"),
        }
    }

    #[test]
    fn test_parse_pin() {
        let sel = parse("U1:3").unwrap();
        match &sel.alternatives[0].segments[0].matcher {
            RecordMatcher::Pin { component, pin } => {
                assert_eq!(component.as_str(), "U1");
                assert_eq!(pin.as_str(), "3");
            }
            _ => panic!("Expected Pin"),
        }
    }

    #[test]
    fn test_parse_pin_by_name() {
        let sel = parse("U1:VCC").unwrap();
        match &sel.alternatives[0].segments[0].matcher {
            RecordMatcher::Pin { component, pin } => {
                assert_eq!(component.as_str(), "U1");
                assert_eq!(pin.as_str(), "VCC");
            }
            _ => panic!("Expected Pin"),
        }
    }

    #[test]
    fn test_parse_part_number() {
        let sel = parse("$LM358").unwrap();
        match &sel.alternatives[0].segments[0].matcher {
            RecordMatcher::PartNumber(p) => assert_eq!(p.as_str(), "LM358"),
            _ => panic!("Expected PartNumber"),
        }
    }

    #[test]
    fn test_parse_net() {
        let sel = parse("~VCC").unwrap();
        match &sel.alternatives[0].segments[0].matcher {
            RecordMatcher::Net(p) => assert_eq!(p.as_str(), "VCC"),
            _ => panic!("Expected Net"),
        }
    }

    #[test]
    fn test_parse_net_pins() {
        let sel = parse("~VCC:pins").unwrap();
        match &sel.alternatives[0].segments[0].matcher {
            RecordMatcher::NetConnected { net, target } => {
                assert_eq!(net.as_str(), "VCC");
                assert_eq!(*target, NetConnectedTarget::Pins);
            }
            _ => panic!("Expected NetConnected"),
        }
    }

    #[test]
    fn test_parse_value() {
        let sel = parse("@10K").unwrap();
        match &sel.alternatives[0].segments[0].matcher {
            RecordMatcher::Value(p) => assert_eq!(p.as_str(), "10K"),
            _ => panic!("Expected Value"),
        }
    }

    #[test]
    fn test_parse_sheet() {
        let sel = parse("#Power").unwrap();
        match &sel.alternatives[0].segments[0].matcher {
            RecordMatcher::Sheet(p) => assert_eq!(p.as_str(), "Power"),
            _ => panic!("Expected Sheet"),
        }
    }

    #[test]
    fn test_parse_designator_with_value() {
        let sel = parse("R*@10K").unwrap();
        match &sel.alternatives[0].segments[0].matcher {
            RecordMatcher::DesignatorWithValue { designator, value } => {
                assert_eq!(designator.as_str(), "R*");
                assert_eq!(value.as_str(), "10K");
            }
            _ => panic!("Expected DesignatorWithValue"),
        }
    }

    #[test]
    fn test_parse_type() {
        let sel = parse("component").unwrap();
        match &sel.alternatives[0].segments[0].matcher {
            RecordMatcher::Type(t) => assert_eq!(*t, RecordType::Component),
            _ => panic!("Expected Type"),
        }
    }

    #[test]
    fn test_parse_property_filter() {
        let sel = parse("U*[rotation=90]").unwrap();
        let segment = &sel.alternatives[0].segments[0];
        assert_eq!(segment.filters.len(), 1);
        assert_eq!(segment.filters[0].property, "rotation");
        assert!(matches!(segment.filters[0].operator, FilterOperator::Equal));
    }

    #[test]
    fn test_parse_pseudo_selector() {
        let sel = parse("U*:unconnected").unwrap();
        let segment = &sel.alternatives[0].segments[0];
        assert_eq!(segment.pseudo.len(), 1);
        assert!(matches!(segment.pseudo[0], PseudoSelector::Unconnected));
    }

    #[test]
    fn test_parse_union() {
        let sel = parse("R*, C*").unwrap();
        assert_eq!(sel.alternatives.len(), 2);
    }

    #[test]
    fn test_parse_child_combinator() {
        let sel = parse("U1 > pin").unwrap();
        let chain = &sel.alternatives[0];
        assert_eq!(chain.segments.len(), 2);
        assert!(matches!(
            chain.segments[0].combinator,
            Some(Combinator::DirectChild)
        ));
    }

    #[test]
    fn test_parse_slash_combinator() {
        let sel = parse("U1/pin").unwrap();
        let chain = &sel.alternatives[0];
        assert_eq!(chain.segments.len(), 2);
        assert!(matches!(
            chain.segments[0].combinator,
            Some(Combinator::DirectChild)
        ));
    }

    #[test]
    fn test_parse_descendant_combinator() {
        let sel = parse("U1 pin").unwrap();
        let chain = &sel.alternatives[0];
        assert_eq!(chain.segments.len(), 2);
        assert!(matches!(
            chain.segments[0].combinator,
            Some(Combinator::Descendant)
        ));
    }

    #[test]
    fn test_parse_complex() {
        let sel = parse("U*:output:unconnected").unwrap();
        let segment = &sel.alternatives[0].segments[0];
        match &segment.matcher {
            RecordMatcher::Designator(p) => assert_eq!(p.as_str(), "U*"),
            _ => panic!("Expected Designator"),
        }
        assert_eq!(segment.pseudo.len(), 2);
    }
}

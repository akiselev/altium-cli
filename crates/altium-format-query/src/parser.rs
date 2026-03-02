use crate::ast::*;
use crate::diagnostic::{Span, Spanned};
use crate::error::{QueryError, QueryErrorCode, QueryResult};
use crate::lexer::{Token, TokenKind, lex};

/// Parse a query string into a `Query` AST.
pub fn parse_query(input: &str) -> QueryResult<Query> {
    let tokens = lex(input)?;
    let mut parser = Parser::new(input, tokens);
    let expr = parser.parse_union()?;
    if parser.pos < parser.tokens.len() {
        let tok = &parser.tokens[parser.pos];
        return Err(QueryError::new(
            QueryErrorCode::UnexpectedToken,
            format!("unexpected token {:?}", tok.kind),
        )
        .with_span(tok.span));
    }
    Ok(Query { expr })
}

struct Parser<'a> {
    #[allow(dead_code)]
    source: &'a str,
    tokens: Vec<Token>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, tokens: Vec<Token>) -> Self {
        Self {
            source,
            tokens,
            pos: 0,
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    fn expect_kind(&mut self, expected: &TokenKind) -> QueryResult<Token> {
        match self.peek() {
            Some(tok) if tok.kind.same_variant(expected) => Ok(self.advance().unwrap()),
            Some(tok) => Err(QueryError::new(
                QueryErrorCode::UnexpectedToken,
                format!("expected {:?}, found {:?}", expected, tok.kind),
            )
            .with_span(tok.span)),
            None => Err(QueryError::new(
                QueryErrorCode::UnexpectedToken,
                format!("expected {:?}, found end of input", expected),
            )),
        }
    }

    /// Check if current token matches the given kind.
    fn check(&self, kind: &TokenKind) -> bool {
        self.peek().is_some_and(|t| t.kind.same_variant(kind))
    }

    // ── Grammar productions ──────────────────────────────────────────────

    /// union = or_expr ("," or_expr)*
    fn parse_union(&mut self) -> QueryResult<Spanned<QueryExpr>> {
        let first = self.parse_or()?;
        if !self.check(&TokenKind::Comma) {
            return Ok(first);
        }
        let mut branches = vec![first];
        while self.check(&TokenKind::Comma) {
            self.advance(); // consume ','
            branches.push(self.parse_or()?);
        }
        let span = branches
            .first()
            .unwrap()
            .span
            .merge(branches.last().unwrap().span);
        Ok(Spanned::new(QueryExpr::Union(branches), span))
    }

    /// or_expr = and_expr ("OR" and_expr)*
    fn parse_or(&mut self) -> QueryResult<Spanned<QueryExpr>> {
        let first = self.parse_and()?;
        if !self.check(&TokenKind::Or) {
            return Ok(first);
        }
        let mut branches = vec![first];
        while self.check(&TokenKind::Or) {
            self.advance(); // consume 'OR'
            branches.push(self.parse_and()?);
        }
        let span = branches
            .first()
            .unwrap()
            .span
            .merge(branches.last().unwrap().span);
        Ok(Spanned::new(QueryExpr::Or(branches), span))
    }

    /// and_expr = not_expr ("AND" not_expr)*
    fn parse_and(&mut self) -> QueryResult<Spanned<QueryExpr>> {
        let first = self.parse_not()?;
        if !self.check(&TokenKind::And) {
            return Ok(first);
        }
        let mut branches = vec![first];
        while self.check(&TokenKind::And) {
            self.advance(); // consume 'AND'
            branches.push(self.parse_not()?);
        }
        let span = branches
            .first()
            .unwrap()
            .span
            .merge(branches.last().unwrap().span);
        Ok(Spanned::new(QueryExpr::And(branches), span))
    }

    /// not_expr = "NOT" not_expr | selector_chain
    fn parse_not(&mut self) -> QueryResult<Spanned<QueryExpr>> {
        if self.check(&TokenKind::Not) {
            let not_tok = self.advance().unwrap();
            let inner = self.parse_not()?;
            let span = not_tok.span.merge(inner.span);
            return Ok(Spanned::new(QueryExpr::Not(Box::new(inner)), span));
        }
        self.parse_selector_chain()
    }

    /// selector_chain = compound_selector (combinator compound_selector)*
    fn parse_selector_chain(&mut self) -> QueryResult<Spanned<QueryExpr>> {
        if self.check(&TokenKind::LParen) {
            return self.parse_paren_group();
        }

        let first_sel = self.parse_compound_selector()?;
        let first_span = first_sel.span;
        let mut segments = vec![SelectorSegment {
            combinator: Combinator::None,
            selector: first_sel,
        }];

        loop {
            // Check for explicit child combinator `>`
            if self.check(&TokenKind::ChildCombinator) {
                self.advance();
                let sel = self.parse_compound_selector()?;
                let seg = SelectorSegment {
                    combinator: Combinator::Child,
                    selector: sel,
                };
                segments.push(seg);
                continue;
            }

            // Check for descendant combinator (implicit — next token starts a selector).
            // We only treat it as descendant if the next token can start a new selector
            // AND we're not about to see a logical operator or structural token.
            if self.can_start_selector() {
                let sel = self.parse_compound_selector()?;
                let seg = SelectorSegment {
                    combinator: Combinator::Descendant,
                    selector: sel,
                };
                segments.push(seg);
                continue;
            }

            break;
        }

        let last_span = segments.last().unwrap().selector.span;
        let span = first_span.merge(last_span);

        if segments.len() == 1 {
            let seg = segments.remove(0);
            // Unwrap single-segment chain into a plain selector
            Ok(Spanned::new(
                QueryExpr::Selector(SelectorChain { segments: vec![seg] }),
                span,
            ))
        } else {
            Ok(Spanned::new(
                QueryExpr::Selector(SelectorChain { segments }),
                span,
            ))
        }
    }

    /// Parenthesized group: "(" union ")"
    fn parse_paren_group(&mut self) -> QueryResult<Spanned<QueryExpr>> {
        let open = self.expect_kind(&TokenKind::LParen)?;
        let inner = self.parse_union()?;
        let close = self.expect_kind(&TokenKind::RParen).map_err(|e| {
            e.with_help("expected ')' to close parenthesized group")
        })?;
        // Preserve the inner expression but update span to include parens
        let span = open.span.merge(close.span);
        Ok(Spanned::new(inner.node, span))
    }

    /// Can the current token start a new compound selector?
    /// Used to detect implicit descendant combinators.
    fn can_start_selector(&self) -> bool {
        match self.peek() {
            None => false,
            Some(tok) => matches!(
                tok.kind,
                TokenKind::Ident(_)
                    | TokenKind::Dollar
                    | TokenKind::At
                    | TokenKind::Percent
                    | TokenKind::Hash
                    | TokenKind::Star
                    | TokenKind::LParen
                    | TokenKind::Colon  // standalone pseudo like `:power`
            ),
        }
    }

    /// compound_selector = base_selector attr_filter* pseudo_class*
    fn parse_compound_selector(&mut self) -> QueryResult<Spanned<CompoundSelector>> {
        let base = self.parse_base_selector()?;
        let start_span = base.span;

        let mut attrs = Vec::new();
        let mut pseudos = Vec::new();

        // Parse attribute filters: [field op value]
        while self.check(&TokenKind::LBracket) {
            attrs.push(self.parse_attribute_filter()?);
        }

        // Parse pseudo-classes: :name
        // But only if it's a known pseudo-class (not a component:pin pattern)
        while self.check(&TokenKind::Colon) && self.is_pseudo_class_ahead() {
            pseudos.push(self.parse_pseudo_class()?);
        }

        let end_span = pseudos
            .last()
            .map(|p| p.span)
            .or_else(|| attrs.last().map(|a| a.span))
            .unwrap_or(start_span);

        Ok(Spanned::new(
            CompoundSelector {
                base,
                attrs,
                pseudos,
            },
            start_span.merge(end_span),
        ))
    }

    /// Check if a `:` followed by an identifier is a pseudo-class (not component:pin).
    fn is_pseudo_class_ahead(&self) -> bool {
        if self.pos + 1 >= self.tokens.len() {
            return false;
        }
        if !matches!(self.tokens[self.pos].kind, TokenKind::Colon) {
            return false;
        }
        match &self.tokens[self.pos + 1].kind {
            TokenKind::Ident(name) => Self::is_known_pseudo(name),
            _ => false,
        }
    }

    fn is_known_pseudo(name: &str) -> bool {
        matches!(
            name.to_ascii_lowercase().as_str(),
            "power"
                | "input"
                | "output"
                | "io"
                | "passive"
                | "hiz"
                | "open-collector"
                | "open-emitter"
                | "virtual"
        )
    }

    fn parse_pseudo_class(&mut self) -> QueryResult<Spanned<PseudoClass>> {
        let colon = self.expect_kind(&TokenKind::Colon)?;
        let ident = self.expect_kind(&TokenKind::Ident(String::new()))?;
        let name = match &ident.kind {
            TokenKind::Ident(s) => s.clone(),
            _ => unreachable!(),
        };
        let span = colon.span.merge(ident.span);
        let pseudo = match name.to_ascii_lowercase().as_str() {
            "power" => PseudoClass::Power,
            "input" => PseudoClass::Input,
            "output" => PseudoClass::Output,
            "io" => PseudoClass::Io,
            "passive" => PseudoClass::Passive,
            "hiz" => PseudoClass::HiZ,
            "open-collector" => PseudoClass::OpenCollector,
            "open-emitter" => PseudoClass::OpenEmitter,
            "virtual" => PseudoClass::Virtual,
            _ => {
                return Err(QueryError::new(
                    QueryErrorCode::UnknownPseudoClass,
                    format!("unknown pseudo-class ':{name}'"),
                )
                .with_span(span)
                .with_help(
                    "known pseudo-classes: :power, :input, :output, :io, :passive, :hiz, :open-collector, :open-emitter, :virtual",
                ));
            }
        };
        Ok(Spanned::new(pseudo, span))
    }

    /// Parse attribute filter: `[field op value]`
    fn parse_attribute_filter(&mut self) -> QueryResult<Spanned<AttributeFilter>> {
        let open = self.expect_kind(&TokenKind::LBracket)?;

        // Parse field path (possibly dotted)
        let field = self.parse_field_path()?;

        // Parse comparison operator
        let op = self.parse_compare_op()?;

        // Parse value
        let value = self.parse_filter_value()?;

        let close = self.expect_kind(&TokenKind::RBracket).map_err(|e| {
            e.with_help("expected ']' to close attribute filter")
        })?;

        let span = open.span.merge(close.span);
        Ok(Spanned::new(
            AttributeFilter { field, op, value },
            span,
        ))
    }

    fn parse_field_path(&mut self) -> QueryResult<FieldPath> {
        let ident_tok = self.expect_kind(&TokenKind::Ident(String::new()))?;
        let first = match &ident_tok.kind {
            TokenKind::Ident(s) => s.clone(),
            _ => unreachable!(),
        };
        let start_span = ident_tok.span;

        // Check for dotted path: `field.name` or `param.Value`
        if self.check(&TokenKind::Dot) {
            self.advance(); // consume '.'
            let name_tok = self.expect_kind(&TokenKind::Ident(String::new()))?;
            let name = match &name_tok.kind {
                TokenKind::Ident(s) => s.clone(),
                _ => unreachable!(),
            };
            let span = start_span.merge(name_tok.span);
            Ok(FieldPath {
                prefix: Some(first),
                name,
                span,
            })
        } else {
            Ok(FieldPath {
                prefix: None,
                name: first,
                span: start_span,
            })
        }
    }

    fn parse_compare_op(&mut self) -> QueryResult<CompareOp> {
        let tok = self.peek().ok_or_else(|| {
            QueryError::new(
                QueryErrorCode::UnexpectedToken,
                "expected comparison operator, found end of input",
            )
        })?;
        let op = match &tok.kind {
            TokenKind::Eq => CompareOp::Eq,
            TokenKind::NotEq => CompareOp::NotEq,
            TokenKind::Contains => CompareOp::Contains,
            TokenKind::StartsWith => CompareOp::StartsWith,
            TokenKind::EndsWith => CompareOp::EndsWith,
            TokenKind::WordMatch => CompareOp::WordMatch,
            TokenKind::Gt => CompareOp::Gt,
            TokenKind::Lt => CompareOp::Lt,
            TokenKind::Gte => CompareOp::Gte,
            TokenKind::Lte => CompareOp::Lte,
            _ => {
                return Err(QueryError::new(
                    QueryErrorCode::UnexpectedToken,
                    format!("expected comparison operator, found {:?}", tok.kind),
                )
                .with_span(tok.span)
                .with_help("valid operators: =, !=, *=, ^=, $=, ~=, >, <, >=, <="));
            }
        };
        self.advance();
        Ok(op)
    }

    fn parse_filter_value(&mut self) -> QueryResult<Spanned<FilterValue>> {
        let tok = self.peek().ok_or_else(|| {
            QueryError::new(
                QueryErrorCode::UnexpectedToken,
                "expected value, found end of input",
            )
        })?;
        let span = tok.span;
        let value = match &tok.kind {
            TokenKind::String(s) => {
                let v = FilterValue::String(s.clone());
                self.advance();
                v
            }
            TokenKind::Integer(n) => {
                let v = FilterValue::Integer(*n);
                self.advance();
                v
            }
            TokenKind::Float(f) => {
                let v = FilterValue::Float(*f);
                self.advance();
                v
            }
            TokenKind::Bool(b) => {
                let v = FilterValue::Bool(*b);
                self.advance();
                v
            }
            TokenKind::Dim(val, unit) => {
                let v = FilterValue::Dim(*val, *unit);
                self.advance();
                v
            }
            TokenKind::Regex(pat) => {
                let v = FilterValue::Regex(pat.clone());
                self.advance();
                v
            }
            TokenKind::Ident(s) => {
                // Bare identifier used as string value (e.g., `[electrical=power]`)
                let v = FilterValue::Ident(s.clone());
                self.advance();

                // Check if there's a wildcard suffix making this a pattern
                // For filter values, wildcards aren't valid — they're just identifiers
                v
            }
            _ => {
                return Err(QueryError::new(
                    QueryErrorCode::UnexpectedToken,
                    format!("expected value, found {:?}", tok.kind),
                )
                .with_span(span));
            }
        };
        Ok(Spanned::new(value, span))
    }

    /// Parse the base selector.
    ///
    /// Disambiguation:
    /// - `*` alone → Any
    /// - `$ident` → PartNumber
    /// - `@ident` → ValuePattern
    /// - `%ident` → NetName
    /// - `#number` → RecordId
    /// - Known type keyword → TypeSelector
    /// - Identifier + `*`/`?` → DesignatorPattern
    /// - Identifier + `:` + non-pseudo identifier → ComponentPin
    /// - Identifier alone → DesignatorPattern(exact match) or TypeSelector
    fn parse_base_selector(&mut self) -> QueryResult<Spanned<BaseSelector>> {
        let tok = self.peek().ok_or_else(|| {
            QueryError::new(
                QueryErrorCode::ExpectedSelector,
                "expected selector, found end of input",
            )
        })?;
        let start_span = tok.span;

        match &tok.kind {
            // Universal selector
            TokenKind::Star => {
                self.advance();
                Ok(Spanned::new(BaseSelector::Any, start_span))
            }

            // Pattern prefixes
            TokenKind::Dollar => {
                self.advance();
                let name = self.parse_pattern_ident()?;
                let end_span = Span::new(start_span.start, (start_span.end as usize + name.len()) as u32);
                Ok(Spanned::new(
                    BaseSelector::PartNumber(name),
                    start_span.merge(end_span),
                ))
            }
            TokenKind::At => {
                self.advance();
                let name = self.parse_pattern_value()?;
                let end_span = self.tokens.get(self.pos.wrapping_sub(1))
                    .map(|t| t.span)
                    .unwrap_or(start_span);
                Ok(Spanned::new(
                    BaseSelector::ValuePattern(name),
                    start_span.merge(end_span),
                ))
            }
            TokenKind::Percent => {
                self.advance();
                let name = self.parse_pattern_ident()?;
                let end_span = self.tokens.get(self.pos.wrapping_sub(1))
                    .map(|t| t.span)
                    .unwrap_or(start_span);
                Ok(Spanned::new(
                    BaseSelector::NetName(name),
                    start_span.merge(end_span),
                ))
            }
            TokenKind::Hash => {
                self.advance();
                let id_tok = self.expect_kind(&TokenKind::Integer(0))?;
                let id = match id_tok.kind {
                    TokenKind::Integer(n) => n,
                    _ => unreachable!(),
                };
                Ok(Spanned::new(
                    BaseSelector::RecordId(id),
                    start_span.merge(id_tok.span),
                ))
            }

            // Standalone pseudo-class (`:power` without a type prefix)
            TokenKind::Colon => {
                // Synthesize as type Any + pseudo-class — but we handle this
                // at compound_selector level. If we reach here, treat `:` as
                // starting an Any selector (the pseudo will be parsed after).
                Ok(Spanned::new(BaseSelector::Any, Span::new(start_span.start, start_span.start)))
            }

            // Identifier: could be type selector, designator pattern, or component:pin
            TokenKind::Ident(name) => {
                let name = name.clone();
                let ident_span = tok.span;

                // Check if this is a known type keyword
                if let Some(ts) = TypeSelector::from_keyword(&name) {
                    self.advance();

                    // But check for component:pin pattern first
                    // A type keyword followed by `:` and a non-pseudo ident is ambiguous.
                    // We resolve: known type + `:pseudo` → type + pseudo-class
                    // This is handled by the compound_selector — pseudo parsing happens there.
                    return Ok(Spanned::new(BaseSelector::Type(ts), ident_span));
                }

                // Not a type keyword — designator pattern or component:pin
                self.advance();

                // Check for wildcard suffix
                if self.check(&TokenKind::Star) {
                    let star = self.advance().unwrap();
                    return Ok(Spanned::new(
                        BaseSelector::DesignatorPattern(DesignatorPattern {
                            prefix: name,
                            wildcard: Wildcard::Star,
                        }),
                        ident_span.merge(star.span),
                    ));
                }
                if self.check(&TokenKind::Question) {
                    let mut count = 0usize;
                    let mut end = ident_span;
                    while self.check(&TokenKind::Question) {
                        end = self.advance().unwrap().span;
                        count += 1;
                    }
                    return Ok(Spanned::new(
                        BaseSelector::DesignatorPattern(DesignatorPattern {
                            prefix: name,
                            wildcard: Wildcard::Fixed(count),
                        }),
                        ident_span.merge(end),
                    ));
                }

                // Check for component:pin pattern (ident:ident where the second is not a pseudo)
                if self.check(&TokenKind::Colon) && !self.is_pseudo_class_ahead() {
                    self.advance(); // consume ':'
                    let pin_tok = self.expect_kind(&TokenKind::Ident(String::new()))?;
                    let pin_name = match &pin_tok.kind {
                        TokenKind::Ident(s) => s.clone(),
                        _ => unreachable!(),
                    };
                    return Ok(Spanned::new(
                        BaseSelector::ComponentPin {
                            component: name,
                            pin: pin_name,
                        },
                        ident_span.merge(pin_tok.span),
                    ));
                }

                // Plain identifier — exact designator match
                Ok(Spanned::new(
                    BaseSelector::DesignatorPattern(DesignatorPattern {
                        prefix: name,
                        wildcard: Wildcard::None,
                    }),
                    ident_span,
                ))
            }

            _ => {
                Err(QueryError::new(
                    QueryErrorCode::ExpectedSelector,
                    format!("expected selector, found {:?}", tok.kind),
                )
                .with_span(start_span))
            }
        }
    }

    /// Parse a pattern identifier after a prefix ($, ~).
    /// Allows alphanumeric identifiers with wildcards.
    fn parse_pattern_ident(&mut self) -> QueryResult<String> {
        let tok = self.peek().ok_or_else(|| {
            QueryError::new(
                QueryErrorCode::ExpectedSelector,
                "expected identifier after pattern prefix",
            )
        })?;
        match &tok.kind {
            TokenKind::Ident(s) => {
                let s = s.clone();
                self.advance();
                // Consume wildcards as part of the pattern
                let mut result = s;
                while self.check(&TokenKind::Star) || self.check(&TokenKind::Question) {
                    match self.advance().unwrap().kind {
                        TokenKind::Star => result.push('*'),
                        TokenKind::Question => result.push('?'),
                        _ => unreachable!(),
                    }
                }
                Ok(result)
            }
            _ => Err(QueryError::new(
                QueryErrorCode::ExpectedSelector,
                format!("expected identifier, found {:?}", tok.kind),
            )
            .with_span(tok.span)),
        }
    }

    /// Parse a value after `@` (can be ident, string, or number-like token).
    fn parse_pattern_value(&mut self) -> QueryResult<String> {
        let tok = self.peek().ok_or_else(|| {
            QueryError::new(
                QueryErrorCode::ExpectedSelector,
                "expected value after '@'",
            )
        })?;
        match &tok.kind {
            TokenKind::Ident(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            TokenKind::Integer(n) => {
                let s = n.to_string();
                self.advance();
                // Check if followed by ident (e.g., @100nF → "100" + "nF")
                if let Some(Token { kind: TokenKind::Ident(suffix), .. }) = self.peek() {
                    let result = format!("{s}{suffix}");
                    self.advance();
                    return Ok(result);
                }
                Ok(s)
            }
            _ => Err(QueryError::new(
                QueryErrorCode::ExpectedSelector,
                format!("expected value after '@', found {:?}", tok.kind),
            )
            .with_span(tok.span)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_type_selector() {
        let q = parse_query("component").unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                assert_eq!(chain.segments.len(), 1);
                match &chain.segments[0].selector.node.base.node {
                    BaseSelector::Type(TypeSelector::Component) => {}
                    other => panic!("expected Type(Component), got {other:?}"),
                }
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_designator_pattern_star() {
        let q = parse_query("R*").unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                match &chain.segments[0].selector.node.base.node {
                    BaseSelector::DesignatorPattern(dp) => {
                        assert_eq!(dp.prefix, "R");
                        assert_eq!(dp.wildcard, Wildcard::Star);
                    }
                    other => panic!("expected DesignatorPattern, got {other:?}"),
                }
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_designator_pattern_question() {
        let q = parse_query("C??").unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                match &chain.segments[0].selector.node.base.node {
                    BaseSelector::DesignatorPattern(dp) => {
                        assert_eq!(dp.prefix, "C");
                        assert_eq!(dp.wildcard, Wildcard::Fixed(2));
                    }
                    other => panic!("expected DesignatorPattern, got {other:?}"),
                }
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_part_number() {
        let q = parse_query("$LM358").unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                match &chain.segments[0].selector.node.base.node {
                    BaseSelector::PartNumber(name) => assert_eq!(name, "LM358"),
                    other => panic!("expected PartNumber, got {other:?}"),
                }
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_value_pattern() {
        let q = parse_query("@10K").unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                match &chain.segments[0].selector.node.base.node {
                    BaseSelector::ValuePattern(v) => assert_eq!(v, "10K"),
                    other => panic!("expected ValuePattern, got {other:?}"),
                }
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_net_name() {
        let q = parse_query("%VCC").unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                match &chain.segments[0].selector.node.base.node {
                    BaseSelector::NetName(name) => assert_eq!(name, "VCC"),
                    other => panic!("expected NetName, got {other:?}"),
                }
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_record_id() {
        let q = parse_query("#42").unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                match &chain.segments[0].selector.node.base.node {
                    BaseSelector::RecordId(id) => assert_eq!(*id, 42),
                    other => panic!("expected RecordId, got {other:?}"),
                }
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_component_pin() {
        let q = parse_query("U1:VCC").unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                match &chain.segments[0].selector.node.base.node {
                    BaseSelector::ComponentPin { component, pin } => {
                        assert_eq!(component, "U1");
                        assert_eq!(pin, "VCC");
                    }
                    other => panic!("expected ComponentPin, got {other:?}"),
                }
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_attribute_filter() {
        let q = parse_query(r#"component[value="10K"]"#).unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                let sel = &chain.segments[0].selector.node;
                assert!(matches!(sel.base.node, BaseSelector::Type(TypeSelector::Component)));
                assert_eq!(sel.attrs.len(), 1);
                let attr = &sel.attrs[0].node;
                assert_eq!(attr.field.name, "value");
                assert!(matches!(attr.op, CompareOp::Eq));
                assert!(matches!(&attr.value.node, FilterValue::String(s) if s == "10K"));
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_multiple_attrs() {
        let q = parse_query("component[x>100][y<200]").unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                let sel = &chain.segments[0].selector.node;
                assert_eq!(sel.attrs.len(), 2);
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_pseudo_class() {
        let q = parse_query("pin:power").unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                let sel = &chain.segments[0].selector.node;
                assert!(matches!(sel.base.node, BaseSelector::Type(TypeSelector::Pin)));
                assert_eq!(sel.pseudos.len(), 1);
                assert_eq!(sel.pseudos[0].node, PseudoClass::Power);
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_child_combinator() {
        let q = parse_query("component > pin").unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                assert_eq!(chain.segments.len(), 2);
                assert_eq!(chain.segments[0].combinator, Combinator::None);
                assert_eq!(chain.segments[1].combinator, Combinator::Child);
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_descendant_combinator() {
        let q = parse_query("component pin").unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                assert_eq!(chain.segments.len(), 2);
                assert_eq!(chain.segments[0].combinator, Combinator::None);
                assert_eq!(chain.segments[1].combinator, Combinator::Descendant);
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_union() {
        let q = parse_query("R*, C*").unwrap();
        assert!(matches!(q.expr.node, QueryExpr::Union(_)));
    }

    #[test]
    fn test_parse_or() {
        let q = parse_query("R* OR C*").unwrap();
        assert!(matches!(q.expr.node, QueryExpr::Or(_)));
    }

    #[test]
    fn test_parse_and() {
        let q = parse_query("component AND pin:power").unwrap();
        assert!(matches!(q.expr.node, QueryExpr::And(_)));
    }

    #[test]
    fn test_parse_not() {
        let q = parse_query("NOT pin:power").unwrap();
        assert!(matches!(q.expr.node, QueryExpr::Not(_)));
    }

    #[test]
    fn test_parse_precedence() {
        // AND binds tighter than OR
        let q = parse_query("R* OR C* AND pin:power").unwrap();
        match &q.expr.node {
            QueryExpr::Or(branches) => {
                assert_eq!(branches.len(), 2);
                // Second branch should be AND
                assert!(matches!(branches[1].node, QueryExpr::And(_)));
            }
            other => panic!("expected Or, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_paren_group() {
        let q = parse_query("(R* OR C*) AND pin:power").unwrap();
        match &q.expr.node {
            QueryExpr::And(branches) => {
                assert_eq!(branches.len(), 2);
                // First branch should be OR (grouped by parens)
                assert!(matches!(branches[0].node, QueryExpr::Or(_)));
            }
            other => panic!("expected And, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_universal_selector() {
        let q = parse_query("*").unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                assert!(matches!(chain.segments[0].selector.node.base.node, BaseSelector::Any));
            }
            other => panic!("expected Selector(Any), got {other:?}"),
        }
    }

    #[test]
    fn test_parse_complex_query() {
        // component[value="10K"] > pin:power
        let q = parse_query(r#"component[value="10K"] > pin:power"#).unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                assert_eq!(chain.segments.len(), 2);
                // First: component[value="10K"]
                let first = &chain.segments[0].selector.node;
                assert!(matches!(first.base.node, BaseSelector::Type(TypeSelector::Component)));
                assert_eq!(first.attrs.len(), 1);
                // Second: pin:power (child combinator)
                assert_eq!(chain.segments[1].combinator, Combinator::Child);
                let second = &chain.segments[1].selector.node;
                assert!(matches!(second.base.node, BaseSelector::Type(TypeSelector::Pin)));
                assert_eq!(second.pseudos.len(), 1);
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_dotted_field() {
        let q = parse_query(r#"component[param.Value="10K"]"#).unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                let attr = &chain.segments[0].selector.node.attrs[0].node;
                assert_eq!(attr.field.prefix.as_deref(), Some("param"));
                assert_eq!(attr.field.name, "Value");
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_dimensional_value() {
        let q = parse_query("track[width>=10mil]").unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                let attr = &chain.segments[0].selector.node.attrs[0].node;
                assert!(matches!(attr.value.node, FilterValue::Dim(10.0, crate::diagnostic::Unit::Mil)));
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_regex_value() {
        let q = parse_query(r#"component[designator=/^U[0-9]+$/]"#).unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                let attr = &chain.segments[0].selector.node.attrs[0].node;
                assert!(matches!(&attr.value.node, FilterValue::Regex(p) if p == "^U[0-9]+$"));
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_standalone_pseudo() {
        // `:power` without a type prefix
        let q = parse_query(":power").unwrap();
        match &q.expr.node {
            QueryExpr::Selector(chain) => {
                let sel = &chain.segments[0].selector.node;
                assert!(matches!(sel.base.node, BaseSelector::Any));
                assert_eq!(sel.pseudos.len(), 1);
                assert_eq!(sel.pseudos[0].node, PseudoClass::Power);
            }
            other => panic!("expected Selector, got {other:?}"),
        }
    }
}

use crate::diagnostic::{BinOp, ParseError, ParseErrorCode, Span, Spanned};

use super::ast::{
    AliasDecl, AnnotationBlockDecl, BoardDecl, BoardItem, ClassDecl, ComparisonRuleDecl,
    ComponentDecl, ComponentItem, DifferentialPairDecl, DocumentBlockDecl, EntityName, EntryDecl,
    ErcLevelEntryDecl, ErcMatrixEntryDecl, Expr, FontBlockDecl, FontDecl, FootprintDecl,
    FootprintItem, FootprintMapDecl, FootprintRef, GraphicDecl, GridDecl, ImportDecl, LetBinding,
    MapEntry, MatchParameterDecl, NetDecl, Object, ObjectItem, OutputBlockDecl,
    OutputGroupBlockDecl, PadDecl, ParamVariationDecl, ParameterDecl, PartBlock, PartItem,
    PcbDocPrimitiveDecl, PinDecl, PlaceDecl, PlacementConstraintDecl, PlacementDecl,
    PlacementGroupDecl, PlacementItem, PlacementSeparateDecl, PolygonDecl, PowerDecl, ProjectDecl,
    ProjectItem, Property, RowDecl, RuleDecl, SchDocObjectDecl, SchDocObjectItem, SheetDecl,
    SheetItem, SpecFile, SpecItem, SwapGroupDecl, VariantBlockDecl, VariationDecl,
    is_graphic_type, is_pcbdoc_block_type, is_pcbdoc_primitive_type, is_schdoc_object_type,
};
use super::lexer::{Token, TokenKind, lex};

/// Parse a spec file source string into an AST.
pub fn parse_spec(source: &str) -> Result<SpecFile, ParseError> {
    let tokens = lex(source)?;
    let mut parser = SpecParser::new(source, tokens);
    parser.parse_file()
}

#[allow(dead_code)]
struct SpecParser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    pos: usize,
}

impl<'a> SpecParser<'a> {
    fn new(source: &'a str, tokens: Vec<Token>) -> Self {
        Self {
            source,
            tokens,
            pos: 0,
        }
    }

    // ── Token navigation ──────────────────────────────────────────────────────

    #[allow(dead_code)]
    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn current_kind(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn current_span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn prev_span(&self) -> Span {
        if self.pos > 0 {
            self.tokens[self.pos - 1].span
        } else {
            Span::new(0, 0)
        }
    }

    fn peek_ahead(&self, offset: usize) -> &TokenKind {
        let idx = (self.pos + offset).min(self.tokens.len() - 1);
        &self.tokens[idx].kind
    }

    fn at_eof(&self) -> bool {
        matches!(self.current_kind(), TokenKind::Eof)
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.current_kind().same_variant(kind)
    }

    fn bump(&mut self) -> &Token {
        let t = &self.tokens[self.pos];
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind, msg: &str) -> Result<Span, ParseError> {
        if self.at(kind) {
            let span = self.current_span();
            self.bump();
            Ok(span)
        } else {
            Err(self.err(msg))
        }
    }

    fn err(&self, msg: impl Into<String>) -> ParseError {
        ParseError::new(ParseErrorCode::E1002, msg.into(), self.current_span())
    }

    // ── Separator / newline handling ─────────────────────────────────────────

    fn skip_newlines(&mut self) {
        while self.eat(&TokenKind::Newline) || self.eat(&TokenKind::Semi) {}
    }

    fn skip_separators(&mut self) {
        while self.eat(&TokenKind::Newline)
            || self.eat(&TokenKind::Comma)
            || self.eat(&TokenKind::Semi)
        {}
    }

    fn eat_separator(&mut self) -> bool {
        if self.eat(&TokenKind::Comma) || self.eat(&TokenKind::Newline) || self.eat(&TokenKind::Semi) {
            self.skip_separators();
            true
        } else {
            false
        }
    }

    // ── Binding prefix detection ─────────────────────────────────────────────

    /// Try to parse [let] IDENT "=". Rewind on failure.
    /// Used to detect binding prefixes before entity keywords.
    #[allow(dead_code)]
    fn try_parse_binding_prefix(&mut self) -> Option<Spanned<String>> {
        let save = self.pos;
        self.eat(&TokenKind::Let); // skip optional `let`
        if let TokenKind::Ident(name) = self.current_kind().clone() {
            let name_span = self.current_span();
            self.bump();
            if self.eat(&TokenKind::Eq) {
                return Some(Spanned::new(name, name_span));
            }
        }
        self.pos = save;
        None
    }

    // ── Identifier helpers ────────────────────────────────────────────────────

    fn expect_ident(&mut self, msg: &str) -> Result<Spanned<String>, ParseError> {
        match self.current_kind().clone() {
            TokenKind::Ident(s) => {
                let span = self.current_span();
                self.bump();
                Ok(Spanned::new(s, span))
            }
            _ => Err(self.err(msg)),
        }
    }

    fn expect_string(&mut self, msg: &str) -> Result<Spanned<String>, ParseError> {
        match self.current_kind().clone() {
            TokenKind::String(s) => {
                let span = self.current_span();
                self.bump();
                Ok(Spanned::new(s, span))
            }
            _ => Err(self.err(msg)),
        }
    }

    fn expect_integer(&mut self, msg: &str) -> Result<Spanned<i32>, ParseError> {
        match self.current_kind().clone() {
            TokenKind::Integer(n) => {
                let span = self.current_span();
                self.bump();
                Ok(Spanned::new(n, span))
            }
            _ => Err(self.err(msg)),
        }
    }

    // ── File-level parsing ────────────────────────────────────────────────────

    fn parse_file(&mut self) -> Result<SpecFile, ParseError> {
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.at_eof() {
            items.push(self.parse_spec_item()?);
            self.skip_separators();
        }
        Ok(SpecFile { items })
    }

    fn parse_spec_item(&mut self) -> Result<Spanned<SpecItem>, ParseError> {
        let start = self.current_span();

        // Handle: import
        if self.at(&TokenKind::Import) {
            let decl = self.parse_import()?;
            let end = self.prev_span();
            return Ok(Spanned::new(SpecItem::Import(decl), start.merge(end)));
        }

        // Handle: component ...
        if self.at(&TokenKind::Component) {
            let decl = self.parse_component(None)?;
            let end = self.prev_span();
            return Ok(Spanned::new(SpecItem::Component(decl), start.merge(end)));
        }

        // Handle: footprint ...
        if self.at(&TokenKind::Footprint) {
            let decl = self.parse_footprint(None)?;
            let end = self.prev_span();
            return Ok(Spanned::new(SpecItem::Footprint(decl), start.merge(end)));
        }

        // Handle: project ...
        if self.at(&TokenKind::Project) {
            let decl = self.parse_project(None)?;
            let end = self.prev_span();
            return Ok(Spanned::new(SpecItem::Project(decl), start.merge(end)));
        }

        // Handle: sheet { ... } (SchDoc metadata)
        if self.at(&TokenKind::Sheet) {
            let decl = self.parse_sheet()?;
            let end = self.prev_span();
            return Ok(Spanned::new(SpecItem::Sheet(decl), start.merge(end)));
        }

        // Handle: net NAME { ... }
        if self.at(&TokenKind::Net) {
            let decl = self.parse_net()?;
            let end = self.prev_span();
            return Ok(Spanned::new(SpecItem::Net(decl), start.merge(end)));
        }

        // Handle: power NAME { ... }
        if self.at(&TokenKind::Power) {
            let decl = self.parse_power()?;
            let end = self.prev_span();
            return Ok(Spanned::new(SpecItem::Power(decl), start.merge(end)));
        }

        // Handle: board NAME { ... } (PcbDoc)
        if self.at(&TokenKind::Board) {
            let decl = self.parse_board()?;
            let end = self.prev_span();
            return Ok(Spanned::new(SpecItem::Board(decl), start.merge(end)));
        }

        // Handle: pad NAME { ... } at top level (PcbDoc pad primitive)
        if self.at(&TokenKind::Pad) {
            let decl = self.parse_pcbdoc_primitive_from_keyword("pad")?;
            let end = self.prev_span();
            return Ok(Spanned::new(SpecItem::PcbDocPrimitive(decl), start.merge(end)));
        }

        // Handle: parameter NAME { ... } at top level (SchDoc sheet-level parameter)
        // `parameter` is a keyword, but in SchDoc context it also appears as a
        // freestanding top-level object (like `wire`, `bus`, etc.).
        if self.at(&TokenKind::Parameter) {
            let decl = self.parse_schdoc_object_keyword("parameter")?;
            let end = self.prev_span();
            return Ok(Spanned::new(SpecItem::SchDocObject(decl), start.merge(end)));
        }

        // Handle: swap_group NAME { ... }
        // Distinguish from a bare property key (which doesn't appear at top level, but handle gracefully).
        if self.at(&TokenKind::SwapGroup) && !self.peek_ahead(1).same_variant(&TokenKind::Colon) {
            let decl = self.parse_swap_group_decl(None)?;
            let end = self.prev_span();
            return Ok(Spanned::new(SpecItem::SwapGroup(decl), start.merge(end)));
        }

        // Skip optional `let`
        let had_let = self.eat(&TokenKind::Let);

        // Look for IDENT =
        if let TokenKind::Ident(_) = self.current_kind() {
            // Peek ahead: IDENT "=" ...
            if self.peek_ahead(1).same_variant(&TokenKind::Eq) {
                let name_span = self.current_span();
                let name = match self.current_kind().clone() {
                    TokenKind::Ident(s) => s,
                    _ => unreachable!(),
                };
                self.bump(); // consume IDENT
                self.bump(); // consume =

                // Check what follows the "="
                match self.current_kind().clone() {
                    TokenKind::Component => {
                        let binding = Some(Spanned::new(name, name_span));
                        let decl = self.parse_component(binding)?;
                        let end = self.prev_span();
                        return Ok(Spanned::new(SpecItem::Component(decl), start.merge(end)));
                    }
                    TokenKind::Footprint => {
                        let binding = Some(Spanned::new(name, name_span));
                        let decl = self.parse_footprint(binding)?;
                        let end = self.prev_span();
                        return Ok(Spanned::new(SpecItem::Footprint(decl), start.merge(end)));
                    }
                    TokenKind::Project => {
                        let binding = Some(Spanned::new(name, name_span));
                        let decl = self.parse_project(binding)?;
                        let end = self.prev_span();
                        return Ok(Spanned::new(SpecItem::Project(decl), start.merge(end)));
                    }
                    TokenKind::SwapGroup => {
                        let binding = Some(Spanned::new(name, name_span));
                        let decl = self.parse_swap_group_decl(binding)?;
                        let end = self.prev_span();
                        return Ok(Spanned::new(SpecItem::SwapGroup(decl), start.merge(end)));
                    }
                    _ => {
                        // It's a let binding
                        let value = self.parse_expr()?;
                        let end = self.prev_span();
                        let binding = LetBinding {
                            name: Spanned::new(name, name_span),
                            value,
                        };
                        return Ok(Spanned::new(
                            SpecItem::LetBinding(binding),
                            start.merge(end),
                        ));
                    }
                }
            }
        }

        if had_let {
            return Err(self.err("expected identifier after 'let'"));
        }

        // placement { ... } top-level block.
        if let TokenKind::Ident(ref name) = self.current_kind().clone() {
            if name == "placement" {
                let decl = self.parse_placement()?;
                let end = self.prev_span();
                return Ok(Spanned::new(SpecItem::Placement(decl), start.merge(end)));
            }
        }

        // PcbDoc block types (polygon, rule, class, differential_pair) — checked before
        // SchDoc types to avoid conflicts (polygon exists in both).
        if let TokenKind::Ident(ref name) = self.current_kind().clone() {
            if is_pcbdoc_block_type(name) {
                return self.parse_pcbdoc_named_block(start);
            }
            if is_pcbdoc_primitive_type(name) {
                let decl = self.parse_pcbdoc_primitive()?;
                let end = self.prev_span();
                return Ok(Spanned::new(SpecItem::PcbDocPrimitive(decl), start.merge(end)));
            }
        }

        // SchDoc object types and graphics as top-level identifier-dispatched blocks
        if let TokenKind::Ident(ref name) = self.current_kind().clone() {
            if is_schdoc_object_type(name) || is_graphic_type(name) {
                let decl = self.parse_schdoc_object()?;
                let end = self.prev_span();
                return Ok(Spanned::new(SpecItem::SchDocObject(decl), start.merge(end)));
            }
        }

        Err(self.err("expected import, component, footprint, project, sheet, net, power, board, placement, or let binding"))
    }

    // ── Import ─────────────────────────────────────────────────────────────

    fn parse_import(&mut self) -> Result<ImportDecl, ParseError> {
        self.expect(&TokenKind::Import, "expected 'import'")?;
        let path = self.expect_string("expected file path string after 'import'")?;
        let alias = if self.eat(&TokenKind::As) {
            Some(self.expect_ident("expected import alias identifier after 'as'")?)
        } else {
            None
        };
        Ok(super::ast::ImportDecl { path, alias })
    }

    // ── SwapGroup ──────────────────────────────────────────────────────────

    fn parse_swap_group_decl(
        &mut self,
        binding: Option<Spanned<String>>,
    ) -> Result<SwapGroupDecl, ParseError> {
        self.expect(&TokenKind::SwapGroup, "expected 'swap_group'")?;
        let name = self.parse_entity_name()?;
        self.skip_newlines();
        let body = self.parse_object()?;
        Ok(SwapGroupDecl { binding, name, body })
    }

    // ── Component ─────────────────────────────────────────────────────────

    fn parse_component(
        &mut self,
        binding: Option<Spanned<String>>,
    ) -> Result<ComponentDecl, ParseError> {
        self.expect(&TokenKind::Component, "expected 'component'")?;
        let name = self.parse_entity_name()?;
        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after component name")?;
        let body = self.parse_component_body()?;
        self.expect(&TokenKind::RBrace, "expected '}' to close component body")?;
        Ok(ComponentDecl { binding, name, body })
    }

    fn parse_component_body(&mut self) -> Result<Vec<Spanned<ComponentItem>>, ParseError> {
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let item = self.parse_component_item()?;
            items.push(item);
            self.skip_separators();
        }
        Ok(items)
    }

    fn parse_component_item(&mut self) -> Result<Spanned<ComponentItem>, ParseError> {
        let start = self.current_span();

        // part block
        if self.at(&TokenKind::Part) {
            let block = self.parse_part_block(None)?;
            let end = self.prev_span();
            return Ok(Spanned::new(ComponentItem::Part(block), start.merge(end)));
        }

        // pin declaration
        if self.at(&TokenKind::Pin) {
            let decl = self.parse_pin(None)?;
            let end = self.prev_span();
            return Ok(Spanned::new(ComponentItem::Pin(decl), start.merge(end)));
        }

        // parameter declaration
        if self.at(&TokenKind::Parameter) {
            let decl = self.parse_parameter(None)?;
            let end = self.prev_span();
            return Ok(Spanned::new(ComponentItem::Parameter(decl), start.merge(end)));
        }

        // alias declaration
        if self.at(&TokenKind::Alias) {
            let decl = self.parse_alias()?;
            let end = self.prev_span();
            return Ok(Spanned::new(ComponentItem::Alias(decl), start.merge(end)));
        }

        // footprint map
        if self.at(&TokenKind::Footprint) {
            let decl = self.parse_footprint_map()?;
            let end = self.prev_span();
            return Ok(Spanned::new(
                ComponentItem::FootprintMap(decl),
                start.merge(end),
            ));
        }

        // let binding at component scope
        if self.at(&TokenKind::Let) {
            let binding = self.parse_let_binding()?;
            let end = self.prev_span();
            return Ok(Spanned::new(
                ComponentItem::LetBinding(binding),
                start.merge(end),
            ));
        }

        // swap_group at component scope: either a declaration or a property.
        // swap_group NAME { ... }  → declaration
        // swap_group: $ref         → property (e.g. on a part body handled via parse_part_item)
        if self.at(&TokenKind::SwapGroup) {
            if self.peek_ahead(1).same_variant(&TokenKind::Colon) {
                // It's a property: swap_group: value
                let prop = self.parse_property()?;
                let end = self.prev_span();
                return Ok(Spanned::new(ComponentItem::Property(prop), start.merge(end)));
            }
            let decl = self.parse_swap_group_decl(None)?;
            let end = self.prev_span();
            return Ok(Spanned::new(ComponentItem::SwapGroup(decl), start.merge(end)));
        }

        // IDENT-led items: property, let binding, or graphic / bound entity
        if let TokenKind::Ident(name) = self.current_kind().clone() {
            let name_span = self.current_span();

            // Check for IDENT "=" or IDENT ":"
            let next = self.peek_ahead(1);

            if next.same_variant(&TokenKind::Colon) {
                // property: key: value
                let prop = self.parse_property()?;
                let end = self.prev_span();
                return Ok(Spanned::new(
                    ComponentItem::Property(prop),
                    start.merge(end),
                ));
            }

            if next.same_variant(&TokenKind::Eq) {
                // Could be: binding = component_item_keyword, or binding = expr (let binding)
                self.bump(); // consume IDENT
                self.bump(); // consume =

                match self.current_kind().clone() {
                    TokenKind::Pin => {
                        let decl =
                            self.parse_pin(Some(Spanned::new(name, name_span)))?;
                        let end = self.prev_span();
                        return Ok(Spanned::new(ComponentItem::Pin(decl), start.merge(end)));
                    }
                    TokenKind::Parameter => {
                        let decl =
                            self.parse_parameter(Some(Spanned::new(name, name_span)))?;
                        let end = self.prev_span();
                        return Ok(Spanned::new(
                            ComponentItem::Parameter(decl),
                            start.merge(end),
                        ));
                    }
                    TokenKind::Part => {
                        let block =
                            self.parse_part_block(Some(Spanned::new(name, name_span)))?;
                        let end = self.prev_span();
                        return Ok(Spanned::new(ComponentItem::Part(block), start.merge(end)));
                    }
                    TokenKind::Ident(ref graphic) if is_graphic_type(graphic) => {
                        let decl = self.parse_graphic(Some(Spanned::new(name, name_span)))?;
                        let end = self.prev_span();
                        return Ok(Spanned::new(
                            ComponentItem::Graphic(decl),
                            start.merge(end),
                        ));
                    }
                    TokenKind::SwapGroup => {
                        let decl =
                            self.parse_swap_group_decl(Some(Spanned::new(name, name_span)))?;
                        let end = self.prev_span();
                        return Ok(Spanned::new(
                            ComponentItem::SwapGroup(decl),
                            start.merge(end),
                        ));
                    }
                    _ => {
                        // It's a let binding: name = expr
                        let value = self.parse_expr()?;
                        let end = self.prev_span();
                        let binding = LetBinding {
                            name: Spanned::new(name, name_span),
                            value,
                        };
                        return Ok(Spanned::new(
                            ComponentItem::LetBinding(binding),
                            start.merge(end),
                        ));
                    }
                }
            }

            // Graphic type identifier (no binding prefix)
            if is_graphic_type(&name) {
                let decl = self.parse_graphic(None)?;
                let end = self.prev_span();
                return Ok(Spanned::new(ComponentItem::Graphic(decl), start.merge(end)));
            }
        }

        Err(self.err(
            "expected component item (property, pin, parameter, alias, footprint, part, graphic, swap_group, or let binding)",
        ))
    }

    // ── Part block ─────────────────────────────────────────────────────────

    fn parse_part_block(
        &mut self,
        binding: Option<Spanned<String>>,
    ) -> Result<PartBlock, ParseError> {
        self.expect(&TokenKind::Part, "expected 'part'")?;
        let number = self.expect_integer("expected integer part number after 'part'")?;
        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after part number")?;
        let body = self.parse_part_body()?;
        self.expect(&TokenKind::RBrace, "expected '}' to close part body")?;
        Ok(PartBlock { binding, number, body })
    }

    fn parse_part_body(&mut self) -> Result<Vec<Spanned<PartItem>>, ParseError> {
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let item = self.parse_part_item()?;
            items.push(item);
            self.skip_separators();
        }
        Ok(items)
    }

    fn parse_part_item(&mut self) -> Result<Spanned<PartItem>, ParseError> {
        let start = self.current_span();

        if self.at(&TokenKind::Pin) {
            let decl = self.parse_pin(None)?;
            let end = self.prev_span();
            return Ok(Spanned::new(PartItem::Pin(decl), start.merge(end)));
        }

        if self.at(&TokenKind::Let) {
            let binding = self.parse_let_binding()?;
            let end = self.prev_span();
            return Ok(Spanned::new(PartItem::LetBinding(binding), start.merge(end)));
        }

        // swap_group: $ref inside a part block is a property.
        if self.at(&TokenKind::SwapGroup) && self.peek_ahead(1).same_variant(&TokenKind::Colon) {
            let prop = self.parse_property()?;
            let end = self.prev_span();
            return Ok(Spanned::new(PartItem::Property(prop), start.merge(end)));
        }

        if let TokenKind::Ident(name) = self.current_kind().clone() {
            let name_span = self.current_span();
            let next = self.peek_ahead(1);

            if next.same_variant(&TokenKind::Eq) {
                self.bump(); // IDENT
                self.bump(); // =
                match self.current_kind().clone() {
                    TokenKind::Pin => {
                        let decl = self.parse_pin(Some(Spanned::new(name, name_span)))?;
                        let end = self.prev_span();
                        return Ok(Spanned::new(PartItem::Pin(decl), start.merge(end)));
                    }
                    TokenKind::Ident(ref graphic) if is_graphic_type(graphic) => {
                        let decl =
                            self.parse_graphic(Some(Spanned::new(name, name_span)))?;
                        let end = self.prev_span();
                        return Ok(Spanned::new(PartItem::Graphic(decl), start.merge(end)));
                    }
                    _ => {
                        let value = self.parse_expr()?;
                        let end = self.prev_span();
                        let binding = LetBinding {
                            name: Spanned::new(name, name_span),
                            value,
                        };
                        return Ok(Spanned::new(
                            PartItem::LetBinding(binding),
                            start.merge(end),
                        ));
                    }
                }
            }

            if is_graphic_type(&name) {
                let decl = self.parse_graphic(None)?;
                let end = self.prev_span();
                return Ok(Spanned::new(PartItem::Graphic(decl), start.merge(end)));
            }

            if next.same_variant(&TokenKind::Colon) {
                self.bump(); // IDENT
                self.bump(); // :
                let value = self.parse_expr()?;
                let end = self.prev_span();
                let prop = Property {
                    key: Spanned::new(name, name_span),
                    value,
                };
                return Ok(Spanned::new(PartItem::Property(prop), start.merge(end)));
            }
        }

        Err(self.err("expected part item (pin, graphic, or let binding)"))
    }

    // ── Pin ────────────────────────────────────────────────────────────────

    fn parse_pin(&mut self, binding: Option<Spanned<String>>) -> Result<PinDecl, ParseError> {
        self.expect(&TokenKind::Pin, "expected 'pin'")?;
        let name = self.parse_entity_name()?;
        self.skip_newlines();
        let body = self.parse_object()?;
        Ok(PinDecl { binding, name, body })
    }

    // ── Parameter ─────────────────────────────────────────────────────────

    fn parse_parameter(
        &mut self,
        binding: Option<Spanned<String>>,
    ) -> Result<ParameterDecl, ParseError> {
        self.expect(&TokenKind::Parameter, "expected 'parameter'")?;
        let name = self.parse_entity_name()?;
        self.skip_newlines();
        let body = self.parse_object()?;
        Ok(ParameterDecl { binding, name, body })
    }

    // ── Alias ──────────────────────────────────────────────────────────────

    fn parse_alias(&mut self) -> Result<AliasDecl, ParseError> {
        self.expect(&TokenKind::Alias, "expected 'alias'")?;
        let name = self.parse_entity_name()?;
        Ok(AliasDecl { name })
    }

    // ── Footprint map (inside component) ─────────────────────────────────

    fn parse_footprint_map(&mut self) -> Result<FootprintMapDecl, ParseError> {
        self.expect(&TokenKind::Footprint, "expected 'footprint'")?;

        // The name can be entity_name or $dollar_path
        let name_start = self.current_span();
        let name = if let TokenKind::DollarIdent(root) = self.current_kind().clone() {
            let root_span = self.current_span();
            self.bump();
            let path = self.parse_dollar_path_tail(root, root_span)?;
            let end = self.prev_span();
            Spanned::new(FootprintRef::DollarPath(path), name_start.merge(end))
        } else {
            let en = self.parse_entity_name()?;
            let end = self.prev_span();
            Spanned::new(FootprintRef::Name(en.node), name_start.merge(end))
        };

        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after footprint reference")?;
        let mut maps = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let map_start = self.current_span();
            self.expect(&TokenKind::Map, "expected 'map' inside footprint body")?;
            self.skip_newlines();
            let body = self.parse_object()?;
            let map_end = self.prev_span();
            maps.push(Spanned::new(
                MapEntry { body },
                map_start.merge(map_end),
            ));
            self.skip_separators();
        }
        self.expect(&TokenKind::RBrace, "expected '}' to close footprint body")?;
        Ok(FootprintMapDecl { name, maps })
    }

    // ── Footprint declaration (top-level) ─────────────────────────────────

    fn parse_footprint(
        &mut self,
        binding: Option<Spanned<String>>,
    ) -> Result<FootprintDecl, ParseError> {
        self.expect(&TokenKind::Footprint, "expected 'footprint'")?;
        let name = self.parse_entity_name()?;
        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after footprint name")?;
        let body = self.parse_footprint_body()?;
        self.expect(&TokenKind::RBrace, "expected '}' to close footprint body")?;
        Ok(FootprintDecl { binding, name, body })
    }

    fn parse_footprint_body(&mut self) -> Result<Vec<Spanned<FootprintItem>>, ParseError> {
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let item = self.parse_footprint_item()?;
            items.push(item);
            self.skip_separators();
        }
        Ok(items)
    }

    fn parse_footprint_item(&mut self) -> Result<Spanned<FootprintItem>, ParseError> {
        let start = self.current_span();

        if self.at(&TokenKind::Pad) {
            let decl = self.parse_pad(None)?;
            let end = self.prev_span();
            return Ok(Spanned::new(FootprintItem::Pad(decl), start.merge(end)));
        }

        if self.at(&TokenKind::Row) {
            self.bump();
            self.skip_newlines();
            let body = self.parse_object()?;
            let end = self.prev_span();
            return Ok(Spanned::new(
                FootprintItem::Row(RowDecl { body }),
                start.merge(end),
            ));
        }

        if self.at(&TokenKind::Column) {
            self.bump();
            self.skip_newlines();
            let body = self.parse_object()?;
            let end = self.prev_span();
            return Ok(Spanned::new(
                FootprintItem::Column(RowDecl { body }),
                start.merge(end),
            ));
        }

        if self.at(&TokenKind::Grid) {
            self.bump();
            self.skip_newlines();
            let body = self.parse_object()?;
            let end = self.prev_span();
            return Ok(Spanned::new(
                FootprintItem::Grid(GridDecl { body }),
                start.merge(end),
            ));
        }

        if self.at(&TokenKind::Let) {
            let binding = self.parse_let_binding()?;
            let end = self.prev_span();
            return Ok(Spanned::new(
                FootprintItem::LetBinding(binding),
                start.merge(end),
            ));
        }

        if let TokenKind::Ident(name) = self.current_kind().clone() {
            let name_span = self.current_span();
            let next = self.peek_ahead(1);

            if next.same_variant(&TokenKind::Colon) {
                let prop = self.parse_property()?;
                let end = self.prev_span();
                return Ok(Spanned::new(
                    FootprintItem::Property(prop),
                    start.merge(end),
                ));
            }

            if next.same_variant(&TokenKind::Eq) {
                self.bump(); // IDENT
                self.bump(); // =
                match self.current_kind().clone() {
                    TokenKind::Pad => {
                        let decl = self.parse_pad(Some(Spanned::new(name, name_span)))?;
                        let end = self.prev_span();
                        return Ok(Spanned::new(FootprintItem::Pad(decl), start.merge(end)));
                    }
                    TokenKind::Ident(ref graphic) if is_graphic_type(graphic) => {
                        let decl =
                            self.parse_graphic(Some(Spanned::new(name, name_span)))?;
                        let end = self.prev_span();
                        return Ok(Spanned::new(
                            FootprintItem::Graphic(decl),
                            start.merge(end),
                        ));
                    }
                    _ => {
                        let value = self.parse_expr()?;
                        let end = self.prev_span();
                        let binding = LetBinding {
                            name: Spanned::new(name, name_span),
                            value,
                        };
                        return Ok(Spanned::new(
                            FootprintItem::LetBinding(binding),
                            start.merge(end),
                        ));
                    }
                }
            }

            if is_graphic_type(&name) {
                let decl = self.parse_graphic(None)?;
                let end = self.prev_span();
                return Ok(Spanned::new(FootprintItem::Graphic(decl), start.merge(end)));
            }
        }

        Err(self.err(
            "expected footprint item (property, pad, row, column, grid, graphic, or let binding)",
        ))
    }

    // ── Project ─────────────────────────────────────────────────────────

    fn parse_project(
        &mut self,
        binding: Option<Spanned<String>>,
    ) -> Result<ProjectDecl, ParseError> {
        self.expect(&TokenKind::Project, "expected 'project'")?;
        let name = self.parse_entity_name()?;
        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after project name")?;
        let body = self.parse_project_body()?;
        self.expect(&TokenKind::RBrace, "expected '}' to close project body")?;
        Ok(ProjectDecl { binding, name, body })
    }

    fn parse_project_body(&mut self) -> Result<Vec<Spanned<ProjectItem>>, ParseError> {
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let item = self.parse_project_item()?;
            items.push(item);
            self.skip_separators();
        }
        Ok(items)
    }

    fn parse_project_item(&mut self) -> Result<Spanned<ProjectItem>, ParseError> {
        let start = self.current_span();

        // let binding at project scope
        if self.at(&TokenKind::Let) {
            let binding = self.parse_let_binding()?;
            let end = self.prev_span();
            return Ok(Spanned::new(
                ProjectItem::LetBinding(binding),
                start.merge(end),
            ));
        }

        // IDENT-led items: keyword blocks or property
        if let TokenKind::Ident(name) = self.current_kind().clone() {
            let name_span = self.current_span();

            // property: key: value
            if self.peek_ahead(1).same_variant(&TokenKind::Colon) {
                let prop = self.parse_property()?;
                let end = self.prev_span();
                return Ok(Spanned::new(
                    ProjectItem::Property(prop),
                    start.merge(end),
                ));
            }

            // let binding without `let` keyword: IDENT = expr
            if self.peek_ahead(1).same_variant(&TokenKind::Eq) {
                self.bump(); // consume IDENT
                self.bump(); // consume =
                let value = self.parse_expr()?;
                let end = self.prev_span();
                let binding = LetBinding {
                    name: Spanned::new(name.clone(), name_span),
                    value,
                };
                return Ok(Spanned::new(
                    ProjectItem::LetBinding(binding),
                    start.merge(end),
                ));
            }

            // Dispatch on block keyword
            match name.as_str() {
                "document" => {
                    self.bump();
                    let decl = self.parse_document_block()?;
                    let end = self.prev_span();
                    return Ok(Spanned::new(
                        ProjectItem::Document(decl),
                        start.merge(end),
                    ));
                }
                "annotation" => {
                    self.bump();
                    let decl = self.parse_annotation_block()?;
                    let end = self.prev_span();
                    return Ok(Spanned::new(
                        ProjectItem::Annotation(decl),
                        start.merge(end),
                    ));
                }
                "erc_matrix" => {
                    self.bump();
                    let entries = self.parse_erc_matrix_block()?;
                    let end = self.prev_span();
                    return Ok(Spanned::new(
                        ProjectItem::ErcMatrix(entries),
                        start.merge(end),
                    ));
                }
                "erc_levels" => {
                    self.bump();
                    let entries = self.parse_erc_levels_block()?;
                    let end = self.prev_span();
                    return Ok(Spanned::new(
                        ProjectItem::ErcLevels(entries),
                        start.merge(end),
                    ));
                }
                "output_group" => {
                    self.bump();
                    let decl = self.parse_output_group_block()?;
                    let end = self.prev_span();
                    return Ok(Spanned::new(
                        ProjectItem::OutputGroup(decl),
                        start.merge(end),
                    ));
                }
                "comparison" => {
                    self.bump();
                    let rules = self.parse_comparison_block()?;
                    let end = self.prev_span();
                    return Ok(Spanned::new(
                        ProjectItem::Comparison(rules),
                        start.merge(end),
                    ));
                }
                "class_gen" => {
                    self.bump();
                    let props = self.parse_property_block()?;
                    let end = self.prev_span();
                    return Ok(Spanned::new(
                        ProjectItem::ClassGen(props),
                        start.merge(end),
                    ));
                }
                "library_update" => {
                    self.bump();
                    let props = self.parse_property_block()?;
                    let end = self.prev_span();
                    return Ok(Spanned::new(
                        ProjectItem::LibraryUpdate(props),
                        start.merge(end),
                    ));
                }
                "variant" => {
                    self.bump();
                    let decl = self.parse_variant_block()?;
                    let end = self.prev_span();
                    return Ok(Spanned::new(
                        ProjectItem::Variant(decl),
                        start.merge(end),
                    ));
                }
                _ => {}
            }
        }

        Err(self.err(
            "expected project item (property, document, annotation, erc_matrix, erc_levels, \
             output_group, comparison, class_gen, library_update, variant, or let binding)",
        ))
    }

    /// document "path/to/file.SchDoc" { key: value, ... }
    fn parse_document_block(&mut self) -> Result<DocumentBlockDecl, ParseError> {
        let path = self.parse_entity_name()?;
        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after document path")?;
        let mut body = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let prop_start = self.current_span();
            let prop = self.parse_property()?;
            let prop_end = self.prev_span();
            body.push(Spanned::new(prop, prop_start.merge(prop_end)));
            self.skip_separators();
        }
        self.expect(&TokenKind::RBrace, "expected '}' to close document block")?;
        Ok(DocumentBlockDecl { path, body })
    }

    /// annotation { key: value, ... match_parameter N { ... } ... }
    fn parse_annotation_block(&mut self) -> Result<AnnotationBlockDecl, ParseError> {
        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after annotation")?;
        let mut properties = Vec::new();
        let mut match_parameters = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            if let TokenKind::Ident(ref name) = self.current_kind().clone() {
                if name == "match_parameter" {
                    let mp_start = self.current_span();
                    self.bump(); // consume "match_parameter"
                    let index = self.expect_integer("expected index after 'match_parameter'")?;
                    self.skip_newlines();
                    let body = self.parse_object()?;
                    let mp_end = self.prev_span();
                    match_parameters.push(Spanned::new(
                        MatchParameterDecl { index, body },
                        mp_start.merge(mp_end),
                    ));
                    self.skip_separators();
                    continue;
                }
            }
            // Regular property
            let prop_start = self.current_span();
            let prop = self.parse_property()?;
            let prop_end = self.prev_span();
            properties.push(Spanned::new(prop, prop_start.merge(prop_end)));
            self.skip_separators();
        }
        self.expect(&TokenKind::RBrace, "expected '}' to close annotation block")?;
        Ok(AnnotationBlockDecl {
            properties,
            match_parameters,
        })
    }

    /// erc_matrix { (row, col): level, ... }
    fn parse_erc_matrix_block(&mut self) -> Result<Vec<Spanned<ErcMatrixEntryDecl>>, ParseError> {
        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after erc_matrix")?;
        let mut entries = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let entry_start = self.current_span();
            // Parse (row, col): level
            self.expect(&TokenKind::LParen, "expected '(' for ERC matrix entry")?;
            let row = self.expect_ident("expected ERC connection code for row")?;
            self.expect(&TokenKind::Comma, "expected ',' between row and col")?;
            self.skip_newlines();
            let col = self.expect_ident("expected ERC connection code for col")?;
            self.expect(&TokenKind::RParen, "expected ')' after col")?;
            self.expect(&TokenKind::Colon, "expected ':' after (row, col)")?;
            self.skip_newlines();
            let level = self.expect_ident("expected error level (no_report, warning, error, fatal)")?;
            let entry_end = self.prev_span();
            entries.push(Spanned::new(
                ErcMatrixEntryDecl { row, col, level },
                entry_start.merge(entry_end),
            ));
            self.skip_separators();
        }
        self.expect(&TokenKind::RBrace, "expected '}' to close erc_matrix block")?;
        Ok(entries)
    }

    /// erc_levels { name: level, ... }
    fn parse_erc_levels_block(&mut self) -> Result<Vec<Spanned<ErcLevelEntryDecl>>, ParseError> {
        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after erc_levels")?;
        let mut entries = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let entry_start = self.current_span();
            let name = self.expect_ident("expected ERC level name")?;
            self.expect(&TokenKind::Colon, "expected ':' after ERC level name")?;
            self.skip_newlines();
            let level = self.parse_expr()?;
            let entry_end = self.prev_span();
            entries.push(Spanned::new(
                ErcLevelEntryDecl { name, level },
                entry_start.merge(entry_end),
            ));
            self.skip_separators();
        }
        self.expect(&TokenKind::RBrace, "expected '}' to close erc_levels block")?;
        Ok(entries)
    }

    /// output_group "Name" { key: value, ... output "Name" { ... } ... }
    fn parse_output_group_block(&mut self) -> Result<OutputGroupBlockDecl, ParseError> {
        let name = self.parse_entity_name()?;
        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after output_group name")?;
        let mut properties = Vec::new();
        let mut outputs = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            if let TokenKind::Ident(ref kw) = self.current_kind().clone() {
                if kw == "output" {
                    let out_start = self.current_span();
                    self.bump(); // consume "output"
                    let out_name = self.parse_entity_name()?;
                    self.skip_newlines();
                    self.expect(&TokenKind::LBrace, "expected '{' after output name")?;
                    let mut out_body = Vec::new();
                    self.skip_newlines();
                    while !self.at(&TokenKind::RBrace) && !self.at_eof() {
                        let prop_start = self.current_span();
                        let prop = self.parse_property()?;
                        let prop_end = self.prev_span();
                        out_body.push(Spanned::new(prop, prop_start.merge(prop_end)));
                        self.skip_separators();
                    }
                    self.expect(&TokenKind::RBrace, "expected '}' to close output block")?;
                    let out_end = self.prev_span();
                    outputs.push(Spanned::new(
                        OutputBlockDecl {
                            name: out_name,
                            body: out_body,
                        },
                        out_start.merge(out_end),
                    ));
                    self.skip_separators();
                    continue;
                }
            }
            // Regular property
            let prop_start = self.current_span();
            let prop = self.parse_property()?;
            let prop_end = self.prev_span();
            properties.push(Spanned::new(prop, prop_start.merge(prop_end)));
            self.skip_separators();
        }
        self.expect(&TokenKind::RBrace, "expected '}' to close output_group block")?;
        Ok(OutputGroupBlockDecl {
            name,
            properties,
            outputs,
        })
    }

    /// comparison { rule "Kind" { ... } ... }
    fn parse_comparison_block(&mut self) -> Result<Vec<Spanned<ComparisonRuleDecl>>, ParseError> {
        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after comparison")?;
        let mut rules = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let rule_start = self.current_span();
            // Expect "rule" keyword-like ident
            let kw = self.expect_ident("expected 'rule' inside comparison block")?;
            if kw.node != "rule" {
                return Err(ParseError::new(
                    ParseErrorCode::E1002,
                    format!("expected 'rule', got '{}'", kw.node),
                    kw.span,
                ));
            }
            let kind = self.parse_entity_name()?;
            self.skip_newlines();
            let body = self.parse_object()?;
            let rule_end = self.prev_span();
            rules.push(Spanned::new(
                ComparisonRuleDecl { kind, body },
                rule_start.merge(rule_end),
            ));
            self.skip_separators();
        }
        self.expect(&TokenKind::RBrace, "expected '}' to close comparison block")?;
        Ok(rules)
    }

    /// A block of properties: { key: value, ... }
    fn parse_property_block(&mut self) -> Result<Vec<Spanned<Property>>, ParseError> {
        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{'")?;
        let mut props = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let prop_start = self.current_span();
            let prop = self.parse_property()?;
            let prop_end = self.prev_span();
            props.push(Spanned::new(prop, prop_start.merge(prop_end)));
            self.skip_separators();
        }
        self.expect(&TokenKind::RBrace, "expected '}'")?;
        Ok(props)
    }

    /// variant "Name" { key: value, ... variation "D" { ... } param_variation "D" { ... } }
    fn parse_variant_block(&mut self) -> Result<VariantBlockDecl, ParseError> {
        let name = self.parse_entity_name()?;
        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after variant name")?;
        let mut properties = Vec::new();
        let mut variations = Vec::new();
        let mut param_variations = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            if let TokenKind::Ident(ref kw) = self.current_kind().clone() {
                match kw.as_str() {
                    "variation" => {
                        let v_start = self.current_span();
                        self.bump();
                        let designator = self.parse_entity_name()?;
                        self.skip_newlines();
                        let body = self.parse_object()?;
                        let v_end = self.prev_span();
                        variations.push(Spanned::new(
                            VariationDecl { designator, body },
                            v_start.merge(v_end),
                        ));
                        self.skip_separators();
                        continue;
                    }
                    "param_variation" => {
                        let v_start = self.current_span();
                        self.bump();
                        let designator = self.parse_entity_name()?;
                        self.skip_newlines();
                        let body = self.parse_object()?;
                        let v_end = self.prev_span();
                        param_variations.push(Spanned::new(
                            ParamVariationDecl { designator, body },
                            v_start.merge(v_end),
                        ));
                        self.skip_separators();
                        continue;
                    }
                    _ => {}
                }
            }
            // Regular property
            let prop_start = self.current_span();
            let prop = self.parse_property()?;
            let prop_end = self.prev_span();
            properties.push(Spanned::new(prop, prop_start.merge(prop_end)));
            self.skip_separators();
        }
        self.expect(&TokenKind::RBrace, "expected '}' to close variant block")?;
        Ok(VariantBlockDecl {
            name,
            properties,
            variations,
            param_variations,
        })
    }

    // ── PcbDoc: board, primitives, named blocks ────────────────────────────

    /// Parse `board NAME { ... }` — board settings block.
    fn parse_board(&mut self) -> Result<BoardDecl, ParseError> {
        self.expect(&TokenKind::Board, "expected 'board'")?;
        let name = self.parse_entity_name()?;
        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after board name")?;
        let body = self.parse_board_body()?;
        self.expect(&TokenKind::RBrace, "expected '}' to close board block")?;
        Ok(BoardDecl { name, body })
    }

    fn parse_board_body(&mut self) -> Result<Vec<Spanned<BoardItem>>, ParseError> {
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let start = self.current_span();

            // let binding
            if self.at(&TokenKind::Let) {
                let binding = self.parse_let_binding()?;
                let end = self.prev_span();
                items.push(Spanned::new(BoardItem::LetBinding(binding), start.merge(end)));
                self.skip_separators();
                continue;
            }

            // property: key: value
            let prop = self.parse_property()?;
            let end = self.prev_span();
            items.push(Spanned::new(BoardItem::Property(prop), start.merge(end)));
            self.skip_separators();
        }
        Ok(items)
    }

    /// Parse `placement { ... }` top-level block.
    fn parse_placement(&mut self) -> Result<PlacementDecl, ParseError> {
        match self.current_kind() {
            TokenKind::Ident(s) if s == "placement" => {
                self.bump();
            }
            _ => return Err(self.err("expected 'placement'")),
        }
        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after 'placement'")?;
        let mut body = Vec::new();
        self.skip_separators();

        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let start = self.current_span();

            if self.at(&TokenKind::Let) {
                let b = self.parse_let_binding()?;
                let end = self.prev_span();
                body.push(Spanned::new(PlacementItem::LetBinding(b), start.merge(end)));
                self.skip_separators();
                continue;
            }

            if let TokenKind::Ident(kind) = self.current_kind().clone() {
                match kind.as_str() {
                    "place" => {
                        let place = self.parse_placement_place()?;
                        let end = self.prev_span();
                        body.push(Spanned::new(PlacementItem::Place(place), start.merge(end)));
                        self.skip_separators();
                        continue;
                    }
                    "left_of" | "right_of" | "above" | "below" => {
                        let c = self.parse_placement_directional_constraint()?;
                        let end = self.prev_span();
                        body.push(Spanned::new(PlacementItem::Constraint(c), start.merge(end)));
                        self.skip_separators();
                        continue;
                    }
                    "optimize" => {
                        self.bump();
                        self.skip_newlines();
                        let obj = self.parse_object()?;
                        let end = self.prev_span();
                        body.push(Spanned::new(PlacementItem::Optimize(obj), start.merge(end)));
                        self.skip_separators();
                        continue;
                    }
                    "clearance" => {
                        self.bump();
                        self.skip_newlines();
                        let obj = self.parse_object()?;
                        let end = self.prev_span();
                        body.push(Spanned::new(PlacementItem::Clearance(obj), start.merge(end)));
                        self.skip_separators();
                        continue;
                    }
                    _ => {}
                }
            }

            if self.at(&TokenKind::Group) {
                let decl = self.parse_placement_group()?;
                let end = self.prev_span();
                body.push(Spanned::new(PlacementItem::GroupDecl(decl), start.merge(end)));
                self.skip_separators();
                continue;
            }

            if self.at(&TokenKind::Separate) {
                let decl = self.parse_placement_separate()?;
                let end = self.prev_span();
                body.push(Spanned::new(PlacementItem::SeparateDecl(decl), start.merge(end)));
                self.skip_separators();
                continue;
            }

            if self.at(&TokenKind::Autoplace) {
                self.bump();
                self.skip_newlines();
                let obj = self.parse_object()?;
                let end = self.prev_span();
                body.push(Spanned::new(PlacementItem::AutoplaceBlock(obj), start.merge(end)));
                self.skip_separators();
                continue;
            }

            let prop = self.parse_property()?;
            let end = self.prev_span();
            body.push(Spanned::new(PlacementItem::Property(prop), start.merge(end)));
            self.skip_separators();
        }

        self.expect(&TokenKind::RBrace, "expected '}' to close placement block")?;
        Ok(PlacementDecl { body })
    }

    fn parse_placement_place(&mut self) -> Result<PlaceDecl, ParseError> {
        match self.current_kind() {
            TokenKind::Ident(s) if s == "place" => {
                self.bump();
            }
            _ => return Err(self.err("expected 'place'")),
        }
        self.skip_newlines();

        let mut designators = Vec::new();
        loop {
            let name = self.parse_entity_name()?;
            designators.push(name);
            self.skip_newlines();
            if !self.eat(&TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
        }

        self.skip_newlines();
        let body = self.parse_object()?;
        Ok(PlaceDecl { designators, body })
    }

    fn parse_placement_directional_constraint(&mut self) -> Result<PlacementConstraintDecl, ParseError> {
        let kind = match self.current_kind().clone() {
            TokenKind::Ident(s) => {
                self.bump();
                s
            }
            _ => return Err(self.err("expected directional placement constraint")),
        };
        self.skip_newlines();
        let a = self.parse_dollar_path_reference()?;
        self.skip_newlines();
        self.expect(&TokenKind::Comma, "expected ',' between placement references")?;
        self.skip_newlines();
        let b = self.parse_dollar_path_reference()?;
        self.skip_newlines();
        let body = if self.at(&TokenKind::LBrace) {
            Some(self.parse_object()?)
        } else {
            None
        };

        Ok(match kind.as_str() {
            "left_of" => PlacementConstraintDecl::LeftOf { a, b, body },
            "right_of" => PlacementConstraintDecl::RightOf { a, b, body },
            "above" => PlacementConstraintDecl::Above { a, b, body },
            "below" => PlacementConstraintDecl::Below { a, b, body },
            _ => return Err(self.err("unsupported directional placement constraint")),
        })
    }

    /// Parse `group NAME { ... }` inside a placement block.
    fn parse_placement_group(&mut self) -> Result<PlacementGroupDecl, ParseError> {
        self.expect(&TokenKind::Group, "expected 'group'")?;
        self.skip_newlines();
        let name_start = self.current_span();
        let name_str = match self.current_kind().clone() {
            TokenKind::Ident(s) => {
                self.bump();
                s
            }
            TokenKind::String(s) => {
                self.bump();
                s
            }
            _ => return Err(self.err("expected group name after 'group'")),
        };
        let name_end = self.prev_span();
        let name = Spanned::new(name_str, name_start.merge(name_end));
        self.skip_newlines();
        let body = self.parse_object()?;
        Ok(PlacementGroupDecl { name, body })
    }

    /// Parse `separate $group_a, $group_b { gap: Nmm }` inside a placement block.
    fn parse_placement_separate(&mut self) -> Result<PlacementSeparateDecl, ParseError> {
        self.expect(&TokenKind::Separate, "expected 'separate'")?;
        self.skip_newlines();
        let mut groups = Vec::new();
        loop {
            let g = self.parse_dollar_path_reference()?;
            groups.push(g);
            self.skip_newlines();
            if !self.eat(&TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
            // stop consuming if next token is LBrace (start of body)
            if self.at(&TokenKind::LBrace) {
                break;
            }
        }
        self.skip_newlines();
        let body = if self.at(&TokenKind::LBrace) {
            Some(self.parse_object()?)
        } else {
            None
        };
        Ok(PlacementSeparateDecl { groups, body })
    }

    /// Parse a PcbDoc primitive from an identifier token: `track { ... }`, `arc { ... }`, etc.
    fn parse_pcbdoc_primitive(&mut self) -> Result<PcbDocPrimitiveDecl, ParseError> {
        let type_start = self.current_span();
        let type_name = match self.current_kind().clone() {
            TokenKind::Ident(s) => {
                self.bump();
                Spanned::new(s, type_start)
            }
            _ => return Err(self.err("expected PcbDoc primitive type identifier")),
        };
        self.skip_newlines();
        // Optional name before '{'
        let name = if !self.at(&TokenKind::LBrace) {
            Some(self.parse_entity_name()?)
        } else {
            None
        };
        self.skip_newlines();
        let body = self.parse_object()?;
        Ok(PcbDocPrimitiveDecl { primitive_type: type_name, name, body })
    }

    /// Parse a PcbDoc primitive from a keyword token (e.g. `pad NAME { ... }` at top level).
    fn parse_pcbdoc_primitive_from_keyword(&mut self, keyword: &str) -> Result<PcbDocPrimitiveDecl, ParseError> {
        let type_start = self.current_span();
        self.bump(); // consume the keyword token
        self.skip_newlines();
        let name = if !self.at(&TokenKind::LBrace) {
            Some(self.parse_entity_name()?)
        } else {
            None
        };
        self.skip_newlines();
        let body = self.parse_object()?;
        Ok(PcbDocPrimitiveDecl {
            primitive_type: Spanned::new(keyword.to_string(), type_start),
            name,
            body,
        })
    }

    /// Parse a PcbDoc named block: `polygon NAME { ... }`, `rule NAME { ... }`, etc.
    fn parse_pcbdoc_named_block(&mut self, start: Span) -> Result<Spanned<SpecItem>, ParseError> {
        let type_name = match self.current_kind().clone() {
            TokenKind::Ident(s) => {
                self.bump();
                s
            }
            _ => return Err(self.err("expected PcbDoc block type identifier")),
        };
        self.skip_newlines();
        match type_name.as_str() {
            "polygon" => {
                let name = self.parse_entity_name()?;
                self.skip_newlines();
                let body = self.parse_object()?;
                let end = self.prev_span();
                Ok(Spanned::new(SpecItem::Polygon(PolygonDecl { name, body }), start.merge(end)))
            }
            "rule" => {
                let name = self.parse_entity_name()?;
                self.skip_newlines();
                let body = self.parse_object()?;
                let end = self.prev_span();
                Ok(Spanned::new(SpecItem::Rule(RuleDecl { name, body }), start.merge(end)))
            }
            "class" => {
                let name = self.parse_entity_name()?;
                self.skip_newlines();
                let body = self.parse_object()?;
                let end = self.prev_span();
                Ok(Spanned::new(SpecItem::Class(ClassDecl { name, body }), start.merge(end)))
            }
            "differential_pair" => {
                let name = self.parse_entity_name()?;
                self.skip_newlines();
                let body = self.parse_object()?;
                let end = self.prev_span();
                Ok(Spanned::new(SpecItem::DifferentialPair(DifferentialPairDecl { name, body }), start.merge(end)))
            }
            _ => unreachable!("guarded by is_pcbdoc_block_type"),
        }
    }

    // ── Pad ────────────────────────────────────────────────────────────────

    fn parse_pad(&mut self, binding: Option<Spanned<String>>) -> Result<PadDecl, ParseError> {
        self.expect(&TokenKind::Pad, "expected 'pad'")?;
        let name = self.parse_entity_name()?;
        self.skip_newlines();
        let body = self.parse_object()?;
        Ok(PadDecl { binding, name, body })
    }

    // ── SchDoc: sheet, net, power, objects ────────────────────────────────

    /// Parse `sheet { ... }` — sheet metadata block (no name).
    fn parse_sheet(&mut self) -> Result<SheetDecl, ParseError> {
        self.expect(&TokenKind::Sheet, "expected 'sheet'")?;
        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after 'sheet'")?;
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let item = self.parse_sheet_item()?;
            items.push(item);
            self.skip_separators();
        }
        self.expect(&TokenKind::RBrace, "expected '}' to close sheet block")?;
        Ok(SheetDecl { body: items })
    }

    fn parse_sheet_item(&mut self) -> Result<Spanned<SheetItem>, ParseError> {
        let start = self.current_span();

        // let binding
        if self.at(&TokenKind::Let) {
            let binding = self.parse_let_binding()?;
            let end = self.prev_span();
            return Ok(Spanned::new(SheetItem::LetBinding(binding), start.merge(end)));
        }

        // fonts { ... } sub-block
        if let TokenKind::Ident(ref name) = self.current_kind().clone() {
            if name == "fonts" {
                let block = self.parse_font_block()?;
                let end = self.prev_span();
                return Ok(Spanned::new(SheetItem::FontBlock(block), start.merge(end)));
            }
        }

        // property: key: value
        let prop = self.parse_property()?;
        let end = self.prev_span();
        Ok(Spanned::new(SheetItem::Property(prop), start.merge(end)))
    }

    /// Parse `fonts { font N { ... } ... }`
    fn parse_font_block(&mut self) -> Result<FontBlockDecl, ParseError> {
        // consume "fonts" ident
        self.bump();
        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after 'fonts'")?;
        let mut fonts = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let start = self.current_span();
            let decl = self.parse_font_decl()?;
            let end = self.prev_span();
            fonts.push(Spanned::new(decl, start.merge(end)));
            self.skip_separators();
        }
        self.expect(&TokenKind::RBrace, "expected '}' to close fonts block")?;
        Ok(FontBlockDecl { fonts })
    }

    /// Parse `font N { name: "...", size: 10 }`
    fn parse_font_decl(&mut self) -> Result<FontDecl, ParseError> {
        // expect "font" as ident
        match self.current_kind().clone() {
            TokenKind::Ident(ref s) if s == "font" => { self.bump(); }
            _ => return Err(self.err("expected 'font' keyword")),
        }
        let id = self.expect_integer("expected font id number")?;
        self.skip_newlines();
        let body = self.parse_object()?;
        Ok(FontDecl { id, body })
    }

    /// Parse `net NAME { pins: [...] }`
    fn parse_net(&mut self) -> Result<NetDecl, ParseError> {
        self.expect(&TokenKind::Net, "expected 'net'")?;
        let name = self.parse_entity_name()?;
        self.skip_newlines();
        let body = self.parse_object()?;
        Ok(NetDecl { name, body })
    }

    /// Parse `power NAME { style: ..., pins: [...] }`
    fn parse_power(&mut self) -> Result<PowerDecl, ParseError> {
        self.expect(&TokenKind::Power, "expected 'power'")?;
        let name = self.parse_entity_name()?;
        self.skip_newlines();
        let body = self.parse_object()?;
        Ok(PowerDecl { name, body })
    }

    /// Parse a SchDoc object whose type name is a keyword (e.g., `parameter`).
    ///
    /// This works identically to `parse_schdoc_object` but accepts the type name
    /// as a string rather than reading an identifier token — needed because
    /// `parameter` is a keyword (`TokenKind::Parameter`) rather than a plain ident.
    fn parse_schdoc_object_keyword(&mut self, type_name: &str) -> Result<SchDocObjectDecl, ParseError> {
        let type_start = self.current_span();
        self.bump(); // consume the keyword token

        let has_name = matches!(type_name, "parameter");

        let name = if has_name && !self.at(&TokenKind::LBrace) {
            Some(self.parse_entity_name()?)
        } else {
            None
        };

        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after SchDoc object type")?;
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let item = self.parse_schdoc_object_item()?;
            items.push(item);
            self.skip_separators();
        }
        self.expect(&TokenKind::RBrace, "expected '}' to close SchDoc object")?;

        Ok(SchDocObjectDecl {
            object_type: Spanned::new(type_name.to_string(), type_start),
            name,
            body: items,
        })
    }

    /// Parse a SchDoc object block: `wire { ... }`, `net_label NAME { ... }`, etc.
    fn parse_schdoc_object(&mut self) -> Result<SchDocObjectDecl, ParseError> {
        let type_start = self.current_span();
        let object_type = match self.current_kind().clone() {
            TokenKind::Ident(s) if is_schdoc_object_type(&s) || is_graphic_type(&s) => {
                self.bump();
                Spanned::new(s, type_start)
            }
            _ => return Err(self.err("expected SchDoc object type identifier")),
        };

        // Some object types have a name (net_label, power_object, port, sheet_symbol,
        // parameter_set, probe); others don't (wire, bus, junction, no_connect, etc.)
        let has_name = matches!(
            object_type.node.as_str(),
            "net_label" | "power_object" | "port" | "sheet_symbol" | "parameter_set" | "probe"
        );

        let name = if has_name && !self.at(&TokenKind::LBrace) {
            Some(self.parse_entity_name()?)
        } else {
            None
        };

        self.skip_newlines();
        self.expect(&TokenKind::LBrace, "expected '{' after SchDoc object type")?;
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let item = self.parse_schdoc_object_item()?;
            items.push(item);
            self.skip_separators();
        }
        self.expect(&TokenKind::RBrace, "expected '}' to close SchDoc object")?;
        Ok(SchDocObjectDecl { object_type, name, body: items })
    }

    fn parse_schdoc_object_item(&mut self) -> Result<Spanned<SchDocObjectItem>, ParseError> {
        let start = self.current_span();

        // parameter block
        if self.at(&TokenKind::Parameter) {
            let decl = self.parse_parameter(None)?;
            let end = self.prev_span();
            return Ok(Spanned::new(
                SchDocObjectItem::Parameter(decl),
                start.merge(end),
            ));
        }

        // let binding
        if self.at(&TokenKind::Let) {
            let binding = self.parse_let_binding()?;
            let end = self.prev_span();
            return Ok(Spanned::new(
                SchDocObjectItem::LetBinding(binding),
                start.merge(end),
            ));
        }

        if let TokenKind::Ident(ref name) = self.current_kind().clone() {
            // "entry" sub-block (inside sheet_symbol)
            if name == "entry" {
                let decl = self.parse_entry()?;
                let end = self.prev_span();
                return Ok(Spanned::new(
                    SchDocObjectItem::Entry(decl),
                    start.merge(end),
                ));
            }

            // graphic sub-blocks — but only if NOT followed by ":" (which means property)
            if is_graphic_type(name) && !self.peek_ahead(1).same_variant(&TokenKind::Colon) {
                let decl = self.parse_graphic(None)?;
                let end = self.prev_span();
                return Ok(Spanned::new(
                    SchDocObjectItem::Graphic(decl),
                    start.merge(end),
                ));
            }
        }

        // property: key: value
        let prop = self.parse_property()?;
        let end = self.prev_span();
        Ok(Spanned::new(
            SchDocObjectItem::Property(prop),
            start.merge(end),
        ))
    }

    /// Parse `entry NAME { ... }` — child of a sheet_symbol
    fn parse_entry(&mut self) -> Result<EntryDecl, ParseError> {
        // consume "entry" ident
        match self.current_kind().clone() {
            TokenKind::Ident(ref s) if s == "entry" => { self.bump(); }
            _ => return Err(self.err("expected 'entry'")),
        }
        let name = self.parse_entity_name()?;
        self.skip_newlines();
        let body = self.parse_object()?;
        Ok(EntryDecl { name, body })
    }

    // ── Graphic declaration ────────────────────────────────────────────────

    fn parse_graphic(&mut self, binding: Option<Spanned<String>>) -> Result<GraphicDecl, ParseError> {
        let type_start = self.current_span();
        let graphic_type = match self.current_kind().clone() {
            TokenKind::Ident(s) if is_graphic_type(&s) => {
                self.bump();
                Spanned::new(s, type_start)
            }
            _ => return Err(self.err("expected graphic type identifier")),
        };
        self.skip_newlines();
        let body = self.parse_object()?;
        Ok(GraphicDecl {
            binding,
            graphic_type,
            body,
        })
    }

    // ── Let binding ────────────────────────────────────────────────────────

    fn parse_let_binding(&mut self) -> Result<LetBinding, ParseError> {
        self.eat(&TokenKind::Let); // optional `let`
        let name = self.expect_ident("expected identifier in let binding")?;
        self.expect(&TokenKind::Eq, "expected '=' after binding name")?;
        let value = self.parse_expr()?;
        Ok(LetBinding { name, value })
    }

    // ── Property ───────────────────────────────────────────────────────────

    fn parse_property(&mut self) -> Result<Property, ParseError> {
        let key = self
            .try_eat_property_key()
            .ok_or_else(|| self.err("expected property key (identifier or keyword)"))?;
        self.expect(&TokenKind::Colon, "expected ':' after property key")?;
        self.skip_newlines();
        let value = self.parse_expr()?;
        Ok(Property { key, value })
    }

    // ── Entity name ────────────────────────────────────────────────────────

    fn parse_entity_name(&mut self) -> Result<Spanned<EntityName>, ParseError> {
        let span = self.current_span();
        match self.current_kind().clone() {
            TokenKind::String(s) => {
                self.bump();
                Ok(Spanned::new(EntityName::String(s), span))
            }
            TokenKind::Integer(n) => {
                self.bump();
                Ok(Spanned::new(EntityName::Integer(n), span))
            }
            TokenKind::Ident(s) => {
                self.bump();
                Ok(Spanned::new(EntityName::Ident(s), span))
            }
            _ => Err(self.err("expected entity name (identifier, quoted string, or integer)")),
        }
    }

    // ── Object parsing ─────────────────────────────────────────────────────

    fn parse_object(&mut self) -> Result<Spanned<Object>, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::LBrace, "expected '{'")?;
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) && !self.at_eof() {
            let item = self.parse_object_item()?;
            items.push(item);
            // Require a separator (comma, newline, or semicolon) between items.
            // Without this, `after: $ref electrical:` on the same line would silently parse.
            if !self.at(&TokenKind::RBrace) && !self.at_eof() && !self.eat_separator() {
                return Err(self.err("expected ',' or newline between properties"));
            }
            self.skip_newlines();
        }
        let end = self.current_span();
        self.expect(&TokenKind::RBrace, "expected '}'")?;
        Ok(Spanned::new(Object { items }, start.merge(end)))
    }

    fn parse_object_item(&mut self) -> Result<Spanned<ObjectItem>, ParseError> {
        let start = self.current_span();

        // Spread: ...expr
        if self.at(&TokenKind::DotDotDot) {
            self.bump();
            let expr = self.parse_expr()?;
            let end = self.prev_span();
            return Ok(Spanned::new(ObjectItem::Spread(expr), start.merge(end)));
        }

        // Let binding inside object: [let] IDENT = expr
        if self.at(&TokenKind::Let) {
            let binding = self.parse_let_binding()?;
            let end = self.prev_span();
            return Ok(Spanned::new(ObjectItem::LetBinding(binding), start.merge(end)));
        }

        // Any identifier-like token followed by ":" is a property.
        // Keywords (pin, pad, map, etc.) are valid property keys inside objects.
        // IDENT "=" is a let binding without `let`.
        if self.peek_ahead(1).same_variant(&TokenKind::Colon) {
            if let Some(key) = self.try_eat_property_key() {
                self.expect(&TokenKind::Colon, "expected ':' after property key")?;
                self.skip_newlines();
                let value = self.parse_expr()?;
                let end = self.prev_span();
                let prop = Property { key, value };
                return Ok(Spanned::new(ObjectItem::Property(prop), start.merge(end)));
            }
        }

        if let TokenKind::Ident(_) = self.current_kind() {
            if self.peek_ahead(1).same_variant(&TokenKind::Eq) {
                // let binding: name = value
                let binding = self.parse_let_binding()?;
                let end = self.prev_span();
                return Ok(Spanned::new(ObjectItem::LetBinding(binding), start.merge(end)));
            }
        }

        Err(self.err("expected object item (property 'key: value', spread '...expr', or let binding)"))
    }

    /// Eat an identifier or keyword token as a property key string.
    fn try_eat_property_key(&mut self) -> Option<Spanned<String>> {
        let span = self.current_span();
        let name = match self.current_kind().clone() {
            TokenKind::Ident(s) => s,
            // Allow all keywords as property keys (e.g. pin: 1, pad: 2 in map objects)
            TokenKind::Import => "import".to_string(),
            TokenKind::As => "as".to_string(),
            TokenKind::Component => "component".to_string(),
            TokenKind::Footprint => "footprint".to_string(),
            TokenKind::Pin => "pin".to_string(),
            TokenKind::Pad => "pad".to_string(),
            TokenKind::Part => "part".to_string(),
            TokenKind::Parameter => "parameter".to_string(),
            TokenKind::Alias => "alias".to_string(),
            TokenKind::Map => "map".to_string(),
            TokenKind::Row => "row".to_string(),
            TokenKind::Column => "column".to_string(),
            TokenKind::Grid => "grid".to_string(),
            TokenKind::Project => "project".to_string(),
            TokenKind::SwapGroup => "swap_group".to_string(),
            TokenKind::Group => "group".to_string(),
            TokenKind::Separate => "separate".to_string(),
            TokenKind::Autoplace => "autoplace".to_string(),
            TokenKind::Let => "let".to_string(),
            TokenKind::True => "true".to_string(),
            TokenKind::False => "false".to_string(),
            TokenKind::Null => "null".to_string(),
            _ => return None,
        };
        self.bump();
        Some(Spanned::new(name, span))
    }

    // ── Dollar path ────────────────────────────────────────────────────────

    fn parse_dollar_path_reference(&mut self) -> Result<Spanned<super::ast::DollarPath>, ParseError> {
        let start = self.current_span();
        let (root, root_span) = match self.current_kind().clone() {
            TokenKind::DollarIdent(s) => {
                let span = self.current_span();
                self.bump();
                (s, span)
            }
            _ => return Err(self.err("expected '$name' reference")),
        };
        let path = self.parse_dollar_path_tail(root, root_span)?;
        let end = self.prev_span();
        Ok(Spanned::new(path, start.merge(end)))
    }

    fn parse_dollar_path_tail(
        &mut self,
        root: String,
        root_span: Span,
    ) -> Result<super::ast::DollarPath, ParseError> {
        let mut steps = Vec::new();
        loop {
            if self.eat(&TokenKind::Dot) {
                let field = self.expect_ident("expected field name after '.'")?;
                let span = field.span;
                steps.push(Spanned::new(
                    super::ast::PathStep::Field(field.node),
                    span,
                ));
            } else if self.eat(&TokenKind::LBracket) {
                let key_start = self.current_span();
                let expr = self.parse_expr()?;
                let key_end = self.prev_span();
                self.expect(&TokenKind::RBracket, "expected ']'")?;
                steps.push(Spanned::new(
                    super::ast::PathStep::Index(expr.node),
                    key_start.merge(key_end),
                ));
            } else {
                break;
            }
        }
        Ok(super::ast::DollarPath {
            root: Spanned::new(root, root_span),
            steps,
        })
    }

    // ── Expression (Pratt parser) ─────────────────────────────────────────

    fn parse_expr(&mut self) -> Result<Spanned<Expr>, ParseError> {
        self.parse_pratt_expr(0)
    }

    fn parse_pratt_expr(&mut self, min_bp: u8) -> Result<Spanned<Expr>, ParseError> {
        let mut lhs = self.parse_prefix_expr()?;

        loop {
            let (op, left_bp, right_bp) = match self.current_kind() {
                TokenKind::Dot => {
                    // Check this is not being suppressed by newline context
                    (InfixOp::Access, 90u8, 91u8)
                }
                TokenKind::LBracket => (InfixOp::Index, 90, 91),
                TokenKind::Star => (InfixOp::Mul, 60, 61),
                TokenKind::Slash => (InfixOp::Div, 60, 61),
                TokenKind::Plus => (InfixOp::Add, 50, 51),
                TokenKind::Minus => (InfixOp::Sub, 50, 51),
                _ => break,
            };

            if left_bp < min_bp {
                break;
            }

            let op_span = self.current_span();
            self.bump(); // consume operator

            let lhs_span = lhs.span;
            lhs = match op {
                InfixOp::Access => {
                    let field = self.expect_ident("expected field name after '.'")?;
                    let field_span = field.span;
                    let span = lhs_span.merge(field_span);
                    Spanned::new(Expr::Path(Box::new(lhs), field), span)
                }
                InfixOp::Index => {
                    self.skip_newlines();
                    let idx = self.parse_pratt_expr(0)?;
                    self.skip_newlines();
                    let end = self.current_span();
                    self.expect(&TokenKind::RBracket, "expected ']'")?;
                    let span = lhs_span.merge(end);
                    Spanned::new(
                        Expr::Index(Box::new(lhs), Box::new(idx)),
                        span,
                    )
                }
                InfixOp::Add => {
                    let rhs = self.parse_pratt_expr(right_bp)?;
                    let span = lhs_span.merge(rhs.span);
                    Spanned::new(
                        Expr::BinOp(
                            Box::new(lhs),
                            Spanned::new(BinOp::Add, op_span),
                            Box::new(rhs),
                        ),
                        span,
                    )
                }
                InfixOp::Sub => {
                    let rhs = self.parse_pratt_expr(right_bp)?;
                    let span = lhs_span.merge(rhs.span);
                    Spanned::new(
                        Expr::BinOp(
                            Box::new(lhs),
                            Spanned::new(BinOp::Sub, op_span),
                            Box::new(rhs),
                        ),
                        span,
                    )
                }
                InfixOp::Mul => {
                    let rhs = self.parse_pratt_expr(right_bp)?;
                    let span = lhs_span.merge(rhs.span);
                    Spanned::new(
                        Expr::BinOp(
                            Box::new(lhs),
                            Spanned::new(BinOp::Mul, op_span),
                            Box::new(rhs),
                        ),
                        span,
                    )
                }
                InfixOp::Div => {
                    let rhs = self.parse_pratt_expr(right_bp)?;
                    let span = lhs_span.merge(rhs.span);
                    Spanned::new(
                        Expr::BinOp(
                            Box::new(lhs),
                            Spanned::new(BinOp::Div, op_span),
                            Box::new(rhs),
                        ),
                        span,
                    )
                }
            };
        }

        Ok(lhs)
    }

    fn parse_prefix_expr(&mut self) -> Result<Spanned<Expr>, ParseError> {
        let start = self.current_span();

        match self.current_kind().clone() {
            // String literal
            TokenKind::String(s) => {
                self.bump();
                Ok(Spanned::new(Expr::String(s), start))
            }

            // Template string
            TokenKind::Template(parts) => {
                self.bump();
                Ok(Spanned::new(Expr::Template(parts), start))
            }

            // Integer
            TokenKind::Integer(n) => {
                self.bump();
                Ok(Spanned::new(Expr::Integer(n), start))
            }

            // Float
            TokenKind::Float(f) => {
                self.bump();
                Ok(Spanned::new(Expr::Float(f), start))
            }

            // Dimensional scalar
            TokenKind::Dim(v, u) => {
                self.bump();
                Ok(Spanned::new(Expr::Dim(v, u), start))
            }

            // Color literal
            TokenKind::Color(r, g, b) => {
                self.bump();
                Ok(Spanned::new(Expr::Color(r, g, b), start))
            }

            // Boolean keywords
            TokenKind::True => {
                self.bump();
                Ok(Spanned::new(Expr::Bool(true), start))
            }
            TokenKind::False => {
                self.bump();
                Ok(Spanned::new(Expr::Bool(false), start))
            }

            // Null
            TokenKind::Null => {
                self.bump();
                Ok(Spanned::new(Expr::Null, start))
            }

            // $ident — dollar reference, possibly with path tail
            TokenKind::DollarIdent(name) => {
                self.bump();
                // Build initial expr: DollarIdent
                let mut expr = Spanned::new(Expr::DollarIdent(name.clone()), start);
                // Follow path steps: .field or [index]
                loop {
                    if self.eat(&TokenKind::Dot) {
                        let field = self.expect_ident("expected field name after '.'")?;
                        let span = start.merge(field.span);
                        expr = Spanned::new(Expr::Path(Box::new(expr), field), span);
                    } else if self.eat(&TokenKind::LBracket) {
                        self.skip_newlines();
                        let idx = self.parse_pratt_expr(0)?;
                        self.skip_newlines();
                        let end = self.current_span();
                        self.expect(&TokenKind::RBracket, "expected ']'")?;
                        let span = start.merge(end);
                        expr = Spanned::new(Expr::Index(Box::new(expr), Box::new(idx)), span);
                    } else {
                        break;
                    }
                }
                Ok(expr)
            }

            // Keywords that can also appear as identifier values in expressions.
            // e.g. `electrical: power` where `power` is a PinElectricalType value,
            //      `style: net` where net could be a valid enum value, etc.
            TokenKind::Power => {
                self.bump();
                Ok(Spanned::new(Expr::Ident("power".to_string()), start))
            }
            TokenKind::Net => {
                self.bump();
                Ok(Spanned::new(Expr::Ident("net".to_string()), start))
            }
            TokenKind::Sheet => {
                self.bump();
                Ok(Spanned::new(Expr::Ident("sheet".to_string()), start))
            }
            // New placement keywords that can also appear as identifier values in expressions.
            // e.g. `unplaced: autoplace`, `algorithm: full_pipeline`
            TokenKind::Autoplace => {
                self.bump();
                Ok(Spanned::new(Expr::Ident("autoplace".to_string()), start))
            }
            TokenKind::Group => {
                self.bump();
                Ok(Spanned::new(Expr::Ident("group".to_string()), start))
            }
            TokenKind::Separate => {
                self.bump();
                Ok(Spanned::new(Expr::Ident("separate".to_string()), start))
            }

            // bare IDENT — let binding ref or enum value, possibly with path tail
            TokenKind::Ident(name) => {
                self.bump();
                let mut expr = Spanned::new(Expr::Ident(name), start);
                loop {
                    if self.eat(&TokenKind::Dot) {
                        let field = self.expect_ident("expected field name after '.'")?;
                        let span = start.merge(field.span);
                        expr = Spanned::new(Expr::Path(Box::new(expr), field), span);
                    } else if self.eat(&TokenKind::LBracket) {
                        self.skip_newlines();
                        let idx = self.parse_pratt_expr(0)?;
                        self.skip_newlines();
                        let end = self.current_span();
                        self.expect(&TokenKind::RBracket, "expected ']'")?;
                        let span = start.merge(end);
                        expr = Spanned::new(Expr::Index(Box::new(expr), Box::new(idx)), span);
                    } else {
                        break;
                    }
                }
                Ok(expr)
            }

            // Unary negation: -expr
            TokenKind::Minus => {
                self.bump();
                let operand = self.parse_pratt_expr(70)?;
                let end = operand.span;
                Ok(Spanned::new(Expr::UnaryNeg(Box::new(operand)), start.merge(end)))
            }

            // Parenthesized expression or 2-tuple (coord)
            TokenKind::LParen => {
                self.bump(); // consume (
                self.skip_newlines();
                let first = self.parse_pratt_expr(0)?;
                self.skip_newlines();

                if self.eat(&TokenKind::Comma) {
                    // It's a 2-element tuple (coord)
                    self.skip_newlines();
                    let second = self.parse_pratt_expr(0)?;
                    self.skip_newlines();
                    // Allow trailing comma
                    self.eat(&TokenKind::Comma);
                    self.skip_newlines();
                    let end = self.current_span();
                    self.expect(&TokenKind::RParen, "expected ')' to close coord tuple")?;
                    let span = start.merge(end);
                    Ok(Spanned::new(
                        Expr::Tuple(Box::new(first), Box::new(second)),
                        span,
                    ))
                } else {
                    // Grouping
                    let end = self.current_span();
                    self.expect(&TokenKind::RParen, "expected ')'")?;
                    let _ = end;
                    Ok(first) // return the inner expression with its span
                }
            }

            // Array: [elem, elem, ...]
            TokenKind::LBracket => {
                self.bump(); // consume [
                let mut elems = Vec::new();
                self.skip_newlines();
                while !self.at(&TokenKind::RBracket) && !self.at_eof() {
                    elems.push(self.parse_pratt_expr(0)?);
                    if !self.eat_separator() {
                        break;
                    }
                    self.skip_newlines();
                }
                let end = self.current_span();
                self.expect(&TokenKind::RBracket, "expected ']'")?;
                Ok(Spanned::new(Expr::Array(elems), start.merge(end)))
            }

            // Object: { ... }
            TokenKind::LBrace => {
                let obj = self.parse_object()?;
                let span = obj.span;
                Ok(Spanned::new(Expr::Object(obj.node), span))
            }

            _ => Err(self.err("expected expression")),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum InfixOp {
    Access,
    Index,
    Add,
    Sub,
    Mul,
    Div,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Unit;
    use crate::ast::*;

    fn parse(src: &str) -> SpecFile {
        parse_spec(src).unwrap_or_else(|e| panic!("parse error: {}", e))
    }

    fn parse_err(src: &str) -> ParseError {
        parse_spec(src).expect_err("expected parse error")
    }

    // ── Import tests ───────────────────────────────────────────────────────

    #[test]
    fn test_import_bare() {
        let f = parse(r#"import "foo.pcblib-spec""#);
        assert_eq!(f.items.len(), 1);
        if let SpecItem::Import(imp) = &f.items[0].node {
            assert_eq!(imp.path.node, "foo.pcblib-spec");
            assert!(imp.alias.is_none());
        } else {
            panic!("expected Import");
        }
    }

    #[test]
    fn test_import_with_alias() {
        let f = parse(r#"import "standard-footprints.pcblib-spec" as footprints"#);
        if let SpecItem::Import(imp) = &f.items[0].node {
            assert_eq!(imp.path.node, "standard-footprints.pcblib-spec");
            assert_eq!(imp.alias.as_ref().unwrap().node, "footprints");
        } else {
            panic!("expected Import");
        }
    }

    // ── Let binding tests ──────────────────────────────────────────────────

    #[test]
    fn test_let_binding_with_keyword() {
        let f = parse("let x = 42");
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            assert_eq!(b.name.node, "x");
            assert!(matches!(b.value.node, Expr::Integer(42)));
        } else {
            panic!("expected LetBinding");
        }
    }

    #[test]
    fn test_let_binding_without_keyword() {
        let f = parse("x = 3.14");
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            assert_eq!(b.name.node, "x");
            assert!(matches!(b.value.node, Expr::Float(_)));
        } else {
            panic!("expected LetBinding");
        }
    }

    #[test]
    fn test_let_binding_object() {
        let f = parse(r#"let passive_pin = { electrical: passive, length: 25, side: outside }"#);
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            assert_eq!(b.name.node, "passive_pin");
            assert!(matches!(b.value.node, Expr::Object(_)));
        } else {
            panic!("expected LetBinding");
        }
    }

    // ── Expression tests ───────────────────────────────────────────────────

    #[test]
    fn test_expr_dim() {
        let f = parse("x = 100mil");
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            assert!(matches!(b.value.node, Expr::Dim(100.0, Unit::Mil)));
        }
    }

    #[test]
    fn test_expr_color() {
        let f = parse("x = #FF0000");
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            assert!(matches!(b.value.node, Expr::Color(0xFF, 0x00, 0x00)));
        }
    }

    #[test]
    fn test_expr_bool() {
        let f = parse("a = true\nb = false");
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            assert!(matches!(b.value.node, Expr::Bool(true)));
        }
        if let SpecItem::LetBinding(b) = &f.items[1].node {
            assert!(matches!(b.value.node, Expr::Bool(false)));
        }
    }

    #[test]
    fn test_expr_null() {
        let f = parse("x = null");
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            assert!(matches!(b.value.node, Expr::Null));
        }
    }

    #[test]
    fn test_expr_tuple() {
        let f = parse("x = (1mm, 2mm)");
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            assert!(matches!(b.value.node, Expr::Tuple(_, _)));
        }
    }

    #[test]
    fn test_expr_array() {
        let f = parse("x = [1, 2, 3]");
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            if let Expr::Array(elems) = &b.value.node {
                assert_eq!(elems.len(), 3);
            } else {
                panic!("expected Array");
            }
        }
    }

    #[test]
    fn test_expr_unary_neg() {
        let f = parse("x = -5");
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            assert!(matches!(b.value.node, Expr::UnaryNeg(_)));
        }
    }

    #[test]
    fn test_expr_binop_add() {
        let f = parse("x = 1 + 2");
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            assert!(matches!(b.value.node, Expr::BinOp(_, _, _)));
        }
    }

    #[test]
    fn test_expr_path() {
        let f = parse("x = $body.left");
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            assert!(matches!(b.value.node, Expr::Path(_, _)));
        }
    }

    #[test]
    fn test_expr_dollar_ident() {
        let f = parse("x = $fp");
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            assert!(matches!(&b.value.node, Expr::DollarIdent(s) if s == "fp"));
        }
    }

    #[test]
    fn test_expr_index() {
        let f = parse(r#"x = $fp["SOT-23"]"#);
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            assert!(matches!(b.value.node, Expr::Index(_, _)));
        }
    }

    #[test]
    fn test_expr_precedence_mul_before_add() {
        let f = parse("x = 2 + 3 * 4");
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            // Should parse as 2 + (3 * 4)
            if let Expr::BinOp(lhs, op, rhs) = &b.value.node {
                assert!(matches!(op.node, BinOp::Add));
                assert!(matches!(lhs.node, Expr::Integer(2)));
                assert!(matches!(rhs.node, Expr::BinOp(_, _, _)));
            } else {
                panic!("expected BinOp");
            }
        }
    }

    // ── Component tests ────────────────────────────────────────────────────

    #[test]
    fn test_empty_component() {
        let f = parse("component R_0603 {}");
        assert_eq!(f.items.len(), 1);
        if let SpecItem::Component(c) = &f.items[0].node {
            assert_eq!(c.name.node.as_str(), "R_0603");
            assert!(c.binding.is_none());
            assert!(c.body.is_empty());
        } else {
            panic!("expected Component");
        }
    }

    #[test]
    fn test_component_with_binding() {
        let f = parse("my_r = component R_0603 {}");
        if let SpecItem::Component(c) = &f.items[0].node {
            assert_eq!(c.binding.as_ref().unwrap().node, "my_r");
        } else {
            panic!("expected Component");
        }
    }

    #[test]
    fn test_component_quoted_name() {
        let f = parse(r#"component "My Part" {}"#);
        if let SpecItem::Component(c) = &f.items[0].node {
            assert_eq!(c.name.node.as_str(), "My Part");
            assert!(matches!(c.name.node, EntityName::String(_)));
        }
    }

    #[test]
    fn test_component_with_properties() {
        let f = parse(r#"
component R {
    designator: "R?"
    description: "Resistor"
}
"#);
        if let SpecItem::Component(c) = &f.items[0].node {
            assert_eq!(c.body.len(), 2);
            if let ComponentItem::Property(p) = &c.body[0].node {
                assert_eq!(p.key.node, "designator");
            }
        }
    }

    #[test]
    fn test_component_with_pin() {
        let f = parse(r#"
component R {
    pin 1 { electrical: passive }
}
"#);
        if let SpecItem::Component(c) = &f.items[0].node {
            assert_eq!(c.body.len(), 1);
            if let ComponentItem::Pin(p) = &c.body[0].node {
                assert_eq!(p.name.node.as_str(), "1");
                assert!(p.binding.is_none());
            }
        }
    }

    #[test]
    fn test_component_pin_with_binding() {
        let f = parse(r#"
component R {
    p1 = pin 1 { electrical: passive }
}
"#);
        if let SpecItem::Component(c) = &f.items[0].node {
            if let ComponentItem::Pin(p) = &c.body[0].node {
                assert_eq!(p.binding.as_ref().unwrap().node, "p1");
            }
        }
    }

    #[test]
    fn test_component_with_alias() {
        let f = parse("component R {\n    alias R0603\n}");
        if let SpecItem::Component(c) = &f.items[0].node {
            if let ComponentItem::Alias(a) = &c.body[0].node {
                assert_eq!(a.name.node.as_str(), "R0603");
            } else {
                panic!("expected Alias");
            }
        }
    }

    #[test]
    fn test_component_with_parameter() {
        let f = parse(r#"component R { parameter Value { text: "{VALUE}" } }"#);
        if let SpecItem::Component(c) = &f.items[0].node {
            if let ComponentItem::Parameter(p) = &c.body[0].node {
                assert_eq!(p.name.node.as_str(), "Value");
            }
        }
    }

    #[test]
    fn test_component_with_footprint_map() {
        let f = parse(r#"
component R {
    footprint R0805 {
        map { pin: 1, pad: 1 }
        map { pin: 2, pad: 2 }
    }
}
"#);
        if let SpecItem::Component(c) = &f.items[0].node {
            if let ComponentItem::FootprintMap(fm) = &c.body[0].node {
                assert_eq!(fm.maps.len(), 2);
                if let FootprintRef::Name(EntityName::Ident(n)) = &fm.name.node {
                    assert_eq!(n, "R0805");
                }
            } else {
                panic!("expected FootprintMap");
            }
        }
    }

    #[test]
    fn test_footprint_map_dollar_path() {
        let f = parse(r#"
component R {
    footprint $fp.DIP8 {
        map { pin: 1, pad: 1 }
    }
}
"#);
        if let SpecItem::Component(c) = &f.items[0].node {
            if let ComponentItem::FootprintMap(fm) = &c.body[0].node {
                assert!(matches!(&fm.name.node, FootprintRef::DollarPath(_)));
            } else {
                panic!("expected FootprintMap");
            }
        }
    }

    #[test]
    fn test_component_with_graphic() {
        let f = parse(r#"
component R {
    body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil) }
}
"#);
        if let SpecItem::Component(c) = &f.items[0].node {
            if let ComponentItem::Graphic(g) = &c.body[0].node {
                assert_eq!(g.graphic_type.node, "rectangle");
                assert_eq!(g.binding.as_ref().unwrap().node, "body");
            } else {
                panic!("expected Graphic, got {:?}", c.body[0].node);
            }
        }
    }

    #[test]
    fn test_component_with_part_block() {
        let f = parse(r#"
component LM358 {
    part 1 {
        pin 1 { electrical: output }
    }
    part 2 {
        pin 5 { electrical: output }
    }
}
"#);
        if let SpecItem::Component(c) = &f.items[0].node {
            assert_eq!(c.body.len(), 2);
            if let ComponentItem::Part(pb) = &c.body[0].node {
                assert_eq!(pb.number.node, 1);
                assert_eq!(pb.body.len(), 1);
            }
        }
    }

    // ── Footprint tests ────────────────────────────────────────────────────

    #[test]
    fn test_empty_footprint() {
        let f = parse("footprint SOT23 {}");
        if let SpecItem::Footprint(fp) = &f.items[0].node {
            assert_eq!(fp.name.node.as_str(), "SOT23");
            assert!(fp.body.is_empty());
        } else {
            panic!("expected Footprint");
        }
    }

    #[test]
    fn test_footprint_with_pad() {
        let f = parse(r#"
footprint SOT23 {
    pad 1 { at: (-0.95mm, -1mm), shape: rectangular }
}
"#);
        if let SpecItem::Footprint(fp) = &f.items[0].node {
            if let FootprintItem::Pad(p) = &fp.body[0].node {
                assert_eq!(p.name.node.as_str(), "1");
            } else {
                panic!("expected Pad");
            }
        }
    }

    #[test]
    fn test_footprint_with_row() {
        let f = parse(r#"
footprint QFP32 {
    row { on: $body.left, at: center, pitch: 0.5mm, count: 8, start: 1 }
}
"#);
        if let SpecItem::Footprint(fp) = &f.items[0].node {
            assert!(matches!(&fp.body[0].node, FootprintItem::Row(_)));
        }
    }

    #[test]
    fn test_footprint_with_column() {
        let f = parse(r#"footprint X { column { pitch: 1mm, count: 4 } }"#);
        if let SpecItem::Footprint(fp) = &f.items[0].node {
            assert!(matches!(&fp.body[0].node, FootprintItem::Column(_)));
        }
    }

    #[test]
    fn test_footprint_with_grid() {
        let f = parse(r#"
footprint BGA256 {
    grid {
        origin: (0, 0)
        rows: 16, cols: 16
        pitch: 1mm
    }
}
"#);
        if let SpecItem::Footprint(fp) = &f.items[0].node {
            assert!(matches!(&fp.body[0].node, FootprintItem::Grid(_)));
        }
    }

    // ── Object tests ───────────────────────────────────────────────────────

    #[test]
    fn test_object_spread() {
        let f = parse(r#"x = { ...defaults, shape: rectangular }"#);
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            if let Expr::Object(obj) = &b.value.node {
                assert_eq!(obj.items.len(), 2);
                assert!(matches!(&obj.items[0].node, ObjectItem::Spread(_)));
                assert!(matches!(&obj.items[1].node, ObjectItem::Property(_)));
            }
        }
    }

    #[test]
    fn test_object_trailing_comma() {
        let f = parse("x = { a: 1, b: 2, }");
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            if let Expr::Object(obj) = &b.value.node {
                assert_eq!(obj.items.len(), 2);
            }
        }
    }

    #[test]
    fn test_object_newline_as_separator() {
        let f = parse("x = {\n    a: 1\n    b: 2\n}");
        if let SpecItem::LetBinding(b) = &f.items[0].node {
            if let Expr::Object(obj) = &b.value.node {
                assert_eq!(obj.items.len(), 2);
            }
        }
    }

    // ── Noise token tests ──────────────────────────────────────────────────

    #[test]
    fn test_semicolons_as_noise() {
        let f = parse("let x = 1; let y = 2;");
        assert_eq!(f.items.len(), 2);
    }

    // ── Error tests ────────────────────────────────────────────────────────

    #[test]
    fn test_error_missing_entity_name() {
        let err = parse_err("component {}");
        assert!(err.message.contains("entity name"));
    }

    #[test]
    fn test_error_missing_brace() {
        let err = parse_err("component R");
        assert!(!err.message.is_empty());
    }

    #[test]
    fn test_error_unknown_top_level() {
        let err = parse_err("42");
        assert!(!err.message.is_empty());
    }

    // ── Example 1: Basic Passive Library ──────────────────────────────────

    #[test]
    fn test_example_1_passives() {
        let src = r#"
// passives.schlib-spec
let passive_pin = { electrical: passive, length: 25, side: outside }
let two_pin_body = { from: (-20mil, -10mil), to: (20mil, 10mil), is_solid: true }

component R {
    designator: "R?"
    description: "Resistor"
    body = rectangle { ...two_pin_body }
    pin 1 { ...passive_pin, on: $body.left, at: center }
    pin 2 { ...passive_pin, on: $body.right, at: center }
    parameter Value { text: "{VALUE}" }
    footprint R0805 { map { pin: 1, pad: 1 }, map { pin: 2, pad: 2 } }
}
"#;
        let f = parse(src);
        assert_eq!(f.items.len(), 3); // 2 let + 1 component
        if let SpecItem::Component(c) = &f.items[2].node {
            assert_eq!(c.name.node.as_str(), "R");
            // designator, description, body (graphic), pin 1, pin 2, parameter, footprint
            assert_eq!(c.body.len(), 7);
        }
    }

    // ── Example 2: QFP with rows ───────────────────────────────────────────

    #[test]
    fn test_example_2_qfp() {
        let src = r#"
let qfp_pad = { shape: rectangular, x_size: 1.5mm, y_size: 0.3mm, layer: "TopLayer", hole_size: 0 }

footprint QFP32 {
    description: "32-pin QFP, 0.8mm pitch, 7x7mm body"
    height: 1.2mm
    body = rectangle { from: (-3.5mm, -3.5mm), to: (3.5mm, 3.5mm) }
    row { on: $body.left, at: center, pitch: 0.8mm, count: 8, start: 1, side: outside, pad: { ...qfp_pad } }
}
"#;
        let f = parse(src);
        assert_eq!(f.items.len(), 2); // 1 let + 1 footprint
        if let SpecItem::Footprint(fp) = &f.items[1].node {
            assert_eq!(fp.name.node.as_str(), "QFP32");
        }
    }

    // ── Example 3: BGA with grid ───────────────────────────────────────────

    #[test]
    fn test_example_3_bga() {
        let src = r#"
footprint BGA256 {
    description: "256-ball BGA, 1mm pitch"
    height: 1.5mm
    grid {
        origin: (0, 0)
        rows: 16, cols: 16
        pitch: 1mm
        naming: alphanumeric
        pad: { shape: round, x_size: 0.4mm, y_size: 0.4mm, layer: "TopLayer", hole_size: 0 }
        skip: [H8, H9, J8, J9]
    }
    pad EP { at: (0, 0), shape: rectangular, x_size: 5mm, y_size: 5mm, layer: "TopLayer" }
}
"#;
        let f = parse(src);
        if let SpecItem::Footprint(fp) = &f.items[0].node {
            assert_eq!(fp.name.node.as_str(), "BGA256");
            // description, height, grid, pad EP
            assert_eq!(fp.body.len(), 4);
        }
    }

    // ── Example 4: Multi-part IC ───────────────────────────────────────────

    #[test]
    fn test_example_4_multipart() {
        let src = r#"
import "standard-footprints.pcblib-spec" as fp

component LM358 {
    designator: "U?"
    part 1 {
        body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
        pin 1 { electrical: output }
        p2 = pin 2 { electrical: input }
        pin 3 { electrical: input }
    }
    part 2 {
        body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
        pin 5 { electrical: input }
        pin 6 { electrical: input }
        pin 7 { electrical: output }
    }
    pin 4 { electrical: power, is_hidden: true }
    pin 8 { electrical: power, is_hidden: true }
    alias LM358N
    footprint $fp.DIP8 {
        map { pin: 1, pad: 1 }
        map { pin: 2, pad: 2 }
    }
}
"#;
        let f = parse(src);
        assert_eq!(f.items.len(), 2); // import + component
        if let SpecItem::Component(c) = &f.items[1].node {
            // designator, part 1, part 2, pin 4, pin 8, alias, footprint
            assert_eq!(c.body.len(), 7);
        }
    }

    // ── Placement autoplace extension tests ────────────────────────────────

    #[test]
    fn placement_autoplace_property_in_place_block() {
        let f = parse(r#"
placement {
    place U1 { autoplace: true, region: center }
}
"#);
        if let SpecItem::Placement(p) = &f.items[0].node {
            if let PlacementItem::Place(place) = &p.body[0].node {
                assert_eq!(place.designators.len(), 1);
                assert_eq!(place.designators[0].node.as_str(), "U1");
                // body contains autoplace: true, region: center
                let body = &place.body.node;
                let has_autoplace = body.items.iter().any(|item| {
                    if let ObjectItem::Property(prop) = &item.node {
                        prop.key.node == "autoplace"
                    } else {
                        false
                    }
                });
                assert!(has_autoplace, "expected autoplace property in place body");
            } else {
                panic!("expected Place item");
            }
        } else {
            panic!("expected Placement");
        }
    }

    #[test]
    fn placement_autoplace_block_full_pipeline() {
        let f = parse(r#"
placement {
    autoplace { algorithm: full_pipeline, grid_snap: 0.5mm }
}
"#);
        if let SpecItem::Placement(p) = &f.items[0].node {
            assert!(
                p.body.iter().any(|item| matches!(item.node, PlacementItem::AutoplaceBlock(_))),
                "expected AutoplaceBlock item"
            );
        } else {
            panic!("expected Placement");
        }
    }

    #[test]
    fn placement_autoplace_block_empty() {
        let f = parse(r#"
placement {
    autoplace {}
}
"#);
        if let SpecItem::Placement(p) = &f.items[0].node {
            assert!(
                p.body.iter().any(|item| matches!(item.node, PlacementItem::AutoplaceBlock(_))),
                "expected AutoplaceBlock item even when empty"
            );
        } else {
            panic!("expected Placement");
        }
    }

    #[test]
    fn placement_unplaced_strategy_autoplace() {
        let f = parse(r#"
placement {
    unplaced: autoplace
}
"#);
        if let SpecItem::Placement(p) = &f.items[0].node {
            let found = p.body.iter().any(|item| {
                if let PlacementItem::Property(prop) = &item.node {
                    prop.key.node == "unplaced"
                } else {
                    false
                }
            });
            assert!(found, "expected unplaced property");
        } else {
            panic!("expected Placement");
        }
    }

    #[test]
    fn placement_group_decl() {
        let f = parse(r#"
placement {
    group analog { components: [U5, R10, C20] }
}
"#);
        if let SpecItem::Placement(p) = &f.items[0].node {
            let found = p.body.iter().any(|item| {
                if let PlacementItem::GroupDecl(g) = &item.node {
                    g.name.node == "analog"
                } else {
                    false
                }
            });
            assert!(found, "expected GroupDecl with name 'analog'");
        } else {
            panic!("expected Placement");
        }
    }

    #[test]
    fn placement_separate_decl() {
        let f = parse(r#"
placement {
    separate $analog, $digital { gap: 8mm }
}
"#);
        if let SpecItem::Placement(p) = &f.items[0].node {
            let found = p.body.iter().any(|item| {
                if let PlacementItem::SeparateDecl(s) = &item.node {
                    s.groups.len() == 2 && s.body.is_some()
                } else {
                    false
                }
            });
            assert!(found, "expected SeparateDecl with 2 groups and a body");
        } else {
            panic!("expected Placement");
        }
    }

    #[test]
    fn placement_no_pin_swap_in_place_block() {
        let f = parse(r#"
placement {
    place U1 { no_pin_swap: [A, B], no_part_swap: true }
}
"#);
        if let SpecItem::Placement(p) = &f.items[0].node {
            if let PlacementItem::Place(place) = &p.body[0].node {
                let has_no_pin_swap = place.body.node.items.iter().any(|item| {
                    if let ObjectItem::Property(prop) = &item.node {
                        prop.key.node == "no_pin_swap"
                    } else {
                        false
                    }
                });
                assert!(has_no_pin_swap, "expected no_pin_swap in place body");
            } else {
                panic!("expected Place item");
            }
        } else {
            panic!("expected Placement");
        }
    }

    #[test]
    fn placement_complete_block_with_all_new_properties() {
        let src = r#"
placement {
    unplaced: autoplace
    allow_pin_swap: true
    allow_part_swap: false
    allow_gate_swap: true
    autoplace { algorithm: full_pipeline, grid_snap: 0.5mm, auto_cluster: true }
    group analog { components: [U5, R10, C20] }
    group digital { components: [U1, U2] }
    separate $analog, $digital { gap: 8mm }
    place U1 { autoplace: true, region: center }
    place U5 { fixed: true, at: (10mm, 20mm) }
    place C1, C2 { autoplace: true, no_part_swap: true }
}
"#;
        let f = parse(src);
        if let SpecItem::Placement(p) = &f.items[0].node {
            let group_count = p.body.iter().filter(|item| matches!(item.node, PlacementItem::GroupDecl(_))).count();
            assert_eq!(group_count, 2, "expected 2 group declarations");
            let place_count = p.body.iter().filter(|item| matches!(item.node, PlacementItem::Place(_))).count();
            assert_eq!(place_count, 3, "expected 3 place declarations");
            let autoplace_count = p.body.iter().filter(|item| matches!(item.node, PlacementItem::AutoplaceBlock(_))).count();
            assert_eq!(autoplace_count, 1, "expected 1 autoplace block");
            let separate_count = p.body.iter().filter(|item| matches!(item.node, PlacementItem::SeparateDecl(_))).count();
            assert_eq!(separate_count, 1, "expected 1 separate declaration");
        } else {
            panic!("expected Placement");
        }
    }
}

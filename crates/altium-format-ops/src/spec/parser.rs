use crate::parser::{BinOp, ParseError, ParseErrorCode, Span, Spanned};

use super::ast::{
    AliasDecl, ComponentDecl, ComponentItem, EntityName, Expr, FootprintDecl, FootprintItem,
    FootprintMapDecl, FootprintRef, GraphicDecl, GridDecl, ImportDecl, LetBinding, MapEntry,
    Object, ObjectItem, PadDecl, ParameterDecl, PartBlock, PartItem, PinDecl, Property, RowDecl,
    SpecFile, SpecItem, is_graphic_type,
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

        Err(self.err("expected import, component, footprint, or let binding"))
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
            "expected component item (property, pin, parameter, alias, footprint, part, graphic, or let binding)",
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

    // ── Pad ────────────────────────────────────────────────────────────────

    fn parse_pad(&mut self, binding: Option<Spanned<String>>) -> Result<PadDecl, ParseError> {
        self.expect(&TokenKind::Pad, "expected 'pad'")?;
        let name = self.parse_entity_name()?;
        self.skip_newlines();
        let body = self.parse_object()?;
        Ok(PadDecl { binding, name, body })
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
            self.eat_separator();
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
    use crate::parser::Unit;
    use crate::spec::ast::*;

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
}

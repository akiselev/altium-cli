//! Structured parser for the lossless CST.
//!
//! Mirrors the token-consumption decisions of the AST recursive-descent parser
//! (`crate::parser`) but, instead of building AST structs, drives a `cstree`
//! `GreenNodeBuilder` to produce a lossless red/green tree. Trivia (whitespace,
//! comments, newlines) are emitted *interleaved* as leaf tokens at the point they
//! occur, so the defining invariant holds by construction:
//!
//! > every token is emitted exactly once, in source order
//!
//! which guarantees `parse(src).text() == src` byte-for-byte on success. On
//! malformed input the parser returns `Err` and no tree (no opaque retention).
//!
//! This milestone ports the shared grammar (expressions, objects, properties,
//! annotations) and the **SchLib** block family (import, let, component, footprint
//! and their items). Other domains (project / schdoc / pcbdoc) return a clear
//! "not yet ported" error and are added next.

use cstree::build::GreenNodeBuilder;

use crate::ast::{is_graphic_type, is_pcbdoc_block_type, is_pcbdoc_primitive_type, is_schdoc_object_type};
use crate::cst::lexer::{LosslessToken, lex_lossless};
use crate::cst::syntax::{ResolvedNode, SyntaxKind as K, SyntaxNode};
use crate::diagnostic::{ParseError, ParseErrorCode, Span};

/// Parse `source` into a lossless, structured CST.
pub fn parse_structured(source: &str) -> Result<ResolvedNode, ParseError> {
    let toks = lex_lossless(source)?;
    let mut p = Parser::new(source, toks);
    p.builder.start_node(K::Root);
    p.parse_file()?;
    p.builder.finish_node();
    if p.pos != p.toks.len() {
        return Err(p.err("unexpected trailing input"));
    }
    // Move the builder out of the parser, dropping the borrowed `source`, so the
    // resulting interner is `'static` as `new_root_with_resolver` requires.
    let Parser { builder, .. } = p;
    let (green, cache) = builder.finish();
    let interner = cache
        .expect("fresh GreenNodeBuilder owns its interner")
        .into_interner()
        .expect("owned interner is recoverable");
    Ok(SyntaxNode::new_root_with_resolver(green, interner))
}

fn is_trivia(k: K) -> bool {
    matches!(
        k,
        K::Whitespace | K::Newline | K::LineComment | K::BlockComment
    )
}

/// True for keyword tokens, which are accepted as property keys.
fn is_keyword_kind(k: K) -> bool {
    matches!(
        k,
        K::ImportKw
            | K::AsKw
            | K::ComponentKw
            | K::FootprintKw
            | K::ProjectKw
            | K::SheetKw
            | K::NetKw
            | K::PowerKw
            | K::PinKw
            | K::PadKw
            | K::PartKw
            | K::ParameterKw
            | K::AliasKw
            | K::RowKw
            | K::ColumnKw
            | K::GridKw
            | K::BoardKw
            | K::SwapGroupKw
            | K::GroupKw
            | K::SeparateKw
            | K::AutoplaceKw
            | K::PadNetKw
            | K::LetKw
            | K::TrueKw
            | K::FalseKw
            | K::NullKw
    )
}

struct Parser<'a> {
    src: &'a str,
    toks: Vec<LosslessToken>,
    pos: usize,
    builder: GreenNodeBuilder<'static, 'static, K>,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str, toks: Vec<LosslessToken>) -> Self {
        Self {
            src,
            toks,
            pos: 0,
            builder: GreenNodeBuilder::new(),
        }
    }

    // ── Token cursor (skips trivia for lookahead; emits it on consume) ────────

    /// Emit any pending trivia tokens into the currently-open node.
    fn flush_trivia(&mut self) {
        while self.pos < self.toks.len() && is_trivia(self.toks[self.pos].kind) {
            let t = &self.toks[self.pos];
            self.builder.token(t.kind, &self.src[t.range.clone()]);
            self.pos += 1;
        }
    }

    /// Kind of the n-th meaningful token ahead (0 = current), or None at EOF.
    fn nth(&self, n: usize) -> Option<K> {
        let mut i = self.pos;
        let mut c = 0;
        while i < self.toks.len() {
            let k = self.toks[i].kind;
            if !is_trivia(k) {
                if c == n {
                    return Some(k);
                }
                c += 1;
            }
            i += 1;
        }
        None
    }

    /// Source text of the n-th meaningful token ahead.
    fn nth_text(&self, n: usize) -> Option<&str> {
        let mut i = self.pos;
        let mut c = 0;
        while i < self.toks.len() {
            let t = &self.toks[i];
            if !is_trivia(t.kind) {
                if c == n {
                    return Some(&self.src[t.range.clone()]);
                }
                c += 1;
            }
            i += 1;
        }
        None
    }

    fn nth_span(&self, n: usize) -> Option<Span> {
        let mut i = self.pos;
        let mut c = 0;
        while i < self.toks.len() {
            let t = &self.toks[i];
            if !is_trivia(t.kind) {
                if c == n {
                    return Some(Span::new(t.range.start as u32, t.range.end as u32));
                }
                c += 1;
            }
            i += 1;
        }
        None
    }

    fn cur(&self) -> Option<K> {
        self.nth(0)
    }
    fn cur_text(&self) -> Option<&str> {
        self.nth_text(0)
    }
    fn at(&self, k: K) -> bool {
        self.cur() == Some(k)
    }
    fn at_eof(&self) -> bool {
        self.cur().is_none()
    }
    fn at_ident(&self, s: &str) -> bool {
        self.at(K::Ident) && self.cur_text() == Some(s)
    }
    fn cur_is_graphic(&self) -> bool {
        self.at(K::Ident) && self.cur_text().is_some_and(is_graphic_type)
    }

    /// Emit pending trivia, then emit the current meaningful token. Caller must
    /// ensure `!at_eof()`.
    fn bump(&mut self) {
        self.flush_trivia();
        debug_assert!(self.pos < self.toks.len(), "bump past EOF");
        let t = &self.toks[self.pos];
        self.builder.token(t.kind, &self.src[t.range.clone()]);
        self.pos += 1;
    }

    fn eat(&mut self, k: K) -> bool {
        if self.at(k) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, k: K, msg: &str) -> Result<(), ParseError> {
        if self.at(k) {
            self.bump();
            Ok(())
        } else {
            Err(self.err(msg))
        }
    }

    /// Eat any run of explicit `,`/`;` separators (newlines are trivia).
    fn eat_separators(&mut self) {
        while self.at(K::Comma) || self.at(K::Semi) {
            self.bump();
        }
    }

    fn err(&self, msg: impl Into<String>) -> ParseError {
        let span = self.nth_span(0).unwrap_or_else(|| {
            let n = self.src.len() as u32;
            Span::new(n, n)
        });
        ParseError::new(ParseErrorCode::E1002, msg, span)
    }

    // ── File / item dispatch ──────────────────────────────────────────────────

    fn parse_file(&mut self) -> Result<(), ParseError> {
        loop {
            self.flush_trivia();
            if self.at_eof() {
                break;
            }
            self.parse_spec_item()?;
        }
        Ok(())
    }

    fn parse_spec_item(&mut self) -> Result<(), ParseError> {
        let cp = self.builder.checkpoint();

        // import (never annotated)
        if self.at(K::ImportKw) {
            self.builder.start_node_at(cp, K::Import);
            self.parse_import()?;
            self.builder.finish_node();
            return Ok(());
        }

        // optional block annotation
        if self.at(K::Hash) {
            self.parse_annotation()?;
        }

        // unannotated/annotated keyword-led blocks
        if self.at(K::ComponentKw) {
            self.builder.start_node_at(cp, K::Component);
            self.parse_component_decl()?;
            self.builder.finish_node();
            return Ok(());
        }
        if self.at(K::FootprintKw) {
            self.builder.start_node_at(cp, K::Footprint);
            self.parse_footprint_decl()?;
            self.builder.finish_node();
            return Ok(());
        }
        if self.at(K::SwapGroupKw) && self.nth(1) != Some(K::Colon) {
            self.builder.start_node_at(cp, K::SwapGroup);
            self.parse_swap_group_decl()?;
            self.builder.finish_node();
            return Ok(());
        }

        // Domains not yet ported in the CST (SchLib-only milestone).
        if self.not_yet_ported_top_level() {
            return Err(self.err(
                "CST parser: this block type is not yet ported (SchLib component/footprint only for now)",
            ));
        }

        // [let] IDENT = (component|footprint|swap_group|expr)
        let has_let = self.at(K::LetKw);
        let id = if has_let { 1 } else { 0 };
        if self.nth(id) == Some(K::Ident) && self.nth(id + 1) == Some(K::Eq) {
            match self.nth(id + 2) {
                Some(K::ComponentKw) => {
                    self.builder.start_node_at(cp, K::Component);
                    self.eat(K::LetKw);
                    self.emit_binding()?;
                    self.parse_component_decl()?;
                    self.builder.finish_node();
                }
                Some(K::FootprintKw) => {
                    self.builder.start_node_at(cp, K::Footprint);
                    self.eat(K::LetKw);
                    self.emit_binding()?;
                    self.parse_footprint_decl()?;
                    self.builder.finish_node();
                }
                Some(K::SwapGroupKw) => {
                    self.builder.start_node_at(cp, K::SwapGroup);
                    self.eat(K::LetKw);
                    self.emit_binding()?;
                    self.parse_swap_group_decl()?;
                    self.builder.finish_node();
                }
                Some(K::ProjectKw) => {
                    return Err(self.err("CST parser: project blocks not yet ported"));
                }
                _ => {
                    self.builder.start_node_at(cp, K::LetBinding);
                    self.eat(K::LetKw);
                    self.bump(); // IDENT
                    self.expect(K::Eq, "expected '=' in let binding")?;
                    self.parse_expr()?;
                    self.builder.finish_node();
                }
            }
            return Ok(());
        }

        if has_let {
            return Err(self.err("expected identifier after 'let'"));
        }
        Err(self.err("expected import, component, footprint, swap_group, or let binding"))
    }

    fn not_yet_ported_top_level(&self) -> bool {
        if matches!(
            self.cur(),
            Some(
                K::ProjectKw
                    | K::SheetKw
                    | K::NetKw
                    | K::PowerKw
                    | K::BoardKw
                    | K::PadKw
                    | K::ParameterKw
            )
        ) {
            return true;
        }
        if self.at_ident("placement") || self.at_ident("routing") {
            return true;
        }
        if let Some(t) = self.cur_text() {
            if self.at(K::Ident)
                && (is_pcbdoc_block_type(t)
                    || is_pcbdoc_primitive_type(t)
                    || is_schdoc_object_type(t)
                    || is_graphic_type(t))
            {
                return true;
            }
        }
        false
    }

    // ── Import ────────────────────────────────────────────────────────────────

    fn parse_import(&mut self) -> Result<(), ParseError> {
        self.expect(K::ImportKw, "expected 'import'")?;
        if !self.at(K::String) {
            return Err(self.err("expected file path string after 'import'"));
        }
        self.bump();
        if self.eat(K::AsKw) {
            if !self.at(K::Ident) {
                return Err(self.err("expected import alias identifier after 'as'"));
            }
            self.bump();
        }
        Ok(())
    }

    // ── Annotation ────────────────────────────────────────────────────────────

    fn parse_annotation(&mut self) -> Result<(), ParseError> {
        self.builder.start_node(K::Annotation);
        self.expect(K::Hash, "expected '#'")?;
        self.expect(K::LBracket, "expected '[' after '#'")?;
        if !self.at_ident("annotation") {
            return Err(self.err("expected 'annotation' after '#['"));
        }
        self.bump();
        self.expect(K::LParen, "expected '(' after 'annotation'")?;
        loop {
            self.flush_trivia();
            if self.at(K::RParen) || self.at_eof() {
                break;
            }
            self.builder.start_node(K::AnnotationArg);
            // key: identifier or the `group` keyword
            if self.at(K::Ident) || self.at(K::GroupKw) {
                self.bump();
            } else {
                return Err(self.err("expected annotation key"));
            }
            self.expect(K::Eq, "expected '=' after annotation key")?;
            // value: string or boolean
            if matches!(self.cur(), Some(K::String | K::TrueKw | K::FalseKw)) {
                self.bump();
            } else {
                return Err(self.err("expected string or boolean annotation value"));
            }
            self.builder.finish_node(); // AnnotationArg
            if !self.eat(K::Comma) {
                break;
            }
        }
        self.expect(K::RParen, "expected ')' to close annotation")?;
        self.expect(K::RBracket, "expected ']' to close annotation")?;
        self.builder.finish_node(); // Annotation
        Ok(())
    }

    /// Emit a `Binding` node wrapping `IDENT =`.
    fn emit_binding(&mut self) -> Result<(), ParseError> {
        self.builder.start_node(K::Binding);
        if !self.at(K::Ident) {
            return Err(self.err("expected binding name"));
        }
        self.bump();
        self.expect(K::Eq, "expected '=' after binding name")?;
        self.builder.finish_node();
        Ok(())
    }

    // ── Component ─────────────────────────────────────────────────────────────

    fn parse_component_decl(&mut self) -> Result<(), ParseError> {
        self.expect(K::ComponentKw, "expected 'component'")?;
        self.parse_name()?;
        self.parse_block(Self::parse_component_item)
    }

    fn parse_component_item(&mut self) -> Result<(), ParseError> {
        let cp = self.builder.checkpoint();
        match self.cur() {
            Some(K::PartKw) => {
                self.builder.start_node_at(cp, K::Part);
                self.parse_part_decl()?;
                self.builder.finish_node();
                Ok(())
            }
            Some(K::PinKw) => {
                let is_conn = matches!(self.nth(1), Some(K::Ident | K::Int | K::String))
                    && self.nth(2) == Some(K::Arrow);
                if is_conn {
                    self.builder.start_node_at(cp, K::PinConnection);
                    self.bump(); // pin
                    self.bump(); // name
                    self.expect(K::Arrow, "expected '->'")?;
                    self.builder.start_node(K::NetTarget);
                    if self.at(K::Hash) {
                        self.bump();
                        if !self.at(K::Ident) {
                            return Err(self.err("expected net name after '#'"));
                        }
                        self.bump();
                    } else if self.at_ident("nc") {
                        self.bump();
                    } else {
                        return Err(self.err("expected '#NET' or 'nc' after '->'"));
                    }
                    self.builder.finish_node(); // NetTarget
                    self.builder.finish_node(); // PinConnection
                    Ok(())
                } else {
                    self.builder.start_node_at(cp, K::Pin);
                    self.parse_pin_decl()?;
                    self.builder.finish_node();
                    Ok(())
                }
            }
            Some(K::ParameterKw) => {
                self.builder.start_node_at(cp, K::Parameter);
                self.parse_parameter_decl()?;
                self.builder.finish_node();
                Ok(())
            }
            Some(K::AliasKw) => {
                self.builder.start_node_at(cp, K::Alias);
                self.parse_alias_decl()?;
                self.builder.finish_node();
                Ok(())
            }
            Some(K::FootprintKw) => {
                self.builder.start_node_at(cp, K::FootprintMap);
                self.parse_footprint_map_decl()?;
                self.builder.finish_node();
                Ok(())
            }
            Some(K::LetKw) => self.wrap_let_binding(cp),
            Some(K::SwapGroupKw) => {
                if self.nth(1) == Some(K::Colon) {
                    self.parse_property()
                } else {
                    self.builder.start_node_at(cp, K::SwapGroup);
                    self.parse_swap_group_decl()?;
                    self.builder.finish_node();
                    Ok(())
                }
            }
            Some(K::PadNetKw) => {
                self.builder.start_node_at(cp, K::PadNet);
                self.bump(); // pad_net
                self.parse_name()?;
                self.expect(K::Colon, "expected ':' after pad name in pad_net")?;
                if !self.at(K::String) {
                    return Err(self.err("expected string literal for net name in pad_net"));
                }
                self.bump();
                self.builder.finish_node();
                Ok(())
            }
            Some(K::Ident) => self.parse_ident_led_item(cp, ItemScope::Component),
            _ => Err(self.err("expected component item")),
        }
    }

    // ── Part ──────────────────────────────────────────────────────────────────

    fn parse_part_decl(&mut self) -> Result<(), ParseError> {
        self.expect(K::PartKw, "expected 'part'")?;
        if !self.at(K::Int) {
            return Err(self.err("expected integer part number after 'part'"));
        }
        self.bump();
        self.parse_block(Self::parse_part_item)
    }

    fn parse_part_item(&mut self) -> Result<(), ParseError> {
        let cp = self.builder.checkpoint();
        match self.cur() {
            Some(K::PinKw) => {
                self.builder.start_node_at(cp, K::Pin);
                self.parse_pin_decl()?;
                self.builder.finish_node();
                Ok(())
            }
            Some(K::LetKw) => self.wrap_let_binding(cp),
            Some(K::SwapGroupKw) if self.nth(1) == Some(K::Colon) => self.parse_property(),
            Some(K::Ident) => self.parse_ident_led_item(cp, ItemScope::Part),
            _ => Err(self.err("expected part item (pin, graphic, property, or let binding)")),
        }
    }

    // ── Footprint ─────────────────────────────────────────────────────────────

    fn parse_footprint_decl(&mut self) -> Result<(), ParseError> {
        self.expect(K::FootprintKw, "expected 'footprint'")?;
        self.parse_name()?;
        self.parse_block(Self::parse_footprint_item)
    }

    fn parse_footprint_item(&mut self) -> Result<(), ParseError> {
        let cp = self.builder.checkpoint();
        match self.cur() {
            Some(K::PadKw) => {
                self.builder.start_node_at(cp, K::Pad);
                self.parse_pad_decl()?;
                self.builder.finish_node();
                Ok(())
            }
            Some(K::RowKw) => self.wrap_keyword_object(cp, K::Row),
            Some(K::ColumnKw) => self.wrap_keyword_object(cp, K::Column),
            Some(K::GridKw) => self.wrap_keyword_object(cp, K::Grid),
            Some(K::LetKw) => self.wrap_let_binding(cp),
            Some(K::Ident) => self.parse_ident_led_item(cp, ItemScope::Footprint),
            _ => Err(self.err("expected footprint item")),
        }
    }

    /// `row {...}` / `column {...}` / `grid {...}` — keyword then object body.
    fn wrap_keyword_object(&mut self, cp: cstree::build::Checkpoint, kind: K) -> Result<(), ParseError> {
        self.builder.start_node_at(cp, kind);
        self.bump(); // keyword
        self.parse_object()?;
        self.builder.finish_node();
        Ok(())
    }

    // ── Footprint map (inside component) ──────────────────────────────────────

    fn parse_footprint_map_decl(&mut self) -> Result<(), ParseError> {
        self.expect(K::FootprintKw, "expected 'footprint'")?;
        if self.at(K::DollarIdent) {
            self.parse_dollar_path()?;
        } else {
            self.parse_name()?;
        }
        if !self.at(K::LBrace) {
            return Ok(()); // implicit 1:1 mapping
        }
        self.builder.start_node(K::Block);
        self.expect(K::LBrace, "expected '{'")?;
        loop {
            self.flush_trivia();
            if self.at(K::RBrace) || self.at_eof() {
                break;
            }
            if self.at_ident("description") {
                self.parse_property()?;
            } else {
                self.builder.start_node(K::PinPadPair);
                self.parse_pin_pad_ref(K::PinKw)?;
                self.expect(K::Colon, "expected ':' between pin and pad references")?;
                self.parse_pin_pad_ref(K::PadKw)?;
                self.builder.finish_node();
            }
            self.eat_separators();
        }
        self.expect(K::RBrace, "expected '}' to close footprint mapping")?;
        self.builder.finish_node();
        Ok(())
    }

    fn parse_pin_pad_ref(&mut self, kw: K) -> Result<(), ParseError> {
        if self.at(kw) {
            self.bump();
            self.parse_name()
        } else {
            self.parse_dollar_path()
        }
    }

    // ── Simple decls (keyword + name + object body) ───────────────────────────

    fn parse_pin_decl(&mut self) -> Result<(), ParseError> {
        self.expect(K::PinKw, "expected 'pin'")?;
        self.parse_name()?;
        self.parse_object()
    }

    fn parse_parameter_decl(&mut self) -> Result<(), ParseError> {
        self.expect(K::ParameterKw, "expected 'parameter'")?;
        self.parse_name()?;
        self.parse_object()
    }

    fn parse_pad_decl(&mut self) -> Result<(), ParseError> {
        self.expect(K::PadKw, "expected 'pad'")?;
        self.parse_name()?;
        self.parse_object()
    }

    fn parse_swap_group_decl(&mut self) -> Result<(), ParseError> {
        self.expect(K::SwapGroupKw, "expected 'swap_group'")?;
        self.parse_name()?;
        self.parse_object()
    }

    fn parse_alias_decl(&mut self) -> Result<(), ParseError> {
        self.expect(K::AliasKw, "expected 'alias'")?;
        self.parse_name()
    }

    /// `GRAPHIC_TYPE { ... }` — graphic type identifier then object body.
    fn parse_graphic_decl(&mut self) -> Result<(), ParseError> {
        if !self.cur_is_graphic() {
            return Err(self.err("expected graphic type identifier"));
        }
        self.bump(); // graphic type ident
        self.parse_object()
    }

    // ── Shared ident-led item logic (component / part / footprint) ────────────

    fn parse_ident_led_item(
        &mut self,
        cp: cstree::build::Checkpoint,
        scope: ItemScope,
    ) -> Result<(), ParseError> {
        let next = self.nth(1);
        if next == Some(K::Colon) {
            return self.parse_property();
        }
        if next == Some(K::Eq) {
            // binding to a keyword decl, or a plain `name = expr` let binding
            match (scope, self.nth(2)) {
                (ItemScope::Component | ItemScope::Part, Some(K::PinKw)) => {
                    self.builder.start_node_at(cp, K::Pin);
                    self.emit_binding()?;
                    self.parse_pin_decl()?;
                    self.builder.finish_node();
                }
                (ItemScope::Component, Some(K::ParameterKw)) => {
                    self.builder.start_node_at(cp, K::Parameter);
                    self.emit_binding()?;
                    self.parse_parameter_decl()?;
                    self.builder.finish_node();
                }
                (ItemScope::Component, Some(K::PartKw)) => {
                    self.builder.start_node_at(cp, K::Part);
                    self.emit_binding()?;
                    self.parse_part_decl()?;
                    self.builder.finish_node();
                }
                (ItemScope::Component, Some(K::SwapGroupKw)) => {
                    self.builder.start_node_at(cp, K::SwapGroup);
                    self.emit_binding()?;
                    self.parse_swap_group_decl()?;
                    self.builder.finish_node();
                }
                (ItemScope::Footprint, Some(K::PadKw)) => {
                    self.builder.start_node_at(cp, K::Pad);
                    self.emit_binding()?;
                    self.parse_pad_decl()?;
                    self.builder.finish_node();
                }
                (_, Some(K::Ident)) if self.nth_text(2).is_some_and(is_graphic_type) => {
                    self.builder.start_node_at(cp, K::Graphic);
                    self.emit_binding()?;
                    self.parse_graphic_decl()?;
                    self.builder.finish_node();
                }
                _ => {
                    self.builder.start_node_at(cp, K::LetBinding);
                    self.bump(); // IDENT
                    self.expect(K::Eq, "expected '=' in let binding")?;
                    self.parse_expr()?;
                    self.builder.finish_node();
                }
            }
            return Ok(());
        }
        if self.cur_is_graphic() {
            self.builder.start_node_at(cp, K::Graphic);
            self.parse_graphic_decl()?;
            self.builder.finish_node();
            return Ok(());
        }
        Err(self.err("expected item (property, declaration, graphic, or let binding)"))
    }

    fn wrap_let_binding(&mut self, cp: cstree::build::Checkpoint) -> Result<(), ParseError> {
        self.builder.start_node_at(cp, K::LetBinding);
        self.eat(K::LetKw);
        if !self.at(K::Ident) {
            return Err(self.err("expected identifier in let binding"));
        }
        self.bump();
        self.expect(K::Eq, "expected '=' after binding name")?;
        self.parse_expr()?;
        self.builder.finish_node();
        Ok(())
    }

    // ── Name / block / object / property ──────────────────────────────────────

    fn parse_name(&mut self) -> Result<(), ParseError> {
        if !matches!(self.cur(), Some(K::String | K::Int | K::Ident)) {
            return Err(self.err("expected entity name (identifier, string, or integer)"));
        }
        self.builder.start_node(K::Name);
        self.bump();
        self.builder.finish_node();
        Ok(())
    }

    /// `{ item* }` item-list body (braces included), using the given item parser.
    fn parse_block(
        &mut self,
        item: fn(&mut Self) -> Result<(), ParseError>,
    ) -> Result<(), ParseError> {
        self.builder.start_node(K::Block);
        self.expect(K::LBrace, "expected '{'")?;
        loop {
            self.flush_trivia();
            if self.at(K::RBrace) || self.at_eof() {
                break;
            }
            item(self)?;
            self.eat_separators();
        }
        self.expect(K::RBrace, "expected '}'")?;
        self.builder.finish_node();
        Ok(())
    }

    /// `{ key: value | ...expr | let x = e }` object literal (braces included).
    fn parse_object(&mut self) -> Result<(), ParseError> {
        self.builder.start_node(K::Object);
        self.expect(K::LBrace, "expected '{'")?;
        loop {
            self.flush_trivia();
            if self.at(K::RBrace) || self.at_eof() {
                break;
            }
            self.parse_object_item()?;
            self.eat_separators();
        }
        self.expect(K::RBrace, "expected '}'")?;
        self.builder.finish_node();
        Ok(())
    }

    fn parse_object_item(&mut self) -> Result<(), ParseError> {
        if self.at(K::DotDotDot) {
            self.builder.start_node(K::Spread);
            self.bump();
            self.parse_expr()?;
            self.builder.finish_node();
            return Ok(());
        }
        if self.at(K::LetKw) {
            let cp = self.builder.checkpoint();
            return self.wrap_let_binding(cp);
        }
        if self.nth(1) == Some(K::Colon) && self.is_property_key() {
            return self.parse_property();
        }
        if self.at(K::Ident) && self.nth(1) == Some(K::Eq) {
            let cp = self.builder.checkpoint();
            return self.wrap_let_binding(cp);
        }
        Err(self.err("expected object item (property, spread, or let binding)"))
    }

    fn is_property_key(&self) -> bool {
        match self.cur() {
            Some(K::Ident) => true,
            Some(k) => is_keyword_kind(k),
            None => false,
        }
    }

    fn parse_property(&mut self) -> Result<(), ParseError> {
        if !self.is_property_key() {
            return Err(self.err("expected property key (identifier or keyword)"));
        }
        self.builder.start_node(K::Property);
        self.bump(); // key
        self.expect(K::Colon, "expected ':' after property key")?;
        self.parse_expr()?;
        self.builder.finish_node();
        Ok(())
    }

    // ── Dollar path ───────────────────────────────────────────────────────────

    fn parse_dollar_path(&mut self) -> Result<(), ParseError> {
        if !self.at(K::DollarIdent) {
            return Err(self.err("expected '$name' reference"));
        }
        self.builder.start_node(K::DollarPath);
        self.bump();
        loop {
            if self.eat(K::Dot) {
                if !self.at(K::Ident) {
                    return Err(self.err("expected field name after '.'"));
                }
                self.bump();
            } else if self.eat(K::LBracket) {
                self.parse_expr()?;
                self.expect(K::RBracket, "expected ']'")?;
            } else {
                break;
            }
        }
        self.builder.finish_node();
        Ok(())
    }

    // ── Expressions (Pratt) ───────────────────────────────────────────────────

    fn parse_expr(&mut self) -> Result<(), ParseError> {
        self.parse_pratt(0)
    }

    fn parse_pratt(&mut self, min_bp: u8) -> Result<(), ParseError> {
        let cp = self.builder.checkpoint();
        self.parse_prefix()?;
        loop {
            let (lbp, rbp, kind) = match self.cur() {
                Some(K::Dot) => (90u8, 91u8, K::PathExpr),
                Some(K::LBracket) => (90, 91, K::IndexExpr),
                Some(K::Star | K::Slash) => (60, 61, K::BinExpr),
                Some(K::Plus | K::Minus) => (50, 51, K::BinExpr),
                _ => break,
            };
            if lbp < min_bp {
                break;
            }
            match kind {
                K::PathExpr => {
                    self.builder.start_node_at(cp, K::PathExpr);
                    self.bump(); // .
                    if !self.at(K::Ident) {
                        return Err(self.err("expected field name after '.'"));
                    }
                    self.bump();
                    self.builder.finish_node();
                }
                K::IndexExpr => {
                    self.builder.start_node_at(cp, K::IndexExpr);
                    self.bump(); // [
                    self.parse_pratt(0)?;
                    self.expect(K::RBracket, "expected ']'")?;
                    self.builder.finish_node();
                }
                _ => {
                    self.builder.start_node_at(cp, K::BinExpr);
                    self.bump(); // operator
                    self.parse_pratt(rbp)?;
                    self.builder.finish_node();
                }
            }
        }
        Ok(())
    }

    fn parse_prefix(&mut self) -> Result<(), ParseError> {
        let cp = self.builder.checkpoint();
        match self.cur() {
            Some(
                K::String | K::Template | K::Int | K::Float | K::Dim | K::Color | K::TrueKw
                | K::FalseKw | K::NullKw | K::DollarIdent,
            ) => {
                self.bump();
                Ok(())
            }
            // Keywords usable as identifier values in expressions.
            Some(K::PowerKw | K::NetKw | K::SheetKw | K::AutoplaceKw | K::GroupKw | K::SeparateKw) => {
                self.bump();
                Ok(())
            }
            Some(K::Ident) => {
                self.bump();
                if self.at(K::LParen) {
                    self.builder.start_node_at(cp, K::CallExpr);
                    self.parse_call_args()?;
                    self.builder.finish_node();
                }
                Ok(())
            }
            Some(K::Minus) => {
                self.builder.start_node_at(cp, K::UnaryExpr);
                self.bump(); // -
                self.parse_pratt(70)?;
                self.builder.finish_node();
                Ok(())
            }
            Some(K::LParen) => {
                self.bump(); // (
                self.parse_pratt(0)?;
                if self.at(K::Comma) {
                    self.builder.start_node_at(cp, K::TupleExpr);
                    self.bump(); // ,
                    self.parse_pratt(0)?;
                    self.eat(K::Comma); // optional trailing comma
                    self.expect(K::RParen, "expected ')' to close tuple")?;
                    self.builder.finish_node();
                } else {
                    self.builder.start_node_at(cp, K::ParenExpr);
                    self.expect(K::RParen, "expected ')'")?;
                    self.builder.finish_node();
                }
                Ok(())
            }
            Some(K::LBracket) => {
                self.builder.start_node_at(cp, K::ArrayExpr);
                self.bump(); // [
                loop {
                    self.flush_trivia();
                    if self.at(K::RBracket) || self.at_eof() {
                        break;
                    }
                    self.parse_pratt(0)?;
                    if !self.eat(K::Comma) {
                        break;
                    }
                }
                self.expect(K::RBracket, "expected ']'")?;
                self.builder.finish_node();
                Ok(())
            }
            Some(K::LBrace) => self.parse_object(),
            _ => Err(self.err("expected expression")),
        }
    }

    fn parse_call_args(&mut self) -> Result<(), ParseError> {
        self.expect(K::LParen, "expected '(' for function call")?;
        loop {
            self.flush_trivia();
            if self.at(K::RParen) || self.at_eof() {
                break;
            }
            self.builder.start_node(K::CallArg);
            if self.at(K::Ident) && self.nth(1) == Some(K::Colon) {
                self.bump(); // name
                self.expect(K::Colon, "expected ':' after argument name")?;
            }
            self.parse_pratt(0)?;
            self.builder.finish_node();
            if !self.eat(K::Comma) {
                break;
            }
        }
        self.expect(K::RParen, "expected ')' to close function call")?;
        Ok(())
    }
}

/// Which container an ident-led item belongs to (controls which `name = KEYWORD`
/// binding forms are accepted).
#[derive(Clone, Copy, PartialEq, Eq)]
enum ItemScope {
    Component,
    Part,
    Footprint,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_roundtrip(src: &str) {
        let root = parse_structured(src)
            .unwrap_or_else(|e| panic!("structured parse failed: {e}\nsource:\n{src}"));
        assert_eq!(root.text(), src, "structured CST must equal source byte-for-byte");
    }

    #[test]
    fn structured_minimal_component() {
        assert_roundtrip("component R_0603 {}");
    }

    #[test]
    fn structured_component_with_items() {
        let src = "\
// a resistor
component R_0603 {
    designator: \"R\"
    description: \"SMD resistor 0603\"
    pin 1 { at: (100mil, 0mil), electrical: passive }
    pin 2 { at: (-100mil, 0mil), electrical: passive }
    parameter \"Value\" { text: \"10k\" }
    footprint C0603
    alias R_0603_ALT
}
";
        assert_roundtrip(src);
    }

    #[test]
    fn structured_footprint_with_pads() {
        let src = "footprint C0603 {\n    pad 1 { shape: rect, at: (0mil, 0mil) }\n    row { count: 2 }\n}\n";
        assert_roundtrip(src);
    }

    #[test]
    fn structured_bindings_and_let() {
        let src = "let w = 10mil\nmy_r = component R { width: w + 2mil }\n";
        assert_roundtrip(src);
    }

    #[test]
    fn structured_import_and_annotation() {
        let src = "import \"std.schlib\" as std\n#[annotation(id = \"AB12CD34\", stable = true)]\ncomponent X {}\n";
        assert_roundtrip(src);
    }

    #[test]
    fn structured_expressions() {
        let src = "component X { a: 1 + 2 * 3, b: $ref.field[0], c: foo(1, key: 2), d: [1, 2, 3] }\n";
        assert_roundtrip(src);
    }

    #[test]
    fn structured_pin_connection_and_footprint_map() {
        let src = "component U1 {\n    pin GPIO4 -> #NET1\n    pin 2 -> nc\n    footprint $fp { pin \"1\": pad \"3\" }\n}\n";
        assert_roundtrip(src);
    }
}

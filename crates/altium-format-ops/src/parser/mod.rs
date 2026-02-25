mod ast;
mod diagnostic;
mod lexer;
mod selector;
mod typecheck;

pub use ast::*;
pub use diagnostic::*;
pub use typecheck::{
    OpsDomain, compile_ops_to_high, compile_ops_to_high_pcbdoc, compile_ops_to_high_pcblib,
    compile_ops_to_high_schdoc, compile_ops_to_high_schlib,
};

use lexer::{Token, TokenKind};

pub fn parse_ops(source: &str) -> Result<OpsFile, ParseError> {
    let tokens = lexer::lex(source)?;
    Parser::new(source, tokens).parse_file()
}

struct Parser<'a> {
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

    fn parse_file(&mut self) -> Result<ast::OpsFile, ParseError> {
        let mut statements = Vec::new();
        self.skip_separators();
        while !self.at(&TokenKind::Eof) {
            statements.push(self.parse_statement()?);
            self.consume_semicolons();
            self.skip_separators();
        }
        Ok(OpsFile { statements })
    }

    fn parse_statement(&mut self) -> Result<ast::Spanned<ast::Statement>, ParseError> {
        let start = self.current().span;
        let stmt = if self.at(&TokenKind::Assert) {
            ast::Statement::Assert(self.parse_assert_stmt()?)
        } else if self.at(&TokenKind::Let) || self.looks_like_binding() {
            ast::Statement::Binding(self.parse_binding(true)?)
        } else {
            ast::Statement::Op(self.parse_op()?)
        };
        let end = self.prev().span;
        Ok(ast::Spanned::new(stmt, start.merge(end)))
    }

    fn parse_assert_stmt(&mut self) -> Result<ast::AssertStmt, ParseError> {
        self.expect(TokenKind::Assert, "expected 'assert'")?;
        self.skip_newlines();

        let had_paren = self.consume_if(&TokenKind::LParen);
        let left = self.parse_expr_with_stops(&[TokenKind::Comma, TokenKind::RParen], true)?;

        let condition = match self.current_kind() {
            TokenKind::EqEq
            | TokenKind::Ne
            | TokenKind::Gt
            | TokenKind::Lt
            | TokenKind::Ge
            | TokenKind::Le => {
                let op_tok = self.bump().clone();
                let right =
                    self.parse_expr_with_stops(&[TokenKind::Comma, TokenKind::RParen], true)?;
                let op = match op_tok.kind {
                    TokenKind::EqEq => CompareOp::Eq,
                    TokenKind::Ne => CompareOp::Ne,
                    TokenKind::Gt => CompareOp::Gt,
                    TokenKind::Lt => CompareOp::Lt,
                    TokenKind::Ge => CompareOp::Ge,
                    TokenKind::Le => CompareOp::Le,
                    _ => unreachable!(),
                };
                let span = left.span.merge(right.span);
                ast::Spanned::new(
                    ast::AssertCondition::Comparison {
                        left,
                        op: ast::Spanned::new(op, op_tok.span),
                        right,
                    },
                    span,
                )
            }
            _ => {
                let span = left.span;
                ast::Spanned::new(ast::AssertCondition::Existence(left), span)
            }
        };

        if had_paren {
            self.expect(TokenKind::RParen, "expected ')' after assert condition")?;
        }

        let message = if self.consume_if(&TokenKind::Comma) {
            Some(self.parse_expr_with_stops(&[TokenKind::Semicolon], true)?)
        } else {
            None
        };

        Ok(ast::AssertStmt { condition, message })
    }

    fn parse_binding(&mut self, allow_let: bool) -> Result<ast::Binding, ParseError> {
        if allow_let {
            self.consume_if(&TokenKind::Let);
        }
        let name_tok = self.expect_ident("expected binding name")?;
        self.expect(TokenKind::Eq, "expected '=' after binding name")?;

        let value_start = self.current().span;
        let value = if self.looks_like_op_rhs() {
            let op = self.parse_op()?;
            let span = value_start.merge(self.prev().span);
            ast::Spanned::new(ast::BindingValue::Op(op), span)
        } else {
            let expr = self.parse_expr_with_stops(&[TokenKind::Semicolon], true)?;
            let span = expr.span;
            ast::Spanned::new(ast::BindingValue::Expr(expr.node), span)
        };

        Ok(ast::Binding {
            name: ast::Spanned::new(name_tok.node, name_tok.span),
            value,
        })
    }

    fn parse_op(&mut self) -> Result<ast::Op, ParseError> {
        let name = self.expect_ident_or_keyword_op("expected operation name")?;
        match name.node.as_str() {
            "edit" => self.parse_edit_op(name),
            "remove" => self.parse_selector_only_op(name),
            "query" => self.parse_selector_only_op(name),
            _ => self.parse_create_op(name),
        }
    }

    fn parse_create_op(&mut self, name: ast::Spanned<String>) -> Result<ast::Op, ParseError> {
        self.skip_newlines();

        let target = if !self.at(&TokenKind::LBrace) {
            let expr = self.parse_expr_with_stops(&[TokenKind::LBrace], true)?;
            Some(expr)
        } else {
            None
        };

        let body = if self.at(&TokenKind::LBrace) {
            Some(self.parse_object()?)
        } else {
            return Err(self.error_here(
                ParseErrorCode::E1004,
                "create operation requires an object body",
                Some("expected '{ ... }' after operation name".to_string()),
            ));
        };

        Ok(ast::Op {
            name,
            target,
            selector: None,
            body,
        })
    }

    fn parse_edit_op(&mut self, name: ast::Spanned<String>) -> Result<ast::Op, ParseError> {
        self.skip_newlines();
        let selector = self.parse_selector_until_lbrace()?;
        let body = Some(self.parse_object()?);
        Ok(ast::Op {
            name,
            target: None,
            selector: Some(selector),
            body,
        })
    }

    fn parse_selector_only_op(
        &mut self,
        name: ast::Spanned<String>,
    ) -> Result<ast::Op, ParseError> {
        self.skip_newlines();
        let selector = self.parse_selector_until_statement_end()?;
        Ok(ast::Op {
            name,
            target: None,
            selector: Some(selector),
            body: None,
        })
    }

    fn parse_selector_until_lbrace(&mut self) -> Result<ast::Spanned<ast::Selector>, ParseError> {
        let start = self.current().span.start as usize;
        let mut end = start;
        while !self.at(&TokenKind::LBrace) && !self.at(&TokenKind::Eof) {
            end = self.current().span.end as usize;
            self.bump();
        }

        if self.at(&TokenKind::Eof) {
            return Err(self.error_here(
                ParseErrorCode::E1004,
                "unterminated selector in edit operation",
                Some("expected '{' after selector".to_string()),
            ));
        }

        let raw = self.source[start..end].trim().to_string();
        if raw.is_empty() {
            return Err(ParseError::new(
                ParseErrorCode::E1004,
                "empty selector is not allowed",
                ast::Span::new(start as u32, end as u32),
            )
            .with_help("example: edit component[designator=R1] { value: \"10K\" }"));
        }

        let leading_ws = self.source[start..end]
            .char_indices()
            .find(|(_, ch)| !ch.is_whitespace())
            .map(|(idx, _)| idx)
            .unwrap_or(0);
        let expr = selector::parse_selector(&raw, start as u32 + leading_ws as u32)?;

        Ok(ast::Spanned::new(
            ast::Selector { raw, expr },
            ast::Span::new(start as u32, end as u32),
        ))
    }

    fn parse_selector_until_statement_end(
        &mut self,
    ) -> Result<ast::Spanned<ast::Selector>, ParseError> {
        let start = self.current().span.start as usize;
        let mut end = start;
        while !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Semicolon) || self.at(&TokenKind::Newline) {
                break;
            }
            end = self.current().span.end as usize;
            self.bump();
        }

        let raw = self.source[start..end].trim().to_string();
        if raw.is_empty() {
            return Err(ParseError::new(
                ParseErrorCode::E1004,
                "empty selector is not allowed",
                ast::Span::new(start as u32, end as u32),
            )
            .with_help("example: query component[designator^=R]"));
        }

        let leading_ws = self.source[start..end]
            .char_indices()
            .find(|(_, ch)| !ch.is_whitespace())
            .map(|(idx, _)| idx)
            .unwrap_or(0);
        let expr = selector::parse_selector(&raw, start as u32 + leading_ws as u32)?;

        Ok(ast::Spanned::new(
            ast::Selector { raw, expr },
            ast::Span::new(start as u32, end as u32),
        ))
    }

    fn parse_object(&mut self) -> Result<ast::Spanned<ast::Object>, ParseError> {
        let start = self.expect(TokenKind::LBrace, "expected '{'")?.span;
        self.skip_separators();

        let mut items = Vec::new();
        while !self.at(&TokenKind::RBrace) {
            if self.at(&TokenKind::Eof) {
                return Err(
                    ParseError::new(ParseErrorCode::E1004, "unterminated object", start)
                        .with_help("expected '}' to close object"),
                );
            }

            let item_start = self.current().span;
            let item = if self.at(&TokenKind::DotDotDot) {
                self.bump();
                let expr = self.parse_expr_with_stops(
                    &[TokenKind::Comma, TokenKind::Newline, TokenKind::RBrace],
                    true,
                )?;
                ast::ObjectItem::Spread(expr)
            } else if self.looks_like_binding() {
                ast::ObjectItem::Binding(self.parse_binding(false)?)
            } else {
                ast::ObjectItem::Field(self.parse_field()?)
            };
            let item_end = self.prev().span;
            items.push(ast::Spanned::new(item, item_start.merge(item_end)));

            if self.at(&TokenKind::RBrace) {
                break;
            }
            if !self.consume_separator() {
                return Err(self.error_here(
                    ParseErrorCode::E1004,
                    "expected ',' or newline between object items",
                    Some(
                        "add ',' between items on one line, or place each item on a new line"
                            .to_string(),
                    ),
                ));
            }
            self.skip_separators();
        }

        let end = self.expect(TokenKind::RBrace, "expected '}'")?.span;
        Ok(ast::Spanned::new(ast::Object { items }, start.merge(end)))
    }

    fn parse_field(&mut self) -> Result<ast::Field, ParseError> {
        let key = self.parse_key()?;
        self.expect(TokenKind::Colon, "expected ':' after field key")?;
        let value = self.parse_expr_with_stops(&[TokenKind::Comma, TokenKind::RBrace], true)?;
        Ok(ast::Field { key, value })
    }

    fn parse_key(&mut self) -> Result<ast::Spanned<ast::Key>, ParseError> {
        let mut segments = Vec::new();
        let first = self.expect_ident("expected field key")?;
        let mut span = first.span;
        segments.push(first);

        while self.consume_if(&TokenKind::Dot) {
            let seg = self.expect_ident("expected key segment after '.'")?;
            span = span.merge(seg.span);
            segments.push(seg);
        }

        Ok(ast::Spanned::new(ast::Key { segments }, span))
    }

    fn parse_expr_with_stops(
        &mut self,
        extra_stops: &[TokenKind],
        stop_at_newline: bool,
    ) -> Result<ast::Spanned<ast::Expr>, ParseError> {
        self.skip_newlines();
        let expr = self.parse_expr_bp(0, extra_stops, stop_at_newline)?;
        Ok(expr)
    }

    fn parse_expr_bp(
        &mut self,
        min_bp: u8,
        extra_stops: &[TokenKind],
        stop_at_newline: bool,
    ) -> Result<ast::Spanned<ast::Expr>, ParseError> {
        self.skip_newlines();
        let mut lhs = self.parse_prefix(extra_stops, stop_at_newline)?;

        loop {
            if self.at(&TokenKind::Eof) {
                break;
            }
            if stop_at_newline && self.at(&TokenKind::Newline) {
                break;
            }
            if extra_stops.iter().any(|k| self.at(k)) {
                break;
            }

            if self.at(&TokenKind::Dot) {
                let (l_bp, _r_bp) = (90u8, 91u8);
                if l_bp < min_bp {
                    break;
                }
                self.bump();
                let field = self.expect_ident("expected field name after '.'")?;
                let span = lhs.span.merge(field.span);
                lhs = ast::Spanned::new(ast::Expr::Path(Box::new(lhs), field), span);
                continue;
            }

            if self.at(&TokenKind::LBracket) {
                let (l_bp, r_bp) = (90u8, 91u8);
                if l_bp < min_bp {
                    break;
                }
                self.bump();
                let idx = self.parse_expr_bp(r_bp, &[TokenKind::RBracket], true)?;
                let rb = self.expect(TokenKind::RBracket, "expected ']' after index")?;
                let span = lhs.span.merge(rb.span);
                lhs = ast::Spanned::new(ast::Expr::Index(Box::new(lhs), Box::new(idx)), span);
                continue;
            }

            let (op, l_bp, r_bp) = match self.current_kind() {
                TokenKind::Star => (ast::BinOp::Mul, 60u8, 61u8),
                TokenKind::Slash => (ast::BinOp::Div, 60u8, 61u8),
                TokenKind::Plus => (ast::BinOp::Add, 50u8, 51u8),
                TokenKind::Minus => (ast::BinOp::Sub, 50u8, 51u8),
                _ => break,
            };

            if l_bp < min_bp {
                break;
            }

            let op_tok = self.bump().clone();
            let rhs = self.parse_expr_bp(r_bp, extra_stops, stop_at_newline)?;
            let span = lhs.span.merge(rhs.span);
            lhs = ast::Spanned::new(
                ast::Expr::BinOp(
                    Box::new(lhs),
                    ast::Spanned::new(op, op_tok.span),
                    Box::new(rhs),
                ),
                span,
            );
        }

        Ok(lhs)
    }

    fn parse_prefix(
        &mut self,
        extra_stops: &[TokenKind],
        _stop_at_newline: bool,
    ) -> Result<ast::Spanned<ast::Expr>, ParseError> {
        let tok = self.bump().clone();
        let out = match tok.kind {
            TokenKind::String(v) => ast::Spanned::new(ast::Expr::String(v), tok.span),
            TokenKind::Template(v) => {
                let tpl = self.parse_template_parts(&v, tok.span.start as usize + 1)?;
                ast::Spanned::new(ast::Expr::TemplateString(tpl), tok.span)
            }
            TokenKind::Integer(v) => ast::Spanned::new(ast::Expr::Integer(v), tok.span),
            TokenKind::Float(v) => ast::Spanned::new(ast::Expr::Float(v), tok.span),
            TokenKind::Dim(v, unit) => ast::Spanned::new(ast::Expr::Dim(v, unit), tok.span),
            TokenKind::Color(r, g, b) => ast::Spanned::new(ast::Expr::Color(r, g, b), tok.span),
            TokenKind::True => ast::Spanned::new(ast::Expr::Bool(true), tok.span),
            TokenKind::False => ast::Spanned::new(ast::Expr::Bool(false), tok.span),
            TokenKind::Null => ast::Spanned::new(ast::Expr::Null, tok.span),
            TokenKind::Ident(v) => ast::Spanned::new(ast::Expr::Ident(v), tok.span),
            TokenKind::DollarIdent(v) => ast::Spanned::new(ast::Expr::DollarIdent(v), tok.span),
            TokenKind::Minus => {
                let rhs = self.parse_expr_bp(70, extra_stops, true)?;
                let span = tok.span.merge(rhs.span);
                ast::Spanned::new(ast::Expr::UnaryNeg(Box::new(rhs)), span)
            }
            TokenKind::LParen => {
                self.skip_newlines();
                let first = self.parse_expr_bp(0, &[TokenKind::Comma, TokenKind::RParen], true)?;
                self.skip_newlines();
                if self.consume_if(&TokenKind::Comma) {
                    let second = self.parse_expr_bp(0, &[TokenKind::RParen], true)?;
                    let rb = self.expect(TokenKind::RParen, "expected ')' after tuple")?;
                    let span = tok.span.merge(rb.span);
                    ast::Spanned::new(ast::Expr::Tuple(Box::new(first), Box::new(second)), span)
                } else {
                    let rb = self.expect(TokenKind::RParen, "expected ')' after expression")?;
                    let _ = rb;
                    first
                }
            }
            TokenKind::LBracket => {
                let mut items = Vec::new();
                self.skip_separators();
                while !self.at(&TokenKind::RBracket) {
                    let expr =
                        self.parse_expr_bp(0, &[TokenKind::Comma, TokenKind::RBracket], true)?;
                    items.push(expr);
                    if self.at(&TokenKind::RBracket) {
                        break;
                    }
                    if !self.consume_if(&TokenKind::Comma) && !self.consume_if(&TokenKind::Newline)
                    {
                        return Err(self.error_here(
                            ParseErrorCode::E1004,
                            "expected ',' or newline between array elements",
                            Some("array syntax: [expr, expr, ...]".to_string()),
                        ));
                    }
                    self.skip_separators();
                }
                let rb = self.expect(TokenKind::RBracket, "expected ']' after array")?;
                let span = tok.span.merge(rb.span);
                ast::Spanned::new(ast::Expr::Array(items), span)
            }
            TokenKind::LBrace => {
                self.pos = self.pos.saturating_sub(1);
                let obj = self.parse_object()?;
                let span = obj.span;
                ast::Spanned::new(ast::Expr::Object(obj.node), span)
            }
            _ => {
                return Err(ParseError::new(
                    ParseErrorCode::E1004,
                    "expected expression",
                    tok.span,
                )
                .with_help(
                    "valid expressions: literals, refs, arrays, objects, tuples, arithmetic",
                ));
            }
        };
        Ok(out)
    }

    fn parse_template_parts(
        &self,
        raw: &str,
        content_start: usize,
    ) -> Result<ast::TemplateString, ParseError> {
        let mut parts = Vec::new();
        let mut i = 0usize;
        let mut lit_start = 0usize;

        while i < raw.len() {
            let ch = raw[i..].chars().next().expect("char boundary");
            if ch == '{' {
                let next = raw[i + 1..].chars().next();
                if next == Some('{') {
                    i += 2;
                    continue;
                }

                if lit_start < i {
                    let lit = raw[lit_start..i].replace("{{", "{").replace("}}", "}");
                    let span = ast::Span::new(
                        (content_start + lit_start) as u32,
                        (content_start + i) as u32,
                    );
                    parts.push(ast::Spanned::new(ast::TemplatePart::Literal(lit), span));
                }

                let expr_start = i + 1;
                let mut depth = 1usize;
                i += 1;
                while i < raw.len() {
                    let c = raw[i..].chars().next().expect("char boundary");
                    match c {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += c.len_utf8();
                }

                if i >= raw.len() {
                    return Err(ParseError::new(
                        ParseErrorCode::E1005,
                        "unterminated template interpolation",
                        ast::Span::new(
                            (content_start + expr_start - 1) as u32,
                            (content_start + raw.len()) as u32,
                        ),
                    )
                    .with_help("close interpolation with '}'"));
                }

                let expr_end = i;
                let expr_source = raw[expr_start..expr_end].trim();
                if expr_source.is_empty() {
                    return Err(ParseError::new(
                        ParseErrorCode::E1005,
                        "empty template interpolation",
                        ast::Span::new(
                            (content_start + expr_start - 1) as u32,
                            (content_start + expr_end + 1) as u32,
                        ),
                    )
                    .with_help("interpolation requires an expression, for example `{U1.value}`"));
                }

                let expr_tokens = lexer::lex(expr_source)?;
                let mut sub = Parser::new(expr_source, expr_tokens);
                let expr = sub.parse_expr_bp(0, &[TokenKind::Eof], true)?;
                if !sub.at(&TokenKind::Eof) {
                    return Err(ParseError::new(
                        ParseErrorCode::E1005,
                        "invalid expression in template interpolation",
                        ast::Span::new(
                            (content_start + expr_start) as u32,
                            (content_start + expr_end) as u32,
                        ),
                    ));
                }

                let mapped = remap_expr_span(expr, (content_start + expr_start) as u32);
                let span = ast::Span::new(
                    (content_start + expr_start - 1) as u32,
                    (content_start + expr_end + 1) as u32,
                );
                parts.push(ast::Spanned::new(
                    ast::TemplatePart::Interpolation(mapped),
                    span,
                ));

                i += 1;
                lit_start = i;
                continue;
            }

            if ch == '}' {
                let next = raw[i + 1..].chars().next();
                if next == Some('}') {
                    i += 2;
                    continue;
                }
                return Err(ParseError::new(
                    ParseErrorCode::E1005,
                    "unescaped '}' in template literal",
                    ast::Span::new((content_start + i) as u32, (content_start + i + 1) as u32),
                )
                .with_help("use '}}' for a literal '}'"));
            }

            i += ch.len_utf8();
        }

        if lit_start < raw.len() {
            let lit = raw[lit_start..].replace("{{", "{").replace("}}", "}");
            let span = ast::Span::new(
                (content_start + lit_start) as u32,
                (content_start + raw.len()) as u32,
            );
            parts.push(ast::Spanned::new(ast::TemplatePart::Literal(lit), span));
        }

        Ok(ast::TemplateString { parts })
    }

    fn looks_like_binding(&self) -> bool {
        if self.at(&TokenKind::Let) {
            return true;
        }
        matches!(self.current_kind(), TokenKind::Ident(_)) && self.peek_is(&TokenKind::Eq)
    }

    fn looks_like_op_rhs(&self) -> bool {
        match self.current_kind() {
            TokenKind::Ident(name) if name == "edit" || name == "remove" || name == "query" => true,
            TokenKind::Ident(_) => self.peek_is(&TokenKind::LBrace),
            _ => false,
        }
    }

    fn expect_ident(&mut self, msg: &str) -> Result<ast::Spanned<String>, ParseError> {
        match self.current_kind().clone() {
            TokenKind::Ident(v) => {
                let span = self.bump().span;
                Ok(ast::Spanned::new(v, span))
            }
            _ => Err(self.error_here(ParseErrorCode::E1004, msg, None)),
        }
    }

    fn expect_ident_or_keyword_op(
        &mut self,
        msg: &str,
    ) -> Result<ast::Spanned<String>, ParseError> {
        match self.current_kind().clone() {
            TokenKind::Ident(v) => {
                let span = self.bump().span;
                Ok(ast::Spanned::new(v, span))
            }
            TokenKind::Assert => {
                let span = self.bump().span;
                Ok(ast::Spanned::new("assert".to_string(), span))
            }
            _ => Err(self.error_here(ParseErrorCode::E1004, msg, None)),
        }
    }

    fn expect(&mut self, kind: TokenKind, msg: &str) -> Result<&Token, ParseError> {
        if self.at(&kind) {
            Ok(self.bump())
        } else {
            Err(self.error_here(
                ParseErrorCode::E1004,
                msg,
                Some(format!("found {:?}", self.current_kind())),
            ))
        }
    }

    fn consume_if(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn consume_separator(&mut self) -> bool {
        self.consume_if(&TokenKind::Comma) || self.consume_if(&TokenKind::Newline)
    }

    fn consume_semicolons(&mut self) {
        while self.consume_if(&TokenKind::Semicolon) {}
    }

    fn skip_separators(&mut self) {
        while self.consume_if(&TokenKind::Newline)
            || self.consume_if(&TokenKind::Comma)
            || self.consume_if(&TokenKind::Semicolon)
        {}
    }

    fn skip_newlines(&mut self) {
        while self.consume_if(&TokenKind::Newline) {}
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn prev(&self) -> &Token {
        if self.pos == 0 {
            &self.tokens[0]
        } else {
            &self.tokens[self.pos - 1]
        }
    }

    fn current_kind(&self) -> &TokenKind {
        &self.current().kind
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.current_kind().same_variant(kind)
    }

    fn peek_is(&self, kind: &TokenKind) -> bool {
        self.tokens
            .get(self.pos + 1)
            .map(|t| t.kind.same_variant(kind))
            .unwrap_or(false)
    }

    fn bump(&mut self) -> &Token {
        let idx = self.pos;
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        &self.tokens[idx]
    }

    fn error_here(
        &self,
        code: ParseErrorCode,
        message: impl Into<String>,
        help: Option<String>,
    ) -> ParseError {
        let mut err = ParseError::new(code, message, self.current().span);
        if let Some(help) = help {
            err = err.with_help(help);
        }
        err
    }
}

fn remap_expr_span(expr: ast::Spanned<ast::Expr>, offset: u32) -> ast::Spanned<ast::Expr> {
    let span = ast::Span::new(expr.span.start + offset, expr.span.end + offset);
    let node = match expr.node {
        ast::Expr::Path(base, seg) => {
            let base = Box::new(remap_expr_span(*base, offset));
            let seg = ast::Spanned::new(
                seg.node,
                ast::Span::new(seg.span.start + offset, seg.span.end + offset),
            );
            ast::Expr::Path(base, seg)
        }
        ast::Expr::Index(base, idx) => {
            let base = Box::new(remap_expr_span(*base, offset));
            let idx = Box::new(remap_expr_span(*idx, offset));
            ast::Expr::Index(base, idx)
        }
        ast::Expr::BinOp(left, op, right) => {
            let left = Box::new(remap_expr_span(*left, offset));
            let right = Box::new(remap_expr_span(*right, offset));
            let op = ast::Spanned::new(
                op.node,
                ast::Span::new(op.span.start + offset, op.span.end + offset),
            );
            ast::Expr::BinOp(left, op, right)
        }
        ast::Expr::UnaryNeg(rhs) => ast::Expr::UnaryNeg(Box::new(remap_expr_span(*rhs, offset))),
        ast::Expr::Tuple(a, b) => ast::Expr::Tuple(
            Box::new(remap_expr_span(*a, offset)),
            Box::new(remap_expr_span(*b, offset)),
        ),
        ast::Expr::Array(items) => ast::Expr::Array(
            items
                .into_iter()
                .map(|v| remap_expr_span(v, offset))
                .collect(),
        ),
        ast::Expr::Object(obj) => ast::Expr::Object(remap_object_span(obj, offset)),
        ast::Expr::TemplateString(ts) => ast::Expr::TemplateString(remap_template_span(ts, offset)),
        other => other,
    };

    ast::Spanned::new(node, span)
}

fn remap_object_span(object: ast::Object, offset: u32) -> ast::Object {
    let items = object
        .items
        .into_iter()
        .map(|item| {
            let span = ast::Span::new(item.span.start + offset, item.span.end + offset);
            let node = match item.node {
                ast::ObjectItem::Binding(binding) => {
                    ast::ObjectItem::Binding(remap_binding_span(binding, offset))
                }
                ast::ObjectItem::Spread(expr) => {
                    ast::ObjectItem::Spread(remap_expr_span(expr, offset))
                }
                ast::ObjectItem::Field(field) => {
                    ast::ObjectItem::Field(remap_field_span(field, offset))
                }
            };
            ast::Spanned::new(node, span)
        })
        .collect();
    ast::Object { items }
}

fn remap_template_span(template: ast::TemplateString, offset: u32) -> ast::TemplateString {
    let parts = template
        .parts
        .into_iter()
        .map(|part| {
            let span = ast::Span::new(part.span.start + offset, part.span.end + offset);
            let node = match part.node {
                ast::TemplatePart::Literal(v) => ast::TemplatePart::Literal(v),
                ast::TemplatePart::Interpolation(expr) => {
                    ast::TemplatePart::Interpolation(remap_expr_span(expr, offset))
                }
            };
            ast::Spanned::new(node, span)
        })
        .collect();

    ast::TemplateString { parts }
}

fn remap_binding_span(binding: ast::Binding, offset: u32) -> ast::Binding {
    let name = ast::Spanned::new(
        binding.name.node,
        ast::Span::new(
            binding.name.span.start + offset,
            binding.name.span.end + offset,
        ),
    );
    let value = match binding.value.node {
        ast::BindingValue::Expr(expr) => ast::Spanned::new(
            ast::BindingValue::Expr(
                remap_expr_span(ast::Spanned::new(expr, binding.value.span), offset).node,
            ),
            ast::Span::new(
                binding.value.span.start + offset,
                binding.value.span.end + offset,
            ),
        ),
        ast::BindingValue::Op(op) => ast::Spanned::new(
            ast::BindingValue::Op(remap_op_span(op, offset)),
            ast::Span::new(
                binding.value.span.start + offset,
                binding.value.span.end + offset,
            ),
        ),
    };

    ast::Binding { name, value }
}

fn remap_field_span(field: ast::Field, offset: u32) -> ast::Field {
    let key = ast::Spanned::new(
        ast::Key {
            segments: field
                .key
                .node
                .segments
                .into_iter()
                .map(|s| {
                    ast::Spanned::new(
                        s.node,
                        ast::Span::new(s.span.start + offset, s.span.end + offset),
                    )
                })
                .collect(),
        },
        ast::Span::new(field.key.span.start + offset, field.key.span.end + offset),
    );

    let value = remap_expr_span(field.value, offset);
    ast::Field { key, value }
}

fn remap_op_span(op: ast::Op, offset: u32) -> ast::Op {
    ast::Op {
        name: ast::Spanned::new(
            op.name.node,
            ast::Span::new(op.name.span.start + offset, op.name.span.end + offset),
        ),
        target: op.target.map(|t| remap_expr_span(t, offset)),
        selector: op.selector.map(|s| {
            ast::Spanned::new(
                s.node,
                ast::Span::new(s.span.start + offset, s.span.end + offset),
            )
        }),
        body: op.body.map(|b| {
            ast::Spanned::new(
                remap_object_span(b.node, offset),
                ast::Span::new(b.span.start + offset, b.span.end + offset),
            )
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "proptest")]
    use proptest::prelude::*;
    #[cfg(feature = "proptest")]
    use proptest::string::string_regex;
    #[cfg(feature = "proptest")]
    use std::panic::{AssertUnwindSafe, catch_unwind};

    fn parse_ok(src: &str) -> OpsFile {
        parse_ops(src).unwrap_or_else(|e| panic!("{}", e.render("test.ops", src)))
    }

    #[cfg(feature = "proptest")]
    fn statement_kinds(file: &OpsFile) -> Vec<&'static str> {
        file.statements
            .iter()
            .map(|s| match s.node {
                Statement::Binding(_) => "binding",
                Statement::Assert(_) => "assert",
                Statement::Op(_) => "op",
            })
            .collect()
    }

    #[cfg(feature = "proptest")]
    fn assert_spans_valid(file: &OpsFile, src_len: u32) {
        for stmt in &file.statements {
            assert!(stmt.span.start <= stmt.span.end);
            assert!(stmt.span.end <= src_len);
            match &stmt.node {
                Statement::Binding(b) => assert_binding_spans_valid(b, src_len),
                Statement::Assert(a) => assert_assert_spans_valid(a, src_len),
                Statement::Op(o) => assert_op_spans_valid(o, src_len),
            }
        }
    }

    #[cfg(feature = "proptest")]
    fn assert_span(span: Span, src_len: u32) {
        assert!(span.start <= span.end);
        assert!(span.end <= src_len);
    }

    #[cfg(feature = "proptest")]
    fn assert_binding_spans_valid(binding: &Binding, src_len: u32) {
        assert_span(binding.name.span, src_len);
        assert_span(binding.value.span, src_len);
        match &binding.value.node {
            BindingValue::Expr(expr) => {
                assert_expr_spans_valid(&Spanned::new(expr.clone(), binding.value.span), src_len)
            }
            BindingValue::Op(op) => assert_op_spans_valid(op, src_len),
        }
    }

    #[cfg(feature = "proptest")]
    fn assert_assert_spans_valid(a: &AssertStmt, src_len: u32) {
        assert_span(a.condition.span, src_len);
        match &a.condition.node {
            AssertCondition::Existence(expr) => assert_expr_spans_valid(expr, src_len),
            AssertCondition::Comparison { left, op, right } => {
                assert_expr_spans_valid(left, src_len);
                assert_span(op.span, src_len);
                assert_expr_spans_valid(right, src_len);
            }
        }
        if let Some(msg) = &a.message {
            assert_expr_spans_valid(msg, src_len);
        }
    }

    #[cfg(feature = "proptest")]
    fn assert_op_spans_valid(op: &Op, src_len: u32) {
        assert_span(op.name.span, src_len);
        if let Some(target) = &op.target {
            assert_expr_spans_valid(target, src_len);
        }
        if let Some(selector) = &op.selector {
            assert_span(selector.span, src_len);
            assert_span(selector.node.expr.span, src_len);
        }
        if let Some(body) = &op.body {
            assert_object_spans_valid(body, src_len);
        }
    }

    #[cfg(feature = "proptest")]
    fn assert_object_spans_valid(object: &Spanned<Object>, src_len: u32) {
        assert_span(object.span, src_len);
        for item in &object.node.items {
            assert_span(item.span, src_len);
            match &item.node {
                ObjectItem::Binding(b) => assert_binding_spans_valid(b, src_len),
                ObjectItem::Spread(v) => assert_expr_spans_valid(v, src_len),
                ObjectItem::Field(f) => {
                    assert_span(f.key.span, src_len);
                    for s in &f.key.node.segments {
                        assert_span(s.span, src_len);
                    }
                    assert_expr_spans_valid(&f.value, src_len);
                }
            }
        }
    }

    #[cfg(feature = "proptest")]
    fn assert_expr_spans_valid(expr: &Spanned<Expr>, src_len: u32) {
        assert_span(expr.span, src_len);
        match &expr.node {
            Expr::Path(base, seg) => {
                assert_expr_spans_valid(base, src_len);
                assert_span(seg.span, src_len);
            }
            Expr::Index(base, idx) => {
                assert_expr_spans_valid(base, src_len);
                assert_expr_spans_valid(idx, src_len);
            }
            Expr::BinOp(left, op, right) => {
                assert_expr_spans_valid(left, src_len);
                assert_span(op.span, src_len);
                assert_expr_spans_valid(right, src_len);
            }
            Expr::UnaryNeg(v) => assert_expr_spans_valid(v, src_len),
            Expr::Tuple(a, b) => {
                assert_expr_spans_valid(a, src_len);
                assert_expr_spans_valid(b, src_len);
            }
            Expr::Array(v) => {
                for item in v {
                    assert_expr_spans_valid(item, src_len);
                }
            }
            Expr::Object(o) => {
                assert_object_spans_valid(&Spanned::new(o.clone(), expr.span), src_len)
            }
            Expr::TemplateString(t) => {
                for p in &t.parts {
                    assert_span(p.span, src_len);
                    if let TemplatePart::Interpolation(e) = &p.node {
                        assert_expr_spans_valid(e, src_len);
                    }
                }
            }
            _ => {}
        }
    }

    #[test]
    fn parses_basic_program() {
        let src = r#"
spacing = 300
assert U1
r1 = add_component { designator: "R1", location: (1000, 800) }
edit component[designator=R1] { value: "20K" }
query R*
remove C*
"#;
        let file = parse_ok(src);
        assert_eq!(file.statements.len(), 6);
    }

    #[test]
    fn parses_template_interpolation() {
        let src = "assert U1, `expected {U1.value}`";
        let file = parse_ok(src);
        let Statement::Assert(assert_stmt) = &file.statements[0].node else {
            panic!("expected assert");
        };
        let msg = assert_stmt.message.as_ref().expect("message");
        let Expr::TemplateString(ts) = &msg.node else {
            panic!("expected template");
        };
        assert_eq!(ts.parts.len(), 2);
    }

    #[test]
    fn parses_selector_into_ast() {
        let src = "edit component[designator^=R] AND NOT pin:power { value: \"20K\" }";
        let file = parse_ok(src);
        let Statement::Op(op) = &file.statements[0].node else {
            panic!("expected op");
        };
        let selector = op.selector.as_ref().expect("selector");
        assert_eq!(
            selector.node.raw,
            "component[designator^=R] AND NOT pin:power"
        );
        assert!(matches!(
            selector.node.expr.node,
            SelectorExpr::And(_) | SelectorExpr::Or(_) | SelectorExpr::Chain(_)
        ));
    }

    #[test]
    fn diagnostic_has_help_text() {
        let src = "x = 20mx";
        let err = parse_ops(src).expect_err("expected parse error");
        let rendered = err.render("bad.ops", src);
        assert!(rendered.contains("unknown unit suffix"));
        assert!(rendered.contains("help: valid units"));
    }

    #[test]
    fn respects_precedence() {
        let src = "x = 1 + 2 * 3";
        let file = parse_ok(src);
        let Statement::Binding(binding) = &file.statements[0].node else {
            panic!("binding");
        };
        let BindingValue::Expr(expr) = &binding.value.node else {
            panic!("expr binding");
        };
        let Expr::BinOp(_, op, rhs) = expr else {
            panic!("binop");
        };
        assert_eq!(op.node, BinOp::Add);
        assert!(matches!(&rhs.node, Expr::BinOp(_, inner_op, _) if inner_op.node == BinOp::Mul));
    }

    #[cfg(feature = "proptest")]
    proptest! {
        #[test]
        fn prop_mul_binds_tighter_than_add(a in -1000i32..1000, b in -1000i32..1000, c in -1000i32..1000) {
            let src = format!("x = {} + {} * {}", a, b, c);
            let file = parse_ok(&src);
            let Statement::Binding(binding) = &file.statements[0].node else { panic!("binding"); };
            let BindingValue::Expr(expr) = &binding.value.node else { panic!("expr"); };
            prop_assert!(matches!(expr, Expr::BinOp(_, op, rhs) if op.node == BinOp::Add && matches!(&rhs.node, Expr::BinOp(_, mul_op, _) if mul_op.node == BinOp::Mul)));
        }

        #[test]
        fn prop_sub_is_left_associative(a in -1000i32..1000, b in -1000i32..1000, c in -1000i32..1000) {
            let src = format!("x = {} - {} - {}", a, b, c);
            let file = parse_ok(&src);
            let Statement::Binding(binding) = &file.statements[0].node else { panic!("binding"); };
            let BindingValue::Expr(expr) = &binding.value.node else { panic!("expr"); };
            prop_assert!(matches!(expr, Expr::BinOp(left, op, _) if op.node == BinOp::Sub && matches!(&left.node, Expr::BinOp(_, lop, _) if lop.node == BinOp::Sub)));
        }

        #[test]
        fn prop_noise_tokens_are_accepted(n in -1000i32..1000) {
            let src = format!("let x = {};\nlet r1 = add_component {{ designator: \"R1\", value: \"{}\", }};", n.abs(), n.abs());
            let file = parse_ok(&src);
            prop_assert_eq!(file.statements.len(), 2);
        }

        #[test]
        fn prop_parser_never_panics_on_random_text(s in string_regex(r"(?s).{0,200}").expect("regex")) {
            let result = catch_unwind(AssertUnwindSafe(|| parse_ops(&s)));
            prop_assert!(result.is_ok(), "parser panicked for input: {:?}", s);
        }

        #[test]
        fn prop_statement_kinds_stable_under_noise(prefix in "[A-Za-z_][A-Za-z0-9_]{0,6}", number in 0i32..10000) {
            let canonical = format!(
                "x = {number}\nassert {prefix}\nadd_component {{ designator: \"R1\", value: \"10K\" }}\nquery R*"
            );
            let noisy = format!(
                "let x = {number};\n// comment\nassert ({prefix});\n/* c */ add_component {{ designator: \"R1\", value: \"10K\", }};\nquery R*;\n"
            );

            let a = parse_ok(&canonical);
            let b = parse_ok(&noisy);
            prop_assert_eq!(statement_kinds(&a), statement_kinds(&b));
            prop_assert_eq!(a.statements.len(), b.statements.len());
        }

        #[test]
        fn prop_span_invariants_hold(prefix in "[A-Za-z_][A-Za-z0-9_]{0,5}", n in 0i32..1000) {
            let src = format!(
                "let p = {{ electrical: passive, length: 25 }}\n\
                 assert {prefix} == {prefix}\n\
                 r1 = add_component {{ designator: \"R1\", location: ({n}, {n}), pins: [{{ designator: \"1\", ...p, offset: (-50, 0) }}] }}\n\
                 edit component[designator=R1] {{ value: \"20K\" }}\n\
                 query R*\n"
            );
            let file = parse_ok(&src);
            assert_spans_valid(&file, src.len() as u32);
        }
    }
}

use crate::ast::*;
use crate::diagnostic::{BinOp, ParseError, Span, Unit};
use crate::lexer::{TemplatePart, Token, TokenKind};
use crate::parser::parse_spec;
use crate::trivia::{ItemTrivia, TriviaLine, scan_trivia_lines};

// ── Public API ────────────────────────────────────────────────────────────────

pub struct FormatConfig {
    pub indent: usize,
    pub max_inline_items: usize,
    pub max_line_width: usize,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            indent: 4,
            max_inline_items: 4,
            max_line_width: 100,
        }
    }
}

pub struct FormatResult {
    pub output: String,
    pub changed: bool,
}

/// Format a spec file source string, preserving top-level comments.
pub fn format_spec(source: &str, config: &FormatConfig) -> Result<FormatResult, ParseError> {
    let ast = parse_spec(source)?;
    let trivia = extract_top_level_trivia(source, &ast);
    let output = pretty_print(source, &ast, &trivia, config);
    let changed = output != source;
    Ok(FormatResult { output, changed })
}

// ── Comment / trivia extraction ───────────────────────────────────────────────

/// Extract trivia for every top-level item plus a trailing trivia entry for
/// text after the last item.
pub fn extract_top_level_trivia(source: &str, ast: &SpecFile) -> Vec<ItemTrivia> {
    let n = ast.items.len();
    if n == 0 {
        return Vec::new();
    }

    // Collect item spans.
    let spans: Vec<Span> = ast.items.iter().map(|i| i.span).collect();

    // Build one trivia entry per item.
    let mut result: Vec<ItemTrivia> = (0..n).map(|_| ItemTrivia::default()).collect();

    for idx in 0..n {
        let item_start = spans[idx].start as usize;
        let item_end = spans[idx].end as usize;

        // Gap before this item: from end of previous item (or 0) to item_start.
        let gap_start = if idx == 0 {
            0
        } else {
            spans[idx - 1].end as usize
        };
        let gap = &source[gap_start..item_start];
        result[idx].leading = scan_trivia_lines(gap);

        // Trailing comment: text on the same line after item_end.
        let rest = &source[item_end..];
        let line_end = rest.find('\n').unwrap_or(rest.len());
        let line_tail = rest[..line_end].trim();
        if line_tail.starts_with("//") {
            result[idx].trailing = Some(line_tail.to_string());
        } else if line_tail.starts_with("/*") {
            if let Some(end_pos) = line_tail.find("*/") {
                result[idx].trailing = Some(line_tail[..end_pos + 2].to_string());
            }
        }
    }

    result
}

// ── Pretty-printer ────────────────────────────────────────────────────────────

struct Printer<'a> {
    config: &'a FormatConfig,
    out: String,
    indent_level: usize,
}

impl<'a> Printer<'a> {
    fn new(config: &'a FormatConfig) -> Self {
        Self {
            config,
            out: String::new(),
            indent_level: 0,
        }
    }

    fn indent_str(&self) -> String {
        " ".repeat(self.indent_level * self.config.indent)
    }

    fn push(&mut self, s: &str) {
        self.out.push_str(s);
    }

    fn push_indent(&mut self) {
        let s = self.indent_str();
        self.out.push_str(&s);
    }

    fn push_newline(&mut self) {
        self.out.push('\n');
    }

    fn indent(&mut self) {
        self.indent_level += 1;
    }

    fn dedent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }

    // ── Trivia ────────────────────────────────────────────────────────────────

    fn emit_leading_trivia(&mut self, trivia: &ItemTrivia, is_first: bool) {
        let lines = &trivia.leading;

        // Find the range of meaningful content: first comment to last comment
        // (strip leading and trailing blank lines).
        let first_comment = lines.iter().position(|l| !matches!(l, TriviaLine::Blank));
        let last_comment = lines.iter().rposition(|l| !matches!(l, TriviaLine::Blank));

        let (start, end) = match (first_comment, last_comment) {
            (Some(s), Some(e)) => (s, e + 1),
            _ => {
                // No comments at all.
                if !is_first {
                    // Just a blank line separator between items.
                    self.push_newline();
                }
                return;
            }
        };

        let comment_lines = &lines[start..end];

        if is_first {
            // Emit comment block (keeping inter-comment blanks), no leading blank.
            for line in comment_lines {
                match line {
                    TriviaLine::Blank => self.push_newline(),
                    TriviaLine::LineComment(s) => {
                        self.push_indent();
                        self.push(s);
                        self.push_newline();
                    }
                    TriviaLine::BlockComment(s) => {
                        self.push_indent();
                        self.push(s);
                        self.push_newline();
                    }
                }
            }
        } else {
            // Emit one blank separator before the comment block, then the comments.
            self.push_newline();
            for line in comment_lines {
                match line {
                    TriviaLine::Blank => self.push_newline(),
                    TriviaLine::LineComment(s) => {
                        self.push_indent();
                        self.push(s);
                        self.push_newline();
                    }
                    TriviaLine::BlockComment(s) => {
                        self.push_indent();
                        self.push(s);
                        self.push_newline();
                    }
                }
            }
        }
    }

    fn emit_trailing_trivia(&mut self, trivia: &ItemTrivia) {
        if let Some(comment) = &trivia.trailing {
            self.push(" ");
            self.push(comment);
        }
    }

    // ── Annotation ────────────────────────────────────────────────────────────

    /// Emit a `#[annotation(id = "...")]` line at the current indent level,
    /// if an annotation is present on the block declaration.
    ///
    /// Only fields with non-default values are emitted:
    /// - `id` is always emitted (always present after compilation).
    /// - `stable` is emitted only when `true`.
    /// - `group` is emitted only when `Some`.
    fn fmt_annotation(&mut self, annotation: &Option<Spanned<BlockAnnotation>>) {
        let ann = match annotation {
            Some(a) => &a.node,
            None => return,
        };

        let id_part = match &ann.id {
            Some(id) => format!("id = \"{}\"", id.node),
            None => return, // no ID — nothing to emit
        };

        let mut parts = vec![id_part];
        if let Some(stable) = &ann.stable {
            if stable.node {
                parts.push("stable = true".to_owned());
            }
        }
        if let Some(group) = &ann.group {
            parts.push(format!("group = \"{}\"", group.node));
        }
        if let Some(source_id) = &ann.source_id {
            parts.push(format!("source_id = \"{}\"", source_id.node));
        }

        self.push_indent();
        self.push("#[annotation(");
        self.push(&parts.join(", "));
        self.push(")]");
        self.push_newline();
    }

    // ── Entity names ──────────────────────────────────────────────────────────

    fn fmt_entity_name(&mut self, name: &EntityName) {
        match name {
            EntityName::Ident(s) => self.push(s),
            EntityName::String(s) => {
                self.push("\"");
                self.push(&escape_string(s));
                self.push("\"");
            }
            EntityName::Integer(n) => self.push(&n.to_string()),
        }
    }

    fn fmt_footprint_ref(&mut self, fref: &FootprintRef) {
        match fref {
            FootprintRef::Name(name) => self.fmt_entity_name(name),
            FootprintRef::DollarPath(dp) => self.fmt_dollar_path(dp),
        }
    }

    fn fmt_dollar_path(&mut self, dp: &DollarPath) {
        self.push("$");
        self.push(&dp.root.node);
        for step in &dp.steps {
            match &step.node {
                PathStep::Field(f) => {
                    self.push(".");
                    self.push(f);
                }
                PathStep::Index(expr) => {
                    self.push("[");
                    self.fmt_expr(expr, false);
                    self.push("]");
                }
            }
        }
    }

    // ── Expressions ───────────────────────────────────────────────────────────

    fn fmt_expr(&mut self, expr: &Expr, parenthesize_binop: bool) {
        match expr {
            Expr::String(s) => {
                self.push("\"");
                self.push(&escape_string(s));
                self.push("\"");
            }
            Expr::Template(parts) => {
                self.push("`");
                for part in parts {
                    match part {
                        TemplatePart::Literal(s) => {
                            // Escape backtick and literal braces in template literals.
                            let escaped = s
                                .replace('\\', "\\\\")
                                .replace('`', "\\`")
                                .replace("{{", "\\{")
                                .replace("}}", "\\}");
                            self.push(&escaped);
                        }
                        TemplatePart::Expr(tokens) => {
                            self.push("{");
                            self.fmt_template_tokens(tokens);
                            self.push("}");
                        }
                    }
                }
                self.push("`");
            }
            Expr::Integer(n) => self.push(&n.to_string()),
            Expr::Float(f) => {
                let s = format_float(*f);
                self.push(&s);
            }
            Expr::Dim(val, unit) => {
                let s = format_dim(*val, *unit);
                self.push(&s);
            }
            Expr::Color(r, g, b) => {
                self.push(&format!("#{:02X}{:02X}{:02X}", r, g, b));
            }
            Expr::Bool(b) => self.push(if *b { "true" } else { "false" }),
            Expr::Null => self.push("null"),
            Expr::Ident(s) => self.push(s),
            Expr::DollarIdent(s) => {
                self.push("$");
                self.push(s);
            }
            Expr::Path(base, field) => {
                self.fmt_expr(&base.node, true);
                self.push(".");
                self.push(&field.node);
            }
            Expr::Index(base, idx) => {
                self.fmt_expr(&base.node, true);
                self.push("[");
                self.fmt_expr(&idx.node, false);
                self.push("]");
            }
            Expr::BinOp(left, op, right) => {
                let op_str = match op.node {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                };
                let needs_parens = parenthesize_binop;
                if needs_parens {
                    self.push("(");
                }
                // Determine if sub-expressions need parens for precedence.
                let is_mul_div = matches!(op.node, BinOp::Mul | BinOp::Div);
                let left_needs_parens = match &left.node {
                    Expr::BinOp(_, sub_op, _) => {
                        is_mul_div && matches!(sub_op.node, BinOp::Add | BinOp::Sub)
                    }
                    _ => false,
                };
                let right_needs_parens = match &right.node {
                    Expr::BinOp(_, sub_op, _) => {
                        is_mul_div && matches!(sub_op.node, BinOp::Add | BinOp::Sub)
                    }
                    _ => false,
                };
                self.fmt_expr(&left.node, left_needs_parens);
                self.push(" ");
                self.push(op_str);
                self.push(" ");
                self.fmt_expr(&right.node, right_needs_parens);
                if needs_parens {
                    self.push(")");
                }
            }
            Expr::UnaryNeg(inner) => {
                self.push("-");
                let needs_parens = matches!(&inner.node, Expr::BinOp(..));
                self.fmt_expr(&inner.node, needs_parens);
            }
            Expr::Tuple(a, b) => {
                self.push("(");
                self.fmt_expr(&a.node, false);
                self.push(", ");
                self.fmt_expr(&b.node, false);
                self.push(")");
            }
            Expr::Array(items) => {
                self.push("[");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.fmt_expr(&item.node, false);
                }
                self.push("]");
            }
            Expr::Object(obj) => {
                self.fmt_object(obj);
            }
            Expr::Call { name, args } => {
                self.push(name);
                self.push("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    if let Some(ref arg_name) = arg.name {
                        self.push(&arg_name.node);
                        self.push(": ");
                    }
                    self.fmt_expr(&arg.value.node, false);
                }
                self.push(")");
            }
        }
    }

    fn fmt_template_tokens(&mut self, tokens: &[Token]) {
        for (i, tok) in tokens.iter().enumerate() {
            if i > 0 {
                // Add space between most tokens, but not before/after . [ ]
                let prev = &tokens[i - 1].kind;
                let cur = &tok.kind;
                let no_space = matches!(prev, TokenKind::Dot | TokenKind::LBracket)
                    || matches!(
                        cur,
                        TokenKind::Dot | TokenKind::RBracket | TokenKind::LBracket
                    );
                if !no_space {
                    self.push(" ");
                }
            }
            match &tok.kind {
                TokenKind::Ident(s) => self.push(s),
                TokenKind::DollarIdent(s) => {
                    self.push("$");
                    self.push(s);
                }
                TokenKind::String(s) => {
                    self.push("\"");
                    self.push(&escape_string(s));
                    self.push("\"");
                }
                TokenKind::Integer(n) => self.push(&n.to_string()),
                TokenKind::Float(f) => self.push(&format_float(*f)),
                TokenKind::Dim(v, u) => self.push(&format_dim(*v, *u)),
                TokenKind::Color(r, g, b) => self.push(&format!("#{:02X}{:02X}{:02X}", r, g, b)),
                TokenKind::Dot => self.push("."),
                TokenKind::LBracket => self.push("["),
                TokenKind::RBracket => self.push("]"),
                TokenKind::LParen => self.push("("),
                TokenKind::RParen => self.push(")"),
                TokenKind::Plus => self.push("+"),
                TokenKind::Minus => self.push("-"),
                TokenKind::Star => self.push("*"),
                TokenKind::Slash => self.push("/"),
                TokenKind::True => self.push("true"),
                TokenKind::False => self.push("false"),
                TokenKind::Null => self.push("null"),
                _ => {} // skip newlines etc.
            }
        }
    }

    // ── Objects ───────────────────────────────────────────────────────────────

    fn fmt_object(&mut self, obj: &Object) {
        if obj.items.is_empty() {
            self.push("{}");
            return;
        }
        // Try inline first.
        if self.can_inline_object(obj) {
            let inline = self.render_object_inline(obj);
            if inline.len() + self.current_line_len() <= self.config.max_line_width {
                self.push(&inline);
                return;
            }
        }
        // Multi-line.
        self.push("{");
        self.push_newline();
        self.indent();
        for item in &obj.items {
            self.push_indent();
            self.fmt_object_item(&item.node);
            self.push_newline();
        }
        self.dedent();
        self.push_indent();
        self.push("}");
    }

    fn can_inline_object(&self, obj: &Object) -> bool {
        if obj.items.len() > self.config.max_inline_items {
            return false;
        }
        obj.items
            .iter()
            .all(|item| is_simple_object_item(&item.node))
    }

    fn render_object_inline(&self, obj: &Object) -> String {
        let mut sub = Printer::new(self.config);
        sub.indent_level = self.indent_level;
        sub.push("{ ");
        for (i, item) in obj.items.iter().enumerate() {
            if i > 0 {
                sub.push(", ");
            }
            sub.fmt_object_item(&item.node);
        }
        sub.push(" }");
        sub.out
    }

    fn current_line_len(&self) -> usize {
        self.out
            .rfind('\n')
            .map(|pos| self.out.len() - pos - 1)
            .unwrap_or(self.out.len())
    }

    fn fmt_object_item(&mut self, item: &ObjectItem) {
        match item {
            ObjectItem::Property(p) => self.fmt_property(p),
            ObjectItem::Spread(expr) => {
                self.push("...");
                self.fmt_expr(&expr.node, false);
            }
            ObjectItem::LetBinding(lb) => self.fmt_let_binding(lb),
        }
    }

    fn fmt_property(&mut self, p: &Property) {
        self.push(&p.key.node);
        self.push(": ");
        self.fmt_expr(&p.value.node, false);
    }

    // ── Let binding ───────────────────────────────────────────────────────────

    fn fmt_let_binding(&mut self, lb: &LetBinding) {
        self.push("let ");
        self.push(&lb.name.node);
        self.push(" = ");
        self.fmt_expr(&lb.value.node, false);
    }

    // ── Top-level items ───────────────────────────────────────────────────────

    fn fmt_spec_item(&mut self, item: &SpecItem) {
        match item {
            SpecItem::Import(decl) => self.fmt_import(decl),
            SpecItem::LetBinding(lb) => self.fmt_let_binding(lb),
            SpecItem::Component(decl) => self.fmt_component(decl),
            SpecItem::Footprint(decl) => self.fmt_footprint(decl),
            SpecItem::Project(decl) => self.fmt_project(decl),
            SpecItem::SwapGroup(decl) => self.fmt_swap_group(decl),
            SpecItem::Sheet(decl) => self.fmt_sheet(decl),
            SpecItem::Net(decl) => self.fmt_net(decl),
            SpecItem::Power(decl) => self.fmt_power(decl),
            SpecItem::SchDocObject(decl) => self.fmt_schdoc_object(decl),
            SpecItem::Board(decl) => self.fmt_board(decl),
            SpecItem::Placement(decl) => self.fmt_placement(decl),
            SpecItem::Routing(decl) => self.fmt_routing(decl),
            SpecItem::PcbDocPrimitive(decl) => self.fmt_pcbdoc_primitive(decl),
            SpecItem::Polygon(decl) => self.fmt_polygon(decl),
            SpecItem::Rule(decl) => self.fmt_rule(decl),
            SpecItem::Class(decl) => self.fmt_class(decl),
            SpecItem::DifferentialPair(decl) => self.fmt_differential_pair(decl),
        }
    }

    fn fmt_import(&mut self, decl: &ImportDecl) {
        self.push("import \"");
        self.push(&escape_string(&decl.path.node));
        self.push("\"");
        if let Some(alias) = &decl.alias {
            self.push(" as ");
            self.push(&alias.node);
        }
    }

    // ── Component ─────────────────────────────────────────────────────────────

    fn fmt_component(&mut self, decl: &ComponentDecl) {
        self.fmt_annotation(&decl.annotation);
        if let Some(binding) = &decl.binding {
            self.push(&binding.node);
            self.push(" = ");
        }
        self.push("component ");
        self.fmt_entity_name(&decl.name.node);
        self.push(" {");
        self.push_newline();
        self.indent();
        for item in &decl.body {
            self.push_indent();
            self.fmt_component_item(&item.node);
            self.push_newline();
        }
        self.dedent();
        self.push("}");
    }

    fn fmt_component_item(&mut self, item: &ComponentItem) {
        match item {
            ComponentItem::Property(p) => self.fmt_property(p),
            ComponentItem::LetBinding(lb) => self.fmt_let_binding(lb),
            ComponentItem::Part(part) => self.fmt_part_block(part),
            ComponentItem::PinConnection(pc) => self.fmt_pin_connection(pc),
            ComponentItem::Pin(pin) => self.fmt_pin_decl(pin),
            ComponentItem::Parameter(param) => self.fmt_parameter_decl(param),
            ComponentItem::Alias(alias) => self.fmt_alias_decl(alias),
            ComponentItem::FootprintMap(fm) => self.fmt_footprint_map(fm),
            ComponentItem::Graphic(g) => self.fmt_graphic_decl(g),
            ComponentItem::SwapGroup(sg) => self.fmt_swap_group(sg),
        }
    }

    fn fmt_part_block(&mut self, part: &PartBlock) {
        if let Some(binding) = &part.binding {
            self.push(&binding.node);
            self.push(" = ");
        }
        self.push("part ");
        self.push(&part.number.node.to_string());
        self.push(" {");
        self.push_newline();
        self.indent();
        for item in &part.body {
            self.push_indent();
            self.fmt_part_item(&item.node);
            self.push_newline();
        }
        self.dedent();
        self.push_indent();
        self.push("}");
    }

    fn fmt_part_item(&mut self, item: &PartItem) {
        match item {
            PartItem::LetBinding(lb) => self.fmt_let_binding(lb),
            PartItem::Pin(pin) => self.fmt_pin_decl(pin),
            PartItem::Graphic(g) => self.fmt_graphic_decl(g),
            PartItem::Property(p) => self.fmt_property(p),
        }
    }

    fn fmt_pin_connection(&mut self, pc: &PinConnectionDecl) {
        self.push("pin ");
        self.push(&pc.pin_name.node);
        self.push(" -> ");
        match &pc.target {
            PinConnectionTarget::NetRef(net) => {
                self.push("#");
                self.push(&net.node);
            }
            PinConnectionTarget::NoConnect => {
                self.push("nc");
            }
        }
    }

    fn fmt_pin_decl(&mut self, pin: &PinDecl) {
        if let Some(binding) = &pin.binding {
            self.push(&binding.node);
            self.push(" = ");
        }
        self.push("pin ");
        self.fmt_entity_name(&pin.name.node);
        self.push(" ");
        self.fmt_object(&pin.body.node);
    }

    fn fmt_parameter_decl(&mut self, param: &ParameterDecl) {
        if let Some(binding) = &param.binding {
            self.push(&binding.node);
            self.push(" = ");
        }
        self.push("parameter ");
        self.fmt_entity_name(&param.name.node);
        self.push(" ");
        self.fmt_object(&param.body.node);
    }

    fn fmt_alias_decl(&mut self, alias: &AliasDecl) {
        self.push("alias ");
        self.fmt_entity_name(&alias.name.node);
    }

    fn fmt_footprint_map(&mut self, fm: &FootprintMapDecl) {
        self.push("footprint ");
        self.fmt_footprint_ref(&fm.name.node);
        match &fm.maps {
            None => {
                // Implicit 1:1 — no body
            }
            Some(pairs) => {
                self.push(" {");
                for (i, pair) in pairs.iter().enumerate() {
                    if i > 0 {
                        self.push(",");
                    }
                    self.push(" ");
                    self.fmt_dollar_path(&pair.node.pin.node);
                    self.push(": ");
                    self.fmt_dollar_path(&pair.node.pad.node);
                }
                self.push(" }");
            }
        }
    }

    fn fmt_graphic_decl(&mut self, g: &GraphicDecl) {
        if let Some(binding) = &g.binding {
            self.push(&binding.node);
            self.push(" = ");
        }
        self.push(&g.graphic_type.node);
        self.push(" ");
        self.fmt_object(&g.body.node);
    }

    fn fmt_swap_group(&mut self, sg: &SwapGroupDecl) {
        if let Some(binding) = &sg.binding {
            self.push(&binding.node);
            self.push(" = ");
        }
        self.push("swap_group ");
        self.fmt_entity_name(&sg.name.node);
        self.push(" ");
        self.fmt_object(&sg.body.node);
    }

    // ── Footprint ─────────────────────────────────────────────────────────────

    fn fmt_footprint(&mut self, decl: &FootprintDecl) {
        self.fmt_annotation(&decl.annotation);
        if let Some(binding) = &decl.binding {
            self.push(&binding.node);
            self.push(" = ");
        }
        self.push("footprint ");
        self.fmt_entity_name(&decl.name.node);
        self.push(" {");
        self.push_newline();
        self.indent();
        for item in &decl.body {
            self.push_indent();
            self.fmt_footprint_item(&item.node);
            self.push_newline();
        }
        self.dedent();
        self.push("}");
    }

    fn fmt_footprint_item(&mut self, item: &FootprintItem) {
        match item {
            FootprintItem::Property(p) => self.fmt_property(p),
            FootprintItem::LetBinding(lb) => self.fmt_let_binding(lb),
            FootprintItem::Pad(pad) => self.fmt_pad_decl(pad),
            FootprintItem::Row(row) => self.fmt_row_decl("row", row),
            FootprintItem::Column(col) => self.fmt_row_decl("column", col),
            FootprintItem::Grid(grid) => self.fmt_grid_decl(grid),
            FootprintItem::Graphic(g) => self.fmt_graphic_decl(g),
        }
    }

    fn fmt_pad_decl(&mut self, pad: &PadDecl) {
        if let Some(binding) = &pad.binding {
            self.push(&binding.node);
            self.push(" = ");
        }
        self.push("pad ");
        self.fmt_entity_name(&pad.name.node);
        self.push(" ");
        self.fmt_object(&pad.body.node);
    }

    fn fmt_row_decl(&mut self, keyword: &str, row: &RowDecl) {
        self.push(keyword);
        self.push(" ");
        self.fmt_object(&row.body.node);
    }

    fn fmt_grid_decl(&mut self, grid: &GridDecl) {
        self.push("grid ");
        self.fmt_object(&grid.body.node);
    }

    // ── Project ───────────────────────────────────────────────────────────────

    fn fmt_project(&mut self, decl: &ProjectDecl) {
        if let Some(binding) = &decl.binding {
            self.push(&binding.node);
            self.push(" = ");
        }
        self.push("project ");
        self.fmt_entity_name(&decl.name.node);
        self.push(" {");
        self.push_newline();
        self.indent();
        for item in &decl.body {
            self.push_indent();
            self.fmt_project_item(&item.node);
            self.push_newline();
        }
        self.dedent();
        self.push("}");
    }

    fn fmt_project_item(&mut self, item: &ProjectItem) {
        match item {
            ProjectItem::Property(p) => self.fmt_property(p),
            ProjectItem::LetBinding(lb) => self.fmt_let_binding(lb),
            ProjectItem::Document(doc) => self.fmt_document_block(doc),
            ProjectItem::Annotation(ann) => self.fmt_annotation_block(ann),
            ProjectItem::ErcMatrix(entries) => self.fmt_erc_matrix(entries),
            ProjectItem::ErcLevels(entries) => self.fmt_erc_levels(entries),
            ProjectItem::OutputGroup(og) => self.fmt_output_group(og),
            ProjectItem::Comparison(rules) => self.fmt_comparison(rules),
            ProjectItem::ClassGen(props) => self.fmt_simple_block("class_gen", props),
            ProjectItem::LibraryUpdate(props) => self.fmt_simple_block("library_update", props),
            ProjectItem::Variant(v) => self.fmt_variant_block(v),
        }
    }

    fn fmt_document_block(&mut self, doc: &DocumentBlockDecl) {
        self.push("document ");
        self.fmt_entity_name(&doc.path.node);
        self.push(" {");
        self.push_newline();
        self.indent();
        for prop in &doc.body {
            self.push_indent();
            self.fmt_property(&prop.node);
            self.push_newline();
        }
        self.dedent();
        self.push_indent();
        self.push("}");
    }

    fn fmt_annotation_block(&mut self, ann: &AnnotationBlockDecl) {
        self.push("annotation {");
        self.push_newline();
        self.indent();
        for prop in &ann.properties {
            self.push_indent();
            self.fmt_property(&prop.node);
            self.push_newline();
        }
        for mp in &ann.match_parameters {
            self.push_indent();
            self.push("match_parameter ");
            self.push(&mp.node.index.node.to_string());
            self.push(" ");
            self.fmt_object(&mp.node.body.node);
            self.push_newline();
        }
        self.dedent();
        self.push_indent();
        self.push("}");
    }

    fn fmt_erc_matrix(&mut self, entries: &[Spanned<ErcMatrixEntryDecl>]) {
        self.push("erc_matrix {");
        self.push_newline();
        self.indent();
        for entry in entries {
            self.push_indent();
            self.push("(");
            self.push(&entry.node.row.node);
            self.push(", ");
            self.push(&entry.node.col.node);
            self.push("): ");
            self.push(&entry.node.level.node);
            self.push_newline();
        }
        self.dedent();
        self.push_indent();
        self.push("}");
    }

    fn fmt_erc_levels(&mut self, entries: &[Spanned<ErcLevelEntryDecl>]) {
        self.push("erc_levels {");
        self.push_newline();
        self.indent();
        for entry in entries {
            self.push_indent();
            self.push(&entry.node.name.node);
            self.push(": ");
            self.fmt_expr(&entry.node.level.node, false);
            self.push_newline();
        }
        self.dedent();
        self.push_indent();
        self.push("}");
    }

    fn fmt_output_group(&mut self, og: &OutputGroupBlockDecl) {
        self.push("output_group ");
        self.fmt_entity_name(&og.name.node);
        self.push(" {");
        self.push_newline();
        self.indent();
        for prop in &og.properties {
            self.push_indent();
            self.fmt_property(&prop.node);
            self.push_newline();
        }
        for output in &og.outputs {
            self.push_indent();
            self.fmt_output_block(&output.node);
            self.push_newline();
        }
        self.dedent();
        self.push_indent();
        self.push("}");
    }

    fn fmt_output_block(&mut self, ob: &OutputBlockDecl) {
        self.push("output ");
        self.fmt_entity_name(&ob.name.node);
        self.push(" {");
        self.push_newline();
        self.indent();
        for prop in &ob.body {
            self.push_indent();
            self.fmt_property(&prop.node);
            self.push_newline();
        }
        self.dedent();
        self.push_indent();
        self.push("}");
    }

    fn fmt_comparison(&mut self, rules: &[Spanned<ComparisonRuleDecl>]) {
        self.push("comparison {");
        self.push_newline();
        self.indent();
        for rule in rules {
            self.push_indent();
            self.push("rule ");
            self.fmt_entity_name(&rule.node.kind.node);
            self.push(" ");
            self.fmt_object(&rule.node.body.node);
            self.push_newline();
        }
        self.dedent();
        self.push_indent();
        self.push("}");
    }

    fn fmt_simple_block(&mut self, keyword: &str, props: &[Spanned<Property>]) {
        self.push(keyword);
        self.push(" {");
        self.push_newline();
        self.indent();
        for prop in props {
            self.push_indent();
            self.fmt_property(&prop.node);
            self.push_newline();
        }
        self.dedent();
        self.push_indent();
        self.push("}");
    }

    fn fmt_variant_block(&mut self, v: &VariantBlockDecl) {
        self.push("variant ");
        self.fmt_entity_name(&v.name.node);
        self.push(" {");
        self.push_newline();
        self.indent();
        for prop in &v.properties {
            self.push_indent();
            self.fmt_property(&prop.node);
            self.push_newline();
        }
        for var in &v.variations {
            self.push_indent();
            self.push("variation ");
            self.fmt_entity_name(&var.node.designator.node);
            self.push(" ");
            self.fmt_object(&var.node.body.node);
            self.push_newline();
        }
        for pv in &v.param_variations {
            self.push_indent();
            self.push("param_variation ");
            self.fmt_entity_name(&pv.node.designator.node);
            self.push(" ");
            self.fmt_object(&pv.node.body.node);
            self.push_newline();
        }
        self.dedent();
        self.push_indent();
        self.push("}");
    }

    // ── Sheet ─────────────────────────────────────────────────────────────────

    fn fmt_sheet(&mut self, decl: &SheetDecl) {
        self.push("sheet {");
        self.push_newline();
        self.indent();
        for item in &decl.body {
            self.push_indent();
            self.fmt_sheet_item(&item.node);
            self.push_newline();
        }
        self.dedent();
        self.push("}");
    }

    fn fmt_sheet_item(&mut self, item: &SheetItem) {
        match item {
            SheetItem::Property(p) => self.fmt_property(p),
            SheetItem::LetBinding(lb) => self.fmt_let_binding(lb),
            SheetItem::FontBlock(fb) => self.fmt_font_block(fb),
            SheetItem::Constraint(c) => self.fmt_constraint_decl(c),
        }
    }

    fn fmt_constraint_decl(&mut self, c: &crate::ast::ConstraintDecl) {
        self.fmt_annotation(&c.annotation);
        self.push_indent();
        self.push("constraint ");
        let kind_str = match c.kind.node {
            crate::ast::ConstraintKind::EdgePlacement => "edge_placement",
            crate::ast::ConstraintKind::Directional => "directional",
            crate::ast::ConstraintKind::Near => "near",
            crate::ast::ConstraintKind::Region => "region",
            crate::ast::ConstraintKind::FixedPosition => "fixed_position",
        };
        self.push(kind_str);
        self.push(" ");
        self.fmt_object(&c.body.node);
    }

    fn fmt_font_block(&mut self, fb: &FontBlockDecl) {
        self.push("fonts {");
        self.push_newline();
        self.indent();
        for font in &fb.fonts {
            self.push_indent();
            self.push("font ");
            self.push(&font.node.id.node.to_string());
            self.push(" ");
            self.fmt_object(&font.node.body.node);
            self.push_newline();
        }
        self.dedent();
        self.push_indent();
        self.push("}");
    }

    // ── Net / Power ───────────────────────────────────────────────────────────

    fn fmt_net(&mut self, decl: &NetDecl) {
        self.fmt_annotation(&decl.annotation);
        self.push("net ");
        self.fmt_entity_name(&decl.name.node);
        self.push(" ");
        self.fmt_object(&decl.body.node);
    }

    fn fmt_power(&mut self, decl: &PowerDecl) {
        self.fmt_annotation(&decl.annotation);
        self.push("power ");
        self.fmt_entity_name(&decl.name.node);
        self.push(" ");
        self.fmt_object(&decl.body.node);
    }

    // ── SchDoc objects ────────────────────────────────────────────────────────

    fn fmt_schdoc_object(&mut self, decl: &SchDocObjectDecl) {
        self.push(&decl.object_type.node);
        if let Some(name) = &decl.name {
            self.push(" ");
            self.fmt_entity_name(&name.node);
        }
        self.push(" {");
        self.push_newline();
        self.indent();
        for item in &decl.body {
            self.push_indent();
            self.fmt_schdoc_object_item(&item.node);
            self.push_newline();
        }
        self.dedent();
        self.push("}");
    }

    fn fmt_schdoc_object_item(&mut self, item: &SchDocObjectItem) {
        match item {
            SchDocObjectItem::Property(p) => self.fmt_property(p),
            SchDocObjectItem::LetBinding(lb) => self.fmt_let_binding(lb),
            SchDocObjectItem::Entry(e) => self.fmt_entry_decl(e),
            SchDocObjectItem::Parameter(p) => self.fmt_parameter_decl(p),
            SchDocObjectItem::Graphic(g) => self.fmt_graphic_decl(g),
        }
    }

    fn fmt_entry_decl(&mut self, e: &EntryDecl) {
        self.push("entry ");
        self.fmt_entity_name(&e.name.node);
        self.push(" ");
        self.fmt_object(&e.body.node);
    }

    // ── Board ─────────────────────────────────────────────────────────────────

    fn fmt_board(&mut self, decl: &BoardDecl) {
        self.fmt_annotation(&decl.annotation);
        self.push("board ");
        self.fmt_entity_name(&decl.name.node);
        self.push(" {");
        self.push_newline();
        self.indent();
        for item in &decl.body {
            self.push_indent();
            self.fmt_board_item(&item.node);
            self.push_newline();
        }
        self.dedent();
        self.push("}");
    }

    fn fmt_board_item(&mut self, item: &BoardItem) {
        match item {
            BoardItem::Property(p) => self.fmt_property(p),
            BoardItem::LetBinding(lb) => self.fmt_let_binding(lb),
        }
    }

    // ── Placement ─────────────────────────────────────────────────────────────

    fn fmt_placement(&mut self, decl: &PlacementDecl) {
        self.fmt_annotation(&decl.annotation);
        self.push("placement {");
        self.push_newline();
        self.indent();
        for item in &decl.body {
            self.push_indent();
            self.fmt_placement_item(&item.node);
            self.push_newline();
        }
        self.dedent();
        self.push("}");
    }

    // ── Routing ───────────────────────────────────────────────────────────────

    fn fmt_routing(&mut self, decl: &RoutingDecl) {
        self.push("routing ");
        self.fmt_object(&decl.body.node);
    }

    fn fmt_placement_item(&mut self, item: &PlacementItem) {
        match item {
            PlacementItem::Property(p) => self.fmt_property(p),
            PlacementItem::LetBinding(lb) => self.fmt_let_binding(lb),
            PlacementItem::Place(place) => self.fmt_place_decl(place),
            PlacementItem::Constraint(c) => self.fmt_placement_constraint(c),
            PlacementItem::Optimize(obj) => {
                self.push("optimize ");
                self.fmt_object(&obj.node);
            }
            PlacementItem::Clearance(obj) => {
                self.push("clearance ");
                self.fmt_object(&obj.node);
            }
            PlacementItem::AutoplaceBlock(obj) => {
                self.push("autoplace ");
                self.fmt_object(&obj.node);
            }
            PlacementItem::Minimize(decl) => {
                self.push("minimize ");
                self.push(&decl.objective.node);
                if let Some(ref subject_to) = decl.subject_to {
                    self.push(" subject_to ");
                    self.fmt_object(&subject_to.node);
                }
            }
            PlacementItem::GroupDecl(group) => {
                self.push("group ");
                self.push(&group.name.node);
                self.push(" ");
                self.fmt_object(&group.body.node);
            }
            PlacementItem::SeparateDecl(sep) => {
                self.push("separate ");
                for (i, g) in sep.groups.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.fmt_dollar_path(&g.node);
                }
                if let Some(body) = &sep.body {
                    self.push(" ");
                    self.fmt_object(&body.node);
                }
            }
        }
    }

    fn fmt_place_decl(&mut self, place: &PlaceDecl) {
        self.push("place ");
        for (i, d) in place.designators.iter().enumerate() {
            if i > 0 {
                self.push(", ");
            }
            self.fmt_entity_name(&d.node);
        }
        self.push(" ");
        self.fmt_object(&place.body.node);
    }

    fn fmt_placement_constraint(&mut self, c: &PlacementConstraintDecl) {
        let (keyword, a, b, body) = match c {
            PlacementConstraintDecl::LeftOf { a, b, body } => ("left_of", a, b, body),
            PlacementConstraintDecl::RightOf { a, b, body } => ("right_of", a, b, body),
            PlacementConstraintDecl::Above { a, b, body } => ("above", a, b, body),
            PlacementConstraintDecl::Below { a, b, body } => ("below", a, b, body),
        };
        self.push(keyword);
        self.push(" ");
        self.fmt_dollar_path(&a.node);
        self.push(", ");
        self.fmt_dollar_path(&b.node);
        if let Some(obj) = body {
            self.push(" ");
            self.fmt_object(&obj.node);
        }
    }

    // ── PcbDoc primitives ─────────────────────────────────────────────────────

    fn fmt_pcbdoc_primitive(&mut self, decl: &PcbDocPrimitiveDecl) {
        self.push(&decl.primitive_type.node);
        if let Some(name) = &decl.name {
            self.push(" ");
            self.fmt_entity_name(&name.node);
        }
        self.push(" ");
        self.fmt_object(&decl.body.node);
    }

    fn fmt_polygon(&mut self, decl: &PolygonDecl) {
        self.fmt_annotation(&decl.annotation);
        self.push("polygon ");
        self.fmt_entity_name(&decl.name.node);
        self.push(" ");
        self.fmt_object(&decl.body.node);
    }

    fn fmt_rule(&mut self, decl: &RuleDecl) {
        self.fmt_annotation(&decl.annotation);
        self.push("rule ");
        self.fmt_entity_name(&decl.name.node);
        self.push(" ");
        self.fmt_object(&decl.body.node);
    }

    fn fmt_class(&mut self, decl: &ClassDecl) {
        self.fmt_annotation(&decl.annotation);
        self.push("class ");
        self.fmt_entity_name(&decl.name.node);
        self.push(" ");
        self.fmt_object(&decl.body.node);
    }

    fn fmt_differential_pair(&mut self, decl: &DifferentialPairDecl) {
        self.push("differential_pair ");
        self.fmt_entity_name(&decl.name.node);
        self.push(" ");
        self.fmt_object(&decl.body.node);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn is_simple_object_item(item: &ObjectItem) -> bool {
    match item {
        ObjectItem::LetBinding(_) => false,
        ObjectItem::Spread(_) => true,
        ObjectItem::Property(p) => is_simple_expr(&p.value.node),
    }
}

fn is_simple_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Object(_) => false,
        Expr::Array(items) => items.iter().all(|i| is_simple_expr(&i.node)),
        Expr::BinOp(l, _, r) => is_simple_expr(&l.node) && is_simple_expr(&r.node),
        Expr::UnaryNeg(e) => is_simple_expr(&e.node),
        Expr::Tuple(a, b) => is_simple_expr(&a.node) && is_simple_expr(&b.node),
        Expr::Path(base, _) => is_simple_expr(&base.node),
        Expr::Index(base, idx) => is_simple_expr(&base.node) && is_simple_expr(&idx.node),
        _ => true,
    }
}

fn escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

fn format_float(f: f64) -> String {
    let s = format!("{}", f);
    if s.contains('.') {
        s
    } else {
        format!("{}.0", s)
    }
}

fn format_dim(val: f64, unit: Unit) -> String {
    let unit_str = match unit {
        Unit::Mil => "mil",
        Unit::Mm => "mm",
        Unit::Inch => "in",
        Unit::Dxp => "dxp",
        Unit::Raw => "raw",
    };
    // Use minimal representation: omit .0 for whole numbers.
    if val.fract() == 0.0 {
        format!("{}{}", val as i64, unit_str)
    } else {
        format!("{}{}", val, unit_str)
    }
}

// ── Main pretty-print entry point ─────────────────────────────────────────────

fn pretty_print(
    source: &str,
    ast: &SpecFile,
    trivia: &[ItemTrivia],
    config: &FormatConfig,
) -> String {
    let mut printer = Printer::new(config);

    for (idx, spanned) in ast.items.iter().enumerate() {
        let item_trivia = trivia.get(idx).cloned().unwrap_or_default();
        let is_first = idx == 0;

        printer.emit_leading_trivia(&item_trivia, is_first);
        printer.fmt_spec_item(&spanned.node);
        printer.emit_trailing_trivia(&item_trivia);
        printer.push_newline();
    }

    // Handle files with no AST items (comment-only files).
    if ast.items.is_empty() {
        let all_trivia = scan_trivia_lines(source);
        let first_comment = all_trivia
            .iter()
            .position(|l| !matches!(l, TriviaLine::Blank));
        let last_comment = all_trivia
            .iter()
            .rposition(|l| !matches!(l, TriviaLine::Blank));
        if let (Some(s), Some(e)) = (first_comment, last_comment) {
            for line in &all_trivia[s..=e] {
                match line {
                    TriviaLine::Blank => printer.push_newline(),
                    TriviaLine::LineComment(c) | TriviaLine::BlockComment(c) => {
                        printer.push(c);
                        printer.push_newline();
                    }
                }
            }
        }
        return printer.out;
    }

    // Handle trailing content after last item (comments at EOF).
    let last_end = ast.items.last().unwrap().span.end as usize;
    let tail = &source[last_end..];
    let tail_trivia = scan_trivia_lines(tail);
    let has_trailing_comment = tail_trivia.iter().any(|l| !matches!(l, TriviaLine::Blank));
    if has_trailing_comment {
        for line in &tail_trivia {
            match line {
                TriviaLine::Blank => printer.push_newline(),
                TriviaLine::LineComment(s) => {
                    printer.push(s);
                    printer.push_newline();
                }
                TriviaLine::BlockComment(s) => {
                    printer.push(s);
                    printer.push_newline();
                }
            }
        }
    }

    printer.out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(source: &str) -> String {
        format_spec(source, &FormatConfig::default())
            .expect("format_spec failed")
            .output
    }

    #[test]
    fn test_idempotency() {
        let input = r#"// Resistors
let passive_pin = { electrical: passive, length: 25 }

component R {
    designator: "R?"
    description: "Resistor"
    pin 1 { ...passive_pin, on: $body.left, at: center }
    parameter Value { text: "{VALUE}" }
    alias RES
    footprint "0402"
}
"#;
        let first = fmt(input);
        let second = fmt(&first);
        assert_eq!(first, second, "formatter is not idempotent");
    }

    #[test]
    fn test_comment_preservation() {
        let input = r#"// First comment
let a = 1

// Second comment
let b = 2
"#;
        let output = fmt(input);
        assert!(
            output.contains("// First comment"),
            "missing first comment in:\n{}",
            output
        );
        assert!(
            output.contains("// Second comment"),
            "missing second comment in:\n{}",
            output
        );
    }

    #[test]
    fn test_inline_object() {
        let input = "let x = { a: 1, b: 2 }\n";
        let output = fmt(input);
        // Should stay inline.
        assert!(
            output.contains("{ a: 1, b: 2 }"),
            "expected inline object in:\n{}",
            output
        );
        assert!(
            !output.contains("{\n"),
            "unexpected multi-line in:\n{}",
            output
        );
    }

    #[test]
    fn test_multiline_object() {
        // 5 items > max_inline_items(4) → multi-line.
        let input = "let x = { a: 1, b: 2, c: 3, d: 4, e: 5 }\n";
        let output = fmt(input);
        assert!(
            output.contains("{\n"),
            "expected multi-line object in:\n{}",
            output
        );
    }

    #[test]
    fn test_expression_formatting() {
        let cases: &[(&str, &str)] = &[
            ("let x = 100mil\n", "let x = 100mil\n"),
            ("let x = 2.54mm\n", "let x = 2.54mm\n"),
            ("let x = #FF0000\n", "let x = #FF0000\n"),
            ("let x = \"hello\\nworld\"\n", "let x = \"hello\\nworld\"\n"),
            ("let x = true\n", "let x = true\n"),
            ("let x = null\n", "let x = null\n"),
            ("let x = $body.left\n", "let x = $body.left\n"),
        ];
        for (input, expected) in cases {
            let output = fmt(input);
            assert_eq!(output, *expected, "input: {:?}", input);
        }
    }

    #[test]
    fn test_component_formatting() {
        let input = r#"component R {
    designator: "R?"
    description: "Resistor"
    body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil), is_solid: true }
    pin 1 { electrical: passive, length: 25, side: outside, on: $body.left, at: center }
    parameter Value { text: "{VALUE}" }
    alias RES
    footprint "0402"
}
"#;
        let output = fmt(input);
        // Re-parse to verify output is valid.
        parse_spec(&output).expect("formatted output should be parseable");
        // Check idempotency.
        let second = fmt(&output);
        assert_eq!(output, second, "component formatting not idempotent");
    }

    #[test]
    fn test_let_binding_formatting() {
        let input = "let passive_pin = { electrical: passive, length: 25 }\n";
        let output = fmt(input);
        assert!(output.starts_with("let passive_pin = "));
    }

    #[test]
    fn test_import_formatting() {
        let cases: &[(&str, &str)] = &[
            ("import \"path/to/lib\"\n", "import \"path/to/lib\"\n"),
            (
                "import \"path/to/lib\" as lib\n",
                "import \"path/to/lib\" as lib\n",
            ),
        ];
        for (input, expected) in cases {
            let output = fmt(input);
            assert_eq!(output, *expected, "input: {:?}", input);
        }
    }

    #[test]
    fn test_binary_ops_precedence() {
        // Multiplication should not add parens when not needed.
        let input = "let x = 2 * 3\n";
        let output = fmt(input);
        assert!(output.contains("2 * 3"), "got: {}", output);
        // Re-parse.
        parse_spec(&output).expect("should be parseable");
    }

    #[test]
    fn test_dim_whole_number_no_decimal() {
        let input = "let x = 100mil\n";
        let output = fmt(input);
        assert!(
            output.contains("100mil"),
            "expected 100mil (no decimal) in:\n{}",
            output
        );
        assert!(
            !output.contains("100.0mil"),
            "unexpected decimal in:\n{}",
            output
        );
    }

    #[test]
    fn test_dim_fractional_keeps_decimal() {
        let input = "let x = 2.54mm\n";
        let output = fmt(input);
        assert!(output.contains("2.54mm"), "expected 2.54mm in:\n{}", output);
    }
}

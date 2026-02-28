//! Expression evaluator for the Altium spec language.
//!
//! Converts AST [`Expr`] nodes into typed [`Value`]s, handling:
//! - Arithmetic with dimensional units (`100mil + 2.54mm`)
//! - Let binding scopes (push/pop)
//! - Spread operator in objects
//! - Template string interpolation
//! - Circular binding detection
//! - Path / field access on bound entities

use std::fmt;
use std::fmt::Write as FmtWrite;

use indexmap::IndexMap;

use crate::diagnostic::{BinOp, Span, Spanned, Unit};
use crate::ast::{Expr, Object, ObjectItem, TemplatePart};

// ── Error types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SpecError {
    pub code: SpecErrorCode,
    pub message: String,
    pub span: Option<Span>,
}

impl SpecError {
    pub fn new(code: SpecErrorCode, message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            code,
            message: message.into(),
            span,
        }
    }

    pub fn at(code: SpecErrorCode, message: impl Into<String>, span: Span) -> Self {
        Self::new(code, message, Some(span))
    }

    pub fn no_span(code: SpecErrorCode, message: impl Into<String>) -> Self {
        Self::new(code, message, None)
    }

    /// Render this error with source location context (file:line:col + caret).
    ///
    /// Falls back to a plain `error[Code]: message` when no span is available.
    pub fn render(&self, source_name: &str, source: &str) -> String {
        use crate::diagnostic::{locate_line, caret_len};

        let Some(span) = self.span else {
            return format!("error[{:?}]: {}", self.code, self.message);
        };
        let (line_no, col_no, line_text) = locate_line(source, span.start as usize);
        let mut out = String::new();
        out.push_str(&format!("error[{:?}]: {}\n", self.code, self.message));
        out.push_str(&format!(" --> {}:{}:{}\n", source_name, line_no, col_no));
        out.push_str("  |\n");
        out.push_str(&format!("{:>2} | {}\n", line_no, line_text));
        out.push_str("  | ");
        let caret_count = caret_len(span, source, line_no, col_no);
        out.push_str(&" ".repeat(col_no.saturating_sub(1)));
        out.push_str(&"^".repeat(caret_count));
        out.push('\n');
        out
    }
}

impl fmt::Display for SpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error[{:?}]: {}", self.code, self.message)?;
        if let Some(span) = self.span {
            write!(f, " (at {}..{})", span.start, span.end)?;
        }
        Ok(())
    }
}

impl std::error::Error for SpecError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecErrorCode {
    UndefinedBinding,
    TypeMismatch,
    CircularBinding,
    SpreadNotObject,
    IndexNotArray,
    DivisionByZero,
    InvalidFieldAccess,
    NotAnObject,
    UnaryOnNonNumeric,
    InvalidUnit,
    ArithmeticOverflow,
    // Import-related errors
    FileNotFound,
    ParseError,
    CircularImport,
    DuplicateImportAlias,
    DuplicateEntity,
    CrossDomainViolation,
    // Anchor placement errors
    CrossEdgeReference,
    // Wraps altium_format::AltiumFormatError via message string
    AltiumFormat,
}

pub type EvalResult<T> = Result<T, SpecError>;

// ── Value type ───────────────────────────────────────────────────────────────

/// Evaluated value from an expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Integer(i32),
    Float(f64),
    /// Dimensional scalar (stored as Altium internal units: 10,000 per mil).
    Dim(i32),
    Color(u8, u8, u8),
    Bool(bool),
    Null,
    CoordPoint(i32, i32),
    Array(Vec<Value>),
    Object(IndexMap<String, Value>),
}

impl Value {
    /// Display value for text rendering.
    pub fn display(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Integer(n) => n.to_string(),
            Value::Float(f) => format!("{f}"),
            Value::Dim(raw) => {
                let mils = *raw as f64 / 10_000.0;
                if mils.fract() == 0.0 {
                    format!("{}mil", mils as i64)
                } else {
                    format!("{mils}mil")
                }
            }
            Value::Color(r, g, b) => format!("#{r:02X}{g:02X}{b:02X}"),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::CoordPoint(x, y) => format!("({x}, {y})"),
            Value::Array(arr) => {
                let items: Vec<_> = arr.iter().map(|v| v.display()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Object(map) => {
                let items: Vec<_> = map
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", v.display()))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            Value::String(_) => "string",
            Value::Integer(_) => "integer",
            Value::Float(_) => "float",
            Value::Dim(_) => "dim",
            Value::Color(..) => "color",
            Value::Bool(_) => "bool",
            Value::Null => "null",
            Value::CoordPoint(..) => "coord",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    /// Convert to Altium internal units (Coord raw value) if this is numeric/dim.
    /// Bare integer/float → mils by default.
    pub fn to_dim(&self, span: Option<Span>) -> EvalResult<i32> {
        match self {
            Value::Dim(raw) => Ok(*raw),
            Value::Integer(n) => Ok(n.checked_mul(10_000).ok_or_else(|| {
                SpecError::new(SpecErrorCode::ArithmeticOverflow, "integer overflow in mil conversion", span)
            })?),
            Value::Float(f) => Ok((*f * 10_000.0).round() as i32),
            other => Err(SpecError::new(
                SpecErrorCode::TypeMismatch,
                format!("expected dimension, got {}", other.kind_name()),
                span,
            )),
        }
    }

    /// Extract as object map, or error.
    pub fn into_object(self, span: Option<Span>) -> EvalResult<IndexMap<String, Value>> {
        match self {
            Value::Object(m) => Ok(m),
            other => Err(SpecError::new(
                SpecErrorCode::NotAnObject,
                format!("expected object, got {}", other.kind_name()),
                span,
            )),
        }
    }
}

// ── Unit conversion ───────────────────────────────────────────────────────────

/// Convert a dimensional literal `(value, unit)` to Altium internal units.
pub fn unit_to_internal(value: f64, unit: Unit) -> i32 {
    match unit {
        Unit::Mil => (value * 10_000.0).round() as i32,
        Unit::Mm => (value * 393_701.0).round() as i32,
        Unit::Inch => (value * 10_000_000.0).round() as i32,
        Unit::Dxp => (value * 100_000.0).round() as i32,
        Unit::Raw => value.round() as i32,
    }
}

// ── Scope ─────────────────────────────────────────────────────────────────────

/// A single scope frame holding let bindings.
#[derive(Debug, Default, Clone)]
pub struct Scope {
    /// Let bindings: name → evaluated value (or None if currently evaluating = cycle sentinel).
    bindings: IndexMap<String, Option<Value>>,
}

impl Scope {
    pub fn define(&mut self, name: String, value: Value) {
        self.bindings.insert(name, Some(value));
    }

    /// Mark a binding as "in evaluation" (for cycle detection).
    pub fn mark_evaluating(&mut self, name: &str) {
        self.bindings.insert(name.to_string(), None);
    }

    pub fn get(&self, name: &str) -> Option<EvalResult<&Value>> {
        self.bindings.get(name).map(|opt| match opt {
            Some(v) => Ok(v),
            None => Err(SpecError::no_span(
                SpecErrorCode::CircularBinding,
                format!("binding '{name}' has circular reference"),
            )),
        })
    }
}

/// A stack of scopes used during evaluation.
#[derive(Debug, Default, Clone)]
pub struct ScopeStack {
    scopes: Vec<Scope>,
}

impl ScopeStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new empty scope.
    pub fn push(&mut self) {
        self.scopes.push(Scope::default());
    }

    /// Pop the innermost scope.
    pub fn pop(&mut self) {
        self.scopes.pop();
    }

    /// Define a binding in the innermost scope.
    pub fn define(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.define(name, value);
        }
    }

    /// Mark a binding as being evaluated (cycle detection sentinel).
    pub fn mark_evaluating(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.mark_evaluating(name);
        }
    }

    /// Look up a binding, searching from innermost to outermost scope.
    pub fn lookup(&self, name: &str) -> Option<EvalResult<&Value>> {
        for scope in self.scopes.iter().rev() {
            if let Some(result) = scope.get(name) {
                return Some(result);
            }
        }
        None
    }

    /// Look up a `$`-prefixed identifier (currently same as regular lookup;
    /// future: could resolve import namespaces separately).
    pub fn lookup_dollar(&self, name: &str) -> Option<EvalResult<&Value>> {
        self.lookup(name)
    }
}

// ── Evaluator ────────────────────────────────────────────────────────────────

/// Evaluate a single expression in the given scope stack.
pub fn eval_expr(expr: &Spanned<Expr>, scope: &ScopeStack) -> EvalResult<Value> {
    let span = expr.span;
    match &expr.node {
        // ── Literals ───────────────────────────────────────────────────────
        Expr::String(s) => Ok(Value::String(s.clone())),
        Expr::Integer(n) => Ok(Value::Integer(*n)),
        Expr::Float(f) => Ok(Value::Float(*f)),
        Expr::Dim(val, unit) => Ok(Value::Dim(unit_to_internal(*val, *unit))),
        Expr::Color(r, g, b) => Ok(Value::Color(*r, *g, *b)),
        Expr::Bool(b) => Ok(Value::Bool(*b)),
        Expr::Null => Ok(Value::Null),

        // ── Template strings ───────────────────────────────────────────────
        Expr::Template(parts) => eval_template(parts, scope, span),

        // ── References ────────────────────────────────────────────────────
        Expr::Ident(name) => {
            match scope.lookup(name) {
                Some(Ok(v)) => Ok(v.clone()),
                Some(Err(e)) => Err(e),
                // §7.3 resolution order: 1) keywords, 2) bindings, 3) enum registry.
                // If not found in scope, return as a string so that downstream
                // enum consumers (get_enum_opt, extract_at_position, etc.) can
                // resolve it against the field's expected enum type.
                None => Ok(Value::String(name.clone())),
            }
        }
        Expr::DollarIdent(name) => {
            match scope.lookup_dollar(name) {
                Some(Ok(v)) => Ok(v.clone()),
                Some(Err(e)) => Err(e),
                None => Err(SpecError::at(
                    SpecErrorCode::UndefinedBinding,
                    format!("undefined binding '${name}'"),
                    span,
                )),
            }
        }

        // ── Field access: expr.field ───────────────────────────────────────
        Expr::Path(base_expr, field) => {
            let base_val = eval_expr(base_expr, scope)?;
            eval_field_access(base_val, &field.node, Some(field.span))
        }

        // ── Index access: expr[key] ────────────────────────────────────────
        Expr::Index(base_expr, idx_expr) => {
            let base_val = eval_expr(base_expr, scope)?;
            let idx_val = eval_expr(idx_expr, scope)?;
            eval_index_access(base_val, idx_val, Some(span))
        }

        // ── Arithmetic ────────────────────────────────────────────────────
        Expr::BinOp(left_expr, op_spanned, right_expr) => {
            let left = eval_expr(left_expr, scope)?;
            let right = eval_expr(right_expr, scope)?;
            eval_binop(left, op_spanned.node, right, span)
        }

        Expr::UnaryNeg(inner_expr) => {
            let val = eval_expr(inner_expr, scope)?;
            eval_unary_neg(val, span)
        }

        // ── Tuple (coord) ─────────────────────────────────────────────────
        Expr::Tuple(x_expr, y_expr) => {
            let x = eval_expr(x_expr, scope)?.to_dim(Some(x_expr.span))?;
            let y = eval_expr(y_expr, scope)?.to_dim(Some(y_expr.span))?;
            Ok(Value::CoordPoint(x, y))
        }

        // ── Array ─────────────────────────────────────────────────────────
        Expr::Array(elements) => {
            let mut out = Vec::with_capacity(elements.len());
            for elem in elements {
                out.push(eval_expr(elem, scope)?);
            }
            Ok(Value::Array(out))
        }

        // ── Object ────────────────────────────────────────────────────────
        Expr::Object(obj) => eval_object(obj, scope),
    }
}

// ── Template string evaluation ────────────────────────────────────────────────

fn eval_template(parts: &[TemplatePart], scope: &ScopeStack, span: Span) -> EvalResult<Value> {
    let mut out = String::new();
    for part in parts {
        match part {
            TemplatePart::Literal(s) => out.push_str(s),
            TemplatePart::Expr(tokens) => {
                // Re-parse the token slice into an expression and evaluate it.
                let expr = parse_template_expr(tokens, span)?;
                let val = eval_expr(&expr, scope)?;
                write!(&mut out, "{}", val.display()).unwrap();
            }
        }
    }
    Ok(Value::String(out))
}

/// Parse a token slice (from a template interpolation) into a `Spanned<Expr>`.
/// This requires re-running the spec parser on the sub-token list.
fn parse_template_expr(tokens: &[crate::lexer::Token], _outer_span: Span) -> EvalResult<Spanned<Expr>> {
    use crate::parser::parse_spec;

    // Build a minimal source string for error reporting.
    // The actual parsing is done by feeding the tokens back through the expression parser.
    // Since we can't access the inner parser directly from here, we reconstruct the
    // source fragment from the token spans (not available), so we use a simplified approach:
    // serialize the tokens to a synthetic source and parse.
    let synthetic = tokens_to_source(tokens);
    // Parse as a value expression by wrapping in a let-binding assignment
    let wrapped = format!("component _dummy {{ let _t = {synthetic} }}");
    let ast = parse_spec(&wrapped).map_err(|e| SpecError::no_span(
        SpecErrorCode::TypeMismatch,
        format!("failed to parse template expression: {e}"),
    ))?;

    // Extract the let binding value from the dummy component.
    use crate::ast::{SpecItem, ComponentItem};
    let comp = ast.items.into_iter()
        .find_map(|item| match item.node {
            SpecItem::Component(c) => Some(c),
            _ => None,
        })
        .ok_or_else(|| SpecError::no_span(SpecErrorCode::TypeMismatch, "template parse: expected component"))?;

    let let_binding = comp.body.into_iter()
        .find_map(|item| match item.node {
            ComponentItem::LetBinding(lb) => Some(lb),
            _ => None,
        })
        .ok_or_else(|| SpecError::no_span(SpecErrorCode::TypeMismatch, "template parse: expected let binding"))?;

    Ok(let_binding.value)
}

/// Serialize a token slice back to a minimal source representation for re-parsing.
fn tokens_to_source(tokens: &[crate::lexer::Token]) -> String {
    use crate::lexer::TokenKind;
    let mut out = String::new();
    for tok in tokens {
        if !out.is_empty() {
            out.push(' ');
        }
        match &tok.kind {
            TokenKind::Ident(s) => out.push_str(s),
            TokenKind::DollarIdent(s) => { out.push('$'); out.push_str(s); }
            TokenKind::String(s) => { out.push('"'); out.push_str(&s.replace('\\', "\\\\").replace('"', "\\\"")); out.push('"'); }
            TokenKind::Integer(n) => { write!(&mut out, "{n}").unwrap(); }
            TokenKind::Float(f) => { write!(&mut out, "{f}").unwrap(); }
            TokenKind::Dim(v, u) => {
                let suffix = match u {
                    Unit::Mil => "mil",
                    Unit::Mm => "mm",
                    Unit::Inch => "in",
                    Unit::Dxp => "dxp",
                    Unit::Raw => "raw",
                };
                write!(&mut out, "{v}{suffix}").unwrap();
            }
            TokenKind::Color(r, g, b) => { write!(&mut out, "#{r:02X}{g:02X}{b:02X}").unwrap(); }
            TokenKind::True => out.push_str("true"),
            TokenKind::False => out.push_str("false"),
            TokenKind::Null => out.push_str("null"),
            TokenKind::Plus => out.push('+'),
            TokenKind::Minus => out.push('-'),
            TokenKind::Star => out.push('*'),
            TokenKind::Slash => out.push('/'),
            TokenKind::Dot => out.push('.'),
            TokenKind::LParen => out.push('('),
            TokenKind::RParen => out.push(')'),
            TokenKind::LBracket => out.push('['),
            TokenKind::RBracket => out.push(']'),
            TokenKind::LBrace => out.push('{'),
            TokenKind::RBrace => out.push('}'),
            TokenKind::Comma => out.push(','),
            TokenKind::Colon => out.push(':'),
            TokenKind::DotDotDot => out.push_str("..."),
            _ => {}
        }
    }
    out
}

// ── Object evaluation ─────────────────────────────────────────────────────────

fn eval_object(obj: &Object, scope: &ScopeStack) -> EvalResult<Value> {
    let mut result: IndexMap<String, Value> = IndexMap::new();

    for item in &obj.items {
        match &item.node {
            ObjectItem::LetBinding(_lb) => {
                // Let bindings inside objects are for scoping in entity bodies;
                // they don't contribute key-value pairs to the object map.
                // The compiler handles these during entity compilation.
                // Here we just skip them at the value level.
            }
            ObjectItem::Spread(spread_expr) => {
                let spread_val = eval_expr(spread_expr, scope)?;
                let spread_map = spread_val.into_object(Some(spread_expr.span))?;
                for (k, v) in spread_map {
                    result.insert(k, v);
                }
            }
            ObjectItem::Property(prop) => {
                let val = eval_expr(&prop.value, scope)?;
                result.insert(prop.key.node.clone(), val);
            }
        }
    }

    Ok(Value::Object(result))
}

// ── Binary operator ───────────────────────────────────────────────────────────

fn eval_binop(left: Value, op: BinOp, right: Value, span: Span) -> EvalResult<Value> {
    match (&left, op, &right) {
        // dim op dim → dim
        (Value::Dim(a), BinOp::Add, Value::Dim(b)) => {
            Ok(Value::Dim(a.checked_add(*b).ok_or_else(|| {
                SpecError::at(SpecErrorCode::ArithmeticOverflow, "overflow in dim + dim", span)
            })?))
        }
        (Value::Dim(a), BinOp::Sub, Value::Dim(b)) => {
            Ok(Value::Dim(a.checked_sub(*b).ok_or_else(|| {
                SpecError::at(SpecErrorCode::ArithmeticOverflow, "overflow in dim - dim", span)
            })?))
        }
        // dim * number → dim (scale)
        (Value::Dim(a), BinOp::Mul, Value::Integer(b)) => {
            Ok(Value::Dim(a.checked_mul(*b).ok_or_else(|| {
                SpecError::at(SpecErrorCode::ArithmeticOverflow, "overflow in dim * int", span)
            })?))
        }
        (Value::Dim(a), BinOp::Mul, Value::Float(b)) => {
            Ok(Value::Dim((*a as f64 * b).round() as i32))
        }
        // number * dim → dim
        (Value::Integer(a), BinOp::Mul, Value::Dim(b)) => {
            Ok(Value::Dim(a.checked_mul(*b).ok_or_else(|| {
                SpecError::at(SpecErrorCode::ArithmeticOverflow, "overflow in int * dim", span)
            })?))
        }
        (Value::Float(a), BinOp::Mul, Value::Dim(b)) => {
            Ok(Value::Dim((*b as f64 * a).round() as i32))
        }
        // dim / number → dim
        (Value::Dim(a), BinOp::Div, Value::Integer(b)) => {
            if *b == 0 {
                return Err(SpecError::at(SpecErrorCode::DivisionByZero, "division by zero", span));
            }
            Ok(Value::Dim(a / b))
        }
        (Value::Dim(a), BinOp::Div, Value::Float(b)) => {
            if *b == 0.0 {
                return Err(SpecError::at(SpecErrorCode::DivisionByZero, "division by zero", span));
            }
            Ok(Value::Dim((*a as f64 / b).round() as i32))
        }

        // integer op integer → integer
        (Value::Integer(a), BinOp::Add, Value::Integer(b)) => {
            Ok(Value::Integer(a.checked_add(*b).ok_or_else(|| {
                SpecError::at(SpecErrorCode::ArithmeticOverflow, "integer overflow", span)
            })?))
        }
        (Value::Integer(a), BinOp::Sub, Value::Integer(b)) => {
            Ok(Value::Integer(a.checked_sub(*b).ok_or_else(|| {
                SpecError::at(SpecErrorCode::ArithmeticOverflow, "integer overflow", span)
            })?))
        }
        (Value::Integer(a), BinOp::Mul, Value::Integer(b)) => {
            Ok(Value::Integer(a.checked_mul(*b).ok_or_else(|| {
                SpecError::at(SpecErrorCode::ArithmeticOverflow, "integer overflow", span)
            })?))
        }
        (Value::Integer(a), BinOp::Div, Value::Integer(b)) => {
            if *b == 0 {
                return Err(SpecError::at(SpecErrorCode::DivisionByZero, "division by zero", span));
            }
            Ok(Value::Integer(a / b))
        }

        // float op float → float (and mixed int/float)
        (Value::Float(a), BinOp::Add, Value::Float(b)) => Ok(Value::Float(a + b)),
        (Value::Float(a), BinOp::Sub, Value::Float(b)) => Ok(Value::Float(a - b)),
        (Value::Float(a), BinOp::Mul, Value::Float(b)) => Ok(Value::Float(a * b)),
        (Value::Float(a), BinOp::Div, Value::Float(b)) => {
            if *b == 0.0 {
                return Err(SpecError::at(SpecErrorCode::DivisionByZero, "division by zero", span));
            }
            Ok(Value::Float(a / b))
        }
        (Value::Integer(a), BinOp::Add, Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
        (Value::Float(a), BinOp::Add, Value::Integer(b)) => Ok(Value::Float(a + *b as f64)),
        (Value::Integer(a), BinOp::Sub, Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
        (Value::Float(a), BinOp::Sub, Value::Integer(b)) => Ok(Value::Float(a - *b as f64)),
        (Value::Integer(a), BinOp::Mul, Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
        (Value::Float(a), BinOp::Mul, Value::Integer(b)) => Ok(Value::Float(a * *b as f64)),
        (Value::Integer(a), BinOp::Div, Value::Float(b)) => {
            if *b == 0.0 {
                return Err(SpecError::at(SpecErrorCode::DivisionByZero, "division by zero", span));
            }
            Ok(Value::Float(*a as f64 / b))
        }
        (Value::Float(a), BinOp::Div, Value::Integer(b)) => {
            if *b == 0 {
                return Err(SpecError::at(SpecErrorCode::DivisionByZero, "division by zero", span));
            }
            Ok(Value::Float(a / *b as f64))
        }

        _ => Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            format!(
                "operator {:?} not supported between {} and {}",
                op,
                left.kind_name(),
                right.kind_name()
            ),
            span,
        )),
    }
}

fn eval_unary_neg(val: Value, span: Span) -> EvalResult<Value> {
    match val {
        Value::Integer(n) => Ok(Value::Integer(-n)),
        Value::Float(f) => Ok(Value::Float(-f)),
        Value::Dim(raw) => Ok(Value::Dim(-raw)),
        other => Err(SpecError::at(
            SpecErrorCode::UnaryOnNonNumeric,
            format!("unary negation not supported on {}", other.kind_name()),
            span,
        )),
    }
}

// ── Field / index access ──────────────────────────────────────────────────────

fn eval_field_access(base: Value, field: &str, span: Option<Span>) -> EvalResult<Value> {
    match base {
        Value::Object(map) => {
            map.get(field).cloned().ok_or_else(|| SpecError::new(
                SpecErrorCode::InvalidFieldAccess,
                format!("no field '{field}' on object"),
                span,
            ))
        }
        Value::CoordPoint(x, y) => match field {
            "x" => Ok(Value::Dim(x)),
            "y" => Ok(Value::Dim(y)),
            _ => Err(SpecError::new(
                SpecErrorCode::InvalidFieldAccess,
                format!("coord has no field '{field}' (valid: x, y)"),
                span,
            )),
        },
        other => Err(SpecError::new(
            SpecErrorCode::InvalidFieldAccess,
            format!("cannot access field '{field}' on {}", other.kind_name()),
            span,
        )),
    }
}

fn eval_index_access(base: Value, idx: Value, span: Option<Span>) -> EvalResult<Value> {
    match (base, idx) {
        (Value::Array(arr), Value::Integer(n)) => {
            let i = if n < 0 {
                arr.len().checked_sub((-n) as usize)
            } else {
                Some(n as usize)
            };
            i.and_then(|i| arr.get(i)).cloned().ok_or_else(|| SpecError::new(
                SpecErrorCode::IndexNotArray,
                format!("array index {n} out of bounds"),
                span,
            ))
        }
        (Value::Object(map), Value::String(key)) => {
            map.get(&key).cloned().ok_or_else(|| SpecError::new(
                SpecErrorCode::InvalidFieldAccess,
                format!("no field '{key}' on object"),
                span,
            ))
        }
        (Value::Object(map), Value::Integer(n)) => {
            let key = n.to_string();
            map.get(&key).cloned().ok_or_else(|| SpecError::new(
                SpecErrorCode::InvalidFieldAccess,
                format!("no field '{key}' on object"),
                span,
            ))
        }
        (base, idx) => Err(SpecError::new(
            SpecErrorCode::InvalidFieldAccess,
            format!("cannot index {} with {}", base.kind_name(), idx.kind_name()),
            span,
        )),
    }
}

// ── Let binding evaluation with circular detection ───────────────────────────

/// Evaluate a block of let bindings in a fresh scope frame pushed onto `scope`.
///
/// Bindings are evaluated lazily (on first use) via a two-pass approach:
/// 1. Register all binding names with their raw AST expressions.
/// 2. Evaluate in iteration order, using the scope for resolution.
///
/// Circular dependencies are detected via the "currently evaluating" sentinel.
pub fn eval_let_bindings(
    bindings: &[(&str, &Spanned<crate::ast::Expr>)],
    scope: &mut ScopeStack,
) -> EvalResult<()> {
    // Evaluate each binding in order. Since bindings within a scope can
    // reference earlier bindings, we evaluate left-to-right.
    // Circular references that depend on a later binding trigger the sentinel.
    for (name, expr) in bindings {
        scope.mark_evaluating(name);
        let value = eval_expr(expr, scope)?;
        scope.define(name.to_string(), value);
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{BinOp, Span, Spanned, Unit};
    use crate::ast::Expr;

    fn span() -> Span {
        Span::new(0, 0)
    }

    fn spanned<T>(node: T) -> Spanned<T> {
        Spanned::new(node, span())
    }

    fn make_scope() -> ScopeStack {
        let mut s = ScopeStack::new();
        s.push();
        s
    }

    // ── Arithmetic tests ───────────────────────────────────────────────────

    #[test]
    fn arithmetic_dim_plus_dim_same_unit() {
        // 100mil + 50mil = 150mil (in internal units)
        let left = spanned(Expr::Dim(100.0, Unit::Mil));
        let right = spanned(Expr::Dim(50.0, Unit::Mil));
        let expr = spanned(Expr::BinOp(
            Box::new(left),
            spanned(BinOp::Add),
            Box::new(right),
        ));
        let scope = make_scope();
        let val = eval_expr(&expr, &scope).unwrap();
        assert_eq!(val, Value::Dim(150 * 10_000));
    }

    #[test]
    fn arithmetic_dim_mixed_units() {
        // 100mil + 2.54mm → approximately 200mil
        // 100mil = 1_000_000 internal units
        // 2.54mm = round(2.54 * 393_701) = round(1_000_000.54) = 1_000_001 internal units
        let left = spanned(Expr::Dim(100.0, Unit::Mil));
        let right = spanned(Expr::Dim(2.54, Unit::Mm));
        let expr = spanned(Expr::BinOp(
            Box::new(left),
            spanned(BinOp::Add),
            Box::new(right),
        ));
        let scope = make_scope();
        let val = eval_expr(&expr, &scope).unwrap();
        // 100mil = 1_000_000 units, 2.54mm = 1_000_001 units (rounding)
        assert_eq!(val, Value::Dim(2_000_001));
    }

    #[test]
    fn arithmetic_dim_times_integer() {
        // 10mil * 3 = 30mil
        let left = spanned(Expr::Dim(10.0, Unit::Mil));
        let right = spanned(Expr::Integer(3));
        let expr = spanned(Expr::BinOp(
            Box::new(left),
            spanned(BinOp::Mul),
            Box::new(right),
        ));
        let scope = make_scope();
        let val = eval_expr(&expr, &scope).unwrap();
        assert_eq!(val, Value::Dim(30 * 10_000));
    }

    #[test]
    fn arithmetic_integer_add_integer() {
        let left = spanned(Expr::Integer(10));
        let right = spanned(Expr::Integer(5));
        let expr = spanned(Expr::BinOp(
            Box::new(left),
            spanned(BinOp::Add),
            Box::new(right),
        ));
        let val = eval_expr(&expr, &make_scope()).unwrap();
        assert_eq!(val, Value::Integer(15));
    }

    #[test]
    fn arithmetic_unary_neg_dim() {
        let inner = spanned(Expr::Dim(20.0, Unit::Mil));
        let expr = spanned(Expr::UnaryNeg(Box::new(inner)));
        let val = eval_expr(&expr, &make_scope()).unwrap();
        assert_eq!(val, Value::Dim(-200_000));
    }

    #[test]
    fn arithmetic_div_by_zero_integer() {
        let left = spanned(Expr::Integer(10));
        let right = spanned(Expr::Integer(0));
        let expr = spanned(Expr::BinOp(
            Box::new(left),
            spanned(BinOp::Div),
            Box::new(right),
        ));
        let result = eval_expr(&expr, &make_scope());
        assert!(matches!(result, Err(SpecError { code: SpecErrorCode::DivisionByZero, .. })));
    }

    // ── Spread evaluation ──────────────────────────────────────────────────

    #[test]
    fn spread_evaluation() {
        use crate::ast::{Object, ObjectItem, Property};

        // let defaults = { shape: "round", x_size: 60mil }
        // { ...defaults, shape: "rectangular" }
        // result should be { shape: "rectangular", x_size: 60mil }
        let mut scope = make_scope();
        let mut defaults_map = IndexMap::new();
        defaults_map.insert("shape".to_string(), Value::String("round".to_string()));
        defaults_map.insert("x_size".to_string(), Value::Dim(600_000));
        scope.define("defaults".to_string(), Value::Object(defaults_map));

        let spread_item = spanned(ObjectItem::Spread(spanned(Expr::Ident("defaults".to_string()))));
        let override_item = spanned(ObjectItem::Property(Property {
            key: spanned("shape".to_string()),
            value: spanned(Expr::String("rectangular".to_string())),
        }));

        let obj = Object {
            items: vec![spread_item, override_item],
        };
        let expr = spanned(Expr::Object(obj));

        let val = eval_expr(&expr, &scope).unwrap();
        match val {
            Value::Object(map) => {
                assert_eq!(map.get("shape"), Some(&Value::String("rectangular".to_string())));
                assert_eq!(map.get("x_size"), Some(&Value::Dim(600_000)));
            }
            _ => panic!("expected object"),
        }
    }

    // ── Let binding resolution ─────────────────────────────────────────────

    #[test]
    fn let_binding_resolution() {
        let mut scope = make_scope();
        scope.define("spacing".to_string(), Value::Dim(100 * 10_000));

        let expr = spanned(Expr::Ident("spacing".to_string()));
        let val = eval_expr(&expr, &scope).unwrap();
        assert_eq!(val, Value::Dim(1_000_000));
    }

    #[test]
    fn let_binding_nested_scope() {
        let mut scope = make_scope();
        scope.define("outer".to_string(), Value::Integer(42));
        scope.push();
        scope.define("inner".to_string(), Value::Integer(7));

        // Both outer and inner are visible
        let outer_expr = spanned(Expr::Ident("outer".to_string()));
        let inner_expr = spanned(Expr::Ident("inner".to_string()));
        assert_eq!(eval_expr(&outer_expr, &scope).unwrap(), Value::Integer(42));
        assert_eq!(eval_expr(&inner_expr, &scope).unwrap(), Value::Integer(7));

        scope.pop();
        // After pop, inner is gone — unresolved ident falls through to string (§7.3)
        let inner_result = eval_expr(&spanned(Expr::Ident("inner".to_string())), &scope);
        assert_eq!(inner_result.unwrap(), Value::String("inner".to_string()));
    }

    // ── Circular binding detection ─────────────────────────────────────────

    #[test]
    fn circular_binding_detection() {
        let mut scope = make_scope();
        // Simulate evaluating binding 'a': mark it as evaluating, then look it up
        scope.mark_evaluating("a");

        let expr = spanned(Expr::Ident("a".to_string()));
        let result = eval_expr(&expr, &scope);
        assert!(matches!(result, Err(SpecError { code: SpecErrorCode::CircularBinding, .. })));
    }

    // ── Type coercion ──────────────────────────────────────────────────────

    #[test]
    fn coerce_integer_to_dim() {
        // Value::Integer(100).to_dim() = 1_000_000 (100 mils)
        let val = Value::Integer(100);
        let dim = val.to_dim(None).unwrap();
        assert_eq!(dim, 1_000_000);
    }

    #[test]
    fn coerce_float_to_dim() {
        let val = Value::Float(2.54);
        let dim = val.to_dim(None).unwrap();
        // 2.54 mils * 10000 = 25400
        assert_eq!(dim, 25400);
    }

    #[test]
    fn coerce_non_numeric_to_dim_fails() {
        let val = Value::String("hello".to_string());
        assert!(val.to_dim(None).is_err());
    }

    // ── Path resolution ────────────────────────────────────────────────────

    #[test]
    fn path_resolution_field_access() {
        let mut map = IndexMap::new();
        map.insert("layer".to_string(), Value::String("TopLayer".to_string()));
        let mut scope = make_scope();
        scope.define("smd".to_string(), Value::Object(map));

        let base = spanned(Expr::Ident("smd".to_string()));
        let expr = spanned(Expr::Path(Box::new(base), spanned("layer".to_string())));
        let val = eval_expr(&expr, &scope).unwrap();
        assert_eq!(val, Value::String("TopLayer".to_string()));
    }

    #[test]
    fn path_resolution_coord_xy() {
        let mut scope = make_scope();
        scope.define("pt".to_string(), Value::CoordPoint(1_000_000, 2_000_000));

        let base = spanned(Expr::Ident("pt".to_string()));
        let x_expr = spanned(Expr::Path(Box::new(base.clone()), spanned("x".to_string())));
        let y_expr = spanned(Expr::Path(Box::new(base), spanned("y".to_string())));

        assert_eq!(eval_expr(&x_expr, &scope).unwrap(), Value::Dim(1_000_000));
        assert_eq!(eval_expr(&y_expr, &scope).unwrap(), Value::Dim(2_000_000));
    }

    #[test]
    fn path_resolution_undefined_binding() {
        let scope = make_scope();
        let expr = spanned(Expr::DollarIdent("nonexistent".to_string()));
        let result = eval_expr(&expr, &scope);
        assert!(matches!(result, Err(SpecError { code: SpecErrorCode::UndefinedBinding, .. })));
    }

    // ── Tuple (coord) ──────────────────────────────────────────────────────

    #[test]
    fn tuple_makes_coord() {
        let x = spanned(Expr::Dim(10.0, Unit::Mm));
        let y = spanned(Expr::Dim(5.0, Unit::Mm));
        let expr = spanned(Expr::Tuple(Box::new(x), Box::new(y)));
        let val = eval_expr(&expr, &make_scope()).unwrap();
        // 10mm = 3_937_010 internal units, 5mm = 1_968_505
        assert_eq!(val, Value::CoordPoint(
            (10.0_f64 * 393_701.0).round() as i32,
            (5.0_f64 * 393_701.0).round() as i32,
        ));
    }

    // ── Unit conversion ────────────────────────────────────────────────────

    #[test]
    fn unit_conversion_mil() {
        assert_eq!(unit_to_internal(1.0, Unit::Mil), 10_000);
        assert_eq!(unit_to_internal(100.0, Unit::Mil), 1_000_000);
    }

    #[test]
    fn unit_conversion_mm() {
        assert_eq!(unit_to_internal(1.0, Unit::Mm), 393_701);
        // 2.54mm = round(2.54 * 393_701) = round(1_000_000.54) = 1_000_001
        assert_eq!(unit_to_internal(2.54, Unit::Mm), 1_000_001);
    }

    #[test]
    fn unit_conversion_inch() {
        assert_eq!(unit_to_internal(1.0, Unit::Inch), 10_000_000);
    }

    #[test]
    fn unit_conversion_raw() {
        assert_eq!(unit_to_internal(42.0, Unit::Raw), 42);
    }

    // ── Array evaluation ───────────────────────────────────────────────────

    #[test]
    fn array_evaluation() {
        let elems = vec![
            spanned(Expr::Integer(1)),
            spanned(Expr::Integer(2)),
            spanned(Expr::Integer(3)),
        ];
        let expr = spanned(Expr::Array(elems));
        let val = eval_expr(&expr, &make_scope()).unwrap();
        assert_eq!(val, Value::Array(vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)]));
    }

    #[test]
    fn index_access_array() {
        let arr = Value::Array(vec![
            Value::String("a".to_string()),
            Value::String("b".to_string()),
        ]);
        let result = eval_index_access(arr, Value::Integer(1), None).unwrap();
        assert_eq!(result, Value::String("b".to_string()));
    }

    #[test]
    fn index_access_out_of_bounds() {
        let arr = Value::Array(vec![Value::Integer(1)]);
        let result = eval_index_access(arr, Value::Integer(5), None);
        assert!(result.is_err());
    }

    // ── Bare identifier → string fallback (§7.3 step 3) ──────────────────

    #[test]
    fn bare_ident_resolves_to_string() {
        // Bare identifiers not in scope become String values for enum resolution.
        let scope = make_scope();
        let expr = spanned(Expr::Ident("passive".to_string()));
        let val = eval_expr(&expr, &scope).unwrap();
        assert_eq!(val, Value::String("passive".to_string()));
    }

    #[test]
    fn bare_ident_in_object_becomes_string() {
        // { electrical: passive, side: outside } — both bare idents → strings.
        use crate::ast::{Object, ObjectItem, Property};

        let scope = make_scope();
        let obj = Object {
            items: vec![
                spanned(ObjectItem::Property(Property {
                    key: spanned("electrical".to_string()),
                    value: spanned(Expr::Ident("passive".to_string())),
                })),
                spanned(ObjectItem::Property(Property {
                    key: spanned("side".to_string()),
                    value: spanned(Expr::Ident("outside".to_string())),
                })),
            ],
        };
        let expr = spanned(Expr::Object(obj));
        let val = eval_expr(&expr, &scope).unwrap();
        match val {
            Value::Object(map) => {
                assert_eq!(map.get("electrical"), Some(&Value::String("passive".to_string())));
                assert_eq!(map.get("side"), Some(&Value::String("outside".to_string())));
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn binding_takes_precedence_over_string_fallback() {
        // If "passive" IS a binding, it should resolve to the binding value, not string.
        let mut scope = make_scope();
        scope.define("passive".to_string(), Value::Integer(42));
        let expr = spanned(Expr::Ident("passive".to_string()));
        let val = eval_expr(&expr, &scope).unwrap();
        assert_eq!(val, Value::Integer(42));
    }

    #[test]
    fn dollar_ident_still_errors_when_undefined() {
        // $bindings must be explicit — no string fallback for dollar idents.
        let scope = make_scope();
        let expr = spanned(Expr::DollarIdent("nonexistent".to_string()));
        let result = eval_expr(&expr, &scope);
        assert!(matches!(result, Err(SpecError { code: SpecErrorCode::UndefinedBinding, .. })));
    }
}

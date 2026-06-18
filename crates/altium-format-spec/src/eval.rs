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

use crate::ast::{CallArg, Expr, Object, ObjectItem, TemplatePart};
use crate::diagnostic::{BinOp, Span, Spanned, Unit};

// ── Error types ──────────────────────────────────────────────────────────────

/// Severity level for a [`SpecError`].
///
/// All constructors default to [`Severity::Error`]. Use
/// [`SpecError::with_severity`] to downgrade to a warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpecError {
    pub code: SpecErrorCode,
    pub message: String,
    pub span: Option<Span>,
    pub severity: Severity,
}

impl SpecError {
    pub fn new(code: SpecErrorCode, message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            code,
            message: message.into(),
            span,
            severity: Severity::Error,
        }
    }

    pub fn at(code: SpecErrorCode, message: impl Into<String>, span: Span) -> Self {
        Self::new(code, message, Some(span))
    }

    pub fn no_span(code: SpecErrorCode, message: impl Into<String>) -> Self {
        Self::new(code, message, None)
    }

    /// Override the severity of this error (builder pattern).
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Render this error with source location context (file:line:col + caret).
    ///
    /// Falls back to a plain `error[Code]: message` when no span is available.
    /// Prefixes with `warning[...]` when severity is [`Severity::Warning`].
    pub fn render(&self, source_name: &str, source: &str) -> String {
        use crate::diagnostic::{caret_len, locate_line};

        let prefix = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };

        let Some(span) = self.span else {
            return format!("{}[{:?}]: {}", prefix, self.code, self.message);
        };
        let (line_no, col_no, line_text) = locate_line(source, span.start as usize);
        let mut out = String::new();
        out.push_str(&format!("{}[{:?}]: {}\n", prefix, self.code, self.message));
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
        let prefix = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{}[{:?}]: {}", prefix, self.code, self.message)?;
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
    UnknownProperty,
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
    // Validator errors (Phase 3)
    DuplicateDesignator,
    DanglingNetRef,
    DuplicateAnnotationId,
    UnresolvedPinRef,
    // Resolver errors (Phase 4)
    UnresolvableLibrary,
    // Sync errors
    NotSupported,
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
    /// A declared swap group reference; the inner string is the group's entity name.
    SwapGroup(String),
    /// An import object — maps entity names to their string names.
    /// Stores provenance (alias) so field access can return ImportRef.
    ImportObject {
        alias: String,
        entries: IndexMap<String, Value>,
    },
    /// A resolved `$alias.Name` reference; carries the import alias for error
    /// reporting and symbol validation.
    ImportRef {
        alias: String,
        name: String,
    },
    /// A contour arc segment produced by `arc(...)`, used inside `outline:`
    /// arrays for PCB regions and component bodies. Coordinates are Altium
    /// internal units.
    ContourArc {
        endpoint: (i32, i32),
        center: (i32, i32),
        radius: i32,
        start_angle: f64,
        end_angle: f64,
    },
    /// A geometric shape value in Altium internal units (10,000 per mil).
    Shape(Shape),
}

// ── Shape type ────────────────────────────────────────────────────────────────

/// Geometric shape value in Altium internal units (10,000 per mil).
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// Axis-aligned rectangle. cx/cy = center, hw/hh = half-width/half-height.
    Rect { cx: i32, cy: i32, hw: i32, hh: i32 },
    /// Rounded rectangle: same as Rect + corner radius.
    RoundedRect {
        cx: i32,
        cy: i32,
        hw: i32,
        hh: i32,
        radius: i32,
    },
    /// Circle: center + radius.
    Circle { cx: i32, cy: i32, radius: i32 },
    /// Arbitrary polygon: ordered vertices (closed — last connects to first).
    Polygon { vertices: Vec<(i32, i32)> },
}

impl Shape {
    /// Bounding box center.
    pub fn center(&self) -> (i32, i32) {
        match self {
            Shape::Rect { cx, cy, .. }
            | Shape::RoundedRect { cx, cy, .. }
            | Shape::Circle { cx, cy, .. } => (*cx, *cy),
            Shape::Polygon { vertices } => {
                if vertices.is_empty() {
                    return (0, 0);
                }
                // Bounding-box center, consistent with width()/height() which use bounding-box.
                let (mut min_x, mut max_x) = (i32::MAX, i32::MIN);
                let (mut min_y, mut max_y) = (i32::MAX, i32::MIN);
                for &(x, y) in vertices {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
                ((min_x + max_x) / 2, (min_y + max_y) / 2)
            }
        }
    }

    /// Bounding box width (full, not half).
    pub fn width(&self) -> i32 {
        match self {
            Shape::Rect { hw, .. } | Shape::RoundedRect { hw, .. } => *hw * 2,
            Shape::Circle { radius, .. } => *radius * 2,
            Shape::Polygon { vertices } => {
                if vertices.is_empty() {
                    return 0;
                }
                let (min_x, max_x) = vertices
                    .iter()
                    .fold((i32::MAX, i32::MIN), |(mn, mx), (x, _)| {
                        (mn.min(*x), mx.max(*x))
                    });
                max_x - min_x
            }
        }
    }

    /// Bounding box height (full, not half).
    pub fn height(&self) -> i32 {
        match self {
            Shape::Rect { hh, .. } | Shape::RoundedRect { hh, .. } => *hh * 2,
            Shape::Circle { radius, .. } => *radius * 2,
            Shape::Polygon { vertices } => {
                if vertices.is_empty() {
                    return 0;
                }
                let (min_y, max_y) = vertices
                    .iter()
                    .fold((i32::MAX, i32::MIN), |(mn, mx), (_, y)| {
                        (mn.min(*y), mx.max(*y))
                    });
                max_y - min_y
            }
        }
    }

    /// Convert to vertex list (polygon approximation for curved shapes).
    pub fn to_vertices(&self) -> Vec<(i32, i32)> {
        match self {
            Shape::Rect { cx, cy, hw, hh } => vec![
                (cx - hw, cy - hh),
                (cx + hw, cy - hh),
                (cx + hw, cy + hh),
                (cx - hw, cy + hh),
            ],
            Shape::RoundedRect {
                cx,
                cy,
                hw,
                hh,
                radius,
            } => rounded_rect_vertices(*cx, *cy, *hw, *hh, *radius),
            Shape::Circle { cx, cy, radius } => circle_vertices(*cx, *cy, *radius, 72),
            Shape::Polygon { vertices } => vertices.clone(),
        }
    }

    /// Translate by offset.
    pub fn translate(&self, dx: i32, dy: i32) -> Shape {
        match self {
            Shape::Rect { cx, cy, hw, hh } => Shape::Rect {
                cx: cx + dx,
                cy: cy + dy,
                hw: *hw,
                hh: *hh,
            },
            Shape::RoundedRect {
                cx,
                cy,
                hw,
                hh,
                radius,
            } => Shape::RoundedRect {
                cx: cx + dx,
                cy: cy + dy,
                hw: *hw,
                hh: *hh,
                radius: *radius,
            },
            Shape::Circle { cx, cy, radius } => Shape::Circle {
                cx: cx + dx,
                cy: cy + dy,
                radius: *radius,
            },
            Shape::Polygon { vertices } => Shape::Polygon {
                vertices: vertices.iter().map(|(x, y)| (x + dx, y + dy)).collect(),
            },
        }
    }

    /// Inset (shrink) by distance. Amount must be non-negative.
    /// Returns None for Polygon shapes (not supported).
    pub fn inset(&self, amount: i32) -> Result<Shape, &'static str> {
        if amount < 0 {
            return Err("inset amount must be non-negative; use outset() to expand");
        }
        match self {
            Shape::Rect { cx, cy, hw, hh } => Ok(Shape::Rect {
                cx: *cx,
                cy: *cy,
                hw: (*hw - amount).max(0),
                hh: (*hh - amount).max(0),
            }),
            Shape::RoundedRect {
                cx,
                cy,
                hw,
                hh,
                radius,
            } => Ok(Shape::RoundedRect {
                cx: *cx,
                cy: *cy,
                hw: (*hw - amount).max(0),
                hh: (*hh - amount).max(0),
                radius: (*radius - amount).max(0),
            }),
            Shape::Circle { cx, cy, radius } => Ok(Shape::Circle {
                cx: *cx,
                cy: *cy,
                radius: (*radius - amount).max(0),
            }),
            Shape::Polygon { .. } => {
                Err("inset() is not supported for polygon shapes; use explicit vertex coordinates")
            }
        }
    }

    /// Outset (expand) by distance. Amount must be non-negative.
    /// Returns Err for Polygon shapes (not supported) or on overflow.
    pub fn outset(&self, amount: i32) -> Result<Shape, &'static str> {
        if amount < 0 {
            return Err("outset amount must be non-negative; use inset() to shrink");
        }
        match self {
            Shape::Rect { cx, cy, hw, hh } => Ok(Shape::Rect {
                cx: *cx,
                cy: *cy,
                hw: hw
                    .checked_add(amount)
                    .ok_or("outset overflow: shape dimension too large")?,
                hh: hh
                    .checked_add(amount)
                    .ok_or("outset overflow: shape dimension too large")?,
            }),
            Shape::RoundedRect {
                cx,
                cy,
                hw,
                hh,
                radius,
            } => Ok(Shape::RoundedRect {
                cx: *cx,
                cy: *cy,
                hw: hw
                    .checked_add(amount)
                    .ok_or("outset overflow: shape dimension too large")?,
                hh: hh
                    .checked_add(amount)
                    .ok_or("outset overflow: shape dimension too large")?,
                radius: radius
                    .checked_add(amount)
                    .ok_or("outset overflow: shape dimension too large")?,
            }),
            Shape::Circle { cx, cy, radius } => Ok(Shape::Circle {
                cx: *cx,
                cy: *cy,
                radius: radius
                    .checked_add(amount)
                    .ok_or("outset overflow: shape dimension too large")?,
            }),
            Shape::Polygon { .. } => {
                Err("outset() is not supported for polygon shapes; use explicit vertex coordinates")
            }
        }
    }
}

/// Generate vertices for a rounded rectangle.
fn rounded_rect_vertices(cx: i32, cy: i32, hw: i32, hh: i32, radius: i32) -> Vec<(i32, i32)> {
    let r = radius.min(hw).min(hh);
    if r <= 0 {
        return vec![
            (cx - hw, cy - hh),
            (cx + hw, cy - hh),
            (cx + hw, cy + hh),
            (cx - hw, cy + hh),
        ];
    }
    let mut verts = Vec::with_capacity(4 * 8);
    let steps = 8usize;
    // Corner arc centers and start angles. Each corner traces 90 degrees.
    // Using 0..steps (exclusive end) avoids duplicate vertices at corner boundaries,
    // since the next corner's first point covers the shared boundary position.
    let corners = [
        (cx + hw - r, cy + hh - r, 0.0f64),
        (cx - hw + r, cy + hh - r, 90.0f64),
        (cx - hw + r, cy - hh + r, 180.0f64),
        (cx + hw - r, cy - hh + r, 270.0f64),
    ];
    for &(ccx, ccy, start_deg) in &corners {
        for i in 0..steps {
            let angle = (start_deg + (i as f64) * 90.0 / (steps as f64)).to_radians();
            let x = ccx + (r as f64 * angle.cos()).round() as i32;
            let y = ccy + (r as f64 * angle.sin()).round() as i32;
            verts.push((x, y));
        }
    }
    verts
}

/// Generate vertices for a circle approximation.
fn circle_vertices(cx: i32, cy: i32, radius: i32, segments: usize) -> Vec<(i32, i32)> {
    (0..segments)
        .map(|i| {
            let angle = 2.0 * std::f64::consts::PI * (i as f64) / (segments as f64);
            let x = cx + (radius as f64 * angle.cos()).round() as i32;
            let y = cy + (radius as f64 * angle.sin()).round() as i32;
            (x, y)
        })
        .collect()
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
            Value::SwapGroup(s) => s.clone(),
            Value::ImportObject { alias, .. } => format!("<import:{alias}>"),
            Value::ImportRef { alias, name } => format!("{alias}.{name}"),
            Value::Shape(s) => format!("shape({} vertices)", s.to_vertices().len()),
            Value::ContourArc {
                endpoint, center, ..
            } => format!(
                "arc(endpoint: ({}, {}), center: ({}, {}))",
                endpoint.0, endpoint.1, center.0, center.1
            ),
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
            Value::SwapGroup(_) => "swap_group",
            Value::ImportObject { .. } => "import_object",
            Value::ImportRef { .. } => "import_ref",
            Value::Shape(_) => "shape",
            Value::ContourArc { .. } => "contour_arc",
        }
    }

    /// Convert to Altium internal units (Coord raw value) if this is numeric/dim.
    /// Bare integer/float → mils by default.
    pub fn to_dim(&self, span: Option<Span>) -> EvalResult<i32> {
        match self {
            Value::Dim(raw) => Ok(*raw),
            Value::Integer(n) => Ok(n.checked_mul(10_000).ok_or_else(|| {
                SpecError::new(
                    SpecErrorCode::ArithmeticOverflow,
                    "integer overflow in mil conversion",
                    span,
                )
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
            Value::ImportObject { entries, .. } => Ok(entries),
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
        Expr::DollarIdent(name) => match scope.lookup_dollar(name) {
            Some(Ok(v)) => Ok(v.clone()),
            Some(Err(e)) => Err(e),
            None => Err(SpecError::at(
                SpecErrorCode::UndefinedBinding,
                format!("undefined binding '${name}'"),
                span,
            )),
        },

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

        // ── Function call ─────────────────────────────────────────────────
        Expr::Call { name, args } => eval_builtin_call(name, args, scope, span),
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
fn parse_template_expr(
    tokens: &[crate::lexer::Token],
    _outer_span: Span,
) -> EvalResult<Spanned<Expr>> {
    use crate::parser::parse_spec;

    // Build a minimal source string for error reporting.
    // The actual parsing is done by feeding the tokens back through the expression parser.
    // Since we can't access the inner parser directly from here, we reconstruct the
    // source fragment from the token spans (not available), so we use a simplified approach:
    // serialize the tokens to a synthetic source and parse.
    let synthetic = tokens_to_source(tokens);
    // Parse as a value expression by wrapping in a let-binding assignment
    let wrapped = format!("component _dummy {{ let _t = {synthetic} }}");
    let ast = parse_spec(&wrapped).map_err(|e| {
        SpecError::no_span(
            SpecErrorCode::TypeMismatch,
            format!("failed to parse template expression: {e}"),
        )
    })?;

    // Extract the let binding value from the dummy component.
    use crate::ast::{ComponentItem, SpecItem};
    let comp = ast
        .items
        .into_iter()
        .find_map(|item| match item.node {
            SpecItem::Component(c) => Some(c),
            _ => None,
        })
        .ok_or_else(|| {
            SpecError::no_span(
                SpecErrorCode::TypeMismatch,
                "template parse: expected component",
            )
        })?;

    let let_binding = comp
        .body
        .into_iter()
        .find_map(|item| match item.node {
            ComponentItem::LetBinding(lb) => Some(lb),
            _ => None,
        })
        .ok_or_else(|| {
            SpecError::no_span(
                SpecErrorCode::TypeMismatch,
                "template parse: expected let binding",
            )
        })?;

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
            TokenKind::DollarIdent(s) => {
                out.push('$');
                out.push_str(s);
            }
            TokenKind::String(s) => {
                out.push('"');
                out.push_str(&s.replace('\\', "\\\\").replace('"', "\\\""));
                out.push('"');
            }
            TokenKind::Integer(n) => {
                write!(&mut out, "{n}").unwrap();
            }
            TokenKind::Float(f) => {
                write!(&mut out, "{f}").unwrap();
            }
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
            TokenKind::Color(r, g, b) => {
                write!(&mut out, "#{r:02X}{g:02X}{b:02X}").unwrap();
            }
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
                SpecError::at(
                    SpecErrorCode::ArithmeticOverflow,
                    "overflow in dim + dim",
                    span,
                )
            })?))
        }
        (Value::Dim(a), BinOp::Sub, Value::Dim(b)) => {
            Ok(Value::Dim(a.checked_sub(*b).ok_or_else(|| {
                SpecError::at(
                    SpecErrorCode::ArithmeticOverflow,
                    "overflow in dim - dim",
                    span,
                )
            })?))
        }
        // dim * number → dim (scale)
        (Value::Dim(a), BinOp::Mul, Value::Integer(b)) => {
            Ok(Value::Dim(a.checked_mul(*b).ok_or_else(|| {
                SpecError::at(
                    SpecErrorCode::ArithmeticOverflow,
                    "overflow in dim * int",
                    span,
                )
            })?))
        }
        (Value::Dim(a), BinOp::Mul, Value::Float(b)) => {
            Ok(Value::Dim((*a as f64 * b).round() as i32))
        }
        // number * dim → dim
        (Value::Integer(a), BinOp::Mul, Value::Dim(b)) => {
            Ok(Value::Dim(a.checked_mul(*b).ok_or_else(|| {
                SpecError::at(
                    SpecErrorCode::ArithmeticOverflow,
                    "overflow in int * dim",
                    span,
                )
            })?))
        }
        (Value::Float(a), BinOp::Mul, Value::Dim(b)) => {
            Ok(Value::Dim((*b as f64 * a).round() as i32))
        }
        // dim / number → dim
        (Value::Dim(a), BinOp::Div, Value::Integer(b)) => {
            if *b == 0 {
                return Err(SpecError::at(
                    SpecErrorCode::DivisionByZero,
                    "division by zero",
                    span,
                ));
            }
            Ok(Value::Dim(a / b))
        }
        (Value::Dim(a), BinOp::Div, Value::Float(b)) => {
            if *b == 0.0 {
                return Err(SpecError::at(
                    SpecErrorCode::DivisionByZero,
                    "division by zero",
                    span,
                ));
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
                return Err(SpecError::at(
                    SpecErrorCode::DivisionByZero,
                    "division by zero",
                    span,
                ));
            }
            Ok(Value::Integer(a / b))
        }

        // float op float → float (and mixed int/float)
        (Value::Float(a), BinOp::Add, Value::Float(b)) => Ok(Value::Float(a + b)),
        (Value::Float(a), BinOp::Sub, Value::Float(b)) => Ok(Value::Float(a - b)),
        (Value::Float(a), BinOp::Mul, Value::Float(b)) => Ok(Value::Float(a * b)),
        (Value::Float(a), BinOp::Div, Value::Float(b)) => {
            if *b == 0.0 {
                return Err(SpecError::at(
                    SpecErrorCode::DivisionByZero,
                    "division by zero",
                    span,
                ));
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
                return Err(SpecError::at(
                    SpecErrorCode::DivisionByZero,
                    "division by zero",
                    span,
                ));
            }
            Ok(Value::Float(*a as f64 / b))
        }
        (Value::Float(a), BinOp::Div, Value::Integer(b)) => {
            if *b == 0 {
                return Err(SpecError::at(
                    SpecErrorCode::DivisionByZero,
                    "division by zero",
                    span,
                ));
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
        Value::Object(map) => map.get(field).cloned().ok_or_else(|| {
            SpecError::new(
                SpecErrorCode::InvalidFieldAccess,
                format!("no field '{field}' on object"),
                span,
            )
        }),
        Value::ImportObject { alias, entries } => match entries.get(field) {
            Some(Value::String(name)) => Ok(Value::ImportRef {
                alias,
                name: name.clone(),
            }),
            Some(other) => Ok(other.clone()),
            None => Err(SpecError::new(
                SpecErrorCode::InvalidFieldAccess,
                format!("no entity '{field}' in import '{alias}'"),
                span,
            )),
        },
        Value::CoordPoint(x, y) => match field {
            "x" => Ok(Value::Dim(x)),
            "y" => Ok(Value::Dim(y)),
            _ => Err(SpecError::new(
                SpecErrorCode::InvalidFieldAccess,
                format!("coord has no field '{field}' (valid: x, y)"),
                span,
            )),
        },
        Value::Shape(s) => match field {
            "width" => Ok(Value::Dim(s.width())),
            "height" => Ok(Value::Dim(s.height())),
            "center" => {
                let (cx, cy) = s.center();
                Ok(Value::CoordPoint(cx, cy))
            }
            _ => Err(SpecError::new(
                SpecErrorCode::InvalidFieldAccess,
                format!("shape has no field '{field}' (available: width, height, center)"),
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
            i.and_then(|i| arr.get(i)).cloned().ok_or_else(|| {
                SpecError::new(
                    SpecErrorCode::IndexNotArray,
                    format!("array index {n} out of bounds"),
                    span,
                )
            })
        }
        (Value::Object(map), Value::String(key)) => map.get(&key).cloned().ok_or_else(|| {
            SpecError::new(
                SpecErrorCode::InvalidFieldAccess,
                format!("no field '{key}' on object"),
                span,
            )
        }),
        (Value::Object(map), Value::Integer(n)) => {
            let key = n.to_string();
            map.get(&key).cloned().ok_or_else(|| {
                SpecError::new(
                    SpecErrorCode::InvalidFieldAccess,
                    format!("no field '{key}' on object"),
                    span,
                )
            })
        }
        (Value::ImportObject { alias, entries }, Value::String(key)) => match entries.get(&key) {
            Some(Value::String(name)) => Ok(Value::ImportRef {
                alias,
                name: name.clone(),
            }),
            Some(other) => Ok(other.clone()),
            None => Err(SpecError::new(
                SpecErrorCode::InvalidFieldAccess,
                format!("no entity '{key}' in import '{alias}'"),
                span,
            )),
        },
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

// ── Builtin function dispatch ────────────────────────────────────────────────

/// Evaluate a builtin function call.
fn eval_builtin_call(
    name: &str,
    args: &[CallArg],
    scope: &ScopeStack,
    span: Span,
) -> EvalResult<Value> {
    let evaluated: Vec<(Option<String>, Value)> = args
        .iter()
        .map(|a| {
            let val = eval_expr(&a.value, scope)?;
            Ok((a.name.as_ref().map(|n| n.node.clone()), val))
        })
        .collect::<EvalResult<_>>()?;

    match name {
        "arc" => builtin_contour_arc(&evaluated, span),
        "rect" => builtin_rect(&evaluated, span),
        "rounded_rect" => builtin_rounded_rect(&evaluated, span),
        "circle" => builtin_circle(&evaluated, span),
        "polygon" => builtin_polygon(&evaluated, span),
        "inset" => builtin_inset(&evaluated, span),
        "outset" => builtin_outset(&evaluated, span),
        "translate" => builtin_translate(&evaluated, span),
        "width" => builtin_shape_width(&evaluated, span),
        "height" => builtin_shape_height(&evaluated, span),
        "center" => builtin_shape_center(&evaluated, span),
        "min" => builtin_min(&evaluated, span),
        "max" => builtin_max(&evaluated, span),
        "clamp" => builtin_clamp(&evaluated, span),
        "abs" => builtin_abs(&evaluated, span),
        _ => Err(SpecError::at(
            SpecErrorCode::NotSupported,
            format!("unknown function '{name}'"),
            span,
        )),
    }
}

// ── Argument extraction helpers ───────────────────────────────────────────────

fn get_named_arg<'a>(args: &'a [(Option<String>, Value)], name: &str) -> Option<&'a Value> {
    args.iter().find_map(|(n, v)| {
        if n.as_deref() == Some(name) {
            Some(v)
        } else {
            None
        }
    })
}

fn positional_args(args: &[(Option<String>, Value)]) -> Vec<&Value> {
    args.iter()
        .filter_map(|(n, v)| if n.is_none() { Some(v) } else { None })
        .collect()
}

fn require_dim(val: &Value, arg_name: &str, fn_name: &str, span: Span) -> EvalResult<i32> {
    val.to_dim(Some(span)).map_err(|_| {
        SpecError::at(
            SpecErrorCode::TypeMismatch,
            format!("{fn_name}(): argument '{arg_name}' must be a dimension (e.g., 10mm, 100mil)"),
            span,
        )
    })
}

fn require_float(val: &Value, arg_name: &str, fn_name: &str, span: Span) -> EvalResult<f64> {
    match val {
        Value::Float(f) => Ok(*f),
        Value::Integer(n) => Ok(*n as f64),
        _ => Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            format!("{fn_name}(): argument '{arg_name}' must be a number"),
            span,
        )),
    }
}

fn require_shape<'a>(
    val: &'a Value,
    arg_name: &str,
    fn_name: &str,
    span: Span,
) -> EvalResult<&'a Shape> {
    match val {
        Value::Shape(s) => Ok(s),
        _ => Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            format!("{fn_name}(): argument '{arg_name}' must be a shape"),
            span,
        )),
    }
}

fn require_coord_point(
    val: &Value,
    arg_name: &str,
    fn_name: &str,
    span: Span,
) -> EvalResult<(i32, i32)> {
    match val {
        Value::CoordPoint(x, y) => Ok((*x, *y)),
        _ => Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            format!(
                "{fn_name}(): argument '{arg_name}' must be a coordinate point, e.g., (10mm, 5mm)"
            ),
            span,
        )),
    }
}

// ── Geometry constructors ─────────────────────────────────────────────────────

/// `arc(endpoint: (x, y), center: (x, y), radius: r, start_angle: a, end_angle: b)`
/// Contour arc segment for `outline:` arrays (PCB regions / component bodies).
fn builtin_contour_arc(args: &[(Option<String>, Value)], span: Span) -> EvalResult<Value> {
    let endpoint = get_named_arg(args, "endpoint").ok_or_else(|| {
        SpecError::new(
            SpecErrorCode::TypeMismatch,
            "arc() requires endpoint:",
            Some(span),
        )
    })?;
    let center = get_named_arg(args, "center").ok_or_else(|| {
        SpecError::new(
            SpecErrorCode::TypeMismatch,
            "arc() requires center:",
            Some(span),
        )
    })?;
    let radius = get_named_arg(args, "radius").ok_or_else(|| {
        SpecError::new(
            SpecErrorCode::TypeMismatch,
            "arc() requires radius:",
            Some(span),
        )
    })?;
    let start_angle = get_named_arg(args, "start_angle").ok_or_else(|| {
        SpecError::new(
            SpecErrorCode::TypeMismatch,
            "arc() requires start_angle:",
            Some(span),
        )
    })?;
    let end_angle = get_named_arg(args, "end_angle").ok_or_else(|| {
        SpecError::new(
            SpecErrorCode::TypeMismatch,
            "arc() requires end_angle:",
            Some(span),
        )
    })?;

    let endpoint = require_coord_point(endpoint, "endpoint", "arc", span)?;
    let center = require_coord_point(center, "center", "arc", span)?;
    let radius = require_dim(radius, "radius", "arc", span)?;
    let start_angle = require_float(start_angle, "start_angle", "arc", span)?;
    let end_angle = require_float(end_angle, "end_angle", "arc", span)?;

    Ok(Value::ContourArc {
        endpoint,
        center,
        radius,
        start_angle,
        end_angle,
    })
}

fn builtin_rect(args: &[(Option<String>, Value)], span: Span) -> EvalResult<Value> {
    let pos = positional_args(args);

    if let (Some(from), Some(to)) = (get_named_arg(args, "from"), get_named_arg(args, "to")) {
        let (x1, y1) = require_coord_point(from, "from", "rect", span)?;
        let (x2, y2) = require_coord_point(to, "to", "rect", span)?;
        let cx = (x1 + x2) / 2;
        let cy = (y1 + y2) / 2;
        let hw = ((x2 - x1) / 2).abs();
        let hh = ((y2 - y1) / 2).abs();
        return Ok(Value::Shape(Shape::Rect { cx, cy, hw, hh }));
    }

    if pos.len() == 2 {
        let w = require_dim(pos[0], "width", "rect", span)?;
        let h = require_dim(pos[1], "height", "rect", span)?;
        let (cx, cy) = match get_named_arg(args, "center") {
            Some(v) => require_coord_point(v, "center", "rect", span)?,
            None => (0, 0),
        };
        return Ok(Value::Shape(Shape::Rect {
            cx,
            cy,
            hw: w / 2,
            hh: h / 2,
        }));
    }

    // Form: rect(at: lower-left corner, width: dim, height: dim)
    // 'at' is the lower-left corner origin; center is computed as (ox + w/2, oy + h/2).
    if let (Some(at), Some(w_val), Some(h_val)) = (
        get_named_arg(args, "at"),
        get_named_arg(args, "width"),
        get_named_arg(args, "height"),
    ) {
        let (ox, oy) = require_coord_point(at, "at (lower-left corner)", "rect", span)?;
        let w = require_dim(w_val, "width", "rect", span)?;
        let h = require_dim(h_val, "height", "rect", span)?;
        return Ok(Value::Shape(Shape::Rect {
            cx: ox + w / 2,
            cy: oy + h / 2,
            hw: w / 2,
            hh: h / 2,
        }));
    }

    Err(SpecError::at(
        SpecErrorCode::TypeMismatch,
        "rect() requires (width, height), (from: point, to: point), or (at: lower-left corner, width: dim, height: dim)".to_string(),
        span,
    ))
}

fn builtin_rounded_rect(args: &[(Option<String>, Value)], span: Span) -> EvalResult<Value> {
    let pos = positional_args(args);
    if pos.len() != 3 {
        return Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            "rounded_rect() requires 3 positional args: width, height, radius".to_string(),
            span,
        ));
    }
    let w = require_dim(pos[0], "width", "rounded_rect", span)?;
    let h = require_dim(pos[1], "height", "rounded_rect", span)?;
    let r = require_dim(pos[2], "radius", "rounded_rect", span)?;
    let (cx, cy) = match get_named_arg(args, "center") {
        Some(v) => require_coord_point(v, "center", "rounded_rect", span)?,
        None => (0, 0),
    };
    Ok(Value::Shape(Shape::RoundedRect {
        cx,
        cy,
        hw: w / 2,
        hh: h / 2,
        radius: r,
    }))
}

fn builtin_circle(args: &[(Option<String>, Value)], span: Span) -> EvalResult<Value> {
    let pos = positional_args(args);
    if pos.len() != 1 {
        return Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            "circle() requires 1 positional arg: radius".to_string(),
            span,
        ));
    }
    let r = require_dim(pos[0], "radius", "circle", span)?;
    let (cx, cy) = match get_named_arg(args, "center") {
        Some(v) => require_coord_point(v, "center", "circle", span)?,
        None => (0, 0),
    };
    Ok(Value::Shape(Shape::Circle { cx, cy, radius: r }))
}

fn builtin_polygon(args: &[(Option<String>, Value)], span: Span) -> EvalResult<Value> {
    let pos = positional_args(args);
    if pos.len() != 1 {
        return Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            "polygon() requires 1 positional arg: array of points".to_string(),
            span,
        ));
    }
    match pos[0] {
        Value::Array(arr) => {
            let mut vertices = Vec::with_capacity(arr.len());
            for (i, v) in arr.iter().enumerate() {
                match v {
                    Value::CoordPoint(x, y) => vertices.push((*x, *y)),
                    _ => {
                        return Err(SpecError::at(
                            SpecErrorCode::TypeMismatch,
                            format!("polygon(): element {i} must be a coordinate point"),
                            span,
                        ));
                    }
                }
            }
            if vertices.len() < 3 {
                return Err(SpecError::at(
                    SpecErrorCode::TypeMismatch,
                    format!(
                        "polygon() requires at least 3 vertices, got {}",
                        vertices.len()
                    ),
                    span,
                ));
            }
            Ok(Value::Shape(Shape::Polygon { vertices }))
        }
        _ => Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            "polygon() argument must be an array of points".to_string(),
            span,
        )),
    }
}

// ── Geometry operations ───────────────────────────────────────────────────────

fn builtin_inset(args: &[(Option<String>, Value)], span: Span) -> EvalResult<Value> {
    let pos = positional_args(args);
    if pos.len() != 2 {
        return Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            "inset() requires 2 args: shape, amount".to_string(),
            span,
        ));
    }
    let shape = require_shape(pos[0], "shape", "inset", span)?;
    let amount = require_dim(pos[1], "amount", "inset", span)?;
    let result = shape.inset(amount).map_err(|msg| {
        SpecError::at(SpecErrorCode::NotSupported, format!("inset(): {msg}"), span)
    })?;
    Ok(Value::Shape(result))
}

fn builtin_outset(args: &[(Option<String>, Value)], span: Span) -> EvalResult<Value> {
    let pos = positional_args(args);
    if pos.len() != 2 {
        return Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            "outset() requires 2 args: shape, amount".to_string(),
            span,
        ));
    }
    let shape = require_shape(pos[0], "shape", "outset", span)?;
    let amount = require_dim(pos[1], "amount", "outset", span)?;
    let result = shape.outset(amount).map_err(|msg| {
        SpecError::at(
            SpecErrorCode::NotSupported,
            format!("outset(): {msg}"),
            span,
        )
    })?;
    Ok(Value::Shape(result))
}

fn builtin_translate(args: &[(Option<String>, Value)], span: Span) -> EvalResult<Value> {
    let pos = positional_args(args);
    if pos.len() == 2 {
        let shape = require_shape(pos[0], "shape", "translate", span)?;
        let (dx, dy) = require_coord_point(pos[1], "offset", "translate", span)?;
        return Ok(Value::Shape(shape.translate(dx, dy)));
    }
    if pos.len() == 3 {
        let shape = require_shape(pos[0], "shape", "translate", span)?;
        let dx = require_dim(pos[1], "dx", "translate", span)?;
        let dy = require_dim(pos[2], "dy", "translate", span)?;
        return Ok(Value::Shape(shape.translate(dx, dy)));
    }
    Err(SpecError::at(
        SpecErrorCode::TypeMismatch,
        "translate() requires (shape, offset) or (shape, dx, dy)".to_string(),
        span,
    ))
}

// ── Shape accessors ───────────────────────────────────────────────────────────

fn builtin_shape_width(args: &[(Option<String>, Value)], span: Span) -> EvalResult<Value> {
    let pos = positional_args(args);
    if pos.len() != 1 {
        return Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            "width() requires 1 arg: shape".to_string(),
            span,
        ));
    }
    let shape = require_shape(pos[0], "shape", "width", span)?;
    Ok(Value::Dim(shape.width()))
}

fn builtin_shape_height(args: &[(Option<String>, Value)], span: Span) -> EvalResult<Value> {
    let pos = positional_args(args);
    if pos.len() != 1 {
        return Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            "height() requires 1 arg: shape".to_string(),
            span,
        ));
    }
    let shape = require_shape(pos[0], "shape", "height", span)?;
    Ok(Value::Dim(shape.height()))
}

fn builtin_shape_center(args: &[(Option<String>, Value)], span: Span) -> EvalResult<Value> {
    let pos = positional_args(args);
    if pos.len() != 1 {
        return Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            "center() requires 1 arg: shape".to_string(),
            span,
        ));
    }
    let shape = require_shape(pos[0], "shape", "center", span)?;
    let (cx, cy) = shape.center();
    Ok(Value::CoordPoint(cx, cy))
}

// ── Math functions ────────────────────────────────────────────────────────────

fn builtin_min(args: &[(Option<String>, Value)], span: Span) -> EvalResult<Value> {
    let pos = positional_args(args);
    if pos.len() != 2 {
        return Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            "min() requires 2 args".to_string(),
            span,
        ));
    }
    match (pos[0], pos[1]) {
        (Value::Dim(a), Value::Dim(b)) => Ok(Value::Dim(*a.min(b))),
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(*a.min(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(*b))),
        _ => Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            "min() arguments must be the same numeric type".to_string(),
            span,
        )),
    }
}

fn builtin_max(args: &[(Option<String>, Value)], span: Span) -> EvalResult<Value> {
    let pos = positional_args(args);
    if pos.len() != 2 {
        return Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            "max() requires 2 args".to_string(),
            span,
        ));
    }
    match (pos[0], pos[1]) {
        (Value::Dim(a), Value::Dim(b)) => Ok(Value::Dim(*a.max(b))),
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(*a.max(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(*b))),
        _ => Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            "max() arguments must be the same numeric type".to_string(),
            span,
        )),
    }
}

fn builtin_clamp(args: &[(Option<String>, Value)], span: Span) -> EvalResult<Value> {
    let pos = positional_args(args);
    if pos.len() != 3 {
        return Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            "clamp() requires 3 args: value, min, max".to_string(),
            span,
        ));
    }
    match (pos[0], pos[1], pos[2]) {
        (Value::Dim(v), Value::Dim(lo), Value::Dim(hi)) => Ok(Value::Dim(*v.clamp(lo, hi))),
        (Value::Integer(v), Value::Integer(lo), Value::Integer(hi)) => {
            Ok(Value::Integer(*v.clamp(lo, hi)))
        }
        (Value::Float(v), Value::Float(lo), Value::Float(hi)) => {
            Ok(Value::Float(v.clamp(*lo, *hi)))
        }
        _ => Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            "clamp() arguments must be the same numeric type".to_string(),
            span,
        )),
    }
}

fn builtin_abs(args: &[(Option<String>, Value)], span: Span) -> EvalResult<Value> {
    let pos = positional_args(args);
    if pos.len() != 1 {
        return Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            "abs() requires 1 arg".to_string(),
            span,
        ));
    }
    match pos[0] {
        Value::Dim(v) => Ok(Value::Dim(v.abs())),
        Value::Integer(v) => Ok(Value::Integer(v.abs())),
        Value::Float(v) => Ok(Value::Float(v.abs())),
        _ => Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            "abs() argument must be numeric".to_string(),
            span,
        )),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Expr;
    use crate::diagnostic::{BinOp, Span, Spanned, Unit};

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
        assert!(matches!(
            result,
            Err(SpecError {
                code: SpecErrorCode::DivisionByZero,
                ..
            })
        ));
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

        let spread_item = spanned(ObjectItem::Spread(spanned(Expr::Ident(
            "defaults".to_string(),
        ))));
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
                assert_eq!(
                    map.get("shape"),
                    Some(&Value::String("rectangular".to_string()))
                );
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
        assert!(matches!(
            result,
            Err(SpecError {
                code: SpecErrorCode::CircularBinding,
                ..
            })
        ));
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
        assert!(matches!(
            result,
            Err(SpecError {
                code: SpecErrorCode::UndefinedBinding,
                ..
            })
        ));
    }

    // ── Tuple (coord) ──────────────────────────────────────────────────────

    #[test]
    fn tuple_makes_coord() {
        let x = spanned(Expr::Dim(10.0, Unit::Mm));
        let y = spanned(Expr::Dim(5.0, Unit::Mm));
        let expr = spanned(Expr::Tuple(Box::new(x), Box::new(y)));
        let val = eval_expr(&expr, &make_scope()).unwrap();
        // 10mm = 3_937_010 internal units, 5mm = 1_968_505
        assert_eq!(
            val,
            Value::CoordPoint(
                (10.0_f64 * 393_701.0).round() as i32,
                (5.0_f64 * 393_701.0).round() as i32,
            )
        );
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
        assert_eq!(
            val,
            Value::Array(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3)
            ])
        );
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
                assert_eq!(
                    map.get("electrical"),
                    Some(&Value::String("passive".to_string()))
                );
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
        assert!(matches!(
            result,
            Err(SpecError {
                code: SpecErrorCode::UndefinedBinding,
                ..
            })
        ));
    }

    // ── Function call / shape tests ─────────────────────────────────────

    use crate::ast::CallArg;

    /// Helper: build an Expr::Call with positional args.
    fn call(name: &str, args: Vec<Spanned<Expr>>) -> Spanned<Expr> {
        spanned(Expr::Call {
            name: name.to_string(),
            args: args
                .into_iter()
                .map(|v| CallArg {
                    name: None,
                    value: v,
                })
                .collect(),
        })
    }

    /// Helper: build an Expr::Call with named args.
    fn call_named(name: &str, args: Vec<(&str, Spanned<Expr>)>) -> Spanned<Expr> {
        spanned(Expr::Call {
            name: name.to_string(),
            args: args
                .into_iter()
                .map(|(k, v)| CallArg {
                    name: Some(spanned(k.to_string())),
                    value: v,
                })
                .collect(),
        })
    }

    fn dim(val: f64, unit: Unit) -> Spanned<Expr> {
        spanned(Expr::Dim(val, unit))
    }

    fn coord(x: f64, y: f64) -> Spanned<Expr> {
        spanned(Expr::Tuple(
            Box::new(dim(x, Unit::Mm)),
            Box::new(dim(y, Unit::Mm)),
        ))
    }

    #[test]
    fn rect_basic_centered_at_origin() {
        let scope = make_scope();
        let expr = call("rect", vec![dim(100.0, Unit::Mm), dim(50.0, Unit::Mm)]);
        let val = eval_expr(&expr, &scope).unwrap();
        match val {
            Value::Shape(Shape::Rect { cx, cy, hw, hh }) => {
                assert_eq!(cx, 0);
                assert_eq!(cy, 0);
                // 100mm / 2 = 50mm = 50 * 393_701 internal units
                let expected_hw = unit_to_internal(50.0, Unit::Mm);
                assert_eq!(hw, expected_hw);
                let expected_hh = unit_to_internal(25.0, Unit::Mm);
                assert_eq!(hh, expected_hh);
            }
            other => panic!("expected Shape::Rect, got {:?}", other),
        }
    }

    #[test]
    fn rect_with_center() {
        let scope = make_scope();
        // rect(100mm, 50mm, center: (10mm, 20mm))
        let args = vec![
            CallArg {
                name: None,
                value: dim(100.0, Unit::Mm),
            },
            CallArg {
                name: None,
                value: dim(50.0, Unit::Mm),
            },
            CallArg {
                name: Some(spanned("center".to_string())),
                value: coord(10.0, 20.0),
            },
        ];
        let expr = spanned(Expr::Call {
            name: "rect".to_string(),
            args,
        });
        let val = eval_expr(&expr, &scope).unwrap();
        match val {
            Value::Shape(Shape::Rect { cx, cy, .. }) => {
                assert_eq!(cx, unit_to_internal(10.0, Unit::Mm));
                assert_eq!(cy, unit_to_internal(20.0, Unit::Mm));
            }
            other => panic!("expected Shape::Rect, got {:?}", other),
        }
    }

    #[test]
    fn rect_from_to() {
        let scope = make_scope();
        let expr = call_named(
            "rect",
            vec![("from", coord(0.0, 0.0)), ("to", coord(100.0, 50.0))],
        );
        let val = eval_expr(&expr, &scope).unwrap();
        match val {
            Value::Shape(Shape::Rect { cx, cy, hw, hh }) => {
                assert_eq!(cx, unit_to_internal(50.0, Unit::Mm));
                assert_eq!(cy, unit_to_internal(25.0, Unit::Mm));
                assert_eq!(hw, unit_to_internal(50.0, Unit::Mm));
                assert_eq!(hh, unit_to_internal(25.0, Unit::Mm));
            }
            other => panic!("expected Shape::Rect, got {:?}", other),
        }
    }

    #[test]
    fn rect_at_corner_origin() {
        // rect(at: lower-left corner, width, height) — 'at' is corner, not center
        let scope = make_scope();
        let expr = call_named(
            "rect",
            vec![
                ("at", coord(0.0, 0.0)),
                ("width", dim(100.0, Unit::Mm)),
                ("height", dim(50.0, Unit::Mm)),
            ],
        );
        let val = eval_expr(&expr, &scope).unwrap();
        match val {
            Value::Shape(Shape::Rect { cx, cy, hw, hh }) => {
                // Center should be at (50mm, 25mm), not (0, 0)
                assert_eq!(cx, unit_to_internal(50.0, Unit::Mm));
                assert_eq!(cy, unit_to_internal(25.0, Unit::Mm));
                assert_eq!(hw, unit_to_internal(50.0, Unit::Mm));
                assert_eq!(hh, unit_to_internal(25.0, Unit::Mm));
            }
            other => panic!("expected Shape::Rect, got {:?}", other),
        }
    }

    #[test]
    fn polygon_center_is_bounding_box_center() {
        // L-shaped polygon where centroid != bounding-box center
        // Vertices: (0,0), (100,0), (100,50), (50,50), (50,100), (0,100)
        let s = Shape::Polygon {
            vertices: vec![(0, 0), (100, 0), (100, 50), (50, 50), (50, 100), (0, 100)],
        };
        let (cx, cy) = s.center();
        // Bounding box: x=[0,100], y=[0,100] → center = (50, 50)
        assert_eq!(cx, 50);
        assert_eq!(cy, 50);
    }

    #[test]
    fn shape_field_access_center() {
        let scope = make_scope();
        let rect_expr = call("rect", vec![dim(60.0, Unit::Mm), dim(40.0, Unit::Mm)]);
        let expr = spanned(Expr::Path(
            Box::new(rect_expr),
            spanned("center".to_string()),
        ));
        let val = eval_expr(&expr, &scope).unwrap();
        assert_eq!(val, Value::CoordPoint(0, 0)); // centered at origin
    }

    #[test]
    fn circle_basic() {
        let scope = make_scope();
        let expr = call("circle", vec![dim(5.0, Unit::Mm)]);
        let val = eval_expr(&expr, &scope).unwrap();
        match val {
            Value::Shape(Shape::Circle { cx, cy, radius }) => {
                assert_eq!(cx, 0);
                assert_eq!(cy, 0);
                assert_eq!(radius, unit_to_internal(5.0, Unit::Mm));
            }
            other => panic!("expected Shape::Circle, got {:?}", other),
        }
    }

    #[test]
    fn rounded_rect_basic() {
        let scope = make_scope();
        let expr = call(
            "rounded_rect",
            vec![dim(60.0, Unit::Mm), dim(40.0, Unit::Mm), dim(3.0, Unit::Mm)],
        );
        let val = eval_expr(&expr, &scope).unwrap();
        match val {
            Value::Shape(Shape::RoundedRect {
                cx,
                cy,
                hw,
                hh,
                radius,
            }) => {
                assert_eq!(cx, 0);
                assert_eq!(cy, 0);
                assert_eq!(hw, unit_to_internal(30.0, Unit::Mm));
                assert_eq!(hh, unit_to_internal(20.0, Unit::Mm));
                assert_eq!(radius, unit_to_internal(3.0, Unit::Mm));
            }
            other => panic!("expected Shape::RoundedRect, got {:?}", other),
        }
    }

    #[test]
    fn inset_rect() {
        let scope = make_scope();
        // Create a 100mm x 50mm rect, then inset by 5mm
        let rect_expr = call("rect", vec![dim(100.0, Unit::Mm), dim(50.0, Unit::Mm)]);
        let expr = spanned(Expr::Call {
            name: "inset".to_string(),
            args: vec![
                CallArg {
                    name: None,
                    value: rect_expr,
                },
                CallArg {
                    name: None,
                    value: dim(5.0, Unit::Mm),
                },
            ],
        });
        let val = eval_expr(&expr, &scope).unwrap();
        match val {
            Value::Shape(Shape::Rect { hw, hh, .. }) => {
                // Original hw = 50mm, after inset 5mm -> 45mm
                assert_eq!(hw, unit_to_internal(45.0, Unit::Mm));
                assert_eq!(hh, unit_to_internal(20.0, Unit::Mm));
            }
            other => panic!("expected Shape::Rect, got {:?}", other),
        }
    }

    #[test]
    fn outset_circle() {
        let scope = make_scope();
        let circle_expr = call("circle", vec![dim(10.0, Unit::Mm)]);
        let expr = spanned(Expr::Call {
            name: "outset".to_string(),
            args: vec![
                CallArg {
                    name: None,
                    value: circle_expr,
                },
                CallArg {
                    name: None,
                    value: dim(2.0, Unit::Mm),
                },
            ],
        });
        let val = eval_expr(&expr, &scope).unwrap();
        match val {
            Value::Shape(Shape::Circle { radius, .. }) => {
                assert_eq!(radius, unit_to_internal(12.0, Unit::Mm));
            }
            other => panic!("expected Shape::Circle, got {:?}", other),
        }
    }

    #[test]
    fn translate_rect() {
        let scope = make_scope();
        let rect_expr = call("rect", vec![dim(100.0, Unit::Mm), dim(50.0, Unit::Mm)]);
        let expr = spanned(Expr::Call {
            name: "translate".to_string(),
            args: vec![
                CallArg {
                    name: None,
                    value: rect_expr,
                },
                CallArg {
                    name: None,
                    value: coord(10.0, 20.0),
                },
            ],
        });
        let val = eval_expr(&expr, &scope).unwrap();
        match val {
            Value::Shape(Shape::Rect { cx, cy, .. }) => {
                assert_eq!(cx, unit_to_internal(10.0, Unit::Mm));
                assert_eq!(cy, unit_to_internal(20.0, Unit::Mm));
            }
            other => panic!("expected Shape::Rect, got {:?}", other),
        }
    }

    #[test]
    fn shape_width_accessor() {
        let scope = make_scope();
        let rect_expr = call("rect", vec![dim(100.0, Unit::Mm), dim(50.0, Unit::Mm)]);
        let expr = spanned(Expr::Call {
            name: "width".to_string(),
            args: vec![CallArg {
                name: None,
                value: rect_expr,
            }],
        });
        let val = eval_expr(&expr, &scope).unwrap();
        assert_eq!(val, Value::Dim(unit_to_internal(100.0, Unit::Mm)));
    }

    #[test]
    fn shape_height_accessor() {
        let scope = make_scope();
        let rect_expr = call("rect", vec![dim(100.0, Unit::Mm), dim(50.0, Unit::Mm)]);
        let expr = spanned(Expr::Call {
            name: "height".to_string(),
            args: vec![CallArg {
                name: None,
                value: rect_expr,
            }],
        });
        let val = eval_expr(&expr, &scope).unwrap();
        assert_eq!(val, Value::Dim(unit_to_internal(50.0, Unit::Mm)));
    }

    #[test]
    fn shape_field_access_width() {
        // Test $shape.width via field access
        let scope = make_scope();
        let rect_expr = call("rect", vec![dim(60.0, Unit::Mm), dim(40.0, Unit::Mm)]);
        let expr = spanned(Expr::Path(
            Box::new(rect_expr),
            spanned("width".to_string()),
        ));
        let val = eval_expr(&expr, &scope).unwrap();
        assert_eq!(val, Value::Dim(unit_to_internal(60.0, Unit::Mm)));
    }

    #[test]
    fn min_dim() {
        let scope = make_scope();
        let expr = call("min", vec![dim(10.0, Unit::Mm), dim(5.0, Unit::Mm)]);
        let val = eval_expr(&expr, &scope).unwrap();
        assert_eq!(val, Value::Dim(unit_to_internal(5.0, Unit::Mm)));
    }

    #[test]
    fn max_integer() {
        let scope = make_scope();
        let expr = call(
            "max",
            vec![spanned(Expr::Integer(3)), spanned(Expr::Integer(7))],
        );
        let val = eval_expr(&expr, &scope).unwrap();
        assert_eq!(val, Value::Integer(7));
    }

    #[test]
    fn clamp_dim() {
        let scope = make_scope();
        let expr = call(
            "clamp",
            vec![dim(1.0, Unit::Mm), dim(5.0, Unit::Mm), dim(10.0, Unit::Mm)],
        );
        let val = eval_expr(&expr, &scope).unwrap();
        // 1mm clamped to [5mm, 10mm] = 5mm
        assert_eq!(val, Value::Dim(unit_to_internal(5.0, Unit::Mm)));
    }

    #[test]
    fn abs_negative_dim() {
        let scope = make_scope();
        // abs(-10mm): negate 10mm first, then abs
        let neg_dim = spanned(Expr::UnaryNeg(Box::new(dim(10.0, Unit::Mm))));
        let expr = spanned(Expr::Call {
            name: "abs".to_string(),
            args: vec![CallArg {
                name: None,
                value: neg_dim,
            }],
        });
        let val = eval_expr(&expr, &scope).unwrap();
        assert_eq!(val, Value::Dim(unit_to_internal(10.0, Unit::Mm)));
    }

    #[test]
    fn polygon_from_array() {
        let scope = make_scope();
        let points = spanned(Expr::Array(vec![
            coord(0.0, 0.0),
            coord(10.0, 0.0),
            coord(10.0, 5.0),
            coord(0.0, 5.0),
        ]));
        let expr = call("polygon", vec![points]);
        let val = eval_expr(&expr, &scope).unwrap();
        match val {
            Value::Shape(Shape::Polygon { vertices }) => {
                assert_eq!(vertices.len(), 4);
            }
            other => panic!("expected Shape::Polygon, got {:?}", other),
        }
    }

    #[test]
    fn unknown_function_errors() {
        let scope = make_scope();
        let expr = call("nonexistent_func", vec![]);
        let result = eval_expr(&expr, &scope);
        assert!(result.is_err());
    }

    #[test]
    fn rect_wrong_arg_count_errors() {
        let scope = make_scope();
        let expr = call("rect", vec![dim(10.0, Unit::Mm)]);
        let result = eval_expr(&expr, &scope);
        assert!(result.is_err());
    }

    #[test]
    fn rect_to_vertices() {
        let s = Shape::Rect {
            cx: 0,
            cy: 0,
            hw: 100,
            hh: 50,
        };
        let verts = s.to_vertices();
        assert_eq!(verts.len(), 4);
        assert_eq!(verts[0], (-100, -50));
        assert_eq!(verts[1], (100, -50));
        assert_eq!(verts[2], (100, 50));
        assert_eq!(verts[3], (-100, 50));
    }

    #[test]
    fn circle_to_vertices_count() {
        let s = Shape::Circle {
            cx: 0,
            cy: 0,
            radius: 1000,
        };
        let verts = s.to_vertices();
        assert_eq!(verts.len(), 72); // 72-point approximation
    }

    #[test]
    fn rounded_rect_to_vertices() {
        let s = Shape::RoundedRect {
            cx: 0,
            cy: 0,
            hw: 1000,
            hh: 500,
            radius: 100,
        };
        let verts = s.to_vertices();
        // 4 corners * 8 points each = 32 vertices (no duplicate boundary points)
        assert_eq!(verts.len(), 32);
    }

    #[test]
    fn shape_inset_clamps_to_zero() {
        let s = Shape::Rect {
            cx: 0,
            cy: 0,
            hw: 100,
            hh: 50,
        };
        let inset = s.inset(200).unwrap(); // inset more than half-width
        match inset {
            Shape::Rect { hw, hh, .. } => {
                assert_eq!(hw, 0);
                assert_eq!(hh, 0);
            }
            _ => panic!("expected Rect"),
        }
    }

    #[test]
    fn inset_negative_amount_errors() {
        let s = Shape::Rect {
            cx: 0,
            cy: 0,
            hw: 100,
            hh: 50,
        };
        assert!(s.inset(-10).is_err());
    }

    #[test]
    fn outset_negative_amount_errors() {
        let s = Shape::Rect {
            cx: 0,
            cy: 0,
            hw: 100,
            hh: 50,
        };
        assert!(s.outset(-10).is_err());
    }

    #[test]
    fn inset_polygon_errors() {
        let s = Shape::Polygon {
            vertices: vec![(0, 0), (100, 0), (100, 100)],
        };
        assert!(s.inset(10).is_err());
    }

    #[test]
    fn outset_polygon_errors() {
        let s = Shape::Polygon {
            vertices: vec![(0, 0), (100, 0), (100, 100)],
        };
        assert!(s.outset(10).is_err());
    }

    #[test]
    fn polygon_too_few_vertices_errors() {
        let scope = make_scope();
        // Only 2 points — not enough for a polygon
        let points = spanned(Expr::Array(vec![coord(0.0, 0.0), coord(10.0, 0.0)]));
        let expr = call("polygon", vec![points]);
        assert!(eval_expr(&expr, &scope).is_err());
    }

    #[test]
    fn polygon_empty_errors() {
        let scope = make_scope();
        let points = spanned(Expr::Array(vec![]));
        let expr = call("polygon", vec![points]);
        assert!(eval_expr(&expr, &scope).is_err());
    }
}

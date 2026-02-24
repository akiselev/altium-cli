#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn merge(self, other: Span) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub const fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpsFile {
    pub statements: Vec<Spanned<Statement>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Binding(Binding),
    Assert(AssertStmt),
    Op(Op),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub name: Spanned<String>,
    pub value: Spanned<BindingValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BindingValue {
    Expr(Expr),
    Op(Op),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssertStmt {
    pub condition: Spanned<AssertCondition>,
    pub message: Option<Spanned<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssertCondition {
    Existence(Spanned<Expr>),
    Comparison {
        left: Spanned<Expr>,
        op: Spanned<CompareOp>,
        right: Spanned<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TemplateString {
    pub parts: Vec<Spanned<TemplatePart>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplatePart {
    Literal(String),
    Interpolation(Spanned<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Op {
    pub name: Spanned<String>,
    pub target: Option<Spanned<Expr>>,
    pub selector: Option<Spanned<Selector>>,
    pub body: Option<Spanned<Object>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Selector {
    pub raw: String,
    pub expr: Spanned<SelectorExpr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectorExpr {
    Or(Vec<Spanned<SelectorExpr>>),
    And(Vec<Spanned<SelectorExpr>>),
    Not(Box<Spanned<SelectorExpr>>),
    Chain(SelectorChain),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectorChain {
    pub first: Spanned<SelectorCompound>,
    pub rest: Vec<Spanned<SelectorLink>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectorLink {
    pub combinator: Spanned<SelectorCombinator>,
    pub right: Spanned<SelectorCompound>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorCombinator {
    Descendant,
    Child,
    Adjacent,
    Sibling,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectorCompound {
    pub head: Spanned<SelectorSimple>,
    pub filters: Vec<Spanned<SelectorFilter>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectorSimple {
    Any,
    DollarRef(String),
    DesignatorPattern {
        ident: String,
        wildcard: Option<SelectorWildcard>,
    },
    NetPattern(String),
    ValuePattern(SelectorValue),
    PartPattern(String),
    IdPattern(i32),
    ComponentPin {
        component: String,
        pin: String,
    },
    Type(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorWildcard {
    AnySuffix,
    OneChar,
    TwoChars,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectorFilter {
    Attribute(SelectorAttribute),
    Pseudo(Spanned<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectorAttribute {
    pub field: Vec<Spanned<String>>,
    pub op: Spanned<SelectorAttrOp>,
    pub value: Spanned<SelectorValue>,
    pub mode: Option<Spanned<SelectorStringMode>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorStringMode {
    CaseInsensitive,
    CaseSensitive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorAttrOp {
    Eq,
    Ne,
    Contains,
    StartsWith,
    EndsWith,
    WordMatch,
    Gt,
    Lt,
    Ge,
    Le,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectorValue {
    String(String),
    Integer(i32),
    Float(f64),
    Dim(f64, Unit),
    Bool(bool),
    Ident(String),
    Regex(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub items: Vec<Spanned<ObjectItem>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectItem {
    Binding(Binding),
    Spread(Spanned<Expr>),
    Field(Field),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub key: Spanned<Key>,
    pub value: Spanned<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    pub segments: Vec<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    String(String),
    TemplateString(TemplateString),
    Integer(i32),
    Float(f64),
    Dim(f64, Unit),
    Color(u8, u8, u8),
    Bool(bool),
    Null,
    Ident(String),
    DollarIdent(String),
    Path(Box<Spanned<Expr>>, Spanned<String>),
    Index(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    BinOp(Box<Spanned<Expr>>, Spanned<BinOp>, Box<Spanned<Expr>>),
    UnaryNeg(Box<Spanned<Expr>>),
    Tuple(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Array(Vec<Spanned<Expr>>),
    Object(Object),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Mil,
    Mm,
    Inch,
    Dxp,
    Raw,
}

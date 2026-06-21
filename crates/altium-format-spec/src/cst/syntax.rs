//! Syntax kinds for the lossless concrete syntax tree (CST).
//!
//! This is the flat `SyntaxKind` enum that backs the `cstree` red/green tree.
//! Every byte of source is represented by exactly one leaf token (including
//! whitespace and comments), so the tree is lossless: `node.text() == source`.
//!
//! Token kinds (leaves) come first, then node kinds (interior). Per the project
//! rules, unknown syntax is a hard parse error — there is no "opaque" catch-all
//! node; `ERROR` exists only for future recoverable-parse support and is not used
//! to silently retain unrecognized input.

use cstree::Syntax;

/// The kind of every token and node in the spec CST.
///
/// `#[repr(u32)]` + `#[derive(Syntax)]` lets `cstree` round-trip the kind through
/// its compact `RawSyntaxKind`. `#[static_text]` marks tokens whose text is always
/// identical (keywords, punctuation, the single-`\n` newline) so `cstree` stores
/// them without interning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Syntax)]
#[repr(u32)]
pub enum SyntaxKind {
    // ── Trivia tokens (leaves) ────────────────────────────────────────────────
    /// A maximal run of spaces, tabs, or carriage returns.
    Whitespace,
    /// A single line feed (`\n`).
    #[static_text("\n")]
    Newline,
    /// A `// …` line comment (text excludes the trailing newline).
    LineComment,
    /// A `/* … */` block comment (may be nested).
    BlockComment,

    // ── Literal / name tokens (leaves) ────────────────────────────────────────
    /// A bare identifier.
    Ident,
    /// A `$identifier`.
    DollarIdent,
    /// A double-quoted string literal (including quotes).
    String,
    /// A backtick template literal (including backticks).
    Template,
    /// An integer literal.
    Int,
    /// A floating-point literal.
    Float,
    /// A dimension literal such as `100mil` (number + unit).
    Dim,
    /// A `#RRGGBB` color literal.
    Color,

    // ── Keyword tokens (leaves) ───────────────────────────────────────────────
    #[static_text("import")]
    ImportKw,
    #[static_text("as")]
    AsKw,
    #[static_text("component")]
    ComponentKw,
    #[static_text("footprint")]
    FootprintKw,
    #[static_text("project")]
    ProjectKw,
    #[static_text("sheet")]
    SheetKw,
    #[static_text("net")]
    NetKw,
    #[static_text("power")]
    PowerKw,
    #[static_text("pin")]
    PinKw,
    #[static_text("pad")]
    PadKw,
    #[static_text("part")]
    PartKw,
    #[static_text("parameter")]
    ParameterKw,
    #[static_text("alias")]
    AliasKw,
    #[static_text("row")]
    RowKw,
    #[static_text("column")]
    ColumnKw,
    #[static_text("grid")]
    GridKw,
    #[static_text("board")]
    BoardKw,
    #[static_text("swap_group")]
    SwapGroupKw,
    #[static_text("group")]
    GroupKw,
    #[static_text("separate")]
    SeparateKw,
    #[static_text("autoplace")]
    AutoplaceKw,
    #[static_text("pad_net")]
    PadNetKw,
    #[static_text("let")]
    LetKw,
    #[static_text("true")]
    TrueKw,
    #[static_text("false")]
    FalseKw,
    #[static_text("null")]
    NullKw,

    // ── Punctuation tokens (leaves) ───────────────────────────────────────────
    #[static_text("{")]
    LBrace,
    #[static_text("}")]
    RBrace,
    #[static_text("(")]
    LParen,
    #[static_text(")")]
    RParen,
    #[static_text("[")]
    LBracket,
    #[static_text("]")]
    RBracket,
    #[static_text(":")]
    Colon,
    #[static_text(",")]
    Comma,
    #[static_text(".")]
    Dot,
    #[static_text("...")]
    DotDotDot,
    #[static_text("=")]
    Eq,
    #[static_text("->")]
    Arrow,
    #[static_text("+")]
    Plus,
    #[static_text("-")]
    Minus,
    #[static_text("*")]
    Star,
    #[static_text("/")]
    Slash,
    #[static_text(";")]
    Semi,
    #[static_text("#")]
    Hash,

    // ── Node kinds (interior) ─────────────────────────────────────────────────
    /// Root of the file; contains all top-level items and trivia.
    Root,

    // Top-level item declarations.
    Import,
    LetBinding,
    Component,
    Footprint,
    SwapGroup,

    // Shared structural nodes.
    /// A `name =` binding prefix on a declaration.
    Binding,
    /// A `#[annotation(...)]` attribute.
    Annotation,
    /// A single `key = value` pair inside an annotation.
    AnnotationArg,
    /// An entity name (identifier, string, or integer) in declaration position.
    Name,
    /// A brace-delimited item-list body (`{ ... }`) of a container declaration.
    Block,
    /// A brace-delimited object literal (`{ key: value, ... }`).
    Object,
    /// A `key: value` property.
    Property,
    /// A `...expr` spread item.
    Spread,

    // Component / footprint children.
    Pin,
    Parameter,
    Part,
    Alias,
    FootprintMap,
    Graphic,
    PinConnection,
    PadNet,
    PinPadPair,
    /// A `#NET` or `nc` pin-connection target.
    NetTarget,
    Pad,
    Row,
    Column,
    Grid,

    // Expression nodes.
    BinExpr,
    UnaryExpr,
    PathExpr,
    IndexExpr,
    TupleExpr,
    ArrayExpr,
    ParenExpr,
    CallExpr,
    CallArg,
    /// A `$root.field[i]` dollar-path reference.
    DollarPath,

    // Project / SchDoc / PcbDoc top-level item declarations.
    Project,
    Sheet,
    Net,
    Power,
    SchDocObject,
    Board,
    Placement,
    Routing,
    PcbPrimitive,
    Polygon,
    Rule,
    Class,
    DiffPair,

    // Project sub-blocks.
    DocumentBlock,
    AnnotationBlock,
    MatchParameter,
    ErcMatrix,
    ErcMatrixEntry,
    ErcLevels,
    ErcLevelEntry,
    OutputGroup,
    Output,
    Comparison,
    ComparisonRule,
    ClassGen,
    LibraryUpdate,
    Variant,
    Variation,
    ParamVariation,

    // Placement sub-blocks.
    Place,
    PlacementConstraint,
    Minimize,
    PlacementGroup,
    PlacementSeparate,
    Optimize,
    Clearance,
    Autoplace,

    // SchDoc sub-blocks.
    Entry,
    Constraint,
    FontBlock,
    Font,

    /// A reserved error node for future recoverable parsing. Not emitted today.
    Error,
}

/// The red-tree node type for the spec CST.
pub type SyntaxNode = cstree::syntax::SyntaxNode<SyntaxKind>;
/// A resolved (interner-attached) red-tree node, from which `.text()` is readable.
pub type ResolvedNode = cstree::syntax::ResolvedNode<SyntaxKind>;
/// A resolved token.
pub type ResolvedToken = cstree::syntax::ResolvedToken<SyntaxKind>;

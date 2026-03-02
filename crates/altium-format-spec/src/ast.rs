pub use crate::diagnostic::{BinOp, Span, Spanned, Unit};
pub use super::lexer::TemplatePart;

/// A parsed spec file.
#[derive(Debug, Clone, PartialEq)]
pub struct SpecFile {
    pub items: Vec<Spanned<SpecItem>>,
}

/// Top-level items in a spec file.
#[derive(Debug, Clone, PartialEq)]
pub enum SpecItem {
    Import(ImportDecl),
    LetBinding(LetBinding),
    Component(ComponentDecl),
    Footprint(FootprintDecl),
    Project(ProjectDecl),
    // SchDoc-specific
    Sheet(SheetDecl),
    Net(NetDecl),
    Power(PowerDecl),
    SchDocObject(SchDocObjectDecl),
    // PcbDoc-specific
    Board(BoardDecl),
    PcbDocPrimitive(PcbDocPrimitiveDecl),
    Polygon(PolygonDecl),
    Rule(RuleDecl),
    Class(ClassDecl),
    DifferentialPair(DifferentialPairDecl),
}

/// import "path" [as alias]
#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    pub path: Spanned<String>,
    pub alias: Option<Spanned<String>>,
}

/// [let] name = expr
#[derive(Debug, Clone, PartialEq)]
pub struct LetBinding {
    pub name: Spanned<String>,
    pub value: Spanned<Expr>,
}

/// [binding =] component NAME { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentDecl {
    pub binding: Option<Spanned<String>>,
    pub name: Spanned<EntityName>,
    pub body: Vec<Spanned<ComponentItem>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentItem {
    Property(Property),
    LetBinding(LetBinding),
    Part(PartBlock),
    Pin(PinDecl),
    Parameter(ParameterDecl),
    Alias(AliasDecl),
    FootprintMap(FootprintMapDecl),
    Graphic(GraphicDecl),
}

/// [binding =] part N { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct PartBlock {
    pub binding: Option<Spanned<String>>,
    pub number: Spanned<i32>,
    pub body: Vec<Spanned<PartItem>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PartItem {
    LetBinding(LetBinding),
    Pin(PinDecl),
    Graphic(GraphicDecl),
}

/// [binding =] pin NAME { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct PinDecl {
    pub binding: Option<Spanned<String>>,
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

/// [binding =] parameter NAME { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterDecl {
    pub binding: Option<Spanned<String>>,
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

/// alias NAME  (no body)
#[derive(Debug, Clone, PartialEq)]
pub struct AliasDecl {
    pub name: Spanned<EntityName>,
}

/// footprint NAME_OR_PATH { map { ... } ... }
#[derive(Debug, Clone, PartialEq)]
pub struct FootprintMapDecl {
    pub name: Spanned<FootprintRef>,
    pub maps: Vec<Spanned<MapEntry>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FootprintRef {
    Name(EntityName),
    DollarPath(DollarPath),
}

/// map { pin: 1, pad: 1 }
#[derive(Debug, Clone, PartialEq)]
pub struct MapEntry {
    pub body: Spanned<Object>,
}

/// [binding =] footprint NAME { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct FootprintDecl {
    pub binding: Option<Spanned<String>>,
    pub name: Spanned<EntityName>,
    pub body: Vec<Spanned<FootprintItem>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FootprintItem {
    Property(Property),
    LetBinding(LetBinding),
    Pad(PadDecl),
    Row(RowDecl),
    Column(RowDecl),
    Grid(GridDecl),
    Graphic(GraphicDecl),
}

/// [binding =] pad NAME { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct PadDecl {
    pub binding: Option<Spanned<String>>,
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

/// row { ... } or column { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct RowDecl {
    pub body: Spanned<Object>,
}

/// grid { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct GridDecl {
    pub body: Spanned<Object>,
}

// ── Project declarations ──────────────────────────────────────────────

/// [binding =] project NAME { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectDecl {
    pub binding: Option<Spanned<String>>,
    pub name: Spanned<EntityName>,
    pub body: Vec<Spanned<ProjectItem>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectItem {
    Property(Property),
    LetBinding(LetBinding),
    Document(DocumentBlockDecl),
    Annotation(AnnotationBlockDecl),
    ErcMatrix(Vec<Spanned<ErcMatrixEntryDecl>>),
    ErcLevels(Vec<Spanned<ErcLevelEntryDecl>>),
    OutputGroup(OutputGroupBlockDecl),
    Comparison(Vec<Spanned<ComparisonRuleDecl>>),
    ClassGen(Vec<Spanned<Property>>),
    LibraryUpdate(Vec<Spanned<Property>>),
    Variant(VariantBlockDecl),
}

/// document "path/to/file.SchDoc" { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentBlockDecl {
    pub path: Spanned<EntityName>,
    pub body: Vec<Spanned<Property>>,
}

/// annotation { ... match_parameter N { ... } ... }
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationBlockDecl {
    pub properties: Vec<Spanned<Property>>,
    pub match_parameters: Vec<Spanned<MatchParameterDecl>>,
}

/// match_parameter N { key: value, ... }
#[derive(Debug, Clone, PartialEq)]
pub struct MatchParameterDecl {
    pub index: Spanned<i32>,
    pub body: Spanned<Object>,
}

/// erc_matrix { (row, col): level, ... }
#[derive(Debug, Clone, PartialEq)]
pub struct ErcMatrixEntryDecl {
    pub row: Spanned<String>,
    pub col: Spanned<String>,
    pub level: Spanned<String>,
}

/// erc_levels { name: level, ... }
#[derive(Debug, Clone, PartialEq)]
pub struct ErcLevelEntryDecl {
    pub name: Spanned<String>,
    pub level: Spanned<Expr>,
}

/// output_group "Name" { output "Name" { ... } ... }
#[derive(Debug, Clone, PartialEq)]
pub struct OutputGroupBlockDecl {
    pub name: Spanned<EntityName>,
    pub properties: Vec<Spanned<Property>>,
    pub outputs: Vec<Spanned<OutputBlockDecl>>,
}

/// output "Name" { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct OutputBlockDecl {
    pub name: Spanned<EntityName>,
    pub body: Vec<Spanned<Property>>,
}

/// comparison { rule "Kind" { ... } ... }
#[derive(Debug, Clone, PartialEq)]
pub struct ComparisonRuleDecl {
    pub kind: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

/// variant "Name" { ... variation "Designator" { ... } ... }
#[derive(Debug, Clone, PartialEq)]
pub struct VariantBlockDecl {
    pub name: Spanned<EntityName>,
    pub properties: Vec<Spanned<Property>>,
    pub variations: Vec<Spanned<VariationDecl>>,
    pub param_variations: Vec<Spanned<ParamVariationDecl>>,
}

/// variation "Designator" { kind: ..., alternate_part: ... }
#[derive(Debug, Clone, PartialEq)]
pub struct VariationDecl {
    pub designator: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

/// param_variation "Designator" { parameter: ..., value: ... }
#[derive(Debug, Clone, PartialEq)]
pub struct ParamVariationDecl {
    pub designator: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

// ── SchDoc declarations ──────────────────────────────────────────────

/// sheet { ... } — sheet metadata block
#[derive(Debug, Clone, PartialEq)]
pub struct SheetDecl {
    pub body: Vec<Spanned<SheetItem>>,
}

/// Items inside a sheet { } metadata block.
#[derive(Debug, Clone, PartialEq)]
pub enum SheetItem {
    Property(Property),
    LetBinding(LetBinding),
    FontBlock(FontBlockDecl),
}

/// fonts { font 1 { ... } font 2 { ... } }
#[derive(Debug, Clone, PartialEq)]
pub struct FontBlockDecl {
    pub fonts: Vec<Spanned<FontDecl>>,
}

/// font N { name: "...", size: 10 }
#[derive(Debug, Clone, PartialEq)]
pub struct FontDecl {
    pub id: Spanned<i32>,
    pub body: Spanned<Object>,
}

/// net NAME { pins: [...] }
#[derive(Debug, Clone, PartialEq)]
pub struct NetDecl {
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

/// power NAME { style: ..., pins: [...] }
#[derive(Debug, Clone, PartialEq)]
pub struct PowerDecl {
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

/// Identifier-dispatched SchDoc object block: wire { ... }, bus { ... }, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct SchDocObjectDecl {
    pub object_type: Spanned<String>,
    pub name: Option<Spanned<EntityName>>,
    pub body: Vec<Spanned<SchDocObjectItem>>,
}

/// Items inside a SchDoc object block.
#[derive(Debug, Clone, PartialEq)]
pub enum SchDocObjectItem {
    Property(Property),
    LetBinding(LetBinding),
    /// Nested child block (e.g. `entry DATA { ... }` inside `sheet_symbol`)
    Entry(EntryDecl),
    /// Nested parameter block
    Parameter(ParameterDecl),
    /// Nested graphic block
    Graphic(GraphicDecl),
}

/// entry NAME { ... } — child of a sheet_symbol
#[derive(Debug, Clone, PartialEq)]
pub struct EntryDecl {
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

// ── PcbDoc declarations ──────────────────────────────────────────────

/// board NAME { settings... }
#[derive(Debug, Clone, PartialEq)]
pub struct BoardDecl {
    pub name: Spanned<EntityName>,
    pub body: Vec<Spanned<BoardItem>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BoardItem {
    Property(Property),
    LetBinding(LetBinding),
}

/// track [NAME] { ... }, arc { ... }, via { ... }, pad NAME { ... }, etc.
/// Also used for dimension { ... } at PcbDoc top level.
#[derive(Debug, Clone, PartialEq)]
pub struct PcbDocPrimitiveDecl {
    pub primitive_type: Spanned<String>,
    pub name: Option<Spanned<EntityName>>,
    pub body: Spanned<Object>,
}

/// polygon NAME { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct PolygonDecl {
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

/// rule NAME { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct RuleDecl {
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

/// class NAME { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

/// differential_pair NAME { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct DifferentialPairDecl {
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

/// [binding =] GRAPHIC_TYPE { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct GraphicDecl {
    pub binding: Option<Spanned<String>>,
    pub graphic_type: Spanned<String>,
    pub body: Spanned<Object>,
}

/// Entity name: identifier, quoted string, or integer
#[derive(Debug, Clone, PartialEq)]
pub enum EntityName {
    Ident(String),
    String(String),
    Integer(i32),
}

impl EntityName {
    /// The string representation used as the identity key.
    pub fn as_str(&self) -> String {
        match self {
            EntityName::Ident(s) => s.clone(),
            EntityName::String(s) => s.clone(),
            EntityName::Integer(n) => n.to_string(),
        }
    }
}

/// Expression AST
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // Literals
    String(String),
    Template(Vec<TemplatePart>),
    Integer(i32),
    Float(f64),
    Dim(f64, Unit),
    Color(u8, u8, u8),
    Bool(bool),
    Null,

    // References
    Ident(String),
    DollarIdent(String),
    Path(Box<Spanned<Expr>>, Spanned<String>),
    Index(Box<Spanned<Expr>>, Box<Spanned<Expr>>),

    // Operators
    BinOp(Box<Spanned<Expr>>, Spanned<BinOp>, Box<Spanned<Expr>>),
    UnaryNeg(Box<Spanned<Expr>>),

    // Compound
    Tuple(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Array(Vec<Spanned<Expr>>),
    Object(Object),
}

/// { items... }
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub items: Vec<Spanned<ObjectItem>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectItem {
    LetBinding(LetBinding),
    Spread(Spanned<Expr>),
    Property(Property),
}

/// key: value
#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub key: Spanned<String>,
    pub value: Spanned<Expr>,
}

/// A dollar-prefixed path: $root.field[index]...
#[derive(Debug, Clone, PartialEq)]
pub struct DollarPath {
    pub root: Spanned<String>,
    pub steps: Vec<Spanned<PathStep>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PathStep {
    Field(String),
    Index(Expr),
}

/// Set of known graphic type identifiers for SchLib.
pub const SCH_GRAPHIC_TYPES: &[&str] = &[
    "line",
    "rectangle",
    "arc",
    "elliptical_arc",
    "ellipse",
    "polyline",
    "polygon",
    "bezier",
    "pie",
    "round_rectangle",
    "label",
    "text_frame",
    "image",
];

/// Set of known graphic type identifiers for PcbLib.
pub const PCB_GRAPHIC_TYPES: &[&str] = &[
    "track",
    "arc",
    "fill",
    "region",
    "text",
    "via",
    "component_body",
    "line",
    "polyline",
];

/// All known graphic type identifiers (union of SchLib and PcbLib).
pub const ALL_GRAPHIC_TYPES: &[&str] = &[
    "line",
    "rectangle",
    "arc",
    "elliptical_arc",
    "ellipse",
    "polyline",
    "polygon",
    "bezier",
    "pie",
    "round_rectangle",
    "label",
    "text_frame",
    "image",
    "track",
    "fill",
    "region",
    "text",
    "via",
    "component_body",
];

pub fn is_graphic_type(s: &str) -> bool {
    ALL_GRAPHIC_TYPES.contains(&s)
}

/// SchDoc object type identifiers that are parsed as top-level identifier-dispatched blocks.
pub const SCHDOC_OBJECT_TYPES: &[&str] = &[
    "wire",
    "bus",
    "net_label",
    "power_object",
    "port",
    "junction",
    "no_connect",
    "bus_entry",
    "sheet_symbol",
    "parameter_set",
    "note",
    "probe",
    "compile_mask",
    "blanket",
    "harness_connector",
    "signal_harness",
];

pub fn is_schdoc_object_type(s: &str) -> bool {
    SCHDOC_OBJECT_TYPES.contains(&s)
}

/// PcbDoc primitive types at top level.
pub const PCBDOC_PRIMITIVE_TYPES: &[&str] = &[
    "track", "arc", "via", "fill", "text", "region", "component_body", "dimension",
];

/// PcbDoc named block types at top level.
pub const PCBDOC_BLOCK_TYPES: &[&str] = &[
    "polygon", "rule", "class", "differential_pair",
];

pub fn is_pcbdoc_primitive_type(s: &str) -> bool {
    PCBDOC_PRIMITIVE_TYPES.contains(&s)
}

pub fn is_pcbdoc_block_type(s: &str) -> bool {
    PCBDOC_BLOCK_TYPES.contains(&s)
}

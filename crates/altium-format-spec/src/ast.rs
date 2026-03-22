pub use super::lexer::TemplatePart;
pub use crate::diagnostic::{BinOp, Span, Spanned, Unit};

/// Sync annotation attached to a block declaration: `#[annotation(id = "...", ...)]`.
/// This is distinct from `AnnotationBlockDecl`, which represents Altium's designator
/// annotation feature inside a project block. `BlockAnnotation` is for the sync system.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockAnnotation {
    pub id: Option<Spanned<String>>,
    pub stable: Option<Spanned<bool>>,
    pub group: Option<Spanned<String>>,
    pub source_id: Option<Spanned<String>>,
}

/// Known keys for a `#[annotation(...)]` attribute.
///
/// Only predefined keys are permitted — arbitrary key-value pairs are not accepted.
///
/// Rationale: if arbitrary keys were allowed, a typo like `stabl = true` would be
/// silently accepted by the parser and have no effect. With a predefined enum the
/// parser rejects unknown keys at parse time, surfacing the mistake immediately.
/// To attach new metadata to a block, add a new variant here rather than introducing
/// a free-form escape hatch.
#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationKey {
    Id,
    Stable,
    Group,
    SourceId,
}

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
    SwapGroup(SwapGroupDecl),
    // SchDoc-specific
    Sheet(SheetDecl),
    Net(NetDecl),
    Power(PowerDecl),
    SchDocObject(SchDocObjectDecl),
    // PcbDoc-specific
    Board(BoardDecl),
    Placement(PlacementDecl),
    Routing(RoutingDecl),
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

/// [binding =] swap_group NAME { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct SwapGroupDecl {
    pub binding: Option<Spanned<String>>,
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
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
    pub annotation: Option<Spanned<BlockAnnotation>>,
    pub binding: Option<Spanned<String>>,
    pub name: Spanned<EntityName>,
    pub body: Vec<Spanned<ComponentItem>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentItem {
    Property(Property),
    LetBinding(LetBinding),
    Part(PartBlock),
    PinConnection(PinConnectionDecl),
    Pin(PinDecl),
    Parameter(ParameterDecl),
    Alias(AliasDecl),
    FootprintMap(FootprintMapDecl),
    Graphic(GraphicDecl),
    SwapGroup(SwapGroupDecl),
}

/// Target of a pin connection declaration: `pin X -> #NET` or `pin X -> nc`.
#[derive(Debug, Clone, PartialEq)]
pub enum PinConnectionTarget {
    /// `#NAME` — a signal or power net reference.
    NetRef(Spanned<String>),
    /// `nc` — a no-connect marker.
    NoConnect,
}

/// `pin NAME -> #NET` or `pin NAME -> nc` inside a schdoc component body.
#[derive(Debug, Clone, PartialEq)]
pub struct PinConnectionDecl {
    /// The pin name or designator (e.g. `GPIO4`, `1`).
    pub pin_name: Spanned<String>,
    /// The connection target.
    pub target: PinConnectionTarget,
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
    Property(Property),
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

/// footprint REF                              — implicit 1:1 pin-to-pad mapping
/// footprint REF { $pin: $ref.pad, ... }      — explicit remapping
#[derive(Debug, Clone, PartialEq)]
pub struct FootprintMapDecl {
    pub name: Spanned<FootprintRef>,
    /// `None` = implicit 1:1 mapping (pin N → pad N for all pads).
    /// `Some(pairs)` = explicit pin:pad remapping.
    pub maps: Option<Vec<Spanned<PinPadPair>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FootprintRef {
    Name(EntityName),
    DollarPath(DollarPath),
}

/// $pin_ref: $footprint_ref.padN
#[derive(Debug, Clone, PartialEq)]
pub struct PinPadPair {
    pub pin: Spanned<DollarPath>,
    pub pad: Spanned<DollarPath>,
}

/// [binding =] footprint NAME { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct FootprintDecl {
    pub annotation: Option<Spanned<BlockAnnotation>>,
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
    pub annotation: Option<Spanned<BlockAnnotation>>,
    pub body: Vec<Spanned<SheetItem>>,
}

/// Items inside a sheet { } metadata block.
#[derive(Debug, Clone, PartialEq)]
pub enum SheetItem {
    Property(Property),
    LetBinding(LetBinding),
    FontBlock(FontBlockDecl),
    Constraint(ConstraintDecl),
}

/// `constraint <kind> { key: value, ... }` — placement constraint inside a sheet block.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintDecl {
    pub annotation: Option<Spanned<BlockAnnotation>>,
    pub kind: Spanned<ConstraintKind>,
    pub body: Spanned<Object>,
}

/// Typed constraint kind — catches typos at parse time.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintKind {
    EdgePlacement,
    Directional,
    Near,
    Region,
    FixedPosition,
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
    pub annotation: Option<Spanned<BlockAnnotation>>,
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

/// power NAME { style: ..., pins: [...] }
#[derive(Debug, Clone, PartialEq)]
pub struct PowerDecl {
    pub annotation: Option<Spanned<BlockAnnotation>>,
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
    pub annotation: Option<Spanned<BlockAnnotation>>,
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
    pub annotation: Option<Spanned<BlockAnnotation>>,
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

/// rule NAME { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct RuleDecl {
    pub annotation: Option<Spanned<BlockAnnotation>>,
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

/// class NAME { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub annotation: Option<Spanned<BlockAnnotation>>,
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

/// differential_pair NAME { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct DifferentialPairDecl {
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

/// routing { ... } top-level block for PcbDoc specs.
#[derive(Debug, Clone, PartialEq)]
pub struct RoutingDecl {
    pub body: Spanned<Object>,
}

/// placement { ... } top-level block for placement solver directives.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacementDecl {
    pub annotation: Option<Spanned<BlockAnnotation>>,
    pub body: Vec<Spanned<PlacementItem>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlacementItem {
    Property(Property),
    LetBinding(LetBinding),
    Place(PlaceDecl),
    Constraint(PlacementConstraintDecl),
    Optimize(Spanned<Object>),
    Clearance(Spanned<Object>),
    GroupDecl(PlacementGroupDecl),
    SeparateDecl(PlacementSeparateDecl),
    AutoplaceBlock(Spanned<Object>),
    /// `minimize { objective_key }` — set the optimization objective.
    ///
    /// Supported objectives: `wirelength`, `congestion`, `area`.
    /// Can include `subject_to { ... }` block for constraint relaxation hints.
    Minimize(MinimizeDecl),
}

/// `minimize { wirelength } subject_to { ... }` — optimization objective
/// with optional constraint relaxation hints.
#[derive(Debug, Clone, PartialEq)]
pub struct MinimizeDecl {
    /// The objective to minimize (e.g., "wirelength", "congestion").
    pub objective: Spanned<String>,
    /// Optional constraint relaxation hints in `subject_to { ... }` block.
    pub subject_to: Option<Spanned<Object>>,
}

/// group NAME { components: [...] }
#[derive(Debug, Clone, PartialEq)]
pub struct PlacementGroupDecl {
    pub name: Spanned<String>,
    pub body: Spanned<Object>,
}

/// separate $group_a, $group_b { gap: Nmm }
#[derive(Debug, Clone, PartialEq)]
pub struct PlacementSeparateDecl {
    pub groups: Vec<Spanned<DollarPath>>,
    pub body: Option<Spanned<Object>>,
}

/// place U1, U2 { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct PlaceDecl {
    pub annotation: Option<Spanned<BlockAnnotation>>,
    pub designators: Vec<Spanned<EntityName>>,
    pub body: Spanned<Object>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlacementConstraintDecl {
    LeftOf {
        a: Spanned<DollarPath>,
        b: Spanned<DollarPath>,
        body: Option<Spanned<Object>>,
    },
    RightOf {
        a: Spanned<DollarPath>,
        b: Spanned<DollarPath>,
        body: Option<Spanned<Object>>,
    },
    Above {
        a: Spanned<DollarPath>,
        b: Spanned<DollarPath>,
        body: Option<Spanned<Object>>,
    },
    Below {
        a: Spanned<DollarPath>,
        b: Spanned<DollarPath>,
        body: Option<Spanned<Object>>,
    },
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

    /// Function call: `name(arg1, key: arg2, ...)`
    Call {
        name: String,
        args: Vec<CallArg>,
    },
}

/// A function call argument: either positional or named.
#[derive(Debug, Clone, PartialEq)]
pub struct CallArg {
    /// `None` for positional args, `Some(name)` for named args.
    pub name: Option<Spanned<String>>,
    pub value: Spanned<Expr>,
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
    "track",
    "arc",
    "via",
    "fill",
    "text",
    "region",
    "component_body",
    "dimension",
];

/// PcbDoc named block types at top level.
pub const PCBDOC_BLOCK_TYPES: &[&str] = &["polygon", "rule", "class", "differential_pair"];

pub fn is_pcbdoc_primitive_type(s: &str) -> bool {
    PCBDOC_PRIMITIVE_TYPES.contains(&s)
}

pub fn is_pcbdoc_block_type(s: &str) -> bool {
    PCBDOC_BLOCK_TYPES.contains(&s)
}

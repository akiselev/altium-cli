# Phase 5: Spec Language Support

## Goal

Add PcbDoc support to the spec language: parse, compile, execute, reconcile, dump.

## Prerequisites

Phase 2 (read) and Phase 3 (write) must be complete. Phase 4c (dump) can be done
concurrently.

## 5a: Parser — `.pcbdoc-spec` Syntax

**New AST nodes:**
```rust
pub struct BoardDecl {
    pub name: Spanned<EntityName>,
    pub body: Vec<Spanned<BoardItem>>,
}

pub enum BoardItem {
    Property(Property),
    LetBinding(LetBinding),
    Net(NetDecl),           // reuse from SchDoc
    Component(ComponentDecl), // reuse from SchDoc
    Primitive(PcbDocPrimitiveDecl),
    Polygon(PolygonDecl),
    Rule(RuleDecl),
    Class(ClassDecl),
    Graphic(GraphicDecl),   // reuse from PcbLib
}

pub struct PcbDocPrimitiveDecl {
    pub primitive_type: Spanned<String>,  // "track", "arc", "via", etc.
    pub name: Option<Spanned<EntityName>>, // optional block-level name
    pub body: Spanned<Object>,
}
```

**Parser changes:**
- Detect `.pcbdoc-spec` extension -> parse as PcbDoc
- Add `board` keyword to top-level dispatch
- Parse primitive blocks: `track [NAME] { ... }`, `via [NAME] { ... }`, etc.
- All primitive type keywords: `track`, `arc`, `via`, `pad`, `fill`, `text`,
  `region`, `component_body`
- Reuse component/net parsing from SchDoc

**Block-level name parsing:**
```
// After parsing the type keyword, check for optional name before '{'
let name = if !self.at(&TokenKind::LBrace) {
    Some(self.parse_entity_name()?)
} else {
    None
};
```

## 5b: Model — `PcbDocSpec`

```rust
pub struct PcbDocSpec {
    pub boards: Vec<BoardSpec>,
}

pub struct BoardSpec {
    pub name: String,
    pub settings: BoardSettingsSpec,
    pub nets: Vec<NetSpec>,
    pub components: Vec<PcbDocComponentSpec>,
    pub tracks: Vec<TrackSpec>,
    pub arcs: Vec<ArcSpec>,
    pub vias: Vec<ViaSpec>,
    pub pads: Vec<PadSpec>,  // Free-standing pads (rare)
    pub fills: Vec<FillSpec>,
    pub texts: Vec<TextSpec>,
    pub regions: Vec<RegionSpec>,
    pub component_bodies: Vec<ComponentBodySpec>,
    pub polygons: Vec<PolygonSpec>,
    pub rules: Vec<RuleSpec>,
    pub classes: Vec<ClassSpec>,
}
```

Each primitive spec type has:
- `id: String` — from block-level name or auto-generated
- `position_index: usize` — positional index for reconciler fallback
- Type-specific fields

## 5c: Compiler — ID Generation

The compiler generates stable IDs using the existing `make_unique_id` pattern,
adapted for PcbDoc:

```rust
fn make_pcbdoc_id(
    &mut self,
    name: Option<&Spanned<EntityName>>,
    type_name: &str,
) -> (String, usize) {
    let counter_key = format!("{}:{}", self.context_name, type_name);
    let position = *self.unnamed_counters.entry(counter_key.clone()).or_insert(0);
    *self.unnamed_counters.get_mut(&counter_key).unwrap() += 1;

    let id = match name {
        Some(n) => n.node.as_str().to_string(),
        None => format!("{}_{}", type_name, position),
    };

    (id, position)
}
```

**Key property**: The positional counter always advances, even for named objects.
This ensures unnamed objects have stable indices regardless of named neighbors.

## 5d: Executor — Apply PcbDoc Spec

```rust
pub fn apply_spec_pcbdoc(doc: &mut PcbDoc, spec: &PcbDocSpec) -> Result<()>
```

1. Call `doc.board()` to get current state
2. For each spec board, merge into the existing board:
   - Add missing nets
   - Add/update components
   - Add/update primitives (by ID)
   - Add/update rules, classes, polygons
3. Call `doc.update_board(&merged_board)`

## 5e: Reconciler — PcbDoc Diff

```rust
pub fn reconcile_pcbdoc(
    spec: &PcbDocSpec,
    existing: &PcbDocBoard,
) -> Vec<EntityChange>
```

**Matching strategy for primitives (two-pass):**
1. Match by ID (exact)
2. For unmatched: match by positional index (rename detection)
3. Remaining spec-only: create. Remaining existing-only: preserve (additive).

**Matching strategy for named collections:**
- Nets: match by name
- Components: match by designator
- Rules: match by name
- Classes: match by name
- Polygons: match by name/ID

## Estimated Scope

- 5a (parser): ~150 lines
- 5b (model): ~200 lines
- 5c (compiler): ~300 lines
- 5d (executor): ~200 lines
- 5e (reconciler): ~400 lines
- Total: ~1,250 lines

# Plan: `pin -> #NET` Syntax + Validated Symbol References

## Overview

Add component-level pin-to-net connection syntax (`pin GPIO4 -> #SDA`) and compile-time validated symbol references (`symbol: $mcu.ESP32_C6`) to the schdoc spec language in altium-cli. The compiler parses pin connections into a model-level `PinConnectionSpec`, and the executor resolves pin positions from imported SchLib data to generate wire stubs, net labels, power symbols, and no-connect markers. Symbol references via import aliases gain provenance tracking and validation against the imported SchLib.

## Decision Log

| Decision | Reasoning | Backing |
|---|---|---|
| `#` prefix for net names | Unambiguous — doesn't conflict with `#RRGGBB` colors (requires 6 hex digits) or `#[...]` annotations (requires `[`). Lexes as `Hash` + `Ident`, two tokens. | user-specified |
| Implicit signal nets | Any `#NAME` not matching a `power` declaration auto-creates a net label. Only power nets need top-level declaration. Reduces boilerplate — most nets are signals. | user-specified |
| Pin lookup: name first, then designator | Try matching pin name (GPIO4, VDD) in SchLib, fall back to designator (1, 2, A14). Most ergonomic — pin names are human-readable identifiers. | user-specified |
| Executor-time resolution | `PinConnectionSpec` stays in model; executor resolves pin positions at apply time. Better separation of concerns — model preserves semantic intent. Round-trip dump is deferred (not in scope). | user-specified |
| 200mil default stub length | Standard Altium schematic grid spacing. Enough room for a net label without crowding adjacent components. | user-specified |
| `nc` keyword for no-connect | `pin X -> nc` (no `#` prefix) generates NoConnect marker at pin tip. Short, matches common schematic convention. Not a net name, so no `#` prefix. | user-specified |
| `->` as new `Arrow` token | Avoids ambiguity with `a - >b` arithmetic expressions. Follows `DotDotDot` lexer pattern for multi-char tokens (peek-ahead in lexer). | default-derived |
| Missing `#` is hard parse error | `pin X -> SDA` (no `#`) is an error, not a warning. The `#` prefix is the user's explicit design choice for unambiguous net identification. Silent fallback would mask typos. | user-specified |
| Power declaration syntax (existing) | `power GND { style: gnd_power }` — already parsed by `TokenKind::Power` / `PowerDecl` in the existing codebase. No new syntax needed for power declarations. | doc-derived |
| Transform order: mirror then rotate | Pin orientation transform: (1) apply mirror (flip 0↔180), then (2) add placement rotation mod 4. Matches existing `transform_pin_position` implementation at compiler.rs:207-217. Must not be reversed. | doc-derived |
| `Value::ImportRef` for provenance | When `$alias.field` evaluates on a named import object, return `Value::ImportRef { alias, name }` instead of `Value::String`. Preserves import context through eval so compiler can validate and produce `SymbolRef::Import`. | user-specified (Option A) |
| Validated vs soft references | `symbol: $mcu.ESP32_C6` is strictly validated (error if not found in imported SchLib). `lib_reference: "Name"` remains a soft string reference for backward compatibility. | default-derived |
| SchLib HashMap contract | `HashMap<String, ComponentSpec>` keyed by `lib_reference` (component name string). CLI loads all imported SchLibs eagerly via existing `compile_imported_schlibs`. Tests construct map inline using existing `make_test_component` pattern. | doc-derived |
| Power symbol orientation | Inferred from stub direction. GND symbols orient to point away from circuit (270° for downward stub). VCC/bar symbols orient toward the rail (90° for upward stub). Horizontal stubs use 0°/180° matching direction. | default-derived |
| Non-90° rotation | Not possible — `RotationBy90` Rust type only allows 0/90/180/270. Enforced at type level, not a runtime concern. | doc-derived |
| power_declarations threading | Pass `power_declarations` as a parameter to `compile_schdoc_component`, not stored on `SpecCompiler` state. Power declarations are per-sheet scoped — `compile_schdoc` iterates sheets and has per-sheet declarations at call time. Storing on compiler state risks cross-sheet leakage in multi-sheet designs. | default-derived |
| IndexMap for ImportObject.entries | Consistent with existing `Value::Object` type which uses `IndexMap` — maintains insertion-order iteration for deterministic diagnostic output (e.g., "available: ESP32_C6, ESP32_S3, ..."). | doc-derived |

## Invisible Knowledge

### Pin Orientation Transform

Pins in SchLib have an `orientation` field (`RotationBy90`: 0/90/180/270) indicating which direction the pin extends FROM the component body toward the connection point. When a component is placed with rotation and/or mirror, the pin orientation transforms:

1. **Mirror** (if `is_mirrored`): flip horizontally — orientation 0 (right) becomes 180 (left) and vice versa. 90 and 270 unchanged.
2. **Rotate**: add component rotation to pin orientation, modulo 4 (i.e., modulo 360°).

The wire stub continues in the pin's **transformed** orientation direction. This is the same transform order used by `transform_pin_position` in the existing codebase.

Example: pin orientation 0° (right) + component mirror=true + rotation=90° → mirror flips to 180° (left) → +90° = 270° (down). Stub extends downward.

### Power Symbol Orientation Convention

Altium power symbols have their own orientation independent of the wire they connect to. Convention:
- **Bar/Arrow styles** (VCC, +3V3): symbol points "toward the rail" — perpendicular to wire stub, oriented upward (90°) by default. For horizontal stubs, rotated to match.
- **Ground styles** (GND, AGND): symbol points "away from circuit" — for a downward stub the GND symbol is at 270° (pointing down). For stubs in other directions, the symbol rotates to always point away.

The mapping from stub direction to power symbol orientation:
| Stub direction | VCC-style orientation | GND-style orientation |
|---|---|---|
| Right (0°) | 0° | 0° |
| Up (90°) | 90° | 90° |
| Left (180°) | 180° | 180° |
| Down (270°) | 270° | 270° |

Power objects in Altium orient the same as the direction they "face." The orientation field on `PowerObject` directly corresponds to the stub direction.

### NetLabel Orientation Convention

Altium's `NetLabel` record (RECORD=25) has an `orientation` field using `RotationBy90`. The orientation controls the text rotation:
- **0° (Rotate0)**: text reads left-to-right, label extends horizontally to the right from its location point
- **90° (Rotate90)**: text reads bottom-to-top, label extends vertically upward
- **180° (Rotate180)**: text reads right-to-left, label extends horizontally to the left
- **270° (Rotate270)**: text reads top-to-bottom, label extends vertically downward

For wire stub generation, the NetLabel orientation should match the stub direction so the label text reads naturally along the wire:

| Stub direction | NetLabel orientation | Text reads |
|---|---|---|
| Right (0°) | 0° | left-to-right, extending right from stub end |
| Up (90°) | 90° | bottom-to-top, extending upward from stub end |
| Left (180°) | 0° | left-to-right, placed at stub end (label extends left from its anchor, but text is still L-to-R) |
| Down (270°) | 90° | bottom-to-top, placed at stub end |

Note: for leftward and downward stubs, the label orientation may need adjustment so text remains readable. The standard Altium convention is to keep labels at 0° or 90° (never 180°/270°) for readability, and use the `justification` field to control anchor direction. Implementation should follow this convention: use 0° for horizontal stubs (both left and right), 90° for vertical stubs (both up and down).

### Why Executor-Time Resolution

`PinConnectionSpec` is a first-class model type that survives compilation. The executor needs access to imported SchLib `ComponentSpec` data (pin positions, orientations, names, designators) to generate wire stubs. This requires threading the `imported_components` HashMap from the CLI layer through to `apply_spec_schdoc`, which currently doesn't receive it.

Round-trip dump (emitting `pin X -> #NET` from a SchDoc) is not in scope for this plan. M4 dumps pin connections as low-level wire/label objects. Future work can reconstruct the high-level syntax from proximity heuristics.

### Import Provenance Tracking

The eval layer currently resolves `$mcu.ESP32_C6` to a plain `Value::String("ESP32_C6")`, losing the information that it came from the `mcu` import alias. By adding `Value::ImportRef { alias, name }`, the compiler can:
1. Know which import the reference came from (for error messages: "not found in import 'mcu'")
2. Produce `SymbolRef::Import { alias, name }` instead of `SymbolRef::Literal`
3. Validate the name exists in the imported SchLib's component list

This is a targeted change to `eval_field_access` — only when the base value is from a named import object (registered in `named_import_objects`) does it return `ImportRef` instead of `String`. All other field access paths are unchanged.

## Milestones

### Milestone 1: Lexer + AST + Parser

**Files**:
- `crates/autopcb-spec/src/lexer.rs`
- `crates/autopcb-spec/src/ast.rs`
- `crates/autopcb-spec/src/parser.rs`

**Flags**: `conformance`

**Requirements**:
- Add `Arrow` token kind (`->`) to lexer, lexed as single token via peek-ahead on `-`
- Add `PinConnectionTarget` enum to AST: `NetRef(Spanned<String>)` for `#NAME`, `NoConnect` for `nc`
- Add `PinConnectionDecl` struct to AST with `pin_name: Spanned<String>` and `target: PinConnectionTarget`
- Add `PinConnection(PinConnectionDecl)` variant to `ComponentItem` enum
- Parse `pin IDENT -> #IDENT` and `pin INTEGER -> #IDENT` inside schdoc component body
- Parse `pin IDENT -> nc` for no-connect
- Pin name accepts bare identifiers or integers (for designator-style names)
- When `pin` keyword is followed by ident/integer then `Arrow`, parse as PinConnection; otherwise fall through to existing pin block parsing (backward compatible)
- `pin GPIO4 -> SDA` (missing `#`) produces clear parse error: "expected '#' before net name"

**Acceptance Criteria**:
- `pin GPIO4 -> #SDA` parses to `PinConnection { pin: "GPIO4", target: NetRef("SDA") }`
- `pin 1 -> #VCC` parses with integer pin designator
- `pin NC1 -> nc` parses to `PinConnection { pin: "NC1", target: NoConnect }`
- `pin 1 { at: (100mil, 0mil) }` still parses as pin block (backward compat)
- `pin GPIO4 -> SDA` errors with "expected '#' before net name"
- Arrow token does not break existing `-` minus usage in expressions

**Tests**:
- **Test files**: `crates/autopcb-spec/src/parser.rs` (inline `#[test]` in existing test module)
- **Test type**: unit (example-based, following existing codebase convention)
- **Backing**: default-derived
- **Scenarios**:
  - Normal: parse `pin GPIO4 -> #SDA`, `pin 1 -> #VCC`, `pin NC1 -> nc`
  - Edge: pin name that matches a keyword (e.g., `pin net -> #CLK`), integer pin name
  - Error: missing `#` produces parse error, missing `->` falls through to pin block
  - Regression: existing pin block syntax unchanged, minus in expressions unchanged

**Code Intent**:
- `lexer.rs`: Add `Arrow` variant to `TokenKind` enum. In the `b'-'` match arm (currently emits `Minus`), peek next byte; if `>`, emit `Arrow` and advance by 2, else emit `Minus`. Add `Arrow` to `same_variant()` match.
- `ast.rs`: Add `PinConnectionTarget` enum with `NetRef(Spanned<String>)` and `NoConnect` variants. Add `PinConnectionDecl` struct with fields `pin_name: Spanned<String>`, `target: PinConnectionTarget`. Add `PinConnection(PinConnectionDecl)` variant to `ComponentItem` enum.
- `parser.rs`: In `parse_component_item` (the function that dispatches on the next token inside a component body), add a branch: when current token is `Pin`, peek ahead — if the token after the pin name/integer is `Arrow`, parse as pin connection. Consume `Pin`, read pin name (ident or integer as string), consume `Arrow`, then dispatch on next token: if `Hash` consume it and read `Ident` for net name (wrap as `NetRef`); if `Ident("nc")` produce `NoConnect`; if bare `Ident` (not `nc`) produce error "expected '#' before net name". Return `ComponentItem::PinConnection(decl)`.

**Code Changes**:

```diff
--- a/crates/autopcb-spec/src/lexer.rs
+++ b/crates/autopcb-spec/src/lexer.rs
@@ -69,6 +69,7 @@ pub enum TokenKind {
     Dot,
     DotDotDot,
     Eq,
+    Arrow,
     Plus,
     Minus,
     Star,
@@ -129,6 +130,7 @@ impl TokenKind {
                 | (DotDotDot, DotDotDot)
                 | (Eq, Eq)
                 | (Plus, Plus)
+                | (Arrow, Arrow)
                 | (Minus, Minus)
                 | (Star, Star)
                 | (Slash, Slash)
@@ -253,8 +255,13 @@ pub fn lex(input: &str) -> Result<(Vec<Token>, Vec<CommentToken>), ParseError> {
             b'+' => {
                 out.push(tok(TokenKind::Plus, i, i + 1));
                 i += 1;
             }
             b'-' => {
-                out.push(tok(TokenKind::Minus, i, i + 1));
-                i += 1;
+                if peek_byte(bytes, i + 1) == Some(b'>') {
+                    out.push(tok(TokenKind::Arrow, i, i + 2));
+                    i += 2;
+                } else {
+                    out.push(tok(TokenKind::Minus, i, i + 1));
+                    i += 1;
+                }
             }
             b'*' => {
```

```diff
--- a/crates/autopcb-spec/src/ast.rs
+++ b/crates/autopcb-spec/src/ast.rs
@@ -91,6 +91,7 @@ pub enum ComponentItem {
     Property(Property),
     LetBinding(LetBinding),
     Part(PartBlock),
+    PinConnection(PinConnectionDecl),
     Pin(PinDecl),
     Parameter(ParameterDecl),
     Alias(AliasDecl),
@@ -103,6 +104,25 @@ pub enum ComponentItem {
     SwapGroup(SwapGroupDecl),
 }

+/// Target of a pin connection declaration: `pin X -> #NET` or `pin X -> nc`.
+#[derive(Debug, Clone, PartialEq)]
+pub enum PinConnectionTarget {
+    /// `#NAME` — a signal or power net reference.
+    NetRef(Spanned<String>),
+    /// `nc` — a no-connect marker.
+    NoConnect,
+}
+
+/// `pin NAME -> #NET` or `pin NAME -> nc` inside a schdoc component body.
+#[derive(Debug, Clone, PartialEq)]
+pub struct PinConnectionDecl {
+    /// The pin name or designator (e.g. `GPIO4`, `1`).
+    pub pin_name: Spanned<String>,
+    /// The connection target.
+    pub target: PinConnectionTarget,
+}
+
 /// [binding =] part N { ... }
 #[derive(Debug, Clone, PartialEq)]
 pub struct PartBlock {
```

```diff
--- a/crates/autopcb-spec/src/parser.rs
+++ b/crates/autopcb-spec/src/parser.rs
@@ -3,7 +3,7 @@ use super::ast::{
     AliasDecl, AnnotationBlockDecl, AnnotationKey, BlockAnnotation, BoardDecl, BoardItem,
     ClassDecl, ComparisonRuleDecl, ComponentDecl, ComponentItem, ConstraintDecl, ConstraintKind,
     DifferentialPairDecl, DocumentBlockDecl, EntityName, EntryDecl, ErcLevelEntryDecl,
-    ErcMatrixEntryDecl, Expr, FontBlockDecl, FontDecl, FootprintDecl, FootprintItem,
+    ErcMatrixEntryDecl, Expr, FontBlockDecl, FontDecl, FootprintDecl, FootprintItem, PinConnectionDecl, PinConnectionTarget,
     FootprintMapDecl, FootprintRef, GraphicDecl, GridDecl, ImportDecl, LetBinding,
     MatchParameterDecl, NetDecl, Object, ObjectItem, OutputBlockDecl, OutputGroupBlockDecl,
     PadDecl, ParamVariationDecl, ParameterDecl, PartBlock, PartItem, PcbDocPrimitiveDecl, PinDecl,
@@ -571,6 +571,47 @@ impl<'a> SpecParser<'a> {
         // pin declaration
         if self.at(&TokenKind::Pin) {
+            // Peek ahead: Pin IDENT Arrow | Pin Integer Arrow → pin connection
+            // Pin IDENT/Integer LBrace → pin block declaration (not a pin connection)
+            let pin_name_offset = 1;
+            let after_name_offset = 2;
+            let is_pin_connection = {
+                let after_pin = self.peek_ahead(pin_name_offset);
+                let name_is_scalar = matches!(
+                    after_pin,
+                    TokenKind::Ident(_) | TokenKind::Integer(_)
+                );
+                name_is_scalar
+                    && self.peek_ahead(after_name_offset).same_variant(&TokenKind::Arrow)
+            };
+            if is_pin_connection {
+                let start = self.current_span();
+                self.bump(); // consume `pin`
+                let pin_name_str = match self.current_kind().clone() {
+                    TokenKind::Ident(s) => {
+                        let span = self.current_span();
+                        self.bump();
+                        Spanned::new(s, span)
+                    }
+                    TokenKind::Integer(n) => {
+                        let span = self.current_span();
+                        self.bump();
+                        Spanned::new(n.to_string(), span)
+                    }
+                    _ => unreachable!("guarded by is_pin_connection check"),
+                };
+                self.expect(&TokenKind::Arrow, "expected '->'")?;
+                let target = match self.current_kind().clone() {
+                    TokenKind::Hash => {
+                        self.bump(); // consume `#`
+                        let net_name = self.expect_ident("expected net name after '#'")?;
+                        PinConnectionTarget::NetRef(net_name)
+                    }
+                    TokenKind::Ident(ref s) if s == "nc" => {
+                        self.bump();
+                        PinConnectionTarget::NoConnect
+                    }
+                    TokenKind::Ident(_) => {
+                        return Err(self.err("expected '#' before net name"));
+                    }
+                    _ => {
+                        return Err(self.err("expected '#NET' or 'nc' after '->'"));
+                    }
+                };
+                let end = self.prev_span();
+                let decl = PinConnectionDecl { pin_name: pin_name_str, target };
+                return Ok(Spanned::new(ComponentItem::PinConnection(decl), start.merge(end)));
+            }
             let decl = self.parse_pin(None)?;
             let end = self.prev_span();
             return Ok(Spanned::new(ComponentItem::Pin(decl), start.merge(end)));
@@ -714,7 +756,7 @@ impl<'a> SpecParser<'a> {

         Err(self.err(
-            "expected component item (property, pin, parameter, alias, footprint, part, graphic, swap_group, or let binding)",
+            "expected component item (property, pin, parameter, alias, footprint, part, graphic, swap_group, pin connection, or let binding)",
         ))
     }
```

---

### Milestone 2: Model + Compiler + Validated Symbol References

**Files**:
- `crates/autopcb-spec/src/model.rs`
- `crates/autopcb-spec/src/compiler.rs`
- `crates/autopcb-spec/src/eval.rs`

**Flags**: `conformance`, `needs-rationale`

**Requirements**:
- Add `PinConnectionSpec` model type with `pin_name: String` and `target: PinConnectionTarget`
- Add model-level `PinConnectionTarget` enum: `Signal(String)`, `Power(String)`, `NoConnect`
- Add `pin_connections: Vec<PinConnectionSpec>` field to `SchDocComponentSpec`
- Add `power_declarations: HashMap<String, PowerObjectStyle>` to `SheetSpec`
- Compiler collects top-level `power` declarations into `power_declarations` map
- Compiler extracts `PinConnection` items from component body and compiles into `PinConnectionSpec`
- Net name matching against `power_declarations` determines `Signal` vs `Power` target
- Add `Value::ImportRef { alias: String, name: String }` variant to `Value` enum in eval.rs
- When `eval_field_access` resolves a field on a value that came from a named import object, return `ImportRef` instead of `String`
- Compiler pattern-matches `ImportRef` in symbol resolution to produce `SymbolRef::Import { alias, name }`
- Compiler validates that `name` exists in `imported_components` when `ImportRef` is used
- Error on validation failure: "symbol '{name}' not found in import '{alias}' (available: {list})"
- `lib_reference: "Name"` path remains as `SymbolRef::Literal` with no strict validation (backward compat)

**Acceptance Criteria**:
- Component with `pin GPIO4 -> #SDA` compiles to `SchDocComponentSpec` with `pin_connections` containing `PinConnectionSpec { pin: "GPIO4", target: Signal("SDA") }`
- Component with `pin VDD -> #3V3` where `power +3V3 { style: bar }` declared → `target: Power("3V3")`
- Component with `pin NC1 -> nc` → `target: NoConnect`
- `symbol: $mcu.ESP32_C6` with ESP32_C6 in imported SchLib → `SymbolRef::Import { alias: "mcu", name: "ESP32_C6" }`, no error
- `symbol: $mcu.TYPO` with TYPO not in imported SchLib → compile error with available names
- `lib_reference: "ESP32-C6"` → `SymbolRef::Literal("ESP32-C6")`, no validation error
- Components without pin connections compile as before (empty `pin_connections` vec)
- Existing `$alias.field` usage in non-import contexts (e.g., component bindings) unaffected

**Tests**:
- **Test files**: `crates/autopcb-spec/src/compiler.rs` (inline `#[test]` in existing test module)
- **Test type**: unit (example-based)
- **Backing**: default-derived
- **Scenarios**:
  - Normal: compile signal/power/nc pin connections, compile validated symbol ref
  - Edge: component with mix of properties and pin connections, symbol ref from bare import (no alias)
  - Error: symbol name not found in import (with helpful error listing available names)
  - Regression: existing `lib_reference` string path unchanged, existing `$alias.field` eval unchanged

**Code Intent**:
- `eval.rs`: Add `ImportRef { alias: String, name: String }` variant to `Value` enum. Add `ImportObject { alias: String, entries: IndexMap<String, Value> }` variant to `Value` enum. In `eval_field_access` (the function handling `expr.field`), add a match arm for `Value::ImportObject { alias, entries }`: look up the field in `entries`, and if it resolves to a `Value::String(name)`, return `Value::ImportRef { alias: alias.clone(), name }` instead. All other field access paths (on `Value::Object`, `Value::CoordPoint`, etc.) remain unchanged.
- `compiler.rs` (line 108, function `compile_spec_with_resolved`): Change the existing `compiler.named_import_objects.insert(alias.clone(), Value::Object(entries))` to `compiler.named_import_objects.insert(alias.clone(), Value::ImportObject { alias: alias.clone(), entries })`. This is the critical change that enables provenance tracking — without it, `$mcu.ESP32_C6` continues resolving to `Value::String` and the `ImportRef` path is never triggered.
- `model.rs`: Add `PinConnectionTarget` enum (`Signal(String)`, `Power(String)`, `NoConnect`). Add `PinConnectionSpec` struct (`pin_name: String`, `target: PinConnectionTarget`). Add `pin_connections: Vec<PinConnectionSpec>` to `SchDocComponentSpec`. Add `power_declarations: HashMap<String, PowerObjectStyle>` to `SheetSpec`.
- `compiler.rs`: In `compile_schdoc`, collect `Power` declarations into a local `power_declarations: HashMap<String, PowerObjectStyle>`. Store it on `SheetSpec`. Pass `power_declarations` as a parameter to `compile_schdoc_component` (not on compiler state — power declarations are per-sheet scoped; storing on compiler state risks cross-sheet leakage). In `compile_schdoc_component`, iterate `ComponentItem::PinConnection` variants from the AST body — for each, look up net name against the passed `power_declarations` to determine `Signal("name")` vs `Power("name")`, or `NoConnect`. Build `PinConnectionSpec` and add to vec. For symbol resolution: match `Value::ImportRef { alias, name }` → produce `SymbolRef::Import { alias, name }`, validate `name` exists in `self.imported_components`, error with available names if not found.

**Code Changes**:

```diff
--- a/crates/autopcb-spec/src/eval.rs
+++ b/crates/autopcb-spec/src/eval.rs
@@ -148,6 +148,14 @@ pub enum Value {
     Array(Vec<Value>),
     Object(IndexMap<String, Value>),
     /// A declared swap group reference; the inner string is the group's entity name.
     SwapGroup(String),
+    /// An import object — maps entity names to their string names.
+    /// Stores provenance (alias) so field access can return ImportRef.
+    ImportObject { alias: String, entries: IndexMap<String, Value> },
+    /// A resolved `$alias.Name` reference; carries the import alias for error
+    /// reporting and symbol validation.
+    ImportRef { alias: String, name: String },
 }

 impl Value {
@@ -173,6 +181,8 @@ impl Value {
             Value::Object(map) => {
                 let items: Vec<_> = map
                     .iter()
                     .map(|(k, v)| format!("{k}: {}", v.display()))
                     .collect();
                 format!("{{{}}}", items.join(", "))
             }
             Value::SwapGroup(s) => s.clone(),
+            Value::ImportObject { alias, .. } => format!("<import:{alias}>"),
+            Value::ImportRef { alias, name } => format!("{alias}.{name}"),
         }
     }
@@ -195,6 +205,8 @@ impl Value {
             Value::Array(_) => "array",
             Value::Object(_) => "object",
             Value::SwapGroup(_) => "swap_group",
+            Value::ImportObject { .. } => "import_object",
+            Value::ImportRef { .. } => "import_ref",
         }
     }
@@ -225,6 +237,8 @@ impl Value {
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
+            Value::ImportObject { entries, .. } => Ok(entries),
             other => Err(SpecError::new(
                 SpecErrorCode::NotAnObject,
                 format!("expected object, got {}", other.kind_name()),
                 span,
             )),
         }
     }
 }
@@ -689,6 +703,16 @@ fn eval_field_access(base: Value, field: &str, span: Option<Span>) -> EvalResult
     match base {
         Value::Object(map) => {
             map.get(field).cloned().ok_or_else(|| SpecError::new(
                 SpecErrorCode::InvalidFieldAccess,
                 format!("no field '{field}' on object"),
                 span,
             ))
         }
+        Value::ImportObject { alias, entries } => {
+            match entries.get(field) {
+                Some(Value::String(name)) => Ok(Value::ImportRef { alias, name: name.clone() }),
+                Some(other) => Ok(other.clone()),
+                None => Err(SpecError::new(
+                    SpecErrorCode::InvalidFieldAccess,
+                    format!("no entity '{field}' in import '{alias}'"),
+                    span,
+                )),
+            }
+        }
         Value::CoordPoint(x, y) => match field {
```

```diff
--- a/crates/autopcb-spec/src/model.rs
+++ b/crates/autopcb-spec/src/model.rs
@@ -96,6 +96,34 @@ pub struct PinPadMap {
     pub pad: String,
 }

+// ── Pin connection model types ───────────────────────────────────────────────
+
+/// Target of a compiled pin connection.
+#[derive(Debug, Clone)]
+pub enum PinConnectionTarget {
+    /// A signal net — generates a NetLabel.
+    Signal(String),
+    /// A power net — generates a PowerObject (style from power_declarations).
+    Power(String),
+    /// No-connect — generates a NoConnect marker, no wire stub.
+    NoConnect,
+}
+
+/// A compiled pin-to-net connection for a placed SchDoc component.
+#[derive(Debug, Clone)]
+pub struct PinConnectionSpec {
+    /// Pin name or designator to look up in the SchLib ComponentSpec.
+    pub pin_name: String,
+    /// The resolved connection target.
+    pub target: PinConnectionTarget,
+}
+
 // ── SchDoc ──────────────────────────────────────────────────────────────────

 pub struct SchDocSpec {
@@ -102,6 +130,9 @@ pub struct SheetSpec {
     pub annotation: Option<CompiledAnnotation>,
     // Sheet metadata
     pub fonts: Vec<FontSpec>,
+    /// Power net declarations collected from top-level `power` items.
+    /// Keyed by net name (without `#` prefix). Used by executor for stub generation.
+    pub power_declarations: std::collections::HashMap<String, PowerObjectStyle>,
     pub custom_width: Option<Coord>,
     pub custom_height: Option<Coord>,
     pub snap_grid_on: Option<bool>,
@@ -153,6 +184,8 @@ pub struct SchDocComponentSpec {
     pub is_mirrored: Option<bool>,
     pub description: Option<String>,
     pub parameters: Vec<ParameterSpec>,
+    /// Pin-to-net connections declared with `pin X -> #NET` syntax.
+    pub pin_connections: Vec<PinConnectionSpec>,
 }
```

```diff
--- a/crates/autopcb-spec/src/compiler.rs
+++ b/crates/autopcb-spec/src/compiler.rs
@@ -104,7 +104,7 @@ pub fn compile_spec_with_resolved(
             if let Some(name) = name {
                 entries.insert(name.clone(), Value::String(name));
             }
         }
-        compiler.named_import_objects.insert(alias.clone(), Value::Object(entries));
+        // ImportObject stores the alias with entries so field access on `$alias.Name`
+        // returns ImportRef, preserving import provenance for symbol validation.
+        compiler.named_import_objects.insert(alias.clone(), Value::ImportObject { alias: alias.clone(), entries });
     }
     compiler.compile(&resolved.root)
 }
@@ -518,6 +518,17 @@ impl SpecCompiler {
     fn compile_schdoc(&mut self, file: &SpecFile) -> Result<SchDocSpec, SpecError> {
         let mut sheet_annotation: Option<CompiledAnnotation> = None;
         let mut fonts = Vec::new();
+        // Pre-pass: collect power net names so pin-connection classification
+        // (Signal vs Power) is order-independent.
+        let mut power_declarations: std::collections::HashMap<String, PowerObjectStyle> =
+            std::collections::HashMap::new();
+        for item in &file.items {
+            if let SpecItem::Power(power_decl) = &item.node {
+                // Placeholder — final style is resolved after all power declarations compile.
+                power_declarations.insert(power_decl.name.node.as_str(), PowerObjectStyle::Bar);
+            }
+        }
         let mut custom_width = None;
         let mut custom_height = None;
         let mut snap_grid_on = None;
@@ -548,7 +565,7 @@ impl SpecCompiler {
                 }
                 SpecItem::Component(comp_decl) => {
-                    let comp = self.compile_schdoc_component(comp_decl)?;
+                    let comp = self.compile_schdoc_component(comp_decl, &power_declarations)?;
                     let binding = build_component_binding(&comp, &self.imported_components);
                     self.scope.define(comp.designator.clone(), binding);
                     components.push(comp);
@@ -570,6 +587,12 @@ impl SpecCompiler {
         }
+        // power_declarations: names from pre-pass, styles from this loop.
+        // Styles were unavailable during pre-pass (power items not yet compiled).
+        for power in &powers {
+            power_declarations.insert(power.name.clone(), power.style);
+        }
+
         let sheet = SheetSpec {
             annotation: sheet_annotation,
             fonts,
+            power_declarations,
             custom_width,
             custom_height,
             snap_grid_on,
@@ -689,7 +707,8 @@ impl SpecCompiler {

     fn compile_schdoc_component(
         &mut self,
         decl: &ComponentDecl,
-    ) -> Result<SchDocComponentSpec, SpecError> {
+        power_declarations: &std::collections::HashMap<String, PowerObjectStyle>,
+    ) -> Result<SchDocComponentSpec, SpecError> {
         let annotation = self.compile_opt_annotation(decl.annotation.as_ref())?;
         let designator = decl.name.node.as_str();
@@ -717,6 +736,26 @@ impl SpecCompiler {
         // Resolve symbol reference: either $alias.Name or lib_reference: "Name"
         let symbol = if let Some(v) = props.get("symbol") {
             match v {
+                Value::ImportRef { alias, name } => {
+                    let alias = alias.clone();
+                    let name = name.clone();
+                    if !self.imported_components.contains_key(&name) {
+                        let available: Vec<String> = self.imported_components
+                            .keys()
+                            .cloned()
+                            .collect();
+                        return Err(SpecError::no_span(
+                            SpecErrorCode::AltiumFormat,
+                            format!(
+                                "symbol '{}' not found in import '{}' (available: {})",
+                                name,
+                                alias,
+                                available.join(", ")
+                            ),
+                        ));
+                    }
+                    SymbolRef::Import { alias, name }
+                }
                 Value::String(s) => {
                     // Plain lib_reference string — no import validation; treated as opaque component name.
                     SymbolRef::Literal(s.clone())
@@ -762,6 +801,28 @@ impl SpecCompiler {

         self.scope.pop();

+        // Compile pin connections
+        let mut pin_connections = Vec::new();
+        for item in &decl.body {
+            if let ComponentItem::PinConnection(conn_decl) = &item.node {
+                let target = match &conn_decl.target {
+                    crate::ast::PinConnectionTarget::NoConnect => {
+                        crate::model::PinConnectionTarget::NoConnect
+                    }
+                    crate::ast::PinConnectionTarget::NetRef(net_name) => {
+                        let name = net_name.node.clone();
+                        if power_declarations.contains_key(&name) {
+                            crate::model::PinConnectionTarget::Power(name)
+                        } else {
+                            crate::model::PinConnectionTarget::Signal(name)
+                        }
+                    }
+                };
+                pin_connections.push(crate::model::PinConnectionSpec {
+                    pin_name: conn_decl.pin_name.node.clone(),
+                    target,
+                });
+            }
+        }
+
         Ok(SchDocComponentSpec {
             annotation,
             designator,
@@ -770,6 +821,7 @@ impl SpecCompiler {
             is_mirrored,
             description,
             parameters,
+            pin_connections,
         })
     }
```

---

### Milestone 3: Executor — Wire Stub Generation

**Files**:
- `crates/autopcb-spec/src/executor.rs`

**Flags**: `complex-algorithm`, `needs-rationale`

**Requirements**:
- `apply_spec_schdoc` gains `imported_components: &HashMap<String, ComponentSpec>` parameter
- For each component's `pin_connections`, resolve pin name against the SchLib `ComponentSpec` (match pin `name` field first, then `designator` field)
- Compute pin tip position in schematic space via existing `transform_pin_position`
- Compute pin orientation in schematic space: mirror flips 0↔180, then add component rotation mod 4
- Generate wire stub: `Wire` with vertices `[pin_tip, pin_tip + 200mil in outward direction]`
- Generate `NetLabel` at stub endpoint for signal nets
- Generate `PowerObject` at stub endpoint for power nets (style from `power_declarations`)
- Generate `NoConnect` at pin tip for nc connections (no wire stub)
- Power object orientation matches stub direction

**Acceptance Criteria**:
- Component at (1000mil, 500mil), pin at symbol-space (200mil, 0) orientation 0° → wire from (1200mil, 500mil) to (1400mil, 500mil), NetLabel "SDA" at (1400mil, 500mil) orientation 0°
- Same pin with component rotated 90° → wire extends upward, NetLabel at top
- Same pin with component mirrored → wire extends leftward (-X direction)
- Mirror + Rotate 90°: pin orient 0° → mirror to 180° → +90° = 270° → stub extends downward
- Mirrored component (leftward stub, transformed_orient=180°) → NetLabel orientation is Rotate0 (not Rotate180); label text reads left-to-right
- Downward stub (transformed_orient=270°) → NetLabel orientation is Rotate90 (not Rotate270); label text reads bottom-to-top
- Power `#GND` with `style: gnd_power` → PowerObject style=gnd_power at stub end, orientation matches stub direction
- `pin X -> nc` → NoConnect marker at pin tip, no Wire generated
- Pin name "GPIO4" matches SchLib pin with `name: Some("GPIO4")`
- Pin name "1" matches SchLib pin with `designator: "1"` (fallback when name doesn't match)
- Pin name "NONEXISTENT" → error: "pin 'NONEXISTENT' not found in symbol 'ESP32_C6' (available pins: GPIO4, GPIO5, VDD, GND, ...)"
- Component without imported SchLib (lib_reference string, no import) → pin connections produce error: "cannot resolve pin connections for 'U1': symbol not found in imported libraries"

**Tests**:
- **Test files**: `crates/autopcb-spec/src/executor.rs` (inline `#[test]` in existing test module)
- **Test type**: unit (example-based)
- **Backing**: default-derived
- **Scenarios**:
  - Normal: signal stub at orientation 0°, power stub with gnd_power style, nc marker
  - Rotation: all 4 orientations (0°, 90°, 180°, 270°) produce correct stub directions
  - Mirror: mirrored component flips stub direction
  - Mirror+Rotation: pin orient 0° + mirror + rot 90° → stub direction 270°
  - Pin lookup: match by name, fallback to designator
  - Error: pin name not found (with available pins list), symbol not in imports

**Code Intent**:
- `executor.rs`: Change `apply_spec_schdoc` signature to accept `imported_components: &HashMap<String, ComponentSpec>`. Pass `sheet_spec.power_declarations` alongside. After placing components (step 2), add step 2.5: for each component with non-empty `pin_connections`, call new `fn generate_pin_connection_stubs`. This function:
  1. Resolves the component's `SymbolRef` to a `ComponentSpec` from `imported_components`
  2. For each `PinConnectionSpec`, calls `fn resolve_pin(lib_comp: &ComponentSpec, pin_name: &str) -> Result<&PinSpec, SpecError>` — tries `pin.name == Some(pin_name)` first, then `pin.designator == pin_name`
  3. Calls `transform_pin_position` for the pin tip location
  4. Calls new `fn transform_pin_orientation(pin_orient: RotationBy90, comp_orient: RotationBy90, is_mirrored: bool) -> RotationBy90` — if mirrored and orient is 0 return 180 (and vice versa), then add comp rotation mod 4
  5. Computes stub endpoint: tip + 200mil in the direction indicated by transformed orientation (0°→+X, 90°→+Y, 180°→-X, 270°→-Y)
  6. For `Signal(name)`: generates `Wire { vertices: [tip, endpoint] }` + `NetLabel { text: name, location: endpoint, orientation: remap_label_orient(transformed_orient) }` where `remap_label_orient` returns `Rotate0` for 0°/180° and `Rotate90` for 90°/270° (per Invisible Knowledge NetLabel convention — labels are always 0° or 90° for readability, never 180°/270°)
  7. For `Power(name)`: generates `Wire { vertices: [tip, endpoint] }` + `PowerObject { text: name, location: endpoint, orientation: transformed_orient, style: power_declarations[name] }` (power objects use the raw transformed orientation — their symbols are designed to render correctly at all 4 orientations)
  8. For `NoConnect`: generates `NoConnect { location: tip }`
  9. Adds all generated objects to sheet via `sheet.add_object()`
- Update all call sites of `apply_spec_schdoc` to pass the new parameter (may be empty HashMap for non-schdoc paths).

**Code Changes**:

```diff
--- a/crates/autopcb-spec/src/executor.rs
+++ b/crates/autopcb-spec/src/executor.rs
@@ -23,7 +23,7 @@ use crate::model::{
     BoardSpec, ComponentSpec, FootprintMapSpec, FootprintSpec, GraphicSpec, GraphicType, LayerSpec,
     PadSpec, ParameterSpec, PcbDocComponentSpec, PcbDocNetSpec, PcbDocPolygonSpec,
     PcbDocPrimitiveSpec, PcbDocSpec, PcbGraphicSpec, PcbGraphicType, SymSpec, PinSpec,
-    PrjPcbSpec, SymSpec, SchDocComponentSpec, SchDocObjectSpec, SchDocSpec, SheetSpec, SymbolRef,
+    PinConnectionTarget, PinConnectionSpec, PrjPcbSpec, SymSpec, SchDocComponentSpec, SchDocObjectSpec, SchDocSpec, SheetSpec, SymbolRef,
 };
@@ -530,12 +530,16 @@ use crate::model::{
 /// Apply a SchDocSpec directly to a document.
 ///
 /// For each sheet in the spec (currently always one):
 /// 1. Apply sheet metadata (fonts, grid settings, custom size)
 /// 2. Add components (matched by designator, add-or-merge)
 /// 3. Add low-level objects (wires, buses, labels, etc.)
-/// 4. Nets/powers will be implemented later (require pin location resolution)
+/// 4. Generate wire stubs for pin connections
 pub fn apply_spec_schdoc(
     spec: &SchDocSpec,
     doc: &mut SchDoc,
+    imported_components: &std::collections::HashMap<String, ComponentSpec>,
 ) -> Result<(), SpecError> {
     for sheet_spec in &spec.sheets {
         let mut sheet = doc.sheet()
@@ -547,12 +551,17 @@ pub fn apply_spec_schdoc(
         // 2. Components
         for comp_spec in &sheet_spec.components {
             apply_schdoc_component(&mut sheet, comp_spec)?;
         }

         // 3. Low-level objects
         for obj_spec in &sheet_spec.objects {
             let obj = schdoc_object_from_spec(obj_spec);
             sheet.add_object(obj);
         }

-        // 4. Nets and powers (wire stub generation)
-        // TODO: requires resolving pin locations from placed components.
-        // For now, nets/powers are a no-op — they'll be implemented when
-        // we add pin location resolution.
+        // 4. Generate wire stubs for pin connections
+        for comp_spec in &sheet_spec.components {
+            if !comp_spec.pin_connections.is_empty() {
+                generate_pin_connection_stubs(
+                    &mut sheet,
+                    comp_spec,
+                    imported_components,
+                    &sheet_spec.power_declarations,
+                )?;
+            }
+        }

         doc.update_sheet(&sheet)
             .map_err(|e| SpecError::no_span(SpecErrorCode::AltiumFormat, e.to_string()))?;
     }
     Ok(())
 }
@@ -648,6 +659,165 @@ fn apply_schdoc_component(
     Ok(())
 }

+// ── Pin connection stub generation ─────────────────────────────────────────
+
+// 200mil matches the standard Altium schematic grid spacing; shorter stubs crowd net labels.
+const STUB_LENGTH_INTERNAL: i32 = 200 * 10_000; // 200mil in internal units (1mil = 10_000)
+
+/// Generate wire stubs (and labels/power symbols/no-connects) for all
+/// `pin_connections` on a placed component.
+fn generate_pin_connection_stubs(
+    sheet: &mut api::SchDocSheet,
+    comp_spec: &SchDocComponentSpec,
+    imported_components: &std::collections::HashMap<String, ComponentSpec>,
+    power_declarations: &std::collections::HashMap<String, PowerObjectStyle>,
+) -> Result<(), SpecError> {
+    // Resolve the SchLib ComponentSpec for this placed component.
+    let lib_ref = match &comp_spec.symbol {
+        SymbolRef::Import { name, .. } => name.as_str(),
+        SymbolRef::Literal(name) => name.as_str(),
+    };
+    let lib_comp = match imported_components.get(lib_ref) {
+        Some(c) => c,
+        None => {
+            return Err(SpecError::no_span(
+                SpecErrorCode::AltiumFormat,
+                format!(
+                    "cannot resolve pin connections for '{}': symbol '{}' not found in imported libraries",
+                    comp_spec.designator, lib_ref
+                ),
+            ));
+        }
+    };
+
+    let comp_orient = comp_spec.orientation.unwrap_or(RotationBy90::Rotate0);
+    let is_mirrored = comp_spec.is_mirrored.unwrap_or(false);
+
+    for conn in &comp_spec.pin_connections {
+        let pin = resolve_pin(lib_comp, &conn.pin_name)?;
+
+        // Pin tip in schematic space.
+        let pin_tip = crate::compiler::transform_pin_position(
+            pin.location,
+            comp_spec.location,
+            comp_orient,
+            is_mirrored,
+        );
+
+        // Transform pin orientation.
+        let transformed_orient = transform_pin_orientation(pin.orientation, comp_orient, is_mirrored);
+
+        match &conn.target {
+            PinConnectionTarget::NoConnect => {
+                sheet.add_object(api::SheetObject::NoConnect(api::NoConnect {
+                    unique_id: String::new(),
+                    location: pin_tip,
+                    color: Color::new(0x000080),
+                    orientation: RotationBy90::Rotate0,
+                    symbol: String::new(),
+                    is_active: true,
+                    suppress_all: false,
+                }));
+            }
+            PinConnectionTarget::Signal(net_name) => {
+                let stub_end = stub_endpoint(pin_tip, transformed_orient);
+                sheet.add_object(api::SheetObject::Wire(api::Wire {
+                    unique_id: String::new(),
+                    vertices: vec![pin_tip, stub_end],
+                    color: Color::new(0x000080),
+                    line_width: PenWidth::Small,
+                    line_style: LineStyle::Solid,
+                }));
+                let label_orient = remap_label_orient(transformed_orient);
+                sheet.add_object(api::SheetObject::NetLabel(api::NetLabel {
+                    unique_id: String::new(),
+                    text: net_name.clone(),
+                    location: stub_end,
+                    orientation: label_orient,
+                    justification: TextJustification::BottomLeft,
+                    font_id: 1,
+                    color: Color::new(0x000080),
+                    is_mirrored: false,
+                }));
+            }
+            PinConnectionTarget::Power(net_name) => {
+                let stub_end = stub_endpoint(pin_tip, transformed_orient);
+                sheet.add_object(api::SheetObject::Wire(api::Wire {
+                    unique_id: String::new(),
+                    vertices: vec![pin_tip, stub_end],
+                    color: Color::new(0x000080),
+                    line_width: PenWidth::Small,
+                    line_style: LineStyle::Solid,
+                }));
+                // Bar fallback — unreachable in normal flow: only nets present in
+                // power_declarations are classified as Power targets.
+                let style = power_declarations
+                    .get(net_name)
+                    .copied()
+                    .unwrap_or(PowerObjectStyle::Bar);
+                sheet.add_object(api::SheetObject::PowerObject(api::PowerObject {
+                    unique_id: String::new(),
+                    text: net_name.clone(),
+                    location: stub_end,
+                    orientation: transformed_orient,
+                    style,
+                    show_net_name: true,
+                    font_id: 1,
+                    color: Color::new(0x000080),
+                    is_cross_sheet_connector: false,
+                }));
+            }
+        }
+    }
+    Ok(())
+}
+
+/// Resolve a pin by name (first) or designator (fallback) from a SchLib component.
+/// Searches both top-level pins and part-block pins (for multi-part components
+/// like dual op-amps or quad gates).
+fn resolve_pin<'a>(lib_comp: &'a ComponentSpec, pin_name: &str) -> Result<&'a PinSpec, SpecError> {
+    // Try matching pin name field first (across all pin sources).
+    // Name match is preferred: names are human-readable (GPIO4, VDD); designators are positional (1, 2).
+    for pin in lib_comp.pins.iter().chain(lib_comp.parts.iter().flat_map(|p| p.pins.iter())) {
+        if pin.name.as_deref() == Some(pin_name) {
+            return Ok(pin);
+        }
+    }
+    // Fallback: match by designator (across all pin sources).
+    for pin in lib_comp.pins.iter().chain(lib_comp.parts.iter().flat_map(|p| p.pins.iter())) {
+        if pin.designator == pin_name {
+            return Ok(pin);
+        }
+    }
+    // Neither name nor designator matched; build the available-pins list for the diagnostic.
+    let available: Vec<String> = lib_comp.pins.iter()
+        .chain(lib_comp.parts.iter().flat_map(|p| p.pins.iter()))
+        .map(|p| p.name.clone().unwrap_or_else(|| p.designator.clone()))
+        .collect();
+    Err(SpecError::no_span(
+        SpecErrorCode::AltiumFormat,
+        format!(
+            "pin '{}' not found in symbol '{}' (available pins: {})",
+            pin_name,
+            lib_comp.lib_reference,
+            available.join(", ")
+        ),
+    ))
+}
+
+/// Transform a pin's orientation from symbol space to schematic space.
+///
+/// Transform order: (1) apply mirror (flip 0↔180), then (2) add component rotation mod 4.
+/// Mirror precedes rotation — this matches `transform_pin_position` semantics; reversing
+/// the order produces wrong directions for mirrored+rotated components.
+fn transform_pin_orientation(
+    pin_orient: RotationBy90,
+    comp_orient: RotationBy90,
+    is_mirrored: bool,
+) -> RotationBy90 {
+    let mirrored_orient = if is_mirrored {
+        match pin_orient {
+            RotationBy90::Rotate0 => RotationBy90::Rotate180,
+            RotationBy90::Rotate180 => RotationBy90::Rotate0,
+            other => other,
+        }
+    } else {
+        pin_orient
+    };
+    let steps = (mirrored_orient as u8 + comp_orient as u8) % 4;
+    match steps {
+        0 => RotationBy90::Rotate0,
+        1 => RotationBy90::Rotate90,
+        2 => RotationBy90::Rotate180,
+        _ => RotationBy90::Rotate270,
+    }
+}
+
+/// Compute the stub endpoint given a pin tip and transformed orientation.
+/// Stub extends 200mil in the direction the pin faces outward.
+fn stub_endpoint(tip: CoordPoint, orient: RotationBy90) -> CoordPoint {
+    let (dx, dy) = match orient {
+        RotationBy90::Rotate0 => (STUB_LENGTH_INTERNAL, 0),
+        RotationBy90::Rotate90 => (0, STUB_LENGTH_INTERNAL),
+        RotationBy90::Rotate180 => (-STUB_LENGTH_INTERNAL, 0),
+        RotationBy90::Rotate270 => (0, -STUB_LENGTH_INTERNAL),
+    };
+    CoordPoint {
+        x: altium_format_types::Coord::new(tip.x.raw() + dx),
+        y: altium_format_types::Coord::new(tip.y.raw() + dy),
+    }
+}
+
+/// Map a pin orientation to a NetLabel orientation.
+///
+/// Altium convention: labels are always 0° or 90°, never 180° or 270°.
+/// Inverted text is unreadable in dense schematics; justification controls
+/// the anchor direction for leftward and downward stubs instead.
+fn remap_label_orient(orient: RotationBy90) -> RotationBy90 {
+    match orient {
+        RotationBy90::Rotate0 | RotationBy90::Rotate180 => RotationBy90::Rotate0,
+        RotationBy90::Rotate90 | RotationBy90::Rotate270 => RotationBy90::Rotate90,
+    }
+}
```

---

### Milestone 4: CLI Integration + ECO/Plan Support

**Files**:
- `crates/altium-cli/src/main.rs` (or wherever `altium apply`/`altium plan` dispatch for schdoc)
- `crates/autopcb-spec/src/eco.rs`

**Flags**: `conformance`

**Requirements**:
- `altium apply` for schdoc specs passes `imported_components` to executor
- `altium plan` shows pin connection stubs in ECO preview
- ECO entries: "Add wire stub U1.GPIO4 → #SDA", "Add power stub U1.VDD → #3V3 (bar)", "Add no-connect U1.NC1"

**Acceptance Criteria**:
- `altium apply test.sch` generates a SchDoc with correct Wire, NetLabel, PowerObject, NoConnect objects
- `altium plan test.sch` shows ECO entries for each pin connection
- `altium dump` or `altium query` on the generated SchDoc returns the expected count of Wire records (one per non-nc pin connection), NetLabel records (one per signal connection), PowerObject records (one per power connection), and NoConnect records (one per nc connection)
- Wire vertex coordinates match expected positions computed from test component placement (verified via `altium dump` output, not just `altium validate` which only checks structural well-formedness)

**Tests**:
- **Test files**: `crates/autopcb-spec/src/executor.rs` (inline `#[test]`, end-to-end through compile+apply pipeline; no separate `tests/` directory exists in the project)
- **Test type**: integration (end-to-end apply + validate)
- **Backing**: default-derived
- **Scenarios**:
  - Apply spec with signal/power/nc pin connections, dump output and verify object counts and positions
  - Plan preview shows expected ECO entry count and descriptions
  - Validated symbol ref error shows in plan mode

**Code Intent**:
- `main.rs`: In the schdoc apply/plan path, call existing `compile_imported_schlibs` to get `HashMap<String, ComponentSpec>`, pass to `apply_spec_schdoc`. The import resolution already happens via `resolve_imports` — ensure the resolved imports feed into both `compile_spec_with_resolved` and the executor.
- `eco.rs`: In the ECO diff generation for schdoc, add entries for pin-connection-generated objects. These appear as regular object additions (Wire, NetLabel, PowerObject, NoConnect) in the ECO since they're materialized by the executor. If the ECO system compares existing sheet state to spec, the generated stubs will show as additions. May need to annotate ECO entries with source info ("from pin connection U1.GPIO4 → #SDA") for clarity.

**Code Changes**:

```diff
--- a/crates/altium-cli/src/main.rs
+++ b/crates/altium-cli/src/main.rs
@@ -1395,6 +1395,9 @@ fn compile_and_resolve(
 struct CompileResult {
     model: autopcb_spec::model::SpecModel,
     /// All import paths (bare + named) for --all processing.
     import_paths: Vec<PathBuf>,
+    /// Compiled SchLib components from imports, keyed by lib_reference.
+    /// Used by SchDoc apply to resolve pin positions.
+    imported_components: std::collections::HashMap<String, autopcb_spec::model::ComponentSpec>,
 }

 fn compile_and_resolve(
@@ -1419,8 +1422,14 @@ fn compile_and_resolve(
     // Compile only the root file's own items, with named imports in scope.
     let imported_components = compile_imported_schlibs(&resolved)
         .map_err(|e| anyhow::anyhow!("{}", e.render(&source_name, source)))?;
+    // Two callees need this map: compile_spec_with_resolved for symbol validation,
+    // apply_spec_schdoc for pin position resolution.
+    let imported_components_for_exec = imported_components.clone();
     let model = compile_spec_with_resolved(&resolved, *domain, imported_components)
         .map_err(|e| anyhow::anyhow!("{}", e.render(&source_name, source)))?;

     // Collect all import paths for --all processing.
     let import_paths: Vec<PathBuf> = resolved
@@ -1430,7 +1439,7 @@ fn compile_and_resolve(
         .chain(resolved.named_imports.values().map(|(p, _)| p.clone()))
         .collect();

-    Ok(CompileResult { model, import_paths })
+    Ok(CompileResult { model, import_paths, imported_components: imported_components_for_exec })
 }
@@ -1159,7 +1159,7 @@ fn run_apply(
     // Apply root spec.
-    apply_for_model(&result.model, target, output, spec_file, &domain)?;
+    apply_for_model(&result.model, target, output, spec_file, &domain, &result.imported_components)?;

     // Apply imports with --all.
     if all {
         for import_path in &result.import_paths {
             let import_domain = detect_spec_domain(import_path)?;
             let import_source = std::fs::read_to_string(import_path)
                 .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", import_path.display()))?;
             let import_result = compile_and_resolve(&import_source, import_path, &import_domain)?;
-            apply_for_model(&import_result.model, None, None, import_path, &import_domain)?;
+            apply_for_model(&import_result.model, None, None, import_path, &import_domain, &import_result.imported_components)?;
         }
     }
@@ -1176,7 +1178,9 @@ fn apply_for_model(
 fn apply_for_model(
     spec_model: &autopcb_spec::model::SpecModel,
     target: Option<&PathBuf>,
     output: Option<&PathBuf>,
     spec_file: &PathBuf,
     domain: &SpecDomain,
+    imported_components: &std::collections::HashMap<String, autopcb_spec::model::ComponentSpec>,
 ) -> anyhow::Result<()> {
@@ -1250,9 +1254,9 @@ fn apply_for_model(
             let out_path = output.cloned().unwrap_or(library_path);

-            apply_spec_schdoc(spec, &mut doc)
+            apply_spec_schdoc(spec, &mut doc, imported_components)
                 .map_err(|e| anyhow::anyhow!("apply failed: {e}"))?;

             doc.save(&out_path)?;
```

Note: The plan for eco.rs does not require new code — pin connection stubs appear as regular Wire/NetLabel/PowerObject/NoConnect additions in the ECO diff because they are materialized by the executor. The reconciler already handles these object types via `schdoc_object_to_add` and the reconcile loop. No changes to eco.rs are needed for the ECO/plan path to show pin connection objects.

---

### Milestone 5: Documentation

**Delegated to**: @agent-technical-writer (mode: post-implementation)

**Source**: `## Invisible Knowledge` section of this plan

**Files**:
- `/home/kiselev/git/ee-template/docs/altium-spec-reference.md`

**Requirements**:
- Document `pin X -> #NET` syntax in `.sch` spec section
- Document validated symbol references via `symbol: $alias.Name`
- Document `power` declaration interaction with pin connections
- Document `nc` keyword for no-connect
- Add complete example showing a component with imports, pin connections, power declarations
- Document error messages for common mistakes (missing `#`, symbol not found)

**Acceptance Criteria**:
- Spec reference has complete syntax documentation for pin connections
- Spec reference documents validated symbol references
- Example is self-contained and compiles correctly
- Error message documentation helps users diagnose issues

Documentation milestone — no code changes.

## Milestone Dependencies

```
M1 (Lexer+AST+Parser) --> M2 (Model+Compiler+ValidatedRefs) --> M3 (Executor) --> M4 (CLI+ECO)
                                                                                      |
                                                                                      v
                                                                                   M5 (Docs)
```

All milestones are sequential — each depends on the previous.

## Risks

| Risk | Mitigation | Anchor |
|---|---|---|
| `Value::ImportRef`/`ImportObject` breaks existing eval paths | Only `eval_field_access` on `ImportObject` returns `ImportRef`; all other paths unchanged. Test existing `$alias.field` usage in non-import contexts. | eval.rs:384 |
| Pin orientation transform produces wrong direction | Unit test all 8 combinations (4 rotations × mirror on/off). Reference: existing `transform_pin_position` test at compiler.rs:5904. | compiler.rs:198 |
| Arrow token breaks minus expressions | Lexer only emits Arrow when `-` is immediately followed by `>`. Space between (`- >`) emits separate Minus + error. Test `a - b` still works. | lexer.rs:256 |
| `imported_components` not available in executor | Thread HashMap from CLI through all call sites. Default to empty HashMap for backward compat — pin connections on components without imports produce clear error at resolution time. | executor.rs:537 |

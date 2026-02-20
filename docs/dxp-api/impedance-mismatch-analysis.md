# Impedance Mismatch Analysis: altium-format vs Altium DXP API

Analysis of the gap between Altium's actual API surface (as reverse-engineered
from Delphi DLLs and .NET decompilation) and our current Rust implementation.

---

## 1. Executive Summary

Our `altium-format` crate treats Altium files as **flat record stores** --
collections of key-value parameter blocks with parent-child relationships.
Altium's internal model is a **rich object graph** with typed primitives,
container hierarchies, iterators, design rules, layer stacks, pad caches,
undo transactions, and cross-document references.

The core mismatch: **we model the serialization format; Altium models the
design domain**. Our records faithfully reproduce the bytes on disk but don't
expose the *semantic operations* that Altium's API provides. This is why
agents keep needing to make internal types public -- the high-level API
doesn't yet provide enough surface area to implement operations like rebuild
without reaching into the backing store.

### What We Have vs What's Needed

| Capability | Our API | Altium API |
|---|---|---|
| Read/write records | Full (flat R/W) | Full (typed properties) |
| Type-safe primitives | Partial (9 PCB, 38 Sch) | Complete (26+ PCB, 120+ Sch) |
| Iterator/query | Basic query language | 6 iterator types with filters |
| Container hierarchy | Group/child IDs only | Add/Remove/Move/Reorder |
| Layer system | V6 byte IDs | V6 + V7 + layer stack management |
| Design rules | Not modeled | 70 rule types, scope evaluation |
| Pad stack | Simple top/mid/bot | Per-layer shapes, sizes, corner radii |
| Mask expansion | Not modeled (in sidecar) | 3-mode resolution chain |
| Unique IDs | Stored but no generation | Document-level generation |
| Cross-document links | Not modeled | Component source links, vault GUIDs |
| Undo/transactions | Not applicable | Begin/End modify pattern |
| Net/class management | Not modeled | Full net analysis, class membership |

---

## 2. Missing PCB Types and Properties

### 2.1 Object Types We Don't Model

Our 10 PCB record types vs Altium's 26 `TObjectId` values:

| TObjectId | Type | Our Status |
|---|---|---|
| 1 eArcObject | PcbArcRecord | Implemented |
| 2 ePadObject | PcbPadRecord | Implemented (incomplete padstack) |
| 3 eViaObject | PcbViaRecord | Implemented (incomplete padstack) |
| 4 eTrackObject | PcbTrackRecord | Implemented |
| 5 eTextObject | PcbTextRecord | Implemented (missing TrueType, barcode) |
| 6 eFillObject | PcbFillRecord | Implemented |
| 7 eFromToObject | **Missing** | Ratsnest endpoint |
| 8 eNetObject | **Missing** | Net grouping, connectivity |
| 9 eComponentObject | **Missing** (only footprint metadata) | Full component with source links |
| 10 ePolygonObject | **Missing** | Copper pour polygon |
| 11 eRegionObject | PcbRegionRecord | Implemented (incomplete) |
| 12 eComponentBodyObject | PcbComponentBodyRecord | Implemented (missing 3D model) |
| 13 eDimensionObject | **Missing** | 11 dimension subtypes |
| 14 eCoordinateObject | **Missing** | Coordinate annotation |
| 15 eClassObject | **Missing** | Net/component classes |
| 16 eRuleObject | **Missing** | 70 design rule types |
| 17 eManualFromToObject | **Missing** | Manual ratsnest |
| 18 eDifferentialPairObject | **Missing** | Diff pair definitions |
| 19 eViolationObject | **Missing** | DRC violations |
| 20 eEmbeddedObject | **Missing** | Generic embedded |
| 21 eEmbeddedBoardObject | **Missing** | Panel arrays |
| 22 eSplitPlaneObject | **Missing** | Split plane regions |
| 23 eTraceObject | **Missing** | Routed trace groups |
| 24 eSpareViaObject | **Missing** | Spare vias |
| 25 eBoardObject | **Missing** | Board root (PcbDoc only) |
| 26 eBoardOutlineObject | **Missing** | Board outline |
| 7 (PcbDoc) eConnectionObject | PcbConnectionRecord | Implemented |

**Priority additions** (needed for rebuild round-trip fidelity):
1. **eNetObject** (8) -- Almost every primitive references a net
2. **eComponentObject** (9) -- PCB components with source links
3. **ePolygonObject** (10) -- Copper pours are fundamental
4. **eDimensionObject** (13) -- Common in real designs
5. **eClassObject** (15) -- Net/component classes used by rules
6. **eRuleObject** (16) -- Design rules govern the entire board
7. **eBoardObject** (25) -- Board-level metadata

### 2.2 Incomplete Pad/Via Modeling

Altium's pad API is the most complex single type with **37 parameters** in
the Delphi `PcbApi_QueryPad` and extensive per-layer support.

**What we're missing on PcbPadRecord:**
- Per-layer shapes via `GetState_StackShapeOnLayer(IV7_Layer)` (local stack mode)
- Per-layer sizes via `GetState_XStackSizeOnLayer/YStackSizeOnLayer`
- Per-layer corner radius via `GetState_StackCRPctOnLayer(IV7_Layer)`
- Per-layer pad offsets `GetState_XPadOffsetOnLayer/YPadOffsetOnLayer`
- Hole slot dimensions `GetState_HoleWidth`, `GetState_HoleRotation`
- Drill type/hole type enums `TExtendedDrillType`, `TExtendedHoleType`
- Hole tolerances `GetState_HolePositiveTolerance/NegativeTolerance`
- Solder mask expansion from hole edge flag
- Swap IDs `GetState_SwapID_Pad/Part`
- Pad cache (resolved electrical properties from design rules)
- Counter hole support (`IPCB_Pad3`)
- Pad/via template links

**What we're missing on PcbViaRecord:**
- Via type enum (Thru, Blind, Buried, Micro, Skip, Backdrill)
- Start/stop layer (V7 layer IDs)
- Per-layer stack sizes
- Backdrill info `PcbApi_QueryViaBackDrill`
- Counter hole parameters

### 2.3 Extended Primitive Properties (IPCB_Primitive2)

Currently serialized in the `ExtendedPrimitiveInformation` sidecar stream
but **not surfaced through our public API**:

- `PasteMaskExpansionMode` (NoMask/Rule/Manual)
- `PasteMaskExpansion` (manual value)
- `SolderMaskExpansionMode` (NoMask/Rule/Manual)
- `SolderMaskExpansion` (manual value)
- `PasteMaskEnabled`, `PasteMaskUsePercent`, `PasteMaskPercent`
- `GUID` (primitive-level GUID from PrimitiveGuids stream)

**Recommendation:** Merge these into the typed record structs. The sidecar
stream is a serialization detail -- at the API level, a pad's mask expansion
mode should be a field on `PcbPadRecord`.

---

## 3. Missing Schematic Types and Properties

### 3.1 Record Type Coverage

We support 38 schematic record types. Altium defines 120+ `TObjectId`
values. Many are niche (harness, FSM, diagram) but several common ones
are missing:

| TObjectId | Type | Priority |
|---|---|---|
| 2 eNote | Implemented (SchNoteRecord) | -- |
| 48 eCrossSheetConnector | **Missing** | Medium |
| 53 eHarnessConnector | **Missing** | Low |
| 54 eHarnessEntry | **Missing** | Low |
| 55 eHarnessConnectorType | **Missing** | Low |
| 56 eSignalHarness | **Missing** | Low |
| 61 eBlanket | Implemented | -- |

Most of the 82 missing types are specialized (harness, FSM, diagram, etc.)
and unlikely to appear in typical designs.

### 3.2 Incomplete Component Modeling

**What we're missing on SchComponentRecord:**
- Multi-part support: `PartCount`, `CurrentPartID`, display mode management
- Library references: `DesignItemId`, `DatabaseTableName`, vault GUIDs
- Variant support: `VariantOption`, variant component links
- Alias management: `AliasCount`, `AliasAt`, `Alias_Add/Remove`
- Implementation links: `ImplementationCount`, `AddImplementation`
- Generic component template GUIDs
- Component kind (`TComponentKind`: Standard, Mechanical, Graphical, NetTie, Jumper)
- Pin access: `GetAllPinCount`, `PinByDesignator`

### 3.3 Incomplete Pin Modeling

**What we're missing on SchPinRecord:**
- IEEE symbols: `Symbol_Inner`, `Symbol_Outer`, `Symbol_InnerEdge`, `Symbol_OuterEdge`
- Pin swap IDs: `SwapIdPart`, `SwapIdPin`, `SwapIdPartPin`, `SwapIdPair`
- Alternate pin functions: `SelectedFunctions`, `DefinedFunctions`
- Custom name/designator formatting (position mode, font mode, custom color)
- Propagation delay
- Package length
- Symbolic name and function display modes

### 3.4 Missing Document-Level Properties

**SchSheetRecord / SchDoc metadata we don't model:**
- Sheet style enum (`TSheetStyle`: A4, A3, A, B, Letter, etc.)
- Reference zones (count, margin, style)
- Title block settings
- Template management (file name, vault GUIDs)
- Grid settings (snap, visible, hotspot, electrical)
- Unit system
- Font table management
- Document name and file format version

---

## 4. Architectural Gaps

### 4.1 No Container Mutation API

Altium provides:
```
PcbApi_AddObjectToContainer(container, object)
PcbApi_DeleteObjectFromContainer(container, object)
SchAPI_AddObjectToContainer(container, objectType)
SchAPI_CreateObject(container, objectType)
```

Our API has **no equivalent**. We can create records via builders and
templates, but there's no typed way to add a pad to a footprint, add a
pin to a component, or remove a wire from a sheet after parsing.

**Recommendation:** Add container mutation methods to group handles:
```rust
impl PcbFootprintHandle {
    pub fn add_pad(&self, record: PcbPadRecord) -> PcbPadHandle { ... }
    pub fn remove_pad(&self, handle: PcbPadHandle) -> Result<()> { ... }
}
impl SchComponentHandle {
    pub fn add_pin(&self, record: SchPinRecord) -> SchPinHandle { ... }
    pub fn remove_child(&self, id: RecordId) -> Result<()> { ... }
}
```

### 4.2 No Typed Iterator Pattern

Altium has 6 iterator types with type/layer/spatial/method filters:
- `BoardIterator` with `AddFilter_ObjectSet`, `AddFilter_LayerSet`, `AddFilter_Method`
- `GroupIterator` for component/net children
- `SpatialIterator` for region-based queries
- `LibraryIterator` for library component traversal

Our query module provides a textual query language but no typed iterator
API. The ops crate has to iterate via low-level record IDs.

**Recommendation:** Add typed iterators on document/group handles:
```rust
impl PcbDoc {
    pub fn iter_primitives(&self) -> PcbPrimitiveIter { ... }
    pub fn iter_primitives_of_type<T: PcbPrimitive>(&self) -> TypedIter<T> { ... }
}
impl PcbFootprintHandle {
    pub fn iter_children(&self) -> GroupChildIter { ... }
    pub fn iter_pads(&self) -> impl Iterator<Item = PcbPadHandle> { ... }
}
```

### 4.3 No Layer System Abstraction

Altium has a rich layer system:
- V6 layer IDs (byte-based, 82 layers, used in binary format)
- V7 layer IDs (32-bit structured: family/genus/species/flags)
- Layer stack objects with stack management
- Layer classification (signal, plane, mechanical, mask, overlay)

Our code stores raw layer byte values in records but doesn't provide:
- A `Layer` enum or newtype
- V6 <-> V7 conversion
- Layer classification queries
- Layer stack traversal

**Recommendation:** Create a `pcb::Layer` type:
```rust
pub enum Layer {
    TopLayer,
    MidLayer(u8),   // 1-30
    BottomLayer,
    TopOverlay,
    BottomOverlay,
    TopSolder,
    BottomSolder,
    TopPaste,
    BottomPaste,
    InternalPlane(u8), // 1-16
    Mechanical(u8),    // 1-16
    KeepOut,
    MultiLayer,
    DrillGuide,
    DrillDrawing,
}
```

### 4.4 No Unique ID Management

Altium provides:
- `IPCB_Board.GenerateUniqueID() -> string`
- `ISchDocument.GenerateUniqueID() -> string` (8-char alphanumeric)
- `IsIDUnique(id, container) -> bool`

Our records store UniqueID values but we don't provide:
- ID generation
- ID uniqueness checking
- Cross-document ID resolution

### 4.5 No Design Rule System

Altium has 70 `TRuleKind` values with scope evaluation, priority resolution,
and constraint checking. Our format doesn't model rules at all.

**Priority for rebuild:** Low (rules are a PcbDoc-only concept, and rebuild
can round-trip the raw parameter blocks).

### 4.6 Sidecar Stream Properties Not Merged Into Records

The key insight from our research: **sidecar streams are a serialization
artifact, not a runtime distinction**. In Altium's model, a pad's mask
expansion mode is just another property on the pad object. It happens to be
stored in `ExtendedPrimitiveInformation` for backwards compatibility.

Our current approach stores sidecar data separately from the main records.
This leaks the serialization format into the API.

**Recommendation:** During document loading, merge sidecar data into the
typed records. During saving, split it back out. The public API should
never expose sidecar streams as a concept.

For SchLib pins specifically:
- PinFrac -> merge fractional coords into pin location/length
- PinDesc -> merge long description into pin description field
- PinMiscData -> merge swap ID pair into pin record
- PinTextData -> merge custom text formatting into pin record
- PinWideText -> merge Unicode text into pin name/designator/description
- PinSymbolLineWidth -> merge into pin record
- PinPackageLength -> merge into pin record
- PinPropagationDelay -> merge into pin record
- PinFunctionData -> merge into pin record (new fields)

For PcbLib/PcbDoc:
- WideStrings -> merge into text records' text field
- UniqueIDPrimitiveInformation -> merge into each record's UniqueID field
- ExtendedPrimitiveInformation -> merge mask expansion into pad/via/track records

---

## 5. API Design Recommendations

### 5.1 Property Access Pattern

Altium uses `GetState_*/SetState_*` pairs (COM heritage). Our Rust API uses
`record.field()` getters on struct fields. This is fine -- Rust structs
with pub fields or getter methods are idiomatic.

**However,** we should ensure complete field coverage. Every `GetState_*`
method in the .NET interfaces represents a property that exists in the
binary format. Missing fields mean incomplete round-trips.

### 5.2 Recommended API Surface

#### Tier 1: Core (needed for rebuild round-trip)

1. **Complete field coverage** on existing record types (pad stack, via layers,
   text TrueType, component source links, pin IEEE symbols)
2. **Sidecar merge** -- load/save transparently handles sidecar streams
3. **Missing fundamental types**: Net, Component (PcbDoc), Polygon, Dimension
4. **Container mutation**: add/remove children on groups

#### Tier 2: High-Level Operations (needed for ops crate)

5. **Typed iterators** on documents and groups
6. **Layer abstraction** with V6 enum and classification
7. **Unique ID generation** at document level
8. **Component queries**: find by designator, find by name, pin lookup by name

#### Tier 3: Advanced (needed for full tool support)

9. **Design rules** modeling and scope evaluation
10. **Net analysis** (connectivity, ratsnest)
11. **Cross-document references** (SchDoc <-> PcbDoc linking via UniqueID)
12. **Pad cache** (resolved electrical properties)

### 5.3 Keeping Implementation Details Private

The key constraint from CLAUDE.md: `ParamCollection`, `FromOrigin/ToOrigin`,
`RecordOrigin`, etc. must stay `pub(crate)`.

**How to add features without exposing internals:**

1. **Merge sidecar data during load/save** -- the Document types already
   own the load/save pipeline. Add sidecar merge as a post-load step and
   sidecar split as a pre-save step. No new public types needed.

2. **Add fields to existing record types** -- proc macro generates the
   `FromOrigin`/`ToOrigin` implementations. Adding a field to a record
   struct is just adding a new `#[altium(...)]` attribute.

3. **Add methods to handle types** -- handles are already public. Adding
   `fn iter_pads(&self)` to `PcbFootprintHandle` uses only internal
   store access, which is already available via the `pub(crate) store` field.

4. **Add methods to document types** -- `PcbLib`, `SchLib`, etc. own the
   store and can provide high-level queries without exposing store internals.

---

## 6. Prioritized Action Items

### Phase 1: Fix Round-Trip Failures (rebuild fidelity)

1. **Merge sidecar streams into records** on load, split on save
   - SchLib: all 9 pin sidecar streams
   - PcbLib/PcbDoc: WideStrings, UniqueIDPrimitiveInformation, ExtendedPrimitiveInformation
2. **Complete PcbPadRecord fields**: per-layer shapes/sizes/CR, hole slot, tolerances
3. **Complete PcbViaRecord fields**: start/stop layer, via type, per-layer sizes
4. **Complete PcbTextRecord fields**: TrueType font properties, barcode, multiline
5. **Complete SchPinRecord fields**: IEEE symbols, swap IDs, custom formatting
6. **Complete SchComponentRecord fields**: multi-part, library refs, component kind

### Phase 2: Add Missing Fundamental Types

7. Add PcbNetRecord (eNetObject = 8)
8. Add PcbComponentRecord (eComponentObject = 9) with source links
9. Add PcbPolygonRecord (ePolygonObject = 10) with pour settings
10. Add PcbDimensionRecord (eDimensionObject = 13) with subtypes
11. Add PcbBoardRecord (eBoardObject = 25) for PcbDoc metadata

### Phase 3: High-Level API

12. Add typed iterators to document and group handles
13. Add container mutation (add/remove) to group handles
14. Add Layer enum with V6 mapping and classification
15. Add UniqueID generation
16. Add component/pin lookup queries

### Phase 4: Advanced Features

17. Design rule types and scope evaluation
18. Net connectivity analysis
19. Cross-document reference resolution
20. Pad cache computation

---

## 7. Source References

- [PCB Delphi API Functions](pcb-api-functions.md) -- 290+ PcbApi_* functions
- [Schematic Delphi API Functions](sch-api-functions.md) -- 135 SchApi_* functions
- [PCB .NET Data Model](pcb-dotnet-model.md) -- Complete interface documentation
- [Schematic .NET Data Model](sch-dotnet-model.md) -- Complete interface documentation
- [Sidecar Streams Deep Dive](sidecar-streams-deep-dive.md) -- Binary format specs
- [SIDECARS.md](../../SIDECARS.md) -- Existing sidecar documentation

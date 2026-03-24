# SchDoc Spec Language — Research Notes

## How Altium Links SchDoc <-> SchLib <-> PcbDoc

### The Linking Chain

```
.sym (footprint) -> PcbLib (footprint geometry)
                ^ footprint name
.sym (component) -> SchLib (symbol definitions: pins, graphics, parameters, footprint refs)
                ^ LIBREFERENCE + SOURCELIBRARYNAME
.sch             -> SchDoc (placed instances + wires + nets)
                ^ SourceDesignator + SourceUniqueId + net names
.pcb             -> PcbDoc (placed footprints + routed copper)
```

### SchDoc -> SchLib Linking (The Key Mechanism)

Each `SchComponent` (Record 1) in a SchDoc carries:

| Field | Purpose |
|-------|---------|
| `LIBREFERENCE` | Component name to look up in SchLib (e.g., "LM358") |
| `SOURCELIBRARYNAME` | Library filename (e.g., "my-parts.SchLib") |
| `LIBRARYPATH` | Full path (or `*` for default search) |
| `DESIGNITEMID` | Unique design-level ID for this placed instance |
| `UNIQUEID` | 8-char persistent ID for cross-doc traceability |

When placed, the SchDoc gets a **full copy** of all pins, parameters, graphics from
the SchLib. "Update from Library" re-syncs by matching `LIBREFERENCE` in
`SOURCELIBRARYNAME`.

### SchDoc Internal Connectivity

SchDoc-only record types that create the netlist:

| Record | Type | Role |
|--------|------|------|
| 27 | SchWire | Electrical connections (indexed vertex lists) |
| 25 | SchNetLabel | Names a net segment |
| 17 | SchPowerObject | VCC/GND/etc. (implicit global nets) |
| 18 | SchPort | Sheet-level IO port |
| 29 | SchJunction | Wire junction dot |
| 26 | SchBus | Multi-signal bus group |
| 37 | SchBusEntry | Bus tap point |
| 15 | SchSheetSymbol | Hierarchical sub-sheet reference (has `FILENAME`) |
| 16 | SchSheetEntry | Entry point on sheet symbol |

### PcbDoc -> SchDoc Linking

Each PCB component in `Components6` stores:
- `SourceDesignator` — matches SchDoc designator (R1, U5)
- `SourceUniqueId` — persistent ID matching SchDoc component UNIQUEID
- `SourceHierarchicalPath` — for multi-sheet designs
- Net names in `Nets6` match schematic net names exactly
- `UniqueIDPrimitiveInformation` sidecar maintains per-primitive GUIDs for ECO


## SchDoc Record Types

### SchDoc-Specific Records (NOT in SchLib)

| Record | Type | Key Fields |
|--------|------|------------|
| 31 | SchSheet | Font table, sheet size, title block, grid settings |
| 39 | SchTemplate | Template file reference |
| 27 | SchWire | LOCATIONCOUNT, X1/Y1/X2/Y2... (1-based indexed vertices), UNIQUEID |
| 26 | SchBus | Same as Wire but for buses, vertex order differs in serialization |
| 25 | SchNetLabel | Text (net name), location, font_id, orientation |
| 17 | SchPowerObject | Text, style (circle/arrow/bar/wave/gnd variants), orientation |
| 18 | SchPort | Name, io_type (unspecified/output/input/bidirectional), style |
| 15 | SchSheetSymbol | Location, corner (size), filename, is_solid |
| 16 | SchSheetEntry | Side (L/R/T/B), io_type, style, name; owned by SheetSymbol |
| 29 | SchJunction | Location, color; NO UniqueID |
| 37 | SchBusEntry | Location, corner (direction) |
| 22 | SchNoConnect | Location, symbol style, ERC suppression pairs |
| 43 | SchParameterSet | Named parameter container |
| 209 | SchNote | Annotation note with author/formatting |
| 211 | SchCompileMask | Compile mask/blanket region |
| 225 | SchDashedRectangle | Dashed rectangle overlay (in Additional stream) |

### Shared Records (Same as SchLib)

| Record | Type | Notes |
|--------|------|-------|
| 1 | SchComponent | In SchDoc: has DESIGNITEMID, ALLPINCOUNT. Pins are text format (not binary) |
| 2 | SchPin | Text format in SchDoc (parameter blocks), binary in SchLib |
| 3-14 | Graphics | Line, Rectangle, Arc, Ellipse, Polyline, Polygon, etc. |
| 28 | SchTextFrame | Rich text box |
| 30 | SchImage | Embedded image |
| 34 | SchDesignator | Reference designator display |
| 41 | SchParameter | Named parameter (most common record type) |
| 44-48 | Implementation | Footprint assignment chain |

### Key Differences: SchDoc vs SchLib

| Aspect | SchDoc | SchLib |
|--------|--------|--------|
| CFB layout | Flat (3 streams: FileHeader, Additional, Storage) | Hierarchical (per-component sub-storages) |
| Pin format | Parameter text (flags=0x00) | Binary (flags=0x01) + 9 sidecar streams |
| OwnerIndex scope | Global absolute index | Component-relative index |
| Sheet record | Record 31, always first content block | Not present |
| SectionKeys/Aliases | Not needed | Required for name-to-CFB-key mapping |
| Component identity | DESIGNITEMID + UNIQUEID | LIBREFERENCE |


## SchDoc CFB Structure

```
Root Storage
+-- /FileHeader       (document header + ALL records in one flat stream)
+-- /Additional       (RECORD=225 dashed rectangles, optional)
+-- /Storage          (embedded images, zlib-compressed, optional)
+-- /ObjectDefinitions       (optional)
+-- /ReuseBlockInfos         (optional)
+-- /ReuseBlocks             (optional)
+-- /ReuseBlocksV2           (optional)
+-- /HarnessConnectionPointConnector (optional)
+-- /Files                   (optional)
```

### FileHeader Stream Layout

1. Header block (flags=0x00): `HEADER=...`, `Weight=N`, `MinorVersion=9`, `UniqueID=...`
2. Block 1 (index 0): SchSheet (RECORD=31) — always first, contains font table
3. Block 2 (index 1): SchTemplate (RECORD=39) — always second
4. Remaining blocks: All other records in depth-first ownership order

### OwnerIndex Model

- 0-based absolute index into the flat record list (excluding header)
- Sheet-level objects: OWNERINDEX=0 (or absent, implicitly owned by sheet)
- Component children: OWNERINDEX = parent component's absolute index
- Implementation chain: 44 -> 45 -> 46/48 via OWNERINDEX

### Wire Serialization Anomaly

- SchWire (RECORD=27): exports UNIQUEID **before** vertices
- SchBus (RECORD=26): exports vertices **before** UNIQUEID
- CompileMask/BusEntry: export UNIQUEID first, before Location/Corner


## Coordinate System

- 10,000 internal units = 1 mil
- DXP fractional encoding: `raw = integer * 100,000 + fractional`
- `_FRAC` keys omitted when zero
- Colors: Win32 COLORREF `0x00BBGGRR` (BGR order)


## Proposed SchDoc Spec Entities

### `place` — Component Instance

```
R1 = place $lib.R_0603 {
    at: (1000mil, 800mil)
    orientation: 0
    value: "10K"           // parameter override
}
```

The `place` keyword bridges library definitions to sheet instances:
1. Looks up component in imported `.sym` file
2. Creates SchComponent with LIBREFERENCE + SOURCELIBRARYNAME
3. Copies all pins, parameters, graphics from library
4. Applies instance overrides
5. Generates UNIQUEID + DESIGNITEMID

### `wire` — Electrical Connection

```
w1 = wire { points: [(x1, y1), (x2, y2), ...] }
wire { from: $R1.1, to: $U1.2 }    // pin-ref shorthand with autorouting
```

### `net_label` — Net Naming

```
net_label VCC { at: (500mil, 1000mil) }
```

### `power` — Power/Ground Symbol

```
power VCC { at: (3000mil, 1200mil), style: arrow }
power GND { at: (3000mil, 100mil), style: gnd_signal }
```

### `port` — Sheet-Level IO

```
port DATA_BUS { at: (0, 500mil), io_type: bidirectional }
```

### `sheet_symbol` — Hierarchical Reference

```
sheet_symbol "Regulators" {
    at: (5000mil, 300mil)
    size: (500mil, 400mil)
    filename: "regulators.SchDoc"
    entry VIN  { side: left, io_type: input }
    entry VOUT { side: right, io_type: output }
}
```

### Identity Keys

| Entity | Identity Key | Source |
|--------|-------------|--------|
| Component Instance | Designator (e.g., "R1") | `place` name |
| Wire | binding name -> UNIQUEID | optional binding |
| Net Label | text (net name) scoped to sheet | name after `net_label` |
| Power Object | text scoped to sheet | name after `power` |
| Port | name | name after `port` |
| Sheet Symbol | name | name after `sheet_symbol` |
| Sheet Entry | name (scoped to sheet symbol) | name after `entry` |
| Junction | auto-generated | position-based |
| Graphic | binding name -> unique_id | optional binding |


## LowOps Needed for SchDoc

| LowOp | Status | Description |
|-------|--------|-------------|
| `CreateSheet` | Needed | Initialize SchDoc with sheet properties (Record 31) |
| `AddComponent` (SchDoc variant) | Needed | Place instance with DESIGNITEMID, position, SOURCELIBRARYNAME |
| `AddWire` | Needed | Create wire with vertex list |
| `AddNetLabel` | Needed | Create net label at position |
| `AddPowerObject` | Needed | Create power/ground symbol |
| `AddPort` | Needed | Create sheet port |
| `AddSheetSymbol` | Needed | Create hierarchical sheet reference |
| `AddSheetEntry` | Needed | Create entry on sheet symbol |
| `AddJunction` | Needed | Create junction dot |
| `AddBus` / `AddBusEntry` | Needed | Create bus primitives |
| `EditComponent` (SchDoc) | Needed | Update position, orientation, parameters |
| `EditWire` | Needed | Update wire vertices |


## Design Decisions (Resolved)

See `design-questions.md` for full rationale.

### 1. All Imports Are Named References

No bare imports. Every import requires `as alias`. Each spec file produces exactly
one output file. Bare import composition (merging `.sym` files) is dropped — `.sch`
specs reference individual library specs directly, which is how real Altium projects
work anyway.

```
import "passives.sym" as passives
import "ics.sym" as ics
import "vendor-parts.SchLib" as vendor      // compiled binary import too

R1 = place $passives.R_0603 { at: (1in, 1in), value: "10K" }
U1 = place $ics.LM358 { at: (2in, 1in) }
```

### 2. Connectivity via Net Labels (No Autorouting)

No wire routing. Each pin gets a short wire stub + net label (or power object).
Connectivity is purely logical via matching net names. Trivially idempotent, no
spatial reasoning needed.

```
net VCC {
    $U1.8, $R1.1, $C1.1
    power { style: arrow }
}

net SIG_A {
    $R1.2, $U1.2
}
```

Compiles to: per pin, a short wire stub from pin tip + net label at endpoint.
Power nets get power objects instead of net labels.

Future: optional autorouting via tool flag (`--route=auto`), same syntax.

### 3. UniqueID via Deterministic MD5 Hash

Use Altium's own `UniqueIdUtils.GenerateUniqueId(seed)` algorithm to produce
native-looking 8-char uppercase A-Z UniqueIds from deterministic seeds.

**Source**: `AD26-dotnet/Altium.Sch.Base/Altium.Sch.Base.Utils/UniqueIdUtils.cs`

**Algorithm:**

```
Input:  seed string (ASCII)
Step 1: MD5(seed_bytes) → 16 bytes → format as 32-char uppercase hex
Step 2: Process in 8 chunks of 4 hex chars:
        For each chunk [c0, c1, c2, c3]:
            h = 19
            h = h * 31 + hex_digit_value(c0)
            h = h * 31 + hex_digit_value(c1)
            h = h * 31 + hex_digit_value(c2)
            h = h * 31 + hex_digit_value(c3)
            output = ALPHABET[h % 26]     // ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
Result: 8 uppercase A-Z characters
```

Where `hex_digit_value`: `'0'-'9'` → 0-9, `'A'-'F'` → 10-15, else → 0.

Note: Altium encodes the seed with Windows-1252 (`EncodingACP`) before MD5. For our
ASCII-only seeds, this is identical to UTF-8 byte encoding.

**Collision resolution**: `GetNextUniqueId()` increments in base-26
(A=0 ... Z=25, with carry): `AAAAAAAZ` → `AAAAABA`.

**Seed format**: `spec:{file_stem}:{entity_type}:{identity_key}`

| Spec Entity | Seed Example |
|------------|-------------|
| Component instance | `spec:psu:inst:R1` |
| Wire (net stub) | `spec:psu:wire:VCC:U1.8` |
| Wire (explicit route) | `spec:psu:wire:clk_route` |
| NetLabel | `spec:psu:netlabel:VCC:0` |
| PowerObject | `spec:psu:power:GND:0` |
| NoConnect | `spec:psu:nc:U1.3` |
| Junction | `spec:psu:junc:SDA:0` |
| Port | `spec:psu:port:DATA_BUS` |
| SheetSymbol | `spec:psu:sheetsym:Regulators` |
| SheetEntry | `spec:psu:sheetentry:Regulators:VIN` |
| Graphic (named) | `spec:psu:gfx:border_rect` |
| Graphic (unnamed) | `spec:psu:gfx:anon:line:3` |

**Key property**: Spec-generated UniqueIds are deterministic — same seed → same ID
across runs. Altium-created records have random IDs, so they never collide with spec IDs.
The reconciler matches by UniqueId to find existing records and apply targeted updates
instead of delete+recreate (solving the topological naming problem).

See `docs/schdoc/plan.md` §10 for the full identity architecture.

### 4. PcbDoc Extension (Future)

Full chain completion: `.pcb` spec → PcbDoc with component placement from
`.sch` spec netlist, net routing, copper pours, design rules.

# Spec-Based Bidirectional Sync: IR Design

Design document for the `SyncSnapshot` intermediate representation and associated
projection, diff, and change-application pipeline that enables bidirectional
synchronization between SchDoc-spec and PcbDoc-spec files.

---

## 1. Motivation

Altium's own ECO system (documented in `eco-change-types.md`, `document-adapters.md`,
`eco-validation-pipeline.md`) operates on compiled, binary document representations:
the schematic is compiled into a UDM adapter tree, and that compiled state is compared
against the PCB's UDM. Our spec files are plain-text declarative descriptions of what
a document should contain. They are not binary documents, have no runtime compiler,
and carry no persistent UniqueIDs.

The goal of this system is to replicate the essential semantics of Altium's
schematic-to-PCB ECO pipeline at the spec level, without depending on the binary
document format. This enables:

- A text-based, version-controllable workflow where schematic and board specs are
  kept in sync through an explicit, reviewable change process.
- Forward sync: pushing new components and net assignments from a SchDoc-spec to a
  PcbDoc-spec after schematic changes.
- Back-annotation: pushing designator changes or pin-swap results from the PcbDoc-spec
  back to the SchDoc-spec.
- A foundation for future bidirectional and three-way merge workflows.

---

## 2. The SyncSnapshot IR

Both `SchDocSpec` and `PcbDocSpec` project into a common `SyncSnapshot`. The snapshot
is a normalized, document-independent representation of the cross-domain data that
matters for synchronization: what components exist, what footprints they use, what nets
exist, and which pins are connected to which nets.

```
SyncSnapshot
├── components: Map<Designator, SyncComponent>
│     ├── designator: String                   -- primary key
│     ├── comment: Option<String>
│     ├── footprint: Option<String>            -- PCB pattern name
│     ├── source_library: Option<String>
│     ├── parameters: Map<String, String>
│     └── pins: Map<PinDesignator, SyncPin>
│           └── net: Option<NetName>           -- None = unconnected
├── nets: Map<NetName, SyncNet>
│     ├── name: String                         -- primary key
│     ├── color: Option<Color>
│     └── pins: Vec<(Designator, PinDesignator)>  -- derived, not stored: from component pins
└── classes: Map<ClassName, SyncClass>         -- future
      ├── kind: ClassKind (NetClass | ComponentClass)
      └── members: Vec<String>
```

### Design rationale

The snapshot intentionally omits:

- Schematic-domain geometry: wire paths, bus entries, net labels, port locations,
  symbol rotation and mirroring. These are presentation details that belong exclusively
  in the SchDoc-spec and have no PCB equivalent.
- PCB-domain placement: component `location`, `rotation`, `layer`. These are determined
  by the PCB layout and must never be clobbered by forward sync.
- Internal PCB primitives: tracks, vias, arcs, fills, polygons, rules, differential
  pairs. These are either auto-generated or managed independently of the schematic.

This mirrors the explicit scope boundary documented in `sch-pcb-mapping.md` §4
("Parameters NOT Synced"): Altium itself excludes `DM_LocationX/Y`, `DM_Rotation`,
`DM_Layer`, `DM_DisplayMode`, and the full symbol geometry from its ECO comparison.

The snapshot is **read-only** and **ephemeral**: it is computed on demand, never
persisted, and discarded after the diff is produced.

---

## 3. Identity Strategy: Designator-Based Matching

Components are matched between snapshots by **designator**. Nets are matched by
**name**. Neither UniqueID nor any other persistent identifier is used.

### Why not UniqueID?

Altium's binary documents embed a per-object `UNIQUEID` parameter (an 8-character
random string like `ABCDEFGH`) that survives copy-paste and serves as the stable
cross-domain identity key. `document-adapters.md` §UniqueID describes how this is
combined with the hierarchy path to produce the matching key used by
`ComponentAdapter.DM_UniqueId()`.

Spec files are plain text. They carry no persistent UUIDs:

- A `SchDocSpec` component is a block identified by its `designator` property.
- A `PcbDocSpec` component is a block identified by its `designator` property.
- There is no mechanism to assign or persist a UUID through the spec text format
  without adding significant complexity.

`sch-pcb-mapping.md` §1 confirms that Altium itself falls back to designator-based
matching (`eMapByDesignator`) when UniqueID links are broken or missing. The `eMapByAny`
strategy tries UniqueID first, then designator. For specs, designator-only matching
is the correct starting point; it is the same fallback Altium uses for unlinked or
newly-placed components.

### Matching rules

- **Components**: exact case-sensitive designator match. (`R1` in SchDoc-spec matches
  `R1` in PcbDoc-spec.)
- **Nets**: exact case-sensitive name match. This matches Altium's net matching:
  `ListPair_Nets.Find_PartialMatchesByName()` uses case-insensitive comparison as a
  secondary step, but primary matching uses the canonical full net name.
- **Pins**: within a matched component pair, pins are matched by designator string
  (e.g. `"1"`, `"A3"`, `"GND"`). This mirrors `ListPair_Pin`'s use of `DM_Id()`
  as the primary key.

### Consequences

- Designator changes are detectable as a remove + add (component not in the other
  snapshot). A future bidirectional mode with a base snapshot would identify the
  component from the base and produce an explicit rename change instead.
- Multi-part components (e.g. a quad op-amp with parts `U1A`, `U1B`, `U1C`, `U1D`)
  are each individual entries in `SchDocSpec.sheets[].components`. They appear as
  separate `SyncComponent` entries keyed by their full designator.

---

## 4. Projection Functions

Two fallible functions produce a `SyncSnapshot` from a compiled spec model. Projection
fails hard on invalid input (dangling refs, duplicate designators) — the caller must
run `validate_*_spec()` before projecting to catch structural errors early.

Module: `crates/altium-format-spec/src/sync.rs`

```rust
pub fn project_schdoc_spec(spec: &SchDocSpec) -> Result<SyncSnapshot, SpecError>
pub fn project_pcbdoc_spec(spec: &PcbDocSpec) -> Result<SyncSnapshot, SpecError>
```

### SchDocSpec projection

Source data from `model::SchDocSpec`:

```
SchDocSpec
  sheets: Vec<SheetSpec>
    components: Vec<SchDocComponentSpec>   -- designator, symbol, parameters
    nets: Vec<NetSpec>                     -- name, pins: Vec<PinRef>
    powers: Vec<PowerSpec>                 -- name, pins: Vec<PinRef>
```

Projection algorithm:

1. **Components**: collect all `SchDocComponentSpec` entries across all sheets.
   Key by `designator`. For each component, build a `SyncPin` map with `net = None`
   initially (pin connectivity comes from the net list, not from the component).

2. **Nets**: collect all `NetSpec` and `PowerSpec` entries. For each `PinRef`
   (`component`, `pin`) in a net's pin list, set `SyncPin.net = Some(net_name)` on
   the corresponding component's pin.

3. **Footprint**: the `SchDocComponentSpec` does not directly carry a footprint name.
   The footprint comes from the symbol's `FootprintMapSpec` entries in the referenced
   SchLib. Resolving this requires library lookup, which is **out of scope for Phase 1**.
   In Phase 1, `SyncComponent.footprint` is `None` for SchDoc-spec projections unless
   the component has an explicit `footprint` parameter.

4. **Parameters**: copy `parameters: Vec<ParameterSpec>` into
   `SyncComponent.parameters: Map<String, String>`.

### PcbDocSpec projection

Source data from `model::PcbDocSpec`:

```
PcbDocSpec
  boards: Vec<BoardSpec>
    components: Vec<PcbDocComponentSpec>   -- designator, pattern, comment, location...
    nets: Vec<PcbDocNetSpec>               -- name, color
```

Projection algorithm:

1. **Components**: collect all `PcbDocComponentSpec` entries across all boards.
   Key by `designator`. Set `footprint = component.pattern`,
   `comment = component.comment`, `source_library = component.source_library`.
   Build an empty `pins` map (see note below).

2. **Nets**: collect all `PcbDocNetSpec` entries. Set `SyncNet.color = net.color`.

3. **Pin connectivity**: `PcbDocSpec` does not currently represent explicit pin-to-net
   membership on board components (the PCB uses pad objects on the copper layers, not
   a netlist pin list). Phase 1 leaves `SyncComponent.pins` empty for PCB projections.
   Forward sync compares nets at the component level (net exists / does not exist in
   PCB) and pin connectivity is derived from `NetSpec.pins` on the schematic side only.
   Full pin-level PCB sync is deferred to Phase 2 once `PcbDocComponentSpec` carries
   explicit net assignments.

### Example

SchDoc-spec fragment:

```
component R1 {
    symbol: $mylib.R_0603
    designator: "R1"
    parameters {
        Value: "10k"
        Tolerance: "1%"
    }
}

net VCC {
    pins: [U1.14, C1.1]
}

net GND {
    pins: [U1.7, C1.2, R1.2]
}
```

Produces `SyncSnapshot`:

```
components:
  R1:
    designator: "R1"
    footprint: None       -- library lookup deferred
    parameters: {Value: "10k", Tolerance: "1%"}
    pins:
      "2": net = Some("GND")

nets:
  VCC: {pins: [(U1, 14), (C1, 1)]}
  GND: {pins: [(U1, 7), (C1, 2), (R1, 2)]}
```

---

## 5. Diff Algorithm

Module: `crates/altium-format-spec/src/sync.rs`

```rust
pub fn diff_snapshots(source: &SyncSnapshot, target: &SyncSnapshot) -> Vec<SyncChange>
pub fn filter_changes(changes: &[SyncChange], policy: &SyncPolicy, direction: SyncDirection) -> Result<Vec<SyncChange>, SpecError>
```

Produces a flat list of `SyncChange` entries describing what must change in `target`
to match `source`. The direction is always expressed as "what the target is missing
or has wrong relative to the source" — the caller decides which direction to apply.

### SyncChange variants

```rust
enum SyncChange {
    // Component-level
    AddComponent { designator: String, data: SyncComponent },
    RemoveComponent { designator: String },
    UpdateComponent { designator: String, fields: Vec<FieldChange> },

    // Net-level
    AddNet { name: String, data: SyncNet },
    RemoveNet { name: String },
    UpdateNet { name: String, fields: Vec<FieldChange> },

    // Pin connectivity
    AddPin { component: String, pin: String, net: NetName },
    RemovePin { component: String, pin: String, net: NetName },
    UpdatePin { component: String, pin: String, old_net: NetName, new_net: NetName },
}

struct FieldChange {
    field: String,
    old_value: Option<String>,
    new_value: Option<String>,
}
```

### Diff procedure

```
1. For each component in source:
     If not in target:                      -> AddComponent
     Else:
       Compare comment, footprint, source_library, parameters
       If any differ:                       -> UpdateComponent (list changed fields)
       For each pin in source component:
         If not in target component:        -> AddPin
         Else if net differs:              -> UpdatePin
       For each pin in target component not in source: -> RemovePin

2. For each component in target not in source:
                                            -> RemoveComponent (with its pins)

3. For each net in source:
     If not in target:                      -> AddNet
     Else:
       Compare color
       If differs:                          -> UpdateNet

4. For each net in target not in source:
                                            -> RemoveNet
```

This mirrors Altium's `ListPair_Components` + `ListPair_Nets` two-pass algorithm
described in `sch-pcb-mapping.md` §9, and the general "removes before adds" ordering
principle from `eco-validation-pipeline.md` §Change Ordering.

### What the diff does NOT include

- Component location, rotation, layer — never synced (see §2 rationale).
- PCB-internal primitives (tracks, vias, polygons, rules) — out of domain.
- Graphical/presentation properties of the schematic — out of domain.
- Pin swap group metadata (`SwapIdPin`, `SwapIdPart`) — Phase 3 only.

---

## 6. Directionality Policy

A `SyncPolicy` determines which properties flow in which direction.

Module: `crates/altium-format-spec/src/sync.rs`

```rust
pub enum SyncDirection {
    Forward,        // SchDoc-spec is authoritative, push to PcbDoc-spec
    Back,           // PcbDoc-spec is authoritative, push to SchDoc-spec
    Bidirectional,  // Detect which side changed (requires base snapshot, Phase 3)
    None,           // Property is domain-specific, never synced
}

pub struct SyncPolicy {
    pub comment:             SyncDirection,
    pub footprint:           SyncDirection,
    pub source_library:      SyncDirection,
    pub parameters:          SyncDirection,
    pub net_name:            SyncDirection,
    pub net_color:           SyncDirection,
    pub pin_net_assignment:  SyncDirection,
    pub component_location:  SyncDirection,
}
```

`SyncPolicy` has no `Default` impl. The CLI always constructs an explicit policy with
named directions per property; an all-`None` default would silently skip all sync.

### Phase 1 forward sync policy

```
comment:             Forward
footprint:           None    (SchDoc projection always yields None — no footprint in spec text)
source_library:      None    (SchDoc projection always yields None)
parameters:          Forward
net_name:            Forward
net_color:           None    (excluded Phase 1 — Altium applies system defaults)
pin_net_assignment:  None    (excluded Phase 1 — PcbDoc spec lacks pin connectivity)
component_location:  None    (PCB placement is board-designer's responsibility, never synced)
```

### Rationale for each direction

**Forward (SchDoc-spec is authoritative)**

- `comment`: Component comments originate in the schematic (BOM-facing). Same as
  Altium `eModification_ChangeComponentComment` which flows Sch→PCB.
- `net_name`: Net identity is defined in the schematic. Same as Altium
  `eModification_ChangeNetName`.

**None (never synced in Phase 1)**

- `footprint`, `source_library`: SchDoc specs do not carry footprint assignments
  directly. Syncing None forward would silently clear all PcbDoc footprint values.
  Footprint sync deferred to Phase 2 resolver.
- `net_color`: Specs typically omit net colors. Syncing None would clear PcbDoc
  display colors. Phase 3 may add color sync.
- `pin_net_assignment`: PcbDoc spec lacks explicit pin-to-net connectivity. Phase 2.
- `component_location`, `component_rotation`, `component_layer`: PCB placement
  decisions. Altium excludes `DM_LocationX/Y`, `DM_Rotation`, `DM_Layer` from ECO.

---

## 7. Change Application

Module: `crates/altium-format-spec/src/sync.rs`

```rust
pub fn apply_sync_changes_to_pcbdoc(
    changes: &[SyncChange],
    spec: &mut PcbDocSpec,
) -> Result<(), SpecError>
```

`spec.boards` must contain exactly one board (`boards.len() == 1`); multi-board specs
return a hard error. All changes are applied to `boards[0]`.

Changes are applied to the target `SpecModel` in Altium's dependency ordering
(drawn from `eco-validation-pipeline.md` §Change Ordering):

```
1. RemovePin (remove pins from nets first)
2. RemoveNet
3. RemoveComponent
4. UpdateComponent
5. AddComponent
6. AddNet
7. AddPin (add pins to nets last, after their nets and components exist)
8. UpdatePin
9. UpdateNet
```

This ordering prevents referential errors where a net is added before its
owning component exists, or a pin is removed from a net that no longer exists.

### Application to PcbDocSpec (forward sync)

When applying changes to a `PcbDocSpec`:

- `AddComponent`: append a new `PcbDocComponentSpec` to `boards[0].components` with
  `designator`, `pattern` (from footprint), `comment`, `source_library`. Set
  `location = None`, `rotation = None`, `layer = None` — placement is the board
  designer's responsibility.
- `RemoveComponent`: remove the matching `PcbDocComponentSpec` by designator.
- `UpdateComponent`: update `pattern`, `comment`, `source_library` fields as needed.
  Never touch `location`, `rotation`, or `layer`.
- `AddNet`: append a new `PcbDocNetSpec` with `name` and `color`.
- `RemoveNet`: remove the matching `PcbDocNetSpec` by name.
- `UpdateNet`: update `color`.
- `AddPin`, `RemovePin`, `UpdatePin`: currently no-op in Phase 1 (PcbDoc nets do not
  carry explicit pin membership). These become meaningful in Phase 2 when the spec
  format gains pin-to-net connectivity for board components.

### Application to SchDocSpec (back-annotation)

When applying back-annotation changes to a `SchDocSpec`:

- `UpdateComponent` with `designator` field change: rename the component's designator
  in `components` and update all `PinRef`s in `nets` and `powers` that reference the
  old designator.
- `UpdatePin` (pin swap, Phase 3): move a `PinRef` from one `NetSpec` to another.
- Location, footprint, comment back-annotation: by policy, these flow Forward only,
  so they produce no changes during back-annotation.

### Idempotency

Apply is idempotent when the change list accurately reflects the diff. Running the
same forward sync twice in a row produces no changes on the second run, because the
target spec will already match the source after the first application.

---

## 8. Three-Way Merge (Future — Phase 3)

The two-way diff (source vs target) is sufficient for forward sync and simple
back-annotation. For true bidirectional sync — detecting which side changed
independently and merging without overwriting — a **base snapshot** is required.

```
base   = SyncSnapshot from last synced state
sch    = SyncSnapshot from current SchDocSpec
pcb    = SyncSnapshot from current PcbDocSpec

forward_changes = diff_snapshots(sch, base)   -- what Sch changed since last sync
back_changes    = diff_snapshots(pcb, base)   -- what PCB changed since last sync
```

A property changed in only one side is applied unambiguously. A property changed in
both sides (conflicting edits) requires a conflict resolution strategy:

- **Last-write wins**: not safe without timestamps.
- **Forward wins**: schematic always wins; PCB back changes are ignored when schematic
  also changed the same property.
- **User resolves**: produce a conflict report and require explicit resolution.

The base snapshot must be persisted somewhere between sync operations. Candidates:

- A `.sync-base` snapshot file alongside the spec files.
- A git-tracked intermediate representation (the snapshot is small and human-readable).
- Embedded as a comment block within the spec file itself.

The storage format and conflict policy are deferred to Phase 3 design.

---

## 9. CLI Interface

### Phase 1: Forward sync

```
altium spec sync --forward myschematic.schdoc-spec myboard.pcbdoc-spec
altium spec sync --forward --dry-run myschematic.schdoc-spec myboard.pcbdoc-spec
altium spec sync --diff myschematic.schdoc-spec myboard.pcbdoc-spec
```

Pipeline:

1. Parse both spec files (`compile_spec`).
2. Validate each spec (`validate_schdoc_spec`, `validate_pcbdoc_spec`).
   - `Ok(warnings)`: print each warning to stderr, continue.
   - `Err(errors)`: join errors, return early.
3. Project to `SyncSnapshot` (`project_schdoc_spec`, `project_pcbdoc_spec`).
4. Diff: `diff_snapshots(&sch_snapshot, &pcb_snapshot)`.
5. Filter: `filter_changes(&changes, &policy, SyncDirection::Forward)`.
6. Without `--dry-run`: apply changes to `PcbDocSpec` (`apply_sync_changes_to_pcbdoc`),
   write back to disk via atomic temp-file-then-rename.
7. Print ECO report (`render_eco_report`).

`--diff` runs steps 1–5 and prints the diff without applying or writing.

`--dry-run` runs the full pipeline including step 6 in memory but skips writing to disk.

The spec file is written back using `rewrite_pcbdoc_spec_with_changes()`, which uses
byte-offset spans to rewrite only the changed blocks, preserving user comments and
formatting in unchanged regions.

### Phase 2: Back-annotation (future)

```
altium spec sync --back myschematic.schdoc-spec myboard.pcbdoc-spec
```

Not implemented in Phase 1. The `filter_changes` infrastructure supports `Back`
direction as forward-compatible scaffolding.

### Phase 3: Bidirectional (future)

```
altium spec sync --bidirectional myschematic.schdoc-spec myboard.pcbdoc-spec
```

Requires a base snapshot for three-way merge. Storage format deferred to Phase 3 design.

---

## 10. What This Design Does NOT Cover

The following are explicitly out of scope for the SyncSnapshot IR:

- **Pin/part swaps** (Phase 3): requires `SwapIdPin`, `SwapIdPart`, and `PairSwapID`
  metadata from SchLib components. The `PinSwapManager` algorithm described in
  `pin-swap-back-annotation.md` operates on compiled pin lists with swap group
  membership; replicating it requires that the spec's `PinSpec.swap_group` and
  `part_swap_group` fields are populated and that the PCB reports a concrete pin
  swap (connectivity change on a swappable pin).

- **Design rule generation from schematic directives**: schematic `ParameterSet`
  directives and blankets can generate PCB rules. `sch-pcb-mapping.md` §7 documents
  this; it requires rule syntax in the PcbDoc-spec.

- **Multi-sheet hierarchy**: `SchDocSpec.sheets` is currently a flat list. Altium's
  hierarchical UniqueIdPath and multi-channel designator prefixing
  (`sch-pcb-mapping.md` §6) are not represented. All components across sheets are
  projected into a single flat `SyncSnapshot.components` map; name collisions across
  sheets would require disambiguation.

- **Net classes / differential pairs** (marked as "future" in the snapshot structure):
  these require `PcbDocClassSpec` and `PcbDocDifferentialPairSpec` entries and
  corresponding schematic directive parsing.

- **Harness ECOs**: harness net types, splices, cables are a separate document domain
  with their own modification kinds (`eModification_AnnotateHarness*`). Not in scope.

- **Vault / managed component GUIDs**: `eModification_ChangeManagedComponentLibraryLink`,
  `VaultGUID`, `ItemGUID`, `RevisionGUID` — no managed-component concept in spec files.

- **Simulation models**: `eModification_ChangeComponentSimulationModel` — no SIM model
  support in the spec format.

---

## 11. Implementation Phases

### Phase 1: Forward sync (SchDoc-spec → PcbDoc-spec)

Deliverables:

1. `SyncSnapshot` struct and associated types in `altium-format-spec`.
2. `project_schdoc_spec(spec: &SchDocSpec) -> SyncSnapshot`.
3. `project_pcbdoc_spec(spec: &PcbDocSpec) -> SyncSnapshot`.
4. `diff_snapshots(source: &SyncSnapshot, target: &SyncSnapshot) -> Vec<SyncChange>`.
5. `apply_sync_changes(changes: &[SyncChange], target: &mut PcbDocSpec, policy: &SyncPolicy)`.
6. CLI command: `altium spec sync --forward <schdoc-spec> <pcbdoc-spec>`.
7. ECO report output using existing `EngineeringChangeOrder` / `render_text()`.

Scope limitations:
- No footprint resolution from SchLib (footprint field left empty in Sch projection).
- No pin connectivity changes applied to PcbDoc-spec (AddPin/RemovePin/UpdatePin are
  generated in the diff but are no-ops in application until PcbDocNetSpec carries pin
  membership).

### Phase 2: Back-annotation (PcbDoc-spec → SchDoc-spec)

Deliverables:

1. `apply_sync_changes` target extended to `SchDocSpec`.
2. Designator rename: update all `PinRef` references when a component designator changes.
3. CLI command: `altium spec sync --back <schdoc-spec> <pcbdoc-spec>`.

### Phase 3: Pin/part swaps and bidirectional merge

Deliverables:

1. Base snapshot persistence (format TBD).
2. Three-way merge: `merge_snapshots(base, sch, pcb) -> MergeResult`.
3. Conflict detection and reporting.
4. Pin swap detection: identify `UpdatePin` changes that constitute valid swap group
   operations, using `PinSpec.swap_group` from SchLib metadata.
5. CLI command: `altium spec sync --bidirectional <schdoc-spec> <pcbdoc-spec>`.

---

## 12. Supporting Evidence: Altium Research

The following findings from the decompiled AD26 codebase inform and validate the
design decisions above.

### Component matching fallback (sch-pcb-mapping.md §1)

`DocumentComparator_ComponentSynchronizer` runs when UniqueID-based matching fails.
It matches components by `DM_PhysicalDesignator()`. This is exactly the designator-based
matching this IR uses as its primary (and only) strategy.

### Net matching by name (document-adapters.md §Net Matching Algorithm)

`NetAdapter.DM_FullNetName()` is the canonical key for net matching. No net UUID exists
in Altium's UDM. Our `SyncNet.name` as the primary key is structurally identical.

### Pin matching by designator (sch-pcb-mapping.md §3)

`ListPair_Pin` matches on `DM_Id()` (pin designator) + `DM_PhysicalPartDesignator()`
(component designator). `SyncSnapshot` pins are keyed identically.

### Execution ordering (eco-validation-pipeline.md §Change Ordering)

Altium's `ChangeManagerUtils.ModificationOrder[]` array encodes removes-before-adds,
containers-before-members. The §7 application ordering above follows the same principle.

### Pin-to-net as the forward sync primitive (eco-change-types.md §Node Operations)

`eModification_AddNode` (value 16) and `eModification_RemoveNode` (value 1) are the
ECO types for adding and removing pins from nets. These correspond directly to
`SyncChange::AddPin` and `SyncChange::RemovePin`. The name "node" in Altium's term
maps to "pin on a net" in our vocabulary.

### What ECO does NOT persist (eco-change-types.md §Serialization / Persistence)

Altium's ECO objects are transient in-memory objects — they are generated, reviewed,
applied, and discarded. The only file artifact is the optional `.ECO` report. This
confirms that the `SyncSnapshot` IR should also be ephemeral: it is a computation
artifact, not a file format to be stored.

### ECOCopies and snapshot semantics (document-adapters.md §ECO Snapshot Creation)

`ECOUtils.CreateDocumentForECO()` creates a snapshot copy of the compiled schematic
with footprint pin mapping applied — a read-only projection of the current state used
only for comparison. `SyncSnapshot` is the same concept at the spec level: a read-only
projection of the current spec state, used only for diffing.

### Schematic-only properties excluded from ECO (sch-pcb-mapping.md §1)

The explicit list of properties excluded from Altium's ECO comparison validates the
`SyncDirection::None` policy for `component_location`. No mechanism in Altium pushes
PCB placement coordinates from either direction as part of the ECO pipeline.

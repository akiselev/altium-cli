# Netlist Sync: Pad-to-Net Assignments from Schematics to Routing

## Problem

The routing pipeline requires `IrNet.pins` to be populated. When these are empty, the
router has nothing to route. Currently, ee-template boards route with 0mm traces because:

1. **Merge destroys pad geometry**: `merge_pcbdoc_spec()` replaces the entire component
   vec when the spec declares ANY components. Since pcbdoc-spec components have
   `pads: Vec::new()`, all imported pad geometry and net assignments from the PcbDoc
   binary are lost.

2. **Sync blocks pin-level changes**: `sync.rs` explicitly rejects `AddPin`/`UpdatePin`
   changes with `SyncDirection::None`, so schematic net assignments never flow to the
   PCB spec.

## Architecture

```
schdoc-spec (pin 1 -> #VCC_3V3)
     ↓ sync
pcbdoc-spec (pad_net 1: "VCC_3V3" on component)
     ↓ compile + merge with PcbDoc import
PcbIr (IrNet.pins populated)
     ↓ route
.routes file
```

## Implementation Phases

### Phase 1: Per-Component Merge (CRITICAL — fixes pad geometry loss)

**File**: `crates/autopcb-ir/src/pcbdoc_import.rs`

Change `merge_board_spec()` component merge from vec-level replacement to
per-component merge by designator:

- Spec properties win: pattern, comment, location, rotation, layer, source_library, parameters
- **Imported pads preserved** when spec pads are empty
- Spec pads win when non-empty
- Imported-only components kept (PcbDoc has components spec doesn't mention)
- Spec-only components kept (spec declares components not yet in PcbDoc)

This single change restores pad geometry and net assignments from the PcbDoc import,
immediately enabling routing on any PcbDoc that has had ECO applied.

### Phase 2: `pad_net` Syntax in pcbdoc-spec

Add pad-to-net assignment syntax to component declarations:

```
component J1 {
    pattern: "HRO_TYPE-C-31-M-12"
    pad_net CC1: "USB_CC1"
    pad_net CC2: "USB_CC2"
    pad_net DP1: "USB_DP_CONN"
}
```

**Files**:
- `ast.rs`: Add `PadNetDecl` variant to component items
- `parser.rs`: Parse `pad_net <designator>: <net>` inside component blocks
- `compiler.rs`: Collect pad_net declarations into `PcbDocComponentSpec.pad_nets`
- `model.rs`: Add `pad_nets: IndexMap<String, String>` to `PcbDocComponentSpec`

### Phase 3: Enable Pin-Level Sync

**File**: `crates/altium-format-spec/src/sync.rs`

1. Handle `AddPin`/`UpdatePin`/`RemovePin` in `apply_sync_changes_to_pcbdoc()`
2. Change `SyncDirection::None` to `SyncDirection::Forward` for `pin_net_assignment`
3. Extend text rewriter to emit `pad_net` lines in component blocks
4. Fix `render_eco_report()` for pin change variants

**File**: `crates/altium-cli/src/main.rs`
- Flip `pin_net_assignment: SyncDirection::Forward` in sync policy

### Phase 4: Wire pad_nets into Merge

**File**: `crates/autopcb-ir/src/pcbdoc_import.rs`

In per-component merge, apply spec `pad_nets` on top of imported pad.net values:
- For each `(pad_designator, net_name)` in `spec_comp.pad_nets`:
  - Find matching pad in imported pads by designator
  - Set `pad.net = Some(net_name)`

### Phase 5: Formatter + Dump

- `formatter.rs`: Format `pad_net` declarations
- `dump.rs`: Emit `pad_net` lines when dumping PcbDoc with pad-net assignments

## Priority

Phase 1 is the highest-impact change — it unblocks routing on any PcbDoc that has had
Altium ECO applied (pad-to-net assignments already in the binary). Phases 2-4 enable the
pure spec-driven workflow where schematics provide the netlist.

## Immediate Test

After Phase 1, `altium routing solve --target cobra.PcbDoc cobra-route.pcbdoc-spec`
should route 18/18 nets (cobra.PcbDoc has pad-to-net assignments from the original
Altium project).

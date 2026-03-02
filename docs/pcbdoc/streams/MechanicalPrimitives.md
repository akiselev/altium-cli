# MechanicalPrimitives Section

## Overview

The `MechanicalPrimitives` section is a **conditional TPrimitivesSection** that stores
a **separate copy** of all PCB primitives residing on mechanical layers. It was introduced
as part of the "Mechanical Layer Types" feature (gated behind `IPCBFeatures.MechanicalLayerTypes()`).

Unlike most TPrimitivesSection subclasses which own a specific primitive type (e.g.,
TTracksSection owns tracks, TArcsSection owns arcs), TMechanicalPrimitivesSection
cross-cuts **all primitive types** -- it collects primitives from *any* section whose
layer is a mechanical layer. During load, primitives read from this section are
dispatched back to their correct owner section via a layer-to-section remapping mechanism.

**Section index**: 63 (in the stream name table)
**CFB storage name**: `MechanicalPrimitives`
**Section name**: `Section_MechanicalPrimitives`
**Delphi class**: `TMechanicalPrimitivesSection` (instance size 0xB8, base + 8 bytes)

## Feature Gate

Both reading and writing this section are conditional on the `MechanicalLayerTypes`
feature flag:

```c
// Pseudocode from FUN_04712f70 — called before Import and Export
bool hasMechanicalLayerTypes = board->Features->MechanicalLayerTypes();
if (!hasMechanicalLayerTypes) {
    return;  // skip entire section
}
```

This means:
- **Older files** (pre-feature) will NOT contain this section
- **AD26 files** with the feature enabled WILL contain it
- If the feature is disabled in the license/edition, the section is skipped even if present

## Class Hierarchy

```
TPrimitivesSection (base, 0xB0 = 176 bytes)
  └── TMechanicalPrimitivesSection (0xB8 = 184 bytes, +8 bytes)
```

The extra 8 bytes (at offset 0xA8) are a pointer to a list that collects
(mechanical-layer, original-layer) remapping records during import, used to
dispatch primitives back to their owning sections after deserialization.

## CFB Storage Layout

Standard TPrimitivesSection format:

```
/MechanicalPrimitives/
    Header      — block-encoded text stream (parameter pairs)
    Data        — block-encoded binary stream (record data)
```

Each record in `Data` is a standard PCB binary primitive record (same format as the
record would have in its native section — Tracks6, Arcs6, Fills6, Texts6, Regions6, etc.).
The record's object ID byte determines its type, same as in all other sections.

## Data Format

### Header Stream

Standard TPrimitivesSection header format:
```
|HEADER=Section_MechanicalPrimitives|
```

The header block contains the record count and other standard section parameters.

### Data Stream

Each record in the Data stream is a standard PCB binary primitive in its native format.
The record format is **identical** to how that primitive type appears in its own section
(e.g., a track on a mechanical layer is serialized exactly as it would be in `Tracks6/Data`).

Records can be any primitive type that can exist on a mechanical layer:
- Tracks (object ID 0x04)
- Arcs (object ID 0x01)
- Fills (object ID 0x06)
- Texts (object ID 0x05)
- Regions (object ID 0x0B)
- Pads (object ID 0x02)
- Component Bodies (object ID 0x12)
- Dimensions (various object IDs)
- Other primitive types with a mechanical layer assignment

The actual primitive type is determined by the object ID byte at the start of each
binary record, following the standard PCB binary record dispatching.

## Export Logic (Save)

During save, the section iterates ALL primitives in the board and selects those on
mechanical layers:

### Export Filter (FUN_0485ee80)

```c
// Pseudocode for ShouldExportPrimitive(board, primitive)
bool ShouldExportPrimitive(TMechanicalPrimitivesSection* self, IPCB_Primitive* prim) {
    TV7_Layer layer = prim->GetState_Layer();  // vtable offset 0x368

    // Skip eV6_NoLayer
    if (IsNoLayer(layer)) return false;

    bool isMechanical = false;

    if (layer.Flags == 0) {
        // V6-compatible layer encoding (Flags bytes are zero)
        // Check if V6 layer byte is in the mechanical range
        uint8_t v6 = (uint8_t)layer.ID;
        if ((v6 - 0x38) < 0x20) {
            // Bitmask 0x0001FFFE selects exactly Mechanical1..Mechanical16
            // (V6 layer values 57..72, offsets 1..16 from base 0x38=56)
            isMechanical = ((1 << (v6 - 0x38)) & 0x0001FFFE) != 0;
        }
    } else {
        // V7-extended layer encoding
        // Flags == 0x0400 means Family=4 = mechanical layer partition
        isMechanical = (layer.Flags == 0x0400);
    }

    if (isMechanical) {
        // Additional check: get the REMAPPED layer and compare
        TV7_Layer remapped = GetRemappedLayer(prim);  // via layer kind mapping
        return (remapped != layer);  // only include if layer was remapped
    }

    return false;
}
```

The bitmask `0x0001FFFE` corresponds to bits 1-16 relative to base layer 0x38 (56):
- bit 0 = layer 56 = `eV6_KeepOutLayer` → NOT selected
- bits 1-16 = layers 57-72 = `eV6_Mechanical1` through `eV6_Mechanical16` → SELECTED
- bits 17+ = DrillDrawing, MultiLayer, etc. → NOT selected

### PrepareToSave (FUN_0485f150)

Before writing records, this function counts how many primitives pass the export filter
by iterating all primitives and checking `ShouldExportPrimitive()`:

```c
void PrepareToSave(TMechanicalPrimitivesSection* self) {
    self->recordCount = 0;
    IPCB_Primitive* prim = self->primitiveList->First();
    while (prim != NULL) {
        if (ShouldExportPrimitive(self, prim)) {
            self->recordCount++;
        }
        prim = self->primitiveList->Next();
    }
}
```

### WriteRecord (FUN_0485f070)

For each primitive that passes the filter, the section serializes it using the
standard TPrimitivesSection record writing mechanism, then stores a (primitive, layer)
mapping entry so the primitive can be associated with its original section.

## Import Logic (Load)

### Import Loop (FUN_0485e9c0)

```c
bool Import_FromFile(TMechanicalPrimitivesSection* self,
                     IPCB_StructuredStorage* storage,
                     bool* success) {
    if (!MechanicalLayerTypes()) {
        *success = false;
        return 0;
    }

    *success = false;
    if (!OpenStorage(self)) return 0;

    int recordCount = ReadRecordCount(self);

    for (int i = 0; i < recordCount; i++) {
        CheckForCancel();
        if (!cancelled) {
            ReadNextRecord(self);       // standard TPrimitivesSection read
            ProcessImportedRecord(self); // FUN_0485ef30 — layer remapping
        }
    }

    *success = true;
    FinalizeRead(self);
    return 0;
}
```

### ProcessImportedRecord (FUN_0485ef30)

After reading each record, this function:
1. Reads the record's parameter string to extract a layer-mapping key
2. Looks up the key in a constant mapping table
3. Creates a remapping entry (original layer → target layer) and stores it in
   the list at offset 0xA8

This allows the loader to re-assign primitives from the MechanicalPrimitives section
back to their correct owning sections.

## Key Types

### TMechanicalLayerKind (byte enum)

Defines the semantic purpose of a mechanical layer:

| Value | Name | Description |
|-------|------|-------------|
| 0 | `mlUndefined` | No assigned type |
| 1 | `mlAssemblyTop` | Assembly drawing (top) |
| 2 | `mlAssemblyBottom` | Assembly drawing (bottom) |
| 3 | `mlAssemblyNotes` | Assembly notes |
| 4 | `mlBoard` | Board outline |
| 5 | `mlCoatingTop` | Conformal coating (top) |
| 6 | `mlCoatingBottom` | Conformal coating (bottom) |
| 7 | `mlComponentCenterTop` | Component center (top) |
| 8 | `mlComponentCenterBottom` | Component center (bottom) |
| 9 | `mlComponentOutlineTop` | Component outline (top) |
| 10 | `mlComponentOutlineBottom` | Component outline (bottom) |
| 11 | `mlCourtyardTop` | Courtyard (top) |
| 12 | `mlCourtyardBottom` | Courtyard (bottom) |
| 13 | `mlDesignatorTop` | Designator (top) |
| 14 | `mlDesignatorBottom` | Designator (bottom) |
| 15 | `mlDimensions` | Dimensions |
| 16 | `mlDimensionsTop` | Dimensions (top) |
| 17 | `mlDimensionsBottom` | Dimensions (bottom) |
| 18 | `mlFabNotes` | Fabrication notes |
| 19 | `mlGluePointsTop` | Glue points (top) |
| 20 | `mlGluePointsBottom` | Glue points (bottom) |
| 21 | `mlGoldPlatingTop` | Gold plating (top) |
| 22 | `mlGoldPlatingBottom` | Gold plating (bottom) |
| 23 | `mlValueTop` | Component value (top) |
| 24 | `mlValueBottom` | Component value (bottom) |
| 25 | `mlVCut` | V-cut scoring |
| 26 | `ml3DBodyTop` | 3D body (top) |
| 27 | `ml3DBodyBottom` | 3D body (bottom) |
| 28 | `mlRouteToolPath` | Route tool path |
| 29 | `mlSheet` | Sheet |
| 30 | `mlBoardShape` | Board shape |
| 31 | `mlOverlayTop` | Overlay (top) |
| 32 | `mlOverlayBottom` | Overlay (bottom) |
| 33 | `mlSolderTop` | Solder (top) |
| 34 | `mlSolderBottom` | Solder (bottom) |
| 35 | `mlPasteTop` | Paste (top) |
| 36 | `mlPasteBottom` | Paste (bottom) |
| 37 | `mlTentingTop` | Tenting (top) |
| 38 | `mlTentingBottom` | Tenting (bottom) |
| 39 | `mlCoveringTop` | Covering (top) |
| 40 | `mlCoveringBottom` | Covering (bottom) |
| 41 | `mlPluggingTop` | Plugging (top) |
| 42 | `mlPluggingBottom` | Plugging (bottom) |
| 43 | `mlFilling` | Filling |
| 44 | `mlCapping` | Capping |
| 45 | `mlDiePadsTop` | Die pads (top) |
| 46 | `mlDiePadsBottom` | Die pads (bottom) |
| 47 | `mlWirebondingTop` | Wire bonding (top) |
| 48 | `mlWirebondingBottom` | Wire bonding (bottom) |

### TMechanicalLayerPairKind (byte enum)

Defines paired relationships between mechanical layers:

| Value | Name | Description |
|-------|------|-------------|
| 0 | `mlpUndefined` | No pairing |
| 1 | `mlpAssembly` | Assembly top/bottom pair |
| 2 | `mlpCoating` | Coating top/bottom pair |
| 3 | `mlpComponentCenter` | Component center pair |
| 4 | `mlpComponentOutline` | Component outline pair |
| 5 | `mlpCourtyard` | Courtyard pair |
| 6 | `mlpDesignator` | Designator pair |
| 7 | `mlpDimensions` | Dimensions pair |
| 8 | `mlpGluePoints` | Glue points pair |
| 9 | `mlpGoldPlating` | Gold plating pair |
| 10 | `mlpValue` | Value pair |
| 11 | `mlp3DBody` | 3D body pair |
| 12 | `mlpOverlay` | Overlay pair |
| 13 | `mlpSolder` | Solder pair |
| 14 | `mlpPaste` | Paste pair |
| 15 | `mlpTenting` | Tenting pair |
| 16 | `mlpCovering` | Covering pair |
| 17 | `mlpPlugging` | Plugging pair |
| 18 | `mlpDiePads` | Die pads pair |
| 19 | `mlpWirebonding` | Wire bonding pair |

### TMechanicalLayerToKindItem (packed struct)

Maps a V7 layer ID to its mechanical kind:

```c
struct TMechanicalLayerToKindItem {  // Pack = 1
    TV7_Layer Layer;                  // 4 bytes — V7 layer ID
    TMechanicalLayerKind Kind;        // 1 byte — mechanical kind
};
```

### V7 Layer Encoding for Mechanical Layers

The `TV7_Layer` struct (4 bytes, union layout):

```c
struct TV7_Layer {        // Pack = 1, LayoutKind.Explicit
    uint32_t ID;          // offset 0: full 32-bit value
    uint16_t Species;     // offset 0: lower 16 bits
    uint8_t  Genus;       // offset 2: layer partition selector
    uint8_t  Family;      // offset 3: layer family
    uint16_t N;           // offset 0: species alias
    uint16_t Flags;       // offset 2: genus+family as 16-bit
};
```

For mechanical layers:
- **V6 encoding** (Mechanical 1-16): `ID = 57 + (index - 1)`, `Flags = 0`
  - Mechanical1 = 57 = 0x39, ..., Mechanical16 = 72 = 0x48
- **V7 encoding** (Mechanical 17+): `Species = index`, `Family = 4` → `Flags = 0x0400`
  - Allows up to 65535 mechanical layers

Detection: `IsMechanicalLayer()` checks:
1. `Flags == 0` AND byte 0 of ID is in V6MechanicalLayers array (57-72), OR
2. `Flags == 0x0400` (Family = 4 = mechanical layer partition)

## Relationship to Other Sections

The MechanicalPrimitives section is **NOT** an independent data store. It is a
**serialization-time optimization** introduced with the Mechanical Layer Types feature:

1. **During save**: Primitives on mechanical layers are DUPLICATED into this section.
   They remain in their original sections (Tracks6, Arcs6, etc.) but are ALSO written
   here with remapping metadata.

2. **During load**: Records from this section are read and dispatched back to their
   owning sections. The remapping entries handle the case where a layer's identity
   changed (e.g., a mechanical layer was reassigned from generic Mechanical4 to a
   typed role like `mlCourtyardTop`).

3. **Without the feature**: This section does not exist. Mechanical layer primitives
   live exclusively in their native sections.

## Relationship to LayerKindMapping

The MechanicalPrimitives section works in tandem with the `LayerKindMapping` section:
- LayerKindMapping stores the mapping from V7 layer IDs to TMechanicalLayerKind values
- MechanicalPrimitives uses this mapping during export to determine which primitives
  need remapping entries and during import to re-dispatch them correctly

## Source References

### Delphi (Advpcb.dll)
| Address | Function | Purpose |
|---------|----------|---------|
| `0x0485e850` | TMechanicalPrimitivesSection.Create | Constructor (calls parent, inits remap list at +0xA8) |
| `0x0485e9c0` | TMechanicalPrimitivesSection.Import_FromFile | Load: feature-gated, reads records with remapping |
| `0x0485eb30` | TMechanicalPrimitivesSection.Export_ToFile | Save: feature-gated, filters mechanical layer prims |
| `0x0485ee80` | (export filter) | Tests if primitive is on a mechanical layer |
| `0x0485ef30` | (import record processor) | Creates layer remap entry for imported record |
| `0x0485f070` | (write record) | Serializes single prim with layer mapping |
| `0x0485f150` | (prepare to save) | Counts qualifying primitives for record count |
| `0x04712f70` | (feature check wrapper) | Calls `Features->MechanicalLayerTypes()` |
| `0x0485ef24` | (constant) | Bitmask `0x0001FFFE` for V6 mechanical layers |

### C# (.NET)
| File | Type/Method | Purpose |
|------|-------------|---------|
| `Altium.SDK.Interfaces/PCB/TMechanicalLayerKind.cs` | `TMechanicalLayerKind` | SDK enum (no explicit values) |
| `Altium.Edp.Interfaces/RT_PCB/TMechanicalLayerKind.cs` | `TMechanicalLayerKind : byte` | Runtime enum |
| `Altium.Edp.Interfaces/RT_PCB/TMechanicalLayerKindConsts.cs` | First/Last | Range: mlUndefined..mlWirebondingBottom |
| `Altium.SDK.Interfaces/PCB/TMechanicalLayerPairKind.cs` | `TMechanicalLayerPairKind` | Pair kinds |
| `Altium.Edp.Interfaces/RT_PCB/TMechanicalLayerToKindItem.cs` | `TMechanicalLayerToKindItem` | Layer→Kind mapping entry |
| `Altium.Edp.Interfaces/RT_PCB/IMechanicalLayerKindMap.cs` | `IMechanicalLayerKindMap` | Mapping interface |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_MechanicalLayer.cs` | `IPCB_MechanicalLayer` | Layer properties (enabled, kind, etc.) |
| `Altium.SDK.Interfaces/PCB/IPCB_MechanicalLayerHelper.cs` | Extension methods | Get/SetState_Kind typed overloads |
| `Altium.Edp.Interfaces/RT_ProductFeatureInterfaces/IPCBFeatures.cs` | `MechanicalLayerTypes()` | Feature gate flag |
| `Altium.Edp.Classes/PCB/V7_Layer.cs` | `MechanicalLayer()` | V7 layer encoding for mechanical layers |
| `Altium.Edp.Classes/PCB/V7_Layer.cs` | `IsMechanicalLayer()` | Layer type detection |

## Implementation Notes

1. **Not present in test files**: None of the current test PcbDoc fixtures contain this
   section, likely because they were created before the MechanicalLayerTypes feature or
   with an edition that lacks it.

2. **Conditional parsing**: The section MUST be feature-gated. If the feature flag is not
   set, the section should be skipped entirely (not errored).

3. **No new primitive types**: This section contains the SAME primitive types as other
   sections (tracks, arcs, fills, regions, texts, pads, etc.). There is no special
   "mechanical primitive" record type.

4. **Layer remapping during load**: The key complexity is that during import, each record
   must be re-dispatched to its correct owning section. The remapping list at offset 0xA8
   handles this.

5. **Duplicate data**: During save, the same primitive data exists in BOTH the native
   section (e.g., Tracks6) and in MechanicalPrimitives. This is intentional redundancy
   for the layer remapping feature.

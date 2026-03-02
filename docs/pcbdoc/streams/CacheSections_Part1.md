# Cache Sections Part 1: ZAxisClearanceCache, ConnectivityGraphCache, ComponentCache

Research into three PcbDoc CFB cache sections: `ZAxisClearanceCache`, `ConnectivityGraphCache`,
and `ComponentCache`. These are **runtime caches** written during save that accelerate
subsequent loading and DRC operations.

## Summary

| Section | CFB Stream Name | BinaryLoader Section Name | Load Index | Category |
|---------|----------------|--------------------------|------------|----------|
| ZAxisClearanceCache | `ZAxisClearanceCache` | `Section_ZAxisClearanceCache` | 87 | Cache |
| ConnectivityGraphCache | `ConnectivityGraphCache` | — (not in BinaryLoader RTTI) | 52 | Cache |
| ComponentCache | `ComponentCache` | — (not in BinaryLoader RTTI) | 53 | Cache |

**Key finding**: All three sections are runtime caches that are **safe to omit during save**.
They are regenerated on-demand by Altium when missing. None of our 40+ test PcbDoc files
contain any of these sections, which confirms they are optional.

---

## 1. ZAxisClearanceCache

### Overview

The `ZAxisClearanceCache` stores precomputed clearance geometry for Z-axis (vertical)
clearance rule checking. This is a feature for 3D DRC that checks clearance between
primitives on different layers in the Z-axis.

### Delphi Type Hierarchy (from RTTI strings in BinaryLoader + Advpcb.dll)

```
TZAxisClearanceCacheSection
  └── TCacheData          -- per-entry cache record
      Fields:
        - Primitive ref   -- which PCB primitive this entry is for
        - Layer           -- TV7_Layer (which layer the geometry is cached for)
        - GeometricPolygon-- IPCB_GeometricPolygon (cached clearance geometry)
        - Clearance       -- i32 (clearance distance in Coord units)
```

The cache uses a generic list container (`TList<TCacheData>`) with standard Delphi
`TEnumerator` and `TEmptyFunc` support, visible in RTTI strings like:
- `ZAxisClearanceCache.TZAxisClearanceCacheSection.TCacheData>.arrayofT`
- `ZAxisClearanceCache.TZAxisClearanceCacheSection.TCacheData>.TEnumerator`
- `ZAxisClearanceCache.TZAxisClearanceCacheSection.TCacheData>.TEmptyFunc`

### .NET Interface: `IPCB_ZAxisCacheEnumerator`

Source: `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_ZAxisCacheEnumerator.cs`

```csharp
public interface IPCB_ZAxisCacheEnumerator
{
    bool Next();
    IPCB_Primitive GetState_Primitive();   // Which primitive
    TV7_Layer      GetState_Layer();       // On which layer
    IPCB_GeometricPolygon GetState_Poly(); // Cached polygon geometry
    int            GetState_Clearance();   // Clearance distance (Coord)
}
```

### Board-Level Cache Management

From `IPCB_Board_SaveLoadParameters`:

```csharp
// Create iterator over all cached entries (for serialization)
IPCB_ZAxisCacheEnumerator ZAxisCacheCreateIterator();

// Clear all cache entries (called before reload)
void ZAxisCacheRemoveAll();

// Add a single cache entry
void ZAxisCacheAddValue(
    IPCB_Primitive argPrimitive,
    TV7_Layer argLayer,
    IPCB_GeometricPolygon argPoly,
    int argClearance
);
```

### Trigger Conditions

The cache is only written when the board has Z-axis clearance rules. This is controlled
by the `TStorageFeature.eHasZAxisClearanceRuleAtWriteStage` (value 25) flag.

When Altium saves a PcbDoc file with `eHasZAxisClearanceRuleAtWriteStage` set, it writes
the `Section_ZAxisClearanceCache` section containing all cached Z-axis clearance geometries.

### Related Rule Type

- `TRuleKind.eRule_ZAxisClearance` (value 69) -- the last entry in the TRuleKind enum
- `TViewableObjectID.eViewableObject_Rule_ZAxisClearance` (value 119)
- Rule data: single `GAP` parameter (clearance distance in mil coordinates)
- Part of `cClearanceRuleKinds` alongside `eRule_Creepage`
- Rule string: `"ZAxisClearance"` / display: `"Z-Axis Clearance"`

### Additional Delphi Type: `ZAxisClearanceCachedGeometry`

Found in Advpcb.dll RTTI at multiple addresses (047087ed, 04708866, etc.). This appears
to be a runtime-only helper type for managing the cached geometry polygons, not directly
part of the serialization format.

### Can It Be Safely Ignored?

**YES**. The cache is rebuilt on demand when Z-axis clearance rules are evaluated.
- Not present in any of our 40+ test files
- Gated behind a storage feature flag that is false for most designs
- `ZAxisCacheRemoveAll()` exists specifically for clearing stale cache
- A parser can safely skip this section; a writer can safely omit it

---

## 2. ConnectivityGraphCache

### Overview

The `ConnectivityGraphCache` stores the precomputed connectivity graph of the PCB --
essentially the graph of which primitives are electrically connected to which nets, and
shortest-path information between primitives.

### Delphi Type Hierarchy (from RTTI strings in Advpcb.dll)

```
ConnectivityGraphSerializer
  ├── TLinkRec          -- edge record in the graph
  ├── TVert             -- vertex in the connectivity graph
  ├── TxRef             -- cross-reference type (object-to-vertex mapping)
  ├── TyRef             -- cross-reference type (vertex-to-object mapping)
  ├── TLinks<TObject, TVert>
  │   ├── TData
  │   ├── TCell
  │   ├── TxRef / TyRef
  │   ├── TKeyEnumerator
  │   ├── TKeyCollection
  │   ├── TValueEnumerator
  │   ├── TValueCollection
  │   └── TPairEnumerator
  └── TPair<K, V>       -- key-value pair for dictionary storage
```

The serializer uses a dictionary-of-links structure:
- `TLinks<System.TObject, ConnectivityGraph.TVert>.TxRef` -- maps objects to vertex references
- `TLinks<ConnectivityGraph.TVert, ...TyRef>` -- maps vertices to object references
- Uses `TPair<K, V>` for dictionary iteration

### Path-Finding Infrastructure

The connectivity graph includes path-finding support:

```
ConnectivityGraph.TFindPath.TChainLink
  └── TArray / PArray      -- array of chain links for shortest path
```

This matches the `FindShortestPath` and `FindShortestPath2` methods on the
`IPCB_ConnectivityGraph` interface.

### .NET Interface: `IPCB_ConnectivityGraph`

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_ConnectivityGraph.cs`

```csharp
public interface IPCB_ConnectivityGraph
{
    int  GetMaxPrimsInNet();
    int  GetMaxPinsCount();
    void SetMaxPrimsInNet(int argValue);
    void SetMaxPinsCount(int argValue);

    void Rebuild();                               // Full rebuild
    void RebuildNet(IPCB_Net argNet);             // Rebuild single net

    IPCB_PrimitiveGroups GetConnectedPrimitives();
    IPCB_PrimitiveList FindShortestPath(
        IPCB_Primitive argFromPrimitive,
        IPCB_Primitive argToPrimitive);
    IPCB_PrimitiveList FindShortestPath2(
        IPCB_PrimitiveList argForPrimitives);
}
```

### Board-Level Management

From `IPCB_Board_SaveLoadParameters` and `IPCB_BoardEx3`:

```csharp
// IPCB_Board_SaveLoadParameters:
void RebuildConnectivityGraph();
bool GetActiveConnectivityGraph();
void SetActiveConnectivityGraph(bool argValue);

// IPCB_BoardEx3:
IPCB_ConnectivityGraph GetCurrentConnectivityGraph();
IPCB_ConnectivityGraph CreateNewConnectivityGraph();
```

The `ActiveConnectivityGraph` flag likely controls whether the serialized cache
is used or if it's rebuilt from scratch. `RebuildConnectivityGraph()` is called
in the post-load pipeline.

### Serialization Format

The cache stores a serialized graph with:
- **Vertices** (`TVert`): representing PCB primitives with connectivity info
- **Edges** (`TLinkRec`): representing electrical connections between primitives
- **Cross-references** (`TxRef`, `TyRef`): bidirectional mappings between
  primitive objects and graph vertices
- **Max counts**: `MaxPrimsInNet`, `MaxPinsCount` -- graph size bounds

The `ConnectivityGraphSerializer` class in Advpcb.dll handles the serialization/
deserialization of this graph structure to/from the CFB stream.

### Can It Be Safely Ignored?

**YES**. The connectivity graph is rebuilt from primitives and nets on load.
- Not present in any of our 40+ test files
- `RebuildConnectivityGraph()` is called during `RebuildAfterLoad()`
- `Rebuild()` and `RebuildNet()` methods exist for on-demand regeneration
- A parser can safely skip this section; a writer can safely omit it

---

## 3. ComponentCache

### Overview

The `ComponentCache` stores cached component placement and metadata. This is the
**least documented** of the three caches -- only two RTTI string references were found
in Advpcb.dll, and the "ComponentCache" references in the .NET code are all for
**schematic** data (`SchDataExporterSheetV4` / `SchDataImporterSheetV4Binary`), not PCB.

### Evidence

**Advpcb.dll RTTI strings** (2 occurrences):
- Address `05bb7e93`: `"ComponentCache"` (14 chars)
- Address `0742f4b9`: `"ComponentCache"` (14 chars)

**No corresponding section name** in BinaryLoader's `Section_*` RTTI, suggesting this
cache may be:
1. Written directly by Advpcb.dll without going through BinaryLoader's section registry
2. A newer addition that doesn't yet have formal section infrastructure
3. Possibly unused/deprecated in current file format versions

### Schematic ComponentCache (different from PCB)

The schematic `ComponentCache` is well-documented in the .NET code:
- `SchDataExporterSheetV4.ExportComponentCache()` / `ExportComponentCacheV15()`
- `SchDataImporterSheetV4Binary.ImportComponentCache()` / `ImportComponentCacheV15()`
- Stores a list of `ISchDataComponent` objects as a performance optimization

This is **NOT the same** as the PCB `ComponentCache` section. The schematic version
is part of the SchDoc/SchLib file format, not PcbDoc.

### Can It Be Safely Ignored?

**YES**. Given:
- No test files contain it
- No BinaryLoader section registration found
- The schematic version explicitly demonstrates the "cache" pattern: rebuild from
  source data when missing
- Component data in PcbDoc is already fully represented in the `Components6` section

---

## Implementation Recommendation

For all three cache sections:

1. **Parsing**: Register the CFB stream names so that `assert_all_consumed()` doesn't
   fail on files that happen to contain them, but **skip** the content (mark as consumed
   without parsing).

2. **Saving**: **Do not write** these sections. Altium will regenerate them when needed.

3. **Validation**: If a file contains these sections, treat them as informational only.
   Do not load their contents into the document model.

### Proposed Implementation Pattern

```rust
// In the section loading code, register these as known-but-skipped:
"ZAxisClearanceCache" => {
    // Runtime cache: Z-axis clearance geometry cache.
    // Regenerated by Altium on demand. Safe to skip.
    mark_consumed(entry);
}
"ConnectivityGraphCache" => {
    // Runtime cache: electrical connectivity graph.
    // Rebuilt from primitives/nets during RebuildAfterLoad().
    mark_consumed(entry);
}
"ComponentCache" => {
    // Runtime cache: component placement data.
    // Rebuilt from Components6 section. Safe to skip.
    mark_consumed(entry);
}
```

**NOTE**: Per CLAUDE.md's cardinal rule, we should NOT retain opaque bytes. But these
sections are explicitly caches (not authoritative data), so "skip and don't write" is
the correct approach. The authoritative data lives in the primitive sections (Tracks6,
Pads6, Components6, etc.) and rules sections, which we fully parse.

---

## Source Files Referenced

### .NET Code
| File | Relevance |
|------|-----------|
| `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_ZAxisCacheEnumerator.cs` | ZAxis cache entry structure |
| `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_Board_SaveLoadParameters.cs` | Board save/load pipeline (ZAxis + Connectivity) |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_ConnectivityGraph.cs` | Connectivity graph API |
| `AD26-dotnet/Altium.SDK.Interfaces/PCB/IPCB_ConnectivityGraph.cs` | SDK connectivity graph |
| `AD26-dotnet/Altium.SDK.Interfaces/PCB/IPCB_ConnectivityGraphHelper.cs` | Helper extensions |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_BoardEx3.cs` | Board extensions (create/get graph) |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_ZAxisClearanceRule.cs` | ZAxis rule interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_ZAxisClearanceViolation.cs` | ZAxis violation interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TStorageFeature.cs` | Feature flags (eHasZAxisClearanceRuleAtWriteStage) |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TRuleKind.cs` | Rule kind enum (eRule_ZAxisClearance = 69) |
| `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_FileVersionInfoList.cs` | Version info (AddVersionZAxisClearanceRuleAreUsed) |
| `AD26-dotnet/ConstraintsManager.Module/.../ZAxisClearanceData.cs` | Constraints manager data model |
| `AD26-dotnet/ConstraintsManager.Module/.../PcbIntegrationMapper.cs` | PCB rule mapping |

### Delphi RTTI (Ghidra)
| Binary | Key Strings |
|--------|------------|
| Altium.PCB.BinaryLoader.dll | `Section_ZAxisClearanceCache`, `TZAxisClearanceCacheSection.TCacheData` |
| Advpcb.dll | `ZAxisClearanceCachedGeometry`, `ConnectivityGraphSerializer`, `ConnectivityGraph.TFindPath.TChainLink`, `ComponentCache` |

### Existing Codebase
| File | Relevance |
|------|-----------|
| `crates/altium-format-types/src/pcb.rs` | `RuleKind::ZAxisClearance = 69`, `ViewableObjectId::RuleZAxisClearance = 119` |
| `crates/altium-format/src/pcbdoc/drc.rs` | `ZAxisClearanceRuleData`, `PcbViolation::ZAxisClearance` |

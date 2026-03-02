# Cache Sections Part 2: GeometryZeroCache, PadViaCacheLibraryLinksSection, SimbeorCacheSection

## Overview

These three CFB sections are secondary cache/link sections in PcbDoc files. They
appear in the master section table at indices 54, 51, and 86 respectively, and are
part of Altium's runtime optimization infrastructure. All three are **Delphi-side
only** — their section names do not appear anywhere in the decompiled .NET code
(AD26-dotnet/), which means they are managed entirely by the Delphi PCB engine
(`BinaryLoader.dll`) and are not exposed through the COM/.NET interfaces.

---

## 1. GeometryZeroCache (Section Index 54)

### CFB Stream Name

`GeometryZeroCache`

### Delphi Internal Name

**Not found.** No `Section_*` constant exists in either the .NET code or the
documented Delphi section table. The stream table maps it as:
`GeometryZeroCache | — | Geometry cache`

### What It Is

The name "GeometryZero" likely refers to geometry at the **zero state** or **origin
state** — i.e., cached geometric representations of primitives at their base
position/rotation before placement transforms are applied. This is consistent with
Altium's rendering pipeline, which computes geometry once in a canonical form and
then transforms it per-instance.

The term "zero" in Altium's codebase typically refers to the identity or baseline
state:
- Board origin coordinates
- Component at zero rotation
- Untransformed pad/via shapes

### Evidence from .NET Code

No references to "GeometryZero" or "GeometryZeroCache" exist in AD26-dotnet/.
This section is entirely Delphi-managed.

### Data Structure

Unknown. No test files with this section have been found in `data/pcbdoc/`.
Based on its position in the section table (index 54, between `ComponentCache`
at 53 and `PrimitiveParameters` at 55), it is part of the cache section group
that also includes:
- `ConnectivityGraphCache` (52)
- `ComponentCache` (53)

### Safe to Ignore During Parsing?

**Yes, with caveats.** Like other cache sections, this is regenerable data — Altium
rebuilds it from the authoritative primitive data. It can be safely skipped during
read, and omitted during write (Altium will regenerate it on next open). However,
per the project's fail-fast philosophy, we should still detect and error on it
until we understand its exact format, rather than silently consuming it.

### Implementation Recommendation

Register `GeometryZeroCache` as a recognized-but-unimplemented section that
produces a clear error with context. No data from this section is needed for
any known feature.

---

## 2. PadViaCacheLibraryLinksSection (Section Index 51)

### CFB Stream Name

`PadViaCacheLibraryLinksSection`

### Delphi Internal Name

**Not found** — no `Section_*` constant in .NET or documented Delphi table.

### Relationship to Other PadVia Sections

This is one of **four** PadVia-related sections in a PcbDoc file:

| Index | Section Name | Category | Purpose |
|-------|-------------|----------|---------|
| 48 | `PadViaLibrary` | Authoritative | Board's local pad/via template library |
| 49 | `PadViaLibraryCache` | Cache | Cached copy of the template library |
| 50 | `PadViaLibraryLinks` | Authoritative | Links between primitives and templates |
| 51 | `PadViaCacheLibraryLinksSection` | Cache | Cached copy of the library links |

The board object exposes two distinct `IPCB_PadViaLibrary` instances:
- `GetState_PadViaLibrary()` — the authoritative local template library (section 48)
- `GetState_PadViaCache()` — a cache of templates (section 49)

The distinction between them in the `PcbHelperService.GetPadViaTemplateById()` method
is instructive:

```csharp
// PcbHelperService.cs:699
public IPCB_PadViaTemplate GetPadViaTemplateById(string id, IPCB_Board board)
{
    // First: check the authoritative library
    IPCB_PadViaLibrary library = board.GetState_PadViaLibrary();
    result = library.FindTemplate(id);
    if (result != null) return result;

    // Fallback: check the cache
    IPCB_PadViaLibrary cache = board.GetState_PadViaCache();
    result = cache.FindTemplate(id);
    if (result != null) return result;

    return null;
}
```

The cache serves as a fallback lookup source. When enumerating libraries:

```csharp
// PcbDataProvider.cs / BoardInformationRepository.cs
private List<IPCB_PadViaLibrary> GetLibraries()
{
    List<IPCB_PadViaLibrary> libs = new();
    libs.Add(Board.GetState_PadViaCache());    // cache first
    libs.Add(Board.GetState_PadViaLibrary());  // authoritative second
    return libs;
}
```

A "freed local template" is identified by checking if the template's LibraryID
matches the cache's LibraryID and the template has zero primitive links:

```csharp
// IsFreedLocalTemplate
if (template.GetState_PrimLinkCount() == 0)
    return Board.GetState_PadViaCache().GetState_LibraryID() == template.GetState_LibraryID();
```

### What PadViaCacheLibraryLinksSection Contains

This section is the **cache counterpart** of `PadViaLibraryLinks` (section 50),
just as `PadViaLibraryCache` (49) is the cache counterpart of `PadViaLibrary` (48).

`PadViaLibraryLinks` stores the association between each pad/via primitive and its
template definition. `PadViaCacheLibraryLinksSection` caches this association data
for faster lookup. The "Cache" in the name indicates it's regenerable from the
authoritative `PadViaLibraryLinks` section.

### Feature Gate

The loading optimization is behind a feature flag:
```csharp
// RT_FeatureNames/Consts.cs
public const string cOptionPCBPerformanceOptimizationLoadingPadViaLinks =
    "PCB.Performance.PadViaTemplate.LoadingOptimization";
```

This suggests that `PadViaCacheLibraryLinksSection` exists specifically to speed
up the PadVia template link resolution during board loading.

### Data Structure

The authoritative `PadViaLibraryLinks` section uses standard param record format
(Header/Data streams). `PadViaCacheLibraryLinksSection` likely mirrors this
structure, being a cached copy. The existing parser already handles
`PadViaLibraryLinks` as a `ParamSectionKind`.

### IPCB_PadViaTemplate Interface

Each template stores:
- `LibraryID` — identifies which library it belongs to
- `TemplateID` — unique template identifier within the library
- `RevisionID` — revision tracking
- `TemplateName` / `TemplateDescription` — display metadata
- `ObjectID` — `TObjectId` (pad or via)
- `Internal` flag — local vs external
- `RemoveUnused` flag
- `PrimLinkCount` — how many primitives reference this template
- `IsBackdrill` flag
- `Hash` / `HashVersion` — content hash for change detection
- `HoleNegativeTolerance` / `HolePositiveTolerance`
- `DynamicName` flag

The `IPCB_PadViaTemplateLink` stores the external link info:
- `LibraryID` — external library identifier
- `TemplateID` — external template identifier
- `RevisionID` — external revision

### Safe to Ignore During Parsing?

**Yes.** This is a performance cache that Altium regenerates from `PadViaLibraryLinks`.
For read-only parsing, the authoritative `PadViaLibraryLinks` section provides all
needed data. For save, omitting this section will cause Altium to regenerate it.

### Implementation Recommendation

Register as recognized-but-unimplemented. The authoritative data is in
`PadViaLibraryLinks` (already parsed as `ParamSectionKind::PadViaLibraryLinks`).
If roundtrip preservation of this cache is ever needed, it likely uses the same
`Header/Data` format as `PadViaLibraryLinks`.

---

## 3. SimbeorCacheSection (Section Index 86)

### CFB Stream Name

`SimbeorCacheSection`

### Delphi Internal Name

`Section_SimberianCache` (from the Delphi section table at `0x01bb4a80`)

Note the name difference: The CFB stream name uses "Simbeor" while the Delphi
internal constant uses "Simberian" — referring to Simberian Inc., the company
that develops Simbeor, a signal integrity simulation tool.

### What Is Simbeor?

Simbeor is a **signal integrity (SI) simulation engine** integrated into Altium
Designer. It provides:
- Impedance calculation at PCB cross-sections
- Signal delay computation per primitive (tracks, vias)
- Signal propagation delay per meter
- Resistance and current calculations
- Full SI project export for external analysis

Altium's integration is through the `IPCB_ElectricalCalculation` interface:

```csharp
// IPCB_ElectricalCalculation.cs
public interface IPCB_ElectricalCalculation
{
    double GetPrimitiveResistance(IPCB_Primitive argPrimitive);
    double GetPrimitiveCurrent(IPCB_Primitive argPrimitive);
    bool CanCalculateSignalDelayForPrimitive(IPCB_Primitive argPrimitive);
    double GetSignalDelayForPrim(IPCB_Primitive argPrimitive);
    double GetSignalDelayPerMeter(IPCB_Primitive argPrimitive);
    void SaveSimbeorProjectForPrimitive(IPCB_Primitive argPrimitive, string argSaveFolder);
    double CalculateImpedanceAtPoint(TV7_Layer layer, TCoordPoint pt, int traceWidth, int dpGap, int radius);
    double CalculateImpedanceAtCrossSection(TV7_Layer layer, TCoordPoint pt, TCoordPoint dir, int traceWidth, int dpGap);
    void ResetDelaysCalculator();
    void SignalDelayExportToParamters(IWideParameterList parameters);
    void SignalDelayImportFromParameters(IWideParameterList parameters);
    void SignalDelayUpdateAfterLoad();
}
```

### Feature Version Flag

The Simbeor engine version is tracked as a feature option:
```csharp
// RT_FeatureNames/Consts.cs
public const string cOptionSimbeorVersion = "PCB.SimbeorVersion";
```

There's also a deprecated command:
```csharp
// IPCBCommands.cs
void DoNotUse_ExportPrimitiveToSimbeor_347();
```

### What the Cache Contains

The `SimbeorCacheSection` caches the results of Simbeor's signal integrity
calculations. Based on the interface, the cached data likely includes:
- Per-primitive signal delay values
- Per-primitive resistance values
- Impedance profile data at specific board cross-sections
- The computational model parameters (layer stackup, dielectric properties)

The `SignalDelayExportToParamters` / `SignalDelayImportFromParameters` methods
suggest the cache is serialized as `|KEY=VALUE|` parameter data (standard Altium
param format), and `SignalDelayUpdateAfterLoad()` indicates it's refreshed when
loading a board.

### Test File

One test file has been found containing this section:
`data/pcbdoc/jsmith-exe__self_balancing_robot__esp32_pcb.PcbDoc`

### Data Structure

Likely standard `Header/Data` format with parameter records, based on the
`ExportToParamters`/`ImportFromParameters` pattern. The Header would contain
a record count, and Data would contain `|KEY=VALUE|` blocks with cached
SI calculation results.

### Safe to Ignore During Parsing?

**Yes.** This is purely a computation cache. The SI values are derived from:
- The layer stackup (dielectric properties, copper thickness)
- Primitive geometry (track width, via hole size, etc.)
- Material properties configured in the layer stack manager

All of these are available from other sections. Omitting the cache during save
will cause Altium to recompute SI values when needed (which is the correct
behavior after any geometry changes anyway).

### Implementation Recommendation

Register as recognized-but-unimplemented. If we encounter this section in test
files, a clear error message should indicate it's a known cache section. For
roundtrip write, it can be safely omitted.

---

## Summary: Safe Ignorability

| Section | Index | Regenerable? | Safe to Skip on Read? | Safe to Omit on Write? |
|---------|-------|--------------|-----------------------|------------------------|
| `GeometryZeroCache` | 54 | Yes | Yes | Yes |
| `PadViaCacheLibraryLinksSection` | 51 | Yes (from PadViaLibraryLinks) | Yes | Yes |
| `SimbeorCacheSection` | 86 | Yes (recomputed from geometry + stackup) | Yes | Yes |

All three sections are **caches of derived data**. They exist purely for runtime
performance optimization. The authoritative data lives in other sections:
- GeometryZeroCache: primitives themselves (Pads6, Vias6, Tracks6, etc.)
- PadViaCacheLibraryLinksSection: PadViaLibraryLinks (section 50)
- SimbeorCacheSection: layer stackup + primitive geometry

For our parser, the recommended approach is:
1. **Recognize** these section names in the storage name dispatcher
2. **Consume** the streams (mark as read in TrackedCfbDocument)
3. **Skip** parsing their contents (they're regenerable)
4. **Omit** them on save (Altium regenerates them)

This is consistent with how we handle `PadViaLibraryCache` (section 49) — it's
the cache counterpart of `PadViaLibrary` and uses the same parser but could
equally be skipped.

---

## Related Sections (Cross-Reference)

Other cache sections documented separately (see CacheSections_Part1.md):
- `ZAxisClearanceCache` (87) — `Section_ZAxisClearanceCache`
- `ConnectivityGraphCache` (52) — connectivity graph cache
- `ComponentCache` (53) — component placement cache
- `PadViaLibraryCache` (49) — already parsed, same format as PadViaLibrary

## Source Files Referenced

### .NET Interfaces (AD26-dotnet/)
- `Altium.Edp.Interfaces/RT_PCB/IPCB_PadViaLibrary.cs` — template library interface
- `Altium.Edp.Interfaces/RT_PCB/IPCB_PadViaTemplate.cs` — individual template
- `Altium.Edp.Interfaces/RT_PCB/IPCB_PadViaTemplateLink.cs` — external link
- `Altium.Edp.Interfaces/RT_PCB/IPCB_PadViaLibraryManager.cs` — library manager
- `Altium.Edp.Interfaces/RT_PCB/IPCB_Board.cs` — `GetState_PadViaCache/Library()`
- `Altium.Edp.Interfaces/RT_PCB/IPCB_ElectricalCalculation.cs` — Simbeor SI engine
- `Altium.Edp.Classes/Altium.Edp.Classes.DataHelpers/PcbHelperService.cs` — template lookup
- `Altium.Dxp.Interfaces/RT_FeatureNames/Consts.cs` — feature flags
- `Altium.ConstraintsManager.Module/...PcbDataProvider.cs` — cache/library usage
- `InteractiveProperties.Providers.PCB.DataModel/PcbViaTemplateBasedDataObject.cs`

### Rust Crate Code
- `crates/altium-format/src/pcbdoc/records.rs` — `ParamSectionKind` enum
- `crates/altium-format/src/pcbdoc/mod.rs` — PadViaLibrary/PadViaLibraryCache parsing
- `crates/altium-format/src/pcblib/library.rs` — `PcbPadViaLibraryConfig`, template parsing

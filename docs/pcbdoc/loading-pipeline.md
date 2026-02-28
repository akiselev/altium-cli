# Loading Pipeline

Complete load and save pipeline for PcbDoc files in exact execution order.
Reconstructed from .NET interface analysis (`AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/`
and `PCBInterfaces/`) and the existing `docs/dxp/pcb-files.md` documentation.

## Loading pipeline

The PcbDoc loading pipeline is significantly more complex than PcbLib because it handles
board-level data (nets, components, polygons, rules, DRC violations), builds an ownership
graph via cross-reference indices, and performs post-load reconstruction of connectivity
and board outline.

### Phase 1: Open CFB and identify format

1. Open the file as an OLE/CFB container using the Structured Storage API.
2. Read the root `/FileHeader` stream:
   - Format: `u32 char_count` + UTF-16LE string (`char_count * 2` bytes).
   - The decoded string must equal `"PCB 5.0 Binary File"` (V5/V6 board format).
   - This is the legacy header format -- note that the `u32` stores the **character count**,
     not the byte count.
3. `RecognizeFile()` validates the format and optionally returns the version number.
   - Returns `eAdvPCBFormat_Binary_V6` (10) for modern PcbDoc files.
4. Optionally read `/FileHeaderSix` stream (if present):
   - Format: pascal-block format (same as PcbLib FileHeader).
   - Block 1: `u32 outer_length` + `u8 string_length` + version string + `f64 version`.
   - Block 2: `u32 outer_length` + `u8 string_length` + unique ID (GUID string).
   - Contains the extended version number (e.g. `5.01`) and document GUID.
5. Read storage feature flags via `GetState_Feature()`:
   - `TStorageFeature` flags indicate which capabilities are present in the file.
   - Key flags that affect loading: `eHasShapeBasedRegions` (5), `eHasShapeBasedCompBodies` (6),
     `eHasExtendedGroupIndicesAreUsed` (23).

Source: `IPCB_StructuredStorage.RecognizeFile()`, `pcb_file_header.rs`

### Phase 2: Discover and register sections

6. Enumerate all top-level CFB storages.
7. For each storage, create an `IPCB_BinarySection` (or specialized subclass) via
   `CreateSection(name)`:
   - The section's type is determined by its storage name (e.g. `"Arcs6"`, `"Board6"`).
   - Each section interface specialization corresponds to a data format category:
     - `IPCB_BoardBinarySection` -- Board6 section (board-level settings)
     - `IPCB_RequiredBinarySection` -- Standard primitive sections (Arcs6, Pads6, etc.)
     - `IPCB_PolygonsBinarySection` -- Polygons6 section (polygon pours)
     - `IPCB_DimensionsSection` -- Dimensions6 (prefixed parameter format)
     - `IPCB_ModelsSection` -- Models section (3D model storage)
     - `IPCB_ModelsNoEmbedSection` -- ModelsNoEmbed (non-embedded model references)
     - `IPCB_TextureSection` -- Textures
     - `IPCB_LayerKindMappingSection` -- LayerKindMapping
     - `IPCB_ViolationSection` -- DRC violations
     - `IPCB_BoardRegionsSection` -- Board regions
     - `IPCB_WirebondTemplateSection` -- Wirebond templates
8. `IsSectionToIgnore(name)` checks whether a section should be skipped during loading.
9. Call `RegisterWithBoard()` on the storage to associate all sections with the board object.

Source: `IPCB_StructuredStorage.CreateSection()`, `IPCB_StructuredStorage.RegisterWithBoard()`

### Phase 3: Import sections from file

10. For each registered section, call `Import_FromFile(options)`:
    - The `TStructuredStorageFileSectionImportOptions` parameter is a bitfield; the only
      known option is `ioSkipModels` (bit 0), which skips loading 3D model binary blobs.
    - Each section reads its `Header` and `Data` streams from its CFB storage.

11. **Header stream format** (all sections):
    - Always exactly 4 bytes: `u32 LE` record count.

12. **Data stream formats** vary by section category:

    **Primitive sections** (Arcs6, Pads6, Vias6, Tracks6, Texts6, Fills6,
    Connections6, Regions6, ComponentBodies6, ShapeBasedRegions6,
    ShapeBasedComponentBodies6, SplitPlaneRegions6, BoardRegions, Texts):
    ```
    [1 byte]  TObjectId type byte
    [4 bytes] u32 LE record length (payload only)
    [N bytes] record payload (binary struct)
    ```

    **Parameter sections** (Board6, Nets6, Components6, Polygons6, Classes6,
    DifferentialPairs6, FromTos6, EmbeddedBoards6, Embeddeds6,
    UniqueIDPrimitiveInformation, ExtendedPrimitiveInformation, PadViaLibrary,
    PadViaLibraryCache, PadViaLibraryLinks, PinPairsSection, SignalClasses,
    SmartUnions, UnionRelations, WaivedViolations, PrimitiveParameters,
    Advanced Placer Options6, Advanced Router Options6,
    Design Rule Checker Options6, Pin Swap Options6):
    ```
    [4 bytes] u32 LE string length (including NUL)
    [N bytes] Win1252 NUL-terminated pipe-delimited string: |KEY1=VALUE1|KEY2=VALUE2|
    ```

    **Prefixed parameter sections** (Rules6, NewRules6, Dimensions6, Coordinates6):
    ```
    [2 bytes] u16 LE prefix word
    [4 bytes] u32 LE string length (including NUL)
    [N bytes] Win1252 NUL-terminated pipe-delimited string
    ```

    **WideStrings6** (flat binary index+UTF-16LE -- different from PcbLib WideStrings!):
    ```
    Per entry:
      [4 bytes] u32 LE: primitive index (sequential 0, 1, 2, ...)
      [4 bytes] u32 LE: byte_length (UTF-16LE byte count, includes NUL terminator)
      [byte_length bytes] UTF-16LE encoded string
    ```
    Note: Some older format versions may use a variant with a `[u16=0]` sentinel
    instead of the u32 index field. See `sidecar-streams.md` for full details.

    **Models section** (special):
    - `Models/Header`: u32 record count
    - `Models/Data`: model metadata parameter blocks
    - `Models/0`, `Models/1`, ...: raw 3D model binary blobs (STEP format)

13. Call `RegisterWithBoard()` on each section after import to add its primitives
    to the board's internal collections.

Source: `IPCB_BinarySection.Import_FromFile()`, `IPCB_BinarySection.RegisterWithBoard()`

### Phase 4: Apply Board6 section (board-level settings)

14. Parse the Board6 parameter blocks to extract board-level settings.
15. Apply board state via `IPCB_Board_SaveLoadParameters`:
    - `SetState_BoardVersion(version)` -- set the board format version (f64 from Board6 params).
    - `SetState_BoardOutline(outline)` -- set the board outline primitive.
    - `SetState_LayersColorsLoaded(true)` -- mark layer colors as loaded.
    - `UpdateLayerStackTables()` -- rebuild layer stack from V9_MASTERSTACK/V9_STACK params.
    - `AssignLayerStackToLayerPairs()` -- assign drill layer pairs.
    - `InitAutoTraceTuningOptions()` -- initialize auto-tune routing options.
16. Check `IPCB_BoardBinarySection.Found_ManualSplitPlanes()`:
    - If true, the file contains manually-drawn split plane regions
      (as opposed to automatic split planes).

Source: `IPCB_Board_SaveLoadParameters`

### Phase 5: Build ownership graph

17. After all sections are imported, build the ownership graph using the
    cross-reference indices stored in each section.

18. Each primitive section tracks ownership via `SetIndexes` / `GetIndexes`:
    ```
    SetIndexes(primitive,
        vNet,        // index into Nets6 (0-based, -1 = no net)
        vPolygon,    // index into Polygons6 (0-based, -1 = no polygon)
        vComponent,  // index into Components6 (0-based, -1 = no component)
        vPadOwner,   // index into parent pad (for shape-based features)
        vCoordinate, // index into Coordinates6 (0-based)
        vDimension   // index into Dimensions6 (0-based)
    );
    ```

19. These 6 indices establish the following relationships:
    - **Net ownership**: tracks, pads, vias, fills, arcs, regions are assigned to nets.
    - **Component ownership**: pads, texts, regions, bodies are grouped under components.
    - **Polygon membership**: regions produced by polygon pouring reference their source polygon.
    - **Pad ownership**: shape-based features (custom pad shapes) reference their parent pad.
    - **Coordinate ownership**: coordinate annotations are grouped.
    - **Dimension ownership**: dimension annotations reference their measurement primitives.

20. Extended group indices (when `eHasExtendedGroupIndicesAreUsed` feature flag is set):
    - Uses `TReferenceToGroup` records (16 bytes each) stored per section.
    - Format: `TPrimitiveKey(ObjectId, IndexForSave)` pair linking primitive to group.
    - `ApplyExtendedIndices()` merges these into the ownership graph.
    - This mechanism supersedes the basic 6-index system for group membership tracking.

Source: `IPCB_BinarySection.SetIndexes()`, `IPCB_BinarySection.ApplyExtendedIndices()`

### Phase 6: Merge sidecar streams

21. **WideStrings6**: Read the binary TLV string table.
    - Each entry is indexed 0-based.
    - Text primitives reference their Unicode string by index.
    - Call `AddWSForLoadList(index, text)` to associate strings with primitives.

22. **UniqueIDPrimitiveInformation**: Read parameter blocks.
    - Each block contains: `PRIMITIVEINDEX`, `PRIMITIVEOBJECTID`, `UNIQUEID`.
    - Merge UNIQUEID into the primitive at the specified (ObjectId, Index) pair.

23. **ExtendedPrimitiveInformation**: Read parameter blocks (same format).
    - Contains extended properties added in later format versions.
    - Merge into primitives by index.

24. **PrimitiveGuids**: Read binary GUID records.
    - Fixed-size 24-byte `TPrimitiveGUID` entries: `i32 ObjectId + i32 IndexForSave + 16-byte GUID`.
    - `ApplyGUIDs()` assigns GUIDs to primitives.

25. **FileVersionInfo**: Read version history entries (informational).

Source: `IPCB_StructuredStorage.AddWSForLoadList()`, `IPCB_BinarySection.ApplyGUIDs()`

### Phase 7: Post-load rebuild

26. `RebuildAfterLoad()` -- triggers internal recalculation of derived state
    (bounding boxes, spatial indices, pad caches).

27. `RebuildConnectivityGraph()` -- rebuild the ratsnest (from-to connections
    between pads/vias based on net assignments).

28. `ValidateBoardOutlineRegions()` -- validate that the board outline forms
    a closed, valid polygon.

29. `AnalyzeAllNets()` -- recompute net connectivity from the loaded primitives.

30. Create default classes and rules if not present in the file:
    - `CreateDefaultNetClasses()`
    - `CreateDefaultComponentClasses()`
    - `CreateDefaultLayerClasses()`
    - `CreateDefaultPolygonClasses()`
    - `CreateDefaultDifferentialPairClasses()`
    - `CreateDefaultPadClasses()`
    - `CreateDefaultFromToClasses()`
    - `CreateDefaultPinPairClasses()`
    - `CreateDefaultRules()`
    - `CreateLayerSetsDefaults()`
    - `CreateDefaultxNet()`
    - `CreateDefaultxNetClasses()`
    - `CreateDefaultMechPairIfNotExist()`

31. `InitializeScopeTester()` -- initialize the DRC scope expression engine.

32. `PostLoadOrSave(false)` -- final post-load housekeeping (the `false` argument
    indicates this is a load, not a save).

33. `SetState_DocumentHasNotChanged()` -- clear the dirty flag.

34. `ForceUpdate_ViaStartStopLayersFromStack()` -- ensure via layer assignments
    match the current layer stack.

Source: `IPCB_Board_SaveLoadParameters`

## Saving pipeline

The save pipeline reverses the load process, serializing the in-memory board state
back to a CFB container.

### Phase 1: Pre-save preparation

1. `PrepareToSave()` on each section -- compute derived fields, assign sequential
   `IndexForSave` values to primitives.

2. Assign `IndexForSave` using `IPCB_IndexForSaveIndexer`:
   - `Clear()` resets all counters.
   - For each primitive, `SetState_IndexForSave(index)` assigns a sequential index
     within its section.
   - `GetIndex(objectId)` returns the current counter for a given object type.
   - `SetIndex(objectId, value)` sets the counter.

3. `CollectExtraPrimitives()` -- gather any extra primitives that need to be
   written (e.g. shape-based regions that are stored in separate sections).

4. `IndexExtraPrimitives(indexer)` -- assign indices to extra primitives.

5. Collect text primitives for WideStrings:
   - `AddTextsForSaveList(primitive)` for each text primitive that needs a
     WideStrings entry.
   - `TextsForSaveListCount()` returns the total.

### Phase 2: Compute ownership indices

6. For each primitive in each section, compute the 6 cross-reference indices:
   - Look up the primitive's net, polygon, component, pad owner, coordinate,
     and dimension in the board's collections.
   - Store via `SetIndexes(primitive, vNet, vPolygon, vComponent, vPadOwner, vCoordinate, vDimension)`.

7. If extended group indices are enabled (`eHasExtendedGroupIndicesAreUsed`):
   - Build `TReferenceToGroup` entries for each group membership.
   - Store via `AddExtendedIndex(ref)` on the appropriate section.

### Phase 3: Write CFB container

8. Create or open the CFB file.

9. Write `/FileHeader` stream:
   - `u32 char_count` + UTF-16LE encoded `"PCB 5.0 Binary File"`.

10. Write `/FileHeaderSix` stream (if V6 format):
    - Pascal-block format: version string + f64 version + unique ID.

11. Set storage feature flags via `SetState_Feature()`.

### Phase 4: Export sections

12. For each section, call `Export_ToFile()`:
    - Write `Header` stream: `u32 LE` record count.
    - Write `Data` stream in the section-appropriate format.
    - Primitive sections: type byte + u32 length + binary payload per record.
    - Parameter sections: u32 length + NUL-terminated parameter string per record.
    - Prefixed parameter sections: u16 prefix + u32 length + parameter string per record.

13. Write sidecar streams:
    - **WideStrings6**: Build binary TLV string table from collected text primitives.
    - **UniqueIDPrimitiveInformation**: Write parameter blocks with PRIMITIVEINDEX,
      PRIMITIVEOBJECTID, UNIQUEID for each primitive that has a unique ID.
    - **ExtendedPrimitiveInformation**: Write parameter blocks for extended properties.
    - **PrimitiveGuids**: Write 24-byte binary GUID records.

14. Write Models section:
    - `Models/Header`: record count.
    - `Models/Data`: model metadata.
    - `Models/0`, `Models/1`, ...: embedded model binary blobs.

### Phase 5: Post-save cleanup

15. `PostLoadOrSave(true)` -- post-save housekeeping (the `true` argument indicates save).
16. `SetState_DocumentHasNotChanged()` -- clear the dirty flag.
17. Close the CFB container and flush to disk.

## Section import order

The section import order during loading is determined by CFB storage enumeration order,
which corresponds to the order entries appear in the CFB directory. Each section is
imported independently via `Import_FromFile()`, then registered with the board via
`RegisterWithBoard()`.

The ownership graph (Phase 5) is only built **after** all sections have been imported.
This means sections do not need to be loaded in a specific dependency order -- the
cross-reference indices are resolved as a batch operation after all data is in memory.

However, certain sections must be processed before post-load operations:
- **Board6** must be loaded before `UpdateLayerStackTables()` can run.
- **Nets6** must be loaded before `RebuildConnectivityGraph()` can build the ratsnest.
- **Models** must be loaded before `ComponentBodies6` can resolve model references.

## Ownership graph construction

PCB ownership is fundamentally different from schematic ownership:

| Aspect | Schematic (SchDoc) | PCB (PcbDoc) |
|--------|-------------------|--------------|
| Ownership model | OWNERINDEX (flat list + parent index) | 6 cross-reference indices per primitive |
| Owner types | Any record can own children | Fixed: net, polygon, component, pad, coordinate, dimension |
| Resolution | Index into single flat list | Index into specific typed section |
| Timing | During import (per-record) | Batch after all sections imported |
| Extended | OWNERINDEXADDITIONALLIST flag | TReferenceToGroup extended indices |

The 6-index ownership model means a single primitive can simultaneously belong to a net,
a component, and a polygon. For example, a pad primitive has:
- `vNet` pointing to its net in Nets6
- `vComponent` pointing to its parent component in Components6
- `vPolygon = -1` (pads are not polygon regions)
- `vPadOwner = -1` (pads are not owned by other pads)
- `vCoordinate = -1` (pads are not coordinate annotations)
- `vDimension = -1` (pads are not dimension annotations)

## Differences from PcbLib pipeline

| Phase | PcbDoc | PcbLib |
|-------|--------|--------|
| FileHeader format | UTF-16LE legacy format + FileHeaderSix | Pascal-block format (single stream) |
| FileHeader content | `"PCB 5.0 Binary File"` | `"PCB 6.0 Binary Library File"` |
| Section discovery | Top-level CFB storages (flat) | Library/ storage + per-footprint storages |
| Section types | 40+ board-level sections | 5-7 per footprint + library-global |
| Ownership | 6-index cross-references (net/polygon/component/pad/coord/dim) | No ownership indices (all primitives belong to the footprint) |
| Nets | Nets6 section with net definitions | No nets (nets are board-level) |
| Components | Components6 section | No component instances (footprint is the component) |
| Rules | Rules6 + NewRules6 sections | No rules |
| DRC | Design Rule Checker Options6 + WaivedViolations | No DRC |
| WideStrings format | Binary TLV (WideStrings6) | Parameter-block (per-footprint WideStrings) |
| SectionKeys | Not used | Maps display names to obfuscated storage keys |
| Post-load rebuild | Full rebuild (connectivity, outline, nets, scope tester) | Minimal (layer sync, owner board update) |
| Board version | f64 from Board6 parameters | f64 from FileHeader |
| 3D Models | Board-level Models/ storage | Library-level Library/Models/ storage |
| Split planes | SplitPlaneRegions6 + automatic/manual detection | Not applicable |

## Error handling

Following the fail-fast philosophy:

- Unknown section names: error (not silently skipped)
- Unknown parameter keys within known sections: error (not silently dropped)
- Record type mismatch (e.g. TObjectId=2 in Arcs6): error
- Truncated binary records: error (record length exceeds available data)
- Invalid cross-reference indices: error (index out of range for target section)
- WideStrings decode errors (invalid UTF-16LE or UTF-8): error
- Checksum mismatch on 3D models: flagged via `IPCB_ModelsNoEmbedSection.InvalidChecksumModelsCount()`
- Missing required sections: error (Board6 must exist)
- Malformed Header stream (not exactly 4 bytes): error

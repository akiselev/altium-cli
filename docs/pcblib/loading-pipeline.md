# Loading Pipeline

The PcbLib loading pipeline reads the CFB container and constructs in-memory footprint
objects. This document describes the exact operation order based on the .NET interface
analysis and real file observations.

## Full library load

### Phase 1: Open and identify format

1. Open CFB file using OLE/Structured Storage API.
2. Read `/FileHeader` stream:
   - Parse block length, header text, version float, key token.
   - Validate header text is `"PCB 6.0 Binary Library File"` (V6 format).
   - Extract format version (should be `eAdvPCBFormat_Library_V6` = 11).

### Phase 2: Read SectionKeys

3. Check for `/SectionKeys` stream (optional).
4. If present, parse the `u32 count` followed by name/key pairs:
   - Build `full_name → cfb_key` mapping.
   - Build reverse `cfb_key → full_name` mapping.

### Phase 3: Read library-global data

5. Read `/Library/Data` stream:
   - Parse library header (FILENAME, KIND, VERSION, DATE, TIME).
   - Parse board defaults and layer stack (V9_MASTERSTACK_*, V9_STACK_LAYER* parameters).
   - Parse `RECORD=Board` continuation records.

6. Read `/Library/ComponentParamsTOC/Data`:
   - Parse footprint summary entries (Name, Pad Count, Height, Description).
   - This provides the footprint index for display without loading individual footprints.

7. Read `/Library/Models/{Header,Data}`:
   - Parse model metadata entries (EMBED, ID, ROTX/Y/Z, DZ, CHECKSUM, NAME).
   - Note: actual model blob streams (`Models/0`, `Models/1`, ...) are loaded on demand.

8. Read `/Library/LayerKindMapping/{Header,Data}`:
   - Parse version and layer kind entries.

9. Read `/Library/PadViaLibrary/{Header,Data}`:
   - Parse pad/via template library metadata.

10. Read `/Library/EmbeddedFonts`:
    - Parse embedded font data (may be empty).

11. Read `/Library/ModelsNoEmbed/{Header,Data}`:
    - Parse non-embedded model references.

12. Read `/Library/Textures/{Header,Data}`:
    - Parse texture data (typically empty).

### Phase 4: Enumerate footprint storages

13. List all top-level CFB storages.
14. Exclude system storages: `FileVersionInfo`, `Library`.
15. For each remaining storage:
    - Check that it contains a `Data` sub-stream (confirming it's a footprint).
    - Resolve display name via SectionKeys mapping (or use storage name directly).
    - Add to the footprint list.

### Phase 5: Load each footprint

For each footprint storage, in enumeration order:

#### 5a. Read Parameters stream

16. Open `<FootprintName>/Parameters` stream.
17. Parse as a length-prefixed parameter block.
18. Extract: PATTERN, HEIGHT, DESCRIPTION, ITEMGUID, REVISIONGUID.

#### 5b. Read Header stream

19. Open `<FootprintName>/Header` stream.
20. Read 4 bytes as `u32` LE record count.

#### 5c. Read Data stream (core primitives)

21. Open `<FootprintName>/Data` stream.
22. Parse pattern name block:
    - Read `u32` block length.
    - Read `u8` string length + ASCII pattern name.
    - Verify pattern name matches Parameters PATTERN (warning if mismatch).
23. Parse packed binary primitive records:
    - Read `u8` type byte (TObjectId).
    - Determine subrecord count: 6 for Pad(2), 2 for Text(5), 1 for all others.
    - For each subrecord: read `u32` length + payload bytes.
    - Dispatch to type-specific parser based on TObjectId.
    - Assign sequential 0-based index to each primitive.
24. Continue until all bytes consumed or expected count reached.

#### 5d. Read WideStrings sidecar

25. Open `<FootprintName>/WideStrings` (if present).
26. Parse as length-prefixed parameter block.
27. For each `ENCODEDTEXT{N}` parameter:
    - Decode comma-separated decimal bytes as UTF-8 string.
    - Merge into the Nth Text primitive (replacing Win1252 string from core record).

#### 5e. Read UniqueIDPrimitiveInformation sidecar

28. Open `<FootprintName>/UniqueIDPrimitiveInformation/{Header,Data}` (if present).
29. Read `u32` count from Header.
30. Parse Data as parameter blocks:
    - For each entry: extract PRIMITIVEINDEX, PRIMITIVEOBJECTID, UNIQUEID.
    - Merge UNIQUEID into the primitive at the specified index.

#### 5f. Read ExtendedPrimitiveInformation sidecar

31. Open `<FootprintName>/ExtendedPrimitiveInformation/{Header,Data}` (if present, rare).
32. Parse same format as UniqueIDPrimitiveInformation.
33. Merge extended properties into primitives by index.

#### 5g. Read PrimitiveGuids sidecar

34. Open `<FootprintName>/PrimitiveGuids/{Header,Data}` (if present).
35. Read `u32` count from Header.
36. Parse Data as binary GUID records.
37. Assign GUIDs to primitives.

#### 5h. Load 3D models (on demand)

38. For each ComponentBody primitive:
    - Extract model GUID from the record.
    - Look up model index in Library/Models/Data entries by GUID match.
    - If embedded (`EMBED=TRUE`): read `Library/Models/{index}` stream.
    - Decompress zlib data to get STEP model content.

### Phase 6: Post-load (optional)

39. Read `/FileVersionInfo/{Header,Data}` (if present):
    - Parse version history entries.

40. Validate library consistency:
    - Verify ComponentParamsTOC matches actual footprint data.
    - Report any missing models or parse errors.

## Single footprint load

When loading one footprint by name:

1. Execute Phases 1-3 (FileHeader, SectionKeys, Library-global data).
2. Resolve the footprint name to a CFB storage key.
3. Execute Phase 5 (5a-5h) for that single footprint only.
4. Load referenced 3D models on demand.

## Comparison with SchLib loading pipeline

| Phase | SchLib | PcbLib |
|-------|--------|--------|
| Header | FileHeader: font table + component index | FileHeader: format ID only |
| Name mapping | SectionKeys (same format) | SectionKeys (same format) |
| Library-wide data | N/A (embedded in FileHeader) | Library/ storage (board defaults, models) |
| Component data | Text records (pipe-delimited) + binary pins | All binary records |
| Sidecar: strings | N/A (inline in text records) | WideStrings (parameter-block format) |
| Sidecar: IDs | N/A | UniqueIDPrimitiveInformation |
| Sidecar: extended | N/A | ExtendedPrimitiveInformation |
| Sidecar: GUIDs | N/A | PrimitiveGuids |
| Sidecar: pins | 9 pin sidecar streams | N/A (no pin concept in PCB) |
| Images | /Storage (embedded images) | N/A (no embedded images) |
| 3D models | N/A | Library/Models/ (STEP files) |
| Additional data | /LibAdditional + per-component Additional | N/A |

## Save pipeline (inverse)

The save pipeline reverses the load process:

1. Write FileHeader stream.
2. Write Library/ storage (board defaults, layer stack, models, fonts, etc.).
3. For each footprint:
   a. Write Parameters stream.
   b. Write Header stream (record count).
   c. Write Data stream (pattern name block + packed binary primitives).
   d. Write WideStrings sidecar (extract Unicode text from Text primitives).
   e. Write UniqueIDPrimitiveInformation sidecar.
   f. Write ExtendedPrimitiveInformation sidecar (if any).
   g. Write PrimitiveGuids sidecar (if any).
4. Write SectionKeys stream (if any names were truncated).
5. Write ComponentParamsTOC (footprint summary index).
6. Write FileVersionInfo.

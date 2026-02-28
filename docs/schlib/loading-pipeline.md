> **Authoritative reference**: See [../../dxp/sch-files.md](../../dxp/sch-files.md)
> for the canonical format specification. This document covers SchLib-specific details.

# Loading Pipeline

The SchLib loading pipeline runs in three sequential phases. This document describes the
exact operation order for both full library loads and single-component loads.

See [../../designs/low-level-api.md](../../designs/low-level-api.md) for the parsing layer
architecture that implements this pipeline.

## Full library load

### Phase 1: ImportBaseWarehouse

#### 1a. ReadBaseWarehouse

1. Open `/FileHeader` stream.
2. Read header block (flags=0x00):
   - `RECORD=0`, `HEADER`, `Weight`, `MinorVersion`, `UniqueID`
3. Read font table: `FontIdCount`, then `Size{N}`, `FontName{N}`, `Italic{N}`, `Bold{N}`,
   `Underline{N}`, `StrikeOut{N}`, `Rotation{N}` for N = 1..FontIdCount.
4. Import library-level display parameters (`AreaColor`, `SnapGridSize`, etc.).
5. Close `/FileHeader`.
6. Load `/SectionKeys` stream (if it exists):
   - Parse `KeyCount`, then `LibRef{N}` / `SectionKey{N}` pairs into a name→key map.
7. `FindFirstStream("Data")`: enumerate all CFB sub-storages that contain a `Data` stream.
8. For each component sub-storage:
   a. Record `component_base_offset` = current count of records in the warehouse.
   b. Read Block 0 of the `Data` stream (always `SchComponent`, flags=0x00, RECORD=1).
   c. Reset component `LOCATION` to origin and `ORIENTATION` to 0.
   d. Add the `SchComponent` to the library's component list.
   e. Read subsequent blocks sequentially until RECORD=0 (end marker) or stream end:
      - If `flags=0x01`: parse as binary pin record (see [binary-pin-format.md](binary-pin-format.md)).
      - If `flags=0x00`: parse as parameter text record (dispatch on `RECORD` value).
      - For every record: `absolute_owner_index = relative_owner_index + component_base_offset`
      - Link each record into its parent via `UpdateOwner(baseWarehouse)`.
9. `FindCloseStream()`.

#### 1b. ProcessImportedBaseWarehouse

Post-processing step on the collected warehouse (internal, no user-visible operations).

### Phase 2: ImportExtendedWarehouse

#### 2a. ReadExtendedWarehouse (Storage stream)

1. Open `/Storage` stream.
2. Read header block (flags=0x00): `RECORD=0`, `HEADER="Icon storage"`, `Weight=<count>`.
3. For each of the `Weight` entry blocks (flags=0x01):
   - Read `0xD0` tag.
   - Read name (id_length + id bytes, name is the image filename).
   - Read inner header (4 bytes: inner_flags + inner_length).
   - Read inner data (zlib-compressed image binary).
   - Create a `SchDataEmbeddedObject` with the name and decompressed data.
4. Close `/Storage`.

#### 2b. ProcessImportedExtendedWarehouse

Match each `SchDataEmbeddedObject` to a `SchImage` record (RECORD=30) by comparing the
object's name to `SchImage.FILENAME`.

#### 2c. ReadAndProcessPinsExtendedData

For each component (in load order), attempt to read 9 pin sidecar streams. Each stream
is optional; skip if not present. Apply in this exact order:

1. `PinFrac` - parse and add fractional coordinate adjustments to each pin
2. `PinDesc` - parse and append overflow description text
3. `PinMiscData` - parse and set `PairSwapID`
4. `PinTextData` - parse and set custom text display settings
5. `PinWideText` - parse and replace text fields (name, designator, description)
6. `PinSymbolLineWidth` - parse and set symbol line width
7. `PinPackageLength` - parse and set package length
8. `PinPropagationDelay` - parse and set propagation delay
9. `PinFunctionData` - parse and set pin functions

See [pin-sidecar-streams.md](pin-sidecar-streams.md) for each stream's format.

### Phase 3: ImportAdditionalWarehouse

1. Check if `/LibAdditional` stream exists. If absent, skip this entire phase.
2. Open `/LibAdditional`.
3. Read header block: `RECORD=0`, `HEADER`, `Weight`.
4. Close `/LibAdditional`.
5. For each component:
   a. Get the component's CFB key (via SectionKeys if needed).
   b. Open `/<key>/Additional` stream (skip if it does not exist).
   c. Read records until RECORD=0 or stream end.
   d. Adjust `OWNERINDEX` values to absolute (add component base offset).
   e. Link via `UpdateOwner()`.

### Phase 4: UpdateDocumentAfterImport

1. Fire `SchDataAfterImportDocumentEvent`.
2. Clear internal warehouse references (free temporary loading state).

## Single component load

When loading one component by `libraryReference`:

1. Import library header and SectionKeys (same as full load steps 1.a.1-6 above).
2. Compute the CFB key for `libraryReference` using the SectionKeys map.
3. Resolve aliases: check `/<key>/Redirection` stream. If present, read `SectionName`
   and re-resolve that name to its CFB key.
4. Open `/<key>/Data` stream directly.
5. Read the `SchComponent` block and all child blocks (same as step 1.a.8 above, but for
   this single component only).
6. Execute Phase 2 (ExtendedWarehouse) for this component only:
   - Read `/Storage` stream and match images.
   - Read the 9 pin sidecar streams for this component.
7. Execute Phase 3 (AdditionalWarehouse) for this component only:
   - Read `/<key>/Additional` stream if it exists.

## Save pipeline

```
1. FillBaseAndAdditionalWarehouses()
   - Flatten the component hierarchy into ordered lists for writing.

2. FillExtendedWarehouse()
   - Collect all SchImage embedded objects.

3. FixBaseWarehouse()
   - Fix any data issues (e.g., deduplicate LibRef names).

4. WriteBaseWarehouseHeader()
   - Write /FileHeader stream as a single parameter block (flags=0x00):
     * RECORD=0, HEADER, Weight, MinorVersion, UniqueID
     * Font table (FontIdCount, Size{N}, FontName{N}, etc.)
     * Display parameters
     * Component index (CompCount, LibRef{N}, CompDescr{N}, PartCount{N},
       AliasCount{N}, Comp{N}Alias{M})

5. WriteBaseWarehouseData()
   - For each component:
     a. Write alias Redirection streams for each alias.
     b. Open /<SectionKey>/Data stream.
     c. Write Block 0: SchComponent (flags=0x00, RECORD=1).
     d. Write child records in order:
        - Pins: flags=0x01 (binary format, Export_Instruction="BINARY")
        - All other children: flags=0x00 (parameter format, Export_Instruction="RECORD")
        - OwnerIndex values are adjusted to relative: relative = absolute - component_base_offset
     e. Write end marker (flags=0x00, RECORD=0).
   - Write /SectionKeys stream (if any component names were truncated).

6. WriteExtendedWarehouse()
   - Write /Storage stream (global images with embedded object envelope).
   - PrepareAndWritePinsExtendedData():
     * For each component, write up to 9 pin sidecar streams (only those with data).

7. WriteAdditionalWarehouse()
   - Write /LibAdditional header stream.
   - Write per-component /<SectionKey>/Additional streams (only if data exists).
```

### Critical save detail

Pins use `Export_Instruction(b, "BINARY")` which produces flags=0x01 blocks. All other
primitives use `Export_Instruction(b, "RECORD")` which produces flags=0x00 blocks. This
is why every binary block in a `Data` stream is a pin.

## Low-level API layer mapping

How each pipeline phase maps to the parsing layers defined in
[low-level-api.md](../../designs/low-level-api.md):

| Pipeline step | Layer 1 (CFB) | Layer 2 (Blocks) | Layer 3 (Data access) | Layer 4 (Records) |
|---|---|---|---|---|
| Read FileHeader | `read_stream("/FileHeader")` | `parse_blocks` | `ParameterCollection::from_bytes` | Header-specific `FromParams` |
| Read SectionKeys | `read_stream("/SectionKeys")` | `parse_blocks` | `ParameterCollection::from_bytes` | Indexed family (`remove_indexed`) |
| Read component Data | `read_stream("/<key>/Data")` | `parse_blocks` (mixed text+binary) | `ParameterCollection` or `BinaryReader` | `SchRecord::from_block` (dual dispatch) |
| Read Storage images | `read_stream("/Storage")` | `parse_blocks` | `parse_embedded_object_stream` + `flate2` decompress | N/A (direct struct) |
| Read pin sidecars | `read_stream_optional("/<key>/PinFrac")` etc. | `parse_blocks` | `parse_embedded_object_stream` + `BinaryReader` or `ParameterCollection::from_utf16le_bytes` | N/A (direct merge) |
| Read Additional | `read_stream_optional("/<key>/Additional")` | `parse_blocks` | `ParameterCollection::from_bytes` | `SchRecord::from_block` |

**Key interactions with the low-level API:**

- **Mixed-format dispatch**: Component Data streams contain both text blocks (flags=0x00,
  most record types) and binary blocks (flags=0x01, pins only). `SchRecord::from_block`
  handles both, using `RECORD=N` for text and binary code `0x02` for binary.
- **RECORD=0 sentinel**: `SchRecord::from_block` returns `Ok(None)` for RECORD=0 end
  markers rather than treating them as unknown record types.
- **Embedded object envelope**: Storage and all 9 pin sidecar streams use the `0xD0`
  envelope format parsed by `parse_embedded_object_stream`. Pin sidecar entries are sparse
  (addressed by pin index in the envelope id field, not by array position).
- **UTF-16LE sidecars**: 7 of 9 pin sidecar streams encode their parameter strings in
  UTF-16LE, requiring `ParameterCollection::from_utf16le_bytes` instead of `from_bytes`.
- **Binary pin coordinates**: Binary pins store coordinates as i16 in DXP units (each =
  100,000 internal units). PinFrac provides the remainder. This is distinct from PCB binary
  coordinates which use i32 directly in internal units.

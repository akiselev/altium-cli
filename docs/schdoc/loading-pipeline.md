> **Authoritative reference**: See [../../dxp/sch-files.md](../../dxp/sch-files.md)
> for the canonical format specification. This document covers SchDoc-specific details.

# Loading Pipeline

Complete load and save pipeline for SchDoc files in exact execution order.

## Loading pipeline

The SchDoc loading pipeline is simpler than SchLib because there are no per-component
storages, no binary pin records, and no pin sidecar streams.

### Step 1: Open CFB container

Open the file as an OLE/CFB container. Enumerate streams.

### Step 2: Import BaseWarehouse (FileHeader stream)

Read the `FileHeader` stream sequentially:

1. **Read header block** (block 0):
   - Parse `HEADER` string: must equal
     `Protel for Windows - Schematic Capture Binary File Version 5.0`
   - Parse `Weight`: total number of object records that follow
   - Parse `MinorVersion`: format version (observed: `2` in LimeSDR test files;
     current Altium 26 writes `13` per sch-files.md Appendix C; accept any value)
   - Parse `UniqueID`: document-level unique identifier

2. **Read content records** (blocks 1..Weight):
   - For each block, read the 4-byte header (size + flags)
   - Flags should always be 0x00 (parameter text)
   - Parse the pipe-delimited key=value payload
   - Extract RECORD value to determine type
   - Deserialize into the appropriate record struct
   - Add to the flat BaseWarehouse list

3. **Build object hierarchy**:
   - Record at index 0 is always RECORD=31 (Sheet)
   - Record at index 1 is always RECORD=39 (Template)
   - All other records reference their parent via OWNERINDEX (0-based absolute)
   - Components (RECORD=1) own pins, designators, parameters, etc.
   - Implementation hierarchy: 44 -> 45 -> 46/48

4. **Parse font table** from the Sheet record (RECORD=31):
   - `FontIdCount` gives the number of fonts
   - For each font N (1-based): `SizeN`, `FontNameN`, optional `BoldN`, `ItalicN`, etc.

### Step 3: Import ExtendedWarehouse (Storage stream)

Read the `Storage` stream for embedded binary objects (images):

1. Read header block: `HEADER=Icon storage`, `Weight=N`
2. For each of the N entry blocks (flags=0x01):
   - Verify 0xD0 tag
   - Extract id (original file path)
   - Extract compressed data size from inner header
   - Decompress using zlib
   - Store as embedded object keyed by id

### Step 4: Import AdditionalWarehouse (Additional stream)

Read the `Additional` stream for supplementary records:

1. Read header block: `HEADER=Protel for Windows - ...`, optional `Weight=N`
2. If Weight > 0, read N content records (RECORD=225 dashed rectangles)
3. Add to AdditionalWarehouse list
4. When resolving OWNERINDEX for Additional records, check the
   `OWNERINDEXADDITIONALLIST` field: if T, OWNERINDEX refers to AdditionalWarehouse;
   if F (default), OWNERINDEX refers to BaseWarehouse

### Step 5: Import optional streams

For each optional stream, check if it exists and import if present:

- **ObjectDefinitions**: Object definition records
- **ReuseBlockInfos**: Reuse block metadata
- **ReuseBlocks** / **ReuseBlocksV2**: Reuse block data
- **HarnessConnectionPointConnector**: Harness connector data
- **Files**: Embedded file data

These are typically not present in standard SchDoc files. Skip silently if absent.

### Step 6: Finalize

Post-load processing:

1. Link embedded images to their SchImage records via FILENAME matching
2. Resolve OWNERINDEX references to build parent-child tree
3. Validate record consistency (every OWNERINDEX must reference a valid record)

## Saving pipeline

The export pipeline writes all data back to the CFB container.

### Step 1: Pre-save mutations

Compute fields that are derived during save:

- `Weight` in the header = total number of records
- `INDEXINSHEET` values: sequential numbering per parent
- Bounding box fields (if applicable)

### Step 2: Write FileHeader stream

1. Write header block (block 0):
   - `HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0`
   - `Weight=<total_record_count>`
   - `MinorVersion=<format_version>` (2 for legacy, 13 for current Altium 26)
   - `UniqueID=<document_unique_id>`

2. Write records in depth-first order:
   - Sheet (RECORD=31) first, with font table
   - Template (RECORD=39) second
   - Template-owned primitives
   - Components with their children
   - Standalone sheet-level objects

3. For each record:
   - Serialize to pipe-delimited key=value string
   - Add NUL terminator
   - Write 4-byte block header: size | (0x00 << 24)
   - Write payload

### Step 3: Write Storage stream

1. Write header block: `HEADER=Icon storage`, `Weight=<image_count>`
2. For each embedded image:
   - Compress image data with zlib
   - Build entry: 0xD0 tag + id string + inner header + compressed data
   - Write as flags=0x01 block

### Step 4: Write Additional stream

1. Write header block with appropriate Weight
2. Write RECORD=225 entries (if any)

### Step 5: Write optional streams

Write ObjectDefinitions, ReuseBlockInfos, etc. if they were present in the original file.

### Step 6: Finalize CFB

Close the CFB container and flush to disk.

## Key differences from SchLib pipeline

| Step | SchDoc | SchLib |
|------|--------|--------|
| Open CFB | Flat (3 streams) | Hierarchical (storages) |
| Read main data | Single FileHeader stream | Per-component Data streams |
| Pin deserialization | Text format (RECORD=2) | Binary format (flags=0x01) |
| Pin sidecar merge | Not needed | 9 sidecar streams per component |
| OwnerIndex resolution | Global absolute indices | Relative per-component, then adjusted |
| SectionKeys | Not needed | Required for long names |
| Aliases | Not applicable | Alias redirection |
| Font table location | In RECORD=31 (Sheet) | In FileHeader stream header |
| Additional stream | RECORD=225 entries | Not present |

## Error handling

Following the fail-fast philosophy:

- Unknown RECORD values: error (not silently skipped)
- Unknown parameter keys: error (not silently dropped)
- Invalid OWNERINDEX: error (must reference valid record)
- Missing required fields: error (not defaulted silently)
- Binary blocks (flags=0x01) in FileHeader: error (only text expected)
- Malformed block headers: error (truncated or oversized)

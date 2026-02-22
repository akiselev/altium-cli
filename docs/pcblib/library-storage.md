# Library Storage

The `/Library/` storage contains library-wide global data that applies to all footprints.

## Library/Data — Board defaults and layer stack

The `Library/Data` stream contains parameter blocks describing the library's board context.
This provides default board settings (layer stack, design rules, etc.) that apply when
editing footprints in the library editor.

### Encoding

The stream uses a non-standard framing: it starts with a 4-byte block header, but the
content is a massive pipe-delimited parameter string. The block framing may not parse
cleanly as standard size-prefixed blocks (the `ole-inspect.py` tool reports block parse
errors on this stream, likely because the data is split across continuation blocks that
don't follow the standard block framing).

**Recommended approach**: Read the entire stream content and parse as a single pipe-delimited
parameter string, ignoring block framing.

### Content structure

The first parameter record (no RECORD key) contains the library header:

| Key | Example | Description |
|-----|---------|-------------|
| `FILENAME` | `C:\...\MyLib.PcbLib` | Original file path (informational) |
| `KIND` | `Protel_Advanced_PCB_Library` | Library type identifier |
| `VERSION` | `3.00` | Library format version string |
| `DATE` | `2019-05-28` | Last modified date |
| `TIME` | `16:44:26` | Last modified time |
| `V9_MASTERSTACK_STYLE` | `0` | Master layer stack style |
| `V9_MASTERSTACK_ID` | `{GUID}` | Master stack GUID |
| `V9_MASTERSTACK_NAME` | `Master layer stack` | Stack name |
| `V9_STACK_LAYER{N}_ID` | `{GUID}` | Per-layer GUID |
| `V9_STACK_LAYER{N}_NAME` | `Top Overlay` | Per-layer name |
| `V9_STACK_LAYER{N}_LAYERID` | `16973830` | Per-layer ID (encoded layer constant) |
| `V9_STACK_LAYER{N}_USEDBYPRIMS` | `TRUE`/`FALSE` | Whether primitives use this layer |
| `V9_STACK_LAYER{N}_COPTHICK` | `1.4mil` | Copper thickness |
| `V9_STACK_LAYER{N}_DIELTYPE` | `0` | Dielectric type |
| `V9_STACK_LAYER{N}_DIELCONST` | `4.800` | Dielectric constant |
| `V9_STACK_LAYER{N}_DIELHEIGHT` | `12.6mil` | Dielectric height |
| `V9_STACK_LAYER{N}_DIELMATERIAL` | `FR-4` | Dielectric material |

Subsequent records have `RECORD=Board` and contain additional layer properties in
continuation blocks. A typical library has 25+ Board records spanning all layer definitions.

### Implementation note

For a read-only PcbLib parser focused on extracting footprint data, the Library/Data stream
can initially be **skipped or read opaquely**. The layer stack information is useful for
layer mapping but is not required to parse the primitive data in individual footprints.

For a write path, this stream must be faithfully round-tripped.

## Library/Header

Always exactly 4 bytes: `u32` LE count. Interpretation is unclear — may be the count of
parameter blocks in the Data stream or a version indicator. Observed value in test files: `0x00000001`.

## Library/EmbeddedFonts

Contains embedded font binary data. In test files, this is a single block with 0-length
payload (empty). Present even in blank libraries.

## Library/ComponentParamsTOC/{Header,Data}

Component Parameter Table of Contents. The Data stream contains parameter blocks with
summary information about each footprint:

| Key | Example | Description |
|-----|---------|-------------|
| `Name` | `CAP0402` | Footprint name |
| `Pad Count` | `3` | Number of pads |
| `Height` | `21.6535` | Component height |
| `Description` | `Chip Capacitor...` | Description text |

This is a read-optimization — it allows Altium to display footprint metadata without
loading each footprint's Data stream.

## Library/LayerKindMapping/{Header,Data}

Maps mechanical layer IDs to layer kinds. The Data stream appears to contain a version
string block followed by mapping entries.

Observed content (blank library):
- Block 0: `31 00 2e 00 30 00 00 00` → UTF-16LE `"1.0\0"` (version)
- Block 1: empty (0 bytes)
- Block 2: empty (0 bytes)

## Library/Models/{Header,Data,0,1,...}

3D model storage. This is a shared model pool — multiple footprints can reference the same
model by index.

### Models/Header

4 bytes: `u32` LE count of model entries.

### Models/Data

Contains parameter blocks for each model entry:

| Key | Example | Description |
|-----|---------|-------------|
| `EMBED` | `TRUE` | Whether model data is embedded |
| `MODELSOURCE` | `Undefined` | Model source type |
| `ID` | `{35957C61-1427-4C32-94C7-A459E0017AD7}` | Model GUID |
| `ROTX` | `0.000` | X rotation |
| `ROTY` | `0.000` | Y rotation |
| `ROTZ` | `270.000` | Z rotation |
| `DZ` | `0` | Z offset |
| `CHECKSUM` | `984310846` | Model data checksum |
| `NAME` | `SOP65P640X110-24N.STEP` | Model filename |

### Models/N (individual model streams)

Each numbered stream (`0`, `1`, ...) contains the raw 3D model data, typically
**zlib-compressed STEP format**. The raw bytes start with `78 9c` (zlib magic).

Decompressed content is standard ISO-10303-21 STEP format:
```
ISO-10303-21;
HEADER;
FILE_DESCRIPTION (( 'STEP AP214' ), '1' );
FILE_NAME ('SOP65P640X110-24N.STEP', ...);
...
```

### Model types

```
enum T3DModelType : u8 {
    Extruded = 0,    // Extruded 2D outline
    Generic  = 1,    // Generic STEP/STP file
    Cylinder = 2,    // Parametric cylinder
    Sphere   = 3,    // Parametric sphere
}
```

## Library/ModelsNoEmbed/{Header,Data}

References to models that are NOT embedded in the library file. These reference external
STEP files by path. Typically empty in self-contained libraries.

## Library/PadViaLibrary/{Header,Data}

Pad/Via template library. Contains a single parameter block:

| Key | Example | Description |
|-----|---------|-------------|
| `PADVIALIBRARY.LIBRARYID` | `{GUID}` | Library GUID |
| `PADVIALIBRARY.LIBRARYNAME` | `<Local>` | Library name |
| `PADVIALIBRARY.DISPLAYUNITS` | `1` | Display unit setting |

This is used by Altium's pad/via library feature for sharing pad stack definitions.

## Library/Textures/{Header,Data}

Texture image data for 3D rendering. Typically empty in standard PCB libraries.

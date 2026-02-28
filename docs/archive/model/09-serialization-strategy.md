# Serialization Strategy

Design document for the serialization/deserialization approach in
`altium-format`. Covers both the schematic text format and the PCB binary
format, derive macro design, strict parsing, sidecar streams, string encoding,
and version handling.

**Design philosophy: fail fast, fail hard. No lenient mode.**

Every byte must be accounted for. Every parameter key must map to a typed
field. Every enum value must be a known variant. If we can't fully parse a
record, that is a bug in our code, not a feature of the file.

---

## 1. Two Format Systems

Altium uses two completely different serialization formats, split by domain:

| Aspect | Schematic (SCH) | PCB |
|--------|-----------------|-----|
| Encoding | Text (pipe-delimited key=value) | Binary (packed little-endian) |
| Type discriminant | `RECORD=N` parameter | `u8 object_id` first byte |
| Field identification | Parameter key names | Byte offset within record |
| Character encoding | Windows-1252 with `%UTF8%` escape | Raw bytes; strings length-prefixed |
| Container framing | Size-prefixed blocks (u32 header) | Size-prefixed subrecords (u8 type + u32 length) |

Both formats share the outer CFB/OLE container and the size-prefixed block
framing layer.

---

## 2. Schematic Text Format

### 2.1 Wire Format

```
[u32 block_header]
  bits [0..23]  = payload size in bytes (little-endian)
  bits [24..31] = flags byte (0x00 = text record, 0x01 = compressed)

[payload bytes]:
  |RECORD=2|OWNERINDEX=0|LOCATION.X=100|LOCATION.Y=200|NAME=VCC|DESIGNATOR=1|\0
```

- Leading `|` before the first key
- `|` separates key=value pairs
- `=` separates key from value
- Trailing null byte (`\0`) terminates the payload
- Keys are case-insensitive for matching

### 2.2 ParameterCollection

The internal representation for parsed text records. This is a transient
parsing structure -- it exists only during deserialization and is not stored
on the parsed record.

```rust
pub(crate) struct ParameterCollection {
    /// Keys and values parsed from the pipe-delimited text.
    /// NOT ordered -- we don't need to preserve key order because we
    /// serialize from typed struct fields, not from stored key-value pairs.
    entries: HashMap<String, String>,
}

impl ParameterCollection {
    /// Parse pipe-delimited text into key-value pairs.
    /// The text must start with '|' and contain only '|key=value' segments.
    pub(crate) fn parse(data: &[u8]) -> Result<Self>;

    /// Remove a key and return its value. Returns None if the key is not
    /// present (case-insensitive matching).
    pub(crate) fn remove(&mut self, key: &str) -> Option<String>;

    /// Remove a key and parse its value. Returns error if the key is
    /// present but the value fails to parse.
    pub(crate) fn remove_parse<T: FromParamValue>(&mut self, key: &str) -> Result<Option<T>>;

    /// Remove a key, parse its value, and error if the key is absent.
    pub(crate) fn remove_required<T: FromParamValue>(&mut self, key: &str) -> Result<T>;

    /// Return all remaining keys. Used to check for unknown parameters
    /// after all known fields have been consumed.
    pub(crate) fn remaining_keys(&self) -> Vec<&str>;

    /// Returns true if all keys have been consumed.
    pub(crate) fn is_empty(&self) -> bool;
}
```

**Why HashMap, not IndexMap?** We don't preserve key order. We don't do
round-trip preservation of the raw parameter text. We parse into typed struct
fields and serialize back from those fields. The serialize output will have
a canonical field order determined by the struct definition.

**Why remove-based parsing?** Each key is consumed as it's parsed. After all
known fields are processed, we check `remaining_keys()`. If anything remains,
parsing fails:

```rust
let remaining = params.remaining_keys();
if !remaining.is_empty() {
    return Err(AltiumFormatError::UnknownParameterKey {
        key: remaining[0].to_string(),
        record_type: "SchArc".to_string(),
    });
}
```

This is the core of the "fail fast" approach for text records. We must
account for every single key in the parameter string.

### 2.3 Parsing Must Error On

1. **Unrecognized keys** -- a key we don't have a field for means our model
   is incomplete. Error: `UnknownParameterKey`.

2. **Missing required keys** -- a key we expect but don't find means the
   file is malformed or our model is wrong. Error: `MissingRequiredParameter`.

3. **Invalid values** -- a value that doesn't parse to the expected type.
   Error: `InvalidParameterValue`.

4. **Unknown RECORD ids** -- a RECORD value we don't have a variant for.
   Error: `UnknownSchRecord`.

### 2.4 FromParamValue / ToParamValue

Single-value conversion traits for individual parameter values:

```rust
pub(crate) trait FromParamValue: Sized {
    fn from_param_value(value: &str) -> Result<Self>;
}

pub(crate) trait ToParamValue {
    fn to_param_value(&self) -> String;
}
```

Built-in implementations:

| Type | Parse | Serialize | Notes |
|------|-------|-----------|-------|
| `i32` | `"100"` -> `100` | `100` -> `"100"` | |
| `f64` | `"3.14"` -> `3.14` | `3.14` -> scientific notation | Match Altium's `E+` format |
| `bool` (short) | `"T"` -> true, `"F"` -> false | true -> `"T"` | SCH text records |
| `bool` (long) | `"TRUE"` -> true, `"FALSE"` -> false | true -> `"TRUE"` | PCB text records |
| `String` | passthrough | passthrough | |
| `Coord` | `"100"` -> `Coord(100)` | `Coord(100)` -> `"100"` | Raw internal units |
| `Color` | `"8388608"` -> `Color(0x800000)` | passthrough | Win32 COLORREF |
| `AltiumEnum` types | `"4"` -> variant | variant -> `"4"` | Via derive macro |

**Boolean format**: Schematic records use `T`/`F`. PCB text records use
`TRUE`/`FALSE`. The correct format is specified per-field via attribute:
`#[altium(param = "LOCKED", bool_format = "long")]`. Default is short (`T`/`F`).

**Error behavior**: `FromParamValue::from_param_value("xyz")` for an `i32`
field returns `Err(InvalidParameterValue)`. No silent defaults, no fallback
values.

---

## 3. PCB Binary Format

### 3.1 Wire Format

Each PCB section's `Data` stream contains records:

```
[u8  object_type_id]        -- TObjectId enum value (1-26)
[u32 subrecord_length]      -- little-endian payload size
[subrecord_length bytes]    -- binary payload
```

Some object types have multiple subrecords (PcbPad=6, PcbText=2, all others=1).
The subrecord count is determined by object type, not encoded in the data.

### 3.2 BinaryReader

A cursor wrapper that tracks consumed bytes:

```rust
pub(crate) struct BinaryReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BinaryReader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self;

    pub(crate) fn read_u8(&mut self) -> Result<u8>;
    pub(crate) fn read_i16le(&mut self) -> Result<i16>;
    pub(crate) fn read_u16le(&mut self) -> Result<u16>;
    pub(crate) fn read_i32le(&mut self) -> Result<i32>;
    pub(crate) fn read_u32le(&mut self) -> Result<u32>;
    pub(crate) fn read_f64le(&mut self) -> Result<f64>;
    pub(crate) fn read_bool(&mut self) -> Result<bool>;
    pub(crate) fn read_coord(&mut self) -> Result<Coord>;
    pub(crate) fn read_coord_point(&mut self) -> Result<CoordPoint>;
    pub(crate) fn read_pascal_string(&mut self) -> Result<String>;
    pub(crate) fn read_string_block(&mut self) -> Result<String>;
    pub(crate) fn read_bytes(&mut self, count: usize) -> Result<&'a [u8]>;

    /// How many bytes remain unconsumed.
    pub(crate) fn remaining(&self) -> usize;

    /// Assert that all bytes have been consumed. Returns error if not.
    pub(crate) fn assert_exhausted(&self, record_type: &str) -> Result<()>;
}
```

**Key invariant**: After parsing a record, we call `reader.assert_exhausted()`.
If there are unconsumed bytes, parsing fails:

```rust
impl BinaryReader<'_> {
    pub(crate) fn assert_exhausted(&self, record_type: &str) -> Result<()> {
        let remaining = self.remaining();
        if remaining > 0 {
            return Err(AltiumFormatError::UnexpectedTrailingBytes {
                record_type: record_type.to_string(),
                count: remaining,
            });
        }
        Ok(())
    }
}
```

This is the binary equivalent of checking for unknown parameter keys. Every
byte in the record must be read into a typed field. Trailing bytes mean our
struct definition is incomplete.

### 3.3 Parsing Must Error On

1. **Unexpected trailing bytes** -- bytes left after parsing all known fields.
   Error: `UnexpectedTrailingBytes`. Our struct is missing fields.

2. **Insufficient bytes** -- record is shorter than expected. Error:
   `BinaryLengthMismatch`. The file is truncated or our expected size is wrong.

3. **Unknown object type ID** -- a type byte we don't handle. Error:
   `UnknownPcbObjectType`. We need to add the record type.

4. **Unknown enum value in binary field** -- e.g., a pad shape byte of 15
   when our TShape enum only goes to 10. Error: `UnknownEnumVariant`.

### 3.4 Binary Type System

The derive macro attribute `#[altium(binary = "...")]` maps to these types:

| Attribute | Rust Type | Size | Read Method |
|-----------|-----------|------|-------------|
| `"u8"` | `u8` | 1 | `read_u8()` |
| `"i8"` | `i8` | 1 | `read_u8() as i8` |
| `"u16le"` | `u16` | 2 | `read_u16le()` |
| `"i16le"` | `i16` | 2 | `read_i16le()` |
| `"u32le"` | `u32` | 4 | `read_u32le()` |
| `"i32le"` | `i32` | 4 | `read_i32le()` |
| `"u64le"` | `u64` | 8 | `read_u64le()` |
| `"i64le"` | `i64` | 8 | `read_i64le()` |
| `"f32le"` | `f32` | 4 | `read_f32le()` |
| `"f64le"` | `f64` | 8 | `read_f64le()` |
| `"bool"` | `bool` | 1 | `read_bool()` (0=false, nonzero=true) |
| `"coord"` | `Coord` | 4 | `read_coord()` (i32le wrapped) |
| `"coord_point"` | `CoordPoint` | 8 | `read_coord_point()` (two i32le) |
| `"pascal_string"` | `String` | 1+N | `read_pascal_string()` |
| `"string_block"` | `String` | 4+N | `read_string_block()` |
| `"bytes(N)"` | `[u8; N]` | N | `read_bytes(N)` |

---

## 4. Derive Macro Design

### 4.1 AltiumDeserialize for Parameter Format

Given this struct:

```rust
#[derive(AltiumDeserialize, AltiumSerialize)]
#[altium(record_id = 12)]
pub struct SchArc {
    #[altium(flatten)]
    pub base: SchGraphicalBase,

    #[altium(param = "RADIUS", with_frac = "RADIUS_FRAC")]
    pub radius: Coord,

    #[altium(param = "LINEWIDTH")]
    pub line_width: PenWidth,

    #[altium(param = "STARTANGLE")]
    pub start_angle: f64,

    #[altium(param = "ENDANGLE")]
    pub end_angle: f64,

    #[altium(param = "COLOR")]
    pub color: Color,

    #[altium(param = "UNIQUEID", default)]
    pub unique_id: String,
}
```

The generated `FromParams` impl:

```rust
impl FromParams for SchArc {
    fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        // RECORD key already consumed by dispatch

        // 1. Parse flattened base (removes its keys from params)
        let base = SchGraphicalBase::from_params(params)?;

        // 2. Parse each named field (removes its key from params)
        let radius = params.remove_coord_with_frac("RADIUS", "RADIUS_FRAC")?;
        let line_width = params.remove_required::<PenWidth>("LINEWIDTH")?;
        let start_angle = params.remove_required::<f64>("STARTANGLE")?;
        let end_angle = params.remove_required::<f64>("ENDANGLE")?;
        let color = params.remove_required::<Color>("COLOR")?;
        let unique_id = params.remove_parse::<String>("UNIQUEID")?
            .unwrap_or_default();

        // 3. Check for unknown keys (THE CRITICAL CHECK)
        let remaining = params.remaining_keys();
        if !remaining.is_empty() {
            return Err(AltiumFormatError::UnknownParameterKey {
                key: remaining[0].to_string(),
                record_type: "SchArc".to_string(),
            });
        }

        Ok(Self { base, radius, line_width, start_angle, end_angle,
                  color, unique_id })
    }
}
```

The generated `ToParams` impl:

```rust
impl ToParams for SchArc {
    fn to_params(&self, params: &mut ParameterCollection) -> Result<()> {
        params.set("RECORD", "12");
        self.base.to_params(params)?;
        params.set_coord_with_frac("RADIUS", "RADIUS_FRAC", self.radius);
        params.set("LINEWIDTH", self.line_width.to_param_value());
        params.set("STARTANGLE", self.start_angle.to_param_value());
        params.set("ENDANGLE", self.end_angle.to_param_value());
        params.set("COLOR", self.color.to_param_value());
        if !self.unique_id.is_empty() {
            params.set("UNIQUEID", &self.unique_id);
        }
        Ok(())
    }
}
```

### 4.2 AltiumDeserialize for Binary Format

Given this struct:

```rust
#[derive(AltiumDeserialize, AltiumSerialize)]
#[altium(object_id = 4)]
pub struct PcbTrack {
    #[altium(flatten)]
    pub header: PcbPrimitiveHeader,

    #[altium(binary = "coord_point")]
    pub start: CoordPoint,

    #[altium(binary = "coord_point")]
    pub end: CoordPoint,

    #[altium(binary = "i32le")]
    pub width: Coord,
}
```

Generated `FromBinary`:

```rust
impl FromBinary for PcbTrack {
    fn from_binary(reader: &mut BinaryReader<'_>) -> Result<Self> {
        let header = PcbPrimitiveHeader::from_binary(reader)?;
        let start = reader.read_coord_point()?;
        let end = reader.read_coord_point()?;
        let width = reader.read_coord()?;

        // THE CRITICAL CHECK: no bytes may remain
        reader.assert_exhausted("PcbTrack")?;

        Ok(Self { header, start, end, width })
    }
}
```

Generated `ToBinary`:

```rust
impl ToBinary for PcbTrack {
    fn to_binary(&self, writer: &mut BinaryWriter) -> Result<()> {
        self.header.to_binary(writer)?;
        writer.write_coord_point(&self.start)?;
        writer.write_coord_point(&self.end)?;
        writer.write_coord(self.width)?;
        Ok(())
    }
}
```

### 4.3 AltiumEnum

```rust
#[derive(AltiumEnum)]
#[repr(u8)]
pub enum PadShape {
    NoShape = 0,
    Rounded = 1,
    Rectangular = 2,
    Octagonal = 3,
    CircleShape = 4,
    ArcShape = 5,
    Terminator = 6,
    RoundRectShape = 7,
    RotatedRectShape = 8,
    RoundedRectangular = 9,
    CustomShape = 10,
}
```

Generates:

```rust
impl TryFrom<u8> for PadShape {
    type Error = AltiumFormatError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::NoShape),
            1 => Ok(Self::Rounded),
            2 => Ok(Self::Rectangular),
            3 => Ok(Self::Octagonal),
            4 => Ok(Self::CircleShape),
            5 => Ok(Self::ArcShape),
            6 => Ok(Self::Terminator),
            7 => Ok(Self::RoundRectShape),
            8 => Ok(Self::RotatedRectShape),
            9 => Ok(Self::RoundedRectangular),
            10 => Ok(Self::CustomShape),
            _ => Err(AltiumFormatError::UnknownEnumVariant {
                value: value as i64,
                enum_type: "PadShape".to_string(),
            }),
        }
    }
}

impl From<PadShape> for u8 {
    fn from(v: PadShape) -> u8 {
        v as u8  // safe because #[repr(u8)]
    }
}

impl FromParamValue for PadShape {
    fn from_param_value(value: &str) -> Result<Self> {
        let n: u8 = value.parse().map_err(|_| AltiumFormatError::InvalidParameterValue {
            key: "PadShape".into(),
            value: value.into(),
            reason: "not a valid integer".into(),
        })?;
        Self::try_from(n)
    }
}
```

**No default fallback.** Unknown values are errors. This forces us to keep
our enum definitions complete.

### 4.4 Field Attribute Reference

**Struct-level attributes:**

| Attribute | Purpose |
|-----------|---------|
| `#[altium(record_id = N)]` | RECORD=N for schematic dispatch |
| `#[altium(object_id = N)]` | Object ID byte for PCB dispatch |

The format (param vs binary) is inferred: `record_id` implies parameter
format, `object_id` implies binary format.

**Field-level attributes for parameter format:**

| Attribute | Purpose | Required? |
|-----------|---------|-----------|
| `#[altium(param = "KEY")]` | Map field to parameter key | Yes (for param fields) |
| `#[altium(param = "KEY", with_frac = "KEY_FRAC")]` | DXP fractional coordinate pair | For Coord fields |
| `#[altium(param = "KEY", default)]` | Use `Default::default()` if key absent | No |
| `#[altium(param = "KEY", bool_format = "long")]` | Use TRUE/FALSE instead of T/F | No |
| `#[altium(flatten)]` | Inline another struct's fields | For base types |
| `#[altium(indexed_coords, x = "X", y = "Y", count = "LOCATIONCOUNT")]` | Variable-length vertex array | For polylines/wires |
| `#[altium(skip)]` | Exclude from serialization | No |

**Field-level attributes for binary format:**

| Attribute | Purpose |
|-----------|---------|
| `#[altium(binary = "TYPE")]` | Read/write as binary type (see type table) |
| `#[altium(flatten)]` | Inline another struct's binary fields |
| `#[altium(skip)]` | Exclude from serialization |

---

## 5. Multi-Subrecord Objects (PCB)

### 5.1 Problem

Most PCB primitives have exactly one subrecord, but PcbPad has 6 and PcbText
has 2. The subrecord count is not encoded in the data -- it's hardcoded per
object type.

### 5.2 Approach

```rust
pub(crate) fn subrecord_count(object_id: u8) -> usize {
    match object_id {
        2 => 6,  // PcbPad
        5 => 2,  // PcbText
        _ => 1,  // All other types
    }
}
```

The section reader reads all subrecords for an object, then passes them to the
type's parser:

```rust
pub(crate) fn read_pcb_primitive(reader: &mut BinaryReader<'_>) -> Result<PcbRecord> {
    let object_id = reader.read_u8()?;
    let count = subrecord_count(object_id);

    let mut subrecords = Vec::with_capacity(count);
    for _ in 0..count {
        let length = reader.read_u32le()? as usize;
        let data = reader.read_bytes(length)?;
        subrecords.push(data);
    }

    dispatch_pcb_record(object_id, &subrecords)
}
```

For multi-subrecord types, each subrecord is parsed by a dedicated method:

```rust
impl PcbPad {
    pub(crate) fn from_subrecords(subrecords: &[&[u8]]) -> Result<Self> {
        if subrecords.len() != 6 {
            return Err(AltiumFormatError::BinaryLengthMismatch {
                record_type: "PcbPad".to_string(),
                expected: 6,
                actual: subrecords.len(),
            });
        }

        let mut r0 = BinaryReader::new(subrecords[0]);
        let main = PcbPadMain::from_binary(&mut r0)?;
        r0.assert_exhausted("PcbPad subrecord 0")?;

        let mut r1 = BinaryReader::new(subrecords[1]);
        let hole = PcbPadHole::from_binary(&mut r1)?;
        r1.assert_exhausted("PcbPad subrecord 1")?;

        // ... subrecords 2-5
        // Each subrecord is fully consumed or we error.

        Ok(Self::from_parts(main, hole, /* ... */))
    }
}
```

**Every subrecord is fully consumed.** No leftover bytes in any subrecord.

---

## 6. Sidecar Streams

### 6.1 What Sidecars Are

Altium evolved its file format by adding supplementary streams alongside the
original data. These "sidecar" streams contain additional or replacement field
values that must be merged into records after initial loading.

**Our approach:** Sidecar data is merged eagerly at load time. After
`SchDoc::open()` returns, every record field is fully populated from both the
primary record and any applicable sidecar streams. The sidecar merging is
invisible to the consumer.

### 6.2 Schematic Sidecars (SchLib, per component)

Loaded in this order after the base records:

| Sidecar Stream | Format | Per-Pin Data | Purpose |
|----------------|--------|-------------|---------|
| `PinFrac` | 12 bytes binary | loc_x_frac, loc_y_frac, length_frac (3x i32le) | Fractional coordinate precision |
| `PinDesc` | ASCII text | Full description string | Overflow for descriptions > 255 chars |
| `PinMiscData` | UTF-16LE parameter blocks | PINPROPAGATIONDELAY, PINPACKAGELENGTH, etc. | Additional pin properties |
| `PinTextData` | Length-prefixed binary | Hidden net name | Net name as length-prefixed string |
| `PinWideText` | UTF-16LE parameter blocks | NAME, DESIGNATOR | Unicode pin name/designator |
| `PinSymbolLineWidth` | 4 bytes binary | Line width (i32le) | Symbol line width |
| `PinPackageLength` | 12 bytes binary | Package length + frac + unknown | Package length data |
| `PinPropagationDelay` | 12 bytes binary | Prop delay + frac + unknown | Timing data |
| `PinFunctionData` | 4 bytes binary | Formal type (i32le) | VHDL formal type |

### 6.3 Schematic Sidecars (SchDoc, global)

| Sidecar Stream | Format | Purpose |
|----------------|--------|---------|
| `WideStrings` | Parameter blocks | Unicode text replacements for all records |
| `UniqueIDs` | Parameter blocks | Unique ID assignments |
| `ExtendedPrimitiveInfo` | Parameter blocks | Extended property overrides |

### 6.4 PCB Sidecars (PcbDoc, global)

| Sidecar Stream | Format | Purpose |
|----------------|--------|---------|
| `WideStrings6/Data` | Binary TLV: `[u32 index][u32 len][UTF-16LE]` | Unicode text for PcbText records |
| `UniqueIDPrimitiveInformation` | Parameter blocks | `PRIMITIVEINDEX`, `UNIQUEID`, `PRIMITIVEKIND` |
| `ExtendedPrimitiveInformation` | Parameter blocks | Mask expansion mode overrides |
| `PrimitiveGuids` | 24 bytes binary per primitive | Checksum/GUID data |

### 6.5 PCB Sidecars (PcbLib, per footprint)

| Sidecar Stream | Format | Purpose |
|----------------|--------|---------|
| `WideStrings` | Parameter blocks (NOT binary TLV) | Unicode text |
| `UniqueIDPrimitiveInformation/Data` | Parameter blocks | Unique IDs |
| `PrimitiveGuids/Data` | 24 bytes binary | GUIDs |

**Key difference:** PcbDoc WideStrings6 uses binary TLV format. PcbLib
WideStrings uses parameter block format. They look similar but parse
differently.

### 6.6 Sidecar Merge Error Handling

- **Missing sidecar stream:** NOT an error. Older files don't have them.
  Fields that would be populated by the sidecar retain their base values.
- **Sidecar present but malformed:** ERROR. If the stream exists, it must
  parse correctly. Corrupt sidecar data is `AltiumFormatError::SidecarError`.
- **Sidecar count mismatch:** ERROR. If PinFrac has 5 entries but there are
  7 pins, that's a corrupt file.
- **Sidecar data fully consumed:** Every byte in the sidecar must be accounted
  for. Extra bytes at the end are an error.

### 6.7 Sidecar Serialization

When writing a file, sidecar streams are regenerated from the record data:

```rust
pub(crate) fn generate_pin_frac(pins: &[SchPin]) -> Vec<u8> {
    let mut data = Vec::with_capacity(pins.len() * 12);
    for pin in pins {
        let (_, loc_x_frac) = encode_dxp_coord(pin.base.location_x.raw());
        let (_, loc_y_frac) = encode_dxp_coord(pin.base.location_y.raw());
        let (_, length_frac) = encode_dxp_coord(pin.pin_length.raw());
        data.extend_from_slice(&loc_x_frac.to_le_bytes());
        data.extend_from_slice(&loc_y_frac.to_le_bytes());
        data.extend_from_slice(&length_frac.to_le_bytes());
    }
    data
}
```

The sidecar generation functions are deterministic -- given the same record
data, they always produce the same bytes.

---

## 7. String Encoding

### 7.1 Windows-1252

The primary text encoding for schematic records. All pipe-delimited parameter
text is Windows-1252 (a superset of ISO-8859-1/Latin-1).

```rust
pub(crate) fn decode_windows_1252(data: &[u8]) -> Result<String> {
    let (cow, _, had_errors) = encoding_rs::WINDOWS_1252.decode(data);
    if had_errors {
        return Err(AltiumFormatError::Encoding(
            "invalid Windows-1252 byte sequence".into()
        ));
    }
    Ok(cow.into_owned())
}

pub(crate) fn encode_windows_1252(text: &str) -> Result<Vec<u8>> {
    let (cow, _, had_errors) = encoding_rs::WINDOWS_1252.encode(text);
    if had_errors {
        return Err(AltiumFormatError::Encoding(
            format!("cannot encode as Windows-1252: {:?}", text)
        ));
    }
    Ok(cow.into_owned())
}
```

### 7.2 %UTF8% Prefix Handling

For values containing characters outside Windows-1252, Altium prefixes the
**key** with `%UTF8%`:

```
|%UTF8%COMPONENTDESCRIPTION=<UTF-8 bytes re-encoded through Windows-1252>|
```

The value bytes are UTF-8 encoded text stored as raw bytes in a Windows-1252
stream. To decode:

1. Detect `%UTF8%` prefix on the key
2. Strip prefix to get real key name
3. The raw bytes of the value ARE the UTF-8 bytes (not re-encoded through
   Windows-1252 -- the bytes are stored directly)
4. Decode as UTF-8

```rust
pub(crate) fn decode_param_key_value(
    raw_key: &[u8],
    raw_value: &[u8],
) -> Result<(String, String)> {
    let key_str = decode_windows_1252(raw_key)?;

    if let Some(real_key) = key_str.strip_prefix("%UTF8%") {
        // Value is raw UTF-8 bytes
        let value = String::from_utf8(raw_value.to_vec())
            .map_err(|e| AltiumFormatError::Encoding(e.to_string()))?;
        Ok((real_key.to_string(), value))
    } else {
        let value = decode_windows_1252(raw_value)?;
        Ok((key_str, value))
    }
}
```

### 7.3 UTF-16LE in Sidecar Streams

PCB WideStrings and SchLib PinWideText use UTF-16 Little Endian:

```rust
pub(crate) fn decode_utf16le(data: &[u8]) -> Result<String> {
    if data.len() % 2 != 0 {
        return Err(AltiumFormatError::Encoding(
            "odd byte count for UTF-16LE".into()
        ));
    }
    let u16_values: Vec<u16> = data.chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    String::from_utf16(&u16_values)
        .map_err(|e| AltiumFormatError::Encoding(e.to_string()))
}
```

### 7.4 PCB Binary Strings

Two formats in binary records:

- **Pascal string**: `[u8 length][length bytes]` -- max 255 chars
- **String block**: `[i32le length][length bytes]` -- for longer strings

Both store raw bytes, typically ASCII. Unicode content comes from the
WideStrings sidecar (which replaces the ASCII text post-load).

### 7.5 Encoding Summary

| Context | Encoding | Error on invalid? |
|---------|----------|-------------------|
| SCH parameter text | Windows-1252 | Yes |
| SCH `%UTF8%` values | UTF-8 (raw bytes) | Yes |
| SchLib PinWideText | UTF-16LE | Yes |
| PCB binary strings | ASCII (raw bytes) | Yes (if non-ASCII) |
| PcbDoc WideStrings6 | UTF-16LE (TLV) | Yes |
| PcbLib WideStrings | Windows-1252 (param format) | Yes |
| PCB text sections (Components6, etc.) | Windows-1252 | Yes |

---

## 8. Coordinate Serialization

### 8.1 Internal Representation

```rust
/// A coordinate value in Altium internal units.
/// 10,000 internal units = 1 mil (0.001 inch).
/// 393,701 internal units ~= 1 mm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Coord(pub i32);
```

### 8.2 DXP Fractional Encoding (Schematic Text)

Schematic records split coordinates into integer and fractional parameters:

```
LOCATION.X=100          -- integer part (each unit = 10 mils)
LOCATION.X_FRAC=5000    -- fractional part (0-99999)

raw = 100 * 100000 + 5000 = 10,005,000 internal units = 1000.5 mils
```

Source: `Rt_Schematic.Consts.cBaseUnit = 100000` — each DXP unit is 100,000
internal units (10 mils). This is confirmed by `SchDataUtils.GetCoord_DXP2004SP1_To_DXP2004SP2`.

```rust
pub(crate) fn decode_dxp_coord(integer: i32, frac: i32) -> Coord {
    Coord(integer * 100_000 + frac)
}

pub(crate) fn encode_dxp_coord(raw: i32) -> (i32, i32) {
    let integer = raw.div_euclid(100_000);
    let frac = raw.rem_euclid(100_000);
    debug_assert!((0..100_000).contains(&frac));
    (integer, frac)
}
```

**Writing rules:**
- If frac == 0, omit the `_FRAC` parameter
- Frac is always in range [0, 99999]
- Negative coordinates use Euclidean division: `raw = -5000` -> `integer = -1, frac = 95000`

### 8.3 PCB Binary Coordinates

Stored directly as i32 little-endian:

```rust
pub(crate) fn read_coord(reader: &mut BinaryReader<'_>) -> Result<Coord> {
    Ok(Coord(reader.read_i32le()?))
}
```

### 8.4 PCB Text Section Coordinates

PCB text sections use mil-suffixed string values:

```
X=4686.0219mil
```

```rust
pub(crate) fn parse_mil_coord(value: &str) -> Result<Coord> {
    let trimmed = value.strip_suffix("mil")
        .ok_or_else(|| AltiumFormatError::InvalidParameterValue {
            key: "coord".into(),
            value: value.into(),
            reason: "missing 'mil' suffix".into(),
        })?;
    let mils: f64 = trimmed.parse()
        .map_err(|_| AltiumFormatError::InvalidParameterValue {
            key: "coord".into(),
            value: value.into(),
            reason: "not a valid number".into(),
        })?;
    Ok(Coord::from_mils(mils))
}
```

---

## 9. Record Dispatch

### 9.1 Schematic Dispatch

```rust
pub(crate) fn parse_sch_record(params: &mut ParameterCollection) -> Result<SchRecord> {
    let record_id = params.remove_required::<i32>("RECORD")?;

    match record_id {
        1  => Ok(SchRecord::Component(SchComponent::from_params(params)?)),
        2  => Ok(SchRecord::Pin(SchPin::from_params(params)?)),
        3  => Ok(SchRecord::Symbol(SchSymbol::from_params(params)?)),
        4  => Ok(SchRecord::Label(SchLabel::from_params(params)?)),
        5  => Ok(SchRecord::Bezier(SchBezier::from_params(params)?)),
        6  => Ok(SchRecord::Polyline(SchPolyline::from_params(params)?)),
        7  => Ok(SchRecord::Polygon(SchPolygon::from_params(params)?)),
        8  => Ok(SchRecord::Ellipse(SchEllipse::from_params(params)?)),
        9  => Ok(SchRecord::Pie(SchPie::from_params(params)?)),
        10 => Ok(SchRecord::RoundRectangle(SchRoundRectangle::from_params(params)?)),
        11 => Ok(SchRecord::EllipticalArc(SchEllipticalArc::from_params(params)?)),
        12 => Ok(SchRecord::Arc(SchArc::from_params(params)?)),
        13 => Ok(SchRecord::Line(SchLine::from_params(params)?)),
        14 => Ok(SchRecord::Rectangle(SchRectangle::from_params(params)?)),
        15 => Ok(SchRecord::SheetSymbol(SchSheetSymbol::from_params(params)?)),
        16 => Ok(SchRecord::SheetEntry(SchSheetEntry::from_params(params)?)),
        17 => Ok(SchRecord::PowerObject(SchPowerObject::from_params(params)?)),
        18 => Ok(SchRecord::Port(SchPort::from_params(params)?)),
        22 => Ok(SchRecord::NoErc(SchNoErc::from_params(params)?)),
        23 => Ok(SchRecord::ErrorMarker(SchErrorMarker::from_params(params)?)),
        25 => Ok(SchRecord::NetLabel(SchNetLabel::from_params(params)?)),
        26 => Ok(SchRecord::Bus(SchBus::from_params(params)?)),
        27 => Ok(SchRecord::Wire(SchWire::from_params(params)?)),
        28 => Ok(SchRecord::TextFrame(SchTextFrame::from_params(params)?)),
        29 => Ok(SchRecord::Junction(SchJunction::from_params(params)?)),
        30 => Ok(SchRecord::Image(SchImage::from_params(params)?)),
        31 => Ok(SchRecord::Sheet(SchSheet::from_params(params)?)),
        32 => Ok(SchRecord::SheetName(SchSheetName::from_params(params)?)),
        33 => Ok(SchRecord::SheetFileName(SchSheetFileName::from_params(params)?)),
        34 => Ok(SchRecord::Designator(SchDesignator::from_params(params)?)),
        37 => Ok(SchRecord::BusEntry(SchBusEntry::from_params(params)?)),
        39 => Ok(SchRecord::Template(SchTemplate::from_params(params)?)),
        41 => Ok(SchRecord::Parameter(SchParameter::from_params(params)?)),
        43 => Ok(SchRecord::ParameterSet(SchParameterSet::from_params(params)?)),
        44 => Ok(SchRecord::ImplementationList(SchImplementationList::from_params(params)?)),
        45 => Ok(SchRecord::Implementation(SchImplementation::from_params(params)?)),
        46 => Ok(SchRecord::ImplementationMap(SchImplementationMap::from_params(params)?)),
        47 => Ok(SchRecord::MapDefiner(SchMapDefiner::from_params(params)?)),
        48 => Ok(SchRecord::ImplementationParameters(SchImplementationParameters::from_params(params)?)),
        209 => Ok(SchRecord::Note(SchNote::from_params(params)?)),
        210 => Ok(SchRecord::Probe(SchProbe::from_params(params)?)),
        225 => Ok(SchRecord::CompileMask(SchCompileMask::from_params(params)?)),
        // Harness records 104-138, 215-218, 220-226, 240-241 ...
        _ => Err(AltiumFormatError::UnknownSchRecord { record_id }),
    }
}
```

**Unknown RECORD values are errors.** Not warnings, not captured as Unknown
variants. If we hit a RECORD id we don't handle, we must add it.

### 9.2 PCB Dispatch

```rust
pub(crate) fn dispatch_pcb_record(
    object_id: u8,
    subrecords: &[&[u8]],
) -> Result<PcbRecord> {
    match object_id {
        1  => Ok(PcbRecord::Arc(PcbArc::from_subrecords(subrecords)?)),
        2  => Ok(PcbRecord::Pad(PcbPad::from_subrecords(subrecords)?)),
        3  => Ok(PcbRecord::Via(PcbVia::from_subrecords(subrecords)?)),
        4  => Ok(PcbRecord::Track(PcbTrack::from_subrecords(subrecords)?)),
        5  => Ok(PcbRecord::Text(PcbText::from_subrecords(subrecords)?)),
        6  => Ok(PcbRecord::Fill(PcbFill::from_subrecords(subrecords)?)),
        10 => Ok(PcbRecord::Polygon(PcbPolygon::from_subrecords(subrecords)?)),
        11 => Ok(PcbRecord::Region(PcbRegion::from_subrecords(subrecords)?)),
        12 => Ok(PcbRecord::ComponentBody(PcbComponentBody::from_subrecords(subrecords)?)),
        13 => Ok(PcbRecord::Dimension(PcbDimension::from_subrecords(subrecords)?)),
        14 => Ok(PcbRecord::Coordinate(PcbCoordinate::from_subrecords(subrecords)?)),
        _  => Err(AltiumFormatError::UnknownPcbObjectType { object_id }),
    }
}
```

Same principle: unknown object IDs are errors.

---

## 10. Version Handling

### 10.1 Target: AD26 (Latest)

We target Altium Designer 26 file formats exclusively. This means:

- **Schematic:** V5 text format (pipe-delimited key=value)
- **PCB:** V6 binary format (19-byte common header, full section set)
- **Container:** CFB Version 3

Older format versions (V3, V4 for PCB; V4 binary for schematic) are out of
scope for the initial implementation. They can be added later as separate
code paths if needed.

### 10.2 Format Identification

**Schematic files:** Identified by the HEADER parameter in the FileHeader block:

```
HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0
HEADER=Protel for Windows - Schematic Library Editor Binary File Version 5.0
```

If the header string doesn't match, return `AltiumFormatError::InvalidFormat`.

**PCB files:** Identified by the FileHeader content:

```
PCB 6.0 Binary File
PCB 6.0 Binary Library File
```

### 10.3 MinorVersion Handling

Within V5 schematic format, `MinorVersion` in the FileHeader indicates
sub-format variations:

- MinorVersion=2 (older): Sidecar streams present (PinFrac, PinPackageLength, etc.)
- MinorVersion=9 (newer): Some sidecar data may be stored inline

Our approach: Always attempt to load sidecar streams. Missing streams are
not errors (they simply don't exist in that format version). Corrupt streams
are errors.

### 10.4 PCB Feature Flags

The Board6 section contains feature flags that determine which sections exist:

```rust
pub(crate) struct PcbFeatureFlags {
    pub nets: bool,
    pub classes: bool,
    pub rules: bool,
    pub polygons: bool,
    pub dimensions: bool,
    pub components: bool,
    pub regions: bool,
    pub component_bodies: bool,
    // ...
}
```

A section's absence is only acceptable if its feature flag is false. If the
flag says the section should exist but it's missing from the CFB container,
that's an error.

---

## 11. Complete Loading Pipelines

### 11.1 SchDoc

```
1. Open CFB container
2. Read /FileHeader stream
3. Parse block 0: verify HEADER string, extract WEIGHT, MinorVersion
4. For each subsequent block in /FileHeader:
   a. Read u32 block header (size + flags)
   b. If flags != 0x00: error (unexpected non-text block in FileHeader)
   c. Parse payload as ParameterCollection
   d. Remove RECORD key, dispatch to record type
   e. Parse all remaining keys into typed fields
   f. Error if any keys remain (unknown parameter)
   g. Append to records list
5. Read /Additional stream (if present)
   a. Parse header block, then parse additional records
6. Read /Storage stream (if present)
   a. Parse embedded resources (compressed blocks with 0xD0 magic)
7. Attempt sidecar merges (each is optional):
   a. WideStrings -> replace text fields with Unicode versions
   b. UniqueIDs -> assign unique IDs to records
   c. ExtendedPrimitiveInfo -> merge extended properties
8. Build ownership tree from OWNERINDEX values
   (validate: no dangling indices, no cycles)
9. Return SchDoc
```

### 11.2 SchLib

```
1. Open CFB container
2. Read /FileHeader
   a. Parse HEADER, COMPCOUNT, LIBREF0..N, PARTCOUNT0..N, etc.
   b. Build component index table
3. Read /SectionKeys (if present) for long component names
4. For each component (by index):
   a. Resolve storage name (direct or via SectionKeys)
   b. Read /{ComponentName}/Data stream
   c. Parse block 0: RECORD=1 (SchComponent)
   d. Parse subsequent blocks: child records
   e. Error on unknown RECORD types
   f. Error on unknown parameter keys in any record
   g. Attempt per-component sidecar merges:
      - PinFrac, PinDesc, PinMiscData, PinTextData,
        PinWideText, PinSymbolLineWidth, PinPackageLength,
        PinPropagationDelay, PinFunctionData
   h. Check for redirect (alias) storage
5. Return SchLib with components list
```

### 11.3 PcbDoc

```
1. Open CFB container
2. Read FileHeader: verify "PCB 6.0 Binary File"
3. Read Board6/Data: parse board parameters, extract feature flags
4. For each binary primitive section (Arcs6, Pads6, Vias6, ...):
   a. Read {Section}/Header: get expected record count
   b. Read {Section}/Data:
      - For each record:
        i.   Read u8 object_id
        ii.  Read subrecord(s) (count determined by object_id)
        iii. Parse binary payload via FromBinary
        iv.  Assert all bytes consumed (no trailing)
   c. Verify record count matches header
   d. Store in typed Vec
5. For each text section (Components6, Nets6, Rules6, Classes6,
   Polygons6, DifferentialPairs6):
   a. Parse as size-prefixed parameter blocks
   b. Error on unknown parameter keys
   c. Store in typed Vec
6. Read Models/Data: parse 3D model references
7. Merge global sidecar streams:
   a. WideStrings6/Data: binary TLV -> merge Unicode text
   b. UniqueIDPrimitiveInformation: assign unique IDs
   c. ExtendedPrimitiveInformation: merge mask expansion modes
   d. PrimitiveGuids: assign GUIDs
8. Return PcbDoc
```

### 11.4 PcbLib

```
1. Open CFB container
2. Read FileHeader: verify "PCB 6.0 Binary Library File"
3. Read Library/Data: parse board-level properties
4. Read Library/ComponentParamsTOC/Data: build footprint index
5. For each footprint:
   a. Read {Footprint}/Parameters: pattern, height, description
   b. Read {Footprint}/Data: pattern name prefix + binary primitives
   c. Parse each primitive (same as PcbDoc binary records)
   d. Assert all bytes consumed per record
   e. Read per-footprint sidecar streams:
      - WideStrings (parameter format, NOT binary TLV)
      - UniqueIDPrimitiveInformation/Data
      - PrimitiveGuids/Data (if present)
6. Return PcbLib
```

---

## 12. Serialization Output (Writing)

### 12.1 Principle

We do NOT preserve the original byte layout. We serialize from typed structs.
This means:

- Parameter key order may differ from the original file
- Binary padding bytes are always zero
- Compressed blocks may compress to different bytes (zlib non-determinism)
- All values are in canonical form

**This is intentional.** Since we don't store unknown data, we don't need
byte-for-byte round-trip. Our output is semantically equivalent, not
byte-identical. The fields contain the same values; the encoding may differ
in non-semantic ways.

### 12.2 SchDoc/SchLib Write Pipeline

```
1. Create new CFB container
2. Write /FileHeader:
   a. Block 0: HEADER string, WEIGHT (computed), MinorVersion
   b. Block 1: RECORD=31 (sheet properties) via ToParams
   c. Blocks 2..N: each SchRecord via ToParams
3. Write /Additional (if needed)
4. Write /Storage (embedded resources, re-compressed)
5. Generate and write sidecar streams from record data
6. Close CFB container
```

### 12.3 PcbDoc/PcbLib Write Pipeline

```
1. Create new CFB container
2. Write FileHeader
3. Write Board6/Data from board parameters
4. For each binary section:
   a. Write {Section}/Header with record count
   b. Write {Section}/Data: for each record, ToBinary -> subrecord framing
5. For each text section:
   a. Write records as parameter blocks
6. Write Models/Data
7. Generate and write sidecar streams from record data
8. Close CFB container
```

---

## 13. Summary of Strictness Guarantees

| Check | SCH Text | PCB Binary | Result on Failure |
|-------|----------|------------|-------------------|
| Unknown record type | RECORD=N not in dispatch | object_id not in dispatch | `UnknownSchRecord` / `UnknownPcbObjectType` |
| Unknown parameter key | Key not mapped to any field | N/A | `UnknownParameterKey` |
| Trailing bytes | N/A (text format) | Bytes after last field | `UnexpectedTrailingBytes` |
| Missing required field | Key absent, no `default` attr | Record too short | `MissingRequiredParameter` / `BinaryLengthMismatch` |
| Invalid value | Can't parse to target type | N/A | `InvalidParameterValue` |
| Unknown enum variant | Integer not in enum | Byte not in enum | `UnknownEnumVariant` |
| Sidecar count mismatch | Entries != record count | Entries != record count | `SidecarError` |
| Sidecar extra bytes | N/A | Bytes after all entries | `SidecarError` |

Every check returns a `Result::Err`. None of them log a warning and continue.
None of them substitute a default value. None of them skip the offending data.

**If we can parse a file, we understand it completely.
If we can't parse it, we tell you exactly why.**

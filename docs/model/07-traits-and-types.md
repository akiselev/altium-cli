# Traits and Core Type Definitions

Rust trait system, error types, and detailed type definitions for `altium-format`.

**Design Philosophy**: Strict parsing. No unknown field preservation, no trailing byte
capture, no "lenient mode". If it parsed, it's valid. If we encounter data we don't
understand, that is a parse error -- a bug in our code. These files go to PCB fabrication;
better to crash than to silently produce wrong output.

---

## 1. Core Traits

### 1.1 Serialization Traits

These are the fundamental I/O traits. Every record type must implement one or both
serialization paths. They are **strict**: leftover bytes or unknown keys are errors.

```rust
/// Deserialize from pipe-delimited key=value parameter collection.
/// Used by schematic records and PCB text-format sections (Components6, Nets6, etc.).
///
/// STRICT: After deserializing all known fields, if any unrecognized keys remain
/// in the ParameterCollection, this MUST return Err(AltiumFormatError::UnknownField).
pub(crate) trait FromParams: Sized {
    fn from_params(params: &ParameterCollection) -> Result<Self>;
}

/// Serialize to pipe-delimited key=value parameter collection.
pub(crate) trait ToParams {
    fn to_params(&self) -> Result<ParameterCollection>;
}

/// Deserialize from a binary byte stream (little-endian).
/// Used by PCB binary records.
///
/// STRICT: After deserializing all known fields, if any bytes remain unread
/// in the record payload, this MUST return Err(AltiumFormatError::UnexpectedTrailingData).
pub(crate) trait FromBinary: Sized {
    fn from_binary(reader: &mut BinaryReader<'_>) -> Result<Self>;
}

/// Serialize to a binary byte stream (little-endian).
pub(crate) trait ToBinary {
    fn to_binary(&self, writer: &mut BinaryWriter<'_>) -> Result<()>;
}
```

**Key difference from old design**: No `_preserving` variants. No `UnknownFields`.
No `unknown_bytes`. The strict variants ARE the only variants. Encountering unknown
data is always an error.

**Design trade-offs:**

| Option | Pros | Cons |
|--------|------|------|
| Lenient parsing + round-trip preservation (old) | Handles any file; unknown data survives | Silently hides parser bugs; risks data corruption |
| **Strict parsing (chosen)** | Every byte accounted for; parser completeness enforced; safe for fabrication | Must fully model every field; new Altium versions break us until we update |
| Combined `Serialize`/`Deserialize` | Simpler API | Param vs binary are completely different formats |
| `serde`-based | Ecosystem compatibility | DXP fractional coords, indexed arrays, binary subrecords don't map to serde's model |

### 1.2 Value Conversion Traits

```rust
/// Convert a single parameter string value to a Rust type.
/// Example: "100" -> i32(100), "T" -> bool(true), "3" -> PinElectricalType::OpenCollector
///
/// Errors on invalid input (no silent defaults).
pub(crate) trait FromParamValue: Sized {
    fn from_param_value(value: &str) -> Result<Self>;
}

/// Convert a Rust type to a parameter string value.
pub(crate) trait ToParamValue {
    fn to_param_value(&self) -> String;
}
```

Standard implementations:
- `i32`, `i64`, `f64`: numeric string conversion (error on non-numeric input)
- `bool`: `"T"`/`"F"` or `"TRUE"`/`"FALSE"` (context-dependent; error on other values)
- `String`: identity
- `Coord`: integer string (raw internal units)
- `Color`: integer string (COLORREF value)
- All `#[derive(AltiumEnum)]` types: integer discriminant string (error on unknown discriminant)

### 1.3 Primitive Traits

```rust
/// Trait for schematic records that participate in the document object model.
pub(crate) trait SchPrimitive {
    /// The RECORD=N value identifying this type.
    const RECORD_ID: SchRecordType;

    /// Owner index in the flat record list (-1 for root-level records).
    fn owner_index(&self) -> i32;
    fn set_owner_index(&mut self, index: i32);

    /// Compute the bounding box of this primitive.
    fn bounding_box(&self) -> BoundingBox;

    /// Human-readable name for this record type (e.g., "Component", "Pin").
    fn record_type_name(&self) -> &'static str;
}

/// Trait for PCB records that participate in the board object model.
pub(crate) trait PcbPrimitive {
    /// The object_id byte identifying this type.
    const OBJECT_ID: PcbObjectId;

    /// The V6 layer this primitive resides on.
    fn layer(&self) -> V6Layer;

    /// Net index (-1 for unconnected).
    fn net(&self) -> i16;

    /// Component index (-1 for board-level primitives).
    fn component(&self) -> i16;

    /// Compute the bounding box of this primitive.
    fn bounding_box(&self) -> BoundingBox;
}
```

### 1.4 Trait Hierarchy

```
                    FromParams / ToParams
                    FromBinary / ToBinary
                    FromParamValue / ToParamValue
                           |
                +----------+----------+
                |                     |
          SchPrimitive          PcbPrimitive
          (RECORD_ID,          (OBJECT_ID,
           owner_index,         layer, net,
           bounding_box)        component,
                |                bounding_box)
                |                     |
     HasLocation (opt.)     HasLocation (opt.)
     HasColor (opt.)        HasNet (opt.)
     HasText (opt.)         HasText (opt.)
```

There is no deep trait hierarchy. Most behavior is captured by:
1. Serialization traits (mechanical -- generated by derive macros)
2. Primitive traits (domain logic -- one per domain)
3. A few optional query traits (described below)

### 1.5 Query Traits

These enable filtering and querying across record types.

```rust
/// Record has a spatial location (center/origin point).
pub(crate) trait HasLocation {
    fn location(&self) -> CoordPoint;
}

/// Record has a color.
pub(crate) trait HasColor {
    fn color(&self) -> Color;
    fn area_color(&self) -> Color;
}

/// Record has a net association (PCB-domain).
pub(crate) trait HasNet {
    fn net_index(&self) -> i16;
}

/// Record has a layer association (PCB-domain).
pub(crate) trait HasLayer {
    fn layer(&self) -> V6Layer;
}

/// Record has text content.
pub(crate) trait HasText {
    fn text(&self) -> &str;
}

/// Record has a unique identifier.
pub(crate) trait HasUniqueId {
    fn unique_id(&self) -> &str;
}
```

**Design decision -- query traits vs methods on enum:**

| Option | Pros | Cons |
|--------|------|------|
| Query traits on inner structs (chosen) | Clean per-type implementation; composable; impl only where meaningful | Must dispatch through enum to reach inner type |
| Methods directly on `SchRecord`/`PcbRecord` enum | Single dispatch point | Returns `Option` for non-applicable variants; large match arms |
| Both (trait on inner + forwarding on enum) | Best of both | More code to maintain |

Recommended: **traits on inner structs** with **forwarding methods on the container
types** (`SchDoc`, `PcbDoc`) for frequently-used queries. Most callers go through
the container, not individual records.

---

## 2. Error Types

### 2.1 AltiumFormatError

Strict error type. Every variant includes enough context to diagnose the problem.
No "soft" variants -- everything that reaches this type is a hard error.

```rust
#[derive(Debug, thiserror::Error)]
pub enum AltiumFormatError {
    // --- I/O errors ---
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CFB error: {0}")]
    Cfb(#[from] cfb::Error),

    // --- Parameter format errors (STRICT) ---
    #[error("missing required parameter '{key}' in {record_type}")]
    MissingParam {
        record_type: &'static str,
        key: String,
    },

    #[error("invalid parameter value for '{key}' in {record_type}: \
             expected {expected}, got '{value}'")]
    InvalidParamValue {
        record_type: &'static str,
        key: String,
        expected: &'static str,
        value: String,
    },

    #[error("unknown parameter '{key}' in {record_type} (value: '{value}')")]
    UnknownField {
        record_type: &'static str,
        key: String,
        value: String,
    },

    #[error("invalid enum discriminant {value} for type {type_name}")]
    InvalidEnumValue {
        type_name: &'static str,
        value: i32,
    },

    // --- Binary format errors (STRICT) ---
    #[error("binary parse error in {record_type} at offset {offset}: {message}")]
    BinaryParse {
        record_type: &'static str,
        offset: u64,
        message: String,
    },

    #[error("unexpected end of data in {record_type}: \
             expected {expected} bytes, got {actual}")]
    UnexpectedEof {
        record_type: &'static str,
        expected: usize,
        actual: usize,
    },

    #[error("unexpected trailing data in {record_type}: \
             {count} bytes remaining after parsing")]
    UnexpectedTrailingData {
        record_type: &'static str,
        count: usize,
    },

    #[error("field count mismatch in {record_type}: \
             expected {expected}, found {actual}")]
    FieldCountMismatch {
        record_type: &'static str,
        expected: usize,
        actual: usize,
    },

    // --- Record type dispatch errors ---
    #[error("unknown PCB object ID: {0}")]
    UnknownPcbObjectId(u8),

    #[error("unknown schematic record type: {0}")]
    UnknownSchRecordType(i32),

    // --- Block/container errors ---
    #[error("invalid block header: size={size}, flags=0x{flags:02x}")]
    InvalidBlockHeader { size: u32, flags: u8 },

    #[error("zlib decompression error: {0}")]
    DecompressionError(String),

    // --- Document structure errors ---
    #[error("missing required OLE stream: {path}")]
    MissingStream { path: String },

    #[error("invalid file header: expected '{expected}', got '{actual}'")]
    InvalidFileHeader {
        expected: &'static str,
        actual: String,
    },

    #[error("record count mismatch in section '{section}': \
             header says {expected}, found {actual}")]
    RecordCountMismatch {
        section: String,
        expected: u32,
        actual: u32,
    },

    #[error("owner index {index} out of bounds (max {max}) \
             in record at position {position}")]
    InvalidOwnerIndex {
        index: i32,
        max: usize,
        position: usize,
    },

    // --- Sidecar merge errors ---
    #[error("sidecar stream '{stream}' has {sidecar_count} entries \
             but expected {record_count}")]
    SidecarCountMismatch {
        stream: String,
        sidecar_count: usize,
        record_count: usize,
    },

    #[error("wide string sidecar references invalid primitive index {index}")]
    InvalidSidecarIndex { index: u32 },

    // --- Coordinate/value errors ---
    #[error("coordinate overflow: value {value} exceeds i32 range")]
    CoordOverflow { value: i64 },
}
```

**Key strict variants not in the old design:**
- `UnknownField`: fires when a parameter key isn't mapped to any struct field.
  This is the core enforcement mechanism -- our parser must know every key.
- `UnexpectedTrailingData`: fires when binary data has bytes left after parsing.
  Every byte must be accounted for.
- `FieldCountMismatch`: fires when the number of fields doesn't match expectations.

### 2.2 Result Type Alias

```rust
/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, AltiumFormatError>;
```

This is `pub` so the ops crate can handle errors from altium-format.

---

## 3. Validation Philosophy

### 3.1 Parse-Don't-Validate

If it parsed successfully, it's valid. There is no separate validation pass.

```
File bytes --> Parse (strict, may fail) --> Typed data (always valid)
                  |
                  v
              AltiumFormatError (if anything is wrong)
```

Invalid data is a parse error, not a warning. There is no `ValidationWarning` type.
There is no `validate()` method that returns a list of issues. If the parser accepts
data, that data is correct.

**Why not validate-on-use?**

| Approach | Pros | Cons |
|----------|------|------|
| Validate-on-use (warnings) | Loads "technically invalid" files | Invalid data can propagate; warnings may be ignored; fabrication risk |
| **Parse-don't-validate (chosen)** | Invalid states are unrepresentable; zero fabrication risk | Strict parser may reject files with minor quirks |

The old validate-on-use approach was motivated by round-trip requirements. Since we
no longer preserve unknown fields, there is no reason to accept data we don't understand.

### 3.2 Implications

- Every field in every record struct is fully typed and validated at parse time.
- Enum values are validated -- unknown discriminants are `InvalidEnumValue` errors.
- Coordinate values are validated -- overflow is a `CoordOverflow` error.
- String values are validated -- malformed UTF-8 or encoding errors are parse errors.
- Owner indices are validated -- out-of-bounds references are `InvalidOwnerIndex` errors.
- Sidecar record counts are validated -- mismatches are `SidecarCountMismatch` errors.

---

## 4. BinaryReader and BinaryWriter

Typed wrappers around byte slices for PCB binary parsing.

```rust
/// Cursor-based binary reader with position tracking and strict bounds checking.
pub(crate) struct BinaryReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BinaryReader<'a> {
    pub fn new(data: &'a [u8]) -> Self { Self { data, pos: 0 } }

    /// Current read position.
    pub fn position(&self) -> usize { self.pos }

    /// Bytes remaining to be read.
    pub fn remaining(&self) -> usize { self.data.len() - self.pos }

    /// Returns error if any bytes remain unread.
    /// Call this at the end of FromBinary to enforce strict parsing.
    pub fn assert_exhausted(&self, record_type: &'static str) -> Result<()> {
        if self.remaining() > 0 {
            Err(AltiumFormatError::UnexpectedTrailingData {
                record_type,
                count: self.remaining(),
            })
        } else {
            Ok(())
        }
    }

    pub fn read_u8(&mut self) -> Result<u8> { ... }
    pub fn read_i8(&mut self) -> Result<i8> { ... }
    pub fn read_u16_le(&mut self) -> Result<u16> { ... }
    pub fn read_i16_le(&mut self) -> Result<i16> { ... }
    pub fn read_u32_le(&mut self) -> Result<u32> { ... }
    pub fn read_i32_le(&mut self) -> Result<i32> { ... }
    pub fn read_u64_le(&mut self) -> Result<u64> { ... }
    pub fn read_i64_le(&mut self) -> Result<i64> { ... }
    pub fn read_f32_le(&mut self) -> Result<f32> { ... }
    pub fn read_f64_le(&mut self) -> Result<f64> { ... }
    pub fn read_bool(&mut self) -> Result<bool> { ... }

    /// Read a Coord (i32le, 10000 units/mil).
    pub fn read_coord(&mut self) -> Result<Coord> {
        Ok(Coord::new(self.read_i32_le()?))
    }

    /// Read a CoordPoint (two consecutive i32le).
    pub fn read_coord_point(&mut self) -> Result<CoordPoint> {
        Ok(CoordPoint {
            x: self.read_coord()?,
            y: self.read_coord()?,
        })
    }

    /// Read a length-prefixed string (i32le length + UTF-8 bytes).
    pub fn read_string_block(&mut self) -> Result<String> { ... }

    /// Read a Pascal-style string (u8 length + bytes).
    pub fn read_pascal_string(&mut self) -> Result<String> { ... }

    /// Read exactly N bytes.
    pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8]> { ... }

    /// Skip N bytes (for documented padding/reserved fields).
    pub fn skip(&mut self, n: usize) -> Result<()> { ... }
}

/// Binary writer with position tracking.
pub(crate) struct BinaryWriter<'a> {
    buf: &'a mut Vec<u8>,
}

impl<'a> BinaryWriter<'a> {
    pub fn new(buf: &'a mut Vec<u8>) -> Self { Self { buf } }

    pub fn write_u8(&mut self, v: u8) -> Result<()> { ... }
    pub fn write_i16_le(&mut self, v: i16) -> Result<()> { ... }
    pub fn write_i32_le(&mut self, v: i32) -> Result<()> { ... }
    pub fn write_f64_le(&mut self, v: f64) -> Result<()> { ... }
    pub fn write_bool(&mut self, v: bool) -> Result<()> { ... }
    pub fn write_coord(&mut self, v: Coord) -> Result<()> { ... }
    pub fn write_coord_point(&mut self, v: CoordPoint) -> Result<()> { ... }
    pub fn write_string_block(&mut self, s: &str) -> Result<()> { ... }
    pub fn write_pascal_string(&mut self, s: &str) -> Result<()> { ... }
    pub fn write_bytes(&mut self, data: &[u8]) -> Result<()> { ... }
    pub fn write_zeroes(&mut self, n: usize) -> Result<()> { ... }
}
```

---

## 5. ParameterCollection Type

```rust
/// Ordered collection of key=value string pairs parsed from Altium's
/// pipe-delimited text format.
///
/// Used for strict deserialization: after all known fields are consumed,
/// remaining keys are errors (not preserved for round-tripping).
pub(crate) struct ParameterCollection {
    params: IndexMap<String, String>,
}

impl ParameterCollection {
    pub fn new() -> Self { ... }

    /// Parse from pipe-delimited string: "|KEY1=VALUE1|KEY2=VALUE2|"
    /// Input is Windows-1252 encoded bytes.
    pub fn from_pipe_delimited(input: &[u8]) -> Result<Self> { ... }

    /// Serialize to pipe-delimited string.
    pub fn to_pipe_delimited(&self) -> Vec<u8> { ... }

    /// Get a parameter value, returning Err(MissingParam) if absent.
    pub fn get_required(&self, key: &str,
                        record_type: &'static str) -> Result<&str> { ... }

    /// Get a parameter value, returning None if absent.
    pub fn get(&self, key: &str) -> Option<&str> { ... }

    /// Get and REMOVE a typed value via FromParamValue.
    /// Removing consumed keys enables strict checking of leftovers.
    pub fn take_required<T: FromParamValue>(&mut self, key: &str,
                                             record_type: &'static str) -> Result<T> {
        let value = self.params.remove(key)
            .ok_or_else(|| AltiumFormatError::MissingParam {
                record_type,
                key: key.to_string(),
            })?;
        T::from_param_value(&value)
    }

    /// Get and REMOVE a typed value, using T::default() if absent.
    pub fn take_or_default<T: FromParamValue + Default>(
        &mut self, key: &str
    ) -> Result<T> {
        match self.params.remove(key) {
            Some(value) => T::from_param_value(&value),
            None => Ok(T::default()),
        }
    }

    /// Get and REMOVE an optional typed value.
    pub fn take_optional<T: FromParamValue>(
        &mut self, key: &str
    ) -> Result<Option<T>> {
        match self.params.remove(key) {
            Some(value) => Ok(Some(T::from_param_value(&value)?)),
            None => Ok(None),
        }
    }

    /// After all known keys have been consumed via take_*, assert
    /// that no unrecognized keys remain.
    /// This is the STRICT enforcement point.
    pub fn assert_empty(&self, record_type: &'static str) -> Result<()> {
        if let Some((key, value)) = self.params.iter().next() {
            Err(AltiumFormatError::UnknownField {
                record_type,
                key: key.clone(),
                value: value.clone(),
            })
        } else {
            Ok(())
        }
    }

    /// Set a parameter.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) { ... }

    /// Set a typed value via ToParamValue.
    pub fn set_typed<T: ToParamValue>(
        &mut self, key: impl Into<String>, value: &T
    ) { ... }

    /// Iterate all remaining parameters (useful for diagnostics).
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> { ... }

    /// Number of remaining (unconsumed) parameters.
    pub fn len(&self) -> usize { self.params.len() }
}
```

**The strict consumption pattern:**

```rust
// Generated by derive macro for each record type:
impl FromParams for SchWire {
    fn from_params(params: &ParameterCollection) -> Result<Self> {
        let mut params = params.clone(); // work on owned copy
        let base = SchGraphicalBase::from_params_consume(&mut params)?;
        let line_width = params.take_or_default::<PenWidth>("LINEWIDTH")?;
        let line_style = params.take_or_default::<LineStyle>("LINESTYLE")?;
        let vertices = params.take_indexed_coords("X", "Y", "LOCATIONCOUNT")?;
        // STRICT: reject any leftover keys
        params.assert_empty("SchWire")?;
        Ok(Self { base, line_width, line_style, vertices })
    }
}
```

---

## 6. Display Implementations

### 6.1 Coord Display

```rust
impl std::fmt::Display for Coord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mils = self.0 as f64 / 10_000.0;
        if mils == mils.trunc() {
            // Integer mil value -- no decimal places
            write!(f, "{}mil", mils as i64)
        } else {
            write!(f, "{:.4}mil", mils)
        }
    }
}

impl std::fmt::Display for CoordPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.r(), self.g(), self.b())
    }
}
```

### 6.2 V6Layer Display

```rust
impl std::fmt::Display for V6Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::NoLayer => f.write_str("NoLayer"),
            Self::TopLayer => f.write_str("TopLayer"),
            Self::BottomLayer => f.write_str("BottomLayer"),
            Self::TopOverlay => f.write_str("TopOverlay"),
            Self::BottomOverlay => f.write_str("BottomOverlay"),
            Self::TopPaste => f.write_str("TopPaste"),
            Self::BottomPaste => f.write_str("BottomPaste"),
            Self::TopSolder => f.write_str("TopSolder"),
            Self::BottomSolder => f.write_str("BottomSolder"),
            Self::MultiLayer => f.write_str("MultiLayer"),
            Self::DrillGuide => f.write_str("DrillGuide"),
            Self::DrillDrawing => f.write_str("DrillDrawing"),
            Self::KeepOutLayer => f.write_str("KeepOutLayer"),
            Self::ConnectLayer => f.write_str("ConnectLayer"),
            layer if layer.is_mechanical() => {
                write!(f, "Mechanical{}", layer.mechanical_number().unwrap())
            }
            layer if layer.is_internal_plane() => {
                write!(f, "InternalPlane{}", layer.internal_plane_number().unwrap())
            }
            layer => {
                let n = *self as u8;
                if (2..=31).contains(&n) {
                    write!(f, "MidLayer{}", n - 1)
                } else {
                    write!(f, "Layer({})", n)
                }
            }
        }
    }
}
```

### 6.3 Enum Display

All enums get `Display` via the `AltiumEnum` derive, showing human-readable names:

```rust
impl std::fmt::Display for PinElectricalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input => f.write_str("Input"),
            Self::InputOutput => f.write_str("I/O"),
            Self::Output => f.write_str("Output"),
            Self::OpenCollector => f.write_str("Open Collector"),
            Self::Passive => f.write_str("Passive"),
            Self::HiZ => f.write_str("Hi-Z"),
            Self::OpenEmitter => f.write_str("Open Emitter"),
            Self::Power => f.write_str("Power"),
        }
    }
}
```

No `Display` for record structs -- they have too many fields. `Debug` is sufficient for
development. The ops crate handles user-facing formatting.

---

## 7. Coord Full Implementation

```rust
impl Coord {
    pub const ZERO: Self = Self(0);
    pub const UNITS_PER_MIL: i32 = 10_000;
    pub const UNITS_PER_INCH: i32 = 10_000_000;

    /// Create from raw internal units.
    pub const fn new(raw: i32) -> Self { Self(raw) }

    /// Get raw internal units value.
    pub const fn raw(self) -> i32 { self.0 }

    /// Create from integer mils. Panics on overflow.
    pub fn from_mils(mils: i32) -> Self {
        Self(mils.checked_mul(Self::UNITS_PER_MIL)
            .expect("coordinate overflow in from_mils"))
    }

    /// Create from fractional mils.
    pub fn from_mils_f64(mils: f64) -> Self {
        Self((mils * Self::UNITS_PER_MIL as f64).round() as i32)
    }

    /// Create from millimeters.
    pub fn from_mm(mm: f64) -> Self {
        // 1 mm = 1/25.4 inch = 10_000_000/25.4 units = 393_700.787... units
        Self((mm * 393_700.787_401_575).round() as i32)
    }

    /// Convert to mils (fractional).
    pub fn to_mils(self) -> f64 {
        self.0 as f64 / Self::UNITS_PER_MIL as f64
    }

    /// Convert to millimeters.
    pub fn to_mm(self) -> f64 {
        self.0 as f64 * 0.000_002_54
    }

    /// Convert to inches.
    pub fn to_inches(self) -> f64 {
        self.0 as f64 / Self::UNITS_PER_INCH as f64
    }

    /// Reconstruct from DXP fractional encoding (schematic parameter format).
    /// raw = integer_part * 100_000 + frac_part
    /// Source: Rt_Schematic.Consts.cBaseUnit = 100000 (each DXP unit = 10 mils).
    pub fn from_dxp_frac(integer: i32, frac: i32) -> Self {
        Self(integer.wrapping_mul(Self::DXP_BASE_UNIT).wrapping_add(frac))
    }

    /// Split into DXP fractional encoding for serialization.
    /// Returns (integer_part, frac_part) where frac_part is in 0..99999.
    pub fn to_dxp_frac(self) -> (i32, i32) {
        let integer = self.0.div_euclid(Self::DXP_BASE_UNIT);
        let frac = self.0.rem_euclid(Self::DXP_BASE_UNIT);
        (integer, frac)
    }

    /// Absolute value.
    pub fn abs(self) -> Self { Self(self.0.abs()) }

    /// Minimum of two coordinates.
    pub fn min(self, other: Self) -> Self { Self(self.0.min(other.0)) }

    /// Maximum of two coordinates.
    pub fn max(self, other: Self) -> Self { Self(self.0.max(other.0)) }
}

// Arithmetic operations
impl std::ops::Add for Coord {
    type Output = Self;
    fn add(self, rhs: Self) -> Self { Self(self.0 + rhs.0) }
}

impl std::ops::Sub for Coord {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self { Self(self.0 - rhs.0) }
}

impl std::ops::Neg for Coord {
    type Output = Self;
    fn neg(self) -> Self { Self(-self.0) }
}

impl std::ops::Mul<i32> for Coord {
    type Output = Self;
    fn mul(self, rhs: i32) -> Self { Self(self.0 * rhs) }
}

impl std::ops::Div<i32> for Coord {
    type Output = Self;
    fn div(self, rhs: i32) -> Self { Self(self.0 / rhs) }
}

impl std::ops::AddAssign for Coord {
    fn add_assign(&mut self, rhs: Self) { self.0 += rhs.0; }
}

impl std::ops::SubAssign for Coord {
    fn sub_assign(&mut self, rhs: Self) { self.0 -= rhs.0; }
}
```

---

## 8. BoundingBox Implementation

```rust
impl BoundingBox {
    /// Create from two points (automatically normalizes to min/max).
    pub fn from_points(a: CoordPoint, b: CoordPoint) -> Self {
        Self {
            min: CoordPoint {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            },
            max: CoordPoint {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            },
        }
    }

    /// Create a bounding box enclosing a single point.
    pub fn from_point(p: CoordPoint) -> Self {
        Self { min: p, max: p }
    }

    /// Create from an iterator of points.
    pub fn from_iter(points: impl IntoIterator<Item = CoordPoint>) -> Option<Self> {
        let mut iter = points.into_iter();
        let first = iter.next()?;
        let mut bbox = Self::from_point(first);
        for p in iter {
            bbox = bbox.union_point(p);
        }
        Some(bbox)
    }

    pub fn min(&self) -> CoordPoint { self.min }
    pub fn max(&self) -> CoordPoint { self.max }
    pub fn width(&self) -> Coord { self.max.x - self.min.x }
    pub fn height(&self) -> Coord { self.max.y - self.min.y }

    pub fn center(&self) -> CoordPoint {
        CoordPoint {
            x: Coord::new((self.min.x.raw() + self.max.x.raw()) / 2),
            y: Coord::new((self.min.y.raw() + self.max.y.raw()) / 2),
        }
    }

    /// Expand this bounding box to include a point.
    pub fn union_point(self, p: CoordPoint) -> Self {
        Self {
            min: CoordPoint {
                x: self.min.x.min(p.x),
                y: self.min.y.min(p.y),
            },
            max: CoordPoint {
                x: self.max.x.max(p.x),
                y: self.max.y.max(p.y),
            },
        }
    }

    /// Union of two bounding boxes.
    pub fn union(self, other: Self) -> Self {
        self.union_point(other.min).union_point(other.max)
    }

    /// Test if a point is within this bounding box.
    pub fn contains(&self, p: CoordPoint) -> bool {
        p.x >= self.min.x && p.x <= self.max.x &&
        p.y >= self.min.y && p.y <= self.max.y
    }

    /// Expand by a margin on all sides.
    pub fn expand(self, margin: Coord) -> Self {
        Self {
            min: CoordPoint {
                x: self.min.x - margin,
                y: self.min.y - margin,
            },
            max: CoordPoint {
                x: self.max.x + margin,
                y: self.max.y + margin,
            },
        }
    }
}
```

---

## 9. Color Implementation

```rust
impl Color {
    pub const BLACK: Self = Self(0x00000000);
    pub const WHITE: Self = Self(0x00FFFFFF);
    pub const RED: Self = Self(0x000000FF);
    pub const GREEN: Self = Self(0x0000FF00);
    pub const BLUE: Self = Self(0x00FF0000);

    pub const fn new(colorref: i32) -> Self { Self(colorref) }
    pub const fn raw(self) -> i32 { self.0 }

    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self((b as i32) << 16 | (g as i32) << 8 | (r as i32))
    }

    pub fn r(self) -> u8 { (self.0 & 0xFF) as u8 }
    pub fn g(self) -> u8 { ((self.0 >> 8) & 0xFF) as u8 }
    pub fn b(self) -> u8 { ((self.0 >> 16) & 0xFF) as u8 }
}
```

---

## 10. Iterators and Document Access

### 10.1 SchDoc Iterators

```rust
impl SchDoc {
    /// The sheet header record (always present, always the first record).
    pub fn sheet(&self) -> &SchSheet { &self.header }

    /// Total number of records.
    pub fn record_count(&self) -> usize { self.records.len() }

    /// Iterate all records.
    pub fn records(&self) -> impl Iterator<Item = &SchRecord> {
        self.records.iter()
    }

    /// Iterate records of a specific type.
    pub fn components(&self) -> impl Iterator<Item = &SchComponent> {
        self.records.iter().filter_map(|r| match r {
            SchRecord::Component(c) => Some(c),
            _ => None,
        })
    }

    pub fn pins(&self) -> impl Iterator<Item = &SchPin> { ... }
    pub fn wires(&self) -> impl Iterator<Item = &SchWire> { ... }
    pub fn net_labels(&self) -> impl Iterator<Item = &SchNetLabel> { ... }
    pub fn parameters(&self) -> impl Iterator<Item = &SchParameter> { ... }
    pub fn power_objects(&self) -> impl Iterator<Item = &SchPowerObject> { ... }
    pub fn ports(&self) -> impl Iterator<Item = &SchPort> { ... }
    pub fn rectangles(&self) -> impl Iterator<Item = &SchRectangle> { ... }
    // ... one per commonly-queried type

    /// Iterate children of a specific record (by parent's index in record list).
    pub fn children_of(&self, parent_index: usize)
        -> impl Iterator<Item = (usize, &SchRecord)>
    {
        self.records.iter().enumerate().filter(move |(_, r)| {
            // match on owner_index via SchPrimitive trait dispatch
            self.owner_index_of(r) == parent_index as i32
        })
    }

    /// Iterate top-level records (owner_index == -1 or absent).
    pub fn top_level(&self) -> impl Iterator<Item = (usize, &SchRecord)> {
        self.records.iter().enumerate().filter(|(_, r)| {
            self.owner_index_of(r) == -1
        })
    }
}
```

### 10.2 PcbDoc Iterators

```rust
impl PcbDoc {
    /// Board-level metadata.
    pub fn board(&self) -> &PcbBoard { &self.board }

    /// Board outline.
    pub fn board_outline(&self) -> &PcbBoardOutline { &self.board_outline }

    // Per-type iterators (zero-cost: just slice iteration)
    pub fn arcs(&self) -> &[PcbArc] { &self.arcs }
    pub fn pads(&self) -> &[PcbPad] { &self.pads }
    pub fn vias(&self) -> &[PcbVia] { &self.vias }
    pub fn tracks(&self) -> &[PcbTrack] { &self.tracks }
    pub fn texts(&self) -> &[PcbText] { &self.texts }
    pub fn fills(&self) -> &[PcbFill] { &self.fills }
    pub fn components(&self) -> &[PcbComponent] { &self.components }
    pub fn regions(&self) -> &[PcbRegion] { &self.regions }
    pub fn component_bodies(&self) -> &[PcbComponentBody] { &self.component_bodies }
    pub fn polygons(&self) -> &[PcbPolygon] { &self.polygons }
    pub fn dimensions(&self) -> &[PcbDimension] { &self.dimensions }
    pub fn nets(&self) -> &[PcbNet] { &self.nets }
    pub fn rules(&self) -> &[PcbRule] { &self.rules }
    pub fn classes(&self) -> &[PcbClass] { &self.classes }
    pub fn models(&self) -> &[PcbModel] { &self.models }

    /// Lookup a net by index. Returns Err if index is out of bounds.
    pub fn net_name(&self, index: i16) -> Result<&str> { ... }

    /// Lookup a component by index. Returns Err if index is out of bounds.
    pub fn component_by_index(&self, index: i16) -> Result<&PcbComponent> { ... }

    /// All primitives on a given layer (cross-type iteration).
    pub fn on_layer(&self, layer: V6Layer) -> impl Iterator<Item = PcbPrimitiveRef<'_>> {
        // Chain iterators over all per-type vecs, filtering by layer
        ...
    }

    /// All primitives belonging to a given net.
    pub fn in_net(&self, net_index: i16) -> impl Iterator<Item = PcbPrimitiveRef<'_>> {
        ...
    }

    /// All primitives belonging to a given component.
    pub fn in_component(&self, component_index: i16)
        -> impl Iterator<Item = PcbPrimitiveRef<'_>>
    {
        ...
    }
}
```

### 10.3 PcbPrimitiveRef -- Lightweight Enum for Cross-Type Iteration

```rust
/// Borrowed reference to any PCB primitive. Used for cross-type queries
/// (e.g., "all primitives on this layer" across arcs, pads, tracks, etc.)
pub enum PcbPrimitiveRef<'a> {
    Arc(&'a PcbArc),
    Pad(&'a PcbPad),
    Via(&'a PcbVia),
    Track(&'a PcbTrack),
    Text(&'a PcbText),
    Fill(&'a PcbFill),
    Region(&'a PcbRegion),
    ComponentBody(&'a PcbComponentBody),
    Dimension(&'a PcbDimension),
}

impl<'a> PcbPrimitiveRef<'a> {
    pub fn layer(&self) -> V6Layer { ... }
    pub fn net(&self) -> i16 { ... }
    pub fn component(&self) -> i16 { ... }
    pub fn bounding_box(&self) -> BoundingBox { ... }
    pub fn object_id(&self) -> PcbObjectId { ... }
}
```

### 10.4 SchLib Iteration

```rust
impl SchLib {
    /// Number of components in the library.
    pub fn component_count(&self) -> usize { self.components.len() }

    /// Iterate all components.
    pub fn components(&self) -> impl Iterator<Item = &SchLibComponent> {
        self.components.iter()
    }

    /// Find a component by name (case-insensitive).
    pub fn find_component(&self, name: &str) -> Option<&SchLibComponent> {
        self.components.iter().find(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// Find a component by name or alias (case-insensitive).
    pub fn find_component_or_alias(&self, name: &str) -> Option<&SchLibComponent> {
        self.components.iter().find(|c| {
            c.name.eq_ignore_ascii_case(name) ||
            c.aliases.iter().any(|a| a.eq_ignore_ascii_case(name))
        })
    }
}

impl SchLibComponent {
    pub fn name(&self) -> &str { &self.name }
    pub fn description(&self) -> &str { &self.description }
    pub fn part_count(&self) -> i32 { self.part_count }
    pub fn aliases(&self) -> &[String] { &self.aliases }

    pub fn records(&self) -> impl Iterator<Item = &SchRecord> {
        self.records.iter()
    }

    pub fn pins(&self) -> impl Iterator<Item = &SchPin> {
        self.records.iter().filter_map(|r| match r {
            SchRecord::Pin(p) => Some(p),
            _ => None,
        })
    }
}
```

### 10.5 PcbLib Iteration

```rust
impl PcbLib {
    pub fn footprint_count(&self) -> usize { self.footprints.len() }

    pub fn footprints(&self) -> impl Iterator<Item = &PcbLibFootprint> {
        self.footprints.iter()
    }

    pub fn find_footprint(&self, pattern: &str) -> Option<&PcbLibFootprint> {
        self.footprints.iter().find(|f| f.pattern.eq_ignore_ascii_case(pattern))
    }
}

impl PcbLibFootprint {
    pub fn pattern(&self) -> &str { &self.pattern }
    pub fn height(&self) -> Coord { self.height }
    pub fn description(&self) -> &str { &self.description }

    pub fn primitives(&self) -> impl Iterator<Item = &PcbRecord> {
        self.primitives.iter()
    }

    pub fn pads(&self) -> impl Iterator<Item = &PcbPad> {
        self.primitives.iter().filter_map(|r| match r {
            PcbRecord::Pad(p) => Some(p),
            _ => None,
        })
    }
}
```

---

## 11. Document Loading Pipeline

### 11.1 SchDoc Loading

```rust
impl SchDoc {
    /// Load a schematic document from a file path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> { ... }

    /// Load from an already-opened CFB compound file.
    pub fn from_cfb(cfb: &mut CompoundFile<impl Read + Seek>) -> Result<Self> { ... }
}

// Internal pipeline (all steps return Result, propagating errors immediately):
// 1. Open CFB container
// 2. Read /FileHeader stream
// 3. Parse block 0 (header: HEADER, WEIGHT -- validate header string)
// 4. Parse blocks 1..N as SchRecord (dispatching on RECORD=N)
//    - Each block: read size-prefixed payload
//    - Decode Windows-1252 to string
//    - Parse as ParameterCollection
//    - Match RECORD value to SchRecordType (error on unknown)
//    - Deserialize via FromParams (strict: rejects unknown keys)
// 5. Read /Additional stream (supplementary parameters) -- same parsing
// 6. Merge sidecar streams (count validated against records):
//    a. WideStrings -- replace truncated ASCII text with full Unicode
//    b. UniqueIDs -- assign unique IDs to primitives
//    c. ExtendedPrimitiveInfo -- merge mask expansion overrides
//    d. PinFrac -- merge fractional pin coordinates
//    e. PinWideText -- merge Unicode pin names/designators
// 7. Validate owner index references (all must be in bounds)
// 8. Return SchDoc
```

### 11.2 PcbDoc Loading

```rust
impl PcbDoc {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> { ... }
    pub fn from_cfb(cfb: &mut CompoundFile<impl Read + Seek>) -> Result<Self> { ... }
}

// Internal pipeline:
// 1. Open CFB container
// 2. Read /Board6/Data (board properties, text format, strict parse)
// 3. For each binary primitive section (Arcs6, Pads6, Vias6, Tracks6, ...):
//    a. Read Header (u32 record count)
//    b. Read Data ([u8 type_id][u32 length][payload] per record)
//    c. Validate type_id against expected PcbObjectId (error on mismatch)
//    d. Deserialize via FromBinary (strict: assert_exhausted at end)
//    e. Validate record count matches header
// 4. Read text-format sections (strict parameter parsing):
//    a. Components6 -> Vec<PcbComponent>
//    b. Nets6 -> Vec<PcbNet>
//    c. Polygons6 -> Vec<PcbPolygon>
//    d. Rules6 -> Vec<PcbRule>
//    e. Classes6 -> Vec<PcbClass>
//    f. Models -> Vec<PcbModel>
// 5. Merge sidecar streams (count validated):
//    a. WideStrings6 (binary TLV: [u32 index][u32 len][UTF-16LE])
//    b. UniqueIDPrimitiveInformation (parameter blocks)
//    c. ExtendedPrimitiveInformation (mask expansion modes)
// 6. Validate cross-references:
//    a. All net indices in primitives reference valid nets
//    b. All component indices reference valid components
// 7. Return PcbDoc
```

### 11.3 Saving

```rust
impl SchDoc {
    /// Write the document to a file path.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> { ... }

    /// Write to an already-opened CFB compound file.
    pub fn to_cfb(&self, cfb: &mut CompoundFile<impl Read + Write + Seek>)
        -> Result<()> { ... }
}

impl PcbDoc {
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> { ... }
    pub fn to_cfb(&self, cfb: &mut CompoundFile<impl Read + Write + Seek>)
        -> Result<()> { ... }
}
```

The save pipeline is the mirror of loading:
1. Serialize records via `ToParams` / `ToBinary`
2. Write size-prefixed blocks to appropriate streams
3. Generate sidecar streams from record data
4. Write to CFB container

Since we have no unknown fields to replay, serialization only writes fields we know about.
The output is a "clean" representation of the data model.

---

## 12. Derive Macro Attributes

### 12.1 AltiumRecord

```rust
/// Generates FromParams + ToParams and/or FromBinary + ToBinary implementations.
/// The generated FromParams STRICTLY rejects unknown parameters.
/// The generated FromBinary STRICTLY rejects trailing bytes.
///
/// Struct-level attributes:
///   #[altium(params)]          -- generate param serialization
///   #[altium(binary)]          -- generate binary serialization
///   #[altium(params, binary)]  -- generate both
///
/// Field attributes for parameter format:
///   #[altium(param = "KEY")]                    -- map to parameter key
///   #[altium(param = "KEY", default)]           -- use Default::default() if missing
///   #[altium(param = "KEY", frac = "KEY_FRAC")] -- DXP fractional coord
///   #[altium(indexed_coords, prefix_x = "X", prefix_y = "Y", count = "N")]
///   #[altium(flatten)]                          -- compose base type
///   #[altium(color)]                            -- Win32 COLORREF
///   #[altium(list)]                             -- comma-separated values
///   #[altium(skip)]                             -- skip during serialization
///
/// Field attributes for binary format:
///   #[altium(binary, ty = "i32le")]             -- basic binary type
///   #[altium(coord_point)]                      -- two i32le as CoordPoint
///   #[altium(coord)]                            -- single i32le as Coord
///   #[altium(string_block)]                     -- i32 length + UTF-8
///   #[altium(pascal_string)]                    -- u8 length + bytes
///   #[altium(array = N)]                        -- fixed-size array
///   #[altium(skip_bytes = N)]                   -- skip N documented padding bytes
```

**Removed attributes (from old design):**
- `#[altium(unknown)]` -- no unknown field capture
- `#[altium(unknown_binary)]` -- no trailing byte capture
- `#[altium(optional)]` -- use concrete types, not Option (unless genuinely optional in Altium)

### 12.2 AltiumEnum

```rust
/// Generates bidirectional integer-to-enum conversion.
/// STRICT: unknown discriminant values produce AltiumFormatError::InvalidEnumValue.
/// No default fallback for unknown values (unlike old design).
///
///   #[derive(AltiumEnum)]
///   #[repr(u8)]
///   pub enum PadShape {
///       #[default]
///       Round = 1,
///       Rectangular = 2,
///       ...
///   }
///
/// Generates:
///   impl TryFrom<u8> for PadShape {
///       type Error = AltiumFormatError;
///       // Returns InvalidEnumValue on unknown discriminant
///   }
///   impl From<PadShape> for u8 { ... }
///   impl FromParamValue for PadShape { ... }
///   impl ToParamValue for PadShape { ... }
```

**Key change**: `TryFrom` instead of `From`. Unknown values are errors, not
silently mapped to a default.

### 12.3 AltiumBase

```rust
/// Generates a composition trait for base types.
///
///   #[altium_base]
///   pub(crate) struct SchPrimitiveBase { ... }
///
/// Generates:
///   pub(crate) trait HasSchPrimitiveBase {
///       fn sch_primitive_base(&self) -> &SchPrimitiveBase;
///       fn sch_primitive_base_mut(&mut self) -> &mut SchPrimitiveBase;
///   }
///
/// Also generates a from_params_consume method that takes a &mut ParameterCollection
/// and REMOVES consumed keys (enabling strict leftover checking).
```

---

## 13. Module Organization

```
crates/altium-format/src/
    lib.rs                  -- pub types: AltiumFormatError, Result, Coord, Color, etc.
    coord.rs                -- Coord, CoordPoint, BoundingBox implementations
    color.rs                -- Color implementation
    unique_id.rs            -- UniqueId generation and validation
    binary.rs               -- BinaryReader, BinaryWriter
    params.rs               -- ParameterCollection, FromParamValue, ToParamValue
    block.rs                -- Block framing: read/write size-prefixed blocks
    cfb_util.rs             -- CFB helper functions
    error.rs                -- AltiumFormatError, Result
    sch/
        mod.rs              -- SchDoc, SchLib public types
        record.rs           -- SchRecord enum, SchRecordType
        types.rs            -- All SchXxx record structs
        base.rs             -- SchPrimitiveBase, SchGraphicalBase
        enums.rs            -- Schematic-specific enums
        parse.rs            -- SchDoc/SchLib loading pipeline
        write.rs            -- SchDoc/SchLib saving pipeline
        sidecar.rs          -- Sidecar stream merge (PinFrac, WideStrings, etc.)
    pcb/
        mod.rs              -- PcbDoc, PcbLib public types
        record.rs           -- PcbRecord enum, PcbObjectId
        types.rs            -- All PcbXxx record structs
        common.rs           -- PcbPrimitiveCommon, PcbFlags
        layer.rs            -- V6Layer, V7Layer
        enums.rs            -- PCB-specific enums
        parse.rs            -- PcbDoc/PcbLib loading pipeline
        write.rs            -- PcbDoc/PcbLib saving pipeline
        sidecar.rs          -- Sidecar stream merge (WideStrings6, UniqueIDs, etc.)
```

This organization:
- Separates SCH and PCB domains into sub-modules
- Keeps shared types at the crate root
- Matches the natural file format separation
- All `pub(crate)` internals are accessible within the crate but hidden from dependents
- No cross-domain dependencies between `sch/` and `pcb/` (they share only root types)

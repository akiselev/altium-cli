//! SchLib file I/O — ported from `SchDataImporterLibraryV5.cs` / `SchDataExporterLibraryV5.cs`.
//!
//! # CFB Storage Hierarchy
//!
//! ```text
//! Root
//! ├── FileHeader          (component list + metadata)
//! ├── SectionKeys         (maps LibRef names → CFB section paths)
//! └── {ComponentSectionKey}/
//!     ├── Data            (component + child records)
//!     ├── PinFrac         (fractional pin coordinates)
//!     ├── PinDesc         (long pin descriptions)
//!     ├── PinWideText     (wide-char pin text)
//!     ├── PinMiscData     (swap IDs)
//!     ├── PinTextData     (custom text display)
//!     ├── PinSymbolLineWidth
//!     ├── PinPackageLength
//!     ├── PinPropagationDelay
//!     └── PinFunctionData
//! ```
//!
//! # Data Stream Format
//!
//! The Data stream is a sequence of serialized records:
//! 1. `RECORD(0)` header marker
//! 2. Component record (RECORD or BINARY instruction)
//! 3. Child records (pins, parameters, etc.)
//! 4. `RECORD(0)` end marker

use serde::{Deserialize, Serialize};
use std::io::{Read, Write, Cursor, Seek};

use crate::error::{AltiumError, Result};
use crate::v2::consts;
use crate::v2::fields::{
    TypedRecord, PinData, ComponentData, ParameterData, RectangleData, LineData,
    ArcData, EllipseData, PolygonData, PolylineData, BezierData, EllipticalArcData,
    PieData, RoundRectangleData, ImageData, DesignatorData, LabelData, SymbolData,
    ImplementationData, ImplementationListData,
};
use crate::v2::io::section_keys::SectionKeyList;
use crate::v2::serializer::SchSerializer;
use crate::v2::serializer::ascii::AsciiSerializer;
use crate::v2::serializer::format_v5::{
    import_pin, import_component, import_parameter, import_rectangle, import_line,
    import_arc, import_ellipse, import_polygon, import_polyline, import_bezier,
    import_elliptical_arc, import_pie, import_round_rectangle, import_image,
    import_designator, import_label, import_symbol, import_implementation,
    import_implementation_list,
};

/// A parsed SchLib library.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SchLibV2 {
    /// Library header info.
    pub header: SchLibHeader,
    /// Components in the library.
    pub components: Vec<SchLibComponent>,
    /// Section key mappings (rebuilt on write, not needed for JSON).
    #[serde(skip)]
    pub section_keys: SectionKeyList,
}

/// SchLib file header.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SchLibHeader {
    pub header_text: String,
    pub weight: i32,
    pub minor_version: i32,
    pub unique_id: String,
    /// Raw FileHeader bytes for lossless roundtrip.
    /// When present, `write()` uses these directly instead of rebuilding.
    #[serde(skip)]
    pub raw: Option<Vec<u8>>,
}

/// A component entry from the FileHeader.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SchLibComponentEntry {
    pub lib_ref: String,
    pub description: String,
    pub part_count: i16,
    pub aliases: Vec<String>,
}

/// A complete component with its records.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SchLibComponent {
    /// Component entry from FileHeader.
    pub entry: SchLibComponentEntry,
    /// Raw serialized records (record_id, param_string pairs).
    /// Each record is stored as an ASCII parameter string.
    pub records: Vec<SchLibRecord>,
    /// Typed/parsed records for strongly-typed access.
    /// Populated during `open()` by parsing raw records via `import_*` functions.
    #[serde(skip)]
    pub typed_records: Vec<TypedRecord>,
}

/// A single record within a component.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchLibRecord {
    /// Object ID / record type.
    pub record_id: u8,
    /// Extended record ID (when record_id == 254).
    pub record_id_ex: Option<i32>,
    /// Decoded parameter string (lossy for binary records).
    pub params: String,
    /// Raw record bytes for lossless roundtrip.
    #[serde(skip)]
    pub raw: Vec<u8>,
}

impl SchLibV2 {
    /// Read a SchLib from a CFB compound file.
    pub fn open<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut cfb = cfb::CompoundFile::open(reader)
            .map_err(|e| AltiumError::Parse(format!("Failed to open CFB: {}", e)))?;

        let mut lib = SchLibV2::default();

        // 1. Read FileHeader
        lib.header = read_file_header(&mut cfb, &mut lib.components)?;

        // 2. Load SectionKeys
        lib.section_keys = read_section_keys(&mut cfb)?;

        // 3. Read Data streams for each component
        for i in 0..lib.components.len() {
            let safe_name = sanitize_cfb_name(&lib.components[i].entry.lib_ref);
            let section_key = lib.section_keys.get_key(&safe_name).to_string();
            let data_path = format!("/{}/{}", section_key, consts::STREAM_DATA);

            if let Ok(mut stream) = cfb.open_stream(&data_path) {
                let mut data = Vec::new();
                stream.read_to_end(&mut data)
                    .map_err(AltiumError::Io)?;
                lib.components[i].records = parse_data_stream(&data)?;
                // Parse raw records into typed records for strongly-typed access
                lib.components[i].typed_records = parse_typed_records(&lib.components[i].records);
            }
        }

        Ok(lib)
    }

    /// Write a SchLib to a CFB compound file.
    pub fn write<W: Read + Write + Seek>(&self, writer: W) -> Result<()> {
        let mut cfb = cfb::CompoundFile::create(writer)
            .map_err(|e| AltiumError::Parse(format!("Failed to create CFB: {}", e)))?;

        // 1. Build section keys (using sanitized names for CFB paths)
        let mut section_keys = SectionKeyList::new();
        for comp in &self.components {
            let safe = sanitize_cfb_name(&comp.entry.lib_ref);
            section_keys.add_key(&safe, 30);
            for alias in &comp.entry.aliases {
                let safe_alias = sanitize_cfb_name(alias);
                section_keys.add_key(&safe_alias, 30);
            }
        }

        // 2. Write FileHeader (use raw bytes for lossless roundtrip if available)
        if let Some(raw) = &self.header.raw {
            let mut stream = cfb.create_stream("/FileHeader")
                .map_err(|e| AltiumError::Parse(format!("Failed to create FileHeader: {}", e)))?;
            stream.write_all(raw).map_err(AltiumError::Io)?;
        } else {
            write_file_header(&mut cfb, &self.header, &self.components)?;
        }

        // 3. Write SectionKeys
        write_section_keys(&mut cfb, &section_keys)?;

        // 4. Write Data stream for each component
        for comp in &self.components {
            let safe_name = sanitize_cfb_name(&comp.entry.lib_ref);
            let section_key = section_keys.get_key(&safe_name).to_string();

            // Create storage for this component
            let storage_path = format!("/{}", section_key);
            cfb.create_storage(&storage_path)
                .map_err(|e| AltiumError::Parse(format!("Failed to create storage '{}': {}", storage_path, e)))?;

            // Write Data stream
            let data_path = format!("/{}/{}", section_key, consts::STREAM_DATA);
            let data = build_data_stream(&comp.records)?;
            let mut stream = cfb.create_stream(&data_path)
                .map_err(|e| AltiumError::Parse(format!("Failed to create stream: {}", e)))?;
            stream.write_all(&data).map_err(AltiumError::Io)?;

            // Write alias redirections
            for alias in &comp.entry.aliases {
                let safe_alias = sanitize_cfb_name(alias);
                let alias_key = section_keys.get_key(&safe_alias).to_string();
                let alias_storage = format!("/{}", alias_key);
                cfb.create_storage(&alias_storage)
                    .map_err(|e| AltiumError::Parse(format!("Failed to create alias storage: {}", e)))?;

                let redir_path = format!("/{}/Redirection", alias_key);
                let mut redir_ser = AsciiSerializer::new_writer();
                redir_ser.export_instruction(0, "RECORD")?;
                redir_ser.export_dynamic_string(&comp.entry.lib_ref, "SectionName")?;
                let redir_params = redir_ser.to_param_string();

                let mut redir_stream = cfb.create_stream(&redir_path)
                    .map_err(|e| AltiumError::Parse(format!("Failed to create redirection: {}", e)))?;
                redir_stream.write_all(redir_params.as_bytes()).map_err(AltiumError::Io)?;
            }
        }

        cfb.flush().map_err(|e| AltiumError::Parse(format!("CFB flush error: {}", e)))?;
        Ok(())
    }

    /// Get a component by LibRef name.
    pub fn get_component(&self, lib_ref: &str) -> Option<&SchLibComponent> {
        self.components.iter().find(|c| c.entry.lib_ref == lib_ref)
    }

    /// Get all component names.
    pub fn component_names(&self) -> Vec<&str> {
        self.components.iter().map(|c| c.entry.lib_ref.as_str()).collect()
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Sanitize a component name for use as a CFB storage name.
/// Altium replaces `/` with `_` since `/` is a path separator in CFB.
fn sanitize_cfb_name(name: &str) -> String {
    name.replace('/', "_")
}

// ============================================================================
// FileHeader read/write
// ============================================================================

fn read_file_header<F: Read + Seek>(
    cfb: &mut cfb::CompoundFile<F>,
    components: &mut Vec<SchLibComponent>,
) -> Result<SchLibHeader> {
    let mut stream = cfb.open_stream("/FileHeader")
        .map_err(|e| AltiumError::Parse(format!("No FileHeader stream: {}", e)))?;

    let mut data = Vec::new();
    stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
    let param_str = String::from_utf8_lossy(&data);

    let mut ser = AsciiSerializer::from_params(&param_str);

    let header = SchLibHeader {
        header_text: ser.import_dynamic_string("HEADER")?,
        weight: ser.import_long_int("Weight")?,
        minor_version: ser.import_long_int("MinorVersion")?,
        unique_id: ser.import_string("UniqueID")?,
        raw: Some(data.clone()),
    };

    let comp_count = ser.import_long_int("CompCount")?;
    for i in 0..comp_count {
        let lib_ref = ser.import_dynamic_string(&format!("LibRef{}", i))?;
        let description = ser.import_string(&format!("CompDescr{}", i))?;
        let part_count = ser.import_short_int(&format!("PartCount{}", i))? as i16;
        let alias_count = ser.import_short_int(&format!("AliasCount{}", i))?;

        let mut aliases = Vec::new();
        for j in 0..alias_count {
            let alias = ser.import_dynamic_string(&format!("Comp{}Alias{}", i, j))?;
            aliases.push(alias);
        }

        components.push(SchLibComponent {
            entry: SchLibComponentEntry {
                lib_ref,
                description,
                part_count,
                aliases,
            },
            records: Vec::new(),
            typed_records: Vec::new(),
        });
    }

    Ok(header)
}

fn write_file_header<F: Read + Write + Seek>(
    cfb: &mut cfb::CompoundFile<F>,
    header: &SchLibHeader,
    components: &[SchLibComponent],
) -> Result<()> {
    let mut ser = AsciiSerializer::new_writer();

    ser.export_instruction(0, "RECORD")?;
    ser.export_dynamic_string(&header.header_text, "HEADER")?;
    ser.export_long_int(header.weight, "Weight")?;
    ser.export_long_int(header.minor_version, "MinorVersion")?;
    ser.export_string(&header.unique_id, "UniqueID")?;
    ser.export_long_int(components.len() as i32, "CompCount")?;

    for (i, comp) in components.iter().enumerate() {
        ser.export_dynamic_string(&comp.entry.lib_ref, &format!("LibRef{}", i))?;
        ser.export_string(&comp.entry.description, &format!("CompDescr{}", i))?;
        ser.export_short_int(comp.entry.part_count as i32, &format!("PartCount{}", i))?;
        ser.export_short_int(comp.entry.aliases.len() as i32, &format!("AliasCount{}", i))?;

        for (j, alias) in comp.entry.aliases.iter().enumerate() {
            ser.export_dynamic_string(alias, &format!("Comp{}Alias{}", i, j))?;
        }
    }

    let params = ser.to_param_string();
    let mut stream = cfb.create_stream("/FileHeader")
        .map_err(|e| AltiumError::Parse(format!("Failed to create FileHeader: {}", e)))?;
    stream.write_all(params.as_bytes()).map_err(AltiumError::Io)?;

    Ok(())
}

// ============================================================================
// SectionKeys read/write
// ============================================================================

fn read_section_keys<F: Read + Seek>(
    cfb: &mut cfb::CompoundFile<F>,
) -> Result<SectionKeyList> {
    let keys = SectionKeyList::new();

    if let Ok(mut stream) = cfb.open_stream("/SectionKeys") {
        let mut data = Vec::new();
        stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
        let param_str = String::from_utf8_lossy(&data);
        let mut ser = AsciiSerializer::from_params(&param_str);

        let count = ser.import_long_int("KeyCount")?;
        for i in 0..count {
            let _name = ser.import_dynamic_string(&format!("Key{}", i))?;
            // Section keys are auto-generated from component names;
            // we rebuild them from the component list rather than
            // storing them separately.
        }
    }

    Ok(keys)
}

fn write_section_keys<F: Read + Write + Seek>(
    cfb: &mut cfb::CompoundFile<F>,
    keys: &SectionKeyList,
) -> Result<()> {
    if keys.is_empty() {
        return Ok(());
    }

    let mut ser = AsciiSerializer::new_writer();
    ser.export_long_int(keys.len() as i32, "KeyCount")?;

    for (i, (name, key)) in keys.iter().enumerate() {
        ser.export_dynamic_string(name, &format!("Key{}", i))?;
        ser.export_dynamic_string(key, &format!("SectionKey{}", i))?;
    }

    let params = ser.to_param_string();
    let mut stream = cfb.create_stream("/SectionKeys")
        .map_err(|e| AltiumError::Parse(format!("Failed to create SectionKeys: {}", e)))?;
    stream.write_all(params.as_bytes()).map_err(AltiumError::Io)?;

    Ok(())
}

// ============================================================================
// Data stream parse/build
// ============================================================================

/// Size flag mask: low 24 bits are the actual length, high byte is flags.
/// Bit 24 (0x01000000) set = binary mode record.
const SIZE_FLAG_MASK: u32 = 0x00FFFFFF;

/// Parse a Data stream into individual records.
///
/// Each record is framed as `[u32 size_with_flags][data]`.
/// - If bit 24 of the size is clear: text mode (ASCII `|KEY=VALUE|` params)
/// - If bit 24 of the size is set: binary mode (sequential typed fields)
///
/// The actual data length is `size & 0x00FFFFFF`.
fn parse_data_stream(data: &[u8]) -> Result<Vec<SchLibRecord>> {
    let mut records = Vec::new();
    let mut cursor = Cursor::new(data);
    let total_len = data.len() as u64;

    while cursor.position() < total_len {
        // Read the u32 size field (includes mode flag in high byte)
        let mut len_buf = [0u8; 4];
        if cursor.read_exact(&mut len_buf).is_err() {
            break;
        }
        let size_raw = u32::from_le_bytes(len_buf);
        let is_binary = (size_raw & !SIZE_FLAG_MASK) != 0;
        let record_len = (size_raw & SIZE_FLAG_MASK) as usize;

        if record_len == 0 {
            continue;
        }

        if cursor.position() as usize + record_len > data.len() {
            break;
        }

        let mut record_data = vec![0u8; record_len];
        if cursor.read_exact(&mut record_data).is_err() {
            break;
        }

        if is_binary {
            // Binary record — store with original size field for lossless roundtrip
            let mut full_raw = Vec::with_capacity(4 + record_len);
            full_raw.extend_from_slice(&len_buf);
            full_raw.extend_from_slice(&record_data);

            // First 4 bytes of data are typically the record type as i32
            let record_type = if record_data.len() >= 4 {
                u32::from_le_bytes([record_data[0], record_data[1], record_data[2], record_data[3]]) as u8
            } else {
                0
            };

            records.push(SchLibRecord {
                record_id: record_type,
                record_id_ex: None,
                params: String::new(),
                raw: full_raw,
            });
        } else {
            // Text record — parse as ASCII params
            let param_str = String::from_utf8_lossy(&record_data).to_string();
            let mut ser = AsciiSerializer::from_params(&param_str);

            let record_id = ser.import_instruction("RECORD").unwrap_or(0);
            let record_id_ex = if record_id == 254 {
                Some(ser.import_instruction_ex("RECORDEX").unwrap_or(0))
            } else {
                None
            };

            records.push(SchLibRecord {
                record_id,
                record_id_ex,
                params: param_str,
                raw: record_data,
            });
        }
    }

    Ok(records)
}

/// Build a Data stream from records.
///
/// Binary records: raw contains `[u32 size_with_flag][data]` — written verbatim.
/// Text records: raw contains just `[data]` — prepend `[u32 size]` before writing.
fn build_data_stream(records: &[SchLibRecord]) -> Result<Vec<u8>> {
    let mut output = Vec::new();

    for record in records {
        if record.params.is_empty() && !record.raw.is_empty() {
            // Binary record — raw includes the size+flag header
            output.extend_from_slice(&record.raw);
        } else if !record.raw.is_empty() {
            // Text record with raw bytes — prepend u32 size (no flag bit)
            let len = record.raw.len() as u32;
            output.extend_from_slice(&len.to_le_bytes());
            output.extend_from_slice(&record.raw);
        } else {
            // Text record without raw — fall back to params string
            let bytes = record.params.as_bytes();
            let len = bytes.len() as u32;
            output.extend_from_slice(&len.to_le_bytes());
            output.extend_from_slice(bytes);
        }
    }

    Ok(output)
}

impl SchLibRecord {
    /// Returns the effective record ID (using extended ID if present).
    pub fn effective_record_id(&self) -> i32 {
        if let Some(ex) = self.record_id_ex {
            ex
        } else {
            self.record_id as i32
        }
    }

    /// Parse this record's params into an AsciiSerializer for field access.
    pub fn to_serializer(&self) -> AsciiSerializer {
        AsciiSerializer::from_params(&self.params)
    }

    /// Parse this raw record into a TypedRecord.
    ///
    /// Returns `TypedRecord::Unknown(record_id)` for unsupported record types.
    pub fn to_typed(&self) -> TypedRecord {
        parse_record_to_typed(self.record_id, &self.params)
    }
}

impl SchLibComponent {
    /// Get an iterator over all pins in this component.
    pub fn pins(&self) -> impl Iterator<Item = &PinData> {
        self.typed_records.iter().filter_map(|r| match r {
            TypedRecord::Pin(p) => Some(p),
            _ => None,
        })
    }

    /// Get a mutable iterator over all pins in this component.
    pub fn pins_mut(&mut self) -> impl Iterator<Item = &mut PinData> {
        self.typed_records.iter_mut().filter_map(|r| match r {
            TypedRecord::Pin(p) => Some(p),
            _ => None,
        })
    }

    /// Get the component data record (first Component record).
    pub fn component_data(&self) -> Option<&ComponentData> {
        self.typed_records.iter().find_map(|r| match r {
            TypedRecord::Component(c) => Some(c),
            _ => None,
        })
    }

    /// Get a mutable reference to the component data record.
    pub fn component_data_mut(&mut self) -> Option<&mut ComponentData> {
        self.typed_records.iter_mut().find_map(|r| match r {
            TypedRecord::Component(c) => Some(c),
            _ => None,
        })
    }

    /// Get an iterator over all parameters in this component.
    pub fn parameters(&self) -> impl Iterator<Item = &ParameterData> {
        self.typed_records.iter().filter_map(|r| match r {
            TypedRecord::Parameter(p) => Some(p),
            _ => None,
        })
    }

    /// Get an iterator over all rectangles in this component.
    pub fn rectangles(&self) -> impl Iterator<Item = &RectangleData> {
        self.typed_records.iter().filter_map(|r| match r {
            TypedRecord::Rectangle(r) => Some(r),
            _ => None,
        })
    }

    /// Get an iterator over all lines in this component.
    pub fn lines(&self) -> impl Iterator<Item = &LineData> {
        self.typed_records.iter().filter_map(|r| match r {
            TypedRecord::Line(l) => Some(l),
            _ => None,
        })
    }

    /// Get an iterator over all arcs in this component.
    pub fn arcs(&self) -> impl Iterator<Item = &ArcData> {
        self.typed_records.iter().filter_map(|r| match r {
            TypedRecord::Arc(a) => Some(a),
            _ => None,
        })
    }

    /// Get an iterator over all polygons in this component.
    pub fn polygons(&self) -> impl Iterator<Item = &PolygonData> {
        self.typed_records.iter().filter_map(|r| match r {
            TypedRecord::Polygon(p) => Some(p),
            _ => None,
        })
    }

    /// Get an iterator over all polylines in this component.
    pub fn polylines(&self) -> impl Iterator<Item = &PolylineData> {
        self.typed_records.iter().filter_map(|r| match r {
            TypedRecord::Polyline(p) => Some(p),
            _ => None,
        })
    }

    /// Get the implementation records for this component.
    pub fn implementations(&self) -> impl Iterator<Item = &ImplementationData> {
        self.typed_records.iter().filter_map(|r| match r {
            TypedRecord::Implementation(i) => Some(i),
            _ => None,
        })
    }

    /// Count the number of pins.
    pub fn pin_count(&self) -> usize {
        self.pins().count()
    }

    /// Get all typed records.
    pub fn typed_records(&self) -> &[TypedRecord] {
        &self.typed_records
    }

    /// Parse raw records into typed records.
    ///
    /// This is called automatically during `SchLibV2::open()`, but can be
    /// called manually if records are modified.
    pub fn parse_typed_records(&mut self) {
        self.typed_records = self.records.iter().map(|r| r.to_typed()).collect();
    }
}

/// Parse a raw record into a TypedRecord based on record_id.
///
/// Record IDs from `SchRecordId` in format/record_ids.rs:
/// - 1: Component
/// - 2: Pin
/// - 3: Symbol
/// - 4: Label
/// - 5: Bezier
/// - 6: Polyline
/// - 7: Polygon
/// - 8: Ellipse
/// - 9: Pie
/// - 10: RoundRectangle
/// - 11: EllipticalArc
/// - 12: Arc
/// - 13: Line
/// - 14: Rectangle
/// - 17: PowerObject
/// - 18: Port
/// - 22: NoERC
/// - 25: NetLabel
/// - 26: Bus
/// - 27: Wire
/// - 28: TextFrame
/// - 29: Junction
/// - 30: Image
/// - 31: Sheet
/// - 34: Designator
/// - 37: BusEntry
/// - 41: Parameter
/// - 44: ImplementationList
/// - 45: Implementation
fn parse_record_to_typed(record_id: u8, params: &str) -> TypedRecord {
    // Skip empty/binary records
    if params.is_empty() {
        return TypedRecord::Unknown(record_id);
    }

    let mut ser = AsciiSerializer::from_params(params);

    match record_id {
        1 => {
            let mut comp = ComponentData::default();
            if import_component(&mut ser, &mut comp).is_ok() {
                TypedRecord::Component(comp)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        2 => {
            let mut pin = PinData::default();
            if import_pin(&mut ser, &mut pin).is_ok() {
                TypedRecord::Pin(pin)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        3 => {
            let mut symbol = SymbolData::default();
            if import_symbol(&mut ser, &mut symbol).is_ok() {
                TypedRecord::Symbol(symbol)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        4 => {
            let mut label = LabelData::default();
            if import_label(&mut ser, &mut label).is_ok() {
                TypedRecord::Label(label)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        5 => {
            let mut bezier = BezierData::default();
            if import_bezier(&mut ser, &mut bezier).is_ok() {
                TypedRecord::Bezier(bezier)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        6 => {
            let mut polyline = PolylineData::default();
            if import_polyline(&mut ser, &mut polyline).is_ok() {
                TypedRecord::Polyline(polyline)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        7 => {
            let mut polygon = PolygonData::default();
            if import_polygon(&mut ser, &mut polygon).is_ok() {
                TypedRecord::Polygon(polygon)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        8 => {
            let mut ellipse = EllipseData::default();
            if import_ellipse(&mut ser, &mut ellipse).is_ok() {
                TypedRecord::Ellipse(ellipse)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        9 => {
            let mut pie = PieData::default();
            if import_pie(&mut ser, &mut pie).is_ok() {
                TypedRecord::Pie(pie)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        10 => {
            let mut rr = RoundRectangleData::default();
            if import_round_rectangle(&mut ser, &mut rr).is_ok() {
                TypedRecord::RoundRectangle(rr)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        11 => {
            let mut ea = EllipticalArcData::default();
            if import_elliptical_arc(&mut ser, &mut ea).is_ok() {
                TypedRecord::EllipticalArc(ea)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        12 => {
            let mut arc = ArcData::default();
            if import_arc(&mut ser, &mut arc).is_ok() {
                TypedRecord::Arc(arc)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        13 => {
            let mut line = LineData::default();
            if import_line(&mut ser, &mut line).is_ok() {
                TypedRecord::Line(line)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        14 => {
            let mut rect = RectangleData::default();
            if import_rectangle(&mut ser, &mut rect).is_ok() {
                TypedRecord::Rectangle(rect)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        34 => {
            let mut desig = DesignatorData::default();
            if import_designator(&mut ser, &mut desig).is_ok() {
                TypedRecord::Designator(desig)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        30 => {
            let mut img = ImageData::default();
            if import_image(&mut ser, &mut img).is_ok() {
                TypedRecord::Image(img)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        41 => {
            let mut param = ParameterData::default();
            if import_parameter(&mut ser, &mut param).is_ok() {
                TypedRecord::Parameter(param)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        44 => {
            let mut impl_list = ImplementationListData::default();
            if import_implementation_list(&mut ser, &mut impl_list).is_ok() {
                TypedRecord::ImplementationList(impl_list)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        45 => {
            let mut imp = ImplementationData::default();
            if import_implementation(&mut ser, &mut imp).is_ok() {
                TypedRecord::Implementation(imp)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        // TODO: Add more record types as needed (17=PowerObject, 18=Port, etc.)
        _ => TypedRecord::Unknown(record_id),
    }
}

/// Parse all raw records into typed records.
fn parse_typed_records(records: &[SchLibRecord]) -> Vec<TypedRecord> {
    records.iter().map(|r| r.to_typed()).collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_stream_round_trip() {
        let records = vec![
            SchLibRecord {
                record_id: 1,
                record_id_ex: None,
                params: "|RECORD=1|LibReference=LM358|PartCount=2|".to_string(),
                raw: Vec::new(),
            },
            SchLibRecord {
                record_id: 2,
                record_id_ex: None,
                params: "|RECORD=2|OwnerIndex=0|Name=VCC|".to_string(),
                raw: Vec::new(),
            },
        ];

        let data = build_data_stream(&records).unwrap();
        let parsed = parse_data_stream(&data).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].record_id, 1);
        assert!(parsed[0].params.contains("LM358"));
        assert_eq!(parsed[1].record_id, 2);
        assert!(parsed[1].params.contains("VCC"));
    }

    #[test]
    fn section_key_integration() {
        let mut keys = SectionKeyList::new();
        keys.add_key("A_Very_Long_Component_Name_That_Is_Over_Thirty", 30);
        let key = keys.get_key("A_Very_Long_Component_Name_That_Is_Over_Thirty");
        assert!(key.len() <= 30);
    }

    #[test]
    fn schlib_record_effective_id() {
        // Normal record
        let rec = SchLibRecord {
            record_id: 2,
            record_id_ex: None,
            params: "|RECORD=2|".to_string(),
            raw: Vec::new(),
        };
        assert_eq!(rec.effective_record_id(), 2);

        // Extended record (record_id=254, actual ID in record_id_ex)
        let rec_ex = SchLibRecord {
            record_id: 254,
            record_id_ex: Some(300),
            params: "|RECORD=254|RECORDEX=300|".to_string(),
            raw: Vec::new(),
        };
        assert_eq!(rec_ex.effective_record_id(), 300);
    }

    #[test]
    fn cfb_round_trip() {
        // Create a SchLib in memory
        let mut lib = SchLibV2::default();
        lib.header = SchLibHeader {
            header_text: "Protel for Windows - Schematic Library Editor Binary File Version 5.0".to_string(),
            weight: 3,
            minor_version: 9,
            unique_id: "TEST123".to_string(),
            raw: None,
        };

        lib.components.push(SchLibComponent {
            entry: SchLibComponentEntry {
                lib_ref: "R1".to_string(),
                description: "Resistor".to_string(),
                part_count: 1,
                aliases: vec![],
            },
            records: vec![
                SchLibRecord {
                    record_id: 1,
                    record_id_ex: None,
                    params: "|RECORD=1|LibReference=R1|PartCount=1|".to_string(),
                    raw: Vec::new(),
                },
                SchLibRecord {
                    record_id: 2,
                    record_id_ex: None,
                    params: "|RECORD=2|OwnerIndex=0|Name=1|Designator=1|".to_string(),
                    raw: Vec::new(),
                },
            ],
            typed_records: vec![],
        });

        // Write to memory buffer
        let buf = Cursor::new(Vec::new());
        lib.write(buf).unwrap();
    }

    #[test]
    fn typed_records_parsing() {
        // Test parsing records into typed structs
        // V2 coordinates: raw param value is in mils, multiplied by 100000 internally
        // E.g., PinLength=2 means 2 mils = 200000 internal units
        let records = vec![
            SchLibRecord {
                record_id: 1,
                record_id_ex: None,
                params: "|RECORD=1|LibReference=LM358|PartCount=2|DisplayModeCount=1|".to_string(),
                raw: Vec::new(),
            },
            SchLibRecord {
                record_id: 2,
                record_id_ex: None,
                // PinLength=2 mils, Location.X=1 mil, Location.Y=2 mils
                params: "|RECORD=2|OwnerIndex=0|OwnerPartId=1|Name=VCC|Designator=1|PinLength=2|Location.X=1|Location.Y=2|PinConglomerate=25|".to_string(),
                raw: Vec::new(),
            },
            SchLibRecord {
                record_id: 14,
                record_id_ex: None,
                // Corner.X=1 mil, Corner.Y=1 mil
                params: "|RECORD=14|Location.X=0|Location.Y=0|Corner.X=1|Corner.Y=1|IsSolid=T|".to_string(),
                raw: Vec::new(),
            },
        ];

        let typed = parse_typed_records(&records);
        assert_eq!(typed.len(), 3);

        // Check component
        match &typed[0] {
            TypedRecord::Component(c) => {
                assert_eq!(c.lib_reference, "LM358");
                assert_eq!(c.part_count, 2);
            }
            _ => panic!("Expected Component record"),
        }

        // Check pin - values are multiplied by 100000 internally
        match &typed[1] {
            TypedRecord::Pin(p) => {
                assert_eq!(p.name, "VCC");
                assert_eq!(p.designator, "1");
                assert_eq!(p.pin_length, 200000);   // 2 * 100000
                assert_eq!(p.location_x, 100000);   // 1 * 100000
                assert_eq!(p.location_y, 200000);   // 2 * 100000
            }
            _ => panic!("Expected Pin record"),
        }

        // Check rectangle
        match &typed[2] {
            TypedRecord::Rectangle(r) => {
                assert!(r.is_solid);
                assert_eq!(r.corner_x, 100000);  // 1 * 100000
                assert_eq!(r.corner_y, 100000);  // 1 * 100000
            }
            _ => panic!("Expected Rectangle record"),
        }
    }

    #[test]
    fn component_pin_accessors() {
        let mut comp = SchLibComponent {
            entry: SchLibComponentEntry {
                lib_ref: "Test".to_string(),
                description: "Test component".to_string(),
                part_count: 1,
                aliases: vec![],
            },
            records: vec![
                SchLibRecord {
                    record_id: 2,
                    record_id_ex: None,
                    params: "|RECORD=2|OwnerIndex=0|Name=PIN1|Designator=1|PinConglomerate=25|".to_string(),
                    raw: Vec::new(),
                },
                SchLibRecord {
                    record_id: 2,
                    record_id_ex: None,
                    params: "|RECORD=2|OwnerIndex=0|Name=PIN2|Designator=2|PinConglomerate=25|".to_string(),
                    raw: Vec::new(),
                },
            ],
            typed_records: vec![],
        };

        // Parse typed records
        comp.parse_typed_records();

        // Test pin count
        assert_eq!(comp.pin_count(), 2);

        // Test pin iterator
        let pin_names: Vec<&str> = comp.pins().map(|p| p.name.as_str()).collect();
        assert_eq!(pin_names, vec!["PIN1", "PIN2"]);
    }
}

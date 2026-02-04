//! SchDoc file I/O — lossless CFB roundtrip.
//!
//! # CFB Storage Hierarchy
//!
//! ```text
//! Root
//! ├── FileHeader    (header params + primitive records)
//! ├── Storage       (icon storage header)
//! └── Additional    (additional params)
//! ```
//!
//! # FileHeader Format
//!
//! The FileHeader stream is a sequence of framed records:
//! - `[u32 size_with_flags][data]`
//! - Low 24 bits = actual length, high byte = flags
//! - Bit 24 set = binary mode record, clear = text/ASCII mode
//! - First record is the header block with HEADER, WEIGHT params
//! - Remaining records are schematic primitives

use serde::{Deserialize, Serialize};
use std::io::{self, Cursor, Read, Seek, Write};

use crate::v2::fields::{
    TypedRecord, PinData, ComponentData, ParameterData, RectangleData, LineData,
    ArcData, EllipseData, PolygonData, PolylineData, BezierData, EllipticalArcData,
    PieData, RoundRectangleData, ImageData, DesignatorData, LabelData, SymbolData,
    ImplementationData, ImplementationListData, WireData, BusData, JunctionData,
    NetLabelData, PowerData, PortData, NoERCData, TextFrameData, BusEntryData,
    SheetData, SheetSymbolData, SheetEntryData, SheetNameData, SheetFileNameData,
};
use crate::v2::serializer::SchSerializer;
use crate::v2::serializer::ascii::AsciiSerializer;
use crate::v2::serializer::format_v5::{
    import_pin, import_component, import_parameter, import_rectangle, import_line,
    import_arc, import_ellipse, import_polygon, import_polyline, import_bezier,
    import_elliptical_arc, import_pie, import_round_rectangle, import_image,
    import_designator, import_label, import_symbol, import_implementation,
    import_implementation_list, import_wire, import_bus, import_junction,
    import_net_label, import_power, import_port, import_no_erc, import_text_frame,
    import_bus_entry, import_sheet, import_sheet_symbol, import_sheet_entry,
    import_sheet_name, import_sheet_file_name,
};

/// A parsed SchDoc file.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SchDocV2 {
    /// Parsed record count from the header.
    pub weight: i32,
    /// Parsed primitive records from the FileHeader stream.
    pub records: Vec<SchDocRecord>,
    /// Typed/parsed records for strongly-typed access.
    /// Populated during `open()` by parsing raw records via `import_*` functions.
    #[serde(skip)]
    pub typed_records: Vec<TypedRecord>,
    /// All raw CFB streams for lossless roundtrip.
    #[serde(skip)]
    pub raw_streams: Vec<(String, Vec<u8>)>,
}

/// A single record within the SchDoc FileHeader.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchDocRecord {
    /// Object ID / record type.
    pub record_id: u8,
    /// Extended record ID (when record_id == 254).
    pub record_id_ex: Option<i32>,
    /// Decoded parameter string (empty for binary records).
    pub params: String,
    /// Raw record bytes for lossless roundtrip.
    #[serde(skip)]
    pub raw: Vec<u8>,
}

/// Size flag mask: low 24 bits are the actual length, high byte is flags.
const SIZE_FLAG_MASK: u32 = 0x00FFFFFF;

impl SchDocV2 {
    /// Open and parse a SchDoc CFB file.
    pub fn open<R: Read + Seek>(reader: R) -> io::Result<Self> {
        let mut cfb = cfb::CompoundFile::open(reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut doc = SchDocV2::default();

        // Parse FileHeader for structured data
        if let Ok(fh_data) = read_cfb_stream(&mut cfb, "/FileHeader") {
            let (weight, records) = parse_file_header(&fh_data);
            doc.weight = weight;
            doc.records = records;
            // Parse raw records into typed records for strongly-typed access
            doc.typed_records = parse_typed_records(&doc.records);
        }

        // Collect ALL streams for lossless roundtrip
        let all_entries: Vec<(String, bool)> = cfb
            .walk()
            .map(|e| (e.path().to_string_lossy().replace('\\', "/"), e.is_stream()))
            .collect();

        for (path, is_stream) in &all_entries {
            if !is_stream {
                continue;
            }
            let normalized = if path.starts_with('/') {
                path.clone()
            } else {
                format!("/{}", path)
            };
            if let Ok(data) = read_cfb_stream(&mut cfb, &normalized) {
                doc.raw_streams.push((normalized, data));
            }
        }

        Ok(doc)
    }

    /// Get an iterator over all components in this document.
    pub fn components(&self) -> impl Iterator<Item = &ComponentData> {
        self.typed_records.iter().filter_map(|r| match r {
            TypedRecord::Component(c) => Some(c),
            _ => None,
        })
    }

    /// Get an iterator over all wires in this document.
    pub fn wires(&self) -> impl Iterator<Item = &WireData> {
        self.typed_records.iter().filter_map(|r| match r {
            TypedRecord::Wire(w) => Some(w),
            _ => None,
        })
    }

    /// Get an iterator over all net labels in this document.
    pub fn net_labels(&self) -> impl Iterator<Item = &NetLabelData> {
        self.typed_records.iter().filter_map(|r| match r {
            TypedRecord::NetLabel(n) => Some(n),
            _ => None,
        })
    }

    /// Get an iterator over all power objects in this document.
    pub fn power_objects(&self) -> impl Iterator<Item = &PowerData> {
        self.typed_records.iter().filter_map(|r| match r {
            TypedRecord::PowerObject(p) => Some(p),
            _ => None,
        })
    }

    /// Get an iterator over all junctions in this document.
    pub fn junctions(&self) -> impl Iterator<Item = &JunctionData> {
        self.typed_records.iter().filter_map(|r| match r {
            TypedRecord::Junction(j) => Some(j),
            _ => None,
        })
    }

    /// Get the sheet header record if present.
    pub fn sheet(&self) -> Option<&SheetData> {
        self.typed_records.iter().find_map(|r| match r {
            TypedRecord::Sheet(s) => Some(s),
            _ => None,
        })
    }

    /// Get all typed records.
    pub fn typed_records(&self) -> &[TypedRecord] {
        &self.typed_records
    }

    /// Parse raw records into typed records.
    ///
    /// This is called automatically during `open()`, but can be
    /// called manually if records are modified.
    pub fn parse_typed_records(&mut self) {
        self.typed_records = parse_typed_records(&self.records);
    }

    /// Write a SchDoc to a CFB compound file, serializing from typed fields.
    ///
    /// The `/FileHeader` stream is rebuilt from `self.records`.
    /// Other streams (Storage, Additional) are written from `raw_streams`.
    pub fn write<W: Read + Write + Seek>(&self, writer: W) -> io::Result<()> {
        let mut cfb = cfb::CompoundFile::create(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Write non-FileHeader streams from raw_streams
        for (path, data) in &self.raw_streams {
            if path == "/FileHeader" {
                continue;
            }
            let mut stream = cfb.create_stream(path)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("create stream {}: {}", path, e)))?;
            stream.write_all(data)?;
        }

        // Rebuild FileHeader from records
        let fh_data = build_file_header(&self.records)?;
        let mut stream = cfb.create_stream("/FileHeader")
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("create FileHeader: {}", e)))?;
        stream.write_all(&fh_data)?;

        cfb.flush()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("CFB flush: {}", e)))?;
        Ok(())
    }
}

/// Parse the FileHeader stream into weight + records.
fn parse_file_header(data: &[u8]) -> (i32, Vec<SchDocRecord>) {
    let mut records = Vec::new();
    let mut cursor = Cursor::new(data);
    let total_len = data.len() as u64;
    let mut weight = 0i32;
    let mut is_first = true;

    while cursor.position() < total_len {
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

        if is_first {
            // First record is the header block — extract WEIGHT
            is_first = false;
            let param_str = String::from_utf8_lossy(&record_data);
            let mut ser = AsciiSerializer::from_params(&param_str);
            weight = ser.import_long_int("WEIGHT").unwrap_or(0);
            // Store as a record too for completeness
            let record_id = ser.import_instruction("RECORD").unwrap_or(0);
            records.push(SchDocRecord {
                record_id,
                record_id_ex: None,
                params: param_str.to_string(),
                raw: record_data,
            });
            continue;
        }

        if is_binary {
            let mut full_raw = Vec::with_capacity(4 + record_len);
            full_raw.extend_from_slice(&len_buf);
            full_raw.extend_from_slice(&record_data);

            let record_type = if record_data.len() >= 4 {
                u32::from_le_bytes([record_data[0], record_data[1], record_data[2], record_data[3]]) as u8
            } else {
                0
            };

            records.push(SchDocRecord {
                record_id: record_type,
                record_id_ex: None,
                params: String::new(),
                raw: full_raw,
            });
        } else {
            let param_str = String::from_utf8_lossy(&record_data).to_string();
            let mut ser = AsciiSerializer::from_params(&param_str);
            let record_id = ser.import_instruction("RECORD").unwrap_or(0);
            let record_id_ex = if record_id == 254 {
                Some(ser.import_instruction_ex("RECORDEX").unwrap_or(0))
            } else {
                None
            };

            records.push(SchDocRecord {
                record_id,
                record_id_ex,
                params: param_str,
                raw: record_data,
            });
        }
    }

    (weight, records)
}

/// Rebuild the FileHeader stream from records.
///
/// Binary records: `raw` contains `[u32 size_with_flag][data]` — written verbatim.
/// Text records: `raw` contains just `[data]` — prepend `[u32 size]` before writing.
/// Text records without raw: fall back to `params` string.
fn build_file_header(records: &[SchDocRecord]) -> io::Result<Vec<u8>> {
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

fn read_cfb_stream<F: Read + Seek>(cfb: &mut cfb::CompoundFile<F>, path: &str) -> io::Result<Vec<u8>> {
    let mut stream = cfb.open_stream(path)
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e))?;
    let mut data = Vec::new();
    stream.read_to_end(&mut data)?;
    Ok(data)
}

impl SchDocRecord {
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
    pub fn to_typed(&self) -> TypedRecord {
        parse_record_to_typed(self.record_id, &self.params)
    }
}

/// Parse a raw record into a TypedRecord based on record_id.
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
        17 => {
            let mut power = PowerData::default();
            if import_power(&mut ser, &mut power).is_ok() {
                TypedRecord::PowerObject(power)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        18 => {
            let mut port = PortData::default();
            if import_port(&mut ser, &mut port).is_ok() {
                TypedRecord::Port(port)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        22 => {
            let mut noerc = NoERCData::default();
            if import_no_erc(&mut ser, &mut noerc).is_ok() {
                TypedRecord::NoERC(noerc)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        25 => {
            let mut netlabel = NetLabelData::default();
            if import_net_label(&mut ser, &mut netlabel).is_ok() {
                TypedRecord::NetLabel(netlabel)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        26 => {
            let mut bus = BusData::default();
            if import_bus(&mut ser, &mut bus).is_ok() {
                TypedRecord::Bus(bus)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        27 => {
            let mut wire = WireData::default();
            if import_wire(&mut ser, &mut wire).is_ok() {
                TypedRecord::Wire(wire)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        28 => {
            let mut tf = TextFrameData::default();
            if import_text_frame(&mut ser, &mut tf).is_ok() {
                TypedRecord::TextFrame(tf)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        29 => {
            let mut junc = JunctionData::default();
            if import_junction(&mut ser, &mut junc).is_ok() {
                TypedRecord::Junction(junc)
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
        31 => {
            let mut sheet = SheetData::default();
            if import_sheet(&mut ser, &mut sheet).is_ok() {
                TypedRecord::Sheet(sheet)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        32 => {
            let mut sn = SheetNameData::default();
            if import_sheet_name(&mut ser, &mut sn).is_ok() {
                TypedRecord::SheetName(sn)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        33 => {
            let mut sfn = SheetFileNameData::default();
            if import_sheet_file_name(&mut ser, &mut sfn).is_ok() {
                TypedRecord::SheetFileName(sfn)
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
        37 => {
            let mut be = BusEntryData::default();
            if import_bus_entry(&mut ser, &mut be).is_ok() {
                TypedRecord::BusEntry(be)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        39 => {
            let mut ss = SheetSymbolData::default();
            if import_sheet_symbol(&mut ser, &mut ss).is_ok() {
                TypedRecord::SheetSymbol(ss)
            } else {
                TypedRecord::Unknown(record_id)
            }
        }
        40 => {
            let mut se = SheetEntryData::default();
            if import_sheet_entry(&mut ser, &mut se).is_ok() {
                TypedRecord::SheetEntry(se)
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
        _ => TypedRecord::Unknown(record_id),
    }
}

/// Parse all raw records into typed records.
fn parse_typed_records(records: &[SchDocRecord]) -> Vec<TypedRecord> {
    records.iter().map(|r| r.to_typed()).collect()
}

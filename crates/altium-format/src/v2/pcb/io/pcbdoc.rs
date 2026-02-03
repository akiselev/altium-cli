//! PcbDoc reader: opens CFB compound file and reads all PCB sections.
//!
//! Each section has Header (u32 record count) and Data (binary records) sub-streams.
//! Most sections use `u8 type + u32 len + data` framing.
//! Connections6 uses `u32 len + data` (no type byte).
//! Parametric sections (Components6, Nets6, Polygons6, etc.) use `u32 len + |KEY=VALUE|` text.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Cursor, Read, Write};

use super::streams;
use crate::v2::pcb::arc::PcbArc;
use crate::v2::pcb::board::PcbBoard;
use crate::v2::pcb::class::PcbClass;
use crate::v2::pcb::component::PcbComponent;
use crate::v2::pcb::connection::PcbConnection;
use crate::v2::pcb::dimension::PcbDimension;
use crate::v2::pcb::fill::PcbFill;
use crate::v2::pcb::net::PcbNet;
use crate::v2::pcb::pad::PcbPad;
use crate::v2::pcb::polygon::PcbPolygon;
use crate::v2::pcb::primitive::PcbObjectId;
use crate::v2::pcb::region::{parse_parametric, serialize_parametric, PcbRegion};
use crate::v2::pcb::rule::PcbRule;
use crate::v2::pcb::text::PcbText;
use crate::v2::pcb::track::PcbTrack;
use crate::v2::pcb::via::PcbVia;

/// A parsed PcbDoc file.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PcbDoc {
    pub board: Option<PcbBoard>,
    pub tracks: Vec<PcbTrack>,
    pub arcs: Vec<PcbArc>,
    pub fills: Vec<PcbFill>,
    pub pads: Vec<PcbPad>,
    pub vias: Vec<PcbVia>,
    pub texts: Vec<PcbText>,
    pub connections: Vec<PcbConnection>,
    pub nets: Vec<PcbNet>,
    pub components: Vec<PcbComponent>,
    pub polygons: Vec<PcbPolygon>,
    pub regions: Vec<PcbRegion>,
    pub component_bodies: Vec<PcbRegion>,
    pub rules: Vec<PcbRule>,
    pub classes: Vec<PcbClass>,
    pub dimensions: Vec<PcbDimension>,
    /// WideStrings table (indexed by widestring_index in Text records).
    pub wide_strings: Vec<String>,
    /// ExtendedPrimitiveInformation records.
    pub extended_primitive_info: Vec<HashMap<String, String>>,
    /// All raw CFB streams for lossless roundtrip.
    #[serde(skip)]
    pub raw_streams: Vec<(String, Vec<u8>)>,
}

impl PcbDoc {
    /// Open and parse a PcbDoc CFB file.
    pub fn open<R: Read + io::Seek>(reader: R) -> io::Result<Self> {
        let mut cfb = cfb::CompoundFile::open(reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut doc = PcbDoc::default();

        // Board6
        if let Ok(data) = read_cfb_stream(&mut cfb, &format!("{}/{}", streams::STREAM_BOARD6, streams::SUB_DATA)) {
            if !data.is_empty() {
                let text = streams::read_parametric_block(&mut Cursor::new(&data))?;
                doc.board = Some(PcbBoard::from_properties(parse_parametric(&text)));
            }
        }

        // Tracks6
        doc.tracks = read_binary_section(&mut cfb, streams::STREAM_TRACKS6, |data| {
            PcbTrack::read_from(&mut Cursor::new(data))
        })?;

        // Arcs6
        doc.arcs = read_binary_section(&mut cfb, streams::STREAM_ARCS6, |data| {
            PcbArc::read_from(&mut Cursor::new(data))
        })?;

        // Fills6
        doc.fills = read_binary_section(&mut cfb, streams::STREAM_FILLS6, |data| {
            PcbFill::read_from(&mut Cursor::new(data))
        })?;

        // Pads6 (multi-block: type byte + 6 subrecords)
        if let Ok(data) = read_cfb_stream(&mut cfb, &format!("{}/{}", streams::STREAM_PADS6, streams::SUB_DATA)) {
            let mut cursor = Cursor::new(&data);
            while cursor.position() < data.len() as u64 {
                // Read type byte
                let mut type_buf = [0u8; 1];
                if cursor.read_exact(&mut type_buf).is_err() {
                    break;
                }
                match PcbPad::read_from(&mut cursor) {
                    Ok(pad) => doc.pads.push(pad),
                    Err(_) => break,
                }
            }
        }

        // Vias6 (single subrecord: type + u32 len + data)
        if let Ok(data) = read_cfb_stream(&mut cfb, &format!("{}/{}", streams::STREAM_VIAS6, streams::SUB_DATA)) {
            let mut cursor = Cursor::new(&data);
            while cursor.position() < data.len() as u64 {
                match streams::read_binary_block(&mut cursor) {
                    Ok((_type_byte, block_data)) => {
                        match PcbVia::from_bytes(&block_data) {
                            Ok(via) => doc.vias.push(via),
                            Err(_) => break,
                        }
                    }
                    Err(_) => break,
                }
            }
        }

        // Texts6 (2 subrecords: type + sub1 + sub2)
        if let Ok(data) = read_cfb_stream(&mut cfb, &format!("{}/{}", streams::STREAM_TEXTS6, streams::SUB_DATA)) {
            let mut cursor = Cursor::new(&data);
            while cursor.position() < data.len() as u64 {
                // Read type byte
                let mut type_buf = [0u8; 1];
                if cursor.read_exact(&mut type_buf).is_err() {
                    break;
                }
                match PcbText::read_from(&mut cursor) {
                    Ok(text) => doc.texts.push(text),
                    Err(_) => break,
                }
            }
        }

        // Connections6 (NO type byte — u32 len + data only)
        if let Ok(data) = read_cfb_stream(&mut cfb, &format!("{}/{}", streams::STREAM_CONNECTIONS6, streams::SUB_DATA)) {
            let mut cursor = Cursor::new(&data);
            while cursor.position() < data.len() as u64 {
                match streams::read_connection_block(&mut cursor) {
                    Ok(block_data) => {
                        match PcbConnection::read_from(&mut Cursor::new(&block_data)) {
                            Ok(conn) => doc.connections.push(conn),
                            Err(_) => break,
                        }
                    }
                    Err(_) => break,
                }
            }
        }

        // Nets6 (parametric)
        doc.nets = read_parametric_section(&mut cfb, streams::STREAM_NETS6, PcbNet::from_properties)?;

        // Components6 (parametric)
        doc.components = read_parametric_section(&mut cfb, streams::STREAM_COMPONENTS6, PcbComponent::from_properties)?;

        // Polygons6 (parametric)
        doc.polygons = read_parametric_section(&mut cfb, streams::STREAM_POLYGONS6, PcbPolygon::from_properties)?;

        // Regions6 (hybrid: type + u32 len + binary+parametric+vertices)
        doc.regions = read_hybrid_section(&mut cfb, streams::STREAM_REGIONS6)?;

        // ComponentBodies6 (hybrid, same structure as Region)
        doc.component_bodies = read_hybrid_section(&mut cfb, streams::STREAM_COMPONENT_BODIES6)?;

        // Rules6 (parametric)
        doc.rules = read_parametric_section(&mut cfb, streams::STREAM_RULES6, PcbRule::from_properties)?;

        // Classes6 (parametric)
        doc.classes = read_parametric_section(&mut cfb, streams::STREAM_CLASSES6, PcbClass::from_properties)?;

        // Dimensions6 (parametric)
        doc.dimensions = read_parametric_section(&mut cfb, streams::STREAM_DIMENSIONS6, PcbDimension::from_properties)?;

        // WideStrings6
        if let Ok(data) = read_cfb_stream(&mut cfb, &format!("{}/{}", streams::STREAM_WIDE_STRINGS6, streams::SUB_DATA)) {
            let mut cursor = Cursor::new(&data);
            while cursor.position() < data.len() as u64 {
                match streams::read_parametric_block(&mut cursor) {
                    Ok(text) => doc.wide_strings.push(text),
                    Err(_) => break,
                }
            }
        }

        // ExtendedPrimitiveInformation
        if let Ok(data) = read_cfb_stream(&mut cfb, &format!("{}/{}", streams::STREAM_EXTENDED_PRIMITIVE_INFO, streams::SUB_DATA)) {
            let mut cursor = Cursor::new(&data);
            while cursor.position() < data.len() as u64 {
                match streams::read_parametric_block(&mut cursor) {
                    Ok(text) => doc.extended_primitive_info.push(parse_parametric(&text)),
                    Err(_) => break,
                }
            }
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

    /// Write a PcbDoc to a CFB compound file, serializing from typed fields.
    ///
    /// Sections that were parsed into typed fields are rebuilt from those types.
    /// All other streams (EmbeddedFonts6, Models, FileVersionInfo, etc.) are
    /// written verbatim from `raw_streams`.
    pub fn write<W: Read + Write + io::Seek>(&self, writer: W) -> io::Result<()> {
        use std::collections::HashSet;

        let mut cfb = cfb::CompoundFile::create(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Build the set of Data stream paths we'll rebuild from types
        let rebuilt_data_paths: HashSet<String> = [
            streams::STREAM_BOARD6,
            streams::STREAM_TRACKS6,
            streams::STREAM_ARCS6,
            streams::STREAM_FILLS6,
            streams::STREAM_PADS6,
            streams::STREAM_VIAS6,
            streams::STREAM_TEXTS6,
            streams::STREAM_CONNECTIONS6,
            streams::STREAM_NETS6,
            streams::STREAM_COMPONENTS6,
            streams::STREAM_POLYGONS6,
            streams::STREAM_REGIONS6,
            streams::STREAM_COMPONENT_BODIES6,
            streams::STREAM_RULES6,
            streams::STREAM_CLASSES6,
            streams::STREAM_DIMENSIONS6,
            streams::STREAM_WIDE_STRINGS6,
            streams::STREAM_EXTENDED_PRIMITIVE_INFO,
        ]
        .iter()
        .map(|s| format!("/{}/{}", s, streams::SUB_DATA))
        .collect();

        // Write all raw streams that we DON'T rebuild
        for (path, data) in &self.raw_streams {
            if rebuilt_data_paths.contains(path.as_str()) {
                continue;
            }
            ensure_parent_storages(&mut cfb, path)?;
            let mut stream = cfb.create_stream(path)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("create stream {}: {}", path, e)))?;
            io::Write::write_all(&mut stream, data)?;
        }

        // Now rebuild and write each typed section's Data stream

        // Board6/Data
        if let Some(board) = &self.board {
            let data = build_parametric_data_single(&board.properties)?;
            write_rebuilt_stream(&mut cfb, streams::STREAM_BOARD6, &data)?;
        }

        // Tracks6/Data
        {
            let data = build_binary_section_data(PcbObjectId::Track as u8, &self.tracks, |t, w| t.write_to(w))?;
            write_rebuilt_stream(&mut cfb, streams::STREAM_TRACKS6, &data)?;
        }

        // Arcs6/Data
        {
            let data = build_binary_section_data(PcbObjectId::Arc as u8, &self.arcs, |a, w| a.write_to(w))?;
            write_rebuilt_stream(&mut cfb, streams::STREAM_ARCS6, &data)?;
        }

        // Fills6/Data
        {
            let data = build_binary_section_data(PcbObjectId::Fill as u8, &self.fills, |f, w| f.write_to(w))?;
            write_rebuilt_stream(&mut cfb, streams::STREAM_FILLS6, &data)?;
        }

        // Pads6/Data (type byte + 6 subrecords, no outer length)
        {
            let mut data = Vec::new();
            for pad in &self.pads {
                data.push(PcbObjectId::Pad as u8);
                pad.write_to(&mut data)?;
            }
            write_rebuilt_stream(&mut cfb, streams::STREAM_PADS6, &data)?;
        }

        // Vias6/Data
        {
            let mut data = Vec::new();
            for via in &self.vias {
                let bytes = via.to_bytes();
                streams::write_binary_block(&mut data, PcbObjectId::Via as u8, &bytes)?;
            }
            write_rebuilt_stream(&mut cfb, streams::STREAM_VIAS6, &data)?;
        }

        // Texts6/Data (type byte + 2 subrecords, no outer length)
        {
            let mut data = Vec::new();
            for text in &self.texts {
                data.push(PcbObjectId::Text as u8);
                text.write_to(&mut data)?;
            }
            write_rebuilt_stream(&mut cfb, streams::STREAM_TEXTS6, &data)?;
        }

        // Connections6/Data (no type byte)
        {
            let mut data = Vec::new();
            for conn in &self.connections {
                let mut record = Vec::new();
                conn.write_to(&mut record)?;
                streams::write_connection_block(&mut data, &record)?;
            }
            write_rebuilt_stream(&mut cfb, streams::STREAM_CONNECTIONS6, &data)?;
        }

        // Parametric sections
        write_parametric_section(&mut cfb, streams::STREAM_NETS6, &self.nets, |n| &n.properties)?;
        write_parametric_section(&mut cfb, streams::STREAM_COMPONENTS6, &self.components, |c| &c.properties)?;
        write_parametric_section(&mut cfb, streams::STREAM_POLYGONS6, &self.polygons, |p| &p.properties)?;
        write_parametric_section(&mut cfb, streams::STREAM_RULES6, &self.rules, |r| &r.properties)?;
        write_parametric_section(&mut cfb, streams::STREAM_CLASSES6, &self.classes, |c| &c.properties)?;
        write_parametric_section(&mut cfb, streams::STREAM_DIMENSIONS6, &self.dimensions, |d| &d.properties)?;

        // Regions6/Data (hybrid: type + u32 len + binary data)
        {
            let data = build_hybrid_section_data(PcbObjectId::Region as u8, &self.regions)?;
            write_rebuilt_stream(&mut cfb, streams::STREAM_REGIONS6, &data)?;
        }

        // ComponentBodies6/Data
        {
            let data = build_hybrid_section_data(PcbObjectId::ComponentBody as u8, &self.component_bodies)?;
            write_rebuilt_stream(&mut cfb, streams::STREAM_COMPONENT_BODIES6, &data)?;
        }

        // WideStrings6/Data
        {
            let mut data = Vec::new();
            for s in &self.wide_strings {
                streams::write_parametric_block(&mut data, s)?;
            }
            write_rebuilt_stream(&mut cfb, streams::STREAM_WIDE_STRINGS6, &data)?;
        }

        // ExtendedPrimitiveInformation/Data
        {
            let mut data = Vec::new();
            for props in &self.extended_primitive_info {
                let text = serialize_parametric(props);
                streams::write_parametric_block(&mut data, &text)?;
            }
            write_rebuilt_stream(&mut cfb, streams::STREAM_EXTENDED_PRIMITIVE_INFO, &data)?;
        }

        cfb.flush()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("CFB flush: {}", e)))?;
        Ok(())
    }
}

/// Ensure all parent storages exist for a given stream path.
fn ensure_parent_storages<F: Read + Write + io::Seek>(
    cfb: &mut cfb::CompoundFile<F>,
    path: &str,
) -> io::Result<()> {
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let mut current = String::new();
    for part in &parts[..parts.len().saturating_sub(1)] {
        current = format!("{}/{}", current, part);
        if !cfb.is_storage(&current) {
            cfb.create_storage(&current)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("create storage {}: {}", current, e)))?;
        }
    }
    Ok(())
}

/// Read a CFB stream, returning its full contents.
fn read_cfb_stream<F: Read + io::Seek>(cfb: &mut cfb::CompoundFile<F>, path: &str) -> io::Result<Vec<u8>> {
    let mut stream = cfb.open_stream(path)
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e))?;
    let mut data = Vec::new();
    stream.read_to_end(&mut data)?;
    Ok(data)
}

/// Read a binary section with standard framing (type + u32 len + data).
fn read_binary_section<F, T>(
    cfb: &mut cfb::CompoundFile<F>,
    section_name: &str,
    parse: impl Fn(&[u8]) -> io::Result<T>,
) -> io::Result<Vec<T>>
where
    F: Read + io::Seek,
{
    let data_path = format!("{}/{}", section_name, streams::SUB_DATA);
    let data = match read_cfb_stream(cfb, &data_path) {
        Ok(d) => d,
        Err(_) => return Ok(Vec::new()),
    };

    let mut records = Vec::new();
    let mut cursor = Cursor::new(&data);
    while cursor.position() < data.len() as u64 {
        match streams::read_binary_block(&mut cursor) {
            Ok((_type_byte, block_data)) => {
                match parse(&block_data) {
                    Ok(record) => records.push(record),
                    Err(_) => break,
                }
            }
            Err(_) => break,
        }
    }
    Ok(records)
}

/// Read a parametric section (u32 len + |KEY=VALUE| text per record).
fn read_parametric_section<F, T>(
    cfb: &mut cfb::CompoundFile<F>,
    section_name: &str,
    from_props: impl Fn(HashMap<String, String>) -> T,
) -> io::Result<Vec<T>>
where
    F: Read + io::Seek,
{
    let data_path = format!("{}/{}", section_name, streams::SUB_DATA);
    let data = match read_cfb_stream(cfb, &data_path) {
        Ok(d) => d,
        Err(_) => return Ok(Vec::new()),
    };

    let mut records = Vec::new();
    let mut cursor = Cursor::new(&data);
    while cursor.position() < data.len() as u64 {
        match streams::read_parametric_block(&mut cursor) {
            Ok(text) => {
                if !text.is_empty() {
                    records.push(from_props(parse_parametric(&text)));
                }
            }
            Err(_) => break,
        }
    }
    Ok(records)
}

/// Read a hybrid section (type + u32 len + binary+parametric data).
fn read_hybrid_section<F: Read + io::Seek>(
    cfb: &mut cfb::CompoundFile<F>,
    section_name: &str,
) -> io::Result<Vec<PcbRegion>> {
    let data_path = format!("{}/{}", section_name, streams::SUB_DATA);
    let data = match read_cfb_stream(cfb, &data_path) {
        Ok(d) => d,
        Err(_) => return Ok(Vec::new()),
    };

    let mut records = Vec::new();
    let mut cursor = Cursor::new(&data);
    while cursor.position() < data.len() as u64 {
        match streams::read_binary_block(&mut cursor) {
            Ok((_type_byte, block_data)) => {
                match PcbRegion::read_from(&block_data) {
                    Ok(region) => records.push(region),
                    Err(_) => break,
                }
            }
            Err(_) => break,
        }
    }
    Ok(records)
}

// ── Write helpers ──────────────────────────────────────────────────────

/// Write a rebuilt Data stream into the CFB, ensuring parent storages exist.
fn write_rebuilt_stream<F: Read + Write + io::Seek>(
    cfb: &mut cfb::CompoundFile<F>,
    section_name: &str,
    data: &[u8],
) -> io::Result<()> {
    let path = format!("/{}/{}", section_name, streams::SUB_DATA);
    ensure_parent_storages(cfb, &path)?;
    let mut stream = cfb.create_stream(&path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("create stream {}: {}", path, e)))?;
    stream.write_all(data)?;
    Ok(())
}

/// Build Data stream for a binary section (type + u32 len + record bytes).
fn build_binary_section_data<T>(
    type_byte: u8,
    records: &[T],
    write_fn: impl Fn(&T, &mut Vec<u8>) -> io::Result<()>,
) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    for record in records {
        let mut record_bytes = Vec::new();
        write_fn(record, &mut record_bytes)?;
        streams::write_binary_block(&mut data, type_byte, &record_bytes)?;
    }
    Ok(data)
}

/// Build Data stream for a hybrid section (type + u32 len + region data).
fn build_hybrid_section_data(type_byte: u8, regions: &[PcbRegion]) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    for region in regions {
        let mut record_bytes = Vec::new();
        region.write_to(&mut record_bytes)?;
        streams::write_binary_block(&mut data, type_byte, &record_bytes)?;
    }
    Ok(data)
}

/// Build and write a parametric section Data stream.
fn write_parametric_section<F, T>(
    cfb: &mut cfb::CompoundFile<F>,
    section_name: &str,
    records: &[T],
    get_props: impl Fn(&T) -> &HashMap<String, String>,
) -> io::Result<()>
where
    F: Read + Write + io::Seek,
{
    let mut data = Vec::new();
    for record in records {
        let text = serialize_parametric(get_props(record));
        streams::write_parametric_block(&mut data, &text)?;
    }
    write_rebuilt_stream(cfb, section_name, &data)
}

/// Build Data stream for Board6 (single parametric record).
fn build_parametric_data_single(props: &HashMap<String, String>) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    let text = serialize_parametric(props);
    streams::write_parametric_block(&mut data, &text)?;
    Ok(data)
}

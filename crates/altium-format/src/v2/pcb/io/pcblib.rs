//! PcbLib reader: opens CFB compound file and reads per-footprint sections.
//!
//! PcbLib organizes primitives per-footprint rather than by type:
//! ```text
//! <file.pcblib>/
//! ├── FileHeader
//! ├── Library/
//! │   ├── Header + Data   (TOC: footprint name list)
//! │   ├── ComponentParamsTOC/
//! │   ├── Models/
//! │   └── ...
//! ├── <Footprint1>/
//! │   ├── Header           (u32: primitive count)
//! │   ├── Data             (binary primitive records)
//! │   ├── Parameters       (pipe-delimited parameters)
//! │   └── WideStrings
//! └── SectionKeys
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Cursor, Read};

use super::streams;
use crate::v2::pcb::arc::PcbArc;
use crate::v2::pcb::fill::PcbFill;
use crate::v2::pcb::pad::PcbPad;
use crate::v2::pcb::primitive::PcbObjectId;
use crate::v2::pcb::region::PcbRegion;
use crate::v2::pcb::text::PcbText;
use crate::v2::pcb::track::PcbTrack;
use crate::v2::pcb::via::PcbVia;

/// A single footprint in a PcbLib.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PcbLibFootprint {
    pub name: String,
    pub primitive_count: u32,
    pub tracks: Vec<PcbTrack>,
    pub arcs: Vec<PcbArc>,
    pub fills: Vec<PcbFill>,
    pub pads: Vec<PcbPad>,
    pub vias: Vec<PcbVia>,
    pub texts: Vec<PcbText>,
    pub regions: Vec<PcbRegion>,
    pub component_bodies: Vec<PcbRegion>,
    /// Parametric properties for this footprint.
    pub parameters: HashMap<String, String>,
    /// Raw CFB streams for this footprint (lossless roundtrip).
    /// Keys are stream names like "Data", "Header", "Parameters", "WideStrings".
    #[serde(skip)]
    pub raw_streams: HashMap<String, Vec<u8>>,
}

/// A parsed PcbLib file.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PcbLib {
    pub footprints: Vec<PcbLibFootprint>,
    /// Raw CFB streams not associated with a footprint (lossless roundtrip).
    /// Keys are full paths like "/Library/EmbeddedFonts", "/SectionKeys", etc.
    #[serde(skip)]
    pub raw_global_streams: Vec<(String, Vec<u8>)>,
}

impl PcbLib {
    /// Open and parse a PcbLib CFB file.
    pub fn open<R: Read + io::Seek>(reader: R) -> io::Result<Self> {
        let mut cfb = cfb::CompoundFile::open(reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Read SectionKeys mapping (pattern name → storage name)
        let section_keys = read_section_keys(&mut cfb);

        // Read Library/Data TOC to get footprint names
        let footprint_names = read_library_toc(&mut cfb)?;

        // Collect all footprint storage paths for excluding from global streams
        let mut footprint_storage_paths: Vec<String> = Vec::new();

        let mut footprints = Vec::new();
        for name in &footprint_names {
            // Resolve storage path via SectionKeys, falling back to name itself
            let storage_key = section_keys
                .get(name)
                .map(|s| s.as_str())
                .unwrap_or(name);
            let fp = read_footprint(&mut cfb, name, storage_key)?;
            // Track the storage path used
            let storage_path = resolve_storage_path(&mut cfb, storage_key);
            footprint_storage_paths.push(storage_path);
            footprints.push(fp);
        }

        // Collect all global (non-footprint) streams for lossless roundtrip
        let all_entries: Vec<(String, bool)> = cfb
            .walk()
            .map(|e| (e.path().to_string_lossy().to_string(), e.is_stream()))
            .collect();

        let mut raw_global_streams = Vec::new();
        for (path, is_stream) in &all_entries {
            if !is_stream {
                continue;
            }
            // Skip footprint-specific streams (we store them per-footprint)
            let is_footprint_stream = footprint_storage_paths.iter().any(|sp| {
                path.starts_with(&format!("{}\\", sp.trim_start_matches('/')))
                    || path.starts_with(&format!("{}/", sp.trim_start_matches('/')))
                    || path.starts_with(sp)
            });
            if is_footprint_stream {
                continue;
            }
            if let Ok(data) = read_cfb_stream(&mut cfb, &path.replace('\\', "/")) {
                raw_global_streams.push((path.replace('\\', "/"), data));
            }
        }

        Ok(PcbLib {
            footprints,
            raw_global_streams,
        })
    }

    /// Write a PcbLib to a CFB compound file.
    pub fn write<W: Read + io::Write + io::Seek>(&self, writer: W) -> io::Result<()> {
        let mut cfb = cfb::CompoundFile::create(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // 1. Write global streams (Library/*, SectionKeys, etc.)
        for (path, data) in &self.raw_global_streams {
            // Ensure parent storages exist
            ensure_parent_storages(&mut cfb, path)?;
            let mut stream = cfb.create_stream(path)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("create stream {}: {}", path, e)))?;
            io::Write::write_all(&mut stream, data)?;
        }

        // 2. Write each footprint's raw streams
        for fp in &self.footprints {
            if fp.raw_streams.is_empty() {
                continue;
            }

            // Determine storage name from raw_streams (they were stored with the key)
            let storage_name = fp.raw_streams.get("__storage_path")
                .map(|v| String::from_utf8_lossy(v).to_string())
                .unwrap_or_else(|| format!("/{}", fp.name));

            // Create storage
            cfb.create_storage(&storage_name)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("create storage {}: {}", storage_name, e)))?;

            // Write each stream
            for (stream_name, data) in &fp.raw_streams {
                if stream_name == "__storage_path" {
                    continue;
                }
                let stream_path = format!("{}/{}", storage_name, stream_name);
                let mut stream = cfb.create_stream(&stream_path)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("create stream {}: {}", stream_path, e)))?;
                io::Write::write_all(&mut stream, data)?;
            }
        }

        cfb.flush()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("CFB flush: {}", e)))?;
        Ok(())
    }
}

/// Read the SectionKeys stream to map pattern names → storage names.
///
/// CFB entry names have a 31-character limit, so long footprint names get
/// truncated storage names. SectionKeys stores the mapping.
fn read_section_keys<F: Read + io::Seek>(cfb: &mut cfb::CompoundFile<F>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let data = match read_cfb_stream(cfb, "/SectionKeys") {
        Ok(d) => d,
        Err(_) => return map,
    };
    let text = String::from_utf8_lossy(&data);
    // Format: pipe-delimited key=value pairs
    for segment in text.split('|') {
        if segment.is_empty() {
            continue;
        }
        if let Some((key, value)) = segment.split_once('=') {
            // key = pattern name, value = storage name
            map.insert(key.to_string(), value.to_string());
        }
    }
    map
}

/// Read the Library/Data TOC to get footprint names.
fn read_library_toc<F: Read + io::Seek>(cfb: &mut cfb::CompoundFile<F>) -> io::Result<Vec<String>> {
    let data = read_cfb_stream(cfb, "/Library/Data")?;
    if data.len() < 4 {
        return Ok(Vec::new());
    }

    let mut cursor = Cursor::new(&data);

    // First: parametric header block
    let _header_text = streams::read_parametric_block(&mut cursor)?;

    // Then: u32 footprint count
    let mut count_buf = [0u8; 4];
    cursor.read_exact(&mut count_buf)?;
    let count = u32::from_le_bytes(count_buf) as usize;

    // Then: array of string blocks (u32 block_len + u8 str_len + string bytes)
    // The block starts with a u8 Pascal-style length prefix.
    let mut names = Vec::with_capacity(count);
    for _ in 0..count {
        let mut len_buf = [0u8; 4];
        cursor.read_exact(&mut len_buf)?;
        let block_len = u32::from_le_bytes(len_buf) as usize;
        let mut block_data = vec![0u8; block_len];
        cursor.read_exact(&mut block_data)?;
        // Skip the leading u8 string-length byte
        let name_bytes = if !block_data.is_empty() {
            &block_data[1..]
        } else {
            &block_data[..]
        };
        // Trim trailing null if present
        let name_bytes = if name_bytes.last() == Some(&0) {
            &name_bytes[..name_bytes.len() - 1]
        } else {
            name_bytes
        };
        names.push(String::from_utf8_lossy(name_bytes).into_owned());
    }

    Ok(names)
}

/// Read a single footprint from the PcbLib.
///
/// `name` is the full footprint name (for the result struct).
/// `storage_key` is the CFB storage name (possibly truncated).
fn read_footprint<F: Read + io::Seek>(
    cfb: &mut cfb::CompoundFile<F>,
    name: &str,
    storage_key: &str,
) -> io::Result<PcbLibFootprint> {
    let mut fp = PcbLibFootprint {
        name: name.to_string(),
        ..Default::default()
    };

    // Build the storage path, handling names with forward slashes
    let storage_path = resolve_storage_path(cfb, storage_key);

    // Header: u32 primitive count
    if let Ok(header_data) = read_cfb_stream(cfb, &format!("{}/Header", storage_path)) {
        if header_data.len() >= 4 {
            fp.primitive_count = u32::from_le_bytes(header_data[0..4].try_into().unwrap());
        }
    }

    // Data: pattern name string block + mixed binary primitive records
    if let Ok(data) = read_cfb_stream(cfb, &format!("{}/Data", storage_path)) {
        let mut cursor = Cursor::new(&data);

        // First block is the pattern name (u32 len + string)
        let mut len_buf = [0u8; 4];
        if cursor.read_exact(&mut len_buf).is_ok() {
            let str_len = u32::from_le_bytes(len_buf) as usize;
            // Skip the pattern name string
            cursor.set_position(cursor.position() + str_len as u64);
        }

        // Then: mixed binary primitive records
        while cursor.position() < data.len() as u64 {
            let saved_pos = cursor.position();
            match read_next_primitive(&mut cursor, &mut fp) {
                Ok(()) => {}
                Err(_) => {
                    // If we didn't advance, we're stuck — break
                    if cursor.position() == saved_pos {
                        break;
                    }
                    // Otherwise the cursor advanced past the bad record, continue
                }
            }
        }
    }

    // Parameters
    if let Ok(param_data) = read_cfb_stream(cfb, &format!("{}/Parameters", storage_path)) {
        let text = String::from_utf8_lossy(&param_data);
        fp.parameters = crate::v2::pcb::region::parse_parametric(text.trim_end_matches('\0'));
    }

    Ok(fp)
}

/// Read the next primitive record from the cursor into the footprint.
///
/// Each record is framed as `u8 type + u32 len + data`, except:
/// - Pad: `u8 type` + 6 subrecords (each with own u32 len prefix)
/// - Text: `u8 type` + 2 subrecords (each with own u32 len prefix)
fn read_next_primitive(cursor: &mut Cursor<&Vec<u8>>, fp: &mut PcbLibFootprint) -> io::Result<()> {
    let mut type_buf = [0u8; 1];
    cursor.read_exact(&mut type_buf)?;
    let type_byte = type_buf[0];

    match PcbObjectId::from_u8(type_byte) {
        Some(PcbObjectId::Pad) => {
            // Pad: 6 subrecords directly on the stream
            let pad = PcbPad::read_from(cursor)?;
            fp.pads.push(pad);
        }
        Some(PcbObjectId::Text) => {
            // Text: 2 subrecords directly on the stream
            let text = PcbText::read_from(cursor)?;
            fp.texts.push(text);
        }
        Some(obj_id) => {
            // Standard framing: u32 len + data
            let mut len_buf = [0u8; 4];
            cursor.read_exact(&mut len_buf)?;
            let len = u32::from_le_bytes(len_buf) as usize;
            let mut block = vec![0u8; len];
            cursor.read_exact(&mut block)?;

            match obj_id {
                PcbObjectId::Track => {
                    if let Ok(track) = PcbTrack::read_from(&mut Cursor::new(&block)) {
                        fp.tracks.push(track);
                    }
                }
                PcbObjectId::Arc => {
                    if let Ok(arc) = PcbArc::read_from(&mut Cursor::new(&block)) {
                        fp.arcs.push(arc);
                    }
                }
                PcbObjectId::Fill => {
                    if let Ok(fill) = PcbFill::read_from(&mut Cursor::new(&block)) {
                        fp.fills.push(fill);
                    }
                }
                PcbObjectId::Via => {
                    if let Ok(via) = PcbVia::from_bytes(&block) {
                        fp.vias.push(via);
                    }
                }
                PcbObjectId::Region => {
                    if let Ok(region) = PcbRegion::read_from(&block) {
                        fp.regions.push(region);
                    }
                }
                PcbObjectId::ComponentBody => {
                    if let Ok(body) = PcbRegion::read_from(&block) {
                        fp.component_bodies.push(body);
                    }
                }
                _ => {} // Skip other known types
            }
        }
        None => {
            // Unknown type — skip by reading u32 len + data
            let mut len_buf = [0u8; 4];
            cursor.read_exact(&mut len_buf)?;
            let len = u32::from_le_bytes(len_buf) as usize;
            let pos = cursor.position();
            cursor.set_position(pos + len as u64);
        }
    }
    Ok(())
}

/// Resolve the CFB storage path for a footprint, with fallback for names containing '/'.
fn resolve_storage_path<F: Read + io::Seek>(
    cfb: &mut cfb::CompoundFile<F>,
    storage_key: &str,
) -> String {
    let path = format!("/{}", storage_key);
    // Check if the storage entry exists
    if cfb.is_storage(&path) {
        return path;
    }
    // Fallback: replace forward slashes with underscores (Altium convention)
    let alt_path = format!("/{}", storage_key.replace('/', "_"));
    if cfb.is_storage(&alt_path) {
        return alt_path;
    }
    // Return original path even if it doesn't exist — reads will fail gracefully
    path
}

fn read_cfb_stream<F: Read + io::Seek>(cfb: &mut cfb::CompoundFile<F>, path: &str) -> io::Result<Vec<u8>> {
    let mut stream = cfb.open_stream(path)
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e))?;
    let mut data = Vec::new();
    stream.read_to_end(&mut data)?;
    Ok(data)
}

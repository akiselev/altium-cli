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
use std::collections::{HashMap, HashSet};
use std::io::{self, Cursor, Read, Write};

use super::streams;
use crate::v2::pcb::arc::PcbArc;
use crate::v2::pcb::fill::PcbFill;
use crate::v2::pcb::pad::PcbPad;
use crate::v2::pcb::primitive::PcbObjectId;
use crate::v2::pcb::region::{serialize_parametric, PcbRegion};
use crate::v2::pcb::text::PcbText;
use crate::v2::pcb::track::PcbTrack;
use crate::v2::pcb::via::PcbVia;

/// Identifies a primitive in the ordering list by type and index within its type vector.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PcbPrimitiveRef {
    Track(usize),
    Arc(usize),
    Fill(usize),
    Pad(usize),
    Via(usize),
    Text(usize),
    Region(usize),
    ComponentBody(usize),
    /// Unknown or failed-to-parse primitive, stored as raw bytes (type byte + framed data).
    Raw(usize),
}

/// A single footprint in a PcbLib.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PcbLibFootprint {
    pub name: String,
    /// CFB storage path (e.g., "/H", "/RADIAL CAPACITOR - 100uF - Old ").
    #[serde(skip)]
    pub storage_path: String,
    pub primitive_count: u32,
    pub tracks: Vec<PcbTrack>,
    pub arcs: Vec<PcbArc>,
    pub fills: Vec<PcbFill>,
    pub pads: Vec<PcbPad>,
    pub vias: Vec<PcbVia>,
    pub texts: Vec<PcbText>,
    pub regions: Vec<PcbRegion>,
    pub component_bodies: Vec<PcbRegion>,
    /// Raw primitives that failed to parse or have unknown type bytes.
    #[serde(skip)]
    pub raw_primitives: Vec<Vec<u8>>,
    /// Original primitive ordering — preserves the interleaved sequence from the file.
    #[serde(skip)]
    pub primitive_order: Vec<PcbPrimitiveRef>,
    /// Parametric properties for this footprint.
    pub parameters: HashMap<String, String>,
    /// Raw Parameters stream bytes for lossless roundtrip.
    #[serde(skip)]
    pub raw_parameters: Vec<u8>,
    /// The raw pattern name block from the start of the Data stream.
    #[serde(skip)]
    pub raw_pattern_name_block: Vec<u8>,
}

/// A parsed PcbLib file.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PcbLib {
    pub footprints: Vec<PcbLibFootprint>,
    /// All raw CFB streams for lossless roundtrip.
    /// Keys are full paths like "/Library/Data", "/SectionKeys", "/{footprint}/Data", etc.
    /// Order is preserved for deterministic output.
    #[serde(skip)]
    pub raw_streams: Vec<(String, Vec<u8>)>,
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

        let mut footprints = Vec::new();
        for name in &footprint_names {
            // Resolve storage path via SectionKeys, falling back to name itself
            let storage_key = section_keys
                .get(name)
                .map(|s| s.as_str())
                .unwrap_or(name);
            let fp = read_footprint(&mut cfb, name, storage_key)?;
            footprints.push(fp);
        }

        // Collect ALL streams for lossless roundtrip
        let all_entries: Vec<(String, bool)> = cfb
            .walk()
            .map(|e| (e.path().to_string_lossy().replace('\\', "/"), e.is_stream()))
            .collect();

        let mut raw_streams = Vec::new();
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
                raw_streams.push((normalized, data));
            }
        }

        Ok(PcbLib {
            footprints,
            raw_streams,
        })
    }

    /// Write a PcbLib to a CFB compound file, serializing from typed fields.
    ///
    /// Per-footprint streams (Data, Header, Parameters) are rebuilt from the
    /// typed fields. All other streams (Library/Data, SectionKeys, FileHeader,
    /// EmbeddedFonts, etc.) are written verbatim from `raw_streams`.
    ///
    /// Storage names that exceed CFB's 31-character limit are automatically
    /// truncated, and a SectionKeys stream is generated to preserve the mapping.
    pub fn write<W: Read + io::Write + io::Seek>(&self, writer: W) -> io::Result<()> {
        let mut cfb = cfb::CompoundFile::create(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Build name remapping for storage paths that exceed CFB's 31-char limit.
        let (path_remap, section_keys_data) = build_name_remap(&self.footprints, &self.raw_streams);

        // Build set of OLD paths we'll rebuild from footprint types
        let mut rebuilt_paths: HashSet<String> = HashSet::new();
        for fp in &self.footprints {
            if !fp.storage_path.is_empty() {
                rebuilt_paths.insert(format!("{}/Data", fp.storage_path));
                rebuilt_paths.insert(format!("{}/Header", fp.storage_path));
                rebuilt_paths.insert(format!("{}/Parameters", fp.storage_path));
            }
        }
        // Also skip the old SectionKeys — we'll write a new one
        rebuilt_paths.insert("/SectionKeys".to_string());

        // Write all raw streams we DON'T rebuild, remapping paths as needed
        for (path, data) in &self.raw_streams {
            if rebuilt_paths.contains(path.as_str()) {
                continue;
            }
            let write_path = remap_path(path, &path_remap);
            ensure_parent_storages(&mut cfb, &write_path)?;
            let mut stream = cfb.create_stream(&write_path)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("create stream {}: {}", write_path, e)))?;
            io::Write::write_all(&mut stream, data)?;
        }

        // Rebuild per-footprint streams from types
        for fp in &self.footprints {
            if fp.storage_path.is_empty() {
                continue;
            }

            let write_storage = path_remap
                .get(&fp.storage_path)
                .cloned()
                .unwrap_or_else(|| fp.storage_path.clone());

            // Header: u32 primitive count (use original count if available)
            let header_path = format!("{}/Header", write_storage);
            ensure_parent_storages(&mut cfb, &header_path)?;
            let count = if !fp.primitive_order.is_empty() {
                fp.primitive_order.len()
            } else {
                fp.tracks.len() + fp.arcs.len() + fp.fills.len()
                    + fp.pads.len() + fp.vias.len() + fp.texts.len()
                    + fp.regions.len() + fp.component_bodies.len()
                    + fp.raw_primitives.len()
            };
            let mut hdr_stream = cfb.create_stream(&header_path)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("create {}: {}", header_path, e)))?;
            hdr_stream.write_all(&(count as u32).to_le_bytes())?;

            // Data: pattern name block + mixed primitive records
            let data_path = format!("{}/Data", write_storage);
            let data = build_footprint_data(fp)?;
            let mut data_stream = cfb.create_stream(&data_path)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("create {}: {}", data_path, e)))?;
            data_stream.write_all(&data)?;

            // Parameters: write raw bytes if available, otherwise serialize from HashMap
            let params_path = format!("{}/Parameters", write_storage);
            let params_bytes = if !fp.raw_parameters.is_empty() {
                fp.raw_parameters.clone()
            } else {
                let params_text = serialize_parametric(&fp.parameters);
                let mut bytes = params_text.into_bytes();
                bytes.push(0); // null terminator
                bytes
            };
            let mut params_stream = cfb.create_stream(&params_path)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("create {}: {}", params_path, e)))?;
            params_stream.write_all(&params_bytes)?;
        }

        // Write SectionKeys stream
        {
            let mut sk_stream = cfb.create_stream("/SectionKeys")
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("create SectionKeys: {}", e)))?;
            sk_stream.write_all(&section_keys_data)?;
        }

        cfb.flush()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("CFB flush: {}", e)))?;
        Ok(())
    }
}

/// Build a name remapping table for CFB storage paths that exceed the 31-char limit.
///
/// Returns `(path_remap, section_keys_data)`:
/// - `path_remap`: maps old storage paths to truncated storage paths (only for paths > 31 chars).
/// - `section_keys_data`: the binary SectionKeys stream content.
///
/// If the original raw_streams contain a SectionKeys and no new remapping is needed,
/// the original bytes are returned unchanged for lossless roundtrip.
fn build_name_remap(
    footprints: &[PcbLibFootprint],
    raw_streams: &[(String, Vec<u8>)],
) -> (HashMap<String, String>, Vec<u8>) {
    // Check if any storage paths need truncation
    let needs_remap = footprints.iter().any(|fp| {
        let storage_name = fp.storage_path.trim_start_matches('/');
        storage_name.encode_utf16().count() > 31
    });

    if !needs_remap {
        // No remapping needed — use original SectionKeys if available
        let original_sk = raw_streams.iter()
            .find(|(path, _)| path == "/SectionKeys")
            .map(|(_, data)| data.clone())
            .unwrap_or_else(|| build_empty_section_keys());
        return (HashMap::new(), original_sk);
    }

    // Need remapping: storage paths already have original truncated names from SectionKeys.
    // This means fp.storage_path is already the truncated path (≤31 chars).
    // Only footprints that STILL exceed 31 chars need new truncation (shouldn't happen
    // if read_section_keys worked, but handle it for safety).
    let mut path_remap: HashMap<String, String> = HashMap::new();
    let mut used_names: HashSet<String> = HashSet::new();
    let mut section_keys_entries: Vec<(String, String)> = Vec::new();

    // Collect all top-level storage names that are already ≤31 chars
    for fp in footprints {
        let storage_name = fp.storage_path.trim_start_matches('/');
        if storage_name.encode_utf16().count() <= 31 {
            used_names.insert(storage_name.to_uppercase());
        }
    }
    for (path, _) in raw_streams {
        let first_part = path.trim_start_matches('/').split('/').next().unwrap_or("");
        if first_part.encode_utf16().count() <= 31 {
            used_names.insert(first_part.to_uppercase());
        }
    }

    for fp in footprints {
        let storage_name = fp.storage_path.trim_start_matches('/');
        if storage_name.encode_utf16().count() <= 31 {
            continue;
        }

        let truncated = truncate_to_utf16_len(storage_name, 28);
        let mut candidate = truncated.clone();
        let mut suffix = 0u32;

        while used_names.contains(&candidate.to_uppercase()) {
            suffix += 1;
            let suffix_str = format!("_{}", suffix);
            let base = truncate_to_utf16_len(storage_name, 31 - suffix_str.len());
            candidate = format!("{}{}", base, suffix_str);
        }

        used_names.insert(candidate.to_uppercase());
        let new_path = format!("/{}", candidate);
        path_remap.insert(fp.storage_path.clone(), new_path);
        section_keys_entries.push((fp.name.clone(), candidate));
    }

    let sk_data = build_section_keys_binary(&section_keys_entries);
    (path_remap, sk_data)
}

/// Build an empty SectionKeys stream (0 entries).
fn build_empty_section_keys() -> Vec<u8> {
    0u32.to_le_bytes().to_vec()
}

/// Build a binary SectionKeys stream from (full_name, truncated_name) entries.
///
/// Format:
/// ```text
/// u32: entry_count
/// For each entry:
///   u32 block_len + u8 pascal_len + string_bytes  (full pattern name)
///   u32 block_len + u8 pascal_len + string_bytes  (truncated storage name)
/// ```
fn build_section_keys_binary(entries: &[(String, String)]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for (full_name, truncated_name) in entries {
        write_pascal_block(&mut data, full_name);
        write_pascal_block(&mut data, truncated_name);
    }
    data
}

/// Write a Pascal-style string block: u32 block_len + u8 pascal_len + string bytes.
fn write_pascal_block(data: &mut Vec<u8>, s: &str) {
    let str_bytes = s.as_bytes();
    let block_len = 1 + str_bytes.len(); // 1 for pascal length byte
    data.extend_from_slice(&(block_len as u32).to_le_bytes());
    data.push(str_bytes.len() as u8);
    data.extend_from_slice(str_bytes);
}

/// Truncate a string to fit within `max_utf16` UTF-16 code units.
fn truncate_to_utf16_len(s: &str, max_utf16: usize) -> String {
    let mut result = String::new();
    let mut utf16_count = 0;
    for ch in s.chars() {
        let ch_len = ch.len_utf16();
        if utf16_count + ch_len > max_utf16 {
            break;
        }
        utf16_count += ch_len;
        result.push(ch);
    }
    result
}

/// Remap a CFB stream path using the storage path remap table.
///
/// For example, if path_remap maps "/LONG NAME" → "/SHORT", then:
/// - "/LONG NAME/Data" → "/SHORT/Data"
/// - "/LONG NAME/Header" → "/SHORT/Header"
/// - "/Other/Data" → "/Other/Data" (unchanged)
fn remap_path(path: &str, path_remap: &HashMap<String, String>) -> String {
    for (old_prefix, new_prefix) in path_remap {
        if path == old_prefix || path.starts_with(&format!("{}/", old_prefix)) {
            return format!("{}{}", new_prefix, &path[old_prefix.len()..]);
        }
    }
    path.to_string()
}

/// Build the Data stream for a footprint from its typed primitives.
///
/// Format: pattern name block + mixed binary primitive records.
/// Primitives are written in the original interleaved order from `primitive_order`.
fn build_footprint_data(fp: &PcbLibFootprint) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();

    // Pattern name block (u32 len + string)
    if !fp.raw_pattern_name_block.is_empty() {
        data.extend_from_slice(&fp.raw_pattern_name_block);
    } else {
        // Fallback: build from name
        let name_bytes = fp.name.as_bytes();
        data.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(name_bytes);
    }

    if !fp.primitive_order.is_empty() {
        // Write primitives in original interleaved order
        for pref in &fp.primitive_order {
            write_primitive_ref(&mut data, fp, pref)?;
        }
    } else {
        // Fallback for footprints created programmatically (no ordering info)
        write_all_primitives_grouped(&mut data, fp)?;
    }

    Ok(data)
}

/// Write a single primitive by reference, preserving original framing.
fn write_primitive_ref(data: &mut Vec<u8>, fp: &PcbLibFootprint, pref: &PcbPrimitiveRef) -> io::Result<()> {
    match pref {
        PcbPrimitiveRef::Track(idx) => {
            let mut record = Vec::new();
            fp.tracks[*idx].write_to(&mut record)?;
            streams::write_binary_block(data, PcbObjectId::Track as u8, &record)?;
        }
        PcbPrimitiveRef::Arc(idx) => {
            let mut record = Vec::new();
            fp.arcs[*idx].write_to(&mut record)?;
            streams::write_binary_block(data, PcbObjectId::Arc as u8, &record)?;
        }
        PcbPrimitiveRef::Fill(idx) => {
            let mut record = Vec::new();
            fp.fills[*idx].write_to(&mut record)?;
            streams::write_binary_block(data, PcbObjectId::Fill as u8, &record)?;
        }
        PcbPrimitiveRef::Pad(idx) => {
            data.push(PcbObjectId::Pad as u8);
            fp.pads[*idx].write_to(data)?;
        }
        PcbPrimitiveRef::Via(idx) => {
            let bytes = fp.vias[*idx].to_bytes();
            streams::write_binary_block(data, PcbObjectId::Via as u8, &bytes)?;
        }
        PcbPrimitiveRef::Text(idx) => {
            data.push(PcbObjectId::Text as u8);
            fp.texts[*idx].write_to(data)?;
        }
        PcbPrimitiveRef::Region(idx) => {
            let mut record = Vec::new();
            fp.regions[*idx].write_to(&mut record)?;
            streams::write_binary_block(data, PcbObjectId::Region as u8, &record)?;
        }
        PcbPrimitiveRef::ComponentBody(idx) => {
            let mut record = Vec::new();
            fp.component_bodies[*idx].write_to(&mut record)?;
            streams::write_binary_block(data, PcbObjectId::ComponentBody as u8, &record)?;
        }
        PcbPrimitiveRef::Raw(idx) => {
            // Write raw bytes verbatim (already includes type byte + framing)
            data.extend_from_slice(&fp.raw_primitives[*idx]);
        }
    }
    Ok(())
}

/// Fallback: write all primitives grouped by type (for programmatically created footprints).
fn write_all_primitives_grouped(data: &mut Vec<u8>, fp: &PcbLibFootprint) -> io::Result<()> {
    for track in &fp.tracks {
        let mut record = Vec::new();
        track.write_to(&mut record)?;
        streams::write_binary_block(data, PcbObjectId::Track as u8, &record)?;
    }
    for arc in &fp.arcs {
        let mut record = Vec::new();
        arc.write_to(&mut record)?;
        streams::write_binary_block(data, PcbObjectId::Arc as u8, &record)?;
    }
    for fill in &fp.fills {
        let mut record = Vec::new();
        fill.write_to(&mut record)?;
        streams::write_binary_block(data, PcbObjectId::Fill as u8, &record)?;
    }
    for pad in &fp.pads {
        data.push(PcbObjectId::Pad as u8);
        pad.write_to(data)?;
    }
    for via in &fp.vias {
        let bytes = via.to_bytes();
        streams::write_binary_block(data, PcbObjectId::Via as u8, &bytes)?;
    }
    for text in &fp.texts {
        data.push(PcbObjectId::Text as u8);
        text.write_to(data)?;
    }
    for region in &fp.regions {
        let mut record = Vec::new();
        region.write_to(&mut record)?;
        streams::write_binary_block(data, PcbObjectId::Region as u8, &record)?;
    }
    for body in &fp.component_bodies {
        let mut record = Vec::new();
        body.write_to(&mut record)?;
        streams::write_binary_block(data, PcbObjectId::ComponentBody as u8, &record)?;
    }
    for raw in &fp.raw_primitives {
        data.extend_from_slice(raw);
    }
    Ok(())
}

/// Read the SectionKeys stream to map pattern names → storage names.
///
/// CFB entry names have a 31-character limit, so long footprint names get
/// truncated storage names. SectionKeys stores the mapping in binary format:
///
/// ```text
/// u32: entry_count
/// For each entry:
///   u32 block_len + u8 pascal_len + string_bytes  (full pattern name)
///   u32 block_len + u8 pascal_len + string_bytes  (truncated storage name)
/// ```
fn read_section_keys<F: Read + io::Seek>(cfb: &mut cfb::CompoundFile<F>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let data = match read_cfb_stream(cfb, "/SectionKeys") {
        Ok(d) => d,
        Err(_) => return map,
    };
    if data.len() < 4 {
        return map;
    }

    let mut cursor = Cursor::new(&data);

    // u32 entry count
    let mut count_buf = [0u8; 4];
    if cursor.read_exact(&mut count_buf).is_err() {
        return map;
    }
    let count = u32::from_le_bytes(count_buf) as usize;

    for _ in 0..count {
        // Full name: u32 block_len + u8 pascal_len + string bytes
        let full_name = match read_pascal_block(&mut cursor) {
            Some(s) => s,
            None => break,
        };
        // Truncated storage name: same format
        let storage_name = match read_pascal_block(&mut cursor) {
            Some(s) => s,
            None => break,
        };
        map.insert(full_name, storage_name);
    }
    map
}

/// Read a Pascal-style string block: u32 block_len + u8 pascal_len + string bytes.
fn read_pascal_block(cursor: &mut Cursor<&Vec<u8>>) -> Option<String> {
    let mut len_buf = [0u8; 4];
    cursor.read_exact(&mut len_buf).ok()?;
    let block_len = u32::from_le_bytes(len_buf) as usize;
    if block_len == 0 {
        return Some(String::new());
    }
    let mut block_data = vec![0u8; block_len];
    cursor.read_exact(&mut block_data).ok()?;
    // Skip leading u8 string-length byte
    let str_bytes = if !block_data.is_empty() {
        &block_data[1..]
    } else {
        &block_data[..]
    };
    Some(String::from_utf8_lossy(str_bytes).into_owned())
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
    fp.storage_path = storage_path.clone();

    // Header: u32 primitive count
    let header_path = format!("{}/Header", storage_path);
    if let Ok(header_data) = read_cfb_stream(cfb, &header_path) {
        if header_data.len() >= 4 {
            fp.primitive_count = u32::from_le_bytes(header_data[0..4].try_into().unwrap());
        }
    }

    // Data: pattern name string block + mixed binary primitive records
    let data_path = format!("{}/Data", storage_path);
    if let Ok(data) = read_cfb_stream(cfb, &data_path) {
        let mut cursor = Cursor::new(&data);

        // First block is the pattern name (u32 len + string) — save it for write
        let mut len_buf = [0u8; 4];
        if cursor.read_exact(&mut len_buf).is_ok() {
            let str_len = u32::from_le_bytes(len_buf) as usize;
            let end = 4 + str_len;
            if end <= data.len() {
                fp.raw_pattern_name_block = data[..end].to_vec();
            }
            cursor.set_position(end as u64);
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
    let params_path = format!("{}/Parameters", storage_path);
    if let Ok(param_data) = read_cfb_stream(cfb, &params_path) {
        fp.raw_parameters = param_data.clone();
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
///
/// Tracks primitive ordering and preserves raw bytes for failed parses.
fn read_next_primitive(cursor: &mut Cursor<&Vec<u8>>, fp: &mut PcbLibFootprint) -> io::Result<()> {
    let record_start = cursor.position();
    let mut type_buf = [0u8; 1];
    cursor.read_exact(&mut type_buf)?;
    let type_byte = type_buf[0];

    match PcbObjectId::from_u8(type_byte) {
        Some(PcbObjectId::Pad) => {
            let before = cursor.position();
            match PcbPad::read_from(cursor) {
                Ok(pad) => {
                    let idx = fp.pads.len();
                    fp.pads.push(pad);
                    fp.primitive_order.push(PcbPrimitiveRef::Pad(idx));
                }
                Err(_) => {
                    let end = cursor.position().max(before);
                    cursor.set_position(record_start);
                    let raw_len = (end - record_start) as usize;
                    let mut raw = vec![0u8; raw_len];
                    cursor.read_exact(&mut raw)?;
                    let idx = fp.raw_primitives.len();
                    fp.raw_primitives.push(raw);
                    fp.primitive_order.push(PcbPrimitiveRef::Raw(idx));
                }
            }
        }
        Some(PcbObjectId::Text) => {
            let before = cursor.position();
            match PcbText::read_from(cursor) {
                Ok(text) => {
                    let idx = fp.texts.len();
                    fp.texts.push(text);
                    fp.primitive_order.push(PcbPrimitiveRef::Text(idx));
                }
                Err(_) => {
                    let end = cursor.position().max(before);
                    cursor.set_position(record_start);
                    let raw_len = (end - record_start) as usize;
                    let mut raw = vec![0u8; raw_len];
                    cursor.read_exact(&mut raw)?;
                    let idx = fp.raw_primitives.len();
                    fp.raw_primitives.push(raw);
                    fp.primitive_order.push(PcbPrimitiveRef::Raw(idx));
                }
            }
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
                    match PcbTrack::from_block(&block) {
                        Ok(track) => {
                            let idx = fp.tracks.len();
                            fp.tracks.push(track);
                            fp.primitive_order.push(PcbPrimitiveRef::Track(idx));
                        }
                        Err(_) => {
                            let mut raw = Vec::with_capacity(1 + 4 + block.len());
                            raw.push(type_byte);
                            raw.extend_from_slice(&len_buf);
                            raw.extend_from_slice(&block);
                            let idx = fp.raw_primitives.len();
                            fp.raw_primitives.push(raw);
                            fp.primitive_order.push(PcbPrimitiveRef::Raw(idx));
                        }
                    }
                }
                PcbObjectId::Arc => {
                    match PcbArc::from_block(&block) {
                        Ok(arc) => {
                            let idx = fp.arcs.len();
                            fp.arcs.push(arc);
                            fp.primitive_order.push(PcbPrimitiveRef::Arc(idx));
                        }
                        Err(_) => {
                            let mut raw = Vec::with_capacity(1 + 4 + block.len());
                            raw.push(type_byte);
                            raw.extend_from_slice(&len_buf);
                            raw.extend_from_slice(&block);
                            let idx = fp.raw_primitives.len();
                            fp.raw_primitives.push(raw);
                            fp.primitive_order.push(PcbPrimitiveRef::Raw(idx));
                        }
                    }
                }
                PcbObjectId::Fill => {
                    match PcbFill::from_block(&block) {
                        Ok(fill) => {
                            let idx = fp.fills.len();
                            fp.fills.push(fill);
                            fp.primitive_order.push(PcbPrimitiveRef::Fill(idx));
                        }
                        Err(_) => {
                            let mut raw = Vec::with_capacity(1 + 4 + block.len());
                            raw.push(type_byte);
                            raw.extend_from_slice(&len_buf);
                            raw.extend_from_slice(&block);
                            let idx = fp.raw_primitives.len();
                            fp.raw_primitives.push(raw);
                            fp.primitive_order.push(PcbPrimitiveRef::Raw(idx));
                        }
                    }
                }
                PcbObjectId::Via => {
                    match PcbVia::from_bytes(&block) {
                        Ok(via) => {
                            let idx = fp.vias.len();
                            fp.vias.push(via);
                            fp.primitive_order.push(PcbPrimitiveRef::Via(idx));
                        }
                        Err(_) => {
                            let mut raw = Vec::with_capacity(1 + 4 + block.len());
                            raw.push(type_byte);
                            raw.extend_from_slice(&len_buf);
                            raw.extend_from_slice(&block);
                            let idx = fp.raw_primitives.len();
                            fp.raw_primitives.push(raw);
                            fp.primitive_order.push(PcbPrimitiveRef::Raw(idx));
                        }
                    }
                }
                PcbObjectId::Region => {
                    match PcbRegion::read_from(&block) {
                        Ok(region) => {
                            let idx = fp.regions.len();
                            fp.regions.push(region);
                            fp.primitive_order.push(PcbPrimitiveRef::Region(idx));
                        }
                        Err(_) => {
                            let mut raw = Vec::with_capacity(1 + 4 + block.len());
                            raw.push(type_byte);
                            raw.extend_from_slice(&len_buf);
                            raw.extend_from_slice(&block);
                            let idx = fp.raw_primitives.len();
                            fp.raw_primitives.push(raw);
                            fp.primitive_order.push(PcbPrimitiveRef::Raw(idx));
                        }
                    }
                }
                PcbObjectId::ComponentBody => {
                    match PcbRegion::read_from(&block) {
                        Ok(body) => {
                            let idx = fp.component_bodies.len();
                            fp.component_bodies.push(body);
                            fp.primitive_order.push(PcbPrimitiveRef::ComponentBody(idx));
                        }
                        Err(_) => {
                            let mut raw = Vec::with_capacity(1 + 4 + block.len());
                            raw.push(type_byte);
                            raw.extend_from_slice(&len_buf);
                            raw.extend_from_slice(&block);
                            let idx = fp.raw_primitives.len();
                            fp.raw_primitives.push(raw);
                            fp.primitive_order.push(PcbPrimitiveRef::Raw(idx));
                        }
                    }
                }
                _ => {
                    // Known enum variant without a handler — store raw
                    let mut raw = Vec::with_capacity(1 + 4 + block.len());
                    raw.push(type_byte);
                    raw.extend_from_slice(&len_buf);
                    raw.extend_from_slice(&block);
                    let idx = fp.raw_primitives.len();
                    fp.raw_primitives.push(raw);
                    fp.primitive_order.push(PcbPrimitiveRef::Raw(idx));
                }
            }
        }
        None => {
            // Unknown type — read u32 len + data and store raw
            let mut len_buf = [0u8; 4];
            cursor.read_exact(&mut len_buf)?;
            let len = u32::from_le_bytes(len_buf) as usize;
            let mut block = vec![0u8; len];
            cursor.read_exact(&mut block)?;
            let mut raw = Vec::with_capacity(1 + 4 + block.len());
            raw.push(type_byte);
            raw.extend_from_slice(&len_buf);
            raw.extend_from_slice(&block);
            let idx = fp.raw_primitives.len();
            fp.raw_primitives.push(raw);
            fp.primitive_order.push(PcbPrimitiveRef::Raw(idx));
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

/// Ensure all parent storages exist for a given stream path.
fn ensure_parent_storages<F: Read + io::Write + io::Seek>(
    cfb: &mut cfb::CompoundFile<F>,
    path: &str,
) -> io::Result<()> {
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    // Create each parent storage (all parts except the last, which is the stream name)
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

fn read_cfb_stream<F: Read + io::Seek>(cfb: &mut cfb::CompoundFile<F>, path: &str) -> io::Result<Vec<u8>> {
    let mut stream = cfb.open_stream(path)
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e))?;
    let mut data = Vec::new();
    stream.read_to_end(&mut data)?;
    Ok(data)
}

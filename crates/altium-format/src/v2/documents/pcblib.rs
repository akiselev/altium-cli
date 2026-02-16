//! PcbLib document I/O using the v2 backing-store architecture.
//!
//! A PcbLib file is a CFB compound file with one storage per footprint:
//! - `/<FootprintName>/Parameters` stream: footprint metadata (pipe-delimited)
//! - `/<FootprintName>/Header` stream: primitive count and version info
//! - `/<FootprintName>/Data` stream: binary primitives (type byte + length + data)
//!
//! The Data stream begins with a length-prefixed pattern name block, followed
//! by packed binary primitive records.

use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};

use crate::error::{AltiumError, Result};
use crate::v2::backing_store::{
    BinaryOrigin, FootprintGroup, ParamOrigin, PcbPrimitiveRef, RecordNode,
    RecordOrigin,
};

use super::section_keys::SectionKeyList;

const STREAM_PARAMETERS: &str = "Parameters";
const STREAM_HEADER: &str = "Header";
const STREAM_DATA: &str = "Data";

/// A parsed PcbLib document using the v2 backing-store architecture.
///
/// Each footprint is a `FootprintGroup` containing metadata, binary primitives,
/// and raw blocks for identity write-back.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PcbLib {
    /// Footprint groups (one per footprint pattern).
    pub footprints: Vec<FootprintGroup>,
    /// Footprint storage names (parallel to `footprints`).
    pub footprint_names: Vec<String>,
    /// Section key mappings (for long footprint names).
    #[serde(skip)]
    pub section_keys: SectionKeyList,
    /// Library-level extra CFB streams (FileHeader, Library/*, etc.),
    /// preserved for round-trip.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub raw_extra_streams: HashMap<String, Vec<u8>>,
}

impl PcbLib {
    /// Open a PcbLib from a reader.
    pub fn open<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut cfb = cfb::CompoundFile::open(reader)
            .map_err(|e| AltiumError::Cfb(format!("Failed to open CFB: {}", e)))?;

        let mut lib = PcbLib::default();

        // Read section keys (if any)
        lib.section_keys = read_pcb_section_keys(&mut cfb)?;

        // Enumerate top-level storages in the CFB to find footprints.
        // We collect the entries first because walk() borrows cfb immutably,
        // and we need mutable access later to open streams.
        let entries: Vec<String> = cfb
            .walk()
            .filter(|e| {
                e.is_storage()
                    && e.path()
                        .parent()
                        .map_or(false, |p| p == std::path::Path::new("/"))
            })
            .filter_map(|e| {
                let name = e.path().file_name()?.to_str()?.to_string();
                // Skip system streams/storages
                if name == "SectionKeys"
                    || name == "FileHeader"
                    || name == "Library"
                {
                    return None;
                }
                Some(name)
            })
            .collect();

        // Collect all stream paths for capturing extra streams per footprint
        let all_stream_paths: Vec<String> = cfb
            .walk()
            .filter(|e| e.is_stream())
            .filter_map(|e| Some(e.path().to_str()?.to_string()))
            .collect();

        for storage_name in &entries {
            // Read Parameters stream (footprint metadata)
            let params_path = format!("/{}/{}", storage_name, STREAM_PARAMETERS);
            let metadata = if let Ok(mut stream) = cfb.open_stream(&params_path) {
                let mut data = Vec::new();
                stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
                let param_str = String::from_utf8_lossy(&data).to_string();
                let origin =
                    RecordOrigin::Param(ParamOrigin::new(&param_str));
                RecordNode::new(0, origin)
            } else {
                RecordNode::new(
                    0,
                    RecordOrigin::Param(ParamOrigin::new("|PATTERN=|")),
                )
            };

            // Read Header stream (primitive count / version info)
            let header_path = format!("/{}/{}", storage_name, STREAM_HEADER);
            let raw_header = if let Ok(mut stream) = cfb.open_stream(&header_path) {
                let mut data = Vec::new();
                stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
                data
            } else {
                Vec::new()
            };

            // Read Data stream (pattern name block + binary primitives)
            let data_path = format!("/{}/{}", storage_name, STREAM_DATA);
            let (primitives, primitive_order, raw_pattern_name) =
                if let Ok(mut stream) = cfb.open_stream(&data_path) {
                    let mut data = Vec::new();
                    stream.read_to_end(&mut data).map_err(AltiumError::Io)?;
                    parse_pcb_data_stream(&data)?
                } else {
                    (Vec::new(), Vec::new(), Vec::new())
                };

            // Capture extra streams in this footprint's storage
            let storage_prefix = format!("/{}/", storage_name);
            let mut extra_streams = HashMap::new();
            for stream_path in &all_stream_paths {
                if let Some(rest) = stream_path.strip_prefix(&storage_prefix) {
                    if rest == STREAM_PARAMETERS
                        || rest == STREAM_HEADER
                        || rest == STREAM_DATA
                    {
                        continue;
                    }
                    if let Ok(mut stream) = cfb.open_stream(stream_path) {
                        let mut data = Vec::new();
                        if stream.read_to_end(&mut data).is_ok() {
                            extra_streams.insert(rest.to_string(), data);
                        }
                    }
                }
            }

            lib.footprint_names.push(storage_name.clone());
            let mut group = FootprintGroup::new(
                metadata,
                primitives,
                raw_pattern_name,
                primitive_order,
                raw_header,
            );
            group.raw_extra_streams = extra_streams;
            lib.footprints.push(group);
        }

        // Capture library-level extra streams (top-level streams/storages
        // that aren't footprint storages)
        let footprint_set: std::collections::HashSet<&str> =
            entries.iter().map(|s| s.as_str()).collect();
        for stream_path in &all_stream_paths {
            // Extract top-level component: /Name or /Name/Child
            let path_no_slash = stream_path.trim_start_matches('/');
            let top_level = path_no_slash.split('/').next().unwrap_or("");
            if footprint_set.contains(top_level) {
                continue; // Already handled per-footprint
            }
            if let Ok(mut stream) = cfb.open_stream(stream_path) {
                let mut data = Vec::new();
                if stream.read_to_end(&mut data).is_ok() {
                    // Store with leading slash stripped for consistent naming
                    lib.raw_extra_streams
                        .insert(path_no_slash.to_string(), data);
                }
            }
        }

        Ok(lib)
    }

    /// Open a PcbLib from a file path.
    pub fn open_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let file = std::fs::File::open(path).map_err(AltiumError::Io)?;
        Self::open(file)
    }

    /// Save a PcbLib to a writer.
    pub fn save<W: Read + Write + Seek>(&self, writer: W) -> Result<()> {
        let mut cfb = cfb::CompoundFile::create(writer)
            .map_err(|e| AltiumError::Cfb(format!("Failed to create CFB: {}", e)))?;

        // Write library-level extra streams first (FileHeader, Library/*, etc.)
        // We need to create any parent storages for nested paths.
        {
            let mut created_storages: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            let mut sorted_extras: Vec<_> = self.raw_extra_streams.iter().collect();
            sorted_extras.sort_by_key(|(k, _)| (*k).clone());
            for (rel_path, data) in &sorted_extras {
                let full_path = format!("/{}", rel_path);
                // Create parent storage if it contains a slash (e.g. "Library/Data")
                if let Some(slash_pos) = rel_path.find('/') {
                    let parent = &rel_path[..slash_pos];
                    let parent_path = format!("/{}", parent);
                    if created_storages.insert(parent_path.clone()) {
                        let _ = cfb.create_storage(&parent_path);
                    }
                }
                if let Ok(mut stream) = cfb.create_stream(&full_path) {
                    let _ = stream.write_all(data);
                }
            }
        }

        for (i, group) in self.footprints.iter().enumerate() {
            let name = &self.footprint_names[i];
            let storage_path = format!("/{}", name);
            cfb.create_storage(&storage_path).map_err(|e| {
                AltiumError::Cfb(format!("Failed to create storage: {}", e))
            })?;

            // Write Parameters stream
            let params_path = format!("/{}/{}", name, STREAM_PARAMETERS);
            let params_data = match &group.metadata.origin {
                RecordOrigin::Param(p) => p.params.to_param_string().into_bytes(),
                _ => Vec::new(),
            };
            let mut stream = cfb.create_stream(&params_path).map_err(|e| {
                AltiumError::Cfb(format!("Failed to create Parameters: {}", e))
            })?;
            stream.write_all(&params_data).map_err(AltiumError::Io)?;

            // Write Header stream
            let header_path = format!("/{}/{}", name, STREAM_HEADER);
            let mut stream = cfb.create_stream(&header_path).map_err(|e| {
                AltiumError::Cfb(format!("Failed to create Header: {}", e))
            })?;
            if group.raw_header.is_empty() {
                let count = group.primitives.len() as u32;
                stream
                    .write_all(&count.to_le_bytes())
                    .map_err(AltiumError::Io)?;
            } else {
                stream
                    .write_all(&group.raw_header)
                    .map_err(AltiumError::Io)?;
            }

            // Write Data stream
            let data_path = format!("/{}/{}", name, STREAM_DATA);
            let data = build_pcb_data_stream(group)?;
            let mut stream = cfb.create_stream(&data_path).map_err(|e| {
                AltiumError::Cfb(format!("Failed to create Data: {}", e))
            })?;
            stream.write_all(&data).map_err(AltiumError::Io)?;

            // Write per-footprint extra streams
            for (rel_path, data) in &group.raw_extra_streams {
                let full_path = format!("/{}/{}", name, rel_path);
                if let Ok(mut stream) = cfb.create_stream(&full_path) {
                    let _ = stream.write_all(data);
                }
            }
        }

        cfb.flush()
            .map_err(|e| AltiumError::Cfb(format!("CFB flush: {}", e)))?;
        Ok(())
    }

    /// Save to a file path.
    pub fn save_file(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let file = std::fs::File::create(path).map_err(AltiumError::Io)?;
        self.save(file)
    }

    /// Returns the number of footprints in the library.
    pub fn footprint_count(&self) -> usize {
        self.footprints.len()
    }

    /// Returns the footprint storage names.
    pub fn names(&self) -> &[String] {
        &self.footprint_names
    }

    /// Find a footprint by name (case-insensitive), returns index.
    pub fn find_footprint(&self, name: &str) -> Option<usize> {
        let name_lower = name.to_lowercase();
        self.footprint_names
            .iter()
            .position(|n| n.to_lowercase() == name_lower)
    }

    /// Returns a unique ID from the library (from the first footprint's UNIQUEID parameter).
    pub fn unique_id(&self) -> String {
        for group in &self.footprints {
            if let Some(param) = group.metadata.origin.as_param() {
                if let Some(v) = param.params.get("UNIQUEID") {
                    let s = v.as_str().to_string();
                    if !s.is_empty() {
                        return s;
                    }
                }
            }
        }
        String::new()
    }

    /// Build and add a new footprint using the builder pattern.
    ///
    /// # Example
    ///
    /// ```ignore
    /// lib.build_footprint("SOIC-8", templates::pcb_footprint_default, |builder| {
    ///     builder.with_metadata(|fp| {
    ///         fp.set_pattern("SOIC-8".into());
    ///     });
    ///     builder.add_pad(templates::pcb_pad_default, |pad| {
    ///         pad.set_position_x(PcbCoord::from_mm(1.27));
    ///     });
    /// });
    /// ```
    pub fn build_footprint(
        &mut self,
        name: &str,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut crate::v2::builders::FootprintBuilder),
    ) {
        let mut builder = crate::v2::builders::FootprintBuilder::new(template);
        build(&mut builder);
        self.footprint_names.push(name.to_string());
        self.footprints.push(builder.build());
    }
}

// ---------------------------------------------------------------------------
// FootprintQueryHandle / FootprintQueryResults
// ---------------------------------------------------------------------------

/// A mutable handle to a single matched footprint in a PcbLib.
pub struct FootprintQueryHandle<'a> {
    footprints: &'a mut [FootprintGroup],
    names: &'a [String],
    index: usize,
}

impl<'a> FootprintQueryHandle<'a> {
    /// Consume this handle, construct a `PcbFootprintView`, pass it to the closure.
    pub fn with_mut<R>(
        self,
        f: impl FnOnce(&str, crate::v2::views::PcbFootprintView<'_>) -> R,
    ) -> R {
        let name = &self.names[self.index];
        let group = &mut self.footprints[self.index];
        let (metadata, primitives) = group.split_borrow();
        let view = crate::v2::views::PcbFootprintView::new(metadata, primitives);
        f(name, view)
    }

    /// Returns the index of the matched footprint.
    pub fn index(&self) -> usize {
        self.index
    }
}

/// Results from a multi-match footprint query on a PcbLib.
pub struct FootprintQueryResults<'a> {
    footprints: &'a mut [FootprintGroup],
    names: &'a [String],
    indices: Vec<usize>,
}

impl<'a> FootprintQueryResults<'a> {
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn for_each_mut(
        self,
        mut f: impl FnMut(&str, crate::v2::views::PcbFootprintView<'_>),
    ) {
        for idx in self.indices {
            let name = &self.names[idx];
            let group = &mut self.footprints[idx];
            let (metadata, primitives) = group.split_borrow();
            let view = crate::v2::views::PcbFootprintView::new(metadata, primitives);
            f(name, view);
        }
    }
}

// ---------------------------------------------------------------------------
// DocumentQuery<PcbFootprint> for PcbLib
// ---------------------------------------------------------------------------

impl crate::v2::traits::DocumentQuery<crate::v2::views::PcbFootprint> for PcbLib {
    type Handle<'a> = FootprintQueryHandle<'a>;
    type Results<'a> = FootprintQueryResults<'a>;

    fn query(
        &mut self,
        q: &str,
    ) -> crate::error::Result<FootprintQueryHandle<'_>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let eval_nodes: Vec<_> = self
            .footprints
            .iter()
            .map(|g| g.metadata.clone())
            .collect();

        let matching = evaluate(&parsed, &eval_nodes);

        match matching.len() {
            0 => Err(crate::error::AltiumError::NoMatch(q.to_string())),
            1 => Ok(FootprintQueryHandle {
                footprints: &mut self.footprints,
                names: &self.footprint_names,
                index: matching[0],
            }),
            n => Err(crate::error::AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    fn query_all(
        &mut self,
        q: &str,
    ) -> crate::error::Result<FootprintQueryResults<'_>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let eval_nodes: Vec<_> = self
            .footprints
            .iter()
            .map(|g| g.metadata.clone())
            .collect();

        let indices = evaluate(&parsed, &eval_nodes);

        Ok(FootprintQueryResults {
            footprints: &mut self.footprints,
            names: &self.footprint_names,
            indices,
        })
    }
}

// ---------------------------------------------------------------------------
// DeepPrimitiveHandle / DeepPrimitiveResults — cross-footprint primitive queries
// ---------------------------------------------------------------------------

/// A mutable handle to a primitive found via deep query across all footprints.
pub struct DeepPrimitiveHandle<'a, T: crate::v2::traits::WrapperFamily> {
    footprints: &'a mut [FootprintGroup],
    fp_index: usize,
    prim_index: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<'a, T: crate::v2::traits::LeafViewConstructor> DeepPrimitiveHandle<'a, T> {
    pub fn with_mut<R>(self, f: impl FnOnce(T::View<'_>) -> R) -> R {
        let node = &mut self.footprints[self.fp_index].primitives[self.prim_index];
        let view = T::make_view(node);
        f(view)
    }
}

/// Results from a deep query across all footprints.
pub struct DeepPrimitiveResults<'a, T: crate::v2::traits::WrapperFamily> {
    footprints: &'a mut [FootprintGroup],
    matches: Vec<(usize, usize)>,
    _marker: std::marker::PhantomData<T>,
}

impl<'a, T: crate::v2::traits::LeafViewConstructor> DeepPrimitiveResults<'a, T> {
    pub fn len(&self) -> usize {
        self.matches.len()
    }

    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    pub fn for_each_mut(self, mut f: impl FnMut(T::View<'_>)) {
        for (fi, pi) in self.matches {
            let node = &mut self.footprints[fi].primitives[pi];
            let view = T::make_view(node);
            f(view);
        }
    }
}

// ---------------------------------------------------------------------------
// DocumentQuery<T: LeafViewConstructor> for PcbLib (blanket deep query)
// ---------------------------------------------------------------------------

impl<T: crate::v2::traits::LeafViewConstructor> crate::v2::traits::DocumentQuery<T> for PcbLib {
    type Handle<'a> = DeepPrimitiveHandle<'a, T>;
    type Results<'a> = DeepPrimitiveResults<'a, T>;

    fn query(&mut self, q: &str) -> crate::error::Result<DeepPrimitiveHandle<'_, T>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let mut matches = Vec::new();
        for (fi, fp) in self.footprints.iter().enumerate() {
            for (pi, prim) in fp.primitives.iter().enumerate() {
                if prim.key == T::record_id() {
                    let all = std::slice::from_ref(prim);
                    if !evaluate(&parsed, all).is_empty() {
                        matches.push((fi, pi));
                    }
                }
            }
        }

        match matches.len() {
            0 => Err(crate::error::AltiumError::NoMatch(q.to_string())),
            1 => {
                let (fi, pi) = matches[0];
                Ok(DeepPrimitiveHandle {
                    footprints: &mut self.footprints,
                    fp_index: fi,
                    prim_index: pi,
                    _marker: std::marker::PhantomData,
                })
            }
            n => Err(crate::error::AltiumError::AmbiguousMatch(n, q.to_string())),
        }
    }

    fn query_all(&mut self, q: &str) -> crate::error::Result<DeepPrimitiveResults<'_, T>> {
        use crate::v2::query::eval::evaluate;
        let parsed = crate::v2::query::parse(q)?;

        let mut matches = Vec::new();
        for (fi, fp) in self.footprints.iter().enumerate() {
            for (pi, prim) in fp.primitives.iter().enumerate() {
                if prim.key == T::record_id() {
                    let all = std::slice::from_ref(prim);
                    if !evaluate(&parsed, all).is_empty() {
                        matches.push((fi, pi));
                    }
                }
            }
        }

        Ok(DeepPrimitiveResults {
            footprints: &mut self.footprints,
            matches,
            _marker: std::marker::PhantomData,
        })
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Read PCB section keys (stub — PcbLib files don't typically use section keys).
fn read_pcb_section_keys<F: Read + Seek>(
    _cfb: &mut cfb::CompoundFile<F>,
) -> Result<SectionKeyList> {
    Ok(SectionKeyList::new())
}

/// Returns the number of subrecords for a given PCB primitive type.
///
/// Pad (type 2) has 6 subrecords; Text (type 5) has 2 subrecords;
/// all others have 1 subrecord.
fn subrecord_count(type_id: u8) -> usize {
    match type_id {
        2 => 6,  // Pad
        5 => 2,  // Text
        _ => 1,
    }
}

/// Parse the PCB Data stream: pattern name block + binary primitives.
///
/// Format:
/// - 4 bytes LE: pattern name length
/// - N bytes: pattern name
/// - For each primitive:
///   - 1 byte: type ID
///   - For single-subrecord types: 4 bytes LE length + data
///   - For multi-subrecord types (Pad=6, Text=2): N sequential
///     (4 bytes LE length + data) blocks stored together
fn parse_pcb_data_stream(
    data: &[u8],
) -> Result<(Vec<RecordNode>, Vec<PcbPrimitiveRef>, Vec<u8>)> {
    let mut cursor = Cursor::new(data);
    let mut primitives = Vec::new();
    let mut primitive_order = Vec::new();

    // Read pattern name block (length-prefixed)
    let pattern_name_block = if data.len() >= 4 {
        let str_len = cursor
            .read_u32::<LittleEndian>()
            .map_err(|_| AltiumError::UnexpectedEof)? as usize;
        if str_len > 0 && cursor.position() as usize + str_len <= data.len() {
            let mut buf = vec![0u8; str_len];
            cursor.read_exact(&mut buf).map_err(AltiumError::Io)?;
            buf
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Read binary primitives
    while (cursor.position() as usize) < data.len() {
        let type_byte = match cursor.read_u8() {
            Ok(b) => b,
            Err(_) => break,
        };

        let n = subrecord_count(type_byte);

        if n == 1 {
            // Single subrecord: read u32 len + data, store data only
            let block_len = match cursor.read_u32::<LittleEndian>() {
                Ok(l) => l as usize,
                Err(_) => break,
            };

            if cursor.position() as usize + block_len > data.len() {
                break;
            }

            let mut block_data = vec![0u8; block_len];
            if cursor.read_exact(&mut block_data).is_err() {
                break;
            }

            let index = primitives.len();
            let origin = RecordOrigin::Binary(BinaryOrigin::new(block_data));
            primitives.push(RecordNode::new(type_byte, origin));
            primitive_order.push(PcbPrimitiveRef::new(type_byte, index));
        } else {
            // Multi-subrecord: read N sequential (u32 len + data) blocks,
            // store ALL bytes including u32 prefixes as one raw_block
            let start = cursor.position() as usize;
            let mut ok = true;
            for _ in 0..n {
                let sub_len = match cursor.read_u32::<LittleEndian>() {
                    Ok(l) => l as usize,
                    Err(_) => { ok = false; break; }
                };
                if cursor.position() as usize + sub_len > data.len() {
                    ok = false;
                    break;
                }
                cursor.set_position(cursor.position() + sub_len as u64);
            }
            if !ok {
                break;
            }
            let end = cursor.position() as usize;
            let block_data = data[start..end].to_vec();

            let index = primitives.len();
            let origin = RecordOrigin::Binary(BinaryOrigin::new(block_data));
            primitives.push(RecordNode::new(type_byte, origin));
            primitive_order.push(PcbPrimitiveRef::new(type_byte, index));
        }
    }

    Ok((primitives, primitive_order, pattern_name_block))
}

/// Build a PCB Data stream from a FootprintGroup.
fn build_pcb_data_stream(group: &FootprintGroup) -> Result<Vec<u8>> {
    let mut output = Vec::new();

    // Write pattern name block
    output
        .write_u32::<LittleEndian>(group.raw_pattern_name_block.len() as u32)
        .map_err(AltiumError::Io)?;
    output.extend_from_slice(&group.raw_pattern_name_block);

    // Write primitives in original order
    for prim_ref in &group.original_primitive_order {
        if prim_ref.index < group.primitives.len() {
            let prim = &group.primitives[prim_ref.index];
            let n = subrecord_count(prim.key);

            // Get the bytes to write (from dirty origin or clean snapshot)
            let bytes = if prim.is_dirty() {
                match &prim.origin {
                    RecordOrigin::Binary(b) => &b.raw_block,
                    RecordOrigin::Param(_) => &[] as &[u8],
                }
            } else {
                &prim.original_snapshot
            };

            output.push(prim.key); // type byte

            if n == 1 {
                // Single subrecord: write u32(len) + bytes
                output
                    .write_u32::<LittleEndian>(bytes.len() as u32)
                    .map_err(AltiumError::Io)?;
                output.extend_from_slice(bytes);
            } else {
                // Multi-subrecord: bytes already contain u32 prefixes,
                // write directly after the type byte
                output.extend_from_slice(bytes);
            }
        }
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcb_data_stream_roundtrip() {
        let mut data = Vec::new();
        // Pattern name: "SOT-23"
        let name = b"SOT-23";
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name);
        // A track primitive: type=4, length=35, zeros
        data.push(4); // type byte
        data.extend_from_slice(&35u32.to_le_bytes()); // length
        data.extend_from_slice(&vec![0u8; 35]); // data

        let (prims, order, pattern_name) =
            parse_pcb_data_stream(&data).unwrap();
        assert_eq!(pattern_name, name);
        assert_eq!(prims.len(), 1);
        assert_eq!(prims[0].key, 4);
        assert_eq!(order.len(), 1);
    }

    #[test]
    fn empty_data_stream() {
        let (prims, order, pattern_name) =
            parse_pcb_data_stream(&[]).unwrap();
        assert!(prims.is_empty());
        assert!(order.is_empty());
        assert!(pattern_name.is_empty());
    }

    #[test]
    fn build_stream_roundtrip() {
        // Build a minimal footprint group
        let block_data = vec![0xAA; 10];
        let prim = RecordNode::new(
            4,
            RecordOrigin::Binary(BinaryOrigin::new(block_data.clone())),
        );
        let group = FootprintGroup::new(
            RecordNode::new(
                0,
                RecordOrigin::Param(ParamOrigin::new("|PATTERN=DIP-8|")),
            ),
            vec![prim],
            b"DIP-8".to_vec(),
            vec![PcbPrimitiveRef::new(4, 0)],
            vec![],
        );

        let data = build_pcb_data_stream(&group).unwrap();
        let (prims, order, pattern_name) =
            parse_pcb_data_stream(&data).unwrap();

        assert_eq!(pattern_name, b"DIP-8");
        assert_eq!(prims.len(), 1);
        assert_eq!(prims[0].key, 4);
        assert_eq!(order.len(), 1);
        assert_eq!(order[0].type_id, 4);
    }

    // -----------------------------------------------------------------------
    // DocumentQuery tests for PcbLib
    // -----------------------------------------------------------------------

    #[test]
    fn pcblib_query_all_footprints() {
        use crate::v2::traits::DocumentQuery;

        let mut lib = PcbLib::default();
        for name in &["SOT-23", "QFP-48", "DIP-8"] {
            lib.footprint_names.push(name.to_string());
            lib.footprints.push(FootprintGroup::new(
                RecordNode::new(
                    0,
                    RecordOrigin::Param(ParamOrigin::new(&format!(
                        "|PATTERN={}|DESCRIPTION={}|",
                        name, name
                    ))),
                ),
                vec![],
                Vec::new(),
                vec![],
                vec![],
            ));
        }

        let results = DocumentQuery::<crate::v2::views::PcbFootprint>::query_all(&mut lib, "#0")
            .unwrap();
        assert_eq!(results.len(), 3); // all have record_id=0
    }

    #[test]
    fn pcblib_deep_query_pad() {
        use crate::v2::traits::DocumentQuery;

        let mut lib = PcbLib::default();
        lib.footprint_names.push("SOT-23".to_string());

        let pad_block = vec![0u8; 40]; // minimal binary block for a pad
        lib.footprints.push(FootprintGroup::new(
            RecordNode::new(
                0,
                RecordOrigin::Param(ParamOrigin::new("|PATTERN=SOT-23|")),
            ),
            vec![
                RecordNode::new(
                    2, // pad type
                    RecordOrigin::Binary(BinaryOrigin::new(pad_block.clone())),
                ),
                RecordNode::new(
                    2,
                    RecordOrigin::Binary(BinaryOrigin::new(pad_block.clone())),
                ),
                RecordNode::new(
                    4, // track type
                    RecordOrigin::Binary(BinaryOrigin::new(vec![0u8; 35])),
                ),
            ],
            Vec::new(),
            vec![
                PcbPrimitiveRef::new(2, 0),
                PcbPrimitiveRef::new(2, 1),
                PcbPrimitiveRef::new(4, 2),
            ],
            vec![],
        ));

        // Use #2 (record_id match) because the AQL element_type_to_record_id
        // for Pad currently maps to a placeholder (100), not the actual PCB type_id (2).
        let results =
            DocumentQuery::<crate::v2::views::PcbPad>::query_all(&mut lib, "#2").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn multiple_primitives() {
        let mut data = Vec::new();
        // Pattern name: "QFP"
        let name = b"QFP";
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name);
        // Track primitive: type=4
        data.push(4);
        data.extend_from_slice(&8u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 8]);
        // Arc primitive: type=1 (single subrecord)
        data.push(1);
        data.extend_from_slice(&12u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 12]);

        let (prims, order, _) = parse_pcb_data_stream(&data).unwrap();
        assert_eq!(prims.len(), 2);
        assert_eq!(prims[0].key, 4);
        assert_eq!(prims[1].key, 1);
        assert_eq!(order.len(), 2);
        assert_eq!(order[0].type_id, 4);
        assert_eq!(order[1].type_id, 1);
    }

    #[test]
    fn pad_multi_subrecord_roundtrip() {
        let mut data = Vec::new();
        // Pattern name: "PAD"
        let name = b"PAD";
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name);
        // Pad primitive: type=2 with 6 subrecords
        data.push(2);
        // Subrecords 1-4: small string subrecords
        for i in 0u8..4 {
            let sub = vec![i; 2]; // 2-byte payload
            data.extend_from_slice(&(sub.len() as u32).to_le_bytes());
            data.extend_from_slice(&sub);
        }
        // Subrecord 5: core data (16 bytes)
        let core = vec![0xAA; 16];
        data.extend_from_slice(&(core.len() as u32).to_le_bytes());
        data.extend_from_slice(&core);
        // Subrecord 6: stack data (8 bytes)
        let stack = vec![0xBB; 8];
        data.extend_from_slice(&(stack.len() as u32).to_le_bytes());
        data.extend_from_slice(&stack);

        let (prims, order, pattern_name) = parse_pcb_data_stream(&data).unwrap();
        assert_eq!(pattern_name, b"PAD");
        assert_eq!(prims.len(), 1);
        assert_eq!(prims[0].key, 2);
        assert_eq!(order.len(), 1);

        // The raw_block should contain all 6 subrecords with u32 prefixes
        let raw = prims[0].origin.as_binary().unwrap();
        // 4*(4+2) + (4+16) + (4+8) = 24 + 20 + 12 = 56
        assert_eq!(raw.raw_block.len(), 56);

        // Round-trip: build and re-parse
        let group = FootprintGroup::new(
            RecordNode::new(0, RecordOrigin::Param(ParamOrigin::new("|PATTERN=PAD|"))),
            prims,
            b"PAD".to_vec(),
            order,
            vec![],
        );
        let rebuilt = build_pcb_data_stream(&group).unwrap();
        let (prims2, _, _) = parse_pcb_data_stream(&rebuilt).unwrap();
        assert_eq!(prims2.len(), 1);
        assert_eq!(prims2[0].key, 2);
        assert_eq!(
            prims2[0].origin.as_binary().unwrap().raw_block.len(),
            56
        );
    }

    #[test]
    fn text_multi_subrecord_roundtrip() {
        let mut data = Vec::new();
        // Pattern name: "TXT"
        let name = b"TXT";
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name);
        // Text primitive: type=5 with 2 subrecords
        data.push(5);
        // Subrecord 1: main text data (40 bytes)
        let sub1 = vec![0xCC; 40];
        data.extend_from_slice(&(sub1.len() as u32).to_le_bytes());
        data.extend_from_slice(&sub1);
        // Subrecord 2: text string (10 bytes)
        let sub2 = b"Hello\0\0\0\0\0";
        data.extend_from_slice(&(sub2.len() as u32).to_le_bytes());
        data.extend_from_slice(sub2);

        let (prims, order, _) = parse_pcb_data_stream(&data).unwrap();
        assert_eq!(prims.len(), 1);
        assert_eq!(prims[0].key, 5);
        // raw_block = (4+40) + (4+10) = 58
        assert_eq!(prims[0].origin.as_binary().unwrap().raw_block.len(), 58);

        // Round-trip
        let group = FootprintGroup::new(
            RecordNode::new(0, RecordOrigin::Param(ParamOrigin::new("|PATTERN=TXT|"))),
            prims,
            b"TXT".to_vec(),
            order,
            vec![],
        );
        let rebuilt = build_pcb_data_stream(&group).unwrap();
        let (prims2, _, _) = parse_pcb_data_stream(&rebuilt).unwrap();
        assert_eq!(prims2.len(), 1);
        assert_eq!(prims2[0].key, 5);
        assert_eq!(prims2[0].origin.as_binary().unwrap().raw_block.len(), 58);
    }
}

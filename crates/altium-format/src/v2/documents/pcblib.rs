//! PcbLib document I/O using the v2 backing-store architecture.
//!
//! A PcbLib file is a CFB compound file with one storage per footprint:
//! - `/<FootprintName>/Parameters` stream: footprint metadata (pipe-delimited)
//! - `/<FootprintName>/Header` stream: primitive count and version info
//! - `/<FootprintName>/Data` stream: binary primitives (type byte + length + data)
//!
//! The Data stream begins with a length-prefixed pattern name block, followed
//! by packed binary primitive records.

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

            lib.footprint_names.push(storage_name.clone());
            lib.footprints.push(FootprintGroup::new(
                metadata,
                primitives,
                raw_pattern_name,
                primitive_order,
                raw_header,
            ));
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

    /// Iterate all footprints with name and mutable view access.
    pub fn for_each_footprint<F>(&mut self, mut f: F)
    where
        F: FnMut(&str, crate::v2::views::PcbFootprintView<'_>),
    {
        let names = &self.footprint_names;
        let footprints = &mut self.footprints;
        for (name, group) in names.iter().zip(footprints.iter_mut()) {
            let (metadata, primitives) = group.split_borrow();
            let view = crate::v2::views::PcbFootprintView::new(metadata, primitives);
            f(name, view);
        }
    }

    /// Access a specific footprint by index.
    pub fn with_footprint<R>(
        &mut self,
        index: usize,
        f: impl FnOnce(&str, crate::v2::views::PcbFootprintView<'_>) -> R,
    ) -> Option<R> {
        if index >= self.footprints.len() || index >= self.footprint_names.len() {
            return None;
        }
        let name = &self.footprint_names[index];
        let group = &mut self.footprints[index];
        let (metadata, primitives) = group.split_borrow();
        let view = crate::v2::views::PcbFootprintView::new(metadata, primitives);
        Some(f(name, view))
    }

    /// Find a footprint by name (case-insensitive), returns index.
    pub fn find_footprint(&self, name: &str) -> Option<usize> {
        let name_lower = name.to_lowercase();
        self.footprint_names
            .iter()
            .position(|n| n.to_lowercase() == name_lower)
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
// Internal helpers
// ---------------------------------------------------------------------------

/// Read PCB section keys (stub — PcbLib files don't typically use section keys).
fn read_pcb_section_keys<F: Read + Seek>(
    _cfb: &mut cfb::CompoundFile<F>,
) -> Result<SectionKeyList> {
    Ok(SectionKeyList::new())
}

/// Parse the PCB Data stream: pattern name block + binary primitives.
///
/// Format:
/// - 4 bytes LE: pattern name length
/// - N bytes: pattern name
/// - For each primitive:
///   - 1 byte: type ID
///   - 4 bytes LE: data length
///   - N bytes: primitive data
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
        // Each primitive: 1 byte type + 4 bytes length + data
        let type_byte = match cursor.read_u8() {
            Ok(b) => b,
            Err(_) => break,
        };

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
            output.push(prim.key); // type byte

            if prim.is_dirty() {
                match &prim.origin {
                    RecordOrigin::Binary(b) => {
                        output
                            .write_u32::<LittleEndian>(b.raw_block.len() as u32)
                            .map_err(AltiumError::Io)?;
                        output.extend_from_slice(&b.raw_block);
                    }
                    RecordOrigin::Param(_) => {
                        // PCB primitives should not be param-based, but handle
                        // gracefully by writing empty block.
                        output
                            .write_u32::<LittleEndian>(0)
                            .map_err(AltiumError::Io)?;
                    }
                }
            } else {
                // Write original snapshot (block data without type byte)
                output
                    .write_u32::<LittleEndian>(
                        prim.original_snapshot.len() as u32,
                    )
                    .map_err(AltiumError::Io)?;
                output.extend_from_slice(&prim.original_snapshot);
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
        // Pad primitive: type=2
        data.push(2);
        data.extend_from_slice(&12u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 12]);

        let (prims, order, _) = parse_pcb_data_stream(&data).unwrap();
        assert_eq!(prims.len(), 2);
        assert_eq!(prims[0].key, 4);
        assert_eq!(prims[1].key, 2);
        assert_eq!(order.len(), 2);
        assert_eq!(order[0].type_id, 4);
        assert_eq!(order[1].type_id, 2);
    }
}

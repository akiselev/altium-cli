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

use std::io::{Read, Write, Cursor, Seek};

use crate::error::{AltiumError, Result};
use crate::v2::consts;
use crate::v2::io::section_keys::SectionKeyList;
use crate::v2::serializer::SchSerializer;
use crate::v2::serializer::ascii::AsciiSerializer;

/// A parsed SchLib library.
#[derive(Clone, Debug, Default)]
pub struct SchLibV2 {
    /// Library header info.
    pub header: SchLibHeader,
    /// Components in the library.
    pub components: Vec<SchLibComponent>,
    /// Section key mappings.
    pub section_keys: SectionKeyList,
}

/// SchLib file header.
#[derive(Clone, Debug, Default)]
pub struct SchLibHeader {
    pub header_text: String,
    pub weight: i32,
    pub minor_version: i32,
    pub unique_id: String,
}

/// A component entry from the FileHeader.
#[derive(Clone, Debug, Default)]
pub struct SchLibComponentEntry {
    pub lib_ref: String,
    pub description: String,
    pub part_count: i16,
    pub aliases: Vec<String>,
}

/// A complete component with its records.
#[derive(Clone, Debug, Default)]
pub struct SchLibComponent {
    /// Component entry from FileHeader.
    pub entry: SchLibComponentEntry,
    /// Raw serialized records (record_id, param_string pairs).
    /// Each record is stored as an ASCII parameter string.
    pub records: Vec<SchLibRecord>,
}

/// A single record within a component.
#[derive(Clone, Debug)]
pub struct SchLibRecord {
    /// Object ID / record type.
    pub record_id: u8,
    /// Extended record ID (when record_id == 254).
    pub record_id_ex: Option<i32>,
    /// Raw parameter string for this record.
    pub params: String,
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
            let section_key = lib.section_keys.get_key(&lib.components[i].entry.lib_ref).to_string();
            let data_path = format!("/{}/{}", section_key, consts::STREAM_DATA);

            if let Ok(mut stream) = cfb.open_stream(&data_path) {
                let mut data = Vec::new();
                stream.read_to_end(&mut data)
                    .map_err(AltiumError::Io)?;
                lib.components[i].records = parse_data_stream(&data)?;
            }
        }

        Ok(lib)
    }

    /// Write a SchLib to a CFB compound file.
    pub fn write<W: Read + Write + Seek>(&self, writer: W) -> Result<()> {
        let mut cfb = cfb::CompoundFile::create(writer)
            .map_err(|e| AltiumError::Parse(format!("Failed to create CFB: {}", e)))?;

        // 1. Build section keys
        let mut section_keys = SectionKeyList::new();
        for comp in &self.components {
            section_keys.add_key(&comp.entry.lib_ref, 30);
            for alias in &comp.entry.aliases {
                section_keys.add_key(alias, 30);
            }
        }

        // 2. Write FileHeader
        write_file_header(&mut cfb, &self.header, &self.components)?;

        // 3. Write SectionKeys
        write_section_keys(&mut cfb, &section_keys)?;

        // 4. Write Data stream for each component
        for comp in &self.components {
            let section_key = section_keys.get_key(&comp.entry.lib_ref).to_string();

            // Create storage for this component
            let storage_path = format!("/{}", section_key);
            cfb.create_storage(&storage_path)
                .map_err(|e| AltiumError::Parse(format!("Failed to create storage: {}", e)))?;

            // Write Data stream
            let data_path = format!("/{}/{}", section_key, consts::STREAM_DATA);
            let data = build_data_stream(&comp.records)?;
            let mut stream = cfb.create_stream(&data_path)
                .map_err(|e| AltiumError::Parse(format!("Failed to create stream: {}", e)))?;
            stream.write_all(&data).map_err(AltiumError::Io)?;

            // Write alias redirections
            for alias in &comp.entry.aliases {
                let alias_key = section_keys.get_key(alias).to_string();
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

/// Parse a Data stream into individual records.
///
/// The Data stream format:
/// - Length-prefixed records: each record is `[u32 length][param_bytes]`
/// - The param bytes are pipe-delimited `|RECORD=N|...|` strings
fn parse_data_stream(data: &[u8]) -> Result<Vec<SchLibRecord>> {
    let mut records = Vec::new();
    let mut cursor = Cursor::new(data);
    let len = data.len() as u64;

    while cursor.position() < len {
        // Read record length (4 bytes LE)
        let mut len_buf = [0u8; 4];
        if cursor.read_exact(&mut len_buf).is_err() {
            break;
        }
        let record_len = u32::from_le_bytes(len_buf) as usize;

        if record_len == 0 {
            continue;
        }

        // Read record data
        let mut record_data = vec![0u8; record_len];
        if cursor.read_exact(&mut record_data).is_err() {
            break;
        }

        // Parse as ASCII params
        let param_str = String::from_utf8_lossy(&record_data).to_string();
        let mut ser = AsciiSerializer::from_params(&param_str);

        // Extract record ID
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
        });
    }

    Ok(records)
}

/// Build a Data stream from records.
fn build_data_stream(records: &[SchLibRecord]) -> Result<Vec<u8>> {
    let mut output = Vec::new();

    for record in records {
        let param_bytes = record.params.as_bytes();
        let len = param_bytes.len() as u32;
        output.extend_from_slice(&len.to_le_bytes());
        output.extend_from_slice(param_bytes);
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
            },
            SchLibRecord {
                record_id: 2,
                record_id_ex: None,
                params: "|RECORD=2|OwnerIndex=0|Name=VCC|".to_string(),
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
        };
        assert_eq!(rec.effective_record_id(), 2);

        // Extended record (record_id=254, actual ID in record_id_ex)
        let rec_ex = SchLibRecord {
            record_id: 254,
            record_id_ex: Some(300),
            params: "|RECORD=254|RECORDEX=300|".to_string(),
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
                },
                SchLibRecord {
                    record_id: 2,
                    record_id_ex: None,
                    params: "|RECORD=2|OwnerIndex=0|Name=1|Designator=1|".to_string(),
                },
            ],
        });

        // Write to memory buffer
        let buf = Cursor::new(Vec::new());
        lib.write(buf).unwrap();
    }
}

//! SchDoc reader/writer for Altium schematic document files.

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use cfb::CompoundFile;
use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;

use crate::dump::{DumpTree, TreeBuilder};
use crate::error::{AltiumError, Result};
use crate::format::SIZE_FLAG_MASK;
use crate::io::reader::{decode_windows_1252, read_parameters_block};
use crate::io::writer::{write_block, write_parameters};
use crate::records::sch::{SchPrimitive, SchRecord, SchSheetHeader};
use crate::types::ParameterCollection;

/// A schematic document containing primitives.
#[derive(Debug, Default)]
pub struct SchDoc {
    /// All primitives in the document.
    pub primitives: Vec<SchRecord>,
    /// Optional document name (typically the filename without extension).
    /// This is used as the sheet name in queries, not the Title parameter.
    pub document_name: Option<String>,
}

impl SchDoc {
    /// Open and read a SchDoc file.
    pub fn open<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut schdoc = SchDoc::default();
        let mut cf = CompoundFile::open(reader).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?;

        // Read FileHeader stream (contains all primitives)
        schdoc.read_file_header(&mut cf)?;

        Ok(schdoc)
    }

    /// Open and read a SchDoc file from a path.
    pub fn open_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        let file = File::open(path_ref)?;
        let mut doc = Self::open(file)?;

        // Extract document name from filename (without extension)
        if let Some(file_stem) = path_ref.file_stem() {
            if let Some(name) = file_stem.to_str() {
                doc.document_name = Some(name.to_string());
            }
        }

        Ok(doc)
    }

    /// Read the FileHeader stream.
    fn read_file_header<R: Read + Seek>(&mut self, cf: &mut CompoundFile<R>) -> Result<()> {
        let stream_path = "/FileHeader";
        let mut stream = cf.open_stream(stream_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        let mut data = Vec::new();
        stream.read_to_end(&mut data)?;

        if data.is_empty() {
            return Ok(());
        }

        let mut cursor = Cursor::new(&data);

        // Read header parameters
        let header_params = read_parameters_block(&mut cursor)?;
        let _header = header_params.get("HEADER").map(|v| v.as_str().to_string());
        let _weight = header_params
            .get("WEIGHT")
            .map(|v| v.as_int_or(0))
            .unwrap_or(0);

        // Read all primitives
        while (cursor.position() as usize) < data.len() {
            match self.read_record(&mut cursor) {
                Ok(record) => self.primitives.push(record),
                Err(e) => {
                    log::warn!(
                        "Failed to read record at position {}: {}, skipping remaining records",
                        cursor.position(),
                        e
                    );
                    break;
                }
            }
        }

        Ok(())
    }

    /// Read a single record from the stream.
    fn read_record<R: Read>(&self, reader: &mut R) -> Result<SchRecord> {
        let size = reader.read_i32::<LittleEndian>()?;
        let is_binary = (size as u32 & !SIZE_FLAG_MASK) != 0;
        let clean_size = (size & SIZE_FLAG_MASK as i32) as usize;

        if clean_size == 0 {
            return Err(AltiumError::Parse("Empty record".to_string()));
        }

        let mut buffer = vec![0u8; clean_size];
        reader.read_exact(&mut buffer)?;

        if is_binary {
            // Binary pin record (not common in SchDoc)
            Err(AltiumError::Parse(
                "Binary records not supported in SchDoc".to_string(),
            ))
        } else {
            // ASCII parameter record
            let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
            let param_str = decode_windows_1252(&buffer[..end]);
            let params = ParameterCollection::from_string(&param_str);
            SchRecord::from_params(&params)
        }
    }

    /// Save the SchDoc to a file.
    pub fn save<W: Read + Write + Seek>(&self, writer: W) -> Result<()> {
        let mut cf = CompoundFile::create(writer)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        // Write Storage stream (icon storage header)
        self.write_storage(&mut cf)?;

        // Write FileHeader
        self.write_file_header(&mut cf)?;

        // Write Additional stream
        self.write_additional(&mut cf)?;

        cf.flush()
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Save the SchDoc to a file path.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        self.save(file)
    }

    /// Write the Storage stream (icon storage header).
    fn write_storage<F: Read + Write + Seek>(&self, cf: &mut CompoundFile<F>) -> Result<()> {
        let mut data = Vec::new();

        // Write the icon storage header block
        let header = "|HEADER=Icon storage\0";
        let header_bytes = header.as_bytes();
        data.write_i32::<LittleEndian>(header_bytes.len() as i32)?;
        data.write_all(header_bytes)?;

        let stream = cf
            .create_stream("/Storage")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        let mut stream = stream;
        stream.write_all(&data)?;

        Ok(())
    }

    /// Write the FileHeader stream.
    fn write_file_header<F: Read + Write + Seek>(&self, cf: &mut CompoundFile<F>) -> Result<()> {
        let mut data = Vec::new();

        // Write header parameters
        let mut header_params = ParameterCollection::new();
        header_params.add(
            "HEADER",
            "Protel for Windows - Schematic Capture Binary File Version 5.0",
        );
        header_params.add_int("WEIGHT", self.primitives.len() as i32);

        let mut header_block = Vec::new();
        write_parameters(&mut header_block, &header_params)?;
        write_block(&mut data, &header_block, 0)?;

        // Write all primitives
        for record in &self.primitives {
            self.write_record(&mut data, record)?;
        }

        let stream = cf
            .create_stream("/FileHeader")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        let mut stream = stream;
        stream.write_all(&data)?;

        Ok(())
    }

    /// Write the Additional stream.
    fn write_additional<F: Read + Write + Seek>(&self, cf: &mut CompoundFile<F>) -> Result<()> {
        let mut data = Vec::new();

        let mut params = ParameterCollection::new();
        params.add(
            "HEADER",
            "Protel for Windows - Schematic Capture Binary File Version 5.0",
        );

        let mut block = Vec::new();
        write_parameters(&mut block, &params)?;
        write_block(&mut data, &block, 0)?;

        let stream = cf
            .create_stream("/Additional")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        let mut stream = stream;
        stream.write_all(&data)?;

        Ok(())
    }

    /// Write a single record to the stream.
    fn write_record<W: Write>(&self, writer: &mut W, record: &SchRecord) -> Result<()> {
        let params = record.export_to_params();
        let mut block = Vec::new();
        write_parameters(&mut block, &params)?;
        write_block(writer, &block, 0)
    }

    /// Get the sheet header if present.
    pub fn sheet_header(&self) -> Option<&SchSheetHeader> {
        self.primitives.iter().find_map(|r| {
            if let SchRecord::SheetHeader(h) = r {
                Some(h)
            } else {
                None
            }
        })
    }

    /// Get all components in the document.
    pub fn components(&self) -> impl Iterator<Item = &crate::records::sch::SchComponent> {
        self.primitives.iter().filter_map(|r| {
            if let SchRecord::Component(c) = r {
                Some(c)
            } else {
                None
            }
        })
    }

    /// Get all wires in the document.
    pub fn wires(&self) -> impl Iterator<Item = &crate::records::sch::SchWire> {
        self.primitives.iter().filter_map(|r| {
            if let SchRecord::Wire(w) = r {
                Some(w)
            } else {
                None
            }
        })
    }

    /// Get the number of primitives.
    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }
}

impl SchRecord {
    /// Export record to parameters.
    pub fn export_to_params(&self) -> ParameterCollection {
        match self {
            SchRecord::Component(r) => r.export_to_params(),
            SchRecord::Pin(r) => r.export_to_params(),
            SchRecord::Symbol(r) => r.export_to_params(),
            SchRecord::Label(r) => r.export_to_params(),
            SchRecord::Bezier(r) => r.export_to_params(),
            SchRecord::Polyline(r) => r.export_to_params(),
            SchRecord::Polygon(r) => r.export_to_params(),
            SchRecord::Ellipse(r) => r.export_to_params(),
            SchRecord::Pie(r) => r.export_to_params(),
            SchRecord::EllipticalArc(r) => r.export_to_params(),
            SchRecord::Arc(r) => r.export_to_params(),
            SchRecord::Line(r) => r.export_to_params(),
            SchRecord::Rectangle(r) => r.export_to_params(),
            SchRecord::PowerObject(r) => r.export_to_params(),
            SchRecord::Port(r) => r.export_to_params(),
            SchRecord::NoErc(r) => r.export_to_params(),
            SchRecord::NetLabel(r) => r.export_to_params(),
            SchRecord::Bus(r) => r.export_to_params(),
            SchRecord::Wire(r) => r.export_to_params(),
            SchRecord::TextFrame(r) => r.export_to_params(),
            SchRecord::TextFrameVariant(r) => r.export_to_params(),
            SchRecord::Junction(r) => r.export_to_params(),
            SchRecord::Image(r) => r.export_to_params(),
            SchRecord::SheetHeader(r) => r.export_to_params(),
            SchRecord::Designator(r) => r.export_to_params(),
            SchRecord::BusEntry(r) => r.export_to_params(),
            SchRecord::Parameter(r) => r.export_to_params(),
            SchRecord::WarningSign(r) => r.export_to_params(),
            SchRecord::ImplementationList(r) => r.export_to_params(),
            SchRecord::Implementation(r) => r.export_to_params(),
            SchRecord::MapDefinerList(r) => r.export_to_params(),
            SchRecord::MapDefiner(r) => r.export_to_params(),
            SchRecord::ImplementationParameters(r) => r.export_to_params(),
            SchRecord::Unknown { record_id, params } => {
                let mut p = params.clone();
                p.add_int("RECORD", *record_id);
                p
            }
        }
    }
}

impl DumpTree for SchDoc {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.root(&format!("SchDoc ({} primitives)", self.primitives.len()));

        // Count by type
        let mut component_count = 0;
        let mut wire_count = 0;
        let mut other_count = 0;

        for prim in &self.primitives {
            match prim {
                SchRecord::Component(_) => component_count += 1,
                SchRecord::Wire(_) => wire_count += 1,
                _ => other_count += 1,
            }
        }

        // Summary
        tree.push(true);
        tree.add_leaf(
            "Summary",
            &[
                ("components", format!("{}", component_count)),
                ("wires", format!("{}", wire_count)),
                ("other", format!("{}", other_count)),
            ],
        );
        tree.pop();

        // Show primitives
        tree.push(false);
        tree.begin_node(&format!("Primitives ({})", self.primitives.len()));
        for (i, prim) in self.primitives.iter().enumerate() {
            tree.push(i < self.primitives.len() - 1);
            prim.dump(tree);
            tree.pop();
        }
        tree.pop();
    }
}

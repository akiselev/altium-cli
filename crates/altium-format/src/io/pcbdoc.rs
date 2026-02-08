//! PcbDoc reader/writer for Altium PCB document files.
//!
//! Supports reading and writing of PCB documents including board data,
//! components, primitives, nets, and design rules.

use cfb::CompoundFile;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::dump::{DumpTree, TreeBuilder};
use crate::error::{AltiumError, Result};
use crate::io::reader::{read_block, read_parameters_block};
use crate::io::writer::write_parameters_block;
use crate::records::pcb::{
    PcbAdvancedPlacerOptions, PcbArc, PcbClass, PcbCoordinate, PcbDimension, PcbDrcOptions,
    PcbFill, PcbObjectId, PcbPinSwapOptions, PcbPolygon, PcbRecord, PcbRegion, PcbRule, PcbText,
    PcbTrack, PcbVia,
};
use crate::traits::FromBinary;
use crate::types::ParameterCollection;

/// A PCB document containing board data.
#[derive(Debug, Default)]
pub struct PcbDoc {
    /// Board header parameters.
    pub board_params: ParameterCollection,
    /// Components placed on the board.
    pub components: Vec<PcbDocComponent>,
    /// Board primitives (not associated with components).
    pub primitives: Vec<PcbRecord>,
    /// Nets in the design.
    pub nets: Vec<String>,
    /// Design rules.
    pub rules: Vec<PcbRule>,
    /// Object classes (net classes, component classes, etc.).
    pub classes: Vec<PcbClass>,
    /// Advanced placer options.
    pub placer_options: Option<PcbAdvancedPlacerOptions>,
    /// Design rule checker options.
    pub drc_options: Option<PcbDrcOptions>,
    /// Pin swap options.
    pub pin_swap_options: Option<PcbPinSwapOptions>,
}

/// A component placed on the board.
#[derive(Debug, Default)]
pub struct PcbDocComponent {
    /// Component designator (e.g., "R1", "U1").
    pub designator: String,
    /// Footprint pattern name.
    pub pattern: String,
    /// Component comment/value.
    pub comment: String,
    /// Component parameters.
    pub params: ParameterCollection,
    /// Primitives belonging to this component.
    pub primitives: Vec<PcbRecord>,
}

impl PcbDoc {
    /// Open and read a PcbDoc file.
    pub fn open<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut pcbdoc = PcbDoc::default();
        let mut cf = CompoundFile::open(reader).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?;

        // Read board header/parameters
        pcbdoc.read_board(&mut cf)?;

        // Read components
        pcbdoc.read_components(&mut cf)?;

        // Read board primitives
        pcbdoc.read_primitives(&mut cf)?;

        // Read nets
        pcbdoc.read_nets(&mut cf)?;

        // Read design rules
        pcbdoc.read_rules(&mut cf)?;

        // Read classes
        pcbdoc.read_classes(&mut cf)?;

        // Read options
        pcbdoc.read_options(&mut cf)?;

        Ok(pcbdoc)
    }

    /// Open and read a PcbDoc file from a path.
    pub fn open_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        Self::open(file)
    }

    /// Read the Board storage.
    fn read_board<R: Read + Seek>(&mut self, cf: &mut CompoundFile<R>) -> Result<()> {
        let data_path = "/Board6/Data";

        if cf.entry(data_path).is_err() {
            // Try alternate path
            return Ok(());
        }

        let mut stream = cf.open_stream(data_path).map_err(|e| {
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

        // Read board parameters
        self.board_params = read_parameters_block(&mut cursor)?;

        Ok(())
    }

    /// Read the Components storage.
    fn read_components<R: Read + Seek>(&mut self, cf: &mut CompoundFile<R>) -> Result<()> {
        let data_path = "/Components6/Data";

        if cf.entry(data_path).is_err() {
            return Ok(());
        }

        let mut stream = cf.open_stream(data_path).map_err(|e| {
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

        // Read components
        while (cursor.position() as usize) < data.len() {
            match self.read_component_record(&mut cursor) {
                Ok(comp) => self.components.push(comp),
                Err(_) => break,
            }
        }

        Ok(())
    }

    /// Read a single component record.
    fn read_component_record<R: Read>(&self, reader: &mut R) -> Result<PcbDocComponent> {
        let params = read_parameters_block(reader)?;

        Ok(PcbDocComponent {
            // PcbDoc uses SOURCEDESIGNATOR for the placed component's designator
            designator: params
                .get("SOURCEDESIGNATOR")
                .or_else(|| params.get("DESIGNATOR"))
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            pattern: params
                .get("PATTERN")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            comment: params
                .get("COMMENT")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            params,
            primitives: Vec::new(),
        })
    }

    /// Read board primitives (tracks, arcs, vias, etc.).
    fn read_primitives<R: Read + Seek>(&mut self, cf: &mut CompoundFile<R>) -> Result<()> {
        use byteorder::ReadBytesExt;

        // Try to read from various primitive storages
        self.read_primitive_storage(cf, "/Tracks6/Data", |cursor, _| {
            let record_id = cursor.read_u8()?;
            if record_id != PcbObjectId::Track.to_byte() {
                return Err(AltiumError::InvalidRecord(format!(
                    "Expected Track record ID (4), got {}",
                    record_id
                )));
            }
            let block = read_block(cursor)?;
            let mut block_cursor = Cursor::new(&block);
            <PcbTrack as FromBinary>::read_from(&mut block_cursor).map(PcbRecord::Track)
        })?;

        self.read_primitive_storage(cf, "/Arcs6/Data", |cursor, _| {
            let record_id = cursor.read_u8()?;
            if record_id != PcbObjectId::Arc.to_byte() {
                return Err(AltiumError::InvalidRecord(format!(
                    "Expected Arc record ID (1), got {}",
                    record_id
                )));
            }
            let block = read_block(cursor)?;
            let mut block_cursor = Cursor::new(&block);
            <PcbArc as FromBinary>::read_from(&mut block_cursor).map(PcbRecord::Arc)
        })?;

        self.read_primitive_storage(cf, "/Vias6/Data", |cursor, _| {
            let record_id = cursor.read_u8()?;
            if record_id != PcbObjectId::Via.to_byte() {
                return Err(AltiumError::InvalidRecord(format!(
                    "Expected Via record ID (3), got {}",
                    record_id
                )));
            }
            let block = read_block(cursor)?;
            let mut block_cursor = Cursor::new(&block);
            <PcbVia as FromBinary>::read_from(&mut block_cursor).map(PcbRecord::Via)
        })?;

        self.read_primitive_storage(cf, "/Fills6/Data", |cursor, _| {
            let record_id = cursor.read_u8()?;
            if record_id != PcbObjectId::Fill.to_byte() {
                return Err(AltiumError::InvalidRecord(format!(
                    "Expected Fill record ID (6), got {}",
                    record_id
                )));
            }
            let block = read_block(cursor)?;
            let mut block_cursor = Cursor::new(&block);
            <PcbFill as FromBinary>::read_from(&mut block_cursor).map(PcbRecord::Fill)
        })?;

        self.read_primitive_storage(cf, "/Regions6/Data", |cursor, _| {
            let record_id = cursor.read_u8()?;
            if record_id != PcbObjectId::Region.to_byte() {
                return Err(AltiumError::InvalidRecord(format!(
                    "Expected Region record ID (11), got {}",
                    record_id
                )));
            }
            let block = read_block(cursor)?;
            let mut block_cursor = Cursor::new(&block);
            <PcbRegion as FromBinary>::read_from(&mut block_cursor).map(PcbRecord::Region)
        })?;

        // Read polygons (copper pours)
        // Note: Polygons6/Data uses parameter format without a record ID byte prefix.
        // Each record is [i32 size][parameter_string] (same as Components6/Data).
        self.read_primitive_storage(cf, "/Polygons6/Data", |cursor, _| {
            let params = read_parameters_block(cursor)?;
            Ok(PcbRecord::Polygon(PcbPolygon::from_params(&params)))
        })?;

        // Read texts
        self.read_primitive_storage(cf, "/Texts6/Data", |cursor, _| {
            let record_id = cursor.read_u8()?;
            if record_id != PcbObjectId::Text.to_byte() {
                return Err(AltiumError::InvalidRecord(format!(
                    "Expected Text record ID (5), got {}",
                    record_id
                )));
            }
            let block = read_block(cursor)?;
            let mut block_cursor = Cursor::new(&block);
            <PcbText as FromBinary>::read_from(&mut block_cursor).map(PcbRecord::Text)
        })?;

        // Read dimensions
        // Dimensions6/Data uses a 2-byte header [u8 version][u8 flags] before each
        // parameter block: [version][flags][i32 size][parameter_string]
        self.read_primitive_storage(cf, "/Dimensions6/Data", |cursor, _| {
            let _version = cursor.read_u8()?;
            let _flags = cursor.read_u8()?;
            let params = read_parameters_block(cursor)?;
            Ok(PcbRecord::Dimension(Box::new(PcbDimension::from_params(
                &params,
            ))))
        })?;

        // Read coordinates
        // Coordinates6/Data format: assumed to use the same 2-byte header as
        // Dimensions6/Data ([u8 version][u8 flags][i32 size][parameter_string]).
        // Not yet verified from real data (all test files have empty streams).
        self.read_primitive_storage(cf, "/Coordinates6/Data", |cursor, _| {
            let _version = cursor.read_u8()?;
            let _flags = cursor.read_u8()?;
            let params = read_parameters_block(cursor)?;
            Ok(PcbRecord::Coordinate(PcbCoordinate::from_params(&params)))
        })?;

        Ok(())
    }

    /// Read a primitive storage stream.
    fn read_primitive_storage<R, F>(
        &mut self,
        cf: &mut CompoundFile<R>,
        path: &str,
        reader_fn: F,
    ) -> Result<()>
    where
        R: Read + Seek,
        F: Fn(&mut Cursor<&Vec<u8>>, usize) -> Result<PcbRecord>,
    {
        if cf.entry(path).is_err() {
            return Ok(());
        }

        let mut stream = cf.open_stream(path).map_err(|e| {
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
        let mut index = 0;

        while (cursor.position() as usize) < data.len() {
            match reader_fn(&mut cursor, index) {
                Ok(record) => self.primitives.push(record),
                Err(_) => break,
            }
            index += 1;
        }

        Ok(())
    }

    /// Read the Nets storage.
    fn read_nets<R: Read + Seek>(&mut self, cf: &mut CompoundFile<R>) -> Result<()> {
        let data_path = "/Nets6/Data";

        if cf.entry(data_path).is_err() {
            return Ok(());
        }

        let mut stream = cf.open_stream(data_path).map_err(|e| {
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

        while (cursor.position() as usize) < data.len() {
            match read_parameters_block(&mut cursor) {
                Ok(params) => {
                    if let Some(name) = params.get("NAME") {
                        self.nets.push(name.as_str().to_string());
                    }
                }
                Err(_) => break,
            }
        }

        Ok(())
    }

    /// Read the Rules storage.
    fn read_rules<R: Read + Seek>(&mut self, cf: &mut CompoundFile<R>) -> Result<()> {
        let data_path = "/Rules6/Data";

        if cf.entry(data_path).is_err() {
            return Ok(());
        }

        let mut stream = cf.open_stream(data_path).map_err(|e| {
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

        while (cursor.position() as usize) < data.len() {
            match PcbRule::read_from(&mut cursor) {
                Ok(rule) => self.rules.push(rule),
                Err(_) => break,
            }
        }

        Ok(())
    }

    /// Read the Classes storage.
    fn read_classes<R: Read + Seek>(&mut self, cf: &mut CompoundFile<R>) -> Result<()> {
        let data_path = "/Classes6/Data";

        if cf.entry(data_path).is_err() {
            return Ok(());
        }

        let mut stream = cf.open_stream(data_path).map_err(|e| {
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

        while (cursor.position() as usize) < data.len() {
            match read_parameters_block(&mut cursor) {
                Ok(params) => {
                    let class = PcbClass::from_params(&params);
                    self.classes.push(class);
                }
                Err(_) => break,
            }
        }

        Ok(())
    }

    /// Read various options streams.
    fn read_options<R: Read + Seek>(&mut self, cf: &mut CompoundFile<R>) -> Result<()> {
        // Read Advanced Placer Options
        if let Ok(params) = Self::read_options_stream(cf, "/Advanced Placer Options6/Data") {
            self.placer_options = Some(PcbAdvancedPlacerOptions::from_params(&params));
        }

        // Read DRC Options
        if let Ok(params) = Self::read_options_stream(cf, "/Design Rule Checker Options6/Data") {
            self.drc_options = Some(PcbDrcOptions::from_params(&params));
        }

        // Read Pin Swap Options
        if let Ok(params) = Self::read_options_stream(cf, "/Pin Swap Options6/Data") {
            self.pin_swap_options = Some(PcbPinSwapOptions::from_params(&params));
        }

        Ok(())
    }

    /// Read a single options stream as parameters.
    fn read_options_stream<R: Read + Seek>(
        cf: &mut CompoundFile<R>,
        path: &str,
    ) -> Result<ParameterCollection> {
        if cf.entry(path).is_err() {
            return Err(AltiumError::Parse(format!("Stream not found: {}", path)));
        }

        let mut stream = cf.open_stream(path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        let mut data = Vec::new();
        stream.read_to_end(&mut data)?;

        if data.is_empty() {
            return Err(AltiumError::Parse("Empty stream".to_string()));
        }

        let mut cursor = Cursor::new(&data);
        read_parameters_block(&mut cursor)
    }

    /// Save the PcbDoc to a file path.
    ///
    /// This performs a read-modify-write operation: it reads the existing file,
    /// updates the rules stream, and writes back to the same path.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // Read the existing file
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.as_ref())?;

        let mut cf = CompoundFile::open(file).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?;

        // Write rules
        self.write_rules(&mut cf)?;

        // Write nets
        self.write_nets(&mut cf)?;

        cf.flush()
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Write rules to the CFB file.
    fn write_rules<R: Read + Write + Seek>(&self, cf: &mut CompoundFile<R>) -> Result<()> {
        let data_path = "/Rules6/Data";

        // Serialize all rules to a buffer
        let mut buffer = Vec::new();
        for rule in &self.rules {
            rule.write_to(&mut buffer)?;
        }

        // Check if stream exists, create if needed
        if cf.entry(data_path).is_err() {
            // For now, just fail if the stream doesn't exist
            // A full implementation would create the stream
            return Err(AltiumError::Parse(
                "Rules6/Data stream not found".to_string(),
            ));
        }

        // Open and truncate the stream
        let mut stream = cf.open_stream(data_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        // Seek to beginning and write
        stream.seek(SeekFrom::Start(0))?;
        stream.write_all(&buffer)?;

        // If new content is shorter, we need to truncate
        // cfb crate's stream should handle this, but let's be safe
        let new_len = buffer.len() as u64;
        stream
            .set_len(new_len)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Write nets to the CFB file.
    fn write_nets<R: Read + Write + Seek>(&self, cf: &mut CompoundFile<R>) -> Result<()> {
        let data_path = "/Nets6/Data";

        if self.nets.is_empty() {
            return Ok(());
        }

        // Serialize all nets to a buffer
        let mut buffer = Vec::new();
        for net_name in &self.nets {
            let mut params = ParameterCollection::new();
            params.add("NAME", net_name);
            write_parameters_block(&mut buffer, &params)?;
        }

        // Check if stream exists, create if needed
        if cf.entry(data_path).is_err() {
            // Create the Nets6 storage and Data stream
            cf.create_storage("/Nets6").map_err(|e| {
                AltiumError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;
            cf.create_stream(data_path).map_err(|e| {
                AltiumError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;
        }

        // Open and write the stream
        let mut stream = cf.open_stream(data_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        stream.seek(SeekFrom::Start(0))?;
        stream.write_all(&buffer)?;

        let new_len = buffer.len() as u64;
        stream
            .set_len(new_len)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Save board parameters to a file path.
    ///
    /// This performs a read-modify-write operation: it reads the existing file,
    /// updates the Board6/Data stream, and writes back.
    pub fn save_board_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.as_ref())?;

        let mut cf = CompoundFile::open(file).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?;

        self.write_board(&mut cf)?;

        cf.flush()
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Write board data to the CFB file.
    fn write_board<R: Read + Write + Seek>(&self, cf: &mut CompoundFile<R>) -> Result<()> {
        use crate::io::writer::write_parameters_block;

        let data_path = "/Board6/Data";

        // Check if stream exists
        if cf.entry(data_path).is_err() {
            return Err(AltiumError::Parse(
                "Board6/Data stream not found".to_string(),
            ));
        }

        // Serialize board params to a buffer
        let mut buffer = Vec::new();
        write_parameters_block(&mut buffer, &self.board_params)?;

        // Open and write the stream
        let mut stream = cf.open_stream(data_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        stream.seek(SeekFrom::Start(0))?;
        stream.write_all(&buffer)?;

        let new_len = buffer.len() as u64;
        stream
            .set_len(new_len)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Save regions (keepouts/cutouts) to a file path.
    ///
    /// This performs a read-modify-write operation.
    pub fn save_regions_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        use crate::io::writer::write_block;
        use crate::traits::ToBinary;

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.as_ref())?;

        let mut cf = CompoundFile::open(file).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?;

        let data_path = "/Regions6/Data";

        // Check if stream exists
        if cf.entry(data_path).is_err() {
            return Err(AltiumError::Parse(
                "Regions6/Data stream not found".to_string(),
            ));
        }

        // Serialize all regions to a buffer
        let mut buffer = Vec::new();
        for prim in &self.primitives {
            if let PcbRecord::Region(r) = prim {
                let mut region_data = Vec::new();
                r.write_to(&mut region_data)?;
                write_block(&mut buffer, &region_data, 0)?;
            }
        }

        // Open and write the stream
        let mut stream = cf.open_stream(data_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        stream.seek(SeekFrom::Start(0))?;
        stream.write_all(&buffer)?;

        let new_len = buffer.len() as u64;
        stream
            .set_len(new_len)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        cf.flush()
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Save polygons (copper pours) to a file path.
    ///
    /// This performs a read-modify-write operation.
    pub fn save_polygons_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.as_ref())?;

        let mut cf = CompoundFile::open(file).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?;

        let data_path = "/Polygons6/Data";

        // Check if stream exists
        if cf.entry(data_path).is_err() {
            return Err(AltiumError::Parse(
                "Polygons6/Data stream not found".to_string(),
            ));
        }

        // Serialize all polygons to a buffer
        let mut buffer = Vec::new();
        for prim in &self.primitives {
            if let PcbRecord::Polygon(p) = prim {
                let params = p.to_params();
                write_parameters_block(&mut buffer, &params)?;
            }
        }

        // Open and write the stream
        let mut stream = cf.open_stream(data_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        stream.seek(SeekFrom::Start(0))?;
        stream.write_all(&buffer)?;

        let new_len = buffer.len() as u64;
        stream
            .set_len(new_len)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        cf.flush()
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Get the number of components.
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Get the number of primitives.
    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }

    /// Get the number of nets.
    pub fn net_count(&self) -> usize {
        self.nets.len()
    }

    /// Get the number of design rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Iterate over design rules.
    pub fn iter_rules(&self) -> impl Iterator<Item = &PcbRule> {
        self.rules.iter()
    }

    /// Iterate over design rules mutably.
    pub fn iter_rules_mut(&mut self) -> impl Iterator<Item = &mut PcbRule> {
        self.rules.iter_mut()
    }

    /// Add a design rule.
    pub fn add_rule(&mut self, rule: PcbRule) {
        self.rules.push(rule);
    }

    /// Find a rule by name.
    pub fn find_rule(&self, name: &str) -> Option<&PcbRule> {
        self.rules.iter().find(|r| r.name == name)
    }

    /// Find a rule by name mutably.
    pub fn find_rule_mut(&mut self, name: &str) -> Option<&mut PcbRule> {
        self.rules.iter_mut().find(|r| r.name == name)
    }

    /// Iterate over components.
    pub fn iter_components(&self) -> impl Iterator<Item = &PcbDocComponent> {
        self.components.iter()
    }

    /// Iterate over primitives.
    pub fn iter_primitives(&self) -> impl Iterator<Item = &PcbRecord> {
        self.primitives.iter()
    }

    /// Count tracks.
    pub fn track_count(&self) -> usize {
        self.primitives
            .iter()
            .filter(|p| matches!(p, PcbRecord::Track(_)))
            .count()
    }

    /// Count vias.
    pub fn via_count(&self) -> usize {
        self.primitives
            .iter()
            .filter(|p| matches!(p, PcbRecord::Via(_)))
            .count()
    }

    /// Count pads (from components).
    pub fn pad_count(&self) -> usize {
        self.components
            .iter()
            .flat_map(|c| &c.primitives)
            .filter(|p| matches!(p, PcbRecord::Pad(_)))
            .count()
    }

    /// Find a component by designator.
    pub fn find_component(&self, designator: &str) -> Option<&PcbDocComponent> {
        self.components
            .iter()
            .find(|c| c.designator.eq_ignore_ascii_case(designator))
    }

    /// Find a component by designator mutably.
    pub fn find_component_mut(&mut self, designator: &str) -> Option<&mut PcbDocComponent> {
        self.components
            .iter_mut()
            .find(|c| c.designator.eq_ignore_ascii_case(designator))
    }

    /// Iterate over components mutably.
    pub fn iter_components_mut(&mut self) -> impl Iterator<Item = &mut PcbDocComponent> {
        self.components.iter_mut()
    }

    /// Write components to the CFB file.
    fn write_components<R: Read + Write + Seek>(&self, cf: &mut CompoundFile<R>) -> Result<()> {
        use crate::io::writer::write_parameters_block;

        let data_path = "/Components6/Data";

        // Check if stream exists
        if cf.entry(data_path).is_err() {
            return Err(AltiumError::Parse(
                "Components6/Data stream not found".to_string(),
            ));
        }

        // Serialize all components to a buffer
        let mut buffer = Vec::new();
        for component in &self.components {
            write_parameters_block(&mut buffer, &component.params)?;
        }

        // Open and truncate the stream
        let mut stream = cf.open_stream(data_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        // Seek to beginning and write
        stream.seek(SeekFrom::Start(0))?;
        stream.write_all(&buffer)?;

        // Truncate to new length
        let new_len = buffer.len() as u64;
        stream
            .set_len(new_len)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Save with component changes.
    pub fn save_with_components<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // Read the existing file
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.as_ref())?;

        let mut cf = CompoundFile::open(file).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?;

        // Write rules
        self.write_rules(&mut cf)?;

        // Write components
        self.write_components(&mut cf)?;

        cf.flush()
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Save all primitives to a file path.
    ///
    /// This comprehensive save method writes all primitive types:
    /// - Tracks
    /// - Vias
    /// - Arcs
    /// - Fills
    /// - Regions
    /// - Polygons
    /// - Dimensions
    /// - Coordinates
    /// - Components
    /// - Rules
    pub fn save_all_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.as_ref())?;

        let mut cf = CompoundFile::open(file).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?;

        // Write tracks
        self.write_tracks(&mut cf)?;

        // Write vias
        self.write_vias(&mut cf)?;

        // Write arcs
        self.write_arcs(&mut cf)?;

        // Write fills
        self.write_fills(&mut cf)?;

        // Write regions
        self.write_regions_internal(&mut cf)?;

        // Write polygons
        self.write_polygons_internal(&mut cf)?;

        // Write dimensions
        self.write_dimensions(&mut cf)?;

        // Write coordinates
        self.write_coordinates(&mut cf)?;

        // Write pads
        self.write_pads(&mut cf)?;

        // Write texts
        self.write_texts(&mut cf)?;

        // Write rules
        self.write_rules(&mut cf)?;

        // Write components
        self.write_components(&mut cf)?;

        // Write nets
        self.write_nets(&mut cf)?;

        cf.flush()
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Write tracks to the CFB file.
    fn write_tracks<R: Read + Write + Seek>(&self, cf: &mut CompoundFile<R>) -> Result<()> {
        use crate::io::writer::write_block;
        use crate::traits::ToBinary;
        use byteorder::WriteBytesExt;

        let data_path = "/Tracks6/Data";

        if cf.entry(data_path).is_err() {
            return Ok(()); // Stream doesn't exist, skip
        }

        let mut buffer = Vec::new();
        for prim in &self.primitives {
            if let PcbRecord::Track(track) = prim {
                // Write RecordID byte
                buffer.write_u8(PcbObjectId::Track.to_byte())?;
                // Write size and data
                let mut track_data = Vec::new();
                track.write_to(&mut track_data)?;
                write_block(&mut buffer, &track_data, 0)?;
            }
        }

        let mut stream = cf.open_stream(data_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        stream.seek(SeekFrom::Start(0))?;
        stream.write_all(&buffer)?;
        stream
            .set_len(buffer.len() as u64)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Write vias to the CFB file.
    fn write_vias<R: Read + Write + Seek>(&self, cf: &mut CompoundFile<R>) -> Result<()> {
        use crate::io::writer::write_block;
        use crate::traits::ToBinary;
        use byteorder::WriteBytesExt;

        let data_path = "/Vias6/Data";

        if cf.entry(data_path).is_err() {
            return Ok(());
        }

        let mut buffer = Vec::new();
        for prim in &self.primitives {
            if let PcbRecord::Via(via) = prim {
                // Write RecordID byte
                buffer.write_u8(PcbObjectId::Via.to_byte())?;
                // Write size and data
                let mut via_data = Vec::new();
                via.write_to(&mut via_data)?;
                write_block(&mut buffer, &via_data, 0)?;
            }
        }

        let mut stream = cf.open_stream(data_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        stream.seek(SeekFrom::Start(0))?;
        stream.write_all(&buffer)?;
        stream
            .set_len(buffer.len() as u64)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Write arcs to the CFB file.
    fn write_arcs<R: Read + Write + Seek>(&self, cf: &mut CompoundFile<R>) -> Result<()> {
        use crate::io::writer::write_block;
        use crate::traits::ToBinary;
        use byteorder::WriteBytesExt;

        let data_path = "/Arcs6/Data";

        if cf.entry(data_path).is_err() {
            return Ok(());
        }

        let mut buffer = Vec::new();
        for prim in &self.primitives {
            if let PcbRecord::Arc(arc) = prim {
                // Write RecordID byte
                buffer.write_u8(PcbObjectId::Arc.to_byte())?;
                // Write size and data
                let mut arc_data = Vec::new();
                arc.write_to(&mut arc_data)?;
                write_block(&mut buffer, &arc_data, 0)?;
            }
        }

        let mut stream = cf.open_stream(data_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        stream.seek(SeekFrom::Start(0))?;
        stream.write_all(&buffer)?;
        stream
            .set_len(buffer.len() as u64)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Write fills to the CFB file.
    fn write_fills<R: Read + Write + Seek>(&self, cf: &mut CompoundFile<R>) -> Result<()> {
        use crate::io::writer::write_block;
        use crate::traits::ToBinary;
        use byteorder::WriteBytesExt;

        let data_path = "/Fills6/Data";

        if cf.entry(data_path).is_err() {
            return Ok(());
        }

        let mut buffer = Vec::new();
        for prim in &self.primitives {
            if let PcbRecord::Fill(fill) = prim {
                // Write RecordID byte
                buffer.write_u8(PcbObjectId::Fill.to_byte())?;
                // Write size and data
                let mut fill_data = Vec::new();
                fill.write_to(&mut fill_data)?;
                write_block(&mut buffer, &fill_data, 0)?;
            }
        }

        let mut stream = cf.open_stream(data_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        stream.seek(SeekFrom::Start(0))?;
        stream.write_all(&buffer)?;
        stream
            .set_len(buffer.len() as u64)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Write pads to the CFB file.
    fn write_pads<R: Read + Write + Seek>(&self, cf: &mut CompoundFile<R>) -> Result<()> {
        use crate::io::writer::write_block;
        use crate::traits::ToBinary;
        use byteorder::WriteBytesExt;

        let data_path = "/Pads6/Data";

        if cf.entry(data_path).is_err() {
            return Ok(());
        }

        let mut buffer = Vec::new();
        for prim in &self.primitives {
            if let PcbRecord::Pad(pad) = prim {
                // Write RecordID byte
                buffer.write_u8(PcbObjectId::Pad.to_byte())?;
                // Write size and data
                let mut pad_data = Vec::new();
                pad.write_to(&mut pad_data)?;
                write_block(&mut buffer, &pad_data, 0)?;
            }
        }

        let mut stream = cf.open_stream(data_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        stream.seek(SeekFrom::Start(0))?;
        stream.write_all(&buffer)?;
        stream
            .set_len(buffer.len() as u64)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Write texts to the CFB file.
    fn write_texts<R: Read + Write + Seek>(&self, cf: &mut CompoundFile<R>) -> Result<()> {
        use crate::io::writer::write_block;
        use crate::traits::ToBinary;
        use byteorder::WriteBytesExt;

        let data_path = "/Texts6/Data";

        if cf.entry(data_path).is_err() {
            return Ok(());
        }

        let mut buffer = Vec::new();
        for prim in &self.primitives {
            if let PcbRecord::Text(text) = prim {
                // Write RecordID byte
                buffer.write_u8(PcbObjectId::Text.to_byte())?;
                // Write size and data
                let mut text_data = Vec::new();
                text.write_to(&mut text_data)?;
                write_block(&mut buffer, &text_data, 0)?;
            }
        }

        let mut stream = cf.open_stream(data_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        stream.seek(SeekFrom::Start(0))?;
        stream.write_all(&buffer)?;
        stream
            .set_len(buffer.len() as u64)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Write regions to the CFB file (internal method).
    fn write_regions_internal<R: Read + Write + Seek>(
        &self,
        cf: &mut CompoundFile<R>,
    ) -> Result<()> {
        use crate::io::writer::write_block;
        use crate::traits::ToBinary;
        use byteorder::WriteBytesExt;

        let data_path = "/Regions6/Data";

        if cf.entry(data_path).is_err() {
            return Ok(());
        }

        let mut buffer = Vec::new();
        for prim in &self.primitives {
            if let PcbRecord::Region(region) = prim {
                // Write RecordID byte
                buffer.write_u8(PcbObjectId::Region.to_byte())?;
                // Write size and data
                let mut region_data = Vec::new();
                region.write_to(&mut region_data)?;
                write_block(&mut buffer, &region_data, 0)?;
            }
        }

        let mut stream = cf.open_stream(data_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        stream.seek(SeekFrom::Start(0))?;
        stream.write_all(&buffer)?;
        stream
            .set_len(buffer.len() as u64)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Write polygons to the CFB file (internal method).
    fn write_polygons_internal<R: Read + Write + Seek>(
        &self,
        cf: &mut CompoundFile<R>,
    ) -> Result<()> {
        let data_path = "/Polygons6/Data";

        if cf.entry(data_path).is_err() {
            return Ok(());
        }

        let mut buffer = Vec::new();
        for prim in &self.primitives {
            if let PcbRecord::Polygon(polygon) = prim {
                let params = polygon.to_params();
                write_parameters_block(&mut buffer, &params)?;
            }
        }

        let mut stream = cf.open_stream(data_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        stream.seek(SeekFrom::Start(0))?;
        stream.write_all(&buffer)?;
        stream
            .set_len(buffer.len() as u64)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Write dimensions to the CFB file.
    fn write_dimensions<R: Read + Write + Seek>(&self, cf: &mut CompoundFile<R>) -> Result<()> {
        use byteorder::WriteBytesExt;

        let data_path = "/Dimensions6/Data";

        if cf.entry(data_path).is_err() {
            return Ok(());
        }

        let mut buffer = Vec::new();
        for prim in &self.primitives {
            if let PcbRecord::Dimension(dim) = prim {
                // Write 2-byte header: version=1, flags=0
                buffer.write_u8(0x01)?;
                buffer.write_u8(0x00)?;
                // Write parameter block
                let params = dim.to_params();
                write_parameters_block(&mut buffer, &params)?;
            }
        }

        let mut stream = cf.open_stream(data_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        stream.seek(SeekFrom::Start(0))?;
        stream.write_all(&buffer)?;
        stream
            .set_len(buffer.len() as u64)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Write coordinates to the CFB file.
    fn write_coordinates<R: Read + Write + Seek>(&self, cf: &mut CompoundFile<R>) -> Result<()> {
        use byteorder::WriteBytesExt;

        let data_path = "/Coordinates6/Data";

        if cf.entry(data_path).is_err() {
            return Ok(());
        }

        let mut buffer = Vec::new();
        for prim in &self.primitives {
            if let PcbRecord::Coordinate(coord) = prim {
                // Write 2-byte header (assumed same as Dimensions6/Data)
                buffer.write_u8(0x01)?;
                buffer.write_u8(0x00)?;
                // Write parameter block
                let params = coord.to_params();
                write_parameters_block(&mut buffer, &params)?;
            }
        }

        let mut stream = cf.open_stream(data_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        stream.seek(SeekFrom::Start(0))?;
        stream.write_all(&buffer)?;
        stream
            .set_len(buffer.len() as u64)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Count arcs.
    pub fn arc_count(&self) -> usize {
        self.primitives
            .iter()
            .filter(|p| matches!(p, PcbRecord::Arc(_)))
            .count()
    }

    /// Count fills.
    pub fn fill_count(&self) -> usize {
        self.primitives
            .iter()
            .filter(|p| matches!(p, PcbRecord::Fill(_)))
            .count()
    }

    /// Count regions.
    pub fn region_count(&self) -> usize {
        self.primitives
            .iter()
            .filter(|p| matches!(p, PcbRecord::Region(_)))
            .count()
    }

    /// Count polygons.
    pub fn polygon_count(&self) -> usize {
        self.primitives
            .iter()
            .filter(|p| matches!(p, PcbRecord::Polygon(_)))
            .count()
    }

    /// Count text elements.
    pub fn text_count(&self) -> usize {
        self.primitives
            .iter()
            .filter(|p| matches!(p, PcbRecord::Text(_)))
            .count()
    }

    /// Count dimensions.
    pub fn dimension_count(&self) -> usize {
        self.primitives
            .iter()
            .filter(|p| matches!(p, PcbRecord::Dimension(_)))
            .count()
    }

    /// Count coordinates.
    pub fn coordinate_count(&self) -> usize {
        self.primitives
            .iter()
            .filter(|p| matches!(p, PcbRecord::Coordinate(_)))
            .count()
    }

    /// Add a track.
    pub fn add_track(&mut self, track: PcbTrack) {
        self.primitives.push(PcbRecord::Track(track));
    }

    /// Add a via.
    pub fn add_via(&mut self, via: PcbVia) {
        self.primitives.push(PcbRecord::Via(via));
    }

    /// Add an arc.
    pub fn add_arc(&mut self, arc: PcbArc) {
        self.primitives.push(PcbRecord::Arc(arc));
    }

    /// Add a fill.
    pub fn add_fill(&mut self, fill: PcbFill) {
        self.primitives.push(PcbRecord::Fill(fill));
    }

    /// Add a region.
    pub fn add_region(&mut self, region: PcbRegion) {
        self.primitives.push(PcbRecord::Region(region));
    }

    /// Add a polygon.
    pub fn add_polygon(&mut self, polygon: PcbPolygon) {
        self.primitives.push(PcbRecord::Polygon(polygon));
    }

    /// Add a dimension.
    pub fn add_dimension(&mut self, dimension: PcbDimension) {
        self.primitives
            .push(PcbRecord::Dimension(Box::new(dimension)));
    }

    /// Add a coordinate.
    pub fn add_coordinate(&mut self, coordinate: PcbCoordinate) {
        self.primitives.push(PcbRecord::Coordinate(coordinate));
    }

    /// Remove primitive at index.
    pub fn remove_primitive(&mut self, index: usize) -> Option<PcbRecord> {
        if index < self.primitives.len() {
            Some(self.primitives.remove(index))
        } else {
            None
        }
    }

    /// Get primitive at index.
    pub fn get_primitive(&self, index: usize) -> Option<&PcbRecord> {
        self.primitives.get(index)
    }

    /// Get mutable primitive at index.
    pub fn get_primitive_mut(&mut self, index: usize) -> Option<&mut PcbRecord> {
        self.primitives.get_mut(index)
    }

    /// Iterate over tracks.
    pub fn iter_tracks(&self) -> impl Iterator<Item = &PcbTrack> {
        self.primitives.iter().filter_map(|p| {
            if let PcbRecord::Track(t) = p {
                Some(t)
            } else {
                None
            }
        })
    }

    /// Iterate over vias.
    pub fn iter_vias(&self) -> impl Iterator<Item = &PcbVia> {
        self.primitives.iter().filter_map(|p| {
            if let PcbRecord::Via(v) = p {
                Some(v)
            } else {
                None
            }
        })
    }

    /// Iterate over arcs.
    pub fn iter_arcs(&self) -> impl Iterator<Item = &PcbArc> {
        self.primitives.iter().filter_map(|p| {
            if let PcbRecord::Arc(a) = p {
                Some(a)
            } else {
                None
            }
        })
    }

    /// Iterate over fills.
    pub fn iter_fills(&self) -> impl Iterator<Item = &PcbFill> {
        self.primitives.iter().filter_map(|p| {
            if let PcbRecord::Fill(f) = p {
                Some(f)
            } else {
                None
            }
        })
    }

    /// Iterate over regions.
    pub fn iter_regions(&self) -> impl Iterator<Item = &PcbRegion> {
        self.primitives.iter().filter_map(|p| {
            if let PcbRecord::Region(r) = p {
                Some(r)
            } else {
                None
            }
        })
    }

    /// Iterate over polygons.
    pub fn iter_polygons(&self) -> impl Iterator<Item = &PcbPolygon> {
        self.primitives.iter().filter_map(|p| {
            if let PcbRecord::Polygon(pol) = p {
                Some(pol)
            } else {
                None
            }
        })
    }

    /// Iterate over texts.
    pub fn iter_texts(&self) -> impl Iterator<Item = &PcbText> {
        self.primitives.iter().filter_map(|p| {
            if let PcbRecord::Text(t) = p {
                Some(t)
            } else {
                None
            }
        })
    }

    /// Iterate over dimensions.
    pub fn iter_dimensions(&self) -> impl Iterator<Item = &PcbDimension> {
        self.primitives.iter().filter_map(|p| {
            if let PcbRecord::Dimension(d) = p {
                Some(d.as_ref())
            } else {
                None
            }
        })
    }

    /// Iterate over coordinates.
    pub fn iter_coordinates(&self) -> impl Iterator<Item = &PcbCoordinate> {
        self.primitives.iter().filter_map(|p| {
            if let PcbRecord::Coordinate(c) = p {
                Some(c)
            } else {
                None
            }
        })
    }

    /// Add a text annotation.
    pub fn add_text(&mut self, text: PcbText) {
        self.primitives.push(PcbRecord::Text(text));
    }
}

impl PcbDocComponent {
    /// Get the X position of the component.
    pub fn x(&self) -> Option<crate::types::Coord> {
        self.params
            .get("X")
            .map(|v| v.as_coord_or(crate::types::Coord::ZERO))
    }

    /// Get the Y position of the component.
    pub fn y(&self) -> Option<crate::types::Coord> {
        self.params
            .get("Y")
            .map(|v| v.as_coord_or(crate::types::Coord::ZERO))
    }

    /// Get the rotation angle in degrees.
    pub fn rotation(&self) -> f64 {
        self.params
            .get("ROTATION")
            .and_then(|v| v.as_str().trim().parse::<f64>().ok())
            .unwrap_or(0.0)
    }

    /// Get the layer.
    pub fn layer(&self) -> crate::types::Layer {
        self.params
            .get("LAYER")
            .and_then(|v| {
                let layer_str = v.as_str();
                // Try exact match first
                crate::types::Layer::from_name(layer_str).or_else(|| {
                    // Try common aliases
                    match layer_str.to_uppercase().as_str() {
                        "TOP" => Some(crate::types::Layer::TOP_LAYER),
                        "BOTTOM" => Some(crate::types::Layer::BOTTOM_LAYER),
                        "TOPOVERLAY" | "TOP_OVERLAY" => Some(crate::types::Layer::TOP_OVERLAY),
                        "BOTTOMOVERLAY" | "BOTTOM_OVERLAY" => {
                            Some(crate::types::Layer::BOTTOM_OVERLAY)
                        }
                        _ => None,
                    }
                })
            })
            .unwrap_or(crate::types::Layer::TOP_LAYER)
    }

    /// Set the X position of the component.
    pub fn set_x(&mut self, x: crate::types::Coord) {
        self.params.add_coord("X", x);
    }

    /// Set the Y position of the component.
    pub fn set_y(&mut self, y: crate::types::Coord) {
        self.params.add_coord("Y", y);
    }

    /// Set the position of the component.
    pub fn set_position(&mut self, x: crate::types::Coord, y: crate::types::Coord) {
        self.set_x(x);
        self.set_y(y);
    }

    /// Set the rotation angle in degrees.
    pub fn set_rotation(&mut self, rotation: f64) {
        // Format as scientific notation like Altium does
        self.params.add("ROTATION", &format!("{:.14E}", rotation));
    }

    /// Set the layer.
    pub fn set_layer(&mut self, layer: crate::types::Layer) {
        self.params.add("LAYER", layer.name());
    }
}

impl DumpTree for PcbDoc {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.root(&format!(
            "PcbDoc ({} components, {} primitives, {} rules)",
            self.components.len(),
            self.primitives.len(),
            self.rules.len()
        ));

        // Summary
        tree.push(
            !self.components.is_empty() || !self.primitives.is_empty() || !self.rules.is_empty(),
        );
        tree.add_leaf(
            "Summary",
            &[
                ("components", format!("{}", self.components.len())),
                ("tracks", format!("{}", self.track_count())),
                ("vias", format!("{}", self.via_count())),
                ("nets", format!("{}", self.nets.len())),
                ("rules", format!("{}", self.rules.len())),
                ("primitives", format!("{}", self.primitives.len())),
            ],
        );
        tree.pop();

        // Components
        if !self.components.is_empty() {
            tree.push(!self.primitives.is_empty());
            tree.begin_node(&format!("Components ({})", self.components.len()));
            for (i, comp) in self.components.iter().enumerate() {
                tree.push(i < self.components.len() - 1);
                comp.dump(tree);
                tree.pop();
            }
            tree.pop();
        }

        // Nets
        if !self.nets.is_empty() {
            tree.push(false);
            tree.add_leaf(
                &format!("Nets ({})", self.nets.len()),
                &[(
                    "first_few",
                    self.nets
                        .iter()
                        .take(5)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", "),
                )],
            );
            tree.pop();
        }
    }
}

impl DumpTree for PcbDocComponent {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![("designator", self.designator.clone())];
        if !self.pattern.is_empty() {
            props.push(("pattern", self.pattern.clone()));
        }
        if !self.comment.is_empty() {
            props.push(("comment", self.comment.clone()));
        }
        tree.add_leaf_with_params("Component", &props, Some(&self.params));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_classes_and_options() {
        let data = std::fs::read("data/PCB1.PcbDoc").expect("Failed to read file");
        let pcbdoc = PcbDoc::open(Cursor::new(&data)).expect("Failed to parse PcbDoc");

        // Should have classes
        assert!(!pcbdoc.classes.is_empty(), "Should have parsed classes");
        println!("Classes: {}", pcbdoc.classes.len());
        for class in &pcbdoc.classes {
            println!("  - {} ({:?})", class.name, class.kind);
        }

        // Should have placer options
        assert!(
            pcbdoc.placer_options.is_some(),
            "Should have placer options"
        );
        let opts = pcbdoc.placer_options.as_ref().unwrap();
        assert!(opts.use_rotation); // Default is true

        // Should have DRC options
        assert!(pcbdoc.drc_options.is_some(), "Should have DRC options");

        // Should have pin swap options
        assert!(
            pcbdoc.pin_swap_options.is_some(),
            "Should have pin swap options"
        );

        // Should have rules
        assert!(!pcbdoc.rules.is_empty(), "Should have rules");
        println!("Rules: {}", pcbdoc.rules.len());
    }

    #[test]
    fn test_read_dimensions() {
        let data = std::fs::read("data/Plumo-2D.PcbDoc").expect("Failed to read file");
        let pcbdoc = PcbDoc::open(Cursor::new(&data)).expect("Failed to parse PcbDoc");

        // Plumo-2D has 2 dimension annotations
        assert_eq!(
            pcbdoc.dimension_count(),
            2,
            "Should have 2 dimensions"
        );

        // Check first dimension
        let dims: Vec<_> = pcbdoc.iter_dimensions().collect();
        let dim0 = &dims[0];
        assert_eq!(
            dim0.dimension_kind,
            crate::records::pcb::DimensionKind::Linear,
            "First dimension should be Linear"
        );
        assert_eq!(dim0.references.len(), 2, "Should have 2 references");
        assert_eq!(
            dim0.references[0].object_string, "BoardOutline",
            "Reference should be BoardOutline"
        );
        assert!(!dim0.font_name.is_empty(), "Should have font name");
        assert_eq!(dim0.text_precision, 2, "Precision should be 2");

        // Check second dimension
        let dim1 = &dims[1];
        assert_eq!(
            dim1.dimension_kind,
            crate::records::pcb::DimensionKind::Linear,
            "Second dimension should be Linear"
        );
        assert_eq!(dim1.references.len(), 2, "Should have 2 references");
    }

    #[test]
    fn test_read_polygons_from_plumo() {
        let data = std::fs::read("data/Plumo-2D.PcbDoc").expect("Failed to read file");
        let pcbdoc = PcbDoc::open(Cursor::new(&data)).expect("Failed to parse PcbDoc");

        // Plumo-2D should have polygons (copper pours)
        let polygon_count = pcbdoc.polygon_count();
        assert!(
            polygon_count > 0,
            "Should have at least one polygon, got {}",
            polygon_count
        );

        // Check that polygons have vertices
        for polygon in pcbdoc.iter_polygons() {
            assert!(
                !polygon.vertices.is_empty(),
                "Polygon should have vertices"
            );
        }
    }

    #[test]
    fn test_dimension_roundtrip() {
        use crate::records::pcb::DimensionKind;

        // Create a dimension and verify parameter round-trip
        let mut params = ParameterCollection::new();
        params.add_int("OBJECTID", 13);
        params.add_int("DIMENSIONKIND", 1);
        params.add("LAYER", "MECHANICAL1");
        params.add("DIMENSIONLAYER", "MECHANICAL1");
        params.add("X1", "1000mil");
        params.add("Y1", "2000mil");
        params.add("X2", "3000mil");
        params.add("Y2", "2000mil");
        params.add_int("REFERENCES_COUNT", 1);
        params.add_int("REFERENCE0PRIM", 0);
        params.add_int("REFERENCE0OBJECTID", 25);
        params.add("REFERENCE0OBJECTSTRING", "BoardOutline");
        params.add("REFERENCE0POINTX", "1000mil");
        params.add("REFERENCE0POINTY", "2000mil");
        params.add_int("REFERENCE0ANCHOR", 3);
        params.add("TEXTPOSITION", "Auto");
        params.add_int("TEXTPRECISION", 2);
        params.add("FONTNAME", "Arial");
        params.add_bool("BOLD", false);
        params.add_bool("ITALIC", false);

        let dim = PcbDimension::from_params(&params);
        assert_eq!(dim.dimension_kind, DimensionKind::Linear);
        assert_eq!(dim.references.len(), 1);
        assert_eq!(dim.references[0].object_string, "BoardOutline");
        assert_eq!(dim.references[0].anchor, 3);
        assert_eq!(dim.text_precision, 2);
        assert_eq!(dim.font_name, "Arial");

        // Round-trip
        let params_out = dim.to_params();
        assert_eq!(
            params_out.get("DIMENSIONKIND").map(|v| v.as_int_or(0)),
            Some(1)
        );
        assert_eq!(
            params_out.get("REFERENCE0OBJECTSTRING").map(|v| v.as_str().to_string()),
            Some("BoardOutline".to_string())
        );
        assert_eq!(
            params_out.get("REFERENCES_COUNT").map(|v| v.as_int_or(0)),
            Some(1)
        );
    }
}

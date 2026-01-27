//! PcbLib reader/writer for Altium PCB footprint library files.

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use cfb::CompoundFile;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;

use crate::error::{AltiumError, Result};
use crate::io::reader::{
    read_block, read_parameters_block, read_pascal_short_string, read_pascal_string,
    read_string_block,
};
use crate::io::writer::{
    write_block, write_parameters, write_pascal_short_string, write_string_block,
};
use crate::records::pcb::{
    PcbArc, PcbComponent, PcbComponentBody, PcbFill, PcbObjectId, PcbPad, PcbRecord, PcbRegion,
    PcbText, PcbTrack, PcbVia,
};
use crate::traits::{FromBinary, ToBinary};
use crate::types::ParameterCollection;

/// A PCB footprint library containing components.
#[derive(Debug, Default)]
pub struct PcbLib {
    /// Section keys mapping pattern names to storage paths.
    section_keys: HashMap<String, String>,
    /// Unique ID of the library.
    pub unique_id: String,
    /// Components (footprints) in the library.
    pub components: Vec<PcbComponent>,
}

impl PcbLib {
    /// Open and read a PcbLib file.
    pub fn open<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut pcblib = PcbLib::default();
        let mut cf = CompoundFile::open(reader).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?;
        // Read file header
        pcblib.read_file_header(&mut cf)?;
        // Read section keys
        pcblib.read_section_keys(&mut cf)?;
        // Read library data (component list)
        pcblib.read_library(&mut cf)?;
        Ok(pcblib)
    }

    /// Open and read a PcbLib file from a path.
    pub fn open_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        Self::open(file)
    }

    /// Save the PcbLib to a file.
    pub fn save<W: Read + Write + Seek>(&self, writer: W) -> Result<()> {
        let mut cf = CompoundFile::create(writer)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        // Write FileHeader
        self.write_file_header(&mut cf)?;

        // Write SectionKeys if needed
        self.write_section_keys(&mut cf)?;

        // Write Library storage
        self.write_library(&mut cf)?;

        cf.flush()
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Save the PcbLib to a file path.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        self.save(file)
    }

    /// Write the FileHeader stream.
    fn write_file_header<F: Read + Write + Seek>(&self, cf: &mut CompoundFile<F>) -> Result<()> {
        let mut data = Vec::new();

        // Version text block
        let version = "PCB 6.0 Binary Library File";
        data.write_i32::<LittleEndian>(version.len() as i32)?;
        write_pascal_short_string(&mut data, version)?;

        // Additional required fields (observed in Altium files)
        // Float value (appears to be version-related)
        data.write_f64::<LittleEndian>(5.0)?;

        // Token string "DVLTOKCO" (required marker)
        write_pascal_short_string(&mut data, "DVLTOKCO")?;

        let stream = cf
            .create_stream("/FileHeader")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        let mut stream = stream;
        stream.write_all(&data)?;

        Ok(())
    }

    /// Write the SectionKeys stream.
    /// Maps pattern names to PCBComponent_N storage names.
    fn write_section_keys<F: Read + Write + Seek>(&self, cf: &mut CompoundFile<F>) -> Result<()> {
        // Always write section keys to map pattern names to PCBComponent_N
        if self.components.is_empty() {
            return Ok(());
        }

        let mut data = Vec::new();
        data.write_i32::<LittleEndian>(self.components.len() as i32)?;

        for (i, comp) in self.components.iter().enumerate() {
            let storage_name = format!("PCBComponent_{}", i + 1);
            crate::io::writer::write_pascal_string(&mut data, &comp.pattern)?;
            write_string_block(&mut data, &storage_name)?;
        }

        let stream = cf
            .create_stream("/SectionKeys")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        let mut stream = stream;
        stream.write_all(&data)?;

        Ok(())
    }

    /// Write the Library storage with all required sub-storages.
    fn write_library<F: Read + Write + Seek>(&self, cf: &mut CompoundFile<F>) -> Result<()> {
        // Create Library storage
        cf.create_storage("/Library")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        // Write Library/Header
        self.write_library_header(cf)?;

        // Write Library/Data with proper parameters
        self.write_library_data(cf)?;

        // Write required sub-storages
        self.write_library_substorages(cf)?;

        // Write FileVersionInfo
        self.write_file_version_info(cf)?;

        // Write each footprint using PCBComponent_N naming
        for (i, comp) in self.components.iter().enumerate() {
            let storage_name = format!("PCBComponent_{}", i + 1);
            self.write_footprint(cf, comp, &storage_name)?;
        }

        Ok(())
    }

    /// Write Library/Header stream.
    fn write_library_header<F: Read + Write + Seek>(&self, cf: &mut CompoundFile<F>) -> Result<()> {
        let mut header_data = Vec::new();
        header_data.write_u32::<LittleEndian>(1)?; // Record count = 1

        let stream = cf
            .create_stream("/Library/Header")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
        let mut stream = stream;
        stream.write_all(&header_data)?;
        Ok(())
    }

    /// Write Library/Data stream with required parameters.
    fn write_library_data<F: Read + Write + Seek>(&self, cf: &mut CompoundFile<F>) -> Result<()> {
        let mut data = Vec::new();

        // Build minimal required library parameters
        let params = Self::build_library_parameters();
        let mut params_block = Vec::new();
        write_parameters(&mut params_block, &params)?;
        write_block(&mut data, &params_block, 0)?;

        // Write footprint count
        data.write_u32::<LittleEndian>(self.components.len() as u32)?;

        // Write footprint storage names (PCBComponent_N format)
        for i in 0..self.components.len() {
            let storage_name = format!("PCBComponent_{}", i + 1);
            write_string_block(&mut data, &storage_name)?;
        }

        let stream = cf
            .create_stream("/Library/Data")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
        let mut stream = stream;
        stream.write_all(&data)?;
        Ok(())
    }

    /// Build minimal required library parameters for Library/Data.
    fn build_library_parameters() -> ParameterCollection {
        let mut params = ParameterCollection::new();

        // Essential parameters that Altium requires
        params.add("KIND", "Protel_Advanced_PCB_Library");
        params.add("VERSION", "1.0");

        // Board configuration (minimal required)
        params.add("BOARDVERSION", "5.01");
        params.add("VISIBLEGRIDMULTFACTOR", "1.000");
        params.add("BIGVISIBLEGRIDMULTFACTOR", "5.000");
        params.add("CURRENT2D3DVIEWSTATE", "2D");

        // Layer settings (minimal)
        params.add("CFG2D.CURRENTLAYER", "TOP");
        params.add("CFG2D.SHOWPADNETS", "TRUE");
        params.add("CFG2D.SHOWPADNUMBERS", "TRUE");
        params.add("CFG2D.SHOWVIANETS", "TRUE");
        params.add("CFG2D.SHOWORIGINMARKER", "TRUE");
        params.add("CFG2D.DISPLAYSPECIALSTRINGS", "FALSE");
        params.add("CFG2D.SHOWTESTPOINTS", "FALSE");
        params.add("CFG2D.SHOWSTATUSINFO", "TRUE");
        params.add("CFG2D.USETRANSPARENTLAYERS", "FALSE");
        params.add("CFG2D.PLANEDRAWMODE", "2");
        params.add("CFG2D.DISPLAYNETNAMESONTRACKS", "1");
        params.add("CFG2D.SINGLELAYERMODESTATE", "3");
        params.add("CFG2D.ORIGINMARKERCOLOR", "16777215");

        // Toggle layers (all enabled)
        params.add(
            "CFG2D.TOGGLELAYERS",
            "1111111111111111111111111111111111111111111111111111111111111111",
        );

        // Grid settings
        params.add("EGENABLED", "TRUE");
        params.add("EGRANGE", "8mil");
        params.add("OGSNAPENABLED", "TRUE");
        params.add("GRIDSNAPENABLED", "TRUE");

        params
    }

    /// Write required Library sub-storages.
    fn write_library_substorages<F: Read + Write + Seek>(
        &self,
        cf: &mut CompoundFile<F>,
    ) -> Result<()> {
        // Create empty storages for Models, Textures, ModelsNoEmbed
        for storage in &[
            "/Library/Models",
            "/Library/Textures",
            "/Library/ModelsNoEmbed",
        ] {
            cf.create_storage(storage)
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

            // Each needs Header and Data streams (empty)
            let header_path = format!("{}/Header", storage);
            let data_path = format!("{}/Data", storage);

            let mut stream = cf
                .create_stream(&header_path)
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
            stream.write_u32::<LittleEndian>(0)?;

            let mut stream = cf
                .create_stream(&data_path)
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
            stream.write_all(&[])?;
        }

        // EmbeddedFonts stream
        {
            let mut stream = cf
                .create_stream("/Library/EmbeddedFonts")
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
            stream.write_u32::<LittleEndian>(0)?;
        }

        // PadViaLibrary storage
        {
            cf.create_storage("/Library/PadViaLibrary")
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

            let mut header = cf
                .create_stream("/Library/PadViaLibrary/Header")
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
            header.write_u32::<LittleEndian>(1)?;

            let mut params = ParameterCollection::new();
            params.add(
                "PADVIALIBRARY.LIBRARYID",
                "{00000000-0000-0000-0000-000000000000}",
            );
            params.add("PADVIALIBRARY.LIBRARYNAME", "<Local>");
            params.add("PADVIALIBRARY.DISPLAYUNITS", "1");
            let mut block = Vec::new();
            write_parameters(&mut block, &params)?;
            let mut data_buf = Vec::new();
            write_block(&mut data_buf, &block, 0)?;

            let mut data = cf
                .create_stream("/Library/PadViaLibrary/Data")
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
            data.write_all(&data_buf)?;
        }

        // LayerKindMapping storage
        {
            cf.create_storage("/Library/LayerKindMapping")
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

            let mut header = cf
                .create_stream("/Library/LayerKindMapping/Header")
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
            header.write_u32::<LittleEndian>(1)?;

            // LayerKindMapping data: version string "1.0" as wide string + padding
            let mut data_buf = Vec::new();
            // Block size
            data_buf.write_u32::<LittleEndian>(8)?;
            // Wide string "1.0" (UTF-16LE)
            for c in "1.0\0".encode_utf16() {
                data_buf.write_u16::<LittleEndian>(c)?;
            }

            let mut data = cf
                .create_stream("/Library/LayerKindMapping/Data")
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
            data.write_all(&data_buf)?;
        }

        // ComponentParamsTOC storage
        {
            cf.create_storage("/Library/ComponentParamsTOC")
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

            let mut header = cf
                .create_stream("/Library/ComponentParamsTOC/Header")
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
            header.write_u32::<LittleEndian>(self.components.len() as u32)?;

            // Build TOC entries for each component
            let mut toc_data = Vec::new();
            for (i, comp) in self.components.iter().enumerate() {
                let storage_name = format!("PCBComponent_{}", i + 1);
                let pad_count = comp
                    .primitives
                    .iter()
                    .filter(|p| matches!(p, PcbRecord::Pad(_)))
                    .count();

                let mut params = ParameterCollection::new();
                params.add("Name", &storage_name);
                params.add("Pad Count", &pad_count.to_string());
                params.add("Height", "0");
                params.add("Description", &comp.description);

                let mut block = Vec::new();
                write_parameters(&mut block, &params)?;
                write_block(&mut toc_data, &block, 0)?;
            }

            let mut data = cf
                .create_stream("/Library/ComponentParamsTOC/Data")
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
            data.write_all(&toc_data)?;
        }

        Ok(())
    }

    /// Write FileVersionInfo storage.
    fn write_file_version_info<F: Read + Write + Seek>(
        &self,
        cf: &mut CompoundFile<F>,
    ) -> Result<()> {
        cf.create_storage("/FileVersionInfo")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        let mut header = cf
            .create_stream("/FileVersionInfo/Header")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
        header.write_u32::<LittleEndian>(1)?;

        // FileVersionInfo data - minimal version info
        let mut params = ParameterCollection::new();
        params.add("VERSIONNUMBER", "1.0");
        params.add("REVISIONDATE", "2024-01-01");

        let mut block = Vec::new();
        write_parameters(&mut block, &params)?;
        let mut data_buf = Vec::new();
        write_block(&mut data_buf, &block, 0)?;

        let mut data = cf
            .create_stream("/FileVersionInfo/Data")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
        data.write_all(&data_buf)?;

        Ok(())
    }

    /// Write a footprint to its storage using the provided storage name.
    fn write_footprint<F: Read + Write + Seek>(
        &self,
        cf: &mut CompoundFile<F>,
        comp: &PcbComponent,
        storage_name: &str,
    ) -> Result<()> {
        // Create storage for footprint using PCBComponent_N naming
        let storage_path = format!("/{}", storage_name);
        cf.create_storage(&storage_path)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        // Write Header
        {
            let mut header_data = Vec::new();
            header_data.write_u32::<LittleEndian>(comp.primitives.len() as u32)?;

            let header_path = format!("{}/Header", storage_path);
            let stream = cf
                .create_stream(&header_path)
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
            let mut stream = stream;
            stream.write_all(&header_data)?;
        }

        // Write Parameters
        {
            let mut params_data = Vec::new();
            let params = comp.export_to_parameters();
            let mut block = Vec::new();
            write_parameters(&mut block, &params)?;
            write_block(&mut params_data, &block, 0)?;

            let params_path = format!("{}/Parameters", storage_path);
            let stream = cf
                .create_stream(&params_path)
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
            let mut stream = stream;
            stream.write_all(&params_data)?;
        }

        // Write Data
        {
            let mut data = Vec::new();

            // Pattern name (the actual footprint name, not storage name)
            write_string_block(&mut data, &comp.pattern)?;

            // Primitives
            for record in &comp.primitives {
                self.write_primitive(&mut data, record)?;
            }

            let data_path = format!("{}/Data", storage_path);
            let stream = cf
                .create_stream(&data_path)
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
            let mut stream = stream;
            stream.write_all(&data)?;
        }

        // Write WideStrings stream (required, even if empty)
        {
            let mut params = ParameterCollection::new();
            params.add("COUNT", "0");
            let mut block = Vec::new();
            write_parameters(&mut block, &params)?;
            let mut wide_data = Vec::new();
            write_block(&mut wide_data, &block, 0)?;

            let wide_path = format!("{}/WideStrings", storage_path);
            let stream = cf
                .create_stream(&wide_path)
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
            let mut stream = stream;
            stream.write_all(&wide_data)?;
        }

        // Write PrimitiveGuids storage (required)
        {
            let guids_path = format!("{}/PrimitiveGuids", storage_path);
            cf.create_storage(&guids_path)
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

            // Header with primitive count
            let header_path = format!("{}/Header", guids_path);
            let mut header = cf
                .create_stream(&header_path)
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
            header.write_u32::<LittleEndian>(comp.primitives.len() as u32)?;

            // Data with empty GUIDs (zeros) for each primitive
            let data_path = format!("{}/Data", guids_path);
            let mut data = cf
                .create_stream(&data_path)
                .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
            // Each primitive gets a 16-byte zero GUID
            for _ in 0..comp.primitives.len() {
                data.write_all(&[0u8; 16])?;
            }
        }

        Ok(())
    }

    /// Write a single PCB primitive.
    fn write_primitive<W: Write>(&self, writer: &mut W, record: &PcbRecord) -> Result<()> {
        // Write object ID byte
        let object_id = match record {
            PcbRecord::Arc(_) => 1,
            PcbRecord::Pad(_) => 2,
            PcbRecord::Via(_) => 3,
            PcbRecord::Track(_) => 4,
            PcbRecord::Text(_) => 5,
            PcbRecord::Fill(_) => 6,
            PcbRecord::Region(_) => 11,
            PcbRecord::ComponentBody(_) => 12,
            PcbRecord::Polygon(_) => {
                return Err(AltiumError::Parse(
                    "Polygons are not supported in PcbLib footprints".to_string(),
                ));
            }
            PcbRecord::Unknown { object_id, .. } => *object_id as u8,
        };
        writer.write_u8(object_id)?;

        // Write primitive data based on type
        match record {
            PcbRecord::Arc(arc) => {
                let mut data = Vec::new();
                arc.write_to(&mut data)?;
                write_block(writer, &data, 0)?;
            }
            PcbRecord::Pad(pad) => {
                // Pad has a special multi-block format, writes directly
                pad.write_to(writer)?;
            }
            PcbRecord::Via(via) => {
                let mut data = Vec::new();
                via.write_to(&mut data)?;
                write_block(writer, &data, 0)?;
            }
            PcbRecord::Track(track) => {
                let mut data = Vec::new();
                track.write_to(&mut data)?;
                write_block(writer, &data, 0)?;
            }
            PcbRecord::Text(text) => {
                // Text has special format: block + ASCII text block
                let mut data = Vec::new();
                text.write_to(&mut data)?;
                write_block(writer, &data, 0)?;
                write_string_block(writer, &text.text)?;
            }
            PcbRecord::Fill(fill) => {
                let mut data = Vec::new();
                fill.write_to(&mut data)?;
                write_block(writer, &data, 0)?;
            }
            PcbRecord::Region(region) => {
                let mut data = Vec::new();
                region.write_to(&mut data)?;
                write_block(writer, &data, 0)?;
            }
            PcbRecord::ComponentBody(body) => {
                let mut data = Vec::new();
                body.write_to(&mut data)?;
                write_block(writer, &data, 0)?;
            }
            PcbRecord::Polygon(_) => {
                // Already handled above with an error, unreachable
                unreachable!("Polygons should have errored in object_id match")
            }
            PcbRecord::Unknown { raw_data, .. } => {
                write_block(writer, raw_data, 0)?;
            }
        }

        Ok(())
    }

    /// Get section key from reference name.
    fn get_section_key(&self, ref_name: &str) -> String {
        self.section_keys
            .get(ref_name)
            .cloned()
            .unwrap_or_else(|| ref_name.to_string())
    }

    /// Read file header stream.
    fn read_file_header<R: Read + Seek>(&mut self, cf: &mut CompoundFile<R>) -> Result<()> {
        let stream_path = "/FileHeader";
        if cf.entry(stream_path).is_err() {
            return Ok(());
        }

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

        // Read version text block (length prefix then Pascal string)
        let _version_len = cursor.read_i32::<LittleEndian>()?;
        let _version_text = read_pascal_short_string(&mut cursor)?;

        // Try to read additional fields (optional, vary by Altium version).
        // These fields may not exist in older file versions, so EOF is acceptable.
        if (cursor.position() as usize) < data.len() {
            // First optional field: appears to be a version-related float value as string.
            // EOF here indicates older file format without extended header.
            let field1 = read_pascal_short_string(&mut cursor).unwrap_or_default();
            if !field1.is_empty() {
                log::trace!("FileHeader optional field 1: {:?}", field1);
            }

            // Second optional field: appears to be a token/marker string (e.g., "DVLTOKCO").
            // EOF here indicates file format without this marker.
            let field2 = read_pascal_short_string(&mut cursor).unwrap_or_default();
            if !field2.is_empty() {
                log::trace!("FileHeader optional field 2: {:?}", field2);
            }

            // Third optional field: unique ID string for the library.
            // Empty is valid for libraries without assigned unique ID.
            if let Ok(uid) = read_pascal_short_string(&mut cursor) {
                self.unique_id = uid;
            }
        }

        Ok(())
    }

    /// Read section keys stream.
    fn read_section_keys<R: Read + Seek>(&mut self, cf: &mut CompoundFile<R>) -> Result<()> {
        let stream_path = "/SectionKeys";
        if cf.entry(stream_path).is_err() {
            return Ok(());
        }

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
        let key_count = cursor.read_i32::<LittleEndian>()?;

        for _ in 0..key_count {
            let lib_ref = read_pascal_string(&mut cursor)?;
            let section_key = read_string_block(&mut cursor)?;

            if !lib_ref.is_empty() && !section_key.is_empty() {
                self.section_keys.insert(lib_ref, section_key);
            }
        }

        Ok(())
    }

    /// Read the library storage.
    fn read_library<R: Read + Seek>(&mut self, cf: &mut CompoundFile<R>) -> Result<()> {
        let storage_path = "/Library";

        // Read header to get record count
        let header_path = format!("{}/Header", storage_path);
        if cf.entry(&header_path).is_ok() {
            let mut stream = cf.open_stream(&header_path).map_err(|e| {
                AltiumError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    e.to_string(),
                ))
            })?;
            let _record_count = stream.read_u32::<LittleEndian>()?;
        }

        // Read data stream
        let data_path = format!("{}/Data", storage_path);
        let mut stream = cf.open_stream(&data_path).map_err(|e| {
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

        // Read library header parameters
        let _header_params = read_parameters_block(&mut cursor)?;

        // Read footprint count and list
        let footprint_count = cursor.read_u32::<LittleEndian>()?;
        let mut ref_names = Vec::with_capacity(footprint_count as usize);

        for _ in 0..footprint_count {
            let ref_name = read_string_block(&mut cursor)?;
            ref_names.push(ref_name);
        }

        // Read each footprint
        for ref_name in ref_names {
            let section_key = self.get_section_key(&ref_name);
            match self.read_footprint(cf, &section_key) {
                Ok(component) => {
                    self.components.push(component);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to read footprint {:?}: {}", ref_name, e);
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Read a footprint from its storage.
    fn read_footprint<R: Read + Seek>(
        &self,
        cf: &mut CompoundFile<R>,
        section_key: &str,
    ) -> Result<PcbComponent> {
        // The section_key is the actual storage name and may contain forward slashes
        // as part of the name (e.g., "C 0805 / 2012"). These are NOT path separators.
        // However, Altium may convert forward slashes to underscores in CFB storage names.
        let mut storage_path = format!("/{}", section_key);

        // If the storage doesn't exist, try replacing forward slashes with underscores
        if cf.entry(&storage_path).is_err() {
            let section_key_alt = section_key.replace('/', "_");
            let alt_path = format!("/{}", section_key_alt);
            if cf.entry(&alt_path).is_ok() {
                storage_path = alt_path;
            }
        }

        // Read header
        let header_path = format!("{}/Header", storage_path);
        let _record_count = if cf.entry(&header_path).is_ok() {
            let mut stream = cf.open_stream(&header_path).map_err(|e| {
                AltiumError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    e.to_string(),
                ))
            })?;
            stream.read_u32::<LittleEndian>()?
        } else {
            0
        };

        let mut component = PcbComponent::default();

        // Read parameters
        let params_path = format!("{}/Parameters", storage_path);
        if cf.entry(&params_path).is_ok() {
            let mut stream = cf.open_stream(&params_path).map_err(|e| {
                AltiumError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    e.to_string(),
                ))
            })?;
            let mut data = Vec::new();
            stream.read_to_end(&mut data)?;

            if !data.is_empty() {
                let mut cursor = Cursor::new(&data);
                let params = read_parameters_block(&mut cursor)?;
                component.import_from_parameters(&params);
            }
        }

        // Read wide strings (for Unicode text)
        let wide_strings = self.read_wide_strings(cf, &storage_path)?;

        // Read data stream
        let data_path = format!("{}/Data", storage_path);
        let mut stream = cf.open_stream(&data_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Footprint data not found: {} - {}", data_path, e),
            ))
        })?;

        let mut data = Vec::new();
        stream.read_to_end(&mut data)?;

        if data.is_empty() {
            return Err(AltiumError::Parse("Empty footprint data".to_string()));
        }

        let mut cursor = Cursor::new(&data);

        // First block is the pattern name (should match component.pattern)
        let pattern = read_string_block(&mut cursor)?;
        if component.pattern.is_empty() {
            component.pattern = pattern;
        }

        // Read primitives
        while (cursor.position() as usize) < data.len() {
            match self.read_primitive(&mut cursor, &wide_strings) {
                Ok(record) => component.primitives.push(record),
                Err(_) => break,
            }
        }

        Ok(component)
    }

    /// Read wide strings storage for Unicode text support.
    fn read_wide_strings<R: Read + Seek>(
        &self,
        cf: &mut CompoundFile<R>,
        storage_path: &str,
    ) -> Result<Vec<String>> {
        let wide_path = format!("{}/WideStrings", storage_path);
        if cf.entry(&wide_path).is_err() {
            return Ok(Vec::new());
        }

        let mut stream = cf.open_stream(&wide_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                e.to_string(),
            ))
        })?;

        let mut data = Vec::new();
        stream.read_to_end(&mut data)?;

        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut cursor = Cursor::new(&data);
        let params = read_parameters_block(&mut cursor)?;

        let mut strings = Vec::new();
        let count = params.get("COUNT").map(|v| v.as_int_or(0)).unwrap_or(0) as usize;

        for i in 0..count {
            let key = format!("WIDESTRING{}", i);
            if let Some(val) = params.get(&key) {
                strings.push(val.as_str().to_string());
            } else {
                strings.push(String::new());
            }
        }

        Ok(strings)
    }

    /// Read a single primitive from the stream.
    fn read_primitive(
        &self,
        cursor: &mut Cursor<&Vec<u8>>,
        wide_strings: &[String],
    ) -> Result<PcbRecord> {
        let object_id = PcbObjectId::from_byte(cursor.read_u8()?);

        match object_id {
            PcbObjectId::Arc => {
                let block = read_block(cursor)?;
                let mut block_cursor = Cursor::new(&block);
                let arc = <PcbArc as FromBinary>::read_from(&mut block_cursor)?;
                Ok(PcbRecord::Arc(arc))
            }
            PcbObjectId::Pad => {
                let pad = PcbPad::read_from(cursor)?;
                Ok(PcbRecord::Pad(Box::new(pad)))
            }
            PcbObjectId::Via => {
                let block = read_block(cursor)?;
                let mut block_cursor = Cursor::new(&block);
                let via = <PcbVia as FromBinary>::read_from(&mut block_cursor)?;
                Ok(PcbRecord::Via(via))
            }
            PcbObjectId::Track => {
                let block = read_block(cursor)?;
                let mut block_cursor = Cursor::new(&block);
                let track = <PcbTrack as FromBinary>::read_from(&mut block_cursor)?;
                Ok(PcbRecord::Track(track))
            }
            PcbObjectId::Text => {
                let block = read_block(cursor)?;
                let mut block_cursor = Cursor::new(&block);
                let mut text = <PcbText as FromBinary>::read_from(&mut block_cursor)?;

                // Read ASCII text from separate block
                let ascii_text = read_string_block(cursor)?;

                // Use wide string if available
                if text.wide_strings_index >= 0
                    && (text.wide_strings_index as usize) < wide_strings.len()
                {
                    text.text = wide_strings[text.wide_strings_index as usize].clone();
                } else {
                    text.text = ascii_text;
                }

                Ok(PcbRecord::Text(text))
            }
            PcbObjectId::Fill => {
                let block = read_block(cursor)?;
                let mut block_cursor = Cursor::new(&block);
                let fill = <PcbFill as FromBinary>::read_from(&mut block_cursor)?;
                Ok(PcbRecord::Fill(fill))
            }
            PcbObjectId::Region => {
                let block = read_block(cursor)?;
                let mut block_cursor = Cursor::new(&block);
                let region = <PcbRegion as FromBinary>::read_from(&mut block_cursor)?;
                Ok(PcbRecord::Region(region))
            }
            PcbObjectId::ComponentBody => {
                let block = read_block(cursor)?;
                let mut block_cursor = Cursor::new(&block);
                let body = <PcbComponentBody as FromBinary>::read_from(&mut block_cursor)?;
                Ok(PcbRecord::ComponentBody(Box::new(body)))
            }
            _ => {
                // Unknown - skip the block
                let block = read_block(cursor)?;
                Ok(PcbRecord::Unknown {
                    object_id,
                    raw_data: block,
                })
            }
        }
    }

    /// Get the number of components.
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Iterate over components.
    pub fn iter(&self) -> impl Iterator<Item = &PcbComponent> {
        self.components.iter()
    }
}

// DumpTree implementation
use crate::dump::{DumpTree, TreeBuilder};

impl DumpTree for PcbLib {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.root(&format!("PcbLib ({} footprints)", self.components.len()));

        for (i, comp) in self.components.iter().enumerate() {
            tree.push(i < self.components.len() - 1);
            comp.dump(tree);
            tree.pop();
        }
    }
}

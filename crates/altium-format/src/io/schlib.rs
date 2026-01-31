//! SchLib reader/writer for Altium schematic library files.

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use cfb::CompoundFile;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;

use crate::error::{AltiumError, Result};
use crate::format::SIZE_FLAG_MASK;
use crate::io::reader::{
    decode_windows_1252, read_parameters_block, read_pascal_short_string, read_string_block,
};
use crate::io::writer::{
    write_block, write_parameters, write_pascal_short_string,
};
use crate::records::sch::{
    PinConglomerateFlags, PinElectricalType, PinSymbol, SchComponent, SchGraphicalBase, SchPin,
    SchRecord, coord_to_dxp_frac, dxp_frac_to_coord,
};
use crate::types::ParameterCollection;

/// A schematic library containing components.
#[derive(Debug, Default)]
pub struct SchLib {
    /// Section keys mapping LIBREF to storage path.
    section_keys: HashMap<String, String>,
    /// Components in the library.
    pub components: Vec<SchLibComponent>,
    /// Raw FileHeader parameters (fonts, grid, sheet settings).
    /// Preserved for round-trip fidelity. Excludes HEADER, WEIGHT,
    /// COMPCOUNT, LIBREF*, PARTCOUNT* (which are regenerated on save).
    pub header_params: ParameterCollection,
}

/// A component in the schematic library.
#[derive(Debug)]
pub struct SchLibComponent {
    /// Component data record.
    pub component: SchComponent,
    /// All primitives belonging to this component.
    pub primitives: Vec<SchRecord>,
}

impl SchLib {
    /// Open and read a SchLib file.
    pub fn open<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut schlib = SchLib::default();
        let mut cf = CompoundFile::open(reader).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?;

        // Read section keys
        schlib.read_section_keys(&mut cf)?;

        // Read file header to get component list
        let ref_names = schlib.read_file_header(&mut cf)?;

        // Read each component
        for ref_name in ref_names.iter() {
            let section_key = schlib.get_section_key(ref_name);
            if let Ok(component) = schlib.read_component(&mut cf, &section_key) {
                schlib.components.push(component);
            }
        }

        Ok(schlib)
    }

    /// Open and read a SchLib file from a path.
    pub fn open_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        Self::open(file)
    }

    /// Save the SchLib to a file.
    pub fn save<W: Read + Write + Seek>(&self, writer: W) -> Result<()> {
        let mut cf = CompoundFile::create_with_version(cfb::Version::V3, writer)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        // Write Storage stream (icon storage header)
        self.write_storage(&mut cf)?;

        // Write FileHeader
        self.write_file_header(&mut cf)?;

        // Write SectionKeys if needed
        self.write_section_keys(&mut cf)?;

        // Write each component
        for comp in &self.components {
            self.write_component(&mut cf, comp)?;
        }

        // Write Redirection streams for component aliases
        self.write_alias_redirections(&mut cf)?;

        cf.flush()
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    /// Save the SchLib to a file path.
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

    /// Write the FileHeader stream in Altium's indexed parameter format.
    ///
    /// The original format stores all component data as indexed parameters within
    /// a single parameter block, rather than using separate binary component count
    /// and string blocks.
    fn write_file_header<F: Read + Write + Seek>(&self, cf: &mut CompoundFile<F>) -> Result<()> {
        let mut data = Vec::new();

        // Build header parameters
        let mut header_params = ParameterCollection::new();
        header_params.add(
            "HEADER",
            "Protel for Windows - Schematic Library Editor Binary File Version 5.0",
        );

        // Calculate WEIGHT as total primitive count across all components
        let total_primitives: usize = self.components.iter().map(|c| c.primitives.len()).sum();
        header_params.add_int("WEIGHT", total_primitives as i32);

        // Merge in stored metadata parameters (fonts, grid, sheet settings)
        for (key, value) in self.header_params.iter() {
            let upper = key.to_uppercase();
            // Skip keys we regenerate
            if upper == "WEIGHT" || upper == "HEADER" {
                continue;
            }
            header_params.add(key, value.as_str());
        }

        // Count total components including aliases
        let mut total_comp_count = self.components.len() as i32;
        for comp in &self.components {
            if !comp.component.alias_list.is_empty() {
                // Count aliases (pipe-separated)
                let alias_count = comp
                    .component
                    .alias_list
                    .split('|')
                    .filter(|s| !s.is_empty())
                    .count();
                total_comp_count += alias_count as i32;
            }
        }
        header_params.add_int("COMPCOUNT", total_comp_count);

        // Write indexed LIBREF/PARTCOUNT/COMPDESCR entries for each component
        for (i, comp) in self.components.iter().enumerate() {
            header_params.add(
                &format!("LIBREF{}", i),
                &comp.component.lib_reference,
            );
            header_params.add_int(
                &format!("PARTCOUNT{}", i),
                comp.component.part_count + 1,
            );
            if !comp.component.component_description.is_empty() {
                header_params.add(
                    &format!("COMPDESCR{}", i),
                    &comp.component.component_description,
                );
            }
            // Write alias entries
            if !comp.component.alias_list.is_empty() {
                let aliases: Vec<&str> = comp
                    .component
                    .alias_list
                    .split('|')
                    .filter(|s| !s.is_empty())
                    .collect();
                if !aliases.is_empty() {
                    header_params.add_int(
                        &format!("ALIASCOUNT{}", i),
                        aliases.len() as i32,
                    );
                    for (j, alias) in aliases.iter().enumerate() {
                        header_params.add(
                            &format!("COMP{}ALIAS{}", i, j),
                            alias,
                        );
                    }
                }
            }
        }

        let mut header_block = Vec::new();
        write_parameters(&mut header_block, &header_params)?;
        write_block(&mut data, &header_block, 0)?;

        let stream = cf
            .create_stream("/FileHeader")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        let mut stream = stream;
        stream.write_all(&data)?;

        Ok(())
    }

    /// Write the SectionKeys stream (for components with names that need aliasing).
    fn write_section_keys<F: Read + Write + Seek>(&self, cf: &mut CompoundFile<F>) -> Result<()> {
        // Only write section keys for components that need them
        let components_needing_keys: Vec<_> = self
            .components
            .iter()
            .filter(|c| Self::needs_section_key(&c.component.lib_reference))
            .collect();

        if components_needing_keys.is_empty() {
            return Ok(());
        }

        let mut data = Vec::new();

        let mut params = ParameterCollection::new();
        params.add_int("KEYCOUNT", components_needing_keys.len() as i32);

        for (i, comp) in components_needing_keys.iter().enumerate() {
            let lib_ref = &comp.component.lib_reference;
            let section_key = Self::get_section_key_for(lib_ref);
            params.add(&format!("LIBREF{}", i), lib_ref);
            params.add(&format!("SECTIONKEY{}", i), &section_key);
        }

        let mut block = Vec::new();
        write_parameters(&mut block, &params)?;
        write_block(&mut data, &block, 0)?;

        let stream = cf
            .create_stream("/SectionKeys")
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        let mut stream = stream;
        stream.write_all(&data)?;

        Ok(())
    }

    /// Write Redirection streams for component aliases.
    ///
    /// When a component has an ALIASLIST, each alias gets its own OLE storage
    /// containing a `Redirection` stream that points to the real component name.
    /// The format is a length-prefixed record: `|SECTIONNAME=<real_name>\0`.
    fn write_alias_redirections<F: Read + Write + Seek>(
        &self,
        cf: &mut CompoundFile<F>,
    ) -> Result<()> {
        for comp in &self.components {
            if comp.component.alias_list.is_empty() {
                continue;
            }

            let aliases: Vec<&str> = comp
                .component
                .alias_list
                .split('|')
                .filter(|s| !s.is_empty())
                .collect();

            for alias in aliases {
                let storage_path = format!("/{}", alias);
                cf.create_storage(&storage_path)
                    .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

                // Build the redirection record: |SECTIONNAME=<real_name>\0
                let record_content =
                    format!("|SECTIONNAME={}\0", comp.component.lib_reference);
                let record_bytes = record_content.as_bytes();

                let mut data = Vec::new();
                data.write_i32::<LittleEndian>(record_bytes.len() as i32)?;
                data.write_all(record_bytes)?;

                let redirect_path = format!("{}/Redirection", storage_path);
                let mut stream = cf
                    .create_stream(&redirect_path)
                    .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;
                stream.write_all(&data)?;
            }
        }

        Ok(())
    }

    /// Check if a component name needs a section key alias.
    fn needs_section_key(name: &str) -> bool {
        name.len() > 31 || name.contains('/')
    }

    /// Generate a section key for a component name.
    fn get_section_key_for(name: &str) -> String {
        let mut key = name.replace('/', "_");
        if key.len() > 31 {
            key.truncate(31);
        }
        key
    }

    /// Write a component to its storage.
    fn write_component<F: Read + Write + Seek>(
        &self,
        cf: &mut CompoundFile<F>,
        comp: &SchLibComponent,
    ) -> Result<()> {
        let section_key = if Self::needs_section_key(&comp.component.lib_reference) {
            Self::get_section_key_for(&comp.component.lib_reference)
        } else {
            comp.component.lib_reference.clone()
        };

        // Create storage for component
        let storage_path = format!("/{}", section_key);
        cf.create_storage(&storage_path)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        // Write Data stream
        let mut data = Vec::new();
        for record in &comp.primitives {
            self.write_record(&mut data, record)?;
        }

        let data_path = format!("{}/Data", storage_path);
        let stream = cf
            .create_stream(&data_path)
            .map_err(|e| AltiumError::Io(std::io::Error::other(e.to_string())))?;

        let mut stream = stream;
        stream.write_all(&data)?;

        Ok(())
    }

    /// Write a single record to the stream.
    fn write_record<W: Write>(&self, writer: &mut W, record: &SchRecord) -> Result<()> {
        match record {
            SchRecord::Pin(pin) => {
                // Pins are written in binary format with flag 0x01
                self.write_binary_pin(writer, pin)
            }
            _ => {
                // Other records are ASCII parameter format
                let params = record.export_to_params();
                let mut block = Vec::new();
                write_parameters(&mut block, &params)?;
                write_block(writer, &block, 0)
            }
        }
    }

    /// Write a binary pin record.
    fn write_binary_pin<W: Write>(&self, writer: &mut W, pin: &SchPin) -> Result<()> {
        let mut data = Vec::new();

        // Record type (2 = pin)
        data.write_i32::<LittleEndian>(2)?;

        // Unknown byte
        data.write_u8(0)?;

        // Owner part ID
        data.write_i16::<LittleEndian>(pin.graphical.base.owner_part_id.unwrap_or(1) as i16)?;

        // Owner part display mode
        data.write_u8(pin.graphical.base.owner_part_display_mode.unwrap_or(0) as u8)?;

        // Symbol edges
        data.write_u8(pin.symbol_inner_edge.to_int() as u8)?;
        data.write_u8(pin.symbol_outer_edge.to_int() as u8)?;
        data.write_u8(pin.symbol_inside.to_int() as u8)?;
        data.write_u8(pin.symbol_outside.to_int() as u8)?;

        // Description
        write_pascal_short_string(&mut data, &pin.description)?;

        // Unknown byte
        data.write_u8(0)?;

        // Electrical type
        data.write_u8(pin.electrical.to_int() as u8)?;

        // Pin conglomerate flags
        data.write_u8(pin.pin_conglomerate.bits())?;

        // Pin length, location X, location Y (as integer mils)
        let (length, _) = coord_to_dxp_frac(pin.pin_length);
        let (loc_x, _) = coord_to_dxp_frac(pin.graphical.location_x);
        let (loc_y, _) = coord_to_dxp_frac(pin.graphical.location_y);
        data.write_i16::<LittleEndian>(length as i16)?;
        data.write_i16::<LittleEndian>(loc_x as i16)?;
        data.write_i16::<LittleEndian>(loc_y as i16)?;

        // Color
        data.write_i32::<LittleEndian>(pin.graphical.color)?;

        // Name, Designator, SwapIdGroup
        write_pascal_short_string(&mut data, &pin.name)?;
        write_pascal_short_string(&mut data, &pin.designator)?;
        write_pascal_short_string(&mut data, &pin.swap_id_group)?;

        // Part and sequence (combined format: "part|&|sequence")
        let part_and_sequence = if pin.swap_id_part == 0 && pin.swap_id_sequence.is_empty() {
            String::new()
        } else if pin.swap_id_part != 0 {
            format!("{}|&|{}", pin.swap_id_part, pin.swap_id_sequence)
        } else {
            format!("|&|{}", pin.swap_id_sequence)
        };
        write_pascal_short_string(&mut data, &part_and_sequence)?;

        // Default value
        write_pascal_short_string(&mut data, &pin.default_value)?;

        // Write block with flag 0x01 (binary record)
        write_block(writer, &data, 0x01)
    }

    /// Get section key from reference name.
    ///
    /// If a SectionKeys mapping exists, use it. Otherwise, if the name
    /// contains '/' (which is invalid in OLE/CFB storage paths), fall back
    /// to the same escaping used when writing: replace '/' with '_' and
    /// truncate to 31 characters.
    fn get_section_key(&self, ref_name: &str) -> String {
        self.section_keys
            .get(ref_name)
            .cloned()
            .unwrap_or_else(|| {
                if Self::needs_section_key(ref_name) {
                    Self::get_section_key_for(ref_name)
                } else {
                    ref_name.to_string()
                }
            })
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

        let mut cursor = Cursor::new(data);
        let params = read_parameters_block(&mut cursor)?;

        let key_count = match params.get("KEYCOUNT") {
            Some(v) => v.as_int_or(0),
            None => 0,
        };

        for i in 0..key_count {
            let lib_ref = match params.get(&format!("LIBREF{}", i)) {
                Some(v) => v.as_str().to_string(),
                None => String::new(),
            };
            let section_key = match params.get(&format!("SECTIONKEY{}", i)) {
                Some(v) => v.as_str().to_string(),
                None => String::new(),
            };

            if !lib_ref.is_empty() && !section_key.is_empty() {
                self.section_keys.insert(lib_ref, section_key);
            }
        }

        Ok(())
    }

    /// Keys that are regenerated on save and should not be stored in header_params.
    const REGENERATED_HEADER_KEYS: &[&str] = &["HEADER", "WEIGHT", "COMPCOUNT"];

    /// Indexed key prefixes for component-index data (regenerated on save).
    /// These are followed by a numeric suffix (e.g., LIBREF0, PARTCOUNT1, COMPDESCR12).
    /// Font keys (FONTNAME*, SIZE*) are metadata and are preserved.
    const COMPONENT_INDEX_PREFIXES: &[&str] = &[
        "LIBREF",
        "PARTCOUNT",
        "COMPDESCR",
        "ALIASCOUNT",
        "PINNUMBER",
    ];

    /// Check if a parameter key is a component-index key that should be skipped.
    fn is_component_index_key(key: &str) -> bool {
        for prefix in Self::COMPONENT_INDEX_PREFIXES {
            if key.starts_with(prefix) && key[prefix.len()..].parse::<i32>().is_ok() {
                return true;
            }
        }
        // Handle COMP<N>ALIAS<M> pattern (e.g., COMP9ALIAS0)
        if key.starts_with("COMP") {
            let rest = &key[4..];
            if let Some(alias_pos) = rest.find("ALIAS") {
                let comp_num = &rest[..alias_pos];
                let alias_num = &rest[alias_pos + 5..];
                if comp_num.parse::<i32>().is_ok() && alias_num.parse::<i32>().is_ok() {
                    return true;
                }
            }
        }
        false
    }

    /// Read file header to get component list.
    fn read_file_header<R: Read + Seek>(&mut self, cf: &mut CompoundFile<R>) -> Result<Vec<String>> {
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
            return Ok(Vec::new());
        }

        let mut cursor = Cursor::new(&data);
        let params = read_parameters_block(&mut cursor)?;

        // Store metadata parameters (everything except regenerated keys and component index)
        for (key, value) in params.iter() {
            let upper = key.to_uppercase();
            if Self::REGENERATED_HEADER_KEYS.contains(&upper.as_str()) {
                continue;
            }
            if Self::is_component_index_key(&upper) {
                continue;
            }
            self.header_params.add(key, value.as_str());
        }

        let mut ref_names = Vec::new();

        // Check if we're at end of stream (use params-based component list)
        if cursor.position() as usize >= data.len() {
            let comp_count = match params.get("COMPCOUNT") {
                Some(v) => v.as_int_or(0),
                None => 0,
            };
            for i in 0..comp_count {
                if let Some(name) = params.get(&format!("LIBREF{}", i)) {
                    ref_names.push(name.as_str().to_string());
                }
            }
        } else {
            // Read binary component list
            use byteorder::{LittleEndian, ReadBytesExt};
            let count = cursor.read_u32::<LittleEndian>()?;
            for _ in 0..count {
                let name = read_string_block(&mut cursor)?;
                ref_names.push(name);
            }
        }

        Ok(ref_names)
    }

    /// Read a component from its storage.
    fn read_component<R: Read + Seek>(
        &self,
        cf: &mut CompoundFile<R>,
        section_key: &str,
    ) -> Result<SchLibComponent> {
        let stream_path = format!("/{}/Data", section_key);
        let mut stream = cf.open_stream(&stream_path).map_err(|e| {
            AltiumError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Component stream not found: {} - {}", stream_path, e),
            ))
        })?;

        let mut data = Vec::new();
        stream.read_to_end(&mut data)?;

        if data.is_empty() {
            return Err(AltiumError::Parse("Empty component data".to_string()));
        }

        let mut cursor = Cursor::new(&data);
        let mut primitives = Vec::new();

        // Read all primitives
        while (cursor.position() as usize) < data.len() {
            match self.read_record(&mut cursor) {
                Ok(record) => primitives.push(record),
                Err(_) => break, // Stop on error
            }
        }

        // First primitive should be the component
        let component = match primitives.first() {
            Some(SchRecord::Component(c)) => c.clone(),
            _ => {
                return Err(AltiumError::Parse(
                    "First record is not a component".to_string(),
                ));
            }
        };

        Ok(SchLibComponent {
            component,
            primitives,
        })
    }

    /// Read a single record from the stream.
    fn read_record<R: Read>(&self, reader: &mut R) -> Result<SchRecord> {
        use byteorder::{LittleEndian, ReadBytesExt};

        let size = reader.read_i32::<LittleEndian>()?;
        let is_binary = (size as u32 & !SIZE_FLAG_MASK) != 0;
        let clean_size = (size & SIZE_FLAG_MASK as i32) as usize;

        if clean_size == 0 {
            return Err(AltiumError::Parse("Empty record".to_string()));
        }

        let mut buffer = vec![0u8; clean_size];
        reader.read_exact(&mut buffer)?;

        if is_binary {
            // Binary pin record
            let mut cursor = Cursor::new(buffer);
            self.read_binary_pin(&mut cursor)
        } else {
            // ASCII parameter record
            let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
            let param_str = decode_windows_1252(&buffer[..end]);
            let params = ParameterCollection::from_string(&param_str);
            SchRecord::from_params(&params)
        }
    }

    /// Read a binary pin record.
    ///
    /// Pin records in SchLib files are stored in a compact binary format rather than
    /// the typical ASCII parameter format used by other records.
    fn read_binary_pin<R: Read>(&self, reader: &mut R) -> Result<SchRecord> {
        let record_type = reader.read_i32::<LittleEndian>()?;
        if record_type != 2 {
            return Err(AltiumError::Parse(format!(
                "Expected pin record type 2, got {}",
                record_type
            )));
        }

        let _unknown1 = reader.read_u8()?; // Unknown byte
        let owner_part_id = reader.read_i16::<LittleEndian>()?;
        let owner_part_display_mode = reader.read_u8()?;
        let symbol_inner_edge = PinSymbol::from_int(reader.read_u8()? as i32);
        let symbol_outer_edge = PinSymbol::from_int(reader.read_u8()? as i32);
        let symbol_inside = PinSymbol::from_int(reader.read_u8()? as i32);
        let symbol_outside = PinSymbol::from_int(reader.read_u8()? as i32);
        let description = read_pascal_short_string(reader)?;
        let _unknown2 = reader.read_u8()?; // Unknown byte
        let electrical = PinElectricalType::from_int(reader.read_u8()? as i32);
        let pin_conglomerate = PinConglomerateFlags::from_int(reader.read_u8()? as i32);
        let pin_length_int = reader.read_i16::<LittleEndian>()? as i32;
        let location_x_int = reader.read_i16::<LittleEndian>()? as i32;
        let location_y_int = reader.read_i16::<LittleEndian>()? as i32;
        let color = reader.read_i32::<LittleEndian>()?;
        let name = read_pascal_short_string(reader)?;
        let designator = read_pascal_short_string(reader)?;
        let swap_id_group = read_pascal_short_string(reader)?;
        let part_and_sequence = read_pascal_short_string(reader)?;
        let default_value = read_pascal_short_string(reader)?;

        // Parse the swap ID part and sequence from the combined string
        let (swap_id_part, swap_id_sequence) = if !part_and_sequence.is_empty() {
            let parts: Vec<&str> = part_and_sequence.split('|').collect();
            if parts.len() == 3 {
                (parts[0].parse().unwrap_or(0), parts[2].to_string())
            } else {
                (0, String::new())
            }
        } else {
            (0, String::new())
        };

        // Convert integer coordinates to raw coord values (multiply by 10000)
        // In binary format, coordinates are stored as integer mils without fractional part
        let pin_length = dxp_frac_to_coord(pin_length_int, 0);
        let location_x = dxp_frac_to_coord(location_x_int, 0);
        let location_y = dxp_frac_to_coord(location_y_int, 0);

        let mut graphical = SchGraphicalBase::default();
        graphical.base.owner_part_id = Some(owner_part_id as i32);
        graphical.base.owner_part_display_mode = Some(owner_part_display_mode as i32);
        graphical.location_x = location_x;
        graphical.location_y = location_y;
        graphical.color = color;

        let pin = SchPin {
            graphical,
            symbol_inner_edge,
            symbol_outer_edge,
            symbol_inside,
            symbol_outside,
            description,
            electrical,
            pin_conglomerate,
            pin_length,
            name,
            designator,
            swap_id_group,
            swap_id_part,
            swap_id_sequence,
            default_value,
            ..Default::default()
        };

        Ok(SchRecord::Pin(pin))
    }

    /// Get the number of components.
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Iterate over components.
    pub fn iter(&self) -> impl Iterator<Item = &SchLibComponent> {
        self.components.iter()
    }
}

impl SchLibComponent {
    /// Get the component name (LIBREFERENCE).
    pub fn name(&self) -> &str {
        &self.component.lib_reference
    }

    /// Get the component description.
    pub fn description(&self) -> &str {
        &self.component.component_description
    }

    /// Get the number of pins.
    pub fn pin_count(&self) -> usize {
        self.primitives
            .iter()
            .filter(|r| matches!(r, SchRecord::Pin(_)))
            .count()
    }

    /// Get total primitive count.
    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }
}

// DumpTree implementations
use crate::dump::{DumpTree, TreeBuilder};

impl DumpTree for SchLib {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.root(&format!("SchLib ({} components)", self.components.len()));

        for (i, comp) in self.components.iter().enumerate() {
            tree.push(i < self.components.len() - 1);
            comp.dump(tree);
            tree.pop();
        }
    }
}

impl DumpTree for SchLibComponent {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.begin_node(&format!("Symbol: {}", self.component.lib_reference));
        tree.push(true);

        // Metadata section
        tree.push(self.primitives.len() > 1);
        let mut meta_props = vec![];
        if !self.component.component_description.is_empty() {
            meta_props.push(("description", self.component.component_description.clone()));
        }
        meta_props.push(("parts", format!("{}", self.component.part_count)));
        meta_props.push(("pins", format!("{}", self.pin_count())));
        meta_props.push(("primitives", format!("{}", self.primitive_count())));
        tree.add_leaf("Info", &meta_props);
        tree.pop();

        // Primitives section (skip first which is the component itself)
        let child_primitives: Vec<_> = self.primitives.iter().skip(1).collect();
        if !child_primitives.is_empty() {
            tree.push(false);
            tree.begin_node(&format!("Primitives ({})", child_primitives.len()));
            for (i, prim) in child_primitives.iter().enumerate() {
                tree.push(i < child_primitives.len() - 1);
                prim.dump(tree);
                tree.pop();
            }
            tree.pop();
        }

        tree.pop();
    }
}

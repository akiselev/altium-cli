//! PcbDoc reader/writer for Altium PCB document files.
//!
//! **DEPRECATED**: V1 IO is replaced by v2 with correct coordinate scale.
//! V1 uses 1 unit/mil (incorrect); v2 uses 10K units/mil.
//!
//! Supports reading and writing of PCB documents including board data,
//! components, primitives, nets, and design rules.

#![allow(unused_imports)]
#![allow(dead_code)]

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
///
/// **DEPRECATED**: Use `v2::pcb::io::pcbdoc::PcbDocV2` instead.
/// V1 has coordinate scale bugs (uses 1 unit/mil instead of 10K units/mil).
#[deprecated(note = "Use v2::pcb::io::pcbdoc::PcbDocV2")]
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
///
/// **DEPRECATED**: Use v2::pcb types instead.
#[deprecated(note = "Use v2::pcb types")]
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

#[allow(deprecated)]
impl PcbDoc {
    /// Open and read a PcbDoc file.
    ///
    /// **DEPRECATED**: Use `v2::pcb::io::pcbdoc::PcbDocV2::open()` instead.
    #[deprecated(note = "Use v2::pcb::io::pcbdoc::PcbDocV2::open()")]
    pub fn open<R: Read + Seek>(_reader: R) -> Result<Self> {
        unimplemented!("Use v2::pcb::io::pcbdoc::PcbDocV2::open() - v1 API has been deprecated")
    }

    /// Open and read a PcbDoc file from a path.
    ///
    /// **DEPRECATED**: Use `v2::pcb::io::pcbdoc::PcbDocV2::open_file()` instead.
    #[deprecated(note = "Use v2::pcb::io::pcbdoc::PcbDocV2::open_file()")]
    pub fn open_file<P: AsRef<Path>>(_path: P) -> Result<Self> {
        unimplemented!(
            "Use v2::pcb::io::pcbdoc::PcbDocV2::open_file() - v1 API has been deprecated"
        )
    }

    // Internal read methods stubbed
    fn read_board<R: Read + Seek>(&mut self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    fn read_components<R: Read + Seek>(&mut self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    fn read_component_record<R: Read>(&self, _reader: &mut R) -> Result<PcbDocComponent> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    fn read_primitives<R: Read + Seek>(&mut self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    fn read_primitive_storage<R, F>(
        &mut self,
        _cf: &mut CompoundFile<R>,
        _path: &str,
        _reader_fn: F,
    ) -> Result<()>
    where
        R: Read + Seek,
        F: Fn(&mut Cursor<&Vec<u8>>, usize) -> Result<PcbRecord>,
    {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    fn read_nets<R: Read + Seek>(&mut self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    fn read_rules<R: Read + Seek>(&mut self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    fn read_classes<R: Read + Seek>(&mut self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    fn read_options<R: Read + Seek>(&mut self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    fn read_options_stream<R: Read + Seek>(
        _cf: &mut CompoundFile<R>,
        _path: &str,
    ) -> Result<ParameterCollection> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    /// Save the PcbDoc to a file path.
    ///
    /// **DEPRECATED**: Use v2 API instead.
    #[deprecated(note = "Use v2::pcb::io::pcbdoc::PcbDocV2")]
    pub fn save_to_file<P: AsRef<Path>>(&self, _path: P) -> Result<()> {
        unimplemented!(
            "Use v2::pcb::io::pcbdoc::PcbDocV2::write_to_file() - v1 API has been deprecated"
        )
    }

    fn write_rules<R: Read + Write + Seek>(&self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
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
    #[deprecated(note = "Use v2::pcb::io::pcbdoc::PcbDocV2")]
    pub fn save_board_to_file<P: AsRef<Path>>(&self, _path: P) -> Result<()> {
        unimplemented!("Use v2::pcb::io::pcbdoc::PcbDocV2 - v1 API has been deprecated")
    }

    fn write_board<R: Read + Write + Seek>(&self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    /// Save regions (keepouts/cutouts) to a file path.
    #[deprecated(note = "Use v2::pcb::io::pcbdoc::PcbDocV2")]
    pub fn save_regions_to_file<P: AsRef<Path>>(&self, _path: P) -> Result<()> {
        unimplemented!("Use v2::pcb::io::pcbdoc::PcbDocV2 - v1 API has been deprecated")
    }

    /// Save polygons (copper pours) to a file path.
    #[deprecated(note = "Use v2::pcb::io::pcbdoc::PcbDocV2")]
    pub fn save_polygons_to_file<P: AsRef<Path>>(&self, _path: P) -> Result<()> {
        unimplemented!("Use v2::pcb::io::pcbdoc::PcbDocV2 - v1 API has been deprecated")
    }

    // Simple accessor methods - kept functional but return empty/default values

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

    fn write_components<R: Read + Write + Seek>(&self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    /// Save with component changes.
    #[deprecated(note = "Use v2::pcb::io::pcbdoc::PcbDocV2")]
    pub fn save_with_components<P: AsRef<Path>>(&self, _path: P) -> Result<()> {
        unimplemented!("Use v2::pcb::io::pcbdoc::PcbDocV2 - v1 API has been deprecated")
    }

    /// Save all primitives to a file path.
    #[deprecated(note = "Use v2::pcb::io::pcbdoc::PcbDocV2")]
    pub fn save_all_to_file<P: AsRef<Path>>(&self, _path: P) -> Result<()> {
        unimplemented!("Use v2::pcb::io::pcbdoc::PcbDocV2 - v1 API has been deprecated")
    }

    fn write_tracks<R: Read + Write + Seek>(&self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    fn write_vias<R: Read + Write + Seek>(&self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    fn write_arcs<R: Read + Write + Seek>(&self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    fn write_fills<R: Read + Write + Seek>(&self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    fn write_pads<R: Read + Write + Seek>(&self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    fn write_texts<R: Read + Write + Seek>(&self, _cf: &mut CompoundFile<R>) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    fn write_regions_internal<R: Read + Write + Seek>(
        &self,
        _cf: &mut CompoundFile<R>,
    ) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
    }

    fn write_polygons_internal<R: Read + Write + Seek>(
        &self,
        _cf: &mut CompoundFile<R>,
    ) -> Result<()> {
        unimplemented!("Replaced by v2::pcb::io::pcbdoc::PcbDocV2")
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

#[allow(deprecated)]
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

#[allow(deprecated)]
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

#[allow(deprecated)]
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

//! Base primitive types and enums for PCB records.

use bitflags::bitflags;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

use crate::error::Result;
use crate::traits::{FromBinary, ToBinary};
use crate::types::{Coord, CoordPoint, CoordRect, Layer};
use altium_format_derive::AltiumRecord;

/// PCB primitive object IDs.
///
/// Based on DXP API TObjectId enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PcbObjectId {
    #[default]
    None = 0,
    /// Arc primitive.
    Arc = 1,
    /// Pad primitive.
    Pad = 2,
    /// Via primitive.
    Via = 3,
    /// Track primitive.
    Track = 4,
    /// Text primitive.
    Text = 5,
    /// Fill (solid rectangle) primitive.
    Fill = 6,
    /// Ratsnest connection.
    Connection = 7,
    /// Net definition.
    Net = 8,
    /// Component (footprint instance).
    Component = 9,
    /// Polygon pour.
    Polygon = 10,
    /// Region (copper/keepout area).
    Region = 11,
    /// Component 3D body.
    ComponentBody = 12,
    /// Dimension annotation.
    Dimension = 13,
    /// Coordinate annotation.
    Coordinate = 14,
    /// Net/component class.
    Class = 15,
    /// Design rule.
    Rule = 16,
    /// From-To definition.
    FromTo = 17,
    /// Differential pair definition.
    DifferentialPair = 18,
    /// DRC violation marker.
    Violation = 19,
    /// Embedded document.
    Embedded = 20,
    /// Embedded board (panel).
    EmbeddedBoard = 21,
    // 22-23 are internal (Trace, SpareVia)
    /// Board definition.
    Board = 24,
    /// Board outline.
    BoardOutline = 25,
}

impl PcbObjectId {
    /// Create from a byte value.
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => PcbObjectId::None,
            1 => PcbObjectId::Arc,
            2 => PcbObjectId::Pad,
            3 => PcbObjectId::Via,
            4 => PcbObjectId::Track,
            5 => PcbObjectId::Text,
            6 => PcbObjectId::Fill,
            7 => PcbObjectId::Connection,
            8 => PcbObjectId::Net,
            9 => PcbObjectId::Component,
            10 => PcbObjectId::Polygon,
            11 => PcbObjectId::Region,
            12 => PcbObjectId::ComponentBody,
            13 => PcbObjectId::Dimension,
            14 => PcbObjectId::Coordinate,
            15 => PcbObjectId::Class,
            16 => PcbObjectId::Rule,
            17 => PcbObjectId::FromTo,
            18 => PcbObjectId::DifferentialPair,
            19 => PcbObjectId::Violation,
            20 => PcbObjectId::Embedded,
            21 => PcbObjectId::EmbeddedBoard,
            24 => PcbObjectId::Board,
            25 => PcbObjectId::BoardOutline,
            _ => PcbObjectId::None,
        }
    }

    /// Convert to byte value.
    pub const fn to_byte(self) -> u8 {
        self as u8
    }

    /// Get the name of this object type.
    pub const fn name(self) -> &'static str {
        match self {
            PcbObjectId::None => "None",
            PcbObjectId::Arc => "Arc",
            PcbObjectId::Pad => "Pad",
            PcbObjectId::Via => "Via",
            PcbObjectId::Track => "Track",
            PcbObjectId::Text => "Text",
            PcbObjectId::Fill => "Fill",
            PcbObjectId::Connection => "Connection",
            PcbObjectId::Net => "Net",
            PcbObjectId::Component => "Component",
            PcbObjectId::Polygon => "Polygon",
            PcbObjectId::Region => "Region",
            PcbObjectId::ComponentBody => "ComponentBody",
            PcbObjectId::Dimension => "Dimension",
            PcbObjectId::Coordinate => "Coordinate",
            PcbObjectId::Class => "Class",
            PcbObjectId::Rule => "Rule",
            PcbObjectId::FromTo => "FromTo",
            PcbObjectId::DifferentialPair => "DifferentialPair",
            PcbObjectId::Violation => "Violation",
            PcbObjectId::Embedded => "Embedded",
            PcbObjectId::EmbeddedBoard => "EmbeddedBoard",
            PcbObjectId::Board => "Board",
            PcbObjectId::BoardOutline => "BoardOutline",
        }
    }
}

bitflags! {
    /// PCB primitive flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PcbFlags: u16 {
        const UNKNOWN2 = 2;
        const UNLOCKED = 4;
        const UNKNOWN8 = 8;
        const UNKNOWN16 = 16;
        const TENTING_TOP = 32;
        const TENTING_BOTTOM = 64;
        const FABRICATION_TOP = 128;
        const FABRICATION_BOTTOM = 256;
        const KEEPOUT = 512;
    }
}

impl Default for PcbFlags {
    fn default() -> Self {
        PcbFlags::UNLOCKED | PcbFlags::UNKNOWN8
    }
}

/// Pad stack mode for pads and vias.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PcbStackMode {
    #[default]
    Simple = 0,
    TopMiddleBottom = 1,
    FullStack = 2,
}

impl PcbStackMode {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => PcbStackMode::Simple,
            1 => PcbStackMode::TopMiddleBottom,
            2 => PcbStackMode::FullStack,
            _ => PcbStackMode::Simple,
        }
    }

    pub const fn to_byte(self) -> u8 {
        self as u8
    }
}

/// Pad shapes.
///
/// Based on DXP API TShape enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PcbPadShape {
    /// No shape defined.
    NoShape = 0,
    /// Round/circular pad.
    #[default]
    Round = 1,
    /// Rectangular pad.
    Rectangular = 2,
    /// Octagonal pad.
    Octagonal = 3,
    /// Circle shape (alternate to Round).
    Circle = 4,
    /// Arc-shaped pad.
    Arc = 5,
    /// Terminator-shaped pad.
    Terminator = 6,
    /// Round rectangle variant.
    RoundRect = 7,
    /// Rotated rectangle.
    RotatedRect = 8,
    /// Rounded rectangular pad.
    RoundedRectangle = 9,
}

impl PcbPadShape {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => PcbPadShape::NoShape,
            1 => PcbPadShape::Round,
            2 => PcbPadShape::Rectangular,
            3 => PcbPadShape::Octagonal,
            4 => PcbPadShape::Circle,
            5 => PcbPadShape::Arc,
            6 => PcbPadShape::Terminator,
            7 => PcbPadShape::RoundRect,
            8 => PcbPadShape::RotatedRect,
            9 => PcbPadShape::RoundedRectangle,
            _ => PcbPadShape::Round,
        }
    }

    pub const fn to_byte(self) -> u8 {
        self as u8
    }

    /// Get the name of this shape.
    pub const fn name(self) -> &'static str {
        match self {
            PcbPadShape::NoShape => "NoShape",
            PcbPadShape::Round => "Round",
            PcbPadShape::Rectangular => "Rectangular",
            PcbPadShape::Octagonal => "Octagonal",
            PcbPadShape::Circle => "Circle",
            PcbPadShape::Arc => "Arc",
            PcbPadShape::Terminator => "Terminator",
            PcbPadShape::RoundRect => "RoundRect",
            PcbPadShape::RotatedRect => "RotatedRect",
            PcbPadShape::RoundedRectangle => "RoundedRectangle",
        }
    }
}

/// Pad hole shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PcbPadHoleShape {
    #[default]
    Round = 0,
    Square = 1,
    Slot = 2,
}

impl PcbPadHoleShape {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => PcbPadHoleShape::Round,
            1 => PcbPadHoleShape::Square,
            2 => PcbPadHoleShape::Slot,
            _ => PcbPadHoleShape::Round,
        }
    }

    pub const fn to_byte(self) -> u8 {
        self as u8
    }
}

/// Text kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PcbTextKind {
    #[default]
    Stroke = 0,
    TrueType = 1,
    BarCode = 2,
}

impl PcbTextKind {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => PcbTextKind::Stroke,
            1 => PcbTextKind::TrueType,
            2 => PcbTextKind::BarCode,
            _ => PcbTextKind::Stroke,
        }
    }

    pub const fn to_byte(self) -> u8 {
        self as u8
    }
}

/// Stroke font types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i16)]
pub enum PcbTextStrokeFont {
    #[default]
    Default = 0,
    SansSerif = 1,
    Serif = 3,
}

impl PcbTextStrokeFont {
    pub fn from_i16(value: i16) -> Self {
        match value {
            0 => PcbTextStrokeFont::Default,
            1 => PcbTextStrokeFont::SansSerif,
            3 => PcbTextStrokeFont::Serif,
            _ => PcbTextStrokeFont::Default,
        }
    }

    pub const fn to_i16(self) -> i16 {
        self as i16
    }
}

/// Text justification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PcbTextJustification {
    BottomRight = 1,
    MiddleRight = 2,
    TopRight = 3,
    BottomCenter = 4,
    #[default]
    MiddleCenter = 5,
    TopCenter = 6,
    BottomLeft = 7,
    MiddleLeft = 8,
    TopLeft = 9,
}

impl PcbTextJustification {
    pub fn from_byte(value: u8) -> Self {
        match value {
            1 => PcbTextJustification::BottomRight,
            2 => PcbTextJustification::MiddleRight,
            3 => PcbTextJustification::TopRight,
            4 => PcbTextJustification::BottomCenter,
            5 => PcbTextJustification::MiddleCenter,
            6 => PcbTextJustification::TopCenter,
            7 => PcbTextJustification::BottomLeft,
            8 => PcbTextJustification::MiddleLeft,
            9 => PcbTextJustification::TopLeft,
            _ => PcbTextJustification::MiddleCenter,
        }
    }

    pub const fn to_byte(self) -> u8 {
        self as u8
    }
}

/// Common fields for all PCB primitives.
#[derive(Debug, Clone, Default)]
pub struct PcbPrimitiveCommon {
    /// PCB layer.
    pub layer: Layer,
    /// Flags.
    pub flags: PcbFlags,
    /// Unique ID (from UniqueIdPrimitiveInformation).
    pub unique_id: Option<String>,
}

impl PcbPrimitiveCommon {
    /// Read common primitive fields from binary stream.
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let layer = Layer(reader.read_u8()?);
        let flags = PcbFlags::from_bits_truncate(reader.read_u16::<LittleEndian>()?);

        // Read and assert 10 0xFF bytes
        let mut ff_bytes = [0u8; 10];
        reader.read_exact(&mut ff_bytes)?;
        // Note: In production, we might want to warn if these aren't all 0xFF

        Ok(PcbPrimitiveCommon {
            layer,
            flags,
            unique_id: None,
        })
    }

    /// Check if the primitive is locked.
    pub fn is_locked(&self) -> bool {
        !self.flags.contains(PcbFlags::UNLOCKED)
    }

    /// Check if top tenting is enabled.
    pub fn is_tenting_top(&self) -> bool {
        self.flags.contains(PcbFlags::TENTING_TOP)
    }

    /// Check if bottom tenting is enabled.
    pub fn is_tenting_bottom(&self) -> bool {
        self.flags.contains(PcbFlags::TENTING_BOTTOM)
    }

    /// Check if this is a keepout.
    pub fn is_keepout(&self) -> bool {
        self.flags.contains(PcbFlags::KEEPOUT)
    }
}

impl FromBinary for PcbPrimitiveCommon {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        PcbPrimitiveCommon::read_from(reader)
    }
}

impl ToBinary for PcbPrimitiveCommon {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u8(self.layer.to_byte())?;
        writer.write_u16::<LittleEndian>(self.flags.bits())?;
        writer.write_all(&[0xFFu8; 10])?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        13
    }
}

impl FromBinary for PcbFlags {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(PcbFlags::from_bits_truncate(
            reader.read_u16::<LittleEndian>()?,
        ))
    }
}

impl ToBinary for PcbFlags {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u16::<LittleEndian>(self.bits())?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        2
    }
}

impl FromBinary for PcbStackMode {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(PcbStackMode::from_byte(reader.read_u8()?))
    }
}

impl ToBinary for PcbStackMode {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u8(self.to_byte())?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        1
    }
}

impl FromBinary for PcbPadShape {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(PcbPadShape::from_byte(reader.read_u8()?))
    }
}

impl ToBinary for PcbPadShape {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u8(self.to_byte())?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        1
    }
}

impl FromBinary for PcbPadHoleShape {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(PcbPadHoleShape::from_byte(reader.read_u8()?))
    }
}

impl ToBinary for PcbPadHoleShape {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u8(self.to_byte())?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        1
    }
}

impl FromBinary for PcbTextKind {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(PcbTextKind::from_byte(reader.read_u8()?))
    }
}

impl ToBinary for PcbTextKind {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u8(self.to_byte())?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        1
    }
}

impl FromBinary for PcbTextStrokeFont {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(PcbTextStrokeFont::from_i16(
            reader.read_i16::<LittleEndian>()?,
        ))
    }
}

impl ToBinary for PcbTextStrokeFont {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_i16::<LittleEndian>(self.to_i16())?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        2
    }
}

impl FromBinary for PcbTextJustification {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(PcbTextJustification::from_byte(reader.read_u8()?))
    }
}

impl ToBinary for PcbTextJustification {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u8(self.to_byte())?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        1
    }
}

// ============================================================================
// Additional enums from DXP API
// ============================================================================

/// Routing corner style.
///
/// Based on DXP API TCornerStyle enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum CornerStyle {
    /// 90 degree corners.
    #[default]
    Deg90 = 0,
    /// 45 degree corners.
    Deg45 = 1,
    /// Rounded corners (arc).
    Round = 2,
}

impl CornerStyle {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => CornerStyle::Deg90,
            1 => CornerStyle::Deg45,
            2 => CornerStyle::Round,
            _ => CornerStyle::Deg90,
        }
    }

    pub const fn to_byte(self) -> u8 {
        self as u8
    }

    pub const fn name(self) -> &'static str {
        match self {
            CornerStyle::Deg90 => "90",
            CornerStyle::Deg45 => "45",
            CornerStyle::Round => "Round",
        }
    }
}

/// Plane connection style for pads/vias.
///
/// Based on DXP API TPlaneConnectionStyle enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PlaneConnectionStyle {
    /// No connection to plane.
    NoConnect = 0,
    /// Relief (thermal) connection.
    #[default]
    ReliefConnect = 1,
    /// Direct connection (no thermal relief).
    DirectConnect = 2,
}

impl PlaneConnectionStyle {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => PlaneConnectionStyle::NoConnect,
            1 => PlaneConnectionStyle::ReliefConnect,
            2 => PlaneConnectionStyle::DirectConnect,
            _ => PlaneConnectionStyle::ReliefConnect,
        }
    }

    pub const fn to_byte(self) -> u8 {
        self as u8
    }

    pub const fn name(self) -> &'static str {
        match self {
            PlaneConnectionStyle::NoConnect => "NoConnect",
            PlaneConnectionStyle::ReliefConnect => "Relief",
            PlaneConnectionStyle::DirectConnect => "Direct",
        }
    }
}

/// Net routing topology.
///
/// Based on DXP API TNetTopology enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum NetTopology {
    /// Shortest path routing.
    #[default]
    Shortest = 0,
    /// Horizontal routing preference.
    Horizontal = 1,
    /// Vertical routing preference.
    Vertical = 2,
    /// Simple daisy chain (sequential).
    DaisyChainSimple = 3,
    /// Mid-driven daisy chain.
    DaisyChainMidDriven = 4,
    /// Balanced daisy chain.
    DaisyChainBalanced = 5,
    /// Starburst (all from center).
    Starburst = 6,
}

impl NetTopology {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => NetTopology::Shortest,
            1 => NetTopology::Horizontal,
            2 => NetTopology::Vertical,
            3 => NetTopology::DaisyChainSimple,
            4 => NetTopology::DaisyChainMidDriven,
            5 => NetTopology::DaisyChainBalanced,
            6 => NetTopology::Starburst,
            _ => NetTopology::Shortest,
        }
    }

    pub const fn to_byte(self) -> u8 {
        self as u8
    }

    pub const fn name(self) -> &'static str {
        match self {
            NetTopology::Shortest => "Shortest",
            NetTopology::Horizontal => "Horizontal",
            NetTopology::Vertical => "Vertical",
            NetTopology::DaisyChainSimple => "DaisyChain",
            NetTopology::DaisyChainMidDriven => "DaisyChainMidDriven",
            NetTopology::DaisyChainBalanced => "DaisyChainBalanced",
            NetTopology::Starburst => "Starburst",
        }
    }
}

/// Dimension object kind.
///
/// Based on DXP API TDimensionKind enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum DimensionKind {
    /// No dimension.
    #[default]
    None = 0,
    /// Linear dimension.
    Linear = 1,
    /// Angular dimension.
    Angular = 2,
    /// Radial dimension.
    Radial = 3,
    /// Leader (callout).
    Leader = 4,
    /// Datum dimension.
    Datum = 5,
    /// Baseline dimension.
    Baseline = 6,
    /// Center dimension.
    Center = 7,
    /// Original dimension.
    Original = 8,
    /// Linear diameter dimension.
    LinearDiameter = 9,
    /// Radial diameter dimension.
    RadialDiameter = 10,
}

impl DimensionKind {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => DimensionKind::None,
            1 => DimensionKind::Linear,
            2 => DimensionKind::Angular,
            3 => DimensionKind::Radial,
            4 => DimensionKind::Leader,
            5 => DimensionKind::Datum,
            6 => DimensionKind::Baseline,
            7 => DimensionKind::Center,
            8 => DimensionKind::Original,
            9 => DimensionKind::LinearDiameter,
            10 => DimensionKind::RadialDiameter,
            _ => DimensionKind::None,
        }
    }

    pub const fn to_byte(self) -> u8 {
        self as u8
    }

    pub const fn name(self) -> &'static str {
        match self {
            DimensionKind::None => "None",
            DimensionKind::Linear => "Linear",
            DimensionKind::Angular => "Angular",
            DimensionKind::Radial => "Radial",
            DimensionKind::Leader => "Leader",
            DimensionKind::Datum => "Datum",
            DimensionKind::Baseline => "Baseline",
            DimensionKind::Center => "Center",
            DimensionKind::Original => "Original",
            DimensionKind::LinearDiameter => "LinearDiameter",
            DimensionKind::RadialDiameter => "RadialDiameter",
        }
    }
}

/// Polygon region kind.
///
/// Based on DXP API TPolyRegionKind enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PolyRegionKind {
    /// Copper region.
    #[default]
    Copper = 0,
    /// Cutout region (removes copper).
    Cutout = 1,
    /// Named region (for design rules).
    NamedRegion = 2,
}

impl PolyRegionKind {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => PolyRegionKind::Copper,
            1 => PolyRegionKind::Cutout,
            2 => PolyRegionKind::NamedRegion,
            _ => PolyRegionKind::Copper,
        }
    }

    pub const fn to_byte(self) -> u8 {
        self as u8
    }

    pub const fn name(self) -> &'static str {
        match self {
            PolyRegionKind::Copper => "Copper",
            PolyRegionKind::Cutout => "Cutout",
            PolyRegionKind::NamedRegion => "NamedRegion",
        }
    }
}

/// Unit system type.
///
/// Based on DXP API TUnit enumeration.
/// Note: This has Metric=0, Imperial=1 (DXP convention).
/// For display units in board settings, see `DisplayUnit` in board.rs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum UnitSystem {
    /// Metric units (mm).
    #[default]
    Metric = 0,
    /// Imperial units (mil/inch).
    Imperial = 1,
}

impl UnitSystem {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => UnitSystem::Metric,
            1 => UnitSystem::Imperial,
            _ => UnitSystem::Metric,
        }
    }

    pub const fn to_byte(self) -> u8 {
        self as u8
    }

    pub const fn name(self) -> &'static str {
        match self {
            UnitSystem::Metric => "Metric",
            UnitSystem::Imperial => "Imperial",
        }
    }
}

/// Text auto-position mode.
///
/// Based on DXP API TTextAutoposition enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum TextAutoposition {
    /// Manual positioning.
    #[default]
    Manual = 0,
    /// Top-left of component.
    TopLeft = 1,
    /// Center-left of component.
    CenterLeft = 2,
    /// Bottom-left of component.
    BottomLeft = 3,
    /// Top-center of component.
    TopCenter = 4,
    /// Center of component.
    CenterCenter = 5,
    /// Bottom-center of component.
    BottomCenter = 6,
    /// Top-right of component.
    TopRight = 7,
    /// Center-right of component.
    CenterRight = 8,
    /// Bottom-right of component.
    BottomRight = 9,
}

impl TextAutoposition {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => TextAutoposition::Manual,
            1 => TextAutoposition::TopLeft,
            2 => TextAutoposition::CenterLeft,
            3 => TextAutoposition::BottomLeft,
            4 => TextAutoposition::TopCenter,
            5 => TextAutoposition::CenterCenter,
            6 => TextAutoposition::BottomCenter,
            7 => TextAutoposition::TopRight,
            8 => TextAutoposition::CenterRight,
            9 => TextAutoposition::BottomRight,
            _ => TextAutoposition::Manual,
        }
    }

    pub const fn to_byte(self) -> u8 {
        self as u8
    }

    pub const fn name(self) -> &'static str {
        match self {
            TextAutoposition::Manual => "Manual",
            TextAutoposition::TopLeft => "TopLeft",
            TextAutoposition::CenterLeft => "CenterLeft",
            TextAutoposition::BottomLeft => "BottomLeft",
            TextAutoposition::TopCenter => "TopCenter",
            TextAutoposition::CenterCenter => "Center",
            TextAutoposition::BottomCenter => "BottomCenter",
            TextAutoposition::TopRight => "TopRight",
            TextAutoposition::CenterRight => "CenterRight",
            TextAutoposition::BottomRight => "BottomRight",
        }
    }
}

/// Component style/package type.
///
/// Based on DXP API TComponentStyle enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ComponentStyle {
    /// Unknown package style.
    #[default]
    Unknown = 0,
    /// Small discrete component.
    Small = 1,
    /// Small SMT component.
    SmallSMT = 2,
    /// Edge connector.
    Edge = 3,
    /// Dual in-line package.
    DIP = 4,
    /// Single in-line package.
    SIP = 5,
    /// SM single in-line package.
    SMSIP = 6,
    /// SM dual in-line package.
    SMDIP = 7,
    /// Leadless chip carrier.
    LCC = 8,
    /// Ball grid array.
    BGA = 9,
    /// Pin grid array.
    PGA = 10,
}

impl ComponentStyle {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => ComponentStyle::Unknown,
            1 => ComponentStyle::Small,
            2 => ComponentStyle::SmallSMT,
            3 => ComponentStyle::Edge,
            4 => ComponentStyle::DIP,
            5 => ComponentStyle::SIP,
            6 => ComponentStyle::SMSIP,
            7 => ComponentStyle::SMDIP,
            8 => ComponentStyle::LCC,
            9 => ComponentStyle::BGA,
            10 => ComponentStyle::PGA,
            _ => ComponentStyle::Unknown,
        }
    }

    pub const fn to_byte(self) -> u8 {
        self as u8
    }

    pub const fn name(self) -> &'static str {
        match self {
            ComponentStyle::Unknown => "Unknown",
            ComponentStyle::Small => "Small",
            ComponentStyle::SmallSMT => "SmallSMT",
            ComponentStyle::Edge => "Edge",
            ComponentStyle::DIP => "DIP",
            ComponentStyle::SIP => "SIP",
            ComponentStyle::SMSIP => "SMSIP",
            ComponentStyle::SMDIP => "SMDIP",
            ComponentStyle::LCC => "LCC",
            ComponentStyle::BGA => "BGA",
            ComponentStyle::PGA => "PGA",
        }
    }
}

/// Dielectric material type in layer stack.
///
/// Based on DXP API TDielectricType enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum DielectricType {
    /// No dielectric.
    #[default]
    None = 0,
    /// Core material.
    Core = 1,
    /// PrePreg material.
    PrePreg = 2,
    /// Surface material.
    SurfaceMaterial = 3,
}

impl DielectricType {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => DielectricType::None,
            1 => DielectricType::Core,
            2 => DielectricType::PrePreg,
            3 => DielectricType::SurfaceMaterial,
            _ => DielectricType::None,
        }
    }

    pub const fn to_byte(self) -> u8 {
        self as u8
    }

    pub const fn name(self) -> &'static str {
        match self {
            DielectricType::None => "None",
            DielectricType::Core => "Core",
            DielectricType::PrePreg => "PrePreg",
            DielectricType::SurfaceMaterial => "SurfaceMaterial",
        }
    }
}

/// Extended drill type.
///
/// Based on DXP API TExtendedDrillType enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ExtendedDrillType {
    /// Standard drilled hole.
    #[default]
    Drilled = 0,
    /// Punched hole.
    Punched = 1,
    /// Laser drilled hole.
    LaserDrilled = 2,
    /// Plasma drilled hole.
    PlasmaDrilled = 3,
}

impl ExtendedDrillType {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => ExtendedDrillType::Drilled,
            1 => ExtendedDrillType::Punched,
            2 => ExtendedDrillType::LaserDrilled,
            3 => ExtendedDrillType::PlasmaDrilled,
            _ => ExtendedDrillType::Drilled,
        }
    }

    pub const fn to_byte(self) -> u8 {
        self as u8
    }

    pub const fn name(self) -> &'static str {
        match self {
            ExtendedDrillType::Drilled => "Drilled",
            ExtendedDrillType::Punched => "Punched",
            ExtendedDrillType::LaserDrilled => "LaserDrilled",
            ExtendedDrillType::PlasmaDrilled => "PlasmaDrilled",
        }
    }
}

/// Board side (top or bottom).
///
/// Based on DXP API TBoardSide enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum BoardSide {
    /// Top side of board.
    #[default]
    Top = 0,
    /// Bottom side of board.
    Bottom = 1,
}

impl BoardSide {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => BoardSide::Top,
            1 => BoardSide::Bottom,
            _ => BoardSide::Top,
        }
    }

    pub const fn to_byte(self) -> u8 {
        self as u8
    }

    pub const fn name(self) -> &'static str {
        match self {
            BoardSide::Top => "Top",
            BoardSide::Bottom => "Bottom",
        }
    }
}

// ============================================================================
// End of DXP API enums
// ============================================================================

/// Base fields for rectangular primitives (Fill, Text).
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(format = "binary")]
pub struct PcbRectangularBase {
    /// Common fields.
    #[altium(flatten)]
    pub common: PcbPrimitiveCommon,
    /// First corner.
    #[altium(coord_point)]
    pub corner1: CoordPoint,
    /// Second corner.
    #[altium(coord_point)]
    pub corner2: CoordPoint,
    /// Rotation angle in degrees.
    pub rotation: f64,
}

impl PcbRectangularBase {
    /// Width of the rectangle.
    pub fn width(&self) -> Coord {
        Coord::from_raw(self.corner2.x.to_raw() - self.corner1.x.to_raw())
    }

    /// Height of the rectangle.
    pub fn height(&self) -> Coord {
        Coord::from_raw(self.corner2.y.to_raw() - self.corner1.y.to_raw())
    }

    /// Calculate bounds (ignoring rotation for now).
    pub fn calculate_bounds(&self) -> CoordRect {
        CoordRect::from_corners(self.corner1, self.corner2)
    }
}

/// Dispatch enum containing all PCB record types.
///
/// Large variants (Pad, ComponentBody) are boxed to reduce enum size on the stack.
#[derive(Debug, Clone)]
pub enum PcbRecord {
    Arc(super::PcbArc),
    Pad(Box<super::PcbPad>),
    Via(super::PcbVia),
    Track(super::PcbTrack),
    Text(super::PcbText),
    Fill(super::PcbFill),
    Region(super::PcbRegion),
    ComponentBody(Box<super::PcbComponentBody>),
    Polygon(super::PcbPolygon),
    /// Unknown record type.
    Unknown {
        object_id: PcbObjectId,
        raw_data: Vec<u8>,
    },
}

impl PcbRecord {
    /// Get the object ID of this record.
    pub fn object_id(&self) -> PcbObjectId {
        match self {
            PcbRecord::Arc(_) => PcbObjectId::Arc,
            PcbRecord::Pad(_) => PcbObjectId::Pad,
            PcbRecord::Via(_) => PcbObjectId::Via,
            PcbRecord::Track(_) => PcbObjectId::Track,
            PcbRecord::Text(_) => PcbObjectId::Text,
            PcbRecord::Fill(_) => PcbObjectId::Fill,
            PcbRecord::Region(_) => PcbObjectId::Region,
            PcbRecord::ComponentBody(_) => PcbObjectId::ComponentBody,
            PcbRecord::Polygon(_) => PcbObjectId::Polygon,
            PcbRecord::Unknown { object_id, .. } => *object_id,
        }
    }

    /// Get the layer of this record.
    pub fn layer(&self) -> Layer {
        match self {
            PcbRecord::Arc(r) => r.common.layer,
            PcbRecord::Pad(r) => r.common.layer,
            PcbRecord::Via(r) => r.common.layer,
            PcbRecord::Track(r) => r.common.layer,
            PcbRecord::Text(r) => r.base.common.layer,
            PcbRecord::Fill(r) => r.base.common.layer,
            PcbRecord::Region(r) => r.common.layer,
            PcbRecord::ComponentBody(r) => r.common.layer,
            PcbRecord::Polygon(r) => r.layer,
            PcbRecord::Unknown { .. } => Layer::default(),
        }
    }
}

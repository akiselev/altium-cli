//! Base primitive trait and SchRecord dispatch enum.
//!
//! **DEPRECATED**: V1 uses trait dispatch for record parsing; v2 uses typed field structs
//! directly (PinData, ComponentData, etc.) for better type safety.

use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::tree::TreeRecord;
use crate::types::{
    CoordPoint, CoordRect, ParameterCollection, coord_to_dxp_frac, dxp_frac_to_coord,
};

/// Base trait for all schematic primitives.
///
/// **DEPRECATED**: V1 uses trait dispatch for record parsing; v2 uses typed field structs
/// directly. Use `v2::fields` types (PinData, ComponentData, etc.) instead.
#[deprecated(note = "Use v2::fields types directly (PinData, ComponentData, etc.)")]
pub trait SchPrimitive: Sized {
    /// Record type ID for this primitive.
    const RECORD_ID: i32;

    /// Import primitive data from parameters.
    fn import_from_params(params: &ParameterCollection) -> Result<Self>;

    /// Export primitive data to parameters.
    fn export_to_params(&self) -> ParameterCollection;

    /// Get the owner index (index of parent primitive in the list).
    fn owner_index(&self) -> i32;

    /// Get the location (if applicable).
    fn location(&self) -> Option<CoordPoint> {
        None
    }

    /// Get the record type name for diagnostics.
    fn record_type_name(&self) -> &'static str;

    /// Get a property value by name (for generic queries).
    fn get_property(&self, _name: &str) -> Option<String> {
        None
    }

    /// Calculate the bounding rectangle.
    fn calculate_bounds(&self) -> CoordRect;
}

/// Common fields shared by all schematic primitives.
///
/// **DEPRECATED**: Use `v2::fields::DataObjectBase` instead.
#[deprecated(note = "Use v2::fields::DataObjectBase")]
#[derive(Debug, Clone)]
pub struct SchPrimitiveBase {
    /// Index of owner primitive in the component's primitive list.
    pub owner_index: i32,
    /// Whether the primitive is not accessible.
    pub is_not_accessible: bool,
    /// Owner part ID (for multi-part symbols).
    pub owner_part_id: Option<i32>,
    /// Owner display mode (for symbols with multiple display modes).
    pub owner_part_display_mode: Option<i32>,
    /// Whether the primitive is graphically locked.
    pub graphically_locked: bool,
    /// Index in sheet — position-based index within the component's Data stream.
    /// In SchLib files: -1 for top-level records (Component, Designator, Comment,
    /// Implementation); positive values = (block_index - 1) for child primitives;
    /// None for records that don't use it (ImplementationList, MapDefinerList, ImplParams).
    pub index_in_sheet: Option<i32>,
}

impl Default for SchPrimitiveBase {
    fn default() -> Self {
        Self {
            owner_index: -1, // -1 indicates no owner
            is_not_accessible: false,
            owner_part_id: None, // None serializes as 1 for pins; overridden to Some(-1) for components
            owner_part_display_mode: None,
            graphically_locked: false,
            index_in_sheet: None, // Computed by writer based on position
        }
    }
}

impl SchPrimitiveBase {
    /// Import base fields from parameters.
    pub fn import_from_params(params: &ParameterCollection) -> Self {
        SchPrimitiveBase {
            owner_index: params
                .get("OWNERINDEX")
                .map(|v| v.as_int_or(0))
                .unwrap_or(0),
            is_not_accessible: params
                .get("ISNOTACCESIBLE")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            owner_part_id: params.get("OWNERPARTID").map(|v| v.as_int_or(0)),
            owner_part_display_mode: params.get("OWNERPARTDISPLAYMODE").map(|v| v.as_int_or(0)),
            graphically_locked: params
                .get("GRAPHICALLYLOCKED")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            index_in_sheet: params.get("INDEXINSHEET").map(|v| v.as_int_or(-1)),
        }
    }

    /// Export base fields to parameters.
    pub fn export_to_params(&self, params: &mut ParameterCollection) {
        params.add_int("OWNERINDEX", self.owner_index);
        // Only emit ISNOTACCESIBLE when true (Altium omits when false)
        if self.is_not_accessible {
            params.add_bool("ISNOTACCESIBLE", true);
        }
        if let Some(part_id) = self.owner_part_id {
            params.add_int("OWNERPARTID", part_id);
        }
        if let Some(display_mode) = self.owner_part_display_mode {
            params.add_int("OWNERPARTDISPLAYMODE", display_mode);
        }
        // Only emit GRAPHICALLYLOCKED when true (Altium omits when false)
        if self.graphically_locked {
            params.add_bool("GRAPHICALLYLOCKED", true);
        }
        // Emit INDEXINSHEET when present (position-based index in SchLib Data streams)
        if let Some(iis) = self.index_in_sheet {
            params.add_int("INDEXINSHEET", iis);
        }
    }

    /// Get the owner index.
    pub fn owner_index(&self) -> i32 {
        self.owner_index
    }

    /// Set the owner index.
    pub fn set_owner_index(&mut self, index: i32) {
        self.owner_index = index;
    }
}

// Trait implementations for derive macro support
impl FromParams for SchPrimitiveBase {
    fn from_params(params: &ParameterCollection) -> Result<Self> {
        Ok(Self::import_from_params(params))
    }
}

impl ToParams for SchPrimitiveBase {
    fn append_to_params(&self, params: &mut ParameterCollection) {
        self.export_to_params(params);
    }
}

/// Common fields for graphical schematic objects (extends SchPrimitiveBase).
///
/// **DEPRECATED**: Use `v2::fields::GraphicalObjectBase` instead.
#[deprecated(note = "Use v2::fields::GraphicalObjectBase")]
#[derive(Debug, Clone, Default)]
pub struct SchGraphicalBase {
    /// Base primitive fields.
    pub base: SchPrimitiveBase,
    /// Location of the object.
    pub location_x: i32,
    pub location_y: i32,
    /// Color (Win32 COLORREF).
    pub color: i32,
    /// Area/fill color (Win32 COLORREF).
    pub area_color: i32,
}

impl SchGraphicalBase {
    /// Standard colors from Altium schematics (Win32 COLORREF format: 0xBBGGRR).
    pub const COLOR_BLUE: i32 = 128; // 0x000080 - Dark blue for components, junctions, ports
    pub const COLOR_RED: i32 = 8388608; // 0x800000 - Dark red for wires, text, labels
    pub const COLOR_LIGHT_CYAN: i32 = 11599871; // 0xB0FFFF - Light cyan for component fill

    /// Create a new SchGraphicalBase with default colors for graphical objects (blue).
    ///
    /// Use this for components, junctions, ports, and other graphical primitives.
    pub fn new_graphical() -> Self {
        Self {
            base: SchPrimitiveBase {
                owner_index: -1,
                is_not_accessible: false,
                owner_part_id: Some(-1),
                owner_part_display_mode: None,
                graphically_locked: false,
                index_in_sheet: None,
            },
            location_x: 0,
            location_y: 0,
            color: Self::COLOR_BLUE,
            area_color: Self::COLOR_LIGHT_CYAN,
        }
    }

    /// Create a new SchGraphicalBase with default colors for wires and text (red).
    ///
    /// Use this for wires, labels, and text objects.
    pub fn new_wire_or_text() -> Self {
        Self {
            base: SchPrimitiveBase {
                owner_index: -1,
                is_not_accessible: false,
                owner_part_id: Some(-1),
                owner_part_display_mode: None,
                graphically_locked: false,
                index_in_sheet: None,
            },
            location_x: 0,
            location_y: 0,
            color: Self::COLOR_RED,
            area_color: 0,
        }
    }

    /// Import graphical fields from parameters.
    pub fn import_from_params(params: &ParameterCollection) -> Self {
        let base = SchPrimitiveBase::import_from_params(params);

        let loc_x = params
            .get("LOCATION.X")
            .map(|v| v.as_int_or(0))
            .unwrap_or(0);
        let loc_x_frac = params
            .get("LOCATION.X_FRAC")
            .map(|v| v.as_int_or(0))
            .unwrap_or(0);
        let loc_y = params
            .get("LOCATION.Y")
            .map(|v| v.as_int_or(0))
            .unwrap_or(0);
        let loc_y_frac = params
            .get("LOCATION.Y_FRAC")
            .map(|v| v.as_int_or(0))
            .unwrap_or(0);

        SchGraphicalBase {
            base,
            location_x: dxp_frac_to_coord(loc_x, loc_x_frac),
            location_y: dxp_frac_to_coord(loc_y, loc_y_frac),
            color: params.get("COLOR").map(|v| v.as_int_or(0)).unwrap_or(0),
            area_color: params.get("AREACOLOR").map(|v| v.as_int_or(0)).unwrap_or(0),
        }
    }

    /// Export graphical fields to parameters.
    pub fn export_to_params(&self, params: &mut ParameterCollection) {
        self.base.export_to_params(params);

        let (x, x_frac) = coord_to_dxp_frac(self.location_x);
        let (y, y_frac) = coord_to_dxp_frac(self.location_y);
        // Only emit location when non-zero (Altium omits zero locations)
        if x != 0 {
            params.add_int("LOCATION.X", x);
        }
        if x_frac != 0 {
            params.add_int("LOCATION.X_FRAC", x_frac);
        }
        if y != 0 {
            params.add_int("LOCATION.Y", y);
        }
        if y_frac != 0 {
            params.add_int("LOCATION.Y_FRAC", y_frac);
        }
        params.add_int("COLOR", self.color);
        // Only emit AREACOLOR when non-zero
        if self.area_color != 0 {
            params.add_int("AREACOLOR", self.area_color);
        }
    }

    /// Get the owner index (delegates to base).
    pub fn owner_index(&self) -> i32 {
        self.base.owner_index
    }

    /// Set the owner index (delegates to base).
    pub fn set_owner_index(&mut self, index: i32) {
        self.base.owner_index = index;
    }
}

// Trait implementations for derive macro support
impl FromParams for SchGraphicalBase {
    fn from_params(params: &ParameterCollection) -> Result<Self> {
        Ok(Self::import_from_params(params))
    }
}

impl ToParams for SchGraphicalBase {
    fn append_to_params(&self, params: &mut ParameterCollection) {
        self.export_to_params(params);
    }
}

/// Dispatch enum containing all schematic record types.
///
/// **DEPRECATED**: V1 uses enum dispatch for dynamic record parsing; v2 uses typed field
/// structs directly for better type safety. Use `v2::io::SchLibV2` or `v2::io::SchDocV2`
/// for parsing, which return typed records directly.
#[deprecated(note = "Use v2 typed structs directly (PinData, ComponentData, etc.)")]
#[derive(Debug, Clone)]
pub enum SchRecord {
    Component(super::SchComponent),
    Pin(super::SchPin),
    Symbol(super::SchSymbol),
    Label(super::SchLabel),
    Bezier(super::SchBezier),
    Polyline(super::SchPolyline),
    Polygon(super::SchPolygon),
    Ellipse(super::SchEllipse),
    Pie(super::SchPie),
    EllipticalArc(super::SchEllipticalArc),
    Arc(super::SchArc),
    Line(super::SchLine),
    Rectangle(super::SchRectangle),
    PowerObject(super::SchPowerObject),
    Port(super::SchPort),
    NoErc(super::SchNoErc),
    NetLabel(super::SchNetLabel),
    Bus(super::SchBus),
    Wire(super::SchWire),
    TextFrame(super::SchTextFrame),
    TextFrameVariant(super::SchTextFrameVariant),
    Junction(super::SchJunction),
    Image(super::SchImage),
    SheetHeader(super::SchSheetHeader),
    Designator(super::SchDesignator),
    BusEntry(super::SchBusEntry),
    Parameter(super::SchParameter),
    WarningSign(super::SchWarningSign),
    ImplementationList(super::SchImplementationList),
    Implementation(super::SchImplementation),
    MapDefinerList(super::SchMapDefinerList),
    MapDefiner(super::SchMapDefiner),
    ImplementationParameters(super::SchImplementationParameters),
    /// Unknown record type - stores raw parameters.
    Unknown {
        record_id: i32,
        params: ParameterCollection,
    },
}

#[allow(deprecated)]
impl SchRecord {
    /// Create a record from parameters by dispatching on RECORD type.
    ///
    /// **DEPRECATED**: Use `v2::io::SchLibV2` or `v2::io::SchDocV2` for parsing.
    #[deprecated(note = "Use v2::io::SchLibV2 or v2::io::SchDocV2 for parsing")]
    pub fn from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchRecord::from_params is deprecated. \
            Use v2::io::SchLibV2::open() or v2::io::SchDocV2::open() for parsing files."
        )
    }

    /// Get the record type ID.
    pub fn record_id(&self) -> i32 {
        match self {
            SchRecord::Component(_) => 1,
            SchRecord::Pin(_) => 2,
            SchRecord::Symbol(_) => 3,
            SchRecord::Label(_) => 4,
            SchRecord::Bezier(_) => 5,
            SchRecord::Polyline(_) => 6,
            SchRecord::Polygon(_) => 7,
            SchRecord::Ellipse(_) => 8,
            SchRecord::Pie(_) => 9,
            SchRecord::EllipticalArc(_) => 11,
            SchRecord::Arc(_) => 12,
            SchRecord::Line(_) => 13,
            SchRecord::Rectangle(_) => 14,
            SchRecord::PowerObject(_) => 17,
            SchRecord::Port(_) => 18,
            SchRecord::NoErc(_) => 22,
            SchRecord::NetLabel(_) => 25,
            SchRecord::Bus(_) => 26,
            SchRecord::Wire(_) => 27,
            SchRecord::TextFrame(_) => 28,
            SchRecord::Junction(_) => 29,
            SchRecord::Image(_) => 30,
            SchRecord::SheetHeader(_) => 31,
            SchRecord::Designator(_) => 34,
            SchRecord::BusEntry(_) => 37,
            SchRecord::Parameter(_) => 41,
            SchRecord::WarningSign(_) => 43,
            SchRecord::ImplementationList(_) => 44,
            SchRecord::Implementation(_) => 45,
            SchRecord::MapDefinerList(_) => 46,
            SchRecord::MapDefiner(_) => 47,
            SchRecord::ImplementationParameters(_) => 48,
            SchRecord::TextFrameVariant(_) => 209,
            SchRecord::Unknown { record_id, .. } => *record_id,
        }
    }

    /// Get the owner index of this record.
    pub fn owner_index(&self) -> i32 {
        match self {
            SchRecord::Component(r) => r.owner_index(),
            SchRecord::Pin(r) => r.owner_index(),
            SchRecord::Symbol(r) => r.owner_index(),
            SchRecord::Label(r) => r.owner_index(),
            SchRecord::Bezier(r) => r.owner_index(),
            SchRecord::Polyline(r) => r.owner_index(),
            SchRecord::Polygon(r) => r.owner_index(),
            SchRecord::Ellipse(r) => r.owner_index(),
            SchRecord::Pie(r) => r.owner_index(),
            SchRecord::EllipticalArc(r) => r.owner_index(),
            SchRecord::Arc(r) => r.owner_index(),
            SchRecord::Line(r) => r.owner_index(),
            SchRecord::Rectangle(r) => r.owner_index(),
            SchRecord::PowerObject(r) => r.owner_index(),
            SchRecord::Port(r) => r.owner_index(),
            SchRecord::NoErc(r) => r.owner_index(),
            SchRecord::NetLabel(r) => r.owner_index(),
            SchRecord::Bus(r) => r.owner_index(),
            SchRecord::Wire(r) => r.owner_index(),
            SchRecord::TextFrame(r) => r.owner_index(),
            SchRecord::TextFrameVariant(r) => r.owner_index(),
            SchRecord::Junction(r) => r.owner_index(),
            SchRecord::Image(r) => r.owner_index(),
            SchRecord::SheetHeader(r) => r.owner_index(),
            SchRecord::Designator(r) => r.owner_index(),
            SchRecord::BusEntry(r) => r.owner_index(),
            SchRecord::Parameter(r) => r.owner_index(),
            SchRecord::WarningSign(r) => r.owner_index(),
            SchRecord::ImplementationList(r) => r.owner_index(),
            SchRecord::Implementation(r) => r.owner_index(),
            SchRecord::MapDefinerList(r) => r.owner_index(),
            SchRecord::MapDefiner(r) => r.owner_index(),
            SchRecord::ImplementationParameters(r) => r.owner_index(),
            SchRecord::Unknown { params, .. } => params
                .get("OWNERINDEX")
                .map(|v| v.as_int_or(0))
                .unwrap_or(0),
        }
    }

    /// Set the owner index for this record.
    pub fn set_owner_index(&mut self, index: i32) {
        match self {
            SchRecord::Component(r) => r.graphical.base.set_owner_index(index),
            SchRecord::Pin(r) => r.graphical.set_owner_index(index),
            SchRecord::Symbol(r) => r.graphical.set_owner_index(index),
            SchRecord::Label(r) => r.graphical.set_owner_index(index),
            SchRecord::Bezier(r) => r.graphical.set_owner_index(index),
            SchRecord::Polyline(r) => r.graphical.set_owner_index(index),
            SchRecord::Polygon(r) => r.graphical.set_owner_index(index),
            SchRecord::Ellipse(r) => r.graphical.set_owner_index(index),
            SchRecord::Pie(r) => r.graphical.set_owner_index(index),
            SchRecord::EllipticalArc(r) => r.graphical.set_owner_index(index),
            SchRecord::Arc(r) => r.graphical.set_owner_index(index),
            SchRecord::Line(r) => r.graphical.set_owner_index(index),
            SchRecord::Rectangle(r) => r.graphical.set_owner_index(index),
            SchRecord::PowerObject(r) => r.graphical.set_owner_index(index),
            SchRecord::Port(r) => r.graphical.set_owner_index(index),
            SchRecord::NoErc(r) => r.graphical.set_owner_index(index),
            SchRecord::NetLabel(r) => r.label.graphical.set_owner_index(index),
            SchRecord::Bus(r) => r.graphical.set_owner_index(index),
            SchRecord::Wire(r) => r.graphical.set_owner_index(index),
            SchRecord::TextFrame(r) => r.graphical.set_owner_index(index),
            SchRecord::TextFrameVariant(r) => r.graphical.set_owner_index(index),
            SchRecord::Junction(r) => r.graphical.set_owner_index(index),
            SchRecord::Image(r) => r.graphical.set_owner_index(index),
            SchRecord::SheetHeader(r) => r.base.set_owner_index(index),
            SchRecord::Designator(r) => r.param.label.graphical.set_owner_index(index),
            SchRecord::BusEntry(r) => r.graphical.set_owner_index(index),
            SchRecord::Parameter(r) => r.label.graphical.set_owner_index(index),
            SchRecord::WarningSign(r) => r.graphical.set_owner_index(index),
            SchRecord::ImplementationList(r) => r.base.set_owner_index(index),
            SchRecord::Implementation(r) => r.base.set_owner_index(index),
            SchRecord::MapDefinerList(r) => r.base.set_owner_index(index),
            SchRecord::MapDefiner(r) => r.base.set_owner_index(index),
            SchRecord::ImplementationParameters(r) => r.base.set_owner_index(index),
            SchRecord::Unknown { params, .. } => {
                params.add_int("OWNERINDEX", index);
            }
        }
    }

    /// Set the index_in_sheet for this record.
    ///
    /// In SchLib files, INDEXINSHEET encodes positional information:
    /// - Component, Designator, Comment, Implementation: Some(-1)
    /// - Child primitives (shapes, pins, params): Some(block_index - 1), or None when 0
    /// - ImplementationList, MapDefinerList, ImplementationParameters: None
    pub fn set_index_in_sheet(&mut self, value: Option<i32>) {
        match self {
            SchRecord::Component(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::Pin(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::Symbol(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::Label(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::Bezier(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::Polyline(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::Polygon(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::Ellipse(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::Pie(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::EllipticalArc(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::Arc(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::Line(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::Rectangle(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::PowerObject(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::Port(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::NoErc(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::NetLabel(r) => r.label.graphical.base.index_in_sheet = value,
            SchRecord::Bus(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::Wire(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::TextFrame(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::TextFrameVariant(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::Junction(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::Image(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::SheetHeader(r) => r.base.index_in_sheet = value,
            SchRecord::Designator(r) => r.param.label.graphical.base.index_in_sheet = value,
            SchRecord::BusEntry(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::Parameter(r) => r.label.graphical.base.index_in_sheet = value,
            SchRecord::WarningSign(r) => r.graphical.base.index_in_sheet = value,
            SchRecord::ImplementationList(r) => r.base.index_in_sheet = value,
            SchRecord::Implementation(r) => r.base.index_in_sheet = value,
            SchRecord::MapDefinerList(r) => r.base.index_in_sheet = value,
            SchRecord::MapDefiner(r) => r.base.index_in_sheet = value,
            SchRecord::ImplementationParameters(r) => r.base.index_in_sheet = value,
            SchRecord::Unknown { .. } => {} // Unknown records don't have structured base
        }
    }

    /// Get location for all record types.
    pub fn location(&self) -> Option<CoordPoint> {
        match self {
            SchRecord::Component(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::Pin(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::Symbol(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::Label(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::Bezier(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::Polyline(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::Polygon(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::Ellipse(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::Pie(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::EllipticalArc(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::Arc(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::Line(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::Rectangle(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::PowerObject(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::Port(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::NoErc(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::NetLabel(r) => Some(CoordPoint::from_raw(
                r.label.graphical.location_x,
                r.label.graphical.location_y,
            )),
            SchRecord::Bus(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::Wire(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::TextFrame(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::TextFrameVariant(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::Junction(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::Image(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::SheetHeader(_) => None,
            SchRecord::Designator(r) => Some(CoordPoint::from_raw(
                r.param.label.graphical.location_x,
                r.param.label.graphical.location_y,
            )),
            SchRecord::BusEntry(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::Parameter(r) => Some(CoordPoint::from_raw(
                r.label.graphical.location_x,
                r.label.graphical.location_y,
            )),
            SchRecord::WarningSign(r) => Some(CoordPoint::from_raw(
                r.graphical.location_x,
                r.graphical.location_y,
            )),
            SchRecord::ImplementationList(_) => None,
            SchRecord::Implementation(_) => None,
            SchRecord::MapDefinerList(_) => None,
            SchRecord::MapDefiner(_) => None,
            SchRecord::ImplementationParameters(_) => None,
            SchRecord::Unknown { .. } => None,
        }
    }

    /// Get record type name for diagnostics.
    pub fn record_type_name(&self) -> &'static str {
        match self {
            SchRecord::Component(_) => "Component",
            SchRecord::Pin(_) => "Pin",
            SchRecord::Symbol(_) => "Symbol",
            SchRecord::Label(_) => "Label",
            SchRecord::Bezier(_) => "Bezier",
            SchRecord::Polyline(_) => "Polyline",
            SchRecord::Polygon(_) => "Polygon",
            SchRecord::Ellipse(_) => "Ellipse",
            SchRecord::Pie(_) => "Pie",
            SchRecord::EllipticalArc(_) => "EllipticalArc",
            SchRecord::Arc(_) => "Arc",
            SchRecord::Line(_) => "Line",
            SchRecord::Rectangle(_) => "Rectangle",
            SchRecord::PowerObject(_) => "PowerObject",
            SchRecord::Port(_) => "Port",
            SchRecord::NoErc(_) => "NoErc",
            SchRecord::NetLabel(_) => "NetLabel",
            SchRecord::Bus(_) => "Bus",
            SchRecord::Wire(_) => "Wire",
            SchRecord::TextFrame(_) => "TextFrame",
            SchRecord::TextFrameVariant(_) => "TextFrameVariant",
            SchRecord::Junction(_) => "Junction",
            SchRecord::Image(_) => "Image",
            SchRecord::SheetHeader(_) => "SheetHeader",
            SchRecord::Designator(_) => "Designator",
            SchRecord::BusEntry(_) => "BusEntry",
            SchRecord::Parameter(_) => "Parameter",
            SchRecord::WarningSign(_) => "WarningSign",
            SchRecord::ImplementationList(_) => "ImplementationList",
            SchRecord::Implementation(_) => "Implementation",
            SchRecord::MapDefinerList(_) => "MapDefinerList",
            SchRecord::MapDefiner(_) => "MapDefiner",
            SchRecord::ImplementationParameters(_) => "ImplementationParameters",
            SchRecord::Unknown { .. } => "Unknown",
        }
    }

    /// Get property value by name for generic queries.
    pub fn get_property(&self, name: &str) -> Option<String> {
        match self {
            SchRecord::Component(r) => match name {
                "LIBREFERENCE" => Some(r.lib_reference.clone()),
                "COMPONENTDESCRIPTION" => Some(r.component_description.clone()),
                "UNIQUEID" => Some(r.unique_id.clone()),
                _ => None,
            },
            SchRecord::Pin(r) => match name {
                "NAME" => Some(r.name.clone()),
                "DESIGNATOR" => Some(r.designator.clone()),
                "DESCRIPTION" => Some(r.description.clone()),
                _ => None,
            },
            SchRecord::Label(r) => match name {
                "TEXT" => Some(r.text.clone()),
                _ => None,
            },
            SchRecord::PowerObject(r) => match name {
                "TEXT" => Some(r.text.clone()),
                _ => None,
            },
            SchRecord::Port(r) => match name {
                "NAME" => Some(r.name.clone()),
                _ => None,
            },
            SchRecord::NetLabel(r) => match name {
                "TEXT" => Some(r.label.text.clone()),
                _ => None,
            },
            SchRecord::TextFrame(r) => match name {
                "TEXT" => Some(r.text.clone()),
                _ => None,
            },
            SchRecord::TextFrameVariant(r) => match name {
                "TEXT" => Some(r.text.clone()),
                _ => None,
            },
            SchRecord::Parameter(r) => match name {
                "NAME" => Some(r.name.clone()),
                "TEXT" => Some(r.label.text.clone()),
                _ => None,
            },
            SchRecord::Designator(r) => match name {
                "NAME" => Some(r.param.name.clone()),
                "TEXT" => Some(r.param.label.text.clone()),
                _ => None,
            },
            SchRecord::WarningSign(r) => match name {
                "NAME" => Some(r.name.clone()),
                _ => None,
            },
            SchRecord::Implementation(r) => match name {
                "MODELNAME" => Some(r.model_name.clone()),
                "MODELTYPE" => Some(r.model_type.clone()),
                _ => None,
            },
            _ => None,
        }
    }
}

/// TreeRecord implementation for SchRecord enables tree structure support.
impl TreeRecord for SchRecord {
    fn owner_index(&self) -> i32 {
        self.owner_index()
    }

    fn set_owner_index(&mut self, index: i32) {
        self.set_owner_index(index)
    }
}

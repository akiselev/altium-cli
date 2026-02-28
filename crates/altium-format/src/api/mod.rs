//! High-level public API for querying and mutating Altium documents.
//!
//! This module provides clean, domain-typed interfaces that abstract away
//! internal format details like `SchRecord`, `owner_index` linking, sidecar
//! streams, and CFB structure.
//!
//! # SchLib Example
//!
//! ```no_run
//! use altium_format::SchLib;
//! use altium_format::api::Component;
//!
//! let lib = SchLib::open("my_library.SchLib").unwrap();
//! for name in lib.component_names() {
//!     let comp = lib.component(&name).unwrap();
//!     println!("{}: {} pins", comp.lib_reference, comp.pins.len());
//! }
//! ```

mod schlib_types;
pub(crate) mod schlib_read;
pub(crate) mod schlib_write;
mod pcblib_types;
pub(crate) mod pcblib_read;
pub(crate) mod pcblib_write;
mod project_types;
pub(crate) mod project_read;
pub(crate) mod project_write;

// ── SchLib types ─────────────────────────────────────────────────────────────

pub use schlib_types::{
    // Component and children
    Component, Pin, PinTextPositioning, Parameter, FootprintMap, PinPadMap,
    // Graphic enum and variants
    Graphic, LineGraphic, RectangleGraphic, RoundRectangleGraphic,
    ArcGraphic, EllipticalArcGraphic, EllipseGraphic, PieGraphic,
    PolylineGraphic, PolygonGraphic, BezierGraphic,
    ImageGraphic, LabelGraphic, TextFrameGraphic,
};

// ── PcbLib types ─────────────────────────────────────────────────────────────

pub use pcblib_types::{
    Footprint, Pad, PcbGraphic,
    TrackGraphic, PcbArcGraphic, FillGraphic, RegionGraphic,
    TextGraphic, ViaGraphic, ComponentBodyGraphic,
};

// ── PrjPcb types ────────────────────────────────────────────────────────────

pub use project_types::{
    Project, DocumentRef, BuildConfiguration, OutputGroup, OutputJob,
    AnnotationSettings, AnnotationMatchParameter, ClassGenSettings,
    LibraryUpdateSettings, DatabaseUpdateSettings, ComparisonOption,
    ErcConnectionMatrix, ErcLevel, ModificationLevel, DifferenceLevel,
    ProjectVariant, ComponentVariation, ParameterVariation,
    ProjectParameter, DiffPairSuffix, NetInfo,
};

// Re-export SchAngle so consumers can construct angle values
pub use crate::param_value::SchAngle;

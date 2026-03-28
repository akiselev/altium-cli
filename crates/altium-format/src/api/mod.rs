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

pub(crate) mod pcb_common;
pub(crate) mod pcbdoc_read;
mod pcbdoc_types;
pub(crate) mod pcbdoc_write;
pub(crate) mod pcblib_read;
mod pcblib_types;
pub(crate) mod pcblib_write;
pub(crate) mod project_read;
mod project_types;
pub(crate) mod project_write;
pub(crate) mod sch_common;
pub(crate) mod schdoc_read;
mod schdoc_types;
pub(crate) mod schdoc_write;
pub(crate) mod schlib_read;
mod schlib_types;
pub(crate) mod schlib_write;

// ── SchLib types ─────────────────────────────────────────────────────────────

pub use schlib_types::{
    ArcGraphic,
    BezierGraphic,
    // Component and children
    Component,
    EllipseGraphic,
    EllipticalArcGraphic,
    FootprintMap,
    // Graphic enum and variants
    Graphic,
    ImageGraphic,
    LabelGraphic,
    LineGraphic,
    Parameter,
    PieGraphic,
    Pin,
    PinPadMap,
    PinTextPositioning,
    PolygonGraphic,
    PolylineGraphic,
    RectangleGraphic,
    RoundRectangleGraphic,
    TextFrameGraphic,
};

// ── SchDoc types ────────────────────────────────────────────────────────────

pub use schdoc_types::{
    Blanket, Bus, BusEntry, CompileMask, ComponentChild, Font, HarnessChild, HarnessConnector,
    Junction, NetLabel, NoConnect, Note, ParameterSet, Port, PowerObject, Probe, SchDocComponent,
    SchDocSheet, SheetEntry, SheetObject, SheetSymbol, SheetSymbolChild, SignalHarness, Template,
    Wire,
};

// ── Shared PCB types ────────────────────────────────────────────────────────

pub use pcb_common::{ContourSegment, PadInnerLayerOverride, PadLayerShape, PadStack, PcbContour};

// ── PcbLib types ─────────────────────────────────────────────────────────────

pub use pcblib_types::{
    ComponentBodyGraphic, FillGraphic, Footprint, Pad, PcbArcGraphic, PcbGraphic, RegionGraphic,
    TextGraphic, TrackGraphic, ViaGraphic,
};

// ── PcbDoc types ────────────────────────────────────────────────────────────

pub use pcbdoc_types::{
    Arc, BoardConnectivity, BoardContour, BoardGeometry, BoardSettings, ComponentBody, DesignRule,
    DifferentialPair, Dimension, DrillPairGroup, Fill, KeepoutZone, LayerPrimitives, LayerStack,
    Model3D, Net, NetClass, NetPin, NetPinList, Pad as PcbDocPad, PcbDocBoard, PcbDocComponent,
    Polygon, Region, RuleParams, StackLayer, Text as PcbDocText, Track, Via,
};

// ── PrjPcb types ────────────────────────────────────────────────────────────

pub use project_types::{
    AnnotationMatchParameter, AnnotationSettings, BuildConfiguration, ClassGenSettings,
    ComparisonOption, ComponentVariation, DatabaseUpdateSettings, DiffPairSuffix, DifferenceLevel,
    DocumentRef, ErcConnectionMatrix, ErcLevel, LibraryUpdateSettings, ModificationLevel, NetInfo,
    OutputGroup, OutputJob, ParameterVariation, Project, ProjectParameter, ProjectVariant,
};

// Re-export SchAngle so consumers can construct angle values
pub use crate::param_value::SchAngle;

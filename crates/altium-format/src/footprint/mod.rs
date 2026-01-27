//! Footprint creation and manipulation module.
//!
//! Provides a builder API for creating PCB footprints programmatically,
//! rendering to SVG and ASCII art for visualization, and measurement
//! utilities for verifying footprint correctness.

mod builder;
pub mod measure;
mod package;
mod render;

pub use builder::{FootprintBuilder, PadRowDirection};
pub use measure::{
    ClearanceResult, FootprintDimensions, Measurement, MeasurementReport, PadDistance, PadInfo,
    PitchAnalysis, analyze_pitch, generate_report, measure_dimensions, measure_pad,
    measure_pad_distance, minimum_pad_clearance, pad_to_silkscreen_clearance,
};
pub use package::{ChipSpec, GullWingSpec, IpcDensity, LeadStyle, PackageType};
pub use render::{AsciiOptions, SvgOptions, render_ascii, render_svg};

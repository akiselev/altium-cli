//! Rendering module for Altium Designer file formats.
//!
//! Provides the `AltiumCanvas` trait and dispatch functions for drawing
//! schematic and PCB primitives. Backends implement `AltiumCanvas` to
//! produce SVG, PDF, PNG, or any other output format.

pub mod canvas;
pub(crate) mod pcb;
pub mod recording;
pub(crate) mod sch;

pub use canvas::{AltiumCanvas, Brush, DrawPoint, FontSpec, Pen, RenderTransform, TextHAlign, TextVAlign};
pub use recording::{DrawCall, NullCanvas, RecordingCanvas};

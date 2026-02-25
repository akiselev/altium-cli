//! PNG rasterization backend for Altium Designer files.
//!
//! Converts Altium documents to PNG by rendering SVG via [`altium_format_render_svg`]
//! and rasterizing with [`resvg`] / [`tiny_skia`].

use resvg::{tiny_skia, usvg};

/// Default pixels-per-mil scale factor for schematic rendering.
/// At 4 px/mil a 500-mil-wide component → 2000px.
pub const DEFAULT_SCALE: f32 = 4.0;

/// Error type for PNG rendering.
#[derive(Debug)]
pub enum PngRenderError {
    AltiumFormat(altium_format::AltiumFormatError),
    SvgParse(usvg::Error),
    PngEncode(String),
    EmptyDocument,
}

impl std::fmt::Display for PngRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AltiumFormat(e) => write!(f, "altium format error: {e}"),
            Self::SvgParse(e) => write!(f, "SVG parse error: {e}"),
            Self::PngEncode(e) => write!(f, "PNG encode error: {e}"),
            Self::EmptyDocument => write!(f, "document has no drawable content"),
        }
    }
}

impl std::error::Error for PngRenderError {}

impl From<altium_format::AltiumFormatError> for PngRenderError {
    fn from(e: altium_format::AltiumFormatError) -> Self {
        Self::AltiumFormat(e)
    }
}

fn svg_to_png(svg_str: &str, scale: f32) -> Result<Vec<u8>, PngRenderError> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg_str, &opt).map_err(PngRenderError::SvgParse)?;
    let sz = tree.size().to_int_size();
    if sz.width() == 0 || sz.height() == 0 {
        return Err(PngRenderError::EmptyDocument);
    }
    let w = ((sz.width() as f32 * scale) as u32).max(1);
    let h = ((sz.height() as f32 * scale) as u32).max(1);
    let mut pixmap = tiny_skia::Pixmap::new(w, h)
        .ok_or_else(|| PngRenderError::PngEncode("failed to allocate pixmap".to_owned()))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|e| PngRenderError::PngEncode(e.to_string()))
}

/// Render a SchLib component to PNG bytes.
pub fn render_schlib_component_png(
    lib: &altium_format::SchLib,
    name: &str,
    scale: f32,
) -> Result<Vec<u8>, PngRenderError> {
    let svg = altium_format_render_svg::render_schlib_component(lib, name)?;
    svg_to_png(&svg, scale)
}

/// Render an entire SchDoc sheet to PNG bytes.
pub fn render_schdoc_png(
    doc: &altium_format::SchDoc,
    scale: f32,
) -> Result<Vec<u8>, PngRenderError> {
    let svg = altium_format_render_svg::render_schdoc(doc)?;
    svg_to_png(&svg, scale)
}

/// Render a PcbLib footprint to PNG bytes.
pub fn render_pcblib_footprint_png(
    lib: &altium_format::PcbLib,
    name: &str,
    scale: f32,
) -> Result<Vec<u8>, PngRenderError> {
    let svg = altium_format_render_svg::render_pcblib_footprint(lib, name)?;
    svg_to_png(&svg, scale)
}

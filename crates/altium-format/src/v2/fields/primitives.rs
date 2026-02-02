//! Primitive geometric record data structs.

use crate::v2::types::*;
use super::GraphicalObjectBase;

/// Arc record data — from `ExportArc`/`ImportArc` (ObjectId::Arc = 12).
#[derive(Clone, Debug, Default)]
pub struct ArcData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub radius: i32,
    pub line_width: Size,
    pub start_angle: f64,
    pub end_angle: f64,
    pub color: u32,
    pub unique_id: String,
}

/// Line record data — from `ExportLine`/`ImportLine` (ObjectId::Line = 13).
#[derive(Clone, Debug, Default)]
pub struct LineData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub corner_x: i32,
    pub corner_y: i32,
    pub line_width: Size,
    pub line_style: LineStyle,
    pub color: u32,
    pub unique_id: String,
}

/// Rectangle record data — from `ExportRectangle`/`ImportRectangle` (ObjectId::Rectangle = 14).
#[derive(Clone, Debug, Default)]
pub struct RectangleData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub corner_x: i32,
    pub corner_y: i32,
    pub line_style: LineStyle,
    pub line_width: Size,
    pub color: u32,
    pub area_color: u32,
    pub is_solid: bool,
    pub transparent: bool,
    pub unique_id: String,
}

/// Ellipse record data — from `ExportEllipse`/`ImportEllipse` (ObjectId::Ellipse = 11).
#[derive(Clone, Debug, Default)]
pub struct EllipseData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub radius: i32,
    pub secondary_radius: i32,
    pub line_width: Size,
    pub color: u32,
    pub area_color: u32,
    pub is_solid: bool,
    pub transparent: bool,
    pub unique_id: String,
}

/// Polygon record data — from `ExportPolygon`/`ImportPolygon` (ObjectId::Polygon = 7).
#[derive(Clone, Debug, Default)]
pub struct PolygonData {
    pub graphical: GraphicalObjectBase,
    pub line_width: Size,
    pub color: u32,
    pub area_color: u32,
    pub is_solid: bool,
    pub transparent: bool,
    pub vertices: Vec<(i32, i32)>,
    pub unique_id: String,
}

/// Polyline record data — from `ExportPolyline`/`ImportPolyline` (ObjectId::Polyline = 6).
#[derive(Clone, Debug, Default)]
pub struct PolylineData {
    pub graphical: GraphicalObjectBase,
    pub line_width: Size,
    pub line_style: LineStyle,
    pub start_line_shape: LineShape,
    pub end_line_shape: LineShape,
    pub line_shape_size: Size,
    pub color: u32,
    pub vertices: Vec<(i32, i32)>,
    pub unique_id: String,
}

/// Bezier record data — from `ExportBezier`/`ImportBezier` (ObjectId::Bezier = 5).
#[derive(Clone, Debug, Default)]
pub struct BezierData {
    pub graphical: GraphicalObjectBase,
    pub line_width: Size,
    pub color: u32,
    pub vertices: Vec<(i32, i32)>,
    pub unique_id: String,
}

/// RoundRectangle record data — from `ExportRoundRectangle`/`ImportRoundRectangle`.
#[derive(Clone, Debug, Default)]
pub struct RoundRectangleData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub corner_x: i32,
    pub corner_y: i32,
    pub corner_x_radius: i32,
    pub corner_y_radius: i32,
    pub line_width: Size,
    pub color: u32,
    pub area_color: u32,
    pub is_solid: bool,
    pub unique_id: String,
}

/// Elliptical arc record data — from `ExportEllipticalArc`/`ImportEllipticalArc`.
#[derive(Clone, Debug, Default)]
pub struct EllipticalArcData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub radius: i32,
    pub secondary_radius: i32,
    pub line_width: Size,
    pub start_angle: f64,
    pub end_angle: f64,
    pub color: u32,
    pub unique_id: String,
}

/// Pie record data — from `ExportPie`/`ImportPie`.
#[derive(Clone, Debug, Default)]
pub struct PieData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub radius: i32,
    pub line_width: Size,
    pub start_angle: f64,
    pub end_angle: f64,
    pub color: u32,
    pub area_color: u32,
    pub is_solid: bool,
}

/// Image record data — from `ExportImage`/`ImportImage` (ObjectId::Image = 30).
#[derive(Clone, Debug, Default)]
pub struct ImageData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub corner_x: i32,
    pub corner_y: i32,
    pub orientation: RotationBy90,
    pub line_width: Size,
    pub color: u32,
    pub is_solid: bool,
    pub keep_aspect: bool,
    pub embed_image: bool,
    pub file_name: String,
    pub unique_id: String,
}

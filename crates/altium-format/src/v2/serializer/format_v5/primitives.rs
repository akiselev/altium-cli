//! Format functions for primitive geometric record types.

use crate::error::Result;
use crate::v2::fields::primitives::*;
use crate::v2::serializer::SchSerializer;
use super::{export_graphical_object, import_graphical_object, export_vertices, import_vertices};

// ============================================================================
// Arc (ObjectId = 12)
// ============================================================================

pub fn export_arc(s: &mut dyn SchSerializer, arc: &ArcData) -> Result<()> {
    export_graphical_object(s, &arc.graphical)?;
    s.export_coord(arc.location_x, "Location.X")?;
    s.export_coord(arc.location_y, "Location.Y")?;
    s.export_coord(arc.radius, "Radius")?;
    s.export_size(arc.line_width, "LineWidth")?;
    s.export_angle(arc.start_angle, "StartAngle")?;
    s.export_angle(arc.end_angle, "EndAngle")?;
    s.export_color(arc.color, "Color")?;
    s.export_dynamic_string(&arc.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_arc(s: &mut dyn SchSerializer, arc: &mut ArcData) -> Result<()> {
    import_graphical_object(s, &mut arc.graphical)?;
    arc.location_x = s.import_coord("Location.X")?;
    arc.location_y = s.import_coord("Location.Y")?;
    arc.radius = s.import_coord("Radius")?;
    arc.line_width = s.import_size("LineWidth")?;
    arc.start_angle = s.import_angle("StartAngle")?;
    arc.end_angle = s.import_angle("EndAngle")?;
    arc.color = s.import_color("Color")?;
    arc.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Ellipse (ObjectId = 11)
// ============================================================================

pub fn export_ellipse(s: &mut dyn SchSerializer, e: &EllipseData) -> Result<()> {
    export_graphical_object(s, &e.graphical)?;
    s.export_coord(e.location_x, "Location.X")?;
    s.export_coord(e.location_y, "Location.Y")?;
    s.export_coord(e.radius, "Radius")?;
    s.export_coord(e.secondary_radius, "SecondaryRadius")?;
    s.export_size(e.line_width, "LineWidth")?;
    s.export_color(e.color, "Color")?;
    s.export_color(e.area_color, "AreaColor")?;
    s.export_boolean(e.is_solid, "IsSolid")?;
    s.export_boolean(e.transparent, "Transparent")?;
    s.export_dynamic_string(&e.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_ellipse(s: &mut dyn SchSerializer, e: &mut EllipseData) -> Result<()> {
    import_graphical_object(s, &mut e.graphical)?;
    e.location_x = s.import_coord("Location.X")?;
    e.location_y = s.import_coord("Location.Y")?;
    e.radius = s.import_coord("Radius")?;
    e.secondary_radius = s.import_coord("SecondaryRadius")?;
    e.line_width = s.import_size("LineWidth")?;
    e.color = s.import_color("Color")?;
    e.area_color = s.import_color("AreaColor")?;
    e.is_solid = s.import_boolean("IsSolid")?;
    e.transparent = s.import_boolean("Transparent")?;
    e.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Line (ObjectId = 13)
// ============================================================================

pub fn export_line(s: &mut dyn SchSerializer, line: &LineData) -> Result<()> {
    export_graphical_object(s, &line.graphical)?;
    s.export_coord(line.location_x, "Location.X")?;
    s.export_coord(line.location_y, "Location.Y")?;
    s.export_coord(line.corner_x, "Corner.X")?;
    s.export_coord(line.corner_y, "Corner.Y")?;
    s.export_size(line.line_width, "LineWidth")?;
    s.export_line_style(line.line_style, "LineStyle")?;
    s.export_color(line.color, "Color")?;
    s.export_dynamic_string(&line.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_line(s: &mut dyn SchSerializer, line: &mut LineData) -> Result<()> {
    import_graphical_object(s, &mut line.graphical)?;
    line.location_x = s.import_coord("Location.X")?;
    line.location_y = s.import_coord("Location.Y")?;
    line.corner_x = s.import_coord("Corner.X")?;
    line.corner_y = s.import_coord("Corner.Y")?;
    line.line_width = s.import_size("LineWidth")?;
    line.line_style = s.import_line_style("LineStyle")?;
    line.color = s.import_color("Color")?;
    line.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Rectangle (ObjectId = 14)
// ============================================================================

pub fn export_rectangle(s: &mut dyn SchSerializer, rect: &RectangleData) -> Result<()> {
    export_graphical_object(s, &rect.graphical)?;
    s.export_coord(rect.location_x, "Location.X")?;
    s.export_coord(rect.location_y, "Location.Y")?;
    s.export_coord(rect.corner_x, "Corner.X")?;
    s.export_coord(rect.corner_y, "Corner.Y")?;
    s.export_line_style(rect.line_style, "LineStyleExt")?;
    s.export_size(rect.line_width, "LineWidth")?;
    s.export_color(rect.color, "Color")?;
    s.export_color(rect.area_color, "AreaColor")?;
    s.export_boolean(rect.is_solid, "IsSolid")?;
    s.export_boolean(rect.transparent, "Transparent")?;
    s.export_dynamic_string(&rect.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_rectangle(s: &mut dyn SchSerializer, rect: &mut RectangleData) -> Result<()> {
    import_graphical_object(s, &mut rect.graphical)?;
    rect.location_x = s.import_coord("Location.X")?;
    rect.location_y = s.import_coord("Location.Y")?;
    rect.corner_x = s.import_coord("Corner.X")?;
    rect.corner_y = s.import_coord("Corner.Y")?;
    rect.line_style = s.import_line_style("LineStyleExt")?;
    rect.line_width = s.import_size("LineWidth")?;
    rect.color = s.import_color("Color")?;
    rect.area_color = s.import_color("AreaColor")?;
    rect.is_solid = s.import_boolean("IsSolid")?;
    rect.transparent = s.import_boolean("Transparent")?;
    rect.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Bezier (ObjectId = 5)
// ============================================================================

pub fn export_bezier(s: &mut dyn SchSerializer, bez: &BezierData) -> Result<()> {
    export_graphical_object(s, &bez.graphical)?;
    s.export_size(bez.line_width, "LineWidth")?;
    s.export_color(bez.color, "Color")?;
    export_vertices(s, &bez.vertices)?;
    s.export_dynamic_string(&bez.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_bezier(s: &mut dyn SchSerializer, bez: &mut BezierData) -> Result<()> {
    import_graphical_object(s, &mut bez.graphical)?;
    bez.line_width = s.import_size("LineWidth")?;
    bez.color = s.import_color("Color")?;
    bez.vertices = import_vertices(s)?;
    bez.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Polyline (ObjectId = 6)
// ============================================================================

pub fn export_polyline(s: &mut dyn SchSerializer, pl: &PolylineData) -> Result<()> {
    export_graphical_object(s, &pl.graphical)?;
    s.export_size(pl.line_width, "LineWidth")?;
    s.export_line_style(pl.line_style, "LineStyle")?;
    s.export_line_shape(pl.start_line_shape, "StartLineShape")?;
    s.export_line_shape(pl.end_line_shape, "EndLineShape")?;
    s.export_size(pl.line_shape_size, "LineShapeSize")?;
    s.export_color(pl.color, "Color")?;
    export_vertices(s, &pl.vertices)?;
    s.export_dynamic_string(&pl.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_polyline(s: &mut dyn SchSerializer, pl: &mut PolylineData) -> Result<()> {
    import_graphical_object(s, &mut pl.graphical)?;
    pl.line_width = s.import_size("LineWidth")?;
    pl.line_style = s.import_line_style("LineStyle")?;
    pl.start_line_shape = s.import_line_shape("StartLineShape")?;
    pl.end_line_shape = s.import_line_shape("EndLineShape")?;
    pl.line_shape_size = s.import_size("LineShapeSize")?;
    pl.color = s.import_color("Color")?;
    pl.vertices = import_vertices(s)?;
    pl.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Polygon (ObjectId = 7)
// ============================================================================

pub fn export_polygon(s: &mut dyn SchSerializer, poly: &PolygonData) -> Result<()> {
    export_graphical_object(s, &poly.graphical)?;
    s.export_size(poly.line_width, "LineWidth")?;
    s.export_color(poly.color, "Color")?;
    s.export_color(poly.area_color, "AreaColor")?;
    s.export_boolean(poly.is_solid, "IsSolid")?;
    s.export_boolean(poly.transparent, "Transparent")?;
    export_vertices(s, &poly.vertices)?;
    s.export_dynamic_string(&poly.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_polygon(s: &mut dyn SchSerializer, poly: &mut PolygonData) -> Result<()> {
    import_graphical_object(s, &mut poly.graphical)?;
    poly.line_width = s.import_size("LineWidth")?;
    poly.color = s.import_color("Color")?;
    poly.area_color = s.import_color("AreaColor")?;
    poly.is_solid = s.import_boolean("IsSolid")?;
    poly.transparent = s.import_boolean("Transparent")?;
    poly.vertices = import_vertices(s)?;
    poly.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// RoundRectangle
// ============================================================================

pub fn export_round_rectangle(s: &mut dyn SchSerializer, rr: &RoundRectangleData) -> Result<()> {
    export_graphical_object(s, &rr.graphical)?;
    s.export_coord(rr.location_x, "Location.X")?;
    s.export_coord(rr.location_y, "Location.Y")?;
    s.export_coord(rr.corner_x, "Corner.X")?;
    s.export_coord(rr.corner_y, "Corner.Y")?;
    s.export_coord(rr.corner_x_radius, "CornerXRadius")?;
    s.export_coord(rr.corner_y_radius, "CornerYRadius")?;
    s.export_size(rr.line_width, "LineWidth")?;
    s.export_color(rr.color, "Color")?;
    s.export_color(rr.area_color, "AreaColor")?;
    s.export_boolean(rr.is_solid, "IsSolid")?;
    s.export_dynamic_string(&rr.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_round_rectangle(s: &mut dyn SchSerializer, rr: &mut RoundRectangleData) -> Result<()> {
    import_graphical_object(s, &mut rr.graphical)?;
    rr.location_x = s.import_coord("Location.X")?;
    rr.location_y = s.import_coord("Location.Y")?;
    rr.corner_x = s.import_coord("Corner.X")?;
    rr.corner_y = s.import_coord("Corner.Y")?;
    rr.corner_x_radius = s.import_coord("CornerXRadius")?;
    rr.corner_y_radius = s.import_coord("CornerYRadius")?;
    rr.line_width = s.import_size("LineWidth")?;
    rr.color = s.import_color("Color")?;
    rr.area_color = s.import_color("AreaColor")?;
    rr.is_solid = s.import_boolean("IsSolid")?;
    rr.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// EllipticalArc
// ============================================================================

pub fn export_elliptical_arc(s: &mut dyn SchSerializer, ea: &EllipticalArcData) -> Result<()> {
    export_graphical_object(s, &ea.graphical)?;
    s.export_coord(ea.location_x, "Location.X")?;
    s.export_coord(ea.location_y, "Location.Y")?;
    s.export_coord(ea.radius, "Radius")?;
    s.export_coord(ea.secondary_radius, "SecondaryRadius")?;
    s.export_size(ea.line_width, "LineWidth")?;
    s.export_angle(ea.start_angle, "StartAngle")?;
    s.export_angle(ea.end_angle, "EndAngle")?;
    s.export_color(ea.color, "Color")?;
    s.export_dynamic_string(&ea.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_elliptical_arc(s: &mut dyn SchSerializer, ea: &mut EllipticalArcData) -> Result<()> {
    import_graphical_object(s, &mut ea.graphical)?;
    ea.location_x = s.import_coord("Location.X")?;
    ea.location_y = s.import_coord("Location.Y")?;
    ea.radius = s.import_coord("Radius")?;
    ea.secondary_radius = s.import_coord("SecondaryRadius")?;
    ea.line_width = s.import_size("LineWidth")?;
    ea.start_angle = s.import_angle("StartAngle")?;
    ea.end_angle = s.import_angle("EndAngle")?;
    ea.color = s.import_color("Color")?;
    ea.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Pie
// ============================================================================

pub fn export_pie(s: &mut dyn SchSerializer, pie: &PieData) -> Result<()> {
    export_graphical_object(s, &pie.graphical)?;
    s.export_coord(pie.location_x, "Location.X")?;
    s.export_coord(pie.location_y, "Location.Y")?;
    s.export_coord(pie.radius, "Radius")?;
    s.export_size(pie.line_width, "LineWidth")?;
    s.export_angle(pie.start_angle, "StartAngle")?;
    s.export_angle(pie.end_angle, "EndAngle")?;
    s.export_color(pie.color, "Color")?;
    s.export_color(pie.area_color, "AreaColor")?;
    s.export_boolean(pie.is_solid, "IsSolid")?;
    Ok(())
}

pub fn import_pie(s: &mut dyn SchSerializer, pie: &mut PieData) -> Result<()> {
    import_graphical_object(s, &mut pie.graphical)?;
    pie.location_x = s.import_coord("Location.X")?;
    pie.location_y = s.import_coord("Location.Y")?;
    pie.radius = s.import_coord("Radius")?;
    pie.line_width = s.import_size("LineWidth")?;
    pie.start_angle = s.import_angle("StartAngle")?;
    pie.end_angle = s.import_angle("EndAngle")?;
    pie.color = s.import_color("Color")?;
    pie.area_color = s.import_color("AreaColor")?;
    pie.is_solid = s.import_boolean("IsSolid")?;
    Ok(())
}

// ============================================================================
// Image (ObjectId = 30)
// ============================================================================

pub fn export_image(s: &mut dyn SchSerializer, img: &ImageData) -> Result<()> {
    export_graphical_object(s, &img.graphical)?;
    s.export_coord(img.location_x, "Location.X")?;
    s.export_coord(img.location_y, "Location.Y")?;
    s.export_coord(img.corner_x, "Corner.X")?;
    s.export_coord(img.corner_y, "Corner.Y")?;
    s.export_rotation_by90(img.orientation, "Orientation")?;
    s.export_size(img.line_width, "LineWidth")?;
    s.export_color(img.color, "Color")?;
    s.export_boolean(img.is_solid, "IsSolid")?;
    s.export_boolean(img.keep_aspect, "KeepAspect")?;
    s.export_boolean(img.embed_image, "EmbedImage")?;
    s.export_dynamic_string(&img.file_name, "FileName")?;
    s.export_dynamic_string(&img.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_image(s: &mut dyn SchSerializer, img: &mut ImageData) -> Result<()> {
    import_graphical_object(s, &mut img.graphical)?;
    img.location_x = s.import_coord("Location.X")?;
    img.location_y = s.import_coord("Location.Y")?;
    img.corner_x = s.import_coord("Corner.X")?;
    img.corner_y = s.import_coord("Corner.Y")?;
    img.orientation = s.import_rotation_by90("Orientation")?;
    img.line_width = s.import_size("LineWidth")?;
    img.color = s.import_color("Color")?;
    img.is_solid = s.import_boolean("IsSolid")?;
    img.keep_aspect = s.import_boolean("KeepAspect")?;
    img.embed_image = s.import_boolean("EmbedImage")?;
    img.file_name = s.import_dynamic_string("FileName")?;
    img.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

//! PCB polygon (copper pour) record type.
//!
//! Polygons are copper pours that fill areas on PCB layers, typically
//! used for power planes, ground planes, and shielding.

use crate::types::{Coord, Layer, ParameterCollection};

/// Type of polygon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PolygonType {
    /// Solid copper pour polygon.
    #[default]
    Polygon,
    /// Split plane polygon.
    SplitPlane,
    /// Cutout region.
    Cutout,
}

impl PolygonType {
    /// Parse from string value.
    pub fn parse(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "POLYGON" => PolygonType::Polygon,
            "SPLITPLANE" => PolygonType::SplitPlane,
            "CUTOUT" => PolygonType::Cutout,
            _ => PolygonType::Polygon,
        }
    }

    /// Convert to string value.
    pub fn as_str(&self) -> &'static str {
        match self {
            PolygonType::Polygon => "Polygon",
            PolygonType::SplitPlane => "SplitPlane",
            PolygonType::Cutout => "Cutout",
        }
    }
}

/// Hatch style for polygon fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HatchStyle {
    /// No fill (outline only).
    None,
    /// 45-degree hatch lines.
    Hatch45,
    /// 90-degree hatch lines.
    Hatch90,
    /// Horizontal hatch lines.
    HatchHorizontal,
    /// Vertical hatch lines.
    HatchVertical,
    /// Solid fill.
    #[default]
    Solid,
}

impl HatchStyle {
    /// Parse from string value.
    pub fn parse(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "NONE" => HatchStyle::None,
            "45DEGREE" | "HATCH45" => HatchStyle::Hatch45,
            "90DEGREE" | "HATCH90" => HatchStyle::Hatch90,
            "HORIZONTAL" => HatchStyle::HatchHorizontal,
            "VERTICAL" => HatchStyle::HatchVertical,
            "SOLID" => HatchStyle::Solid,
            _ => HatchStyle::Solid,
        }
    }

    /// Convert to string value.
    pub fn as_str(&self) -> &'static str {
        match self {
            HatchStyle::None => "None",
            HatchStyle::Hatch45 => "45Degree",
            HatchStyle::Hatch90 => "90Degree",
            HatchStyle::HatchHorizontal => "Horizontal",
            HatchStyle::HatchVertical => "Vertical",
            HatchStyle::Solid => "Solid",
        }
    }
}

/// Vertex kind in polygon outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PolygonVertexKind {
    /// Straight line segment.
    #[default]
    Line = 0,
    /// Arc segment.
    Arc = 1,
}

impl PolygonVertexKind {
    /// Parse from integer value.
    pub fn from_int(value: i32) -> Self {
        match value {
            1 => PolygonVertexKind::Arc,
            _ => PolygonVertexKind::Line,
        }
    }

    /// Convert to integer value.
    pub fn to_int(self) -> i32 {
        self as i32
    }
}

/// A vertex in the polygon outline.
#[derive(Debug, Clone, Default)]
pub struct PolygonVertex {
    /// Vertex kind (line or arc).
    pub kind: PolygonVertexKind,
    /// X coordinate of the vertex.
    pub x: Coord,
    /// Y coordinate of the vertex.
    pub y: Coord,
    /// Arc center X (for arc vertices).
    pub center_x: Coord,
    /// Arc center Y (for arc vertices).
    pub center_y: Coord,
    /// Arc start angle in degrees.
    pub start_angle: f64,
    /// Arc end angle in degrees.
    pub end_angle: f64,
    /// Arc radius.
    pub radius: Coord,
}

/// PCB polygon (copper pour) record.
///
/// Polygons are copper pours that fill areas on PCB layers. They automatically
/// avoid pads, tracks, and other obstacles while maintaining clearance rules.
#[derive(Debug, Clone, Default)]
pub struct PcbPolygon {
    /// Layer the polygon is on.
    pub layer: Layer,
    /// Whether the polygon is locked.
    pub locked: bool,
    /// Whether this is a polygon outline (not filled).
    pub polygon_outline: bool,
    /// Whether the polygon was user-routed.
    pub user_routed: bool,
    /// Whether this is a keepout region.
    pub keepout: bool,
    /// Union index (for grouping).
    pub union_index: i32,
    /// Whether primitives are locked.
    pub primitive_lock: bool,
    /// Type of polygon (Polygon, SplitPlane, Cutout).
    pub polygon_type: PolygonType,
    /// Whether to pour over all same-net objects.
    pub pour_over: bool,
    /// Whether to remove dead copper islands.
    pub remove_dead: bool,
    /// Grid size for pour.
    pub grid_size: Coord,
    /// Track width for pour.
    pub track_width: Coord,
    /// Hatch style for fill.
    pub hatch_style: HatchStyle,
    /// Whether to use octagons for thermal relief.
    pub use_octagons: bool,
    /// Minimum primitive length.
    pub min_prim_length: Coord,
    /// Net name the polygon is connected to.
    pub net_name: String,
    /// Unique ID.
    pub unique_id: String,
    /// Polygon outline vertices.
    pub vertices: Vec<PolygonVertex>,
    /// All parameters for round-tripping.
    pub params: ParameterCollection,
}

impl PcbPolygon {
    /// Parse a polygon from parameters.
    pub fn from_params(params: &ParameterCollection) -> Self {
        let mut polygon = Self {
            layer: params
                .get("LAYER")
                .map(|v| v.as_layer())
                .unwrap_or_default(),
            locked: params
                .get("LOCKED")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            polygon_outline: params
                .get("POLYGONOUTLINE")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            user_routed: params
                .get("USERROUTED")
                .map(|v| v.as_bool_or(true))
                .unwrap_or(true),
            keepout: params
                .get("KEEPOUT")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            union_index: params
                .get("UNIONINDEX")
                .map(|v| v.as_int_or(0))
                .unwrap_or(0),
            primitive_lock: params
                .get("PRIMITIVELOCK")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            polygon_type: params
                .get("POLYGONTYPE")
                .map(|v| PolygonType::parse(v.as_str()))
                .unwrap_or_default(),
            pour_over: params
                .get("POUROVER")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            remove_dead: params
                .get("REMOVEDEAD")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            grid_size: params
                .get("GRIDSIZE")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            track_width: params
                .get("TRACKWIDTH")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            hatch_style: params
                .get("HATCHSTYLE")
                .map(|v| HatchStyle::parse(v.as_str()))
                .unwrap_or_default(),
            use_octagons: params
                .get("USEOCTAGONS")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            min_prim_length: params
                .get("MINPRIMLENGTH")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            net_name: params
                .get("NET")
                .or_else(|| params.get("NETNAME"))
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            unique_id: params
                .get("UNIQUEID")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            vertices: Vec::new(),
            params: params.clone(),
        };

        // Parse vertices (KIND0, VX0, VY0, CX0, CY0, SA0, EA0, R0, ...)
        let mut idx = 0;
        loop {
            let kind_key = format!("KIND{}", idx);
            let vx_key = format!("VX{}", idx);
            let vy_key = format!("VY{}", idx);

            if !params.contains(&vx_key) {
                break;
            }

            let vertex = PolygonVertex {
                kind: params
                    .get(&kind_key)
                    .map(|v| PolygonVertexKind::from_int(v.as_int_or(0)))
                    .unwrap_or_default(),
                x: params
                    .get(&vx_key)
                    .and_then(|v| v.as_coord().ok())
                    .unwrap_or_default(),
                y: params
                    .get(&vy_key)
                    .and_then(|v| v.as_coord().ok())
                    .unwrap_or_default(),
                center_x: params
                    .get(&format!("CX{}", idx))
                    .and_then(|v| v.as_coord().ok())
                    .unwrap_or_default(),
                center_y: params
                    .get(&format!("CY{}", idx))
                    .and_then(|v| v.as_coord().ok())
                    .unwrap_or_default(),
                start_angle: params
                    .get(&format!("SA{}", idx))
                    .map(|v| v.as_double_or(0.0))
                    .unwrap_or(0.0),
                end_angle: params
                    .get(&format!("EA{}", idx))
                    .map(|v| v.as_double_or(0.0))
                    .unwrap_or(0.0),
                radius: params
                    .get(&format!("R{}", idx))
                    .and_then(|v| v.as_coord().ok())
                    .unwrap_or_default(),
            };

            polygon.vertices.push(vertex);
            idx += 1;
        }

        polygon
    }

    /// Export to parameters.
    pub fn to_params(&self) -> ParameterCollection {
        let mut params = self.params.clone();

        params.add("LAYER", &self.layer.to_string());
        params.add("LOCKED", if self.locked { "TRUE" } else { "FALSE" });
        params.add(
            "POLYGONOUTLINE",
            if self.polygon_outline {
                "TRUE"
            } else {
                "FALSE"
            },
        );
        params.add(
            "USERROUTED",
            if self.user_routed { "TRUE" } else { "FALSE" },
        );
        params.add("KEEPOUT", if self.keepout { "TRUE" } else { "FALSE" });
        params.add_int("UNIONINDEX", self.union_index);
        params.add(
            "PRIMITIVELOCK",
            if self.primitive_lock { "TRUE" } else { "FALSE" },
        );
        params.add("POLYGONTYPE", self.polygon_type.as_str());
        params.add("POUROVER", if self.pour_over { "TRUE" } else { "FALSE" });
        params.add(
            "REMOVEDEAD",
            if self.remove_dead { "TRUE" } else { "FALSE" },
        );
        params.add_coord("GRIDSIZE", self.grid_size);
        params.add_coord("TRACKWIDTH", self.track_width);
        params.add("HATCHSTYLE", self.hatch_style.as_str());
        params.add(
            "USEOCTAGONS",
            if self.use_octagons { "TRUE" } else { "FALSE" },
        );
        params.add_coord("MINPRIMLENGTH", self.min_prim_length);

        if !self.net_name.is_empty() {
            params.add("NET", &self.net_name);
        }
        if !self.unique_id.is_empty() {
            params.add("UNIQUEID", &self.unique_id);
        }

        // Write vertices
        for (idx, vertex) in self.vertices.iter().enumerate() {
            params.add_int(&format!("KIND{}", idx), vertex.kind.to_int());
            params.add_coord(&format!("VX{}", idx), vertex.x);
            params.add_coord(&format!("VY{}", idx), vertex.y);
            params.add_coord(&format!("CX{}", idx), vertex.center_x);
            params.add_coord(&format!("CY{}", idx), vertex.center_y);
            params.add_double(&format!("SA{}", idx), vertex.start_angle, 14);
            params.add_double(&format!("EA{}", idx), vertex.end_angle, 14);
            params.add_coord(&format!("R{}", idx), vertex.radius);
        }

        params
    }

    /// Get the number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
}

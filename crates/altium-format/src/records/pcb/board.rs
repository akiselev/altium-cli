//! PCB board settings and outline record type.
//!
//! The board record contains global PCB settings, grid configuration,
//! and the board outline definition.

use crate::types::{Coord, Layer, ParameterCollection};

use super::polygon::{HatchStyle, PolygonType, PolygonVertex, PolygonVertexKind};

/// Display unit mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum DisplayUnit {
    /// Imperial units (mils).
    #[default]
    Imperial = 0,
    /// Metric units (mm).
    Metric = 1,
}

impl DisplayUnit {
    /// Parse from integer value.
    pub fn from_int(value: i32) -> Self {
        match value {
            1 => DisplayUnit::Metric,
            _ => DisplayUnit::Imperial,
        }
    }

    /// Convert to integer value.
    pub fn to_int(self) -> i32 {
        self as i32
    }
}

/// Designator display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum DesignatorDisplayMode {
    /// Show physical designators.
    #[default]
    Physical = 0,
    /// Show logical designators.
    Logical = 1,
}

impl DesignatorDisplayMode {
    /// Parse from integer value.
    pub fn from_int(value: i32) -> Self {
        match value {
            1 => DesignatorDisplayMode::Logical,
            _ => DesignatorDisplayMode::Physical,
        }
    }

    /// Convert to integer value.
    pub fn to_int(self) -> i32 {
        self as i32
    }
}

/// PCB board settings and outline.
///
/// Contains global board settings including grid configuration,
/// units, and the board outline polygon.
#[derive(Debug, Clone, Default)]
pub struct PcbBoard {
    /// Layer (typically TOP).
    pub layer: Layer,
    /// Whether locked.
    pub locked: bool,
    /// Whether this is a polygon outline only.
    pub polygon_outline: bool,
    /// Source filename.
    pub filename: String,
    /// File format kind (e.g., "Protel_Advanced_PCB").
    pub kind: String,
    /// File version (e.g., "5,01").
    pub version: String,
    /// Creation/modification date.
    pub date: String,
    /// Creation/modification time.
    pub time: String,
    /// Origin X coordinate.
    pub origin_x: Coord,
    /// Origin Y coordinate.
    pub origin_y: Coord,
    /// Big visible grid size.
    pub big_visible_grid_size: f64,
    /// Visible grid size.
    pub visible_grid_size: f64,
    /// Electrical grid range.
    pub electrical_grid_range: Coord,
    /// Whether electrical grid is enabled.
    pub electrical_grid_enabled: bool,
    /// Snap grid size.
    pub snap_grid_size: f64,
    /// Snap grid size X.
    pub snap_grid_size_x: f64,
    /// Snap grid size Y.
    pub snap_grid_size_y: f64,
    /// Track placement grid size.
    pub track_grid_size: f64,
    /// Via placement grid size.
    pub via_grid_size: f64,
    /// Component placement grid size.
    pub component_grid_size: f64,
    /// Component grid size X.
    pub component_grid_size_x: f64,
    /// Component grid size Y.
    pub component_grid_size_y: f64,
    /// Whether to show dot grid.
    pub dot_grid: bool,
    /// Display unit mode (Imperial/Metric).
    pub display_unit: DisplayUnit,
    /// Designator display mode.
    pub designator_display_mode: DesignatorDisplayMode,
    /// Whether primitives are locked.
    pub primitive_lock: bool,
    /// Default polygon type.
    pub polygon_type: PolygonType,
    /// Default pour over setting.
    pub pour_over: bool,
    /// Default remove dead copper setting.
    pub remove_dead: bool,
    /// Default grid size for polygons.
    pub grid_size: Coord,
    /// Default track width.
    pub track_width: Coord,
    /// Default hatch style.
    pub hatch_style: HatchStyle,
    /// Whether to use octagons.
    pub use_octagons: bool,
    /// Minimum primitive length.
    pub min_prim_length: Coord,
    /// Board outline vertices.
    pub outline: Vec<PolygonVertex>,
    /// All parameters for round-tripping.
    pub params: ParameterCollection,
}

impl PcbBoard {
    /// Parse board settings from parameters.
    pub fn from_params(params: &ParameterCollection) -> Self {
        let mut board = Self {
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
            filename: params
                .get("FILENAME")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            kind: params
                .get("KIND")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            version: params
                .get("VERSION")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            date: params
                .get("DATE")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            time: params
                .get("TIME")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            origin_x: params
                .get("ORIGINX")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            origin_y: params
                .get("ORIGINY")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            big_visible_grid_size: params
                .get("BIGVISIBLEGRIDSIZE")
                .map(|v| v.as_double_or(0.0))
                .unwrap_or(0.0),
            visible_grid_size: params
                .get("VISIBLEGRIDSIZE")
                .map(|v| v.as_double_or(0.0))
                .unwrap_or(0.0),
            electrical_grid_range: params
                .get("ELECTRICALGRIDRANGE")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            electrical_grid_enabled: params
                .get("ELECTRICALGRIDENABLED")
                .map(|v| v.as_bool_or(true))
                .unwrap_or(true),
            snap_grid_size: params
                .get("SNAPGRIDSIZE")
                .map(|v| v.as_double_or(0.0))
                .unwrap_or(0.0),
            snap_grid_size_x: params
                .get("SNAPGRIDSIZEX")
                .map(|v| v.as_double_or(0.0))
                .unwrap_or(0.0),
            snap_grid_size_y: params
                .get("SNAPGRIDSIZEY")
                .map(|v| v.as_double_or(0.0))
                .unwrap_or(0.0),
            track_grid_size: params
                .get("TRACKGRIDSIZE")
                .map(|v| v.as_double_or(0.0))
                .unwrap_or(0.0),
            via_grid_size: params
                .get("VIAGRIDSIZE")
                .map(|v| v.as_double_or(0.0))
                .unwrap_or(0.0),
            component_grid_size: params
                .get("COMPONENTGRIDSIZE")
                .map(|v| v.as_double_or(0.0))
                .unwrap_or(0.0),
            component_grid_size_x: params
                .get("COMPONENTGRIDSIZEX")
                .map(|v| v.as_double_or(0.0))
                .unwrap_or(0.0),
            component_grid_size_y: params
                .get("COMPONENTGRIDSIZEY")
                .map(|v| v.as_double_or(0.0))
                .unwrap_or(0.0),
            dot_grid: params
                .get("DOTGRID")
                .map(|v| v.as_bool_or(true))
                .unwrap_or(true),
            display_unit: params
                .get("DISPLAYUNIT")
                .map(|v| DisplayUnit::from_int(v.as_int_or(0)))
                .unwrap_or_default(),
            designator_display_mode: params
                .get("DESIGNATORDISPLAYMODE")
                .map(|v| DesignatorDisplayMode::from_int(v.as_int_or(0)))
                .unwrap_or_default(),
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
            outline: Vec::new(),
            params: params.clone(),
        };

        // Parse board outline vertices (same format as polygon)
        let mut idx = 0;
        loop {
            let vx_key = format!("VX{}", idx);
            let vy_key = format!("VY{}", idx);

            if !params.contains(&vx_key) {
                break;
            }

            let vertex = PolygonVertex {
                kind: params
                    .get(&format!("KIND{}", idx))
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

            board.outline.push(vertex);
            idx += 1;
        }

        board
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
        params.add("FILENAME", &self.filename);
        params.add("KIND", &self.kind);
        params.add("VERSION", &self.version);
        params.add("DATE", &self.date);
        params.add("TIME", &self.time);
        params.add_coord("ORIGINX", self.origin_x);
        params.add_coord("ORIGINY", self.origin_y);
        params.add_double("BIGVISIBLEGRIDSIZE", self.big_visible_grid_size, 3);
        params.add_double("VISIBLEGRIDSIZE", self.visible_grid_size, 3);
        params.add_coord("ELECTRICALGRIDRANGE", self.electrical_grid_range);
        params.add(
            "ELECTRICALGRIDENABLED",
            if self.electrical_grid_enabled {
                "TRUE"
            } else {
                "FALSE"
            },
        );
        params.add_double("SNAPGRIDSIZE", self.snap_grid_size, 6);
        params.add_double("SNAPGRIDSIZEX", self.snap_grid_size_x, 6);
        params.add_double("SNAPGRIDSIZEY", self.snap_grid_size_y, 6);
        params.add_double("TRACKGRIDSIZE", self.track_grid_size, 6);
        params.add_double("VIAGRIDSIZE", self.via_grid_size, 6);
        params.add_double("COMPONENTGRIDSIZE", self.component_grid_size, 6);
        params.add_double("COMPONENTGRIDSIZEX", self.component_grid_size_x, 6);
        params.add_double("COMPONENTGRIDSIZEY", self.component_grid_size_y, 6);
        params.add("DOTGRID", if self.dot_grid { "TRUE" } else { "FALSE" });
        params.add_int("DISPLAYUNIT", self.display_unit.to_int());
        params.add_int(
            "DESIGNATORDISPLAYMODE",
            self.designator_display_mode.to_int(),
        );
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

        // Write outline vertices
        for (idx, vertex) in self.outline.iter().enumerate() {
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

    /// Get the number of outline vertices.
    pub fn outline_vertex_count(&self) -> usize {
        self.outline.len()
    }

    /// Check if using metric units.
    pub fn is_metric(&self) -> bool {
        matches!(self.display_unit, DisplayUnit::Metric)
    }
}

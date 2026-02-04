//! Schematic record types.
//!
//! **DEPRECATED**: V1 record types are being replaced by v2 field structs.
//! V1 suffers from coordinate scale bugs and field type mismatches. Migration map:
//! - `SchPin` -> `v2::fields::PinData`
//! - `SchComponent` -> `v2::fields::ComponentData`
//! - `SchLabel` -> `v2::fields::LabelData`
//! - `SchWire` -> `v2::fields::WireData`
//! - `SchPrimitive` trait -> `v2::fields` typed structs directly
//! - `SchRecord` enum -> `v2::fields` typed structs directly
//!
//! Contains all schematic primitive record types like pins, wires, rectangles, etc.

#![allow(deprecated)]

mod arc;
mod bezier;
mod bus;
mod bus_entry;
mod common;
mod component;
mod designator;
mod ellipse;
mod image;
mod implementation;
mod junction;
mod label;
mod line;
mod netlabel;
mod no_erc;
mod parameter;
mod pie;
mod pin;
mod pin_new; // Test derive macro implementation
mod polygon;
mod polyline;
mod port;
mod power;
mod primitive;
mod rectangle;
mod sheet;
mod symbol;
mod text_frame;
mod warning_sign;
mod wire;

pub use arc::{SchArc, SchEllipticalArc};
pub use bezier::*;
pub use bus::*;
pub use bus_entry::*;
pub use common::*;
pub use component::*;
pub use designator::*;
pub use ellipse::*;
pub use image::*;
pub use implementation::*;
pub use junction::*;
pub use label::*;
pub use line::*;
pub use netlabel::*;
pub use no_erc::*;
pub use parameter::*;
pub use pie::*;
pub use pin::*;
pub use polygon::*;
pub use polyline::*;
pub use port::*;
pub use power::*;
pub use primitive::*;
pub use rectangle::*;
pub use sheet::*;
pub use symbol::*;
pub use text_frame::*;
pub use warning_sign::*;
pub use wire::*;

#[cfg(test)]
mod tests;

// DumpTree implementations
use crate::dump::{DumpTree, TreeBuilder, fmt_angle, fmt_bool, fmt_coord, fmt_point};

impl DumpTree for SchComponent {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![("lib_reference", self.lib_reference.clone())];
        if !self.component_description.is_empty() {
            props.push(("description", self.component_description.clone()));
        }
        props.push((
            "location",
            fmt_point(self.graphical.location_x, self.graphical.location_y),
        ));
        props.push(("parts", format!("{}", self.part_count)));
        tree.add_leaf("Component", &props);
    }
}

impl DumpTree for SchPin {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![];
        if !self.designator.is_empty() {
            props.push(("designator", self.designator.clone()));
        }
        if !self.name.is_empty() {
            props.push(("name", self.name.clone()));
        }
        props.push((
            "location",
            fmt_point(self.graphical.location_x, self.graphical.location_y),
        ));
        props.push(("length", fmt_coord(self.pin_length)));
        props.push(("electrical", format!("{:?}", self.electrical)));
        if self.is_hidden() {
            props.push(("hidden", "yes".to_string()));
        }
        tree.add_leaf("Pin", &props);
    }
}

impl DumpTree for SchSymbol {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![(
            "location",
            fmt_point(self.graphical.location_x, self.graphical.location_y),
        )];
        if self.scale_factor != 1.0 {
            props.push(("scale", format!("{:.2}", self.scale_factor)));
        }
        if self.is_mirrored {
            props.push(("mirrored", "yes".to_string()));
        }
        tree.add_leaf("Symbol", &props);
    }
}

impl DumpTree for SchLabel {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![
            ("text", format!("\"{}\"", self.text)),
            (
                "location",
                fmt_point(self.graphical.location_x, self.graphical.location_y),
            ),
        ];
        if self.is_hidden {
            props.push(("hidden", "yes".to_string()));
        }
        tree.add_leaf("Label", &props);
    }
}

impl DumpTree for SchBezier {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "Bezier",
            &[("control_points", format!("{}", self.vertices.len()))],
        );
    }
}

impl DumpTree for SchPolyline {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "Polyline",
            &[
                ("vertices", format!("{}", self.vertices.len())),
                ("style", format!("{:?}", self.line_style)),
            ],
        );
    }
}

impl DumpTree for SchPolygon {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![("vertices", format!("{}", self.vertices.len()))];
        if self.is_solid {
            props.push(("filled", "yes".to_string()));
        }
        tree.add_leaf("Polygon", &props);
    }
}

impl DumpTree for SchEllipse {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![
            (
                "center",
                fmt_point(self.graphical.location_x, self.graphical.location_y),
            ),
            (
                "radius",
                format!(
                    "{} × {}",
                    fmt_coord(self.radius_x),
                    fmt_coord(self.radius_y)
                ),
            ),
        ];
        if self.is_solid {
            props.push(("filled", "yes".to_string()));
        }
        tree.add_leaf("Ellipse", &props);
    }
}

impl DumpTree for SchPie {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![
            (
                "center",
                fmt_point(self.graphical.location_x, self.graphical.location_y),
            ),
            (
                "radii",
                format!(
                    "{} × {}",
                    fmt_coord(self.radius),
                    fmt_coord(if self.secondary_radius == 0 {
                        self.radius
                    } else {
                        self.secondary_radius
                    })
                ),
            ),
            (
                "angles",
                format!(
                    "{} → {}",
                    fmt_angle(self.start_angle),
                    fmt_angle(self.end_angle)
                ),
            ),
        ];
        if self.is_solid {
            props.push(("filled", "yes".to_string()));
        }
        tree.add_leaf("Pie", &props);
    }
}

impl DumpTree for SchEllipticalArc {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "EllipticalArc",
            &[
                (
                    "center",
                    fmt_point(self.graphical.location_x, self.graphical.location_y),
                ),
                (
                    "radii",
                    format!(
                        "{} × {}",
                        fmt_coord(self.radius),
                        fmt_coord(self.secondary_radius)
                    ),
                ),
                (
                    "angles",
                    format!(
                        "{} → {}",
                        fmt_angle(self.start_angle),
                        fmt_angle(self.end_angle)
                    ),
                ),
            ],
        );
    }
}

impl DumpTree for SchArc {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "Arc",
            &[
                (
                    "center",
                    fmt_point(self.graphical.location_x, self.graphical.location_y),
                ),
                ("radius", fmt_coord(self.radius)),
                (
                    "angles",
                    format!(
                        "{} → {}",
                        fmt_angle(self.start_angle),
                        fmt_angle(self.end_angle)
                    ),
                ),
            ],
        );
    }
}

impl DumpTree for SchLine {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "Line",
            &[
                (
                    "start",
                    fmt_point(self.graphical.location_x, self.graphical.location_y),
                ),
                ("end", fmt_point(self.corner_x, self.corner_y)),
            ],
        );
    }
}

impl DumpTree for SchRectangle {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![
            (
                "corner1",
                fmt_point(self.graphical.location_x, self.graphical.location_y),
            ),
            ("corner2", fmt_point(self.corner_x, self.corner_y)),
        ];
        if self.is_solid {
            props.push(("filled", "yes".to_string()));
        }
        tree.add_leaf("Rectangle", &props);
    }
}

impl DumpTree for SchPowerObject {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "PowerObject",
            &[
                ("net", format!("\"{}\"", self.text)),
                ("style", format!("{:?}", self.style)),
                (
                    "location",
                    fmt_point(self.graphical.location_x, self.graphical.location_y),
                ),
            ],
        );
    }
}

impl DumpTree for SchPort {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![
            ("name", format!("\"{}\"", self.name)),
            (
                "location",
                fmt_point(self.graphical.location_x, self.graphical.location_y),
            ),
        ];
        props.push(("io_type", format!("{:?}", self.io_type)));
        props.push(("style", format!("{:?}", self.style)));
        if self.width > 0 || self.height > 0 {
            props.push(("size", format!("{}x{}", self.width, self.height)));
        }
        if !self.harness_type.is_empty() {
            props.push(("harness", self.harness_type.clone()));
        }
        tree.add_leaf("Port", &props);
    }
}

impl DumpTree for SchNetLabel {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "NetLabel",
            &[
                ("net", format!("\"{}\"", self.label.text)),
                (
                    "location",
                    fmt_point(
                        self.label.graphical.location_x,
                        self.label.graphical.location_y,
                    ),
                ),
            ],
        );
    }
}

impl DumpTree for SchWire {
    fn dump(&self, tree: &mut TreeBuilder) {
        let vertices = &self.vertices;
        if vertices.len() == 2 {
            tree.add_leaf(
                "Wire",
                &[
                    ("start", fmt_point(vertices[0].0, vertices[0].1)),
                    ("end", fmt_point(vertices[1].0, vertices[1].1)),
                ],
            );
        } else {
            tree.add_leaf(
                "Wire",
                &[("segments", format!("{}", vertices.len().saturating_sub(1)))],
            );
        }
    }
}

impl DumpTree for SchTextFrame {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "TextFrame",
            &[
                ("text", format!("\"{}\"", self.text)),
                (
                    "corner1",
                    fmt_point(self.graphical.location_x, self.graphical.location_y),
                ),
                ("corner2", fmt_point(self.corner_x, self.corner_y)),
            ],
        );
    }
}

impl DumpTree for SchTextFrameVariant {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "TextFrameVariant",
            &[
                ("text", format!("\"{}\"", self.text)),
                (
                    "corner1",
                    fmt_point(self.graphical.location_x, self.graphical.location_y),
                ),
                ("corner2", fmt_point(self.corner_x, self.corner_y)),
            ],
        );
    }
}

impl DumpTree for SchJunction {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "Junction",
            &[(
                "location",
                fmt_point(self.graphical.location_x, self.graphical.location_y),
            )],
        );
    }
}

impl DumpTree for SchImage {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![
            (
                "corner1",
                fmt_point(self.graphical.location_x, self.graphical.location_y),
            ),
            ("corner2", fmt_point(self.corner_x, self.corner_y)),
        ];
        if !self.filename.is_empty() {
            props.push(("file", self.filename.clone()));
        }
        props.push(("embedded", fmt_bool(self.embed_image)));
        tree.add_leaf("Image", &props);
    }
}

impl DumpTree for SchSheetHeader {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "SheetHeader",
            &[
                ("fonts", format!("{}", self.font_id_count)),
                ("sheet_size", format!("{}", self.sheet_size)),
            ],
        );
    }
}

impl DumpTree for SchParameter {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "Parameter",
            &[
                ("name", self.name.clone()),
                ("value", format!("\"{}\"", self.label.text)),
                (
                    "location",
                    fmt_point(
                        self.label.graphical.location_x,
                        self.label.graphical.location_y,
                    ),
                ),
            ],
        );
    }
}

impl DumpTree for SchWarningSign {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![(
            "location",
            fmt_point(self.graphical.location_x, self.graphical.location_y),
        )];
        if !self.name.is_empty() {
            props.push(("name", self.name.clone()));
        }
        tree.add_leaf("WarningSign", &props);
    }
}

impl DumpTree for SchDesignator {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "Designator",
            &[
                ("name", self.param.name.clone()),
                ("value", format!("\"{}\"", self.param.label.text)),
                (
                    "location",
                    fmt_point(
                        self.param.label.graphical.location_x,
                        self.param.label.graphical.location_y,
                    ),
                ),
            ],
        );
    }
}

impl DumpTree for SchImplementationList {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "ImplementationList",
            &[("owner_index", format!("{}", self.base.owner_index))],
        );
    }
}

impl DumpTree for SchImplementation {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![
            ("model_name", self.model_name.clone()),
            ("model_type", self.model_type.clone()),
        ];
        if self.is_current {
            props.push(("current", "yes".to_string()));
        }
        tree.add_leaf("Implementation", &props);
    }
}

impl DumpTree for SchMapDefinerList {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "MapDefinerList",
            &[("owner_index", format!("{}", self.base.owner_index))],
        );
    }
}

impl DumpTree for SchMapDefiner {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "MapDefiner",
            &[
                ("interface", self.designator_interface.clone()),
                (
                    "impl_count",
                    format!("{}", self.designator_implementation.len()),
                ),
            ],
        );
    }
}

impl DumpTree for SchImplementationParameters {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "ImplementationParameters",
            &[("owner_index", format!("{}", self.base.owner_index))],
        );
    }
}

impl DumpTree for SchBus {
    fn dump(&self, tree: &mut TreeBuilder) {
        let vertices = &self.vertices;
        if vertices.len() == 2 {
            tree.add_leaf(
                "Bus",
                &[
                    ("start", fmt_point(vertices[0].0, vertices[0].1)),
                    ("end", fmt_point(vertices[1].0, vertices[1].1)),
                ],
            );
        } else {
            tree.add_leaf(
                "Bus",
                &[("segments", format!("{}", vertices.len().saturating_sub(1)))],
            );
        }
    }
}

impl DumpTree for SchBusEntry {
    fn dump(&self, tree: &mut TreeBuilder) {
        let (bus_x, bus_y) = self.bus_point();
        let (wire_x, wire_y) = self.wire_point();
        tree.add_leaf(
            "BusEntry",
            &[
                ("bus_point", fmt_point(bus_x, bus_y)),
                ("wire_point", fmt_point(wire_x, wire_y)),
            ],
        );
    }
}

impl DumpTree for SchNoErc {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![(
            "location",
            fmt_point(self.graphical.location_x, self.graphical.location_y),
        )];
        if self.is_active {
            props.push(("active", "yes".to_string()));
        }
        tree.add_leaf("NoERC", &props);
    }
}

impl DumpTree for SchRecord {
    fn dump(&self, tree: &mut TreeBuilder) {
        match self {
            SchRecord::Component(r) => r.dump(tree),
            SchRecord::Pin(r) => r.dump(tree),
            SchRecord::Symbol(r) => r.dump(tree),
            SchRecord::Label(r) => r.dump(tree),
            SchRecord::Bezier(r) => r.dump(tree),
            SchRecord::Polyline(r) => r.dump(tree),
            SchRecord::Polygon(r) => r.dump(tree),
            SchRecord::Ellipse(r) => r.dump(tree),
            SchRecord::Pie(r) => r.dump(tree),
            SchRecord::EllipticalArc(r) => r.dump(tree),
            SchRecord::Arc(r) => r.dump(tree),
            SchRecord::Line(r) => r.dump(tree),
            SchRecord::Rectangle(r) => r.dump(tree),
            SchRecord::PowerObject(r) => r.dump(tree),
            SchRecord::Port(r) => r.dump(tree),
            SchRecord::NoErc(r) => r.dump(tree),
            SchRecord::NetLabel(r) => r.dump(tree),
            SchRecord::Bus(r) => r.dump(tree),
            SchRecord::Wire(r) => r.dump(tree),
            SchRecord::TextFrame(r) => r.dump(tree),
            SchRecord::TextFrameVariant(r) => r.dump(tree),
            SchRecord::Junction(r) => r.dump(tree),
            SchRecord::Image(r) => r.dump(tree),
            SchRecord::SheetHeader(r) => r.dump(tree),
            SchRecord::Designator(r) => r.dump(tree),
            SchRecord::BusEntry(r) => r.dump(tree),
            SchRecord::Parameter(r) => r.dump(tree),
            SchRecord::WarningSign(r) => r.dump(tree),
            SchRecord::ImplementationList(r) => r.dump(tree),
            SchRecord::Implementation(r) => r.dump(tree),
            SchRecord::MapDefinerList(r) => r.dump(tree),
            SchRecord::MapDefiner(r) => r.dump(tree),
            SchRecord::ImplementationParameters(r) => r.dump(tree),
            SchRecord::Unknown { record_id, params } => {
                tree.add_leaf(
                    "Unknown",
                    &[
                        ("record_id", format!("{}", record_id)),
                        ("params", format!("{} fields", params.len())),
                    ],
                );
            }
        }
    }
}

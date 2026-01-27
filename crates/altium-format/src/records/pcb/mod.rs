//! PCB record types for Altium PCB library files.
//!
//! PCB records use binary format with object IDs.

mod arc;
mod board;
mod class;
mod component;
mod component_body;
mod connection;
mod differential_pair;
mod fill;
mod fromto;
mod net;
mod options;
mod outline;
mod pad;
mod polygon;
mod primitive;
mod region;
mod rule;
mod text;
mod track;
mod via;

pub use arc::*;
pub use board::*;
pub use class::*;
pub use component::*;
pub use component_body::*;
pub use connection::*;
pub use differential_pair::*;
pub use fill::*;
pub use fromto::*;
pub use net::*;
pub use options::*;
pub use pad::*;
pub use polygon::*;
pub use primitive::*;
pub use region::*;
pub use rule::*;
pub use text::*;
pub use track::*;
pub use via::*;

pub(crate) use outline::PcbOutline;

// DumpTree implementations
use crate::dump::{
    DumpTree, TreeBuilder, fmt_angle, fmt_bool, fmt_coord_point, fmt_coord_val, fmt_layer,
};

impl DumpTree for PcbArc {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "Arc",
            &[
                ("layer", fmt_layer(&self.common.layer)),
                ("center", fmt_coord_point(&self.location)),
                ("radius", fmt_coord_val(&self.radius)),
                (
                    "angles",
                    format!(
                        "{} → {}",
                        fmt_angle(self.start_angle),
                        fmt_angle(self.end_angle)
                    ),
                ),
                ("width", fmt_coord_val(&self.width)),
            ],
        );
    }
}

impl DumpTree for PcbTrack {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "Track",
            &[
                ("layer", fmt_layer(&self.common.layer)),
                ("start", fmt_coord_point(&self.start)),
                ("end", fmt_coord_point(&self.end)),
                ("width", fmt_coord_val(&self.width)),
            ],
        );
    }
}

impl DumpTree for PcbPad {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![
            ("designator", self.designator.clone()),
            ("layer", fmt_layer(&self.common.layer)),
            ("location", fmt_coord_point(&self.location)),
            ("size (top)", fmt_coord_point(&self.size_top())),
            ("shape (top)", format!("{:?}", self.shape_top())),
            ("hole", fmt_coord_val(&self.hole_size)),
        ];
        if self.rotation != 0.0 {
            props.push(("rotation", fmt_angle(self.rotation)));
        }
        props.push(("plated", fmt_bool(self.is_plated)));
        tree.add_leaf("Pad", &props);
    }
}

impl DumpTree for PcbVia {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "Via",
            &[
                ("location", fmt_coord_point(&self.location)),
                (
                    "layers",
                    format!(
                        "{} → {}",
                        fmt_layer(&self.from_layer),
                        fmt_layer(&self.to_layer)
                    ),
                ),
                ("diameter", fmt_coord_val(&self.diameter())),
                ("hole", fmt_coord_val(&self.hole_size)),
            ],
        );
    }
}

impl DumpTree for PcbText {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![
            ("text", format!("\"{}\"", self.text)),
            ("layer", fmt_layer(&self.base.common.layer)),
            ("position", fmt_coord_point(&self.base.corner1)),
            ("height", fmt_coord_val(&self.height())),
        ];
        if self.base.rotation != 0.0 {
            props.push(("rotation", fmt_angle(self.base.rotation)));
        }
        if self.mirrored {
            props.push(("mirrored", "yes".to_string()));
        }
        if !self.font_name.is_empty() {
            props.push(("font", self.font_name.clone()));
        }
        tree.add_leaf("Text", &props);
    }
}

impl DumpTree for PcbFill {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![
            ("layer", fmt_layer(&self.base.common.layer)),
            ("corner1", fmt_coord_point(&self.base.corner1)),
            ("corner2", fmt_coord_point(&self.base.corner2)),
        ];
        if self.base.rotation != 0.0 {
            props.push(("rotation", fmt_angle(self.base.rotation)));
        }
        tree.add_leaf("Fill", &props);
    }
}

impl DumpTree for PcbRegion {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "Region",
            &[
                ("layer", fmt_layer(&self.common.layer)),
                ("vertices", format!("{} points", self.outline.len())),
            ],
        );
    }
}

impl DumpTree for PcbComponentBody {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![("layer", fmt_layer(&self.common.layer))];
        if !self.unique_id.is_empty() {
            props.push(("unique_id", self.unique_id.clone()));
        }
        if !self.model_id.is_empty() {
            props.push(("3D model", self.model_id.clone()));
        }
        props.push(("height", fmt_coord_val(&self.overall_height)));
        props.push(("vertices", format!("{} points", self.outline.len())));
        tree.add_leaf("ComponentBody", &props);
    }
}

impl DumpTree for PcbRecord {
    fn dump(&self, tree: &mut TreeBuilder) {
        match self {
            PcbRecord::Arc(r) => r.dump(tree),
            PcbRecord::Pad(r) => r.dump(tree),
            PcbRecord::Via(r) => r.dump(tree),
            PcbRecord::Track(r) => r.dump(tree),
            PcbRecord::Text(r) => r.dump(tree),
            PcbRecord::Fill(r) => r.dump(tree),
            PcbRecord::Region(r) => r.dump(tree),
            PcbRecord::ComponentBody(r) => r.dump(tree),
            PcbRecord::Polygon(r) => r.dump(tree),
            PcbRecord::Unknown {
                object_id,
                raw_data,
            } => {
                tree.add_leaf(
                    "Unknown",
                    &[
                        ("object_id", format!("{:?}", object_id)),
                        ("size", format!("{} bytes", raw_data.len())),
                    ],
                );
            }
        }
    }
}

impl DumpTree for PcbComponent {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.begin_node(&format!("Footprint: {}", self.pattern));
        tree.push(true);

        // Metadata section
        tree.push(!self.primitives.is_empty());
        let mut meta_props = vec![];
        if !self.description.is_empty() {
            meta_props.push(("description", self.description.clone()));
        }
        if self.height.to_raw() != 0 {
            meta_props.push(("height", fmt_coord_val(&self.height)));
        }
        meta_props.push(("pads", format!("{}", self.pad_count())));
        meta_props.push(("primitives", format!("{}", self.primitive_count())));
        tree.add_leaf("Info", &meta_props);
        tree.pop();

        // Primitives section
        if !self.primitives.is_empty() {
            tree.push(false);
            tree.begin_node(&format!("Primitives ({})", self.primitives.len()));
            for (i, prim) in self.primitives.iter().enumerate() {
                tree.push(i < self.primitives.len() - 1);
                prim.dump(tree);
                tree.pop();
            }
            tree.pop();
        }

        tree.pop();
    }
}

impl DumpTree for PcbRule {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![
            ("kind", format!("{}", self.kind)),
            ("enabled", fmt_bool(self.enabled)),
            ("priority", format!("{}", self.priority)),
        ];

        if !self.scope1_expression.is_empty() && self.scope1_expression != "All" {
            props.push(("scope1", self.scope1_expression.clone()));
        }
        if !self.scope2_expression.is_empty() && self.scope2_expression != "All" {
            props.push(("scope2", self.scope2_expression.clone()));
        }
        if !self.comment.is_empty() {
            props.push(("comment", self.comment.clone()));
        }

        tree.add_leaf(&format!("Rule: {}", self.name), &props);
    }
}

impl DumpTree for PcbPolygon {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![
            ("layer", fmt_layer(&self.layer)),
            ("type", self.polygon_type.as_str().to_string()),
            ("vertices", format!("{} points", self.vertices.len())),
        ];
        if !self.net_name.is_empty() {
            props.push(("net", self.net_name.clone()));
        }
        props.push(("hatch", self.hatch_style.as_str().to_string()));
        tree.add_leaf("Polygon", &props);
    }
}

impl DumpTree for PcbNet {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![("name", self.name.clone())];
        if !self.layer_widths.is_empty() {
            props.push((
                "layer widths",
                format!("{} layers", self.layer_widths.len()),
            ));
        }
        tree.add_leaf("Net", &props);
    }
}

impl DumpTree for PcbDifferentialPair {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "DifferentialPair",
            &[
                ("name", self.name.clone()),
                ("positive", self.positive_net_name.clone()),
                ("negative", self.negative_net_name.clone()),
            ],
        );
    }
}

impl DumpTree for PcbFromTo {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "FromTo",
            &[
                ("from", self.from.clone()),
                ("to", self.to.clone()),
                ("net", self.net.clone()),
            ],
        );
    }
}

impl DumpTree for PcbConnection {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.add_leaf(
            "Connection",
            &[
                ("net_index", format!("{}", self.net_index)),
                (
                    "from",
                    format!(
                        "({}, {})",
                        fmt_coord_val(&self.from_x),
                        fmt_coord_val(&self.from_y)
                    ),
                ),
                (
                    "to",
                    format!(
                        "({}, {})",
                        fmt_coord_val(&self.to_x),
                        fmt_coord_val(&self.to_y)
                    ),
                ),
            ],
        );
    }
}

impl DumpTree for PcbBoard {
    fn dump(&self, tree: &mut TreeBuilder) {
        let mut props = vec![
            ("version", self.version.clone()),
            (
                "display_unit",
                if self.is_metric() { "mm" } else { "mil" }.to_string(),
            ),
            ("outline", format!("{} vertices", self.outline.len())),
        ];
        if !self.date.is_empty() {
            props.push(("date", self.date.clone()));
        }
        tree.add_leaf("Board", &props);
    }
}

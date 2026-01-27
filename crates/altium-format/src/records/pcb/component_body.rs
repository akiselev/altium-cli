//! PCB ComponentBody record.

use std::io::Read;

use altium_derive::AltiumRecord;

use super::{PcbOutline, primitive::PcbPrimitiveCommon};
use crate::error::Result;
use crate::traits::FromBinary;
use crate::types::{Coord, CoordPoint, CoordRect, ParameterCollection};

/// PCB Component Body primitive (3D model reference).
#[derive(Debug, Clone, Default)]
pub struct PcbComponentBody {
    /// Common primitive fields.
    pub common: PcbPrimitiveCommon,
    /// Outline vertices.
    pub outline: Vec<CoordPoint>,
    /// V7 layer name.
    pub v7_layer: String,
    /// Body name.
    pub name: String,
    /// Kind.
    pub kind: i32,
    /// Sub-polygon index.
    pub sub_poly_index: i32,
    /// Union index.
    pub union_index: i32,
    /// Arc resolution.
    pub arc_resolution: Coord,
    /// Whether shape-based.
    pub is_shape_based: bool,
    /// Stand-off height.
    pub standoff_height: Coord,
    /// Overall height.
    pub overall_height: Coord,
    /// Body projection type.
    pub body_projection: i32,
    /// 3D body color (COLORREF).
    pub body_color_3d: i32,
    /// 3D body opacity (0.0 - 1.0).
    pub body_opacity_3d: f64,
    /// Unique identifier for the component body.
    pub unique_id: String,
    /// Texture name.
    pub texture: String,
    /// Texture center.
    pub texture_center: CoordPoint,
    /// Texture size.
    pub texture_size: CoordPoint,
    /// Texture rotation.
    pub texture_rotation: f64,
    /// Model ID (links to STEP model).
    pub model_id: String,
    /// Model checksum.
    pub model_checksum: i32,
    /// Whether model is embedded.
    pub model_embed: bool,
    /// 2D model location.
    pub model_2d_location: CoordPoint,
    /// 2D model rotation.
    pub model_2d_rotation: f64,
    /// 3D model rotation X.
    pub model_3d_rot_x: f64,
    /// 3D model rotation Y.
    pub model_3d_rot_y: f64,
    /// 3D model rotation Z.
    pub model_3d_rot_z: f64,
    /// 3D model Z offset.
    pub model_3d_dz: Coord,
    /// Model snap count.
    pub model_snap_count: i32,
    /// Model type.
    pub model_type: i32,
    /// STEP model data (if loaded).
    pub step_model: Option<String>,
}

#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(format = "binary")]
struct PcbComponentBodyBinary {
    #[altium(flatten)]
    common: PcbPrimitiveCommon,
    _unknown1: u32,
    _unknown2: u8,
    parameters: ParameterCollection,
    outline: PcbOutline,
}

impl FromBinary for PcbComponentBody {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let raw = <PcbComponentBodyBinary as FromBinary>::read_from(reader)?;
        let mut body = PcbComponentBody {
            common: raw.common,
            outline: raw.outline.into(),
            ..Default::default()
        };
        body.import_from_parameters(&raw.parameters);
        Ok(body)
    }
}

impl PcbComponentBody {
    /// Import fields from parameters.
    fn import_from_parameters(&mut self, p: &ParameterCollection) {
        self.v7_layer = p
            .get("V7_LAYER")
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        self.name = p
            .get("NAME")
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        self.kind = p.get("KIND").map(|v| v.as_int_or(0)).unwrap_or(0);
        self.sub_poly_index = p.get("SUBPOLYINDEX").map(|v| v.as_int_or(-1)).unwrap_or(-1);
        self.union_index = p.get("UNIONINDEX").map(|v| v.as_int_or(0)).unwrap_or(0);
        self.arc_resolution =
            Coord::from_raw(p.get("ARCRESOLUTION").map(|v| v.as_int_or(0)).unwrap_or(0));
        self.is_shape_based = p
            .get("ISSHAPEBASED")
            .map(|v| v.as_bool_or(false))
            .unwrap_or(false);
        self.standoff_height =
            Coord::from_raw(p.get("STANDOFFHEIGHT").map(|v| v.as_int_or(0)).unwrap_or(0));
        self.overall_height =
            Coord::from_raw(p.get("OVERALLHEIGHT").map(|v| v.as_int_or(0)).unwrap_or(0));
        self.body_projection = p.get("BODYPROJECTION").map(|v| v.as_int_or(0)).unwrap_or(0);
        self.body_color_3d = p
            .get("BODYCOLOR3D")
            .map(|v| v.as_int_or(14737632))
            .unwrap_or(14737632);
        self.body_opacity_3d = p
            .get("BODYOPACITY3D")
            .map(|v| v.as_double_or(1.0))
            .unwrap_or(1.0);

        if let Some(id_str) = p.get("IDENTIFIER") {
            let chars: Vec<char> = id_str
                .as_str()
                .split(',')
                .filter_map(|s| s.parse::<u32>().ok())
                .filter_map(char::from_u32)
                .collect();
            self.unique_id = chars.into_iter().collect();
        }

        self.texture = p
            .get("TEXTURE")
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        self.texture_center = CoordPoint::from_raw(
            p.get("TEXTURECENTERX").map(|v| v.as_int_or(0)).unwrap_or(0),
            p.get("TEXTURECENTERY").map(|v| v.as_int_or(0)).unwrap_or(0),
        );
        self.texture_size = CoordPoint::from_raw(
            p.get("TEXTURESIZEX").map(|v| v.as_int_or(0)).unwrap_or(0),
            p.get("TEXTURESIZEY").map(|v| v.as_int_or(0)).unwrap_or(0),
        );
        self.texture_rotation = p
            .get("TEXTUREROTATION")
            .map(|v| v.as_double_or(0.0))
            .unwrap_or(0.0);
        self.model_id = p
            .get("MODELID")
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        self.model_checksum = p.get("MODEL.CHECKSUM").map(|v| v.as_int_or(0)).unwrap_or(0);
        self.model_embed = p
            .get("MODEL.EMBED")
            .map(|v| v.as_bool_or(true))
            .unwrap_or(true);
        self.model_2d_location = CoordPoint::from_raw(
            p.get("MODEL.2D.X").map(|v| v.as_int_or(0)).unwrap_or(0),
            p.get("MODEL.2D.Y").map(|v| v.as_int_or(0)).unwrap_or(0),
        );
        self.model_2d_rotation = p
            .get("MODEL.2D.ROTATION")
            .map(|v| v.as_double_or(0.0))
            .unwrap_or(0.0);
        self.model_3d_rot_x = p
            .get("MODEL.3D.ROTX")
            .map(|v| v.as_double_or(0.0))
            .unwrap_or(0.0);
        self.model_3d_rot_y = p
            .get("MODEL.3D.ROTY")
            .map(|v| v.as_double_or(0.0))
            .unwrap_or(0.0);
        self.model_3d_rot_z = p
            .get("MODEL.3D.ROTZ")
            .map(|v| v.as_double_or(0.0))
            .unwrap_or(0.0);
        self.model_3d_dz =
            Coord::from_raw(p.get("MODEL.3D.DZ").map(|v| v.as_int_or(0)).unwrap_or(0));
        self.model_snap_count = p
            .get("MODEL.SNAPCOUNT")
            .map(|v| v.as_int_or(0))
            .unwrap_or(0);
        self.model_type = p
            .get("MODEL.MODELTYPE")
            .map(|v| v.as_int_or(1))
            .unwrap_or(1);
    }

    /// Calculate the bounding rectangle.
    pub fn calculate_bounds(&self) -> CoordRect {
        if self.outline.is_empty() {
            return CoordRect::default();
        }

        let min_x = self.outline.iter().map(|p| p.x.to_raw()).min().unwrap_or(0);
        let max_x = self.outline.iter().map(|p| p.x.to_raw()).max().unwrap_or(0);
        let min_y = self.outline.iter().map(|p| p.y.to_raw()).min().unwrap_or(0);
        let max_y = self.outline.iter().map(|p| p.y.to_raw()).max().unwrap_or(0);

        CoordRect::from_raw(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// Export fields to parameters.
    fn export_to_parameters(&self) -> ParameterCollection {
        let mut p = ParameterCollection::new();

        if !self.v7_layer.is_empty() {
            p.add("V7_LAYER", &self.v7_layer);
        }
        if !self.name.is_empty() {
            p.add("NAME", &self.name);
        }
        p.add_int("KIND", self.kind);
        p.add_int("SUBPOLYINDEX", self.sub_poly_index);
        p.add_int("UNIONINDEX", self.union_index);
        p.add_int("ARCRESOLUTION", self.arc_resolution.to_raw());
        p.add("ISSHAPEBASED", if self.is_shape_based { "T" } else { "F" });
        p.add_int("STANDOFFHEIGHT", self.standoff_height.to_raw());
        p.add_int("OVERALLHEIGHT", self.overall_height.to_raw());
        p.add_int("BODYPROJECTION", self.body_projection);
        p.add_int("BODYCOLOR3D", self.body_color_3d);
        p.add("BODYOPACITY3D", &format!("{}", self.body_opacity_3d));

        if !self.unique_id.is_empty() {
            let codes: Vec<String> = self
                .unique_id
                .chars()
                .map(|c| (c as u32).to_string())
                .collect();
            p.add("IDENTIFIER", &codes.join(","));
        }

        if !self.texture.is_empty() {
            p.add("TEXTURE", &self.texture);
        }
        p.add_int("TEXTURECENTERX", self.texture_center.x.to_raw());
        p.add_int("TEXTURECENTERY", self.texture_center.y.to_raw());
        p.add_int("TEXTURESIZEX", self.texture_size.x.to_raw());
        p.add_int("TEXTURESIZEY", self.texture_size.y.to_raw());
        p.add("TEXTUREROTATION", &format!("{}", self.texture_rotation));

        if !self.model_id.is_empty() {
            p.add("MODELID", &self.model_id);
        }
        p.add_int("MODEL.CHECKSUM", self.model_checksum);
        p.add("MODEL.EMBED", if self.model_embed { "T" } else { "F" });
        p.add_int("MODEL.2D.X", self.model_2d_location.x.to_raw());
        p.add_int("MODEL.2D.Y", self.model_2d_location.y.to_raw());
        p.add("MODEL.2D.ROTATION", &format!("{}", self.model_2d_rotation));
        p.add("MODEL.3D.ROTX", &format!("{}", self.model_3d_rot_x));
        p.add("MODEL.3D.ROTY", &format!("{}", self.model_3d_rot_y));
        p.add("MODEL.3D.ROTZ", &format!("{}", self.model_3d_rot_z));
        p.add_int("MODEL.3D.DZ", self.model_3d_dz.to_raw());
        p.add_int("MODEL.SNAPCOUNT", self.model_snap_count);
        p.add_int("MODEL.MODELTYPE", self.model_type);

        p
    }
}

use crate::traits::ToBinary;
use std::io::Write;

impl ToBinary for PcbComponentBody {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        // Common primitive fields
        self.common.write_to(writer)?;

        // Unknown fields
        writer.write_u32::<LittleEndian>(0)?; // _unknown1
        writer.write_u8(0)?; // _unknown2

        // Parameters
        let params = self.export_to_parameters();
        params.write_to(writer)?;

        // Outline
        let outline: PcbOutline = self.outline.clone().into();
        outline.write_to(writer)?;

        Ok(())
    }

    fn binary_size(&self) -> usize {
        let outline: PcbOutline = self.outline.clone().into();
        let params = self.export_to_parameters();
        13 + 5 + params.binary_size() + outline.binary_size()
    }
}

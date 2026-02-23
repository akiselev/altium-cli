use altium_format_types::{Color, Coord, CoordPoint, RegionKind};

use crate::binary_io::BinaryReader;
use crate::param_collection::ParameterCollection;
use crate::pcblib::primitives::common::parse_common_header;
use crate::pcblib::PcbComponentBody;
use crate::{AltiumFormatError, Result};

/// Parses a mil-format coordinate string (e.g. "59.0551mil", "0mil", "-3.937mil").
/// Some locales use comma as decimal separator. Returns `Coord::ZERO` if key is absent.
fn parse_mil_param(params: &mut ParameterCollection, key: &str) -> Result<Coord> {
    let raw: Option<String> = params.remove_optional(key)?;
    match raw {
        None => Ok(Coord::ZERO),
        Some(s) => {
            let trimmed = s.strip_suffix("mil").unwrap_or(&s);
            let normalized = trimmed.replace(',', ".");
            let mils: f64 =
                normalized.parse().map_err(|e: std::num::ParseFloatError| {
                    AltiumFormatError::InvalidParamValue {
                        key: key.to_owned(),
                        detail: format!("cannot parse '{}' as mil value: {}", s, e),
                    }
                })?;
            Ok(Coord::from_mils_f64(mils))
        }
    }
}

/// Parses a ComponentBody primitive from its single PcbLib subrecord.
///
/// ComponentBody inherits from Region in the Altium type hierarchy. The binary
/// layout is:
///
///   [13 bytes]  Common header (PcbPrimitiveCommon)
///   [1 byte]    Region kind (u8, always 0 = Copper)
///   [4 bytes]   Inner vertex count (i32 LE, always 0)
///   [4 bytes]   Parameter string length (u32 LE, includes NUL terminator)
///   [N bytes]   Win1252 parameter string (pipe-delimited |KEY=VALUE|)
///   [4 bytes]   Outline vertex count (i32 LE)
///   [V*16 bytes] Outline vertices as f64 (x, y) pairs
///
/// The parameter string carries body properties (STANDOFFHEIGHT, OVERALLHEIGHT,
/// BODYCOLOR3D, etc.) and the 3D model reference (MODELID, MODEL.NAME,
/// MODEL.3D.ROTX/Y/Z, etc.).
pub(crate) fn parse_component_body(data: &[u8]) -> Result<PcbComponentBody> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;

    // ComponentBody inherits Region: region_kind + inner vertex count.
    // In practice these are always 0 — the actual outline uses f64 vertices at the end.
    let _region_kind = RegionKind::try_from(reader.read_u8()?)?;
    let inner_vertex_count = reader.read_i32_le()?;
    if inner_vertex_count != 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "ComponentBody.inner_vertex_count".to_owned(),
            detail: format!("expected 0, got {}", inner_vertex_count),
        });
    }

    // Length-prefixed parameter string (Win1252, includes NUL terminator).
    let param_len = reader.read_u32_le()? as usize;
    let param_bytes = reader.read_bytes(param_len)?;
    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(param_bytes);
    let mut params = ParameterCollection::from_str(&decoded)?;

    // Extract all known parameters.
    let v7_layer = params.remove_optional::<String>("V7_LAYER")?.unwrap_or_default();
    let name = params.remove_optional::<String>("NAME")?.unwrap_or_default();
    let kind = params.remove_optional::<i32>("KIND")?.unwrap_or(0);
    let subpoly_index = params.remove_optional::<i32>("SUBPOLYINDEX")?.unwrap_or(-1);
    let union_index = params.remove_optional::<i32>("UNIONINDEX")?.unwrap_or(0);
    let standoff_height = parse_mil_param(&mut params, "STANDOFFHEIGHT")?;
    let overall_height = parse_mil_param(&mut params, "OVERALLHEIGHT")?;
    let body_projection = params.remove_optional::<i32>("BODYPROJECTION")?.unwrap_or(0);
    let body_color_3d = params.remove_optional::<Color>("BODYCOLOR3D")?.unwrap_or(Color::new(0));
    let body_opacity_3d = params.remove_optional::<f64>("BODYOPACITY3D")?.unwrap_or(1.0);
    let model_guid = params.remove_optional::<String>("MODELID")?.unwrap_or_default();
    let model_checksum = params.remove_optional::<String>("MODEL.CHECKSUM")?.unwrap_or_default();
    let model_embed = params
        .remove_optional::<String>("MODEL.EMBED")?
        .map(|s| s.eq_ignore_ascii_case("TRUE"))
        .unwrap_or(false);
    let model_name = params.remove_optional::<String>("MODEL.NAME")?.unwrap_or_default();
    let model_2d_x = parse_mil_param(&mut params, "MODEL.2D.X")?;
    let model_2d_y = parse_mil_param(&mut params, "MODEL.2D.Y")?;
    let model_2d_rotation = params.remove_optional::<f64>("MODEL.2D.ROTATION")?.unwrap_or(0.0);
    let rotation_x = params.remove_optional::<f64>("MODEL.3D.ROTX")?.unwrap_or(0.0);
    let rotation_y = params.remove_optional::<f64>("MODEL.3D.ROTY")?.unwrap_or(0.0);
    let rotation_z = params.remove_optional::<f64>("MODEL.3D.ROTZ")?.unwrap_or(0.0);
    let model_3d_dz = parse_mil_param(&mut params, "MODEL.3D.DZ")?;
    let model_type = params.remove_optional::<i32>("MODEL.MODELTYPE")?.unwrap_or(0);
    let model_source = params.remove_optional::<String>("MODEL.MODELSOURCE")?.unwrap_or_default();
    params.assert_exhausted()?;

    // Outline vertices: i32 count + f64 (x, y) pairs.
    // These define the 2D body outline polygon. Coordinates are in internal units
    // (10,000 = 1 mil) stored as f64; we convert to Coord by rounding.
    let vertex_count_raw = reader.read_i32_le()?;
    if vertex_count_raw < 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "ComponentBody.outline_vertex_count".to_owned(),
            detail: format!("vertex_count must be >= 0, got {}", vertex_count_raw),
        });
    }
    let vertex_count = vertex_count_raw as usize;
    let bytes_needed = vertex_count * 16; // each (f64, f64) pair = 16 bytes
    if reader.remaining() < bytes_needed {
        return Err(AltiumFormatError::BinaryReadPastEnd {
            offset: reader.position(),
            needed: bytes_needed,
            available: reader.remaining(),
        });
    }
    let mut outline = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        let x = reader.read_f64_le()?;
        let y = reader.read_f64_le()?;
        outline.push(CoordPoint::new(
            Coord::from_internal(x.round() as i32),
            Coord::from_internal(y.round() as i32),
        ));
    }

    reader.assert_exhausted()?;

    Ok(PcbComponentBody {
        common,
        v7_layer,
        name,
        kind,
        subpoly_index,
        union_index,
        standoff_height,
        overall_height,
        body_projection,
        body_color_3d,
        body_opacity_3d,
        model_guid,
        model_checksum,
        model_embed,
        model_name,
        model_2d_x,
        model_2d_y,
        model_2d_rotation,
        rotation_x,
        rotation_y,
        rotation_z,
        model_3d_dz,
        model_type,
        model_source,
        outline,
        unique_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use altium_format_types::Coord;
    use crate::binary_io::BinaryWriter;

    fn write_common_header(w: &mut BinaryWriter) {
        w.write_u8(57); // layer = Mechanical1
        w.write_u8(0x0C); // pad_byte
        w.write_u16_le(0xFF00); // flags
        w.write_i32_le(-1); // net_index
        w.write_u16_le(0xFFFF); // polygon_index
        w.write_u16_le(0xFFFF); // component_index
        w.write_u8(0xFF); // unknown
    }

    fn make_param_string(model_id: &str, standoff_mil: &str) -> Vec<u8> {
        let s = format!(
            "|V7_LAYER=MECHANICAL1|NAME= |KIND=0|SUBPOLYINDEX=-1|UNIONINDEX=0\
             |STANDOFFHEIGHT={standoff_mil}mil\
             |OVERALLHEIGHT=50mil\
             |BODYPROJECTION=0|BODYCOLOR3D=14342874\
             |BODYOPACITY3D=1.000\
             |MODELID={model_id}\
             |MODEL.CHECKSUM=0\
             |MODEL.EMBED=TRUE\
             |MODEL.NAME=test.step\
             |MODEL.2D.X=0mil|MODEL.2D.Y=0mil|MODEL.2D.ROTATION=0\
             |MODEL.3D.ROTX=0|MODEL.3D.ROTY=0|MODEL.3D.ROTZ=90\
             |MODEL.3D.DZ=0mil\
             |MODEL.MODELTYPE=1\
             |MODEL.MODELSOURCE=Undefined|\0"
        );
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(&s);
        encoded.to_vec()
    }

    fn make_component_body_data(
        model_id: &str,
        standoff_mil: &str,
        vertices: &[(f64, f64)],
    ) -> Vec<u8> {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_u8(0); // region_kind = Copper
        w.write_i32_le(0); // inner_vertex_count = 0
        let param_bytes = make_param_string(model_id, standoff_mil);
        w.write_u32_le(param_bytes.len() as u32);
        w.write_bytes(&param_bytes);
        w.write_i32_le(vertices.len() as i32);
        for &(x, y) in vertices {
            w.write_f64_le(x);
            w.write_f64_le(y);
        }
        w.finish()
    }

    #[test]
    fn parse_component_body_basic() {
        let model_id = "{EF6F5E91-7F7C-4522-A44A-A71C4497D723}";
        let vertices = [
            (-300_000.0, -250_000.0),
            (300_000.0, -250_000.0),
            (300_000.0, 250_000.0),
            (-300_000.0, 250_000.0),
        ];
        let data = make_component_body_data(model_id, "10", &vertices);
        let body = parse_component_body(&data).unwrap();
        assert_eq!(body.model_guid, model_id);
        assert_eq!(body.standoff_height, Coord::from_mils_f64(10.0));
        assert_eq!(body.rotation_x, 0.0);
        assert_eq!(body.rotation_y, 0.0);
        assert_eq!(body.rotation_z, 90.0);
        assert_eq!(body.outline.len(), 4);
        assert_eq!(body.outline[0].x.to_internal(), -300_000);
        assert_eq!(body.outline[0].y.to_internal(), -250_000);
        assert_eq!(body.outline[3].x.to_internal(), -300_000);
        assert_eq!(body.outline[3].y.to_internal(), 250_000);
        assert!(body.unique_id.is_none());
    }

    #[test]
    fn parse_component_body_empty_outline() {
        let data = make_component_body_data(
            "{00000000-0000-0000-0000-000000000000}",
            "0",
            &[],
        );
        let body = parse_component_body(&data).unwrap();
        assert!(body.outline.is_empty());
        assert_eq!(body.standoff_height, Coord::ZERO);
    }

    #[test]
    fn parse_component_body_negative_standoff() {
        let data = make_component_body_data(
            "{ABCDEF01-2345-6789-ABCD-EF0123456789}",
            "-3.937",
            &[(0.0, 0.0), (100_000.0, 0.0), (100_000.0, 100_000.0), (0.0, 100_000.0)],
        );
        let body = parse_component_body(&data).unwrap();
        assert_eq!(body.standoff_height, Coord::from_mils_f64(-3.937));
    }

    #[test]
    fn parse_component_body_truncated_returns_error() {
        let data = [0u8; 10]; // way too short
        let result = parse_component_body(&data);
        assert!(result.is_err());
    }

    #[test]
    fn parse_component_body_negative_vertex_count_returns_error() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_u8(0); // region_kind
        w.write_i32_le(0); // inner_vertex_count
        let param_str = b"|\0";
        w.write_u32_le(param_str.len() as u32);
        w.write_bytes(param_str);
        w.write_i32_le(-1); // negative vertex count
        let data = w.finish();
        let result = parse_component_body(&data);
        assert!(matches!(
            result,
            Err(AltiumFormatError::InvalidParamValue { .. })
        ));
    }
}

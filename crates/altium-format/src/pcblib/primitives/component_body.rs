use altium_format_types::{Color, Coord, CoordPoint, PolySegmentKind, RegionKind};

use crate::binary_io::BinaryReader;
use crate::param_collection::ParameterCollection;
use crate::pcblib::primitives::common::parse_common_header;
use crate::pcblib::{Contour, PcbComponentBody, PolySegment};
use crate::{AltiumFormatError, Result};

/// Parses a PolySegmentKind from its u8 string representation.
fn parse_poly_segment_kind(s: &str) -> Result<PolySegmentKind> {
    let raw: u8 = s
        .trim()
        .parse()
        .map_err(|_| AltiumFormatError::InvalidParamValue {
            key: "KIND".to_owned(),
            detail: format!("cannot parse '{}' as u8 PolySegmentKind", s),
        })?;
    PolySegmentKind::try_from(raw).map_err(|e| AltiumFormatError::InvalidParamValue {
        key: "KIND".to_owned(),
        detail: e.to_string(),
    })
}

/// Parses a mil-format coordinate string from a raw string value.
fn parse_mil_param_str(s: &str, key: &str) -> Result<Coord> {
    if s.is_empty() {
        return Ok(Coord::ZERO);
    }
    let trimmed = s.strip_suffix("mil").unwrap_or(s);
    let normalized = trimmed.trim().replace(',', ".");
    let mils: f64 = normalized.parse().map_err(|e: std::num::ParseFloatError| {
        AltiumFormatError::InvalidParamValue {
            key: key.to_owned(),
            detail: format!("cannot parse '{}' as mil value: {}", s, e),
        }
    })?;
    Ok(Coord::from_mils_f64(mils))
}

/// Parses an f64 from a string (handles scientific notation and leading spaces).
fn parse_float_str(s: &str, key: &str) -> Result<f64> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(0.0);
    }
    trimmed
        .parse::<f64>()
        .map_err(|e| AltiumFormatError::InvalidParamValue {
            key: key.to_owned(),
            detail: format!("cannot parse '{}' as f64: {}", s, e),
        })
}

/// Decodes an IDENTIFIER value from comma-separated UTF-16 code units.
///
/// Format: `"67,65,80,67,50,48,49,50"` → `"CAPC2012"`.
/// An empty string input returns an empty string.
fn decode_identifier(raw: &str) -> Result<String> {
    if raw.is_empty() {
        return Ok(String::new());
    }
    let code_units: std::result::Result<Vec<u16>, _> =
        raw.split(',').map(|s| s.trim().parse::<u16>()).collect();
    let code_units = code_units.map_err(|e| AltiumFormatError::InvalidParamValue {
        key: "IDENTIFIER".to_owned(),
        detail: format!("cannot parse comma-separated UTF-16 code units: {}", e),
    })?;
    String::from_utf16(&code_units).map_err(|e| AltiumFormatError::InvalidParamValue {
        key: "IDENTIFIER".to_owned(),
        detail: format!("invalid UTF-16 sequence: {}", e),
    })
}

/// Encodes an identifier string as comma-separated UTF-16 code units.
///
/// Inverse of [`decode_identifier`]. Example: `"CAPC2012"` → `"67,65,80,67,50,48,49,50"`.
pub(crate) fn encode_identifier(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    s.encode_utf16()
        .map(|u| u.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// Formats a float in Altium's scientific notation: `" 0.00000000000000E+0000"`.
///
/// Altium uses a Delphi-style format with 14 decimal digits, 4-digit exponent with
/// explicit sign, and a leading space for non-negative values.
pub(crate) fn format_scientific_float(value: f64) -> String {
    if value == 0.0 {
        return " 0.00000000000000E+0000".to_owned();
    }
    let (mantissa, exponent) = if value == 0.0 {
        (0.0, 0)
    } else {
        let exp = value.abs().log10().floor() as i32;
        let man = value / 10_f64.powi(exp);
        (man, exp)
    };
    let sign_char = if exponent >= 0 { '+' } else { '-' };
    let prefix = if mantissa >= 0.0 { " " } else { "-" };
    format!(
        "{}{:.14}E{}{:04}",
        prefix,
        mantissa.abs(),
        sign_char,
        exponent.unsigned_abs()
    )
}

/// Parses a float parameter that may use scientific notation (e.g. " 0.00000000000000E+0000").
/// Returns `0.0` if key is absent.
fn parse_scientific_float(params: &mut ParameterCollection, key: &str) -> Result<f64> {
    let raw: Option<String> = params.remove_optional(key)?;
    match raw {
        None => Ok(0.0),
        Some(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Ok(0.0);
            }
            trimmed
                .parse::<f64>()
                .map_err(|e| AltiumFormatError::InvalidParamValue {
                    key: key.to_owned(),
                    detail: format!("cannot parse '{}' as float: {}", s, e),
                })
        }
    }
}

/// Parses a mil-format coordinate string (e.g. "59.0551mil", "0mil", "-3.937mil").
/// Some locales use comma as decimal separator. Returns `Coord::ZERO` if key is absent.
fn parse_mil_param(params: &mut ParameterCollection, key: &str) -> Result<Coord> {
    let raw: Option<String> = params.remove_optional(key)?;
    match raw {
        None => Ok(Coord::ZERO),
        Some(s) => {
            let trimmed = s.strip_suffix("mil").unwrap_or(&s);
            let normalized = trimmed.replace(',', ".");
            let mils: f64 = normalized.parse().map_err(|e: std::num::ParseFloatError| {
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

/// Reads a shape-based contour: i32 edge_count + (edge_count + 1) × TPolySegment (37 bytes each).
fn read_polysegment_contour(reader: &mut BinaryReader, label: &str) -> Result<Vec<PolySegment>> {
    let edge_count_raw = reader.read_i32_le()?;
    if edge_count_raw < 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: format!("ComponentBody.{label}_edge_count"),
            detail: format!("edge_count must be >= 0, got {edge_count_raw}"),
        });
    }
    let edge_count = edge_count_raw as usize;
    let vertex_count = edge_count + 1; // closing vertex
    let bytes_needed = vertex_count * 37;
    if reader.remaining() < bytes_needed {
        return Err(AltiumFormatError::BinaryReadPastEnd {
            offset: reader.position(),
            needed: bytes_needed,
            available: reader.remaining(),
        });
    }
    let mut segments = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        let kind_raw = reader.read_u8()?;
        let kind = PolySegmentKind::try_from(kind_raw).map_err(|e| {
            AltiumFormatError::InvalidParamValue {
                key: format!("ComponentBody.{label}_poly_segment_kind"),
                detail: e.to_string(),
            }
        })?;
        let vx = reader.read_i32_le()?;
        let vy = reader.read_i32_le()?;
        let cx = reader.read_i32_le()?;
        let cy = reader.read_i32_le()?;
        let radius = reader.read_i32_le()?;
        let angle1 = reader.read_f64_le()?;
        let angle2 = reader.read_f64_le()?;
        segments.push(PolySegment {
            kind,
            vertex: CoordPoint::new(Coord::from_internal(vx), Coord::from_internal(vy)),
            center: CoordPoint::new(Coord::from_internal(cx), Coord::from_internal(cy)),
            radius: Coord::from_internal(radius),
            angle1,
            angle2,
        });
    }
    Ok(segments)
}

/// Parses a ComponentBody primitive from its single PcbLib subrecord.
///
/// Binary layout (inherits Region):
///   [13 bytes]  Common header (layer, flags, net, polygon, component, coord, dim indices)
///   [1 byte]    Region kind (always 0 = Copper for component bodies)
///   [4 bytes]   Inner vertex count (always 0)
///   [4 bytes]   Param string length
///   [N bytes]   Win1252 parameter string (pipe-delimited |KEY=VALUE|)
///   [4 bytes]   Outline vertex count (i32 LE)
///   [V*16 bytes] Outline vertices as f64 (x, y) pairs (legacy)
///     -or- [V*37 bytes] Outline vertices as TPolySegment (shape-based)
///
/// The parameter string carries body properties (STANDOFFHEIGHT, OVERALLHEIGHT,
/// BODYCOLOR3D, etc.) and the 3D model reference (MODELID, MODEL.NAME,
/// MODEL.3D.ROTX/Y/Z, etc.).
pub(crate) fn parse_component_body(
    data: &[u8],
    is_shape_based_section: bool,
) -> Result<PcbComponentBody> {
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
    // Region-inherited parameters
    let v7_layer = params
        .remove_optional::<String>("V7_LAYER")?
        .unwrap_or_default();
    let name = params
        .remove_optional::<String>("NAME")?
        .unwrap_or_default();
    let kind = params.remove_optional::<i32>("KIND")?.unwrap_or(0);
    let subpoly_index = params.remove_optional::<i32>("SUBPOLYINDEX")?.unwrap_or(-1);
    let union_index = params.remove_optional::<i32>("UNIONINDEX")?.unwrap_or(0);
    let arc_resolution = parse_mil_param(&mut params, "ARCRESOLUTION")?;
    let is_shape_based = params
        .remove_optional::<bool>("ISSHAPEBASED")?
        .unwrap_or(false);
    let cavity_height = parse_mil_param(&mut params, "CAVITYHEIGHT")?;
    // ComponentBody parameters
    let standoff_height = parse_mil_param(&mut params, "STANDOFFHEIGHT")?;
    let overall_height = parse_mil_param(&mut params, "OVERALLHEIGHT")?;
    let body_projection = params
        .remove_optional::<i32>("BODYPROJECTION")?
        .unwrap_or(0);
    let body_color_3d = params
        .remove_optional::<Color>("BODYCOLOR3D")?
        .unwrap_or(Color::new(0));
    let body_opacity_3d = params
        .remove_optional::<f64>("BODYOPACITY3D")?
        .unwrap_or(1.0);
    let identifier_raw = params
        .remove_optional::<String>("IDENTIFIER")?
        .unwrap_or_default();
    let identifier = decode_identifier(&identifier_raw)?;
    let texture = params
        .remove_optional::<String>("TEXTURE")?
        .unwrap_or_default();
    let texture_center_x = parse_mil_param(&mut params, "TEXTURECENTERX")?;
    let texture_center_y = parse_mil_param(&mut params, "TEXTURECENTERY")?;
    let texture_size_x = parse_mil_param(&mut params, "TEXTURESIZEX")?;
    let texture_size_y = parse_mil_param(&mut params, "TEXTURESIZEY")?;
    let texture_rotation = parse_scientific_float(&mut params, "TEXTUREROTATION")?;
    let body_override_color = params
        .remove_optional::<bool>("BODYOVERRIDECOLOR")?
        .unwrap_or(false);
    // 3D model parameters
    let model_guid = params
        .remove_optional::<String>("MODELID")?
        .unwrap_or_default();
    let model_checksum = params
        .remove_optional::<String>("MODEL.CHECKSUM")?
        .unwrap_or_default();
    let model_embed = params
        .remove_optional::<bool>("MODEL.EMBED")?
        .unwrap_or(false);
    let model_name = params
        .remove_optional::<String>("MODEL.NAME")?
        .unwrap_or_default();
    let model_2d_x = parse_mil_param(&mut params, "MODEL.2D.X")?;
    let model_2d_y = parse_mil_param(&mut params, "MODEL.2D.Y")?;
    let model_2d_rotation = params
        .remove_optional::<f64>("MODEL.2D.ROTATION")?
        .unwrap_or(0.0);
    let rotation_x = params
        .remove_optional::<f64>("MODEL.3D.ROTX")?
        .unwrap_or(0.0);
    let rotation_y = params
        .remove_optional::<f64>("MODEL.3D.ROTY")?
        .unwrap_or(0.0);
    let rotation_z = params
        .remove_optional::<f64>("MODEL.3D.ROTZ")?
        .unwrap_or(0.0);
    let model_3d_dz = parse_mil_param(&mut params, "MODEL.3D.DZ")?;
    let model_type = params
        .remove_optional::<i32>("MODEL.MODELTYPE")?
        .unwrap_or(0);
    let model_source = params
        .remove_optional::<String>("MODEL.MODELSOURCE")?
        .unwrap_or_default();
    // Snap points: MODEL.SNAPCOUNT + MODEL.S{n}X/Y/Z (raw i32 internal units)
    let snap_count = params
        .remove_optional::<i32>("MODEL.SNAPCOUNT")?
        .unwrap_or(0);
    let mut model_snap_points = Vec::with_capacity(snap_count.max(0) as usize);
    for i in 0..snap_count.max(0) {
        let sx = params
            .remove_optional::<i32>(&format!("MODEL.S{}X", i))?
            .unwrap_or(0);
        let sy = params
            .remove_optional::<i32>(&format!("MODEL.S{}Y", i))?
            .unwrap_or(0);
        let sz = params
            .remove_optional::<i32>(&format!("MODEL.S{}Z", i))?
            .unwrap_or(0);
        model_snap_points.push((
            Coord::from_internal(sx),
            Coord::from_internal(sy),
            Coord::from_internal(sz),
        ));
    }
    // Extruded body Z bounds (only present for extruded model types)
    let model_extruded_min_z = parse_mil_param(&mut params, "MODEL.EXTRUDED.MINZ")?;
    let model_extruded_max_z = parse_mil_param(&mut params, "MODEL.EXTRUDED.MAXZ")?;
    // Cylinder model parameters (only present for cylinder model types)
    let model_cylinder_radius = parse_mil_param(&mut params, "MODEL.CYLINDER.RADIUS")?;
    let model_cylinder_height = parse_mil_param(&mut params, "MODEL.CYLINDER.HEIGHT")?;
    let model_sphere_radius = parse_mil_param(&mut params, "MODEL.SPHERE.RADIUS")?;

    // Shape-based component bodies include indexed edge geometry in the param string.
    // ComponentBody inherits from Region so it can also have these params.
    // In PcbLib monolithic Data streams, these carry arc data supplementing the
    // binary f64 vertex contour. In PcbDoc ShapeBasedComponentBodies6 sections,
    // these params are absent. We parse them into PolySegment form here and
    // regenerate them during serialization.
    let shape_text_segments: Option<Vec<PolySegment>>;
    if is_shape_based {
        let shape_vertex_count = params
            .remove_optional::<i32>("MAINCONTOURVERTEXCOUNT")?
            .unwrap_or(0);
        if shape_vertex_count > 0 {
            let mut segs = Vec::with_capacity(shape_vertex_count as usize);
            for i in 0..shape_vertex_count {
                let idx = i.to_string();
                let kind_raw: String = params
                    .remove_optional(&format!("KIND{}", idx))?
                    .unwrap_or_default();
                let vx: String = params
                    .remove_optional(&format!("VX{}", idx))?
                    .unwrap_or_default();
                let vy: String = params
                    .remove_optional(&format!("VY{}", idx))?
                    .unwrap_or_default();
                let cx: String = params
                    .remove_optional(&format!("CX{}", idx))?
                    .unwrap_or_default();
                let cy: String = params
                    .remove_optional(&format!("CY{}", idx))?
                    .unwrap_or_default();
                let sa: String = params
                    .remove_optional(&format!("SA{}", idx))?
                    .unwrap_or_default();
                let ea: String = params
                    .remove_optional(&format!("EA{}", idx))?
                    .unwrap_or_default();
                let r: String = params
                    .remove_optional(&format!("R{}", idx))?
                    .unwrap_or_default();
                segs.push(PolySegment {
                    kind: parse_poly_segment_kind(&kind_raw)?,
                    vertex: CoordPoint::new(
                        parse_mil_param_str(&vx, &format!("VX{}", idx))?,
                        parse_mil_param_str(&vy, &format!("VY{}", idx))?,
                    ),
                    center: CoordPoint::new(
                        parse_mil_param_str(&cx, &format!("CX{}", idx))?,
                        parse_mil_param_str(&cy, &format!("CY{}", idx))?,
                    ),
                    angle1: parse_float_str(&sa, &format!("SA{}", idx))?,
                    angle2: parse_float_str(&ea, &format!("EA{}", idx))?,
                    radius: parse_mil_param_str(&r, &format!("R{}", idx))?,
                });
            }
            shape_text_segments = Some(segs);
        } else {
            shape_text_segments = None;
        }
    } else {
        shape_text_segments = None;
    }

    params.assert_exhausted()?;

    // The section kind determines the binary vertex format. ALL records in
    // ShapeBasedComponentBodies6 use TPolySegment format.
    let use_shape_based = is_shape_based_section;

    // Outline vertices.
    let outline = if use_shape_based {
        Contour::ShapeBased(read_polysegment_contour(&mut reader, "outline")?)
    } else {
        // Legacy f64 vertex pairs.
        let vertex_count_raw = reader.read_i32_le()?;
        if vertex_count_raw < 0 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "ComponentBody.outline_vertex_count".to_owned(),
                detail: format!("vertex_count must be >= 0, got {}", vertex_count_raw),
            });
        }
        let vertex_count = vertex_count_raw as usize;
        let bytes_needed = vertex_count * 16;
        if reader.remaining() < bytes_needed {
            return Err(AltiumFormatError::BinaryReadPastEnd {
                offset: reader.position(),
                needed: bytes_needed,
                available: reader.remaining(),
            });
        }
        let mut vertices = Vec::with_capacity(vertex_count);
        for _ in 0..vertex_count {
            let x = reader.read_f64_le()?;
            let y = reader.read_f64_le()?;
            vertices.push(CoordPoint::new(
                Coord::from_internal(x.round() as i32),
                Coord::from_internal(y.round() as i32),
            ));
        }
        Contour::Legacy(vertices)
    };

    reader.assert_exhausted()?;

    Ok(PcbComponentBody {
        common,
        v7_layer,
        name,
        kind,
        subpoly_index,
        union_index,
        arc_resolution,
        is_shape_based,
        cavity_height,
        standoff_height,
        overall_height,
        body_projection,
        body_color_3d,
        body_opacity_3d,
        identifier,
        texture,
        texture_center_x,
        texture_center_y,
        texture_size_x,
        texture_size_y,
        texture_rotation,
        body_override_color,
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
        model_snap_points,
        model_extruded_min_z,
        model_extruded_max_z,
        model_cylinder_radius,
        model_cylinder_height,
        model_sphere_radius,
        outline,
        shape_text_segments,
        unique_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_io::BinaryWriter;
    use altium_format_types::Coord;

    fn write_common_header(w: &mut BinaryWriter) {
        w.write_u8(57); // layer = Mechanical1
        w.write_u16_le(0x000C); // flags
        w.write_u16_le(0xFFFF); // net_index = none
        w.write_u16_le(0xFFFF); // polygon_index = none
        w.write_u16_le(0xFFFF); // component_index = none
        w.write_u16_le(0xFFFF); // coordinate_index = none
        w.write_u16_le(0xFFFF); // dimension_index = none
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
        let body = parse_component_body(&data, false).unwrap();
        assert_eq!(body.model_guid, model_id);
        assert_eq!(body.standoff_height, Coord::from_mils_f64(10.0));
        assert_eq!(body.rotation_x, 0.0);
        assert_eq!(body.rotation_y, 0.0);
        assert_eq!(body.rotation_z, 90.0);
        let pts = match &body.outline {
            Contour::Legacy(pts) => pts,
            _ => panic!("expected Legacy contour"),
        };
        assert_eq!(pts.len(), 4);
        assert_eq!(pts[0].x.to_internal(), -300_000);
        assert_eq!(pts[0].y.to_internal(), -250_000);
        assert_eq!(pts[3].x.to_internal(), -300_000);
        assert_eq!(pts[3].y.to_internal(), 250_000);
        assert!(body.unique_id.is_none());
    }

    #[test]
    fn parse_component_body_empty_outline() {
        let data = make_component_body_data("{00000000-0000-0000-0000-000000000000}", "0", &[]);
        let body = parse_component_body(&data, false).unwrap();
        assert!(matches!(&body.outline, Contour::Legacy(pts) if pts.is_empty()));
        assert_eq!(body.standoff_height, Coord::ZERO);
    }

    #[test]
    fn parse_component_body_negative_standoff() {
        let data = make_component_body_data(
            "{ABCDEF01-2345-6789-ABCD-EF0123456789}",
            "-3.937",
            &[
                (0.0, 0.0),
                (100_000.0, 0.0),
                (100_000.0, 100_000.0),
                (0.0, 100_000.0),
            ],
        );
        let body = parse_component_body(&data, false).unwrap();
        assert_eq!(body.standoff_height, Coord::from_mils_f64(-3.937));
    }

    #[test]
    fn parse_component_body_truncated_returns_error() {
        let data = [0u8; 10]; // way too short
        let result = parse_component_body(&data, false);
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
        let result = parse_component_body(&data, false);
        assert!(matches!(
            result,
            Err(AltiumFormatError::InvalidParamValue { .. })
        ));
    }
}

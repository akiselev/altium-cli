use altium_format_types::{Coord, CoordPoint, PolySegmentKind, RegionKind};

use crate::binary_io::BinaryReader;
use crate::param_collection::ParameterCollection;
use crate::pcblib::{Contour, PolySegment, PcbRegion};
use crate::pcblib::primitives::common::parse_common_header;
use crate::{AltiumFormatError, Result};

/// Parses a mil-format coordinate string (e.g. "0.5mil", "0mil", "-3.937mil").
/// Returns `Coord::ZERO` if key is absent.
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

/// Reads a contour (vertex array) of f64 (x, y) pairs from the binary reader.
///
/// Format: i32 LE vertex count + N * (f64 LE x, f64 LE y) pairs.
/// Coordinates are in internal units (10,000 = 1 mil); we round to Coord.
fn read_f64_contour(reader: &mut BinaryReader, label: &str) -> Result<Vec<CoordPoint>> {
    let vertex_count_raw = reader.read_i32_le()?;
    if vertex_count_raw < 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: format!("Region.{}_vertex_count", label),
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
    let mut vertices = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        let x = reader.read_f64_le()?;
        let y = reader.read_f64_le()?;
        vertices.push(CoordPoint::new(
            Coord::from_internal(x.round() as i32),
            Coord::from_internal(y.round() as i32),
        ));
    }
    Ok(vertices)
}

/// Reads a shape-based contour: i32 edge_count + (edge_count + 1) × TPolySegment (37 bytes each).
fn read_polysegment_contour(reader: &mut BinaryReader, label: &str) -> Result<Vec<PolySegment>> {
    let edge_count_raw = reader.read_i32_le()?;
    if edge_count_raw < 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: format!("Region.{label}_edge_count"),
            detail: format!("edge_count must be >= 0, got {edge_count_raw}"),
        });
    }
    let edge_count = edge_count_raw as usize;
    let vertex_count = edge_count + 1; // closing vertex
    let bytes_needed = vertex_count * 37; // each TPolySegment = 37 bytes
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
                key: format!("Region.{label}_poly_segment_kind"),
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

/// Parses a Region primitive from its single PcbLib subrecord.
///
/// Binary layout:
///   [13 bytes]     Common header (PcbPrimitiveCommon)
///   [1 byte]       Region kind (u8 → RegionKind)
///   [4 bytes]      Hole count (i32 LE — number of hole contours)
///   [4 bytes]      Parameter string length (u32 LE, includes NUL terminator)
///   [N bytes]      Win1252 parameter string (pipe-delimited |KEY=VALUE|)
///   [4 + V*16]     Main contour: vertex count (i32 LE) + f64 (x, y) pairs
///   [4 + V*16] * H Hole contours (one per hole_count)
///
/// This is the same base format as ComponentBody (which inherits from Region).
pub(crate) fn parse_region(data: &[u8], is_shape_based_section: bool) -> Result<PcbRegion> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;
    let kind = RegionKind::try_from(reader.read_u8()?)?;

    // Hole count — number of hole contours that follow the main contour.
    let hole_count_raw = reader.read_i32_le()?;
    if hole_count_raw < 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Region.hole_count".to_owned(),
            detail: format!("hole_count must be >= 0, got {}", hole_count_raw),
        });
    }
    let hole_count = hole_count_raw as usize;

    // Length-prefixed parameter string (Win1252, includes NUL terminator).
    let param_len = reader.read_u32_le()? as usize;
    let param_bytes = reader.read_bytes(param_len)?;
    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(param_bytes);
    let mut params = ParameterCollection::from_str(&decoded)?;

    // Extract region parameters.
    let v7_layer = params
        .remove_optional::<String>("V7_LAYER")?
        .unwrap_or_default();
    let name = params
        .remove_optional::<String>("NAME")?
        .unwrap_or_default();
    let param_kind = params.remove_optional::<i32>("KIND")?.unwrap_or(0);
    let subpoly_index = params.remove_optional::<i32>("SUBPOLYINDEX")?.unwrap_or(-1);
    let union_index = params.remove_optional::<i32>("UNIONINDEX")?.unwrap_or(0);
    let arc_resolution = parse_mil_param(&mut params, "ARCRESOLUTION")?;
    let is_shape_based = params
        .remove_optional::<bool>("ISSHAPEBASED")?
        .unwrap_or(false);
    let cavity_height = parse_mil_param(&mut params, "CAVITYHEIGHT")?;
    let keepout_restrictions = params
        .remove_optional::<i32>("KEEPOUTRESTRICTIONS")?
        .unwrap_or(0);
    let layer = params
        .remove_optional::<String>("LAYER")?
        .unwrap_or_default();
    let keepout = params
        .remove_optional::<bool>("KEEPOUT")?
        .unwrap_or(false);
    let is_board_cutout = params
        .remove_optional::<bool>("ISBOARDCUTOUT")?
        .unwrap_or(false);
    let pad_index = params.remove_optional::<i32>("PADINDEX")?.unwrap_or(-1);

    // BoardRegion-specific parameters (present when OBJECTKIND=BoardRegion in PcbDoc)
    let object_kind = params
        .remove_optional::<String>("OBJECTKIND")?
        .unwrap_or_default();
    let bending_line_count = params
        .remove_optional::<i32>("BENDINGLINECOUNT")?
        .unwrap_or(0);
    let locked_3d = params
        .remove_optional::<bool>("LOCKED3D")?
        .unwrap_or(false);
    let layer_stack_id = params
        .remove_optional::<String>("LAYERSTACKID")?
        .unwrap_or_default();

    // Shape-based regions include indexed edge geometry in the param string
    // (MAINCONTOURVERTEXCOUNT, KIND0, VX0, VY0, CX0, CY0, SA0, EA0, R0, ...).
    // We consume these to pass assert_exhausted but don't store them separately —
    // the actual geometry comes from the f64 vertex arrays below.
    if is_shape_based {
        let shape_vertex_count = params
            .remove_optional::<i32>("MAINCONTOURVERTEXCOUNT")?
            .unwrap_or(0);
        for i in 0..shape_vertex_count {
            let idx = i.to_string();
            // Each shape-based edge has: KIND, VX, VY, CX, CY, SA, EA, R
            params.remove_optional::<String>(&format!("KIND{}", idx))?;
            params.remove_optional::<String>(&format!("VX{}", idx))?;
            params.remove_optional::<String>(&format!("VY{}", idx))?;
            params.remove_optional::<String>(&format!("CX{}", idx))?;
            params.remove_optional::<String>(&format!("CY{}", idx))?;
            params.remove_optional::<String>(&format!("SA{}", idx))?;
            params.remove_optional::<String>(&format!("EA{}", idx))?;
            params.remove_optional::<String>(&format!("R{}", idx))?;
        }
        // Hole contour shape data
        for h in 0..hole_count {
            let hole_key = format!("HOLECONTOUR{}VERTEXCOUNT", h);
            let hole_vertex_count = params.remove_optional::<i32>(&hole_key)?.unwrap_or(0);
            for i in 0..hole_vertex_count {
                let prefix = format!("HOLECONTOUR{}", h);
                params.remove_optional::<String>(&format!("{}KIND{}", prefix, i))?;
                params.remove_optional::<String>(&format!("{}VX{}", prefix, i))?;
                params.remove_optional::<String>(&format!("{}VY{}", prefix, i))?;
                params.remove_optional::<String>(&format!("{}CX{}", prefix, i))?;
                params.remove_optional::<String>(&format!("{}CY{}", prefix, i))?;
                params.remove_optional::<String>(&format!("{}SA{}", prefix, i))?;
                params.remove_optional::<String>(&format!("{}EA{}", prefix, i))?;
                params.remove_optional::<String>(&format!("{}R{}", prefix, i))?;
            }
        }
    }

    params.assert_exhausted()?;

    // In ShapeBasedRegions6, the OUTLINE uses TPolySegment binary format (37-byte
    // records, N+1 vertices). Holes ALWAYS use legacy f64 format (16-byte pairs)
    // even in ShapeBasedRegions6 — the section kind only affects the outline format.
    let outline = if is_shape_based_section {
        Contour::ShapeBased(read_polysegment_contour(&mut reader, "outline")?)
    } else {
        Contour::Legacy(read_f64_contour(&mut reader, "outline")?)
    };

    // Hole contours: always legacy f64 vertex pairs.
    let mut holes = Vec::with_capacity(hole_count);
    for i in 0..hole_count {
        holes.push(Contour::Legacy(read_f64_contour(&mut reader, &format!("hole{}", i))?));
    }

    reader.assert_exhausted()?;

    Ok(PcbRegion {
        common,
        kind,
        v7_layer,
        name,
        param_kind,
        subpoly_index,
        union_index,
        arc_resolution,
        is_shape_based,
        cavity_height,
        keepout_restrictions,
        layer,
        keepout,
        is_board_cutout,
        pad_index,
        object_kind,
        bending_line_count,
        locked_3d,
        layer_stack_id,
        outline,
        holes,
        unique_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AltiumFormatError;
    use crate::binary_io::BinaryWriter;

    fn write_common_header(w: &mut BinaryWriter) {
        w.write_u8(1); // layer = TopLayer
        w.write_u16_le(0x000C); // flags
        w.write_u16_le(0xFFFF); // net_index = none
        w.write_u16_le(0xFFFF); // polygon_index = none
        w.write_u16_le(0xFFFF); // component_index = none
        w.write_u16_le(0xFFFF); // coordinate_index = none
        w.write_u16_le(0xFFFF); // dimension_index = none
    }

    fn make_param_string(params: &str) -> Vec<u8> {
        let s = format!("{}\0", params);
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(&s);
        encoded.to_vec()
    }

    #[test]
    fn parse_region_no_vertices() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_u8(0); // kind = Copper
        w.write_i32_le(0); // hole_count = 0
        let params = make_param_string("|");
        w.write_u32_le(params.len() as u32);
        w.write_bytes(&params);
        w.write_i32_le(0); // main contour vertex_count = 0
        let data = w.finish();
        let region = parse_region(&data, false).unwrap();
        assert_eq!(region.kind, RegionKind::Copper);
        assert!(matches!(&region.outline, Contour::Legacy(pts) if pts.is_empty()));
        assert!(region.holes.is_empty());
    }

    #[test]
    fn parse_region_with_outline() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_u8(1); // kind = Cutout
        w.write_i32_le(0); // hole_count = 0
        let params = make_param_string(
            "|V7_LAYER=TOP|NAME= |KIND=0|SUBPOLYINDEX=-1|UNIONINDEX=0|ARCRESOLUTION=0.5mil|ISSHAPEBASED=FALSE|CAVITYHEIGHT=0mil|",
        );
        w.write_u32_le(params.len() as u32);
        w.write_bytes(&params);
        // 3 vertices as f64 pairs
        w.write_i32_le(3);
        w.write_f64_le(0.0);
        w.write_f64_le(0.0);
        w.write_f64_le(10_000.0);
        w.write_f64_le(0.0);
        w.write_f64_le(10_000.0);
        w.write_f64_le(10_000.0);
        let data = w.finish();
        let region = parse_region(&data, false).unwrap();
        assert_eq!(region.kind, RegionKind::Cutout);
        let pts = match &region.outline {
            Contour::Legacy(pts) => pts,
            _ => panic!("expected Legacy contour"),
        };
        assert_eq!(pts.len(), 3);
        assert_eq!(pts[0].x.to_internal(), 0);
        assert_eq!(pts[1].x.to_internal(), 10_000);
        assert_eq!(pts[2].y.to_internal(), 10_000);
        assert!(region.holes.is_empty());
        assert_eq!(region.v7_layer, "TOP");
        assert!(!region.is_shape_based);
    }

    #[test]
    fn parse_region_with_hole() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_u8(0); // kind = Copper
        w.write_i32_le(1); // hole_count = 1
        let params = make_param_string("|");
        w.write_u32_le(params.len() as u32);
        w.write_bytes(&params);
        // Main contour: 4 vertices
        w.write_i32_le(4);
        for &(x, y) in &[
            (0.0, 0.0),
            (100_000.0, 0.0),
            (100_000.0, 100_000.0),
            (0.0, 100_000.0),
        ] {
            w.write_f64_le(x);
            w.write_f64_le(y);
        }
        // Hole contour: 3 vertices
        w.write_i32_le(3);
        for &(x, y) in &[
            (20_000.0, 20_000.0),
            (80_000.0, 20_000.0),
            (50_000.0, 80_000.0),
        ] {
            w.write_f64_le(x);
            w.write_f64_le(y);
        }
        let data = w.finish();
        let region = parse_region(&data, false).unwrap();
        let outline_pts = match &region.outline {
            Contour::Legacy(pts) => pts,
            _ => panic!("expected Legacy contour"),
        };
        assert_eq!(outline_pts.len(), 4);
        assert_eq!(region.holes.len(), 1);
        let hole_pts = match &region.holes[0] {
            Contour::Legacy(pts) => pts,
            _ => panic!("expected Legacy hole contour"),
        };
        assert_eq!(hole_pts.len(), 3);
        assert_eq!(hole_pts[0].x.to_internal(), 20_000);
    }

    #[test]
    fn parse_region_negative_hole_count_returns_error() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_u8(0);
        w.write_i32_le(-1); // negative hole_count
        let data = w.finish();
        let result = parse_region(&data, false);
        assert!(matches!(
            result,
            Err(AltiumFormatError::InvalidParamValue { .. })
        ));
    }

    #[test]
    fn truncated_region_returns_error() {
        let data = [0u8; 5];
        let result = parse_region(&data, false);
        assert!(matches!(
            result,
            Err(AltiumFormatError::BinaryReadPastEnd { .. })
        ));
    }
}

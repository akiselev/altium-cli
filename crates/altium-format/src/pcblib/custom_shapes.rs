//! PcbLib CustomShapes, CustomMaskShapes, and CornerRadiusChamfer sidecar parsers.
//!
//! All three streams use the parameter-block format:
//!   [4 bytes] u32 LE: entry count
//!   For each entry:
//!     [4 bytes] u32 LE: parameter string length (including NUL terminator)
//!     [N bytes] NUL-terminated parameter string (|KEY=VALUE| pipe-delimited)
//!
//! CustomShapes entries use `S{N}.` prefixed parameters for per-layer shape defs.
//! CustomMaskShapes entries use `SPM{N}.` prefixed parameters for per-layer mask defs.
//! CornerRadiusChamfer entries use `SCR{N}.` prefixed parameters for corner radius defs.

use altium_format_types::{Coord, PadShapeSubKind};

use crate::binary_io::{BinaryReader, BinaryWriter};
use crate::param_collection::ParameterCollection;
use crate::{AltiumFormatError, Result};

// ---------------------------------------------------------------------------
// CustomShapes
// ---------------------------------------------------------------------------

/// Corner enable flags for rounded/chamfered rectangle custom shapes.
#[derive(Debug, Clone)]
pub(crate) struct CustomShapeCorners {
    pub(crate) bottom_left: bool,
    pub(crate) bottom_right: bool,
    pub(crate) top_right: bool,
    pub(crate) top_left: bool,
    pub(crate) corner_size: Coord,
}

/// One per-layer shape definition within a CustomShapes entry.
#[derive(Debug, Clone)]
pub(crate) struct CustomShapeLayerDef {
    pub(crate) layer: String,
    pub(crate) x_size: Coord,
    pub(crate) y_size: Coord,
    pub(crate) shape_kind: PadShapeSubKind,
    pub(crate) corners: Option<CustomShapeCorners>,
}

/// One entry from the CustomShapes sidecar stream.
#[derive(Debug, Clone)]
pub(crate) struct CustomShapeEntry {
    pub(crate) primitive_index: usize,
    pub(crate) layer_defs: Vec<CustomShapeLayerDef>,
}

/// Parses a CustomShapes sidecar stream.
pub(crate) fn parse_custom_shapes(data: &[u8]) -> Result<Vec<CustomShapeEntry>> {
    let entries = parse_param_block_entries(data, "CustomShapes")?;
    let mut result = Vec::with_capacity(entries.len());
    for (i, mut params) in entries.into_iter().enumerate() {
        let primitive_index = parse_primitive_index(&mut params, "CustomShapes", i)?;

        let mut layer_defs = Vec::new();
        for n in 0.. {
            let layer_key = format!("S{n}.LAYER");
            if params.keys_matching(&layer_key).is_empty() {
                break;
            }
            let prefix = format!("S{n}.");
            let layer: String = params.remove_required(&format!("{prefix}LAYER"))?;
            let x_size = parse_coord_param(&mut params, &format!("{prefix}XSIZE"))?;
            let y_size = parse_coord_param(&mut params, &format!("{prefix}YSIZE"))?;
            let shape_kind_val: u8 = params.remove_required(&format!("{prefix}SHAPEKIND"))?;
            let shape_kind = PadShapeSubKind::try_from(shape_kind_val).map_err(|_| {
                AltiumFormatError::InvalidParamValue {
                    key: format!("{prefix}SHAPEKIND"),
                    detail: format!("unknown PadShapeSubKind value: {shape_kind_val}"),
                }
            })?;

            let corners = match shape_kind {
                PadShapeSubKind::RoundedRectangle | PadShapeSubKind::ChamferedRectangle => {
                    let cps_prefix = format!("{prefix}CPS.");
                    let bottom_left = parse_bool_param(&mut params, &format!("{cps_prefix}BLCE"))?;
                    let bottom_right = parse_bool_param(&mut params, &format!("{cps_prefix}BRCE"))?;
                    let top_right = parse_bool_param(&mut params, &format!("{cps_prefix}TRCE"))?;
                    let top_left = parse_bool_param(&mut params, &format!("{cps_prefix}TLCE"))?;
                    let corner_size = parse_coord_param(&mut params, &format!("{cps_prefix}CS"))?;
                    Some(CustomShapeCorners {
                        bottom_left,
                        bottom_right,
                        top_right,
                        top_left,
                        corner_size,
                    })
                }
                _ => None,
            };

            layer_defs.push(CustomShapeLayerDef {
                layer,
                x_size,
                y_size,
                shape_kind,
                corners,
            });
        }

        params.assert_exhausted()?;

        result.push(CustomShapeEntry {
            primitive_index,
            layer_defs,
        });
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// CustomMaskShapes
// ---------------------------------------------------------------------------

/// One per-layer mask shape definition.
#[derive(Debug, Clone)]
pub(crate) struct CustomMaskLayerDef {
    pub(crate) layer: String,
    pub(crate) shape: String,
    pub(crate) x_size: Coord,
    pub(crate) y_size: Coord,
    pub(crate) corner_radius_percent: Option<i32>,
}

/// One entry from the CustomMaskShapes sidecar stream.
#[derive(Debug, Clone)]
pub(crate) struct CustomMaskShapeEntry {
    pub(crate) primitive_index: usize,
    pub(crate) mask_defs: Vec<CustomMaskLayerDef>,
}

/// Parses a CustomMaskShapes sidecar stream.
pub(crate) fn parse_custom_mask_shapes(data: &[u8]) -> Result<Vec<CustomMaskShapeEntry>> {
    let entries = parse_param_block_entries(data, "CustomMaskShapes")?;
    let mut result = Vec::with_capacity(entries.len());
    for (i, mut params) in entries.into_iter().enumerate() {
        let primitive_index = parse_primitive_index(&mut params, "CustomMaskShapes", i)?;

        let mut mask_defs = Vec::new();
        for n in 0.. {
            let layer_key = format!("SPM{n}.LAYER");
            if params.keys_matching(&layer_key).is_empty() {
                break;
            }
            let prefix = format!("SPM{n}.");
            let layer: String = params.remove_required(&format!("{prefix}LAYER"))?;
            let shape: String = params.remove_required(&format!("{prefix}SHAPE"))?;
            let x_size = parse_coord_param(&mut params, &format!("{prefix}XSIZE"))?;
            let y_size = parse_coord_param(&mut params, &format!("{prefix}YSIZE"))?;
            let corner_radius_percent = params.remove_optional::<i32>(&format!("{prefix}CRPCT"))?;

            mask_defs.push(CustomMaskLayerDef {
                layer,
                shape,
                x_size,
                y_size,
                corner_radius_percent,
            });
        }

        params.assert_exhausted()?;

        result.push(CustomMaskShapeEntry {
            primitive_index,
            mask_defs,
        });
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// CornerRadiusChamfer
// ---------------------------------------------------------------------------

/// One per-layer corner radius definition.
#[derive(Debug, Clone)]
pub(crate) struct CornerRadiusLayerDef {
    pub(crate) layer: String,
    pub(crate) corner_radius_size: Coord,
}

/// One entry from the CornerRadiusChamfer sidecar stream.
#[derive(Debug, Clone)]
pub(crate) struct CornerRadiusChamferEntry {
    pub(crate) primitive_index: usize,
    pub(crate) layer_defs: Vec<CornerRadiusLayerDef>,
}

/// Parses a CornerRadiusChamfer sidecar stream.
pub(crate) fn parse_corner_radius_chamfer(data: &[u8]) -> Result<Vec<CornerRadiusChamferEntry>> {
    let entries = parse_param_block_entries(data, "CornerRadiusChamfer")?;
    let mut result = Vec::with_capacity(entries.len());
    for (i, mut params) in entries.into_iter().enumerate() {
        let primitive_index = parse_primitive_index(&mut params, "CornerRadiusChamfer", i)?;

        let mut layer_defs = Vec::new();
        for n in 0.. {
            let layer_key = format!("SCR{n}.LAYER");
            if params.keys_matching(&layer_key).is_empty() {
                break;
            }
            let prefix = format!("SCR{n}.");
            let layer: String = params.remove_required(&format!("{prefix}LAYER"))?;
            let corner_radius_size = parse_coord_param(&mut params, &format!("{prefix}CRSIZE"))?;

            layer_defs.push(CornerRadiusLayerDef {
                layer,
                corner_radius_size,
            });
        }

        params.assert_exhausted()?;

        result.push(CornerRadiusChamferEntry {
            primitive_index,
            layer_defs,
        });
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Parses the u32-count + (u32-len + NUL-terminated-params)* parameter-block format.
///
/// Returns a Vec of ParameterCollections, one per entry.
fn parse_param_block_entries(data: &[u8], stream_name: &str) -> Result<Vec<ParameterCollection>> {
    let mut reader = BinaryReader::new(data);
    let count = reader.read_u32_le()? as usize;

    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let str_len = reader.read_u32_le()? as usize;
        let str_bytes = reader.read_bytes(str_len)?;
        let stripped = str_bytes.strip_suffix(b"\x00").unwrap_or(str_bytes);
        let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(stripped);
        let params = ParameterCollection::from_str(&decoded)?;
        entries.push(params);
    }

    reader
        .assert_exhausted()
        .map_err(|_| AltiumFormatError::InvalidParamValue {
            key: stream_name.to_owned(),
            detail: format!(
                "{} trailing bytes after {} entries",
                reader.remaining(),
                count,
            ),
        })?;

    Ok(entries)
}

/// Extracts and validates PRIMITIVEINDEX from a parameter collection.
fn parse_primitive_index(
    params: &mut ParameterCollection,
    stream_name: &str,
    entry_index: usize,
) -> Result<usize> {
    let primitive_index: i32 = params.remove_required("PRIMITIVEINDEX")?;
    if primitive_index < 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "PRIMITIVEINDEX".to_owned(),
            detail: format!(
                "{stream_name} entry {entry_index}: negative primitive index: {primitive_index}"
            ),
        });
    }
    Ok(primitive_index as usize)
}

/// Parses a coordinate value from a parameter string.
///
/// Altium stores coordinates as internal units (i32) in these sidecar streams.
fn parse_coord_param(params: &mut ParameterCollection, key: &str) -> Result<Coord> {
    let value: i32 = params.remove_required(key)?;
    Ok(Coord::from_internal(value))
}

/// Parses a boolean parameter (TRUE/FALSE string).
fn parse_bool_param(params: &mut ParameterCollection, key: &str) -> Result<bool> {
    let value: String = params.remove_required(key)?;
    match value.to_ascii_uppercase().as_str() {
        "TRUE" => Ok(true),
        "FALSE" => Ok(false),
        _ => Err(AltiumFormatError::InvalidParamValue {
            key: key.to_owned(),
            detail: format!("expected TRUE or FALSE, got '{value}'"),
        }),
    }
}

/// Validates that custom shape entries reference valid pad primitives.
pub(crate) fn validate_custom_shape_entries(
    primitives: &[crate::pcblib::PcbPrimitive],
    custom_shapes: &[CustomShapeEntry],
    custom_mask_shapes: &[CustomMaskShapeEntry],
    corner_radius_chamfer: &[CornerRadiusChamferEntry],
) -> Result<()> {
    let primitive_count = primitives.len();

    for entry in custom_shapes {
        let idx = entry.primitive_index;
        let primitive =
            primitives
                .get(idx)
                .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                    key: "PRIMITIVEINDEX".to_owned(),
                    detail: format!(
                        "CustomShapes primitive index {idx} out of range \
                     (footprint has {primitive_count} primitives)"
                    ),
                })?;
        if !matches!(primitive, crate::pcblib::PcbPrimitive::Pad(_)) {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "PRIMITIVEINDEX".to_owned(),
                detail: format!("CustomShapes primitive index {idx} is not a Pad"),
            });
        }
    }

    for entry in custom_mask_shapes {
        let idx = entry.primitive_index;
        let primitive =
            primitives
                .get(idx)
                .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                    key: "PRIMITIVEINDEX".to_owned(),
                    detail: format!(
                        "CustomMaskShapes primitive index {idx} out of range \
                     (footprint has {primitive_count} primitives)"
                    ),
                })?;
        if !matches!(primitive, crate::pcblib::PcbPrimitive::Pad(_)) {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "PRIMITIVEINDEX".to_owned(),
                detail: format!("CustomMaskShapes primitive index {idx} is not a Pad"),
            });
        }
    }

    for entry in corner_radius_chamfer {
        let idx = entry.primitive_index;
        let primitive =
            primitives
                .get(idx)
                .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                    key: "PRIMITIVEINDEX".to_owned(),
                    detail: format!(
                        "CornerRadiusChamfer primitive index {idx} out of range \
                     (footprint has {primitive_count} primitives)"
                    ),
                })?;
        if !matches!(primitive, crate::pcblib::PcbPrimitive::Pad(_)) {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "PRIMITIVEINDEX".to_owned(),
                detail: format!("CornerRadiusChamfer primitive index {idx} is not a Pad"),
            });
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

/// Serializes a list of parameter byte buffers into the parameter-block format.
///
/// Format: u32 count + (u32 len + param bytes) per entry.
/// Each entry is already NUL-terminated (from `ParameterCollection::to_bytes()`).
fn serialize_param_block_entries(entries: &[Vec<u8>]) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    w.write_u32_le(entries.len() as u32);
    for bytes in entries {
        w.write_u32_le(bytes.len() as u32);
        w.write_bytes(bytes);
    }
    w.finish()
}

fn bool_str(v: bool) -> &'static str {
    if v { "TRUE" } else { "FALSE" }
}

/// Serializes CustomShapes entries to the sidecar stream format.
pub(crate) fn serialize_custom_shapes(entries: &[CustomShapeEntry]) -> Vec<u8> {
    let param_entries: Vec<Vec<u8>> = entries
        .iter()
        .map(|entry| {
            let mut params = ParameterCollection::new();
            params.insert("PRIMITIVEINDEX", entry.primitive_index.to_string());
            for (n, def) in entry.layer_defs.iter().enumerate() {
                let prefix = format!("S{n}.");
                params.insert(&format!("{prefix}LAYER"), def.layer.clone());
                params.insert(
                    &format!("{prefix}XSIZE"),
                    def.x_size.to_internal().to_string(),
                );
                params.insert(
                    &format!("{prefix}YSIZE"),
                    def.y_size.to_internal().to_string(),
                );
                params.insert(
                    &format!("{prefix}SHAPEKIND"),
                    (def.shape_kind as u8).to_string(),
                );
                if let Some(corners) = &def.corners {
                    let cps = format!("{prefix}CPS.");
                    params.insert(
                        &format!("{cps}BLCE"),
                        bool_str(corners.bottom_left).to_owned(),
                    );
                    params.insert(
                        &format!("{cps}BRCE"),
                        bool_str(corners.bottom_right).to_owned(),
                    );
                    params.insert(
                        &format!("{cps}TRCE"),
                        bool_str(corners.top_right).to_owned(),
                    );
                    params.insert(&format!("{cps}TLCE"), bool_str(corners.top_left).to_owned());
                    params.insert(
                        &format!("{cps}CS"),
                        corners.corner_size.to_internal().to_string(),
                    );
                }
            }
            params.to_bytes()
        })
        .collect();
    serialize_param_block_entries(&param_entries)
}

/// Serializes CustomMaskShapes entries to the sidecar stream format.
pub(crate) fn serialize_custom_mask_shapes(entries: &[CustomMaskShapeEntry]) -> Vec<u8> {
    let param_entries: Vec<Vec<u8>> = entries
        .iter()
        .map(|entry| {
            let mut params = ParameterCollection::new();
            params.insert("PRIMITIVEINDEX", entry.primitive_index.to_string());
            for (n, def) in entry.mask_defs.iter().enumerate() {
                let prefix = format!("SPM{n}.");
                params.insert(&format!("{prefix}LAYER"), def.layer.clone());
                params.insert(&format!("{prefix}SHAPE"), def.shape.clone());
                params.insert(
                    &format!("{prefix}XSIZE"),
                    def.x_size.to_internal().to_string(),
                );
                params.insert(
                    &format!("{prefix}YSIZE"),
                    def.y_size.to_internal().to_string(),
                );
                if let Some(crpct) = def.corner_radius_percent {
                    params.insert(&format!("{prefix}CRPCT"), crpct.to_string());
                }
            }
            params.to_bytes()
        })
        .collect();
    serialize_param_block_entries(&param_entries)
}

/// Serializes CornerRadiusChamfer entries to the sidecar stream format.
pub(crate) fn serialize_corner_radius_chamfer(entries: &[CornerRadiusChamferEntry]) -> Vec<u8> {
    let param_entries: Vec<Vec<u8>> = entries
        .iter()
        .map(|entry| {
            let mut params = ParameterCollection::new();
            for (n, def) in entry.layer_defs.iter().enumerate() {
                let prefix = format!("SCR{n}.");
                params.insert(&format!("{prefix}LAYER"), def.layer.clone());
                params.insert(
                    &format!("{prefix}CRSIZE"),
                    def.corner_radius_size.to_internal().to_string(),
                );
            }
            params.insert("PRIMITIVEINDEX", entry.primitive_index.to_string());
            params.to_bytes()
        })
        .collect();
    serialize_param_block_entries(&param_entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a parameter-block stream from a list of parameter strings.
    fn make_param_block_stream(entries: &[&str]) -> Vec<u8> {
        let mut w = BinaryWriter::new();
        w.write_u32_le(entries.len() as u32);
        for entry in entries {
            let bytes = format!("{entry}\x00");
            let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(&bytes);
            w.write_u32_le(encoded.len() as u32);
            w.write_bytes(&encoded);
        }
        w.finish()
    }

    #[test]
    fn parse_custom_shapes_empty_stream() {
        let data = make_param_block_stream(&[]);
        let entries = parse_custom_shapes(&data).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_custom_shapes_single_rounded_rectangle() {
        let data = make_param_block_stream(&[
            "|PRIMITIVEINDEX=3|S0.LAYER=TOP|S0.XSIZE=275592|S0.YSIZE=110236|S0.SHAPEKIND=3|S0.CPS.BLCE=FALSE|S0.CPS.BRCE=TRUE|S0.CPS.TRCE=TRUE|S0.CPS.TLCE=FALSE|S0.CPS.CS=0",
        ]);
        let entries = parse_custom_shapes(&data).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].primitive_index, 3);
        assert_eq!(entries[0].layer_defs.len(), 1);

        let def = &entries[0].layer_defs[0];
        assert_eq!(def.layer, "TOP");
        assert_eq!(def.x_size, Coord::from_internal(275592));
        assert_eq!(def.y_size, Coord::from_internal(110236));
        assert_eq!(def.shape_kind, PadShapeSubKind::RoundedRectangle);

        let corners = def.corners.as_ref().unwrap();
        assert!(!corners.bottom_left);
        assert!(corners.bottom_right);
        assert!(corners.top_right);
        assert!(!corners.top_left);
        assert_eq!(corners.corner_size, Coord::ZERO);
    }

    #[test]
    fn parse_custom_shapes_multiple_entries() {
        let data = make_param_block_stream(&[
            "|PRIMITIVEINDEX=3|S0.LAYER=TOP|S0.XSIZE=275592|S0.YSIZE=110236|S0.SHAPEKIND=3|S0.CPS.BLCE=FALSE|S0.CPS.BRCE=TRUE|S0.CPS.TRCE=TRUE|S0.CPS.TLCE=FALSE|S0.CPS.CS=0",
            "|PRIMITIVEINDEX=4|S0.LAYER=TOP|S0.XSIZE=275592|S0.YSIZE=110236|S0.SHAPEKIND=3|S0.CPS.BLCE=TRUE|S0.CPS.BRCE=FALSE|S0.CPS.TRCE=FALSE|S0.CPS.TLCE=TRUE|S0.CPS.CS=0",
        ]);
        let entries = parse_custom_shapes(&data).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].primitive_index, 3);
        assert_eq!(entries[1].primitive_index, 4);
    }

    #[test]
    fn parse_custom_mask_shapes_single_custom() {
        let data = make_param_block_stream(&[
            "|PRIMITIVEINDEX=3|SPM0.LAYER=TOPPASTE|SPM0.SHAPE=CUSTOM|SPM0.XSIZE=0|SPM0.YSIZE=0",
        ]);
        let entries = parse_custom_mask_shapes(&data).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].primitive_index, 3);
        assert_eq!(entries[0].mask_defs.len(), 1);

        let def = &entries[0].mask_defs[0];
        assert_eq!(def.layer, "TOPPASTE");
        assert_eq!(def.shape, "CUSTOM");
        assert_eq!(def.x_size, Coord::ZERO);
        assert_eq!(def.y_size, Coord::ZERO);
        assert!(def.corner_radius_percent.is_none());
    }

    #[test]
    fn parse_custom_mask_shapes_rounded_rectangle_with_crpct() {
        let data = make_param_block_stream(&[
            "|PRIMITIVEINDEX=7|SPM0.LAYER=TOPPASTE|SPM0.SHAPE=ROUNDEDRECTANGLE|SPM0.XSIZE=3543307|SPM0.YSIZE=2755906|SPM0.CRPCT=20",
        ]);
        let entries = parse_custom_mask_shapes(&data).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].primitive_index, 7);

        let def = &entries[0].mask_defs[0];
        assert_eq!(def.layer, "TOPPASTE");
        assert_eq!(def.shape, "ROUNDEDRECTANGLE");
        assert_eq!(def.x_size, Coord::from_internal(3543307));
        assert_eq!(def.y_size, Coord::from_internal(2755906));
        assert_eq!(def.corner_radius_percent, Some(20));
    }

    #[test]
    fn parse_corner_radius_chamfer_single() {
        let data =
            make_param_block_stream(&["|SCR0.LAYER=TOP|SCR0.CRSIZE=275590|PRIMITIVEINDEX=0"]);
        let entries = parse_corner_radius_chamfer(&data).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].primitive_index, 0);
        assert_eq!(entries[0].layer_defs.len(), 1);
        assert_eq!(entries[0].layer_defs[0].layer, "TOP");
        assert_eq!(
            entries[0].layer_defs[0].corner_radius_size,
            Coord::from_internal(275590)
        );
    }

    #[test]
    fn parse_corner_radius_chamfer_multiple() {
        let data = make_param_block_stream(&[
            "|SCR0.LAYER=TOP|SCR0.CRSIZE=275590|PRIMITIVEINDEX=0",
            "|SCR0.LAYER=TOP|SCR0.CRSIZE=275590|PRIMITIVEINDEX=1",
        ]);
        let entries = parse_corner_radius_chamfer(&data).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].primitive_index, 0);
        assert_eq!(entries[1].primitive_index, 1);
    }

    #[test]
    fn parse_param_block_short_stream_errors() {
        // Too short to even read count
        let err = parse_custom_shapes(&[0x01]).unwrap_err();
        assert!(
            format!("{err}").contains("read past end") || format!("{err}").contains("needed"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parse_custom_shapes_bad_shape_kind_errors() {
        let data = make_param_block_stream(&[
            "|PRIMITIVEINDEX=0|S0.LAYER=TOP|S0.XSIZE=100|S0.YSIZE=200|S0.SHAPEKIND=99",
        ]);
        let err = parse_custom_shapes(&data).unwrap_err();
        assert!(matches!(err, AltiumFormatError::InvalidParamValue { .. }));
    }

    #[test]
    fn parse_custom_shapes_missing_primitive_index_errors() {
        let data =
            make_param_block_stream(&["|S0.LAYER=TOP|S0.XSIZE=100|S0.YSIZE=200|S0.SHAPEKIND=0"]);
        let err = parse_custom_shapes(&data).unwrap_err();
        assert!(matches!(err, AltiumFormatError::MissingParam(_)));
    }

    #[test]
    fn parse_custom_shapes_trailing_bytes_errors() {
        // Valid 0-entry stream, but with extra trailing bytes
        let mut data = make_param_block_stream(&[]);
        data.extend_from_slice(&[0xFF, 0xFF]);
        let err = parse_custom_shapes(&data).unwrap_err();
        assert!(matches!(err, AltiumFormatError::InvalidParamValue { .. }));
    }

    #[test]
    fn custom_shapes_serialize_roundtrip() {
        let data = make_param_block_stream(&[
            "|PRIMITIVEINDEX=3|S0.LAYER=TOP|S0.XSIZE=275592|S0.YSIZE=110236|S0.SHAPEKIND=3|S0.CPS.BLCE=FALSE|S0.CPS.BRCE=TRUE|S0.CPS.TRCE=TRUE|S0.CPS.TLCE=FALSE|S0.CPS.CS=0",
            "|PRIMITIVEINDEX=4|S0.LAYER=TOP|S0.XSIZE=275592|S0.YSIZE=110236|S0.SHAPEKIND=3|S0.CPS.BLCE=TRUE|S0.CPS.BRCE=FALSE|S0.CPS.TRCE=FALSE|S0.CPS.TLCE=TRUE|S0.CPS.CS=0",
        ]);
        let entries = parse_custom_shapes(&data).unwrap();
        let serialized = serialize_custom_shapes(&entries);
        let reparsed = parse_custom_shapes(&serialized).unwrap();
        assert_eq!(reparsed.len(), 2);
        assert_eq!(reparsed[0].primitive_index, 3);
        assert_eq!(reparsed[1].primitive_index, 4);
        assert_eq!(reparsed[0].layer_defs[0].layer, "TOP");
        assert_eq!(
            reparsed[0].layer_defs[0].x_size,
            Coord::from_internal(275592)
        );
        let corners = reparsed[0].layer_defs[0].corners.as_ref().unwrap();
        assert!(!corners.bottom_left);
        assert!(corners.bottom_right);
    }

    #[test]
    fn custom_mask_shapes_serialize_roundtrip() {
        let data = make_param_block_stream(&[
            "|PRIMITIVEINDEX=7|SPM0.LAYER=TOPPASTE|SPM0.SHAPE=ROUNDEDRECTANGLE|SPM0.XSIZE=3543307|SPM0.YSIZE=2755906|SPM0.CRPCT=20",
        ]);
        let entries = parse_custom_mask_shapes(&data).unwrap();
        let serialized = serialize_custom_mask_shapes(&entries);
        let reparsed = parse_custom_mask_shapes(&serialized).unwrap();
        assert_eq!(reparsed.len(), 1);
        assert_eq!(reparsed[0].primitive_index, 7);
        assert_eq!(reparsed[0].mask_defs[0].shape, "ROUNDEDRECTANGLE");
        assert_eq!(reparsed[0].mask_defs[0].corner_radius_percent, Some(20));
    }

    #[test]
    fn corner_radius_chamfer_serialize_roundtrip() {
        let data = make_param_block_stream(&[
            "|SCR0.LAYER=TOP|SCR0.CRSIZE=275590|PRIMITIVEINDEX=0",
            "|SCR0.LAYER=TOP|SCR0.CRSIZE=39370|PRIMITIVEINDEX=1",
        ]);
        let entries = parse_corner_radius_chamfer(&data).unwrap();
        let serialized = serialize_corner_radius_chamfer(&entries);
        let reparsed = parse_corner_radius_chamfer(&serialized).unwrap();
        assert_eq!(reparsed.len(), 2);
        assert_eq!(reparsed[0].primitive_index, 0);
        assert_eq!(
            reparsed[0].layer_defs[0].corner_radius_size,
            Coord::from_internal(275590)
        );
        assert_eq!(reparsed[1].primitive_index, 1);
        assert_eq!(
            reparsed[1].layer_defs[0].corner_radius_size,
            Coord::from_internal(39370)
        );
    }
}

//! Schematic polygon record (RECORD=7).

use super::enums::*;
use crate::coord::{AltiumCoord, SchCoord};
use crate::newtypes::UniqueId;
use crate::traits::RecordType;
use altium_format_derive::altium_record;

/// Schematic polygon record -- RECORD=7.
///
/// Represents a polygon primitive on a schematic sheet.
/// Vertex data is skipped for now (handled in later phases).
#[altium_record(kind = "sch", record_id = 7, codec = "params")]
pub struct SchPolygonRecord {
    // --- Base object fields (flattened from GraphicalObjectBase) ---
    #[altium(key = "OwnerIndex")]
    owner_index: i32,

    #[altium(key = "OwnerPartId")]
    owner_part_id: i16,

    #[altium(key = "OwnerPartDisplayMode")]
    owner_part_display_mode: u8,

    #[altium(key = "IndexInSheet")]
    index_in_sheet: i32,

    #[altium(key = "IsNotAccesible")]
    is_not_accessible: bool,

    #[altium(key = "GraphicallyLocked")]
    graphically_locked: bool,

    // --- Polygon-specific fields ---
    #[altium(key = "LineWidth")]
    line_width: Size,

    #[altium(key = "Color")]
    color: u32,

    #[altium(key = "AreaColor")]
    area_color: u32,

    #[altium(key = "IsSolid")]
    is_solid: bool,

    #[altium(key = "Transparent")]
    transparent: bool,

    // Vertices are skipped for now -- handled in later phases
    // vertices: Vec<(SchCoord, SchCoord)>,
    #[altium(key = "UniqueID")]
    unique_id: UniqueId,
}

impl SchPolygonRecord {
    fn is_vertex_key(key: &str) -> bool {
        if key == "LOCATIONCOUNT" {
            return true;
        }
        if let Some(rest) = key.strip_prefix('X') {
            if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                return true;
            }
            if let Some(n) = rest.strip_suffix("_FRAC") {
                return !n.is_empty() && n.chars().all(|c| c.is_ascii_digit());
            }
        }
        if let Some(rest) = key.strip_prefix('Y') {
            if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                return true;
            }
            if let Some(n) = rest.strip_suffix("_FRAC") {
                return !n.is_empty() && n.chars().all(|c| c.is_ascii_digit());
            }
        }
        false
    }

    /// Returns indexed polygon vertices (Xn/Yn) as raw `SchCoord` units.
    pub fn vertices(&self) -> Vec<(i32, i32)> {
        let params = &self.origin().param().params;
        let count = params
            .get("LOCATIONCOUNT")
            .map(|v| v.as_int_or(0))
            .unwrap_or(0)
            .max(0) as usize;

        let mut out = Vec::with_capacity(count);
        for i in 1..=count {
            let x_key = format!("X{}", i);
            let x_frac_key = format!("X{}_FRAC", i);
            let y_key = format!("Y{}", i);
            let y_frac_key = format!("Y{}_FRAC", i);

            let x_int = params.get(&x_key).map(|v| v.as_int_or(0)).unwrap_or(0);
            let x_frac = params.get(&x_frac_key).map(|v| v.as_int_or(0)).unwrap_or(0);
            let y_int = params.get(&y_key).map(|v| v.as_int_or(0)).unwrap_or(0);
            let y_frac = params.get(&y_frac_key).map(|v| v.as_int_or(0)).unwrap_or(0);

            out.push((
                SchCoord::from_dxp_parts(x_int, x_frac).to_raw(),
                SchCoord::from_dxp_parts(y_int, y_frac).to_raw(),
            ));
        }

        out
    }

    /// Replaces indexed polygon vertices (Xn/Yn) and LOCATIONCOUNT.
    pub fn set_vertices(&mut self, vertices: &[(i32, i32)]) {
        let params = &mut self.origin_mut().param_mut().params;
        let keys_to_remove: Vec<String> = params
            .iter()
            .map(|(k, _)| k.to_string())
            .filter(|k| {
                if k == "LOCATIONCOUNT" {
                    return true;
                }
                if let Some(rest) = k.strip_prefix('X') {
                    if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                        return true;
                    }
                    if let Some(n) = rest.strip_suffix("_FRAC") {
                        return !n.is_empty() && n.chars().all(|c| c.is_ascii_digit());
                    }
                }
                if let Some(rest) = k.strip_prefix('Y') {
                    if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                        return true;
                    }
                    if let Some(n) = rest.strip_suffix("_FRAC") {
                        return !n.is_empty() && n.chars().all(|c| c.is_ascii_digit());
                    }
                }
                false
            })
            .collect();
        for key in keys_to_remove {
            params.remove(&key);
        }

        params.add_int("LOCATIONCOUNT", vertices.len() as i32);
        for (idx, (x_raw, y_raw)) in vertices.iter().enumerate() {
            let i = idx + 1;
            let (x_int, x_frac) = SchCoord::from_raw(*x_raw).to_dxp_parts();
            let (y_int, y_frac) = SchCoord::from_raw(*y_raw).to_dxp_parts();
            params.add_int(&format!("X{}", i), x_int);
            if x_frac != 0 {
                params.add_int(&format!("X{}_FRAC", i), x_frac);
            }
            params.add_int(&format!("Y{}", i), y_int);
            if y_frac != 0 {
                params.add_int(&format!("Y{}_FRAC", i), y_frac);
            }
        }
    }

    /// Copies indexed vertex params (`LOCATIONCOUNT`, `Xn`, `Yn`, `*_FRAC`)
    /// exactly from `src`, preserving sparse/missing/non-canonical forms.
    pub fn copy_vertices_from(&mut self, src: &Self) {
        let src_params = &src.origin().param().params;
        let to_copy: Vec<(String, String)> = src_params
            .iter()
            .filter(|(k, _)| Self::is_vertex_key(k))
            .map(|(k, v)| (k.to_string(), v.as_str().to_string()))
            .collect();

        let dst_params = &mut self.origin_mut().param_mut().params;
        let keys_to_remove: Vec<String> = dst_params
            .iter()
            .map(|(k, _)| k.to_string())
            .filter(|k| Self::is_vertex_key(k))
            .collect();
        for key in keys_to_remove {
            dst_params.remove(&key);
        }
        for (k, v) in to_copy {
            dst_params.add(&k, &v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip_polygon_getter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=7|LineWidth=1|Color=255|AreaColor=16777215|IsSolid=T|Transparent=F|",
        ));
        let rec = SchPolygonRecord::from_origin(origin);
        assert!(rec.is_solid().unwrap());
        assert!(!rec.transparent().unwrap());
        assert_eq!(rec.color().unwrap(), 255);
        assert_eq!(rec.area_color().unwrap(), 16777215);
    }

    #[test]
    fn roundtrip_polygon_setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=7|LineWidth=1|Color=255|IsSolid=F|",
        ));
        let mut rec = SchPolygonRecord::from_origin(origin);
        rec.set_is_solid(true);
        assert!(rec.is_solid().unwrap());
    }
}

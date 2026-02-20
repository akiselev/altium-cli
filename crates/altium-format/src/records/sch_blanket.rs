//! Blanket record (RECORD=225).

use crate::coord::SchCoord;
use altium_format_derive::altium_record;

/// Blanket record — region overlay for grouping/annotation on a schematic.
///
/// Corresponds to `BlanketData` / `ExportBlanket` in the v1 API.
///
/// Note: vertices (`Vec<(i32,i32)>`) are skipped in this phase and will be
/// handled with custom codec logic in a later phase.
#[altium_record(kind = "sch", record_id = 225, codec = "params")]
pub struct SchBlanketRecord {
    // --- GraphicalObjectBase (flattened) ---
    #[altium(key = "OWNERINDEX")]
    owner_index: i32,
    #[altium(key = "OWNERPARTID")]
    owner_part_id: i16,
    #[altium(key = "OWNERPARTDISPLAYMODE")]
    owner_part_display_mode: i32,
    #[altium(key = "INDEXINSHEET")]
    index_in_sheet: i32,
    #[altium(key = "ISNOTACCESIBLE")]
    is_not_accessible: bool,
    #[altium(key = "GRAPHICALLYLOCKED")]
    graphically_locked: bool,

    // --- Blanket-specific fields ---
    #[altium(key = "LOCATION.X")]
    location_x: SchCoord,
    #[altium(key = "LOCATION.Y")]
    location_y: SchCoord,
    #[altium(key = "CORNER.X")]
    corner_x: SchCoord,
    #[altium(key = "CORNER.Y")]
    corner_y: SchCoord,
    /// Line width (0=Smallest, 1=Small, 2=Medium, 3=Large).
    #[altium(key = "LINEWIDTH")]
    line_width: i32,
    #[altium(key = "COLOR")]
    color: u32,
    #[altium(key = "AREACOLOR")]
    area_color: u32,
    #[altium(key = "COLLAPSED")]
    collapsed: bool,
    /// Line style (0=Solid, 1=Dashed, 2=Dotted, 3=DashDotted).
    #[altium(key = "LINESTYLE")]
    line_style: i32,
    #[altium(key = "UNIQUEID")]
    unique_id: String,

    /// Vertex coordinates — skipped; handled in later phase.
    #[altium(skip)]
    _vertices: i32,
}

impl SchBlanketRecord {
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

    /// Returns indexed blanket vertices (Xn/Yn) as raw `SchCoord` units.
    pub fn vertices(&self) -> Vec<(i32, i32)> {
        use crate::coord::AltiumCoord;
        use crate::traits::RecordType;

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

    /// Replaces indexed blanket vertices (Xn/Yn) and LOCATIONCOUNT.
    pub fn set_vertices(&mut self, vertices: &[(i32, i32)]) {
        use crate::coord::AltiumCoord;
        use crate::traits::RecordType;

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
        use crate::traits::RecordType;

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
    fn roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=225|OWNERINDEX=0|LOCATION.X=100|LOCATION.Y=200|CORNER.X=500|CORNER.Y=600|LINEWIDTH=1|COLOR=0|AREACOLOR=16777215|COLLAPSED=F|LINESTYLE=0|UNIQUEID=ABCD1234|",
        ));
        let rec = SchBlanketRecord::from_origin(origin);
        assert_eq!(rec.line_width(), 1);
        assert_eq!(rec.line_style(), 0);
        assert!(!rec.collapsed());
        assert_eq!(rec.unique_id(), "ABCD1234");
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=225|LINESTYLE=0|"));
        let mut rec = SchBlanketRecord::from_origin(origin);
        rec.set_line_style(1);
        assert_eq!(rec.line_style(), 1);
        rec.set_collapsed(true);
        assert!(rec.collapsed());
    }
}

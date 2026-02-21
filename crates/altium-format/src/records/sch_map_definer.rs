//! Map definer record (RECORD=47).

use altium_format_derive::altium_record;

/// Map definer record — maps one schematic pin designator to one or more
/// implementation (footprint) pad designators.
///
/// Corresponds to `MapDefiner` (`TObjectId.eMapDefiner`, binary code 47).
#[altium_record(kind = "sch", record_id = 47, codec = "params")]
pub struct SchMapDefinerRecord {
    // --- DataObjectBase (flattened) ---
    #[altium(key = "OWNERINDEX")]
    owner_index: i32,
    #[altium(key = "ISNOTACCESIBLE")]
    is_not_accessible: bool,
    #[altium(key = "OWNERINDEXADDITIONALLIST")]
    owner_index_additional_list: bool,
    #[altium(key = "INDEXINSHEET")]
    index_in_sheet: i32,

    // --- MapDefiner-specific fields ---
    #[altium(key = "DESINTF")]
    designator_interface: String,
    #[altium(key = "DESIMPCOUNT")]
    designator_implementation_count: i32,
    // Dynamic keys (`DESIMP0`, `DESIMP1`, ...) are accessed via helper methods.
    #[altium(key = "DESIMP0")]
    designator_implementation_0: String,
}

impl SchMapDefinerRecord {
    /// Returns all implementation-side designators (`DESIMPn`) in order.
    pub fn implementation_designators(&self) -> crate::error::Result<Vec<String>> {
        use crate::traits::RecordType;
        let params = &self.origin().param().params;
        let count = self.designator_implementation_count()?.max(0) as usize;
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            let key = format!("DESIMP{}", i);
            let value = params
                .get(&key)
                .map(|v| v.as_str().to_string())
                .unwrap_or_default();
            out.push(value);
        }
        Ok(out)
    }

    /// Replaces all implementation-side designators (`DESIMPn`), updating
    /// `DESIMPCOUNT` accordingly.
    pub fn set_implementation_designators(&mut self, values: &[String]) {
        use crate::traits::RecordType;
        let params = &mut self.origin_mut().param_mut().params;

        let keys_to_remove: Vec<String> = params
            .iter()
            .map(|(k, _)| k.to_string())
            .filter(|k| {
                k.strip_prefix("DESIMP")
                    .and_then(|suffix| suffix.parse::<usize>().ok())
                    .is_some()
            })
            .collect();
        for key in keys_to_remove {
            params.remove(&key);
        }

        params.add_int("DESIMPCOUNT", values.len() as i32);
        for (idx, value) in values.iter().enumerate() {
            params.add(&format!("DESIMP{}", idx), value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip() -> crate::error::Result<()> {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=47|OWNERINDEX=24|DESINTF=3|DESIMPCOUNT=2|DESIMP0=3|DESIMP1=A3|",
        ));
        let rec = SchMapDefinerRecord::from_origin(origin);
        assert_eq!(rec.owner_index()?, 24);
        assert_eq!(rec.designator_interface()?, "3");
        assert_eq!(rec.designator_implementation_count()?, 2);
        assert_eq!(
            rec.implementation_designators()?,
            vec!["3".to_string(), "A3".to_string()]
        );
        Ok(())
    }

    #[test]
    fn setter() -> crate::error::Result<()> {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=47|DESINTF=1|DESIMPCOUNT=1|"));
        let mut rec = SchMapDefinerRecord::from_origin(origin);
        rec.set_designator_interface("P1".to_string());
        rec.set_implementation_designators(&["A1".to_string(), "B1".to_string()]);
        assert_eq!(rec.designator_interface()?, "P1");
        assert_eq!(rec.designator_implementation_count()?, 2);
        assert_eq!(
            rec.implementation_designators()?,
            vec!["A1".to_string(), "B1".to_string()]
        );
        Ok(())
    }
}

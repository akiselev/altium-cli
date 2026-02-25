use crate::ops::model::ComposedOp;

use altium_format::pcb_ops_core::{PcbDocLowOp, QueryOp};

pub fn lower_composed_to_pcbdoc_low(composed_ops: &[ComposedOp]) -> crate::Result<Vec<PcbDocLowOp>> {
    let mut out = Vec::with_capacity(composed_ops.len());
    for op in composed_ops {
        match op {
            ComposedOp::Query(v) => out.push(PcbDocLowOp::Query(QueryOp {
                opid: v.opid.clone(),
                selector: v.selector.clone(),
            })),
            ComposedOp::AddTrack(v) => out.push(PcbDocLowOp::AddTrack(v.0.clone())),
            ComposedOp::AddVia(v) => out.push(PcbDocLowOp::AddVia(v.0.clone())),
            _ => {
                return Err(crate::AltiumOperationError::Unimplemented(
                    "op is not supported for pcbdoc domain".to_owned(),
                ))
            }
        }
    }
    Ok(out)
}

use crate::ops::model::ComposedOp;

use altium_format::pcb_ops_core::{PcbLibLowOp, QueryOp};

pub fn lower_composed_to_pcblib_low(composed_ops: &[ComposedOp]) -> crate::Result<Vec<PcbLibLowOp>> {
    let mut out = Vec::with_capacity(composed_ops.len());
    for op in composed_ops {
        match op {
            ComposedOp::Query(v) => out.push(PcbLibLowOp::Query(QueryOp {
                opid: v.opid.clone(),
                selector: v.selector.clone(),
            })),
            ComposedOp::AddFootprint(v) => out.push(PcbLibLowOp::AddFootprint(v.0.clone())),
            ComposedOp::AddTrack(v) => out.push(PcbLibLowOp::AddTrack(v.0.clone())),
            ComposedOp::AddVia(v) => out.push(PcbLibLowOp::AddVia(v.0.clone())),
            ComposedOp::AddPad(v) => out.push(PcbLibLowOp::AddPad(v.0.clone())),
            _ => {
                return Err(crate::AltiumOperationError::Unimplemented(
                    "op is not supported for pcblib domain".to_owned(),
                ))
            }
        }
    }
    Ok(out)
}

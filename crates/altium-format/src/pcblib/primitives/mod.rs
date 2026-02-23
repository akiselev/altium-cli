pub(crate) mod arc;
pub(crate) mod common;
pub(crate) mod component_body;
pub(crate) mod fill;
pub(crate) mod pad;
pub(crate) mod region;
pub(crate) mod text;
pub(crate) mod track;
pub(crate) mod via;

use altium_format_types::PcbObjectId;

use crate::pcblib::PcbPrimitive;
use crate::{AltiumFormatError, Result};

/// Dispatches a single PcbLib primitive payload to the appropriate parser.
///
/// PcbLib uses single-record format: each primitive has exactly one
/// length-prefixed payload (unlike PcbDoc which uses multi-subrecord
/// format for Pad and Text).
pub(crate) fn dispatch_primitive(
    object_id: PcbObjectId,
    data: &[u8],
) -> Result<PcbPrimitive> {
    match object_id {
        PcbObjectId::Arc => arc::parse_arc(data).map(PcbPrimitive::Arc),
        PcbObjectId::Track => track::parse_track(data).map(PcbPrimitive::Track),
        PcbObjectId::Via => via::parse_via(data).map(PcbPrimitive::Via),
        PcbObjectId::Fill => fill::parse_fill(data).map(PcbPrimitive::Fill),
        PcbObjectId::Text => text::parse_text(data).map(PcbPrimitive::Text),
        PcbObjectId::Region => region::parse_region(data).map(PcbPrimitive::Region),
        PcbObjectId::Pad => pad::parse_pad(data).map(PcbPrimitive::Pad),
        PcbObjectId::ComponentBody => {
            component_body::parse_component_body(data).map(PcbPrimitive::ComponentBody)
        }
        other => Err(AltiumFormatError::UnknownObjectId(other as u8)),
    }
}

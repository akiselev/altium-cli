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

/// Dispatches PcbLib primitive subrecords to the appropriate parser.
///
/// Most primitives have a single subrecord. Pad has 6 subrecords and
/// Text has 2 subrecords — the subrecord count is determined by the
/// caller based on the object type.
pub(crate) fn dispatch_primitive(
    object_id: PcbObjectId,
    subrecords: &[&[u8]],
) -> Result<PcbPrimitive> {
    match object_id {
        PcbObjectId::Arc => arc::parse_arc(subrecords[0]).map(PcbPrimitive::Arc),
        PcbObjectId::Track => track::parse_track(subrecords[0]).map(PcbPrimitive::Track),
        PcbObjectId::Via => via::parse_via(subrecords[0]).map(PcbPrimitive::Via),
        PcbObjectId::Fill => fill::parse_fill(subrecords[0]).map(PcbPrimitive::Fill),
        PcbObjectId::Text => text::parse_text(subrecords).map(PcbPrimitive::Text),
        PcbObjectId::Region => region::parse_region(subrecords[0], false).map(PcbPrimitive::Region),
        PcbObjectId::Pad => pad::parse_pad(subrecords).map(PcbPrimitive::Pad),
        PcbObjectId::ComponentBody => component_body::parse_component_body(subrecords[0], false)
            .map(PcbPrimitive::ComponentBody),
        other => Err(AltiumFormatError::UnknownObjectId(other as u8)),
    }
}

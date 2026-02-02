//! PCB ComponentBody record (ID=12, hybrid binary+parametric).
//!
//! Same structure as Region with additional 3D-specific parametric properties:
//! STANDOFFHEIGHT, OVERALLHEIGHT, BODYPROJECTION, BODYCOLOR3D, BODYOPACITY3D,
//! IDENTIFIER, MODELID, MODELTYPE.

use super::region::PcbRegion;

/// PCB ComponentBody record.
///
/// Structurally identical to Region — uses the same 18-byte binary header,
/// parametric properties, and vertex format. The difference is the type byte
/// (12 vs 11) and the 3D-specific properties in the parametric block.
pub type PcbComponentBody = PcbRegion;

//! PCB Via binary record (246+ bytes core, multi-section).
//!
//! Framing: `u8 type(3)` + `u32 len` + data.
//!
//! Via writer (Ghidra FUN_0187fa70) produces multiple sections:
//! 1. Core via data (246 bytes) — FUN_0185b5a0
//! 2. Extended entries (N × 9 bytes with count/stride header)
//! 3. Additional section (42 bytes) — FUN_0185d0a0
//! 4. Pad layer entries (M × 30 bytes with count/stride header)
//! 5. Trailing data (9 bytes) — FUN_0185d900
//!
//! For standard vias: 246 + 8 + 42 + 8 + 9 = 313 bytes (within u32 len).

use serde::{Deserialize, Serialize};
use std::io;

use super::coord::PcbCoord;
use super::primitive::PcbCommonHeader;

/// PCB Via record.
///
/// We store the full raw data for round-trip fidelity and extract known fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PcbVia {
    /// Common header (bytes 0-12, but byte 0 layer is skipped by KiCad).
    pub header: PcbCommonHeader,
    pub position_x: PcbCoord,
    pub position_y: PcbCoord,
    pub diameter: PcbCoord,
    pub hole_size: PcbCoord,
    pub layer_start: u8,
    pub layer_end: u8,

    // Extended fields (if core >= 75 bytes)
    pub thermal_relief_airgap: Option<PcbCoord>,
    pub thermal_relief_conductor_count: Option<u8>,
    pub thermal_relief_conductor_width: Option<PcbCoord>,
    pub soldermask_expansion_front: Option<PcbCoord>,
    pub soldermask_expansion_manual: Option<bool>,
    pub via_mode: Option<u8>,

    /// Per-layer diameters (32 entries, if via_mode is pad-stack).
    pub diameter_by_layer: Option<[i32; 32]>,

    // Additional extended fields (if core >= 246 bytes)
    pub soldermask_expansion_linked: Option<bool>,
    pub soldermask_expansion_back: Option<PcbCoord>,

    // Premium fields (if core >= 307 bytes)
    pub pos_tolerance: Option<PcbCoord>,
    pub neg_tolerance: Option<PcbCoord>,

    /// Full raw data for round-trip fidelity.
    #[serde(skip)]
    raw: Vec<u8>,
}

impl PcbVia {
    /// Parse from a single subrecord's data bytes (after type + len consumed).
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        if data.len() < 31 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "via data too short"));
        }

        let rd = |off: usize| -> i32 {
            i32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
        };

        let mut r = std::io::Cursor::new(&data[..13]);
        let header = PcbCommonHeader::read_from(&mut r)?;

        let position_x = PcbCoord::from_raw(rd(13));
        let position_y = PcbCoord::from_raw(rd(17));
        let diameter = PcbCoord::from_raw(rd(21));
        let hole_size = PcbCoord::from_raw(rd(25));
        let layer_start = data[29];
        let layer_end = data[30];

        let len = data.len();

        let thermal_relief_airgap = if len > 35 { Some(PcbCoord::from_raw(rd(32))) } else { None };
        let thermal_relief_conductor_count = if len > 36 { Some(data[36]) } else { None };
        let thermal_relief_conductor_width = if len > 41 { Some(PcbCoord::from_raw(rd(38))) } else { None };
        let soldermask_expansion_front = if len > 57 { Some(PcbCoord::from_raw(rd(54))) } else { None };
        let soldermask_expansion_manual = if len > 66 { Some(data[66] & 0x02 != 0) } else { None };
        let via_mode = if len > 74 { Some(data[74]) } else { None };

        let diameter_by_layer = if len > 203 {
            let mut arr = [0i32; 32];
            for i in 0..32 {
                arr[i] = rd(75 + i * 4);
            }
            Some(arr)
        } else {
            None
        };

        let soldermask_expansion_linked = if len >= 242 { Some(data[241] & 0x01 != 0) } else { None };
        let soldermask_expansion_back = if len >= 246 { Some(PcbCoord::from_raw(rd(242))) } else { None };

        let pos_tolerance = if len >= 295 { Some(PcbCoord::from_raw(rd(291))) } else { None };
        let neg_tolerance = if len >= 299 { Some(PcbCoord::from_raw(rd(295))) } else { None };

        Ok(Self {
            header,
            position_x,
            position_y,
            diameter,
            hole_size,
            layer_start,
            layer_end,
            thermal_relief_airgap,
            thermal_relief_conductor_count,
            thermal_relief_conductor_width,
            soldermask_expansion_front,
            soldermask_expansion_manual,
            via_mode,
            diameter_by_layer,
            soldermask_expansion_linked,
            soldermask_expansion_back,
            pos_tolerance,
            neg_tolerance,
            raw: data.to_vec(),
        })
    }

    /// Serialize back to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        // Round-trip: return raw if we have it
        if !self.raw.is_empty() {
            return self.raw.clone();
        }
        // Minimal: build a 31-byte core
        let mut buf = vec![0u8; 31];
        {
            let mut cursor = std::io::Cursor::new(&mut buf[..13]);
            let _ = self.header.write_to(&mut cursor);
        }
        buf[13..17].copy_from_slice(&self.position_x.to_raw().to_le_bytes());
        buf[17..21].copy_from_slice(&self.position_y.to_raw().to_le_bytes());
        buf[21..25].copy_from_slice(&self.diameter.to_raw().to_le_bytes());
        buf[25..29].copy_from_slice(&self.hole_size.to_raw().to_le_bytes());
        buf[29] = self.layer_start;
        buf[30] = self.layer_end;
        buf
    }
}

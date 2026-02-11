//! Phase 1A — Coordinate types for V2 API.
//!
//! This module will be fully populated by Track 1A. For now, it contains
//! minimal definitions needed by other Phase 1 tracks.

/// PCB coordinate: 10,000 internal units per mil.
///
/// Used for all PCB binary record coordinates.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PcbCoord(pub(crate) i32);

impl PcbCoord {
    /// Creates a coordinate from raw internal units.
    #[inline]
    pub const fn from_raw(raw: i32) -> Self {
        PcbCoord(raw)
    }

    /// Returns the raw internal unit value.
    #[inline]
    pub const fn to_raw(self) -> i32 {
        self.0
    }
}

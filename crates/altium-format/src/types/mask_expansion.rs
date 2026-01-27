//! Mask expansion state type.

use crate::error::Result;
use crate::traits::{FromBinary, ToBinary};
use crate::types::Coord;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Read;

/// Mask expansion mode (replaces manual bool + value Coord pairs).
///
/// The type system prevents invalid states: you cannot have manual=false with a non-zero value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MaskExpansion {
    /// Automatic expansion (calculated by design rules).
    #[default]
    Auto,
    /// Manual expansion with specified value.
    Manual(Coord),
}

impl MaskExpansion {
    /// Check if manual override is active.
    pub fn is_manual(&self) -> bool {
        matches!(self, MaskExpansion::Manual(_))
    }

    /// Get the expansion value (returns zero for Auto).
    pub fn value(&self) -> Coord {
        match self {
            MaskExpansion::Auto => Coord::default(),
            MaskExpansion::Manual(v) => *v,
        }
    }
}

impl FromBinary for MaskExpansion {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let manual_flag = reader.read_u8()?;
        let value = Coord::from_raw(reader.read_i32::<LittleEndian>()?);

        // Altium binary format uses flag value 2 (not 1) for manual mode, value 0 for auto mode.
        // Verified against PCB1.PcbLib test files. Using != 0 check for robustness against
        // potential format variations while documenting observed value.
        Ok(if manual_flag != 0 {
            MaskExpansion::Manual(value)
        } else {
            MaskExpansion::Auto
        })
    }
}

impl ToBinary for MaskExpansion {
    fn write_to<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            MaskExpansion::Auto => {
                writer.write_u8(0)?;
                writer.write_i32::<LittleEndian>(0)?;
            }
            MaskExpansion::Manual(v) => {
                // Altium uses flag value 2 for manual mode (verified in test files)
                writer.write_u8(2)?;
                writer.write_i32::<LittleEndian>(v.to_raw())?;
            }
        }
        Ok(())
    }

    fn binary_size(&self) -> usize {
        5
    }
}

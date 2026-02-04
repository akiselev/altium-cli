//! PCB connection (ratsnest) record type.
//!
//! Connections represent the unrouted "ratsnest" lines between
//! pads that need to be connected but aren't yet routed.
//!
//! **DEPRECATED**: Use `v2::pcb::PcbConnectionV2` with `v2::pcb::io` instead.

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

use crate::error::Result;
use crate::traits::{FromBinary, ToBinary};
use crate::types::Coord;

/// PCB connection (ratsnest) record.
///
/// A connection represents an unrouted connection between two points
/// on the PCB. These are shown as "ratsnest" lines in the editor.
///
/// Binary format (43 bytes):
/// - Bytes 0-3: Net index (u16) + flags
/// - Bytes 4-7: Unknown (0xFF padding)
/// - Bytes 8-15: From point (X, Y as i32)
/// - Bytes 16-23: To point (X, Y as i32)
/// - Bytes 24-42: Component/pad indices and flags
///
/// **DEPRECATED**: Use `v2::pcb::PcbConnectionV2` instead.
#[deprecated(note = "Use v2::pcb::PcbConnectionV2")]
#[derive(Debug, Clone, Default)]
pub struct PcbConnection {
    /// Net index this connection belongs to.
    pub net_index: u16,
    /// From X coordinate.
    pub from_x: Coord,
    /// From Y coordinate.
    pub from_y: Coord,
    /// To X coordinate.
    pub to_x: Coord,
    /// To Y coordinate.
    pub to_y: Coord,
    /// From component index (-1 if none).
    pub from_component_index: i16,
    /// From pad index (-1 if none).
    pub from_pad_index: i16,
    /// To component index (-1 if none).
    pub to_component_index: i16,
    /// To pad index (-1 if none).
    pub to_pad_index: i16,
    /// Raw data for round-tripping unknown fields.
    pub raw_data: Vec<u8>,
}

impl PcbConnection {
    /// Binary record size.
    pub const SIZE: usize = 43;
}

impl FromBinary for PcbConnection {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        // Read the entire record
        let mut data = vec![0u8; Self::SIZE];
        reader.read_exact(&mut data)?;

        let mut cursor = std::io::Cursor::new(&data);

        // Parse net index (bytes 0-1)
        let net_index = cursor.read_u16::<LittleEndian>()?;

        // Skip bytes 2-7 (flags and padding)
        cursor.set_position(8);

        // Parse from point (bytes 8-15)
        let from_x = Coord::from_raw(cursor.read_i32::<LittleEndian>()?);
        let from_y = Coord::from_raw(cursor.read_i32::<LittleEndian>()?);

        // Parse to point (bytes 16-23)
        let to_x = Coord::from_raw(cursor.read_i32::<LittleEndian>()?);
        let to_y = Coord::from_raw(cursor.read_i32::<LittleEndian>()?);

        // Skip to component/pad indices (bytes 28+)
        cursor.set_position(28);

        // Component/pad indices appear to be near the end
        // Format varies, store raw data for now
        let from_component_index = cursor.read_i16::<LittleEndian>().unwrap_or(-1);
        cursor.set_position(32);
        let from_pad_index = cursor.read_i16::<LittleEndian>().unwrap_or(-1);
        cursor.set_position(35);
        let to_component_index = cursor.read_i16::<LittleEndian>().unwrap_or(-1);
        cursor.set_position(39);
        let to_pad_index = cursor.read_i16::<LittleEndian>().unwrap_or(-1);

        Ok(PcbConnection {
            net_index,
            from_x,
            from_y,
            to_x,
            to_y,
            from_component_index,
            from_pad_index,
            to_component_index,
            to_pad_index,
            raw_data: data,
        })
    }
}

impl ToBinary for PcbConnection {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        // If we have raw data, write it as-is for round-tripping
        if self.raw_data.len() == Self::SIZE {
            writer.write_all(&self.raw_data)?;
        } else {
            // Create new record
            let mut data = vec![0u8; Self::SIZE];
            let mut cursor = std::io::Cursor::new(&mut data[..]);

            cursor.write_u16::<LittleEndian>(self.net_index)?;

            // Fill padding with 0xFF
            cursor.set_position(4);
            cursor.write_all(&[0xFF; 4])?;

            // Write coordinates
            cursor.write_i32::<LittleEndian>(self.from_x.to_raw())?;
            cursor.write_i32::<LittleEndian>(self.from_y.to_raw())?;
            cursor.write_i32::<LittleEndian>(self.to_x.to_raw())?;
            cursor.write_i32::<LittleEndian>(self.to_y.to_raw())?;

            writer.write_all(&data)?;
        }
        Ok(())
    }

    fn binary_size(&self) -> usize {
        Self::SIZE
    }
}

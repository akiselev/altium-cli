//! Binary serialization traits for PCB records.

use crate::error::Result;
use std::io::{Read, Write};

/// Import a type from a binary stream.
///
/// This trait is automatically implemented by the `AltiumRecord` derive macro
/// for binary-based record types (PCB records).
pub trait FromBinary: Sized {
    /// Read this type from a binary stream.
    fn read_from<R: Read>(reader: &mut R) -> Result<Self>;

    /// Read this type and preserve remaining bytes for non-destructive editing.
    ///
    /// The `block_size` parameter indicates the total size of the data block,
    /// allowing remaining bytes to be captured.
    fn read_from_preserving<R: Read>(
        reader: &mut R,
        _block_size: usize,
    ) -> Result<(Self, Vec<u8>)> {
        let value = Self::read_from(reader)?;
        Ok((value, Vec::new()))
    }
}

/// Export a type to a binary stream.
///
/// This trait is automatically implemented by the `AltiumRecord` derive macro
/// for binary-based record types (PCB records).
pub trait ToBinary {
    /// Write this type to a binary stream.
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()>;

    /// Calculate the binary size of this type.
    ///
    /// Used for writing block headers that include size information.
    fn binary_size(&self) -> usize;
}

// Basic type implementations

impl FromBinary for i8 {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        use byteorder::ReadBytesExt;
        Ok(reader.read_i8()?)
    }
}

impl ToBinary for i8 {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        use byteorder::WriteBytesExt;
        writer.write_i8(*self)?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        1
    }
}

impl FromBinary for u8 {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        use byteorder::ReadBytesExt;
        Ok(reader.read_u8()?)
    }
}

impl ToBinary for u8 {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(*self)?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        1
    }
}

impl FromBinary for i16 {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(reader.read_i16::<LittleEndian>()?)
    }
}

impl ToBinary for i16 {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_i16::<LittleEndian>(*self)?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        2
    }
}

impl FromBinary for u16 {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(reader.read_u16::<LittleEndian>()?)
    }
}

impl ToBinary for u16 {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u16::<LittleEndian>(*self)?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        2
    }
}

impl FromBinary for i32 {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(reader.read_i32::<LittleEndian>()?)
    }
}

impl ToBinary for i32 {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_i32::<LittleEndian>(*self)?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        4
    }
}

impl FromBinary for u32 {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(reader.read_u32::<LittleEndian>()?)
    }
}

impl ToBinary for u32 {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u32::<LittleEndian>(*self)?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        4
    }
}

impl FromBinary for i64 {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(reader.read_i64::<LittleEndian>()?)
    }
}

impl ToBinary for i64 {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_i64::<LittleEndian>(*self)?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        8
    }
}

impl FromBinary for u64 {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(reader.read_u64::<LittleEndian>()?)
    }
}

impl ToBinary for u64 {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u64::<LittleEndian>(*self)?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        8
    }
}

impl FromBinary for f32 {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(reader.read_f32::<LittleEndian>()?)
    }
}

impl ToBinary for f32 {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_f32::<LittleEndian>(*self)?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        4
    }
}

impl FromBinary for f64 {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(reader.read_f64::<LittleEndian>()?)
    }
}

impl ToBinary for f64 {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_f64::<LittleEndian>(*self)?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        8
    }
}

impl FromBinary for bool {
    fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        use byteorder::ReadBytesExt;
        Ok(reader.read_u8()? != 0)
    }
}

impl ToBinary for bool {
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(if *self { 1 } else { 0 })?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        1
    }
}

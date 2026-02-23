use altium_format_types::{PcbFlags, V6Layer};

use crate::binary_io::BinaryReader;
use crate::pcblib::PcbPrimitiveCommon;
use crate::Result;

pub(crate) fn parse_common_header(reader: &mut BinaryReader) -> Result<PcbPrimitiveCommon> {
    let layer_byte = reader.read_u8()?;
    let layer = V6Layer::try_from(layer_byte)?;
    let pad_byte = reader.read_u8()?;
    let flags_raw = reader.read_u16_le()?;
    let flags = PcbFlags::new(flags_raw);
    let net_index = reader.read_i32_le()?;
    let polygon_index = reader.read_u16_le()?;
    let component_index = reader.read_u16_le()?;
    let unknown = reader.read_u8()?;
    Ok(PcbPrimitiveCommon {
        layer,
        pad_byte,
        flags,
        net_index,
        polygon_index,
        component_index,
        unknown,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_io::BinaryWriter;

    #[test]
    fn parse_common_header_known_bytes() {
        let mut w = BinaryWriter::new();
        w.write_u8(1);  // layer = TopCopper (V6Layer::TopLayer)
        w.write_u8(0);  // pad_byte
        w.write_u16_le(0x0000); // flags
        w.write_i32_le(-1); // net_index = -1 (no net)
        w.write_u16_le(0xFFFF); // polygon_index
        w.write_u16_le(0x0000); // component_index
        w.write_u8(0);  // unknown
        let data = w.finish();
        let mut reader = BinaryReader::new(&data);
        let common = parse_common_header(&mut reader).unwrap();
        reader.assert_exhausted().unwrap();
        assert_eq!(common.pad_byte, 0);
        assert_eq!(common.net_index, -1);
        assert_eq!(common.polygon_index, 0xFFFF);
        assert_eq!(common.component_index, 0x0000);
        assert_eq!(common.unknown, 0);
    }
}

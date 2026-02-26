use altium_format_types::{PcbFlags, V6Layer};

use crate::Result;
use crate::binary_io::BinaryReader;
use crate::pcblib::PcbPrimitiveCommon;

pub(crate) fn parse_common_header(reader: &mut BinaryReader) -> Result<PcbPrimitiveCommon> {
    let layer = V6Layer::try_from(reader.read_u8()?)?;
    let flags = PcbFlags::new(reader.read_u16_le()?);
    let net_index = reader.read_u16_le()?;
    let polygon_index = reader.read_u16_le()?;
    let component_index = reader.read_u16_le()?;
    let coordinate_index = reader.read_u16_le()?;
    let dimension_index = reader.read_u16_le()?;
    Ok(PcbPrimitiveCommon {
        layer,
        flags,
        net_index,
        polygon_index,
        component_index,
        coordinate_index,
        dimension_index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_io::BinaryWriter;

    #[test]
    fn parse_common_header_known_bytes() {
        let mut w = BinaryWriter::new();
        w.write_u8(1); // layer = TopLayer
        w.write_u16_le(0x0000); // flags
        w.write_u16_le(0xFFFF); // net_index = none
        w.write_u16_le(0xFFFF); // polygon_index = none
        w.write_u16_le(0xFFFF); // component_index = none
        w.write_u16_le(0xFFFF); // coordinate_index = none
        w.write_u16_le(0xFFFF); // dimension_index = none
        let data = w.finish();
        let mut reader = BinaryReader::new(&data);
        let common = parse_common_header(&mut reader).unwrap();
        reader.assert_exhausted().unwrap();
        assert_eq!(common.net_index, 0xFFFF);
        assert_eq!(common.polygon_index, 0xFFFF);
        assert_eq!(common.component_index, 0xFFFF);
        assert_eq!(common.coordinate_index, 0xFFFF);
        assert_eq!(common.dimension_index, 0xFFFF);
    }
}

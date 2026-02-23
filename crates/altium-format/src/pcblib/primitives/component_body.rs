use crate::pcblib::PcbComponentBody;
use crate::{AltiumFormatError, Result};

pub(crate) fn parse_component_body(data: &[u8]) -> Result<PcbComponentBody> {
    Err(AltiumFormatError::InvalidParamValue {
        key: "ComponentBody".to_owned(),
        detail: format!(
            "ComponentBody parser not yet implemented (record is {} bytes); \
             run investigation with `altium cfb dump` before implementing",
            data.len()
        ),
    })
}

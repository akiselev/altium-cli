use altium_format_types::pcb::PcbFileFormatVersion;

pub trait Document {
    fn version(&self) -> PcbFileFormatVersion;
}

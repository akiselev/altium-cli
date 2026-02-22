pub trait PcbDocOps {
    fn validate(&self) -> crate::Result<()>;
}

impl PcbDocOps for altium_format::PcbDoc {
    fn validate(&self) -> crate::Result<()> {
        Err(crate::AltiumOperationError::Unimplemented(
            "PcbDocOps::validate is not implemented yet".to_string(),
        ))
    }
}

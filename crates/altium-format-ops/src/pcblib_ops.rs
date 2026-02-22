pub trait PcbLibOps {
    fn validate(&self) -> crate::Result<()>;
}

impl PcbLibOps for altium_format::PcbLib {
    fn validate(&self) -> crate::Result<()> {
        Err(crate::AltiumOperationError::Unimplemented(
            "PcbLibOps::validate is not implemented yet".to_string(),
        ))
    }
}

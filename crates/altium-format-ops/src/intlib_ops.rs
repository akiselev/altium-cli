pub trait IntLibOps {
    fn validate(&self) -> crate::Result<()>;
}

impl IntLibOps for altium_format::IntLib {
    fn validate(&self) -> crate::Result<()> {
        Err(crate::AltiumOperationError::Unimplemented(
            "IntLib validation is not implemented yet".to_string(),
        ))
    }
}

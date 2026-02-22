pub trait AltiumProjectOps {
    fn validate(&self) -> crate::Result<()>;
}

impl AltiumProjectOps for altium_format::AltiumProject {
    fn validate(&self) -> crate::Result<()> {
        Err(crate::AltiumOperationError::Unimplemented(
            "AltiumProjectOps::validate is not implemented yet".to_string(),
        ))
    }
}

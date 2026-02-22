pub trait SchDocOps {
    fn validate(&self) -> crate::Result<()>;
}

impl SchDocOps for altium_format::SchDoc {
    fn validate(&self) -> crate::Result<()> {
        Err(crate::AltiumOperationError::Unimplemented(
            "SchDocOps::validate is not implemented yet".to_string(),
        ))
    }
}

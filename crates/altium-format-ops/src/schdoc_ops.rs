pub trait SchDocOps {
    fn validate(&self) -> crate::Result<()>;
}

impl SchDocOps for altium_format::SchDoc {
    fn validate(&self) -> crate::Result<()> {
        self.validate_invariants()?;
        Ok(())
    }
}

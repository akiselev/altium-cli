use std::path::Path;

pub trait SchDocOps {
    fn validate(&self) -> crate::Result<()>;
    fn save_as(&self, output: &Path) -> crate::Result<()>;
}

impl SchDocOps for altium_format::SchDoc {
    fn validate(&self) -> crate::Result<()> {
        self.validate_invariants()?;
        Ok(())
    }

    fn save_as(&self, output: &Path) -> crate::Result<()> {
        self.save(output)?;
        Ok(())
    }
}

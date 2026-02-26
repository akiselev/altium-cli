use std::path::Path;

pub trait PcbDocOps {
    fn validate(&self) -> crate::Result<()>;
    fn save_as(&self, output: &Path) -> crate::Result<()>;
}

impl PcbDocOps for altium_format::PcbDoc {
    fn validate(&self) -> crate::Result<()> {
        self.validate_invariants()?;
        Ok(())
    }

    fn save_as(&self, output: &Path) -> crate::Result<()> {
        self.save(output)?;
        Ok(())
    }
}

use std::path::Path;

pub trait SchLibOps {
    fn validate(&self) -> crate::Result<()>;
    fn save_as(&self, output: &Path) -> crate::Result<()>;
}

impl SchLibOps for altium_format::SchLib {
    fn validate(&self) -> crate::Result<()> {
        Ok(())
    }

    fn save_as(&self, output: &Path) -> crate::Result<()> {
        self.save(output)?;
        Ok(())
    }
}

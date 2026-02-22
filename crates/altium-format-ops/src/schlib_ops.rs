pub trait SchLibOps {
    fn validate(&self) -> crate::Result<()>;
}

impl SchLibOps for altium_format::SchLib {
    fn validate(&self) -> crate::Result<()> {
        Ok(())
    }
}

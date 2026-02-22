pub trait PcbDocOps {
    fn validate(&self) -> crate::Result<()>;
}

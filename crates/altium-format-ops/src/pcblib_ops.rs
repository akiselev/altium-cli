pub trait PcbLibOps {
    fn validate(&self) -> crate::Result<()>;
}

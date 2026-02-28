use std::path::Path;

pub struct IntLib {
    // TODO: Define the structure
}

impl IntLib {
    pub fn open(_path: impl AsRef<Path>) -> crate::Result<Self> {
        Err(crate::AltiumFormatError::NotImplemented("IntLib".into()))
    }
}

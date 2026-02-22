use std::path::Path;

pub struct AltiumProject {
    // TODO: Define the structure
}

impl AltiumProject {
    pub fn open(path: impl AsRef<Path>) -> crate::Result<Self> {
        let path = path.as_ref();
        let _file = std::fs::File::open(path)?;
        Ok(Self {})
    }
}

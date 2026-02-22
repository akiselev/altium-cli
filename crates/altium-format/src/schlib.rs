use std::path::Path;

pub struct SchLib {
    // TODO: Define the structure
}

impl SchLib {
    pub fn open(path: impl AsRef<Path>) -> crate::Result<Self> {
        let path = path.as_ref();
        let _file = std::fs::File::open(path)?;
        Ok(Self {})
    }
}

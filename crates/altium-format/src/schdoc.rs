use std::path::Path;

pub struct SchDoc {
    // TODO: Define the structure
}

impl SchDoc {
    pub fn open(path: impl AsRef<Path>) -> crate::Result<Self> {
        let path = path.as_ref();
        // Verify the file exists and is readable
        let _file = std::fs::File::open(path)?;
        Ok(Self {})
    }
}

use std::path::Path;

use crate::VersionInfo;

pub trait SchLibOps {
    fn validate(&self) -> crate::Result<()>;
    fn save_as(&self, output: &Path) -> crate::Result<()>;
    fn version(&self) -> crate::Result<VersionInfo>;
}

impl SchLibOps for altium_format::SchLib {
    fn validate(&self) -> crate::Result<()> {
        self.validate_invariants()?;
        Ok(())
    }

    fn save_as(&self, output: &Path) -> crate::Result<()> {
        self.save(output)?;
        Ok(())
    }

    fn version(&self) -> crate::Result<VersionInfo> {
        Ok(VersionInfo {
            header: self.version_header().to_owned(),
            minor_version: self.minor_version(),
            file_version_info: self.file_version_info().map(|s| s.to_owned()),
        })
    }
}

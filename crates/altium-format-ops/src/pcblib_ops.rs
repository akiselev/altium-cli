use crate::VersionInfo;
use std::path::Path;

pub trait PcbLibOps {
    fn validate(&self) -> crate::Result<()>;
    fn version(&self) -> crate::Result<VersionInfo>;
    fn save_as(&self, output: &Path) -> crate::Result<()>;
}

impl PcbLibOps for altium_format::PcbLib {
    fn validate(&self) -> crate::Result<()> {
        Ok(())
    }

    fn version(&self) -> crate::Result<VersionInfo> {
        Ok(VersionInfo {
            header: self.version_header().to_owned(),
            minor_version: self.minor_version() as i32,
            file_version_info: self.file_version_info().map(|s| s.to_owned()),
        })
    }

    fn save_as(&self, output: &Path) -> crate::Result<()> {
        Ok(self.save(output)?)
    }
}

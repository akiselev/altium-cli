use std::path::Path;

pub trait PcbDocOps {
    fn validate(&self) -> crate::Result<()>;
    fn save_as(&self, output: &Path) -> crate::Result<()>;
}

impl PcbDocOps for altium_format::PcbDoc {
    fn validate(&self) -> crate::Result<()> {
        const EXPECTED_PCBDOC_HEADER: &str = "PCB 6.0 Binary File";
        if self.version_header() != EXPECTED_PCBDOC_HEADER {
            return Err(crate::AltiumOperationError::AltiumFormat(
                altium_format::AltiumFormatError::InvalidParamValue {
                    key: "FileHeaderSix".to_owned(),
                    detail: format!(
                        "expected header {}, got {}",
                        EXPECTED_PCBDOC_HEADER,
                        self.version_header()
                    ),
                },
            ));
        }

        if self.minor_version() <= 0.0 {
            return Err(crate::AltiumOperationError::AltiumFormat(
                altium_format::AltiumFormatError::InvalidParamValue {
                    key: "FileHeaderSix.minor_version".to_owned(),
                    detail: format!(
                        "expected positive minor version, got {}",
                        self.minor_version()
                    ),
                },
            ));
        }

        Ok(())
    }

    fn save_as(&self, output: &Path) -> crate::Result<()> {
        self.save(output)?;
        Ok(())
    }
}

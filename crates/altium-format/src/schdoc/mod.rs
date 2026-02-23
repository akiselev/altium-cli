mod dispatch;
mod fileheader;
mod types;

use std::path::Path;

use altium_format_types::constants::record_structure::{HEADER, RECORD, RECORD_EX, WEIGHT};
use altium_format_types::constants::streams::{
    ADDITIONAL, FILES, FILE_HEADER, HARNESS_CONNECTION_POINT_CONNECTOR, OBJECT_DEFINITIONS,
    REUSE_BLOCK_INFOS, REUSE_BLOCKS, REUSE_BLOCKS_V2, STORAGE,
};

use crate::block_stream::{BlockFormat, parse_blocks};
use crate::embedded_object::parse_embedded_object_stream;
use crate::param_collection::ParameterCollection;
use crate::schdoc::dispatch::dispatch_record_type;
use crate::schdoc::fileheader::parse_fileheader_stream;
use crate::schdoc::types::{SchDocEmbeddedObject, SchDocHeaderMetadata};
use crate::tracked_cfb::TrackedCfbDocument;
use crate::{AltiumFormatError, Result, ResultExt};

pub use types::SchDoc;

impl SchDoc {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut tracked =
            TrackedCfbDocument::open(path.as_ref()).context("opening SchDoc CFB container")?;

        let (root_storages, root_streams) = tracked
            .list_entries("/")
            .context("listing root CFB entries")?;
        if !root_storages.is_empty() {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "/".to_owned(),
                detail: format!(
                    "SchDoc root must not contain storages, found: {}",
                    root_storages.join(", ")
                ),
            });
        }

        for stream in &root_streams {
            match stream.as_str() {
                FILE_HEADER | STORAGE | ADDITIONAL => {}
                OBJECT_DEFINITIONS
                | REUSE_BLOCK_INFOS
                | REUSE_BLOCKS
                | REUSE_BLOCKS_V2
                | HARNESS_CONNECTION_POINT_CONNECTOR
                | FILES => {
                    return Err(AltiumFormatError::InvalidParamValue {
                        key: stream.clone(),
                        detail: "optional SchDoc stream is present but not implemented yet"
                            .to_owned(),
                    });
                }
                _ => {
                    return Err(AltiumFormatError::InvalidParamValue {
                        key: stream.clone(),
                        detail: "unexpected top-level stream for SchDoc".to_owned(),
                    });
                }
            }
        }

        let fileheader_data = tracked
            .read_stream("/FileHeader")
            .context("reading /FileHeader")?;
        let parsed_fileheader =
            parse_fileheader_stream(&fileheader_data).context("parsing /FileHeader")?;

        let embedded_objects = if root_streams.iter().any(|s| s == STORAGE) {
            parse_storage_stream(
                &tracked
                    .read_stream("/Storage")
                    .context("reading /Storage")?,
            )
            .context("parsing /Storage")?
        } else {
            Vec::new()
        };

        let additional_records = if root_streams.iter().any(|s| s == ADDITIONAL) {
            parse_additional_stream(
                &tracked
                    .read_stream("/Additional")
                    .context("reading /Additional")?,
            )
            .context("parsing /Additional")?
        } else {
            Vec::new()
        };

        tracked
            .assert_all_consumed()
            .context("validating SchDoc stream consumption")?;

        Ok(Self {
            header: SchDocHeaderMetadata {
                header: parsed_fileheader.header.header,
                weight: parsed_fileheader.header.weight,
                minor_version: parsed_fileheader.header.minor_version,
                unique_id: parsed_fileheader.header.unique_id,
            },
            records: parsed_fileheader.records,
            additional_records,
            embedded_objects,
        })
    }
}

fn parse_storage_stream(data: &[u8]) -> Result<Vec<SchDocEmbeddedObject>> {
    let blocks = parse_blocks(data).context("parsing /Storage block stream")?;
    let entries = parse_embedded_object_stream(&blocks).context("decoding /Storage entries")?;

    Ok(entries
        .into_iter()
        .map(|e| SchDocEmbeddedObject {
            id: e.id,
            data: e.inner_data,
        })
        .collect())
}

fn parse_additional_stream(data: &[u8]) -> Result<Vec<crate::sch_records::SchRecord>> {
    let blocks = parse_blocks(data).context("parsing /Additional block stream")?;
    if blocks.is_empty() {
        return Err(AltiumFormatError::InvalidParamValue {
            key: ADDITIONAL.to_owned(),
            detail: "stream has no blocks".to_owned(),
        });
    }
    if blocks[0].format != BlockFormat::Text {
        return Err(AltiumFormatError::InvalidParamValue {
            key: ADDITIONAL.to_owned(),
            detail: "header block must be text".to_owned(),
        });
    }

    let mut header_params =
        ParameterCollection::from_bytes(&blocks[0].data).context("parsing /Additional header")?;
    let header: String = header_params
        .remove_required(HEADER)
        .context("reading /Additional HEADER")?;
    let weight: usize = header_params.remove_with_default(WEIGHT, 0usize)?;
    header_params
        .assert_exhausted()
        .context("/Additional header has unknown parameters")?;

    let mut records = Vec::with_capacity(weight);
    for (idx, block) in blocks.iter().enumerate().skip(1) {
        if block.format != BlockFormat::Text {
            return Err(AltiumFormatError::InvalidParamValue {
                key: ADDITIONAL.to_owned(),
                detail: format!("record block #{idx} must be text"),
            });
        }

        let mut params = ParameterCollection::from_bytes(&block.data)
            .with_context(|| format!("parsing /Additional block #{idx}"))?;
        let record_raw: i32 = params
            .remove_required(RECORD)
            .with_context(|| format!("/Additional block #{idx} missing RECORD"))?;
        let record_type_val = if record_raw == 254 {
            params
                .remove_required::<i32>(RECORD_EX)
                .with_context(|| format!("/Additional block #{idx} missing RECORDEX"))?
        } else {
            record_raw
        };

        let record = dispatch_record_type(record_type_val, &mut params).with_context(|| {
            format!("dispatching /Additional block #{idx} RECORD={record_type_val}")
        })?;
        params.assert_exhausted().with_context(|| {
            format!("/Additional block #{idx} RECORD={record_type_val} has unknown parameters")
        })?;
        records.push(record);
    }

    if records.len() != weight {
        return Err(AltiumFormatError::RecordCountMismatch {
            section: format!("/Additional ({header})"),
            expected: weight,
            actual: records.len(),
        });
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schdoc_fixture_path(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/schdoc")
            .join(name)
    }

    #[test]
    fn open_schdoc_fixture_reaches_parser_path() {
        let path = schdoc_fixture_path("myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc");
        match SchDoc::open(&path) {
            Ok(_) => {}
            Err(AltiumFormatError::Io(e)) => panic!("unexpected IO error while opening fixture: {e}"),
            Err(_) => {}
        }
    }
}

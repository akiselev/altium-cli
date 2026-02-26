use altium_format_types::SchRecordType;
use altium_format_types::constants::file_headers::SCH_SHEET_BINARY_HEADER_V50;
use altium_format_types::constants::record_structure::UNIQUE_ID;
use altium_format_types::constants::record_structure::{HEADER, RECORD, RECORD_EX, WEIGHT};
use altium_format_types::constants::sheet::MINOR_VERSION;

use crate::block_stream::{BlockFormat, parse_blocks};
use crate::param_collection::ParameterCollection;
use crate::sch_records::SchRecord;
use crate::schdoc::dispatch::dispatch_record_type;
use crate::schdoc::types::SchDocHeaderMetadata;
use crate::{AltiumFormatError, Result, ResultExt};

#[derive(Debug)]
pub(crate) struct ParsedFileHeader {
    pub header: SchDocHeaderMetadata,
    pub records: Vec<SchRecord>,
}

pub(crate) fn parse_fileheader_stream(data: &[u8]) -> Result<ParsedFileHeader> {
    let blocks = parse_blocks(data).context("parsing /FileHeader block stream")?;
    if blocks.is_empty() {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "FileHeader".to_owned(),
            detail: "stream has no blocks".to_owned(),
        });
    }

    let mut header_params = match blocks[0].format {
        BlockFormat::Text => ParameterCollection::from_bytes(&blocks[0].data)
            .context("parsing /FileHeader block #0")?,
        BlockFormat::Binary => {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "FileHeader".to_owned(),
                detail: "block #0 must be text".to_owned(),
            });
        }
    };

    let header: String = header_params
        .remove_required(HEADER)
        .context("reading /FileHeader HEADER")?;
    if header != SCH_SHEET_BINARY_HEADER_V50 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: HEADER.to_owned(),
            detail: format!(
                "expected {:?}, got {:?}",
                SCH_SHEET_BINARY_HEADER_V50, header
            ),
        });
    }

    let weight: i32 = header_params
        .remove_required(WEIGHT)
        .context("reading /FileHeader Weight")?;
    if weight < 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: WEIGHT.to_owned(),
            detail: format!("Weight must be non-negative, got {weight}"),
        });
    }

    // Older files may omit MinorVersion entirely.
    let minor_version: i32 = header_params
        .remove_optional::<i32>(MINOR_VERSION)?
        .unwrap_or(0);
    let unique_id: String = header_params
        .remove_with_default(UNIQUE_ID, String::new())
        .context("reading /FileHeader UniqueID")?;

    let mut records = Vec::with_capacity(weight as usize);

    if let Some(record_raw) = header_params.remove_optional::<i32>(RECORD)? {
        let record_type_val = if record_raw == 254 {
            header_params.remove_required::<i32>(RECORD_EX)?
        } else {
            record_raw
        };
        let parsed =
            dispatch_record_type(record_type_val, &mut header_params).with_context(|| {
                format!("dispatching /FileHeader block #0 RECORD={record_type_val}")
            })?;
        header_params.assert_exhausted().with_context(|| {
            format!("/FileHeader block #0 RECORD={record_type_val} has unknown parameters")
        })?;
        records.push(parsed);
    } else {
        header_params
            .assert_exhausted()
            .context("/FileHeader block #0 has unknown parameters")?;
    }

    for (block_idx, block) in blocks.iter().enumerate().skip(1) {
        if block.format != BlockFormat::Text {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "FileHeader".to_owned(),
                detail: format!("block #{block_idx} must be text, found binary"),
            });
        }

        let mut params = ParameterCollection::from_bytes(&block.data)
            .with_context(|| format!("parsing /FileHeader block #{block_idx}"))?;
        let record_raw: i32 = params
            .remove_required(RECORD)
            .with_context(|| format!("/FileHeader block #{block_idx} missing RECORD"))?;
        let record_type_val = if record_raw == 254 {
            params
                .remove_required::<i32>(RECORD_EX)
                .with_context(|| format!("/FileHeader block #{block_idx} missing RECORDEX"))?
        } else {
            record_raw
        };

        let record = dispatch_record_type(record_type_val, &mut params).with_context(|| {
            format!("dispatching /FileHeader block #{block_idx} RECORD={record_type_val}")
        })?;
        params.assert_exhausted().with_context(|| {
            format!(
                "/FileHeader block #{block_idx} RECORD={record_type_val} has unknown parameters"
            )
        })?;

        records.push(record);
    }

    if records.len() != weight as usize {
        return Err(AltiumFormatError::RecordCountMismatch {
            section: "/FileHeader".to_owned(),
            expected: weight as usize,
            actual: records.len(),
        });
    }

    let Some(first_record) = records.first() else {
        return Err(AltiumFormatError::InvalidParamValue {
            key: WEIGHT.to_owned(),
            detail: "Weight indicates no records; expected at least the Sheet record".to_owned(),
        });
    };
    if !matches!(first_record, SchRecord::Sheet(_)) {
        let actual = record_type_of(first_record) as i32;
        return Err(AltiumFormatError::InvalidParamValue {
            key: RECORD.to_owned(),
            detail: format!("first content record must be RECORD=31 (Sheet), got RECORD={actual}"),
        });
    }

    Ok(ParsedFileHeader {
        header: SchDocHeaderMetadata {
            header,
            weight,
            minor_version,
            unique_id,
        },
        records,
    })
}

fn record_type_of(record: &SchRecord) -> SchRecordType {
    match record {
        SchRecord::Sheet(_) => SchRecordType::Sheet,
        SchRecord::Template(_) => SchRecordType::Template,
        SchRecord::Wire(_) => SchRecordType::Wire,
        SchRecord::Bus(_) => SchRecordType::Bus,
        SchRecord::NetLabel(_) => SchRecordType::NetLabel,
        SchRecord::PowerObject(_) => SchRecordType::PowerObject,
        SchRecord::Port(_) => SchRecordType::Port,
        SchRecord::NoConnect(_) => SchRecordType::NoErc,
        SchRecord::Junction(_) => SchRecordType::Junction,
        SchRecord::SheetName(_) => SchRecordType::SheetName,
        SchRecord::SheetFileName(_) => SchRecordType::SheetFileName,
        SchRecord::SheetSymbol(_) => SchRecordType::SheetSymbol,
        SchRecord::SheetEntry(_) => SchRecordType::SheetEntry,
        SchRecord::BusEntry(_) => SchRecordType::BusEntry,
        SchRecord::ParameterSet(_) => SchRecordType::ParameterSet,
        SchRecord::Note(_) => SchRecordType::Note,
        SchRecord::Probe(_) => SchRecordType::Probe,
        SchRecord::CompileMask(_) => SchRecordType::CompileMask,
        SchRecord::Blanket(_) => SchRecordType::Blanket,
        SchRecord::Component(_) => SchRecordType::Component,
        SchRecord::Pin(_) => SchRecordType::Pin,
        SchRecord::Symbol(_) => SchRecordType::Symbol,
        SchRecord::Line(_) => SchRecordType::Line,
        SchRecord::Rectangle(_) => SchRecordType::Rectangle,
        SchRecord::RoundRectangle(_) => SchRecordType::RoundRectangle,
        SchRecord::Arc(_) => SchRecordType::Arc,
        SchRecord::EllipticalArc(_) => SchRecordType::EllipticalArc,
        SchRecord::Ellipse(_) => SchRecordType::Ellipse,
        SchRecord::Pie(_) => SchRecordType::Pie,
        SchRecord::Polyline(_) => SchRecordType::Polyline,
        SchRecord::Polygon(_) => SchRecordType::Polygon,
        SchRecord::Bezier(_) => SchRecordType::Bezier,
        SchRecord::Image(_) => SchRecordType::Image,
        SchRecord::Label(_) => SchRecordType::Label,
        SchRecord::Hyperlink(_) => SchRecordType::Hyperlink,
        SchRecord::Designator(_) => SchRecordType::Designator,
        SchRecord::Parameter(_) => SchRecordType::Parameter,
        SchRecord::TextFrame(_) => SchRecordType::TextFrame,
        SchRecord::ImplementationList(_) => SchRecordType::ImplementationList,
        SchRecord::Implementation(_) => SchRecordType::Implementation,
        SchRecord::ImplementationMap(_) => SchRecordType::ImplementationMap,
        SchRecord::MapDefiner(_) => SchRecordType::MapDefiner,
        SchRecord::ParameterList(_) => SchRecordType::ParameterList,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_stream::{write_binary_block, write_text_block};

    fn text_payload(s: &str) -> Vec<u8> {
        let mut out = s.as_bytes().to_vec();
        out.push(0);
        out
    }

    #[test]
    fn parses_minimal_fileheader() {
        let mut stream = write_text_block(&text_payload(
            "|HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0|Weight=1|MinorVersion=2|UniqueID=ABCDEFGH|",
        ));
        stream.extend_from_slice(&write_text_block(&text_payload(
            "|RECORD=31|FontIdCount=1|Size1=10|FontName1=Arial|",
        )));

        let parsed = parse_fileheader_stream(&stream).expect("minimal SchDoc header should parse");
        assert_eq!(parsed.header.weight, 1);
        assert_eq!(parsed.records.len(), 1);
        assert!(matches!(parsed.records[0], SchRecord::Sheet(_)));
    }

    #[test]
    fn rejects_wrong_header_string() {
        let mut stream = write_text_block(&text_payload(
            "|HEADER=Wrong Header|Weight=1|MinorVersion=2|UniqueID=ABCDEFGH|",
        ));
        stream.extend_from_slice(&write_text_block(&text_payload(
            "|RECORD=31|FontIdCount=1|Size1=10|FontName1=Arial|",
        )));

        let err = parse_fileheader_stream(&stream).expect_err("must reject invalid HEADER");
        assert!(matches!(err, AltiumFormatError::InvalidParamValue { key, .. } if key == HEADER));
    }

    #[test]
    fn rejects_when_first_record_is_not_sheet() {
        let mut stream = write_text_block(&text_payload(
            "|HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0|Weight=1|MinorVersion=2|UniqueID=ABCDEFGH|",
        ));
        stream.extend_from_slice(&write_text_block(&text_payload("|RECORD=41|")));

        let err =
            parse_fileheader_stream(&stream).expect_err("must require RECORD=31 as first record");
        assert!(matches!(err, AltiumFormatError::InvalidParamValue { key, .. } if key == RECORD));
    }

    #[test]
    fn parses_header_without_minor_version() {
        let mut stream = write_text_block(&text_payload(
            "|HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0|Weight=1|UniqueID=ABCDEFGH|",
        ));
        stream.extend_from_slice(&write_text_block(&text_payload(
            "|RECORD=31|FontIdCount=1|Size1=10|FontName1=Arial|",
        )));

        let parsed =
            parse_fileheader_stream(&stream).expect("header without MinorVersion should parse");
        assert_eq!(parsed.header.minor_version, 0);
        assert_eq!(parsed.records.len(), 1);
        assert!(matches!(parsed.records[0], SchRecord::Sheet(_)));
    }

    #[test]
    fn rejects_binary_blocks_in_fileheader_records() {
        let mut stream = write_text_block(&text_payload(
            "|HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0|Weight=1|MinorVersion=2|UniqueID=ABCDEFGH|",
        ));
        stream.extend_from_slice(&write_binary_block(&[0xAA, 0xBB]));

        let err = parse_fileheader_stream(&stream).expect_err("must reject binary record blocks");
        assert!(
            matches!(err, AltiumFormatError::InvalidParamValue { key, .. } if key == "FileHeader")
        );
    }
}

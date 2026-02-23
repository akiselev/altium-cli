use altium_format_types::SchRecordType;

use crate::param_collection::ParameterCollection;
use crate::sch_records::{
    SchArc, SchBezier, SchDesignator, SchEllipse, SchEllipticalArc, SchImage,
    SchImplementation, SchImplementationList, SchImplementationMap, SchLabel, SchLine,
    SchMapDefiner, SchParameter, SchParameterList, SchPie, SchPolygon, SchPolyline,
    SchRecord, SchRectangle, SchRoundRectangle, SchSheet, SchSymbol, SchTemplate,
    SchTextFrame, parse_component_record,
};
use crate::{AltiumFormatError, Result, ResultExt};

pub(crate) fn dispatch_record_type(
    record_type_val: i32,
    params: &mut ParameterCollection,
) -> Result<SchRecord> {
    let record_type = SchRecordType::try_from(record_type_val)?;

    macro_rules! dispatch {
        ($ty:ty => $variant:expr) => {{
            let parsed = <$ty>::from_params(params).with_context(|| {
                format!("RECORD={record_type_val} ({ty_name})", ty_name = stringify!($ty))
            })?;
            Ok($variant(parsed))
        }};
    }

    match record_type {
        SchRecordType::Sheet => {
            let sheet = SchSheet::from_params(params)
                .with_context(|| format!("RECORD={record_type_val} (SchSheet)"))?;
            Ok(SchRecord::Sheet(sheet))
        }
        SchRecordType::Template => dispatch!(SchTemplate => SchRecord::Template),
        SchRecordType::Component => {
            let comp = parse_component_record(params)
                .with_context(|| format!("RECORD={record_type_val} (SchComponent)"))?;
            Ok(SchRecord::Component(comp))
        }
        SchRecordType::Pin => Err(AltiumFormatError::InvalidParamValue {
            key: "RECORD".to_owned(),
            detail: "RECORD=2 (Pin) text parsing is not implemented yet for SchDoc".to_owned(),
        }),
        SchRecordType::Symbol => dispatch!(SchSymbol => SchRecord::Symbol),
        SchRecordType::Label => dispatch!(SchLabel => SchRecord::Label),
        SchRecordType::Bezier => dispatch!(SchBezier => SchRecord::Bezier),
        SchRecordType::Polyline => dispatch!(SchPolyline => SchRecord::Polyline),
        SchRecordType::Polygon => dispatch!(SchPolygon => SchRecord::Polygon),
        SchRecordType::Ellipse => dispatch!(SchEllipse => SchRecord::Ellipse),
        SchRecordType::Pie => dispatch!(SchPie => SchRecord::Pie),
        SchRecordType::RoundRectangle => dispatch!(SchRoundRectangle => SchRecord::RoundRectangle),
        SchRecordType::EllipticalArc => dispatch!(SchEllipticalArc => SchRecord::EllipticalArc),
        SchRecordType::Arc => dispatch!(SchArc => SchRecord::Arc),
        SchRecordType::Line => dispatch!(SchLine => SchRecord::Line),
        SchRecordType::Rectangle => dispatch!(SchRectangle => SchRecord::Rectangle),
        SchRecordType::TextFrame => dispatch!(SchTextFrame => SchRecord::TextFrame),
        SchRecordType::Image => dispatch!(SchImage => SchRecord::Image),
        SchRecordType::Designator => dispatch!(SchDesignator => SchRecord::Designator),
        SchRecordType::Parameter => dispatch!(SchParameter => SchRecord::Parameter),
        SchRecordType::ImplementationList => {
            dispatch!(SchImplementationList => SchRecord::ImplementationList)
        }
        SchRecordType::Implementation => dispatch!(SchImplementation => SchRecord::Implementation),
        SchRecordType::ImplementationMap => {
            dispatch!(SchImplementationMap => SchRecord::ImplementationMap)
        }
        SchRecordType::MapDefiner => {
            let map = SchMapDefiner::from_params(params)
                .with_context(|| format!("RECORD={record_type_val} (SchMapDefiner)"))?;
            Ok(SchRecord::MapDefiner(map))
        }
        SchRecordType::ParameterList => dispatch!(SchParameterList => SchRecord::ParameterList),
        _ => Err(AltiumFormatError::UnknownRecordType(record_type_val)),
    }
}

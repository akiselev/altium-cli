use altium_format_types::SchRecordType;

use crate::param_collection::ParameterCollection;
use crate::sch_records::{
    SchArc, SchBezier, SchBlanket, SchBus, SchBusEntry, SchCompileMask, SchDesignator, SchEllipse,
    SchEllipticalArc, SchImage, SchImplementation, SchImplementationList, SchImplementationMap,
    SchJunction, SchLabel, SchLine, SchMapDefiner, SchNetLabel, SchNoConnect, SchNote,
    SchParameter, SchParameterList, SchParameterSet, SchPie, SchPolygon, SchPolyline, SchPort,
    SchPowerObject, SchProbe, SchRecord, SchRectangle, SchRoundRectangle, SchSheet, SchSheetEntry,
    SchSheetFileName, SchSheetName, SchSheetSymbol, SchSymbol, SchTemplate, SchTextFrame, SchWire,
    parse_component_record, parse_text_pin,
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
                format!(
                    "RECORD={record_type_val} ({ty_name})",
                    ty_name = stringify!($ty)
                )
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
        SchRecordType::Wire => dispatch!(SchWire => SchRecord::Wire),
        SchRecordType::Bus => dispatch!(SchBus => SchRecord::Bus),
        SchRecordType::NetLabel => dispatch!(SchNetLabel => SchRecord::NetLabel),
        SchRecordType::PowerObject => dispatch!(SchPowerObject => SchRecord::PowerObject),
        SchRecordType::Port => dispatch!(SchPort => SchRecord::Port),
        SchRecordType::NoErc => dispatch!(SchNoConnect => SchRecord::NoConnect),
        SchRecordType::Junction => dispatch!(SchJunction => SchRecord::Junction),
        SchRecordType::SheetName => dispatch!(SchSheetName => SchRecord::SheetName),
        SchRecordType::SheetFileName => dispatch!(SchSheetFileName => SchRecord::SheetFileName),
        SchRecordType::SheetSymbol => dispatch!(SchSheetSymbol => SchRecord::SheetSymbol),
        SchRecordType::SheetEntry => dispatch!(SchSheetEntry => SchRecord::SheetEntry),
        SchRecordType::BusEntry => dispatch!(SchBusEntry => SchRecord::BusEntry),
        SchRecordType::ParameterSet => dispatch!(SchParameterSet => SchRecord::ParameterSet),
        SchRecordType::Note => dispatch!(SchNote => SchRecord::Note),
        SchRecordType::Probe => dispatch!(SchProbe => SchRecord::Probe),
        SchRecordType::CompileMask => dispatch!(SchCompileMask => SchRecord::CompileMask),
        SchRecordType::Blanket => dispatch!(SchBlanket => SchRecord::Blanket),
        SchRecordType::Component => {
            let comp = parse_component_record(params)
                .with_context(|| format!("RECORD={record_type_val} (SchComponent)"))?;
            Ok(SchRecord::Component(comp))
        }
        SchRecordType::Pin => {
            let pin = parse_text_pin(params)
                .with_context(|| format!("RECORD={record_type_val} (SchPin text)"))?;
            Ok(SchRecord::Pin(pin))
        }
        SchRecordType::Symbol => dispatch!(SchSymbol => SchRecord::Symbol),
        SchRecordType::Label => dispatch!(SchLabel => SchRecord::Label),
        SchRecordType::Hyperlink => dispatch!(SchLabel => SchRecord::Hyperlink),
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

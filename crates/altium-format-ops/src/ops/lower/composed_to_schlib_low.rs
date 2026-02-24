use crate::ops::model::ComposedOp;
use altium_format_types::{Coord, CoordPoint, RotationBy90};

use altium_format::sch_ops_core::{
    AddAliasOp, ComponentRefOp, ComponentRootOp, ComponentTextOp, EditComponentOp, EditRecordOp,
    ImplementationOp, MapDefinerOp, ParameterOp, PinOp, QueryComponentsOp, QueryOp, QueryPinsOp,
    QueryRecordsOp, RemoveAliasOp, RemoveComponentOp, RemoveRecordsOp, SchLibLowOp,
};

pub fn lower_composed_to_schlib_low(composed_ops: &[ComposedOp]) -> Vec<SchLibLowOp> {
    composed_ops
        .iter()
        .map(|op| match op {
            ComposedOp::CreateComponentRoot(v) => {
                SchLibLowOp::CreateComponentRoot(ComponentRootOp {
                    opid: v.opid.clone(),
                    id: v.id.clone(),
                    lib_reference: v.lib_reference.clone(),
                    designator: v.designator.clone(),
                    value: v.value.clone(),
                })
            }
            ComposedOp::CreateComponentDesignator(v) => {
                SchLibLowOp::CreateComponentDesignator(ComponentTextOp {
                    opid: v.opid.clone(),
                    component_ref: v.component_ref.clone(),
                    text: v.text.clone(),
                })
            }
            ComposedOp::CreateComponentComment(v) => {
                SchLibLowOp::CreateComponentComment(ComponentTextOp {
                    opid: v.opid.clone(),
                    component_ref: v.component_ref.clone(),
                    text: v.text.clone(),
                })
            }
            ComposedOp::AddPin(v) => SchLibLowOp::AddPin(PinOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                designator: v.designator.clone(),
                name: v.name.clone(),
                electrical: v.electrical.clone(),
                length: v.length_mils.map(Coord::from_mils),
                at: v
                    .at
                    .map(|(x, y)| CoordPoint::new(Coord::from_mils(x), Coord::from_mils(y))),
                rotation: v.rotation.map(rotation_from_degrees),
            }),
            ComposedOp::CreateImplementationList(v) => {
                SchLibLowOp::CreateImplementationList(ComponentRefOp {
                    opid: v.opid.clone(),
                    component_ref: v.component_ref.clone(),
                })
            }
            ComposedOp::CreateImplementation(v) => {
                SchLibLowOp::CreateImplementation(ImplementationOp {
                    opid: v.opid.clone(),
                    component_ref: v.component_ref.clone(),
                    model_name: v.model_name.clone(),
                    model_type: None,
                    is_current: None,
                })
            }
            ComposedOp::CreateImplementationMap(v) => {
                SchLibLowOp::CreateImplementationMap(ComponentRefOp {
                    opid: v.opid.clone(),
                    component_ref: v.component_ref.clone(),
                })
            }
            ComposedOp::CreateMapDefiner(v) => SchLibLowOp::CreateMapDefiner(MapDefinerOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                pin_designator: v.pin_designator.clone(),
                pad_designator: v.pad_designator.clone(),
            }),
            ComposedOp::CreateParameterList(v) => {
                SchLibLowOp::CreateParameterList(ComponentRefOp {
                    opid: v.opid.clone(),
                    component_ref: v.component_ref.clone(),
                })
            }
            ComposedOp::AddParameter(v) => SchLibLowOp::AddParameter(ParameterOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                name: v.name.clone(),
                text: v.text.clone(),
                is_hidden: v.is_hidden,
            }),
            ComposedOp::AddAlias(v) => SchLibLowOp::AddAlias(AddAliasOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                alias_name: v.alias_name.clone(),
            }),
            ComposedOp::RemoveAlias(v) => SchLibLowOp::RemoveAlias(RemoveAliasOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                alias_name: v.alias_name.clone(),
            }),
            ComposedOp::RemoveComponent(v) => SchLibLowOp::RemoveComponent(RemoveComponentOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
            }),
            ComposedOp::EditComponent(v) => SchLibLowOp::EditComponent(EditComponentOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                description: v.description.clone(),
                part_count: v.part_count,
                display_mode_count: v.display_mode_count,
                component_kind: v.component_kind,
                show_hidden_pins: v.show_hidden_pins,
            }),
            ComposedOp::EditRecord(v) => SchLibLowOp::EditRecord(EditRecordOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                selector: v.selector.clone(),
                patch: v.patch.clone(),
            }),
            ComposedOp::RemoveRecords(v) => SchLibLowOp::RemoveRecords(RemoveRecordsOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                selector: v.selector.clone(),
            }),
            ComposedOp::Query(v) => SchLibLowOp::Query(QueryOp {
                opid: v.opid.clone(),
                selector: v.selector.clone(),
            }),
            ComposedOp::QueryComponents(v) => SchLibLowOp::QueryComponents(QueryComponentsOp {
                opid: v.opid.clone(),
                pattern: v.pattern.clone(),
            }),
            ComposedOp::QueryPins(v) => SchLibLowOp::QueryPins(QueryPinsOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
            }),
            ComposedOp::QueryRecords(v) => SchLibLowOp::QueryRecords(QueryRecordsOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                record_type: v.record_type,
            }),
            ComposedOp::AddLine(v) => SchLibLowOp::AddLine(v.0.clone()),
            ComposedOp::AddRectangle(v) => SchLibLowOp::AddRectangle(v.0.clone()),
            ComposedOp::AddArc(v) => SchLibLowOp::AddArc(v.0.clone()),
            ComposedOp::AddEllipticalArc(v) => SchLibLowOp::AddEllipticalArc(v.0.clone()),
            ComposedOp::AddEllipse(v) => SchLibLowOp::AddEllipse(v.0.clone()),
            ComposedOp::AddPolyline(v) => SchLibLowOp::AddPolyline(v.0.clone()),
            ComposedOp::AddPolygon(v) => SchLibLowOp::AddPolygon(v.0.clone()),
            ComposedOp::AddBezier(v) => SchLibLowOp::AddBezier(v.0.clone()),
            ComposedOp::AddPie(v) => SchLibLowOp::AddPie(v.0.clone()),
            ComposedOp::AddRoundRectangle(v) => SchLibLowOp::AddRoundRectangle(v.0.clone()),
            ComposedOp::AddLabel(v) => SchLibLowOp::AddLabel(v.0.clone()),
            ComposedOp::AddTextFrame(v) => SchLibLowOp::AddTextFrame(v.0.clone()),
            ComposedOp::AddImage(v) => SchLibLowOp::AddImage(v.0.clone()),
        })
        .collect()
}

fn rotation_from_degrees(deg: i32) -> RotationBy90 {
    match deg.rem_euclid(360) {
        0 => RotationBy90::Rotate0,
        90 => RotationBy90::Rotate90,
        180 => RotationBy90::Rotate180,
        270 => RotationBy90::Rotate270,
        _ => RotationBy90::Rotate0,
    }
}

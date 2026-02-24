use crate::ops::model::ComposedOp;

use altium_format::sch_ops_core::{
    AddAliasOp, ComponentRefOp, ComponentRootOp, ComponentTextOp, EditComponentOp, EditRecordOp,
    ImplementationOp, MapDefinerOp, ParameterOp, PinOp, QueryComponentsOp, QueryOp, QueryPinsOp,
    QueryRecordsOp, RemoveAliasOp, RemoveComponentOp, RemoveRecordsOp, SchDocLowOp,
};

pub fn lower_composed_to_schdoc_low(composed_ops: &[ComposedOp]) -> Vec<SchDocLowOp> {
    composed_ops
        .iter()
        .map(|op| match op {
            ComposedOp::CreateComponentRoot(v) => {
                SchDocLowOp::CreateComponentRoot(ComponentRootOp {
                    opid: v.opid.clone(),
                    id: v.id.clone(),
                    lib_reference: v.lib_reference.clone(),
                    designator: v.designator.clone(),
                    value: v.value.clone(),
                })
            }
            ComposedOp::CreateComponentDesignator(v) => {
                SchDocLowOp::CreateComponentDesignator(ComponentTextOp {
                    opid: v.opid.clone(),
                    component_ref: v.component_ref.clone(),
                    text: v.text.clone(),
                })
            }
            ComposedOp::CreateComponentComment(v) => {
                SchDocLowOp::CreateComponentComment(ComponentTextOp {
                    opid: v.opid.clone(),
                    component_ref: v.component_ref.clone(),
                    text: v.text.clone(),
                })
            }
            ComposedOp::AddPin(v) => SchDocLowOp::AddPin(PinOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                designator: v.designator.clone(),
                name: v.name.clone(),
                electrical: v.electrical.clone(),
                length_mils: v.length_mils,
            }),
            ComposedOp::CreateImplementationList(v) => {
                SchDocLowOp::CreateImplementationList(ComponentRefOp {
                    opid: v.opid.clone(),
                    component_ref: v.component_ref.clone(),
                })
            }
            ComposedOp::CreateImplementation(v) => {
                SchDocLowOp::CreateImplementation(ImplementationOp {
                    opid: v.opid.clone(),
                    component_ref: v.component_ref.clone(),
                    model_name: v.model_name.clone(),
                    model_type: None,
                    is_current: None,
                })
            }
            ComposedOp::CreateImplementationMap(v) => {
                SchDocLowOp::CreateImplementationMap(ComponentRefOp {
                    opid: v.opid.clone(),
                    component_ref: v.component_ref.clone(),
                })
            }
            ComposedOp::CreateMapDefiner(v) => SchDocLowOp::CreateMapDefiner(MapDefinerOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                pin_designator: v.pin_designator.clone(),
                pad_designator: v.pad_designator.clone(),
            }),
            ComposedOp::CreateParameterList(v) => {
                SchDocLowOp::CreateParameterList(ComponentRefOp {
                    opid: v.opid.clone(),
                    component_ref: v.component_ref.clone(),
                })
            }
            ComposedOp::AddParameter(v) => SchDocLowOp::AddParameter(ParameterOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                name: v.name.clone(),
                text: v.text.clone(),
                is_hidden: v.is_hidden,
            }),
            ComposedOp::AddAlias(v) => SchDocLowOp::AddAlias(AddAliasOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                alias_name: v.alias_name.clone(),
            }),
            ComposedOp::RemoveAlias(v) => SchDocLowOp::RemoveAlias(RemoveAliasOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                alias_name: v.alias_name.clone(),
            }),
            ComposedOp::RemoveComponent(v) => SchDocLowOp::RemoveComponent(RemoveComponentOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
            }),
            ComposedOp::EditComponent(v) => SchDocLowOp::EditComponent(EditComponentOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                description: v.description.clone(),
                part_count: v.part_count,
                display_mode_count: v.display_mode_count,
                component_kind: v.component_kind,
                show_hidden_pins: v.show_hidden_pins,
            }),
            ComposedOp::EditRecord(v) => SchDocLowOp::EditRecord(EditRecordOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                selector: v.selector.clone(),
                patch: v.patch.clone(),
            }),
            ComposedOp::RemoveRecords(v) => SchDocLowOp::RemoveRecords(RemoveRecordsOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                selector: v.selector.clone(),
            }),
            ComposedOp::Query(v) => SchDocLowOp::Query(QueryOp {
                opid: v.opid.clone(),
                selector: v.selector.clone(),
            }),
            ComposedOp::QueryComponents(v) => SchDocLowOp::QueryComponents(QueryComponentsOp {
                opid: v.opid.clone(),
                pattern: v.pattern.clone(),
            }),
            ComposedOp::QueryPins(v) => SchDocLowOp::QueryPins(QueryPinsOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
            }),
            ComposedOp::QueryRecords(v) => SchDocLowOp::QueryRecords(QueryRecordsOp {
                opid: v.opid.clone(),
                component_ref: v.component_ref.clone(),
                record_type: v.record_type,
            }),
            ComposedOp::AddLine(v) => SchDocLowOp::AddLine(v.0.clone()),
            ComposedOp::AddRectangle(v) => SchDocLowOp::AddRectangle(v.0.clone()),
            ComposedOp::AddArc(v) => SchDocLowOp::AddArc(v.0.clone()),
            ComposedOp::AddEllipticalArc(v) => SchDocLowOp::AddEllipticalArc(v.0.clone()),
            ComposedOp::AddEllipse(v) => SchDocLowOp::AddEllipse(v.0.clone()),
            ComposedOp::AddPolyline(v) => SchDocLowOp::AddPolyline(v.0.clone()),
            ComposedOp::AddPolygon(v) => SchDocLowOp::AddPolygon(v.0.clone()),
            ComposedOp::AddBezier(v) => SchDocLowOp::AddBezier(v.0.clone()),
            ComposedOp::AddPie(v) => SchDocLowOp::AddPie(v.0.clone()),
            ComposedOp::AddRoundRectangle(v) => SchDocLowOp::AddRoundRectangle(v.0.clone()),
            ComposedOp::AddLabel(v) => SchDocLowOp::AddLabel(v.0.clone()),
            ComposedOp::AddTextFrame(v) => SchDocLowOp::AddTextFrame(v.0.clone()),
            ComposedOp::AddImage(v) => SchDocLowOp::AddImage(v.0.clone()),
        })
        .collect()
}

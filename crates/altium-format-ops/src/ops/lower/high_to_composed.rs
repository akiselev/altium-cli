use crate::ops::model::{
    AddArcNode, AddBezierNode, AddComponentOp, AddEllipseNode, AddEllipticalArcNode,
    AddFootprintNode, AddImageNode, AddLabelNode, AddLineNode, AddPieNode, AddPinOp,
    AddPolygonNode, AddPolylineNode, AddRectangleNode, AddRoundRectangleNode, AddTextFrameNode,
    AddTrackNode, AliasNode, ComponentRefNode, ComponentRoot, ComponentText, ComposedOp,
    EditComponentNode, EditRecordNode, HighOp, ImplementationNode, MapDefinerNode, ParameterNode,
    PinNode, QueryComponentsNode, QueryNode, QueryPinsNode, QueryRecordsNode, RemoveComponentNode,
    RemoveRecordsNode,
};
use altium_format_types::{Coord, CoordPoint};

pub fn lower_high_ops(high_ops: &[HighOp]) -> Vec<ComposedOp> {
    let mut out = Vec::new();
    for (i, op) in high_ops.iter().enumerate() {
        let base_opid = op
            .opid()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("op_{:04}", i + 1));
        lower_high_op(op, &base_opid, &mut out);
    }
    out
}

fn lower_high_op(op: &HighOp, base_opid: &str, out: &mut Vec<ComposedOp>) {
    match op {
        HighOp::AddComponent(v) => lower_add_component(v, base_opid, out),
        HighOp::AddPin(v) => out.push(ComposedOp::AddPin(pin_from_add_pin(v, base_opid))),
        HighOp::AddParameter(v) => out.push(ComposedOp::AddParameter(ParameterNode {
            opid: base_opid.to_owned(),
            component_ref: v.component_ref.clone(),
            name: v.name.clone(),
            text: v.text.clone(),
            is_hidden: v.is_hidden,
        })),
        HighOp::AddAlias(v) => out.push(ComposedOp::AddAlias(AliasNode {
            opid: base_opid.to_owned(),
            component_ref: v.component_ref.clone(),
            alias_name: v.alias_name.clone(),
        })),
        HighOp::RemoveAlias(v) => out.push(ComposedOp::RemoveAlias(AliasNode {
            opid: base_opid.to_owned(),
            component_ref: v.component_ref.clone(),
            alias_name: v.alias_name.clone(),
        })),
        HighOp::RemoveComponent(v) => out.push(ComposedOp::RemoveComponent(RemoveComponentNode {
            opid: base_opid.to_owned(),
            component_ref: v.component_ref.clone(),
        })),
        HighOp::EditComponent(v) => out.push(ComposedOp::EditComponent(EditComponentNode {
            opid: base_opid.to_owned(),
            component_ref: v.component_ref.clone(),
            description: v.description.clone(),
            part_count: v.part_count,
            display_mode_count: v.display_mode_count,
            component_kind: v.component_kind,
            show_hidden_pins: v.show_hidden_pins,
        })),
        HighOp::EditRecord(v) => out.push(ComposedOp::EditRecord(EditRecordNode {
            opid: base_opid.to_owned(),
            component_ref: v.component_ref.clone(),
            selector: v.selector.clone(),
            patch: v.patch.clone(),
        })),
        HighOp::RemoveRecords(v) => out.push(ComposedOp::RemoveRecords(RemoveRecordsNode {
            opid: base_opid.to_owned(),
            component_ref: v.component_ref.clone(),
            selector: v.selector.clone(),
        })),
        HighOp::Query(v) => out.push(ComposedOp::Query(QueryNode {
            opid: base_opid.to_owned(),
            selector: v.selector.clone(),
        })),
        HighOp::QueryComponents(v) => out.push(ComposedOp::QueryComponents(QueryComponentsNode {
            opid: base_opid.to_owned(),
            pattern: v.pattern.clone(),
        })),
        HighOp::QueryPins(v) => out.push(ComposedOp::QueryPins(QueryPinsNode {
            opid: base_opid.to_owned(),
            component_ref: v.component_ref.clone(),
        })),
        HighOp::QueryRecords(v) => out.push(ComposedOp::QueryRecords(QueryRecordsNode {
            opid: base_opid.to_owned(),
            component_ref: v.component_ref.clone(),
            record_type: v.record_type,
        })),
        HighOp::AddLine(v) => out.push(ComposedOp::AddLine(AddLineNode(
            altium_format::sch_ops_core::AddLineOp {
                opid: base_opid.to_owned(),
                component_ref: v.component_ref.clone(),
                from: CoordPoint::new(Coord::from_mils(v.from.0), Coord::from_mils(v.from.1)),
                to: CoordPoint::new(Coord::from_mils(v.to.0), Coord::from_mils(v.to.1)),
                color: v.color,
                line_width: v.line_width,
                line_style: v.line_style,
                owner_part_id: v.owner_part_id,
                owner_part_display_mode: v.owner_part_display_mode,
            },
        ))),
        HighOp::AddRectangle(v) => out.push(ComposedOp::AddRectangle(AddRectangleNode(
            altium_format::sch_ops_core::AddRectangleOp {
                opid: base_opid.to_owned(),
                component_ref: v.component_ref.clone(),
                from: CoordPoint::new(Coord::from_mils(v.from.0), Coord::from_mils(v.from.1)),
                to: CoordPoint::new(Coord::from_mils(v.to.0), Coord::from_mils(v.to.1)),
                color: v.color,
                area_color: v.area_color,
                is_solid: v.is_solid,
                transparent: v.transparent,
                line_width: v.line_width,
                owner_part_id: v.owner_part_id,
                owner_part_display_mode: v.owner_part_display_mode,
            },
        ))),
        HighOp::AddArc(v) => out.push(ComposedOp::AddArc(AddArcNode(
            altium_format::sch_ops_core::AddArcOp {
                opid: base_opid.to_owned(),
                component_ref: v.component_ref.clone(),
                cx_mils: v.cx_mils,
                cy_mils: v.cy_mils,
                radius_mils: v.radius_mils,
                start_angle: v.start_angle,
                end_angle: v.end_angle,
                color: v.color,
                line_width: v.line_width,
                owner_part_id: v.owner_part_id,
                owner_part_display_mode: v.owner_part_display_mode,
            },
        ))),
        HighOp::AddEllipticalArc(v) => out.push(ComposedOp::AddEllipticalArc(
            AddEllipticalArcNode(altium_format::sch_ops_core::AddEllipticalArcOp {
                opid: base_opid.to_owned(),
                component_ref: v.component_ref.clone(),
                cx_mils: v.cx_mils,
                cy_mils: v.cy_mils,
                radius_mils: v.radius_mils,
                secondary_radius_mils: v.secondary_radius_mils,
                start_angle: v.start_angle,
                end_angle: v.end_angle,
                color: v.color,
                line_width: v.line_width,
                owner_part_id: v.owner_part_id,
                owner_part_display_mode: v.owner_part_display_mode,
            }),
        )),
        HighOp::AddEllipse(v) => out.push(ComposedOp::AddEllipse(AddEllipseNode(
            altium_format::sch_ops_core::AddEllipseOp {
                opid: base_opid.to_owned(),
                component_ref: v.component_ref.clone(),
                cx_mils: v.cx_mils,
                cy_mils: v.cy_mils,
                radius_mils: v.radius_mils,
                secondary_radius_mils: v.secondary_radius_mils,
                color: v.color,
                area_color: v.area_color,
                is_solid: v.is_solid,
                line_width: v.line_width,
                owner_part_id: v.owner_part_id,
                owner_part_display_mode: v.owner_part_display_mode,
            },
        ))),
        HighOp::AddPolyline(v) => out.push(ComposedOp::AddPolyline(AddPolylineNode(
            altium_format::sch_ops_core::AddPolylineOp {
                opid: base_opid.to_owned(),
                component_ref: v.component_ref.clone(),
                points_mils: v.points_mils.clone(),
                color: v.color,
                line_width: v.line_width,
                line_style: v.line_style,
                owner_part_id: v.owner_part_id,
                owner_part_display_mode: v.owner_part_display_mode,
            },
        ))),
        HighOp::AddPolygon(v) => out.push(ComposedOp::AddPolygon(AddPolygonNode(
            altium_format::sch_ops_core::AddPolygonOp {
                opid: base_opid.to_owned(),
                component_ref: v.component_ref.clone(),
                points_mils: v.points_mils.clone(),
                color: v.color,
                area_color: v.area_color,
                is_solid: v.is_solid,
                line_width: v.line_width,
                owner_part_id: v.owner_part_id,
                owner_part_display_mode: v.owner_part_display_mode,
            },
        ))),
        HighOp::AddBezier(v) => out.push(ComposedOp::AddBezier(AddBezierNode(
            altium_format::sch_ops_core::AddBezierOp {
                opid: base_opid.to_owned(),
                component_ref: v.component_ref.clone(),
                points_mils: v.points_mils.clone(),
                color: v.color,
                line_width: v.line_width,
                owner_part_id: v.owner_part_id,
                owner_part_display_mode: v.owner_part_display_mode,
            },
        ))),
        HighOp::AddPie(v) => out.push(ComposedOp::AddPie(AddPieNode(
            altium_format::sch_ops_core::AddPieOp {
                opid: base_opid.to_owned(),
                component_ref: v.component_ref.clone(),
                cx_mils: v.cx_mils,
                cy_mils: v.cy_mils,
                radius_mils: v.radius_mils,
                start_angle: v.start_angle,
                end_angle: v.end_angle,
                color: v.color,
                area_color: v.area_color,
                is_solid: v.is_solid,
                line_width: v.line_width,
                owner_part_id: v.owner_part_id,
                owner_part_display_mode: v.owner_part_display_mode,
            },
        ))),
        HighOp::AddRoundRectangle(v) => out.push(ComposedOp::AddRoundRectangle(
            AddRoundRectangleNode(altium_format::sch_ops_core::AddRoundRectangleOp {
                opid: base_opid.to_owned(),
                component_ref: v.component_ref.clone(),
                from: CoordPoint::new(Coord::from_mils(v.from.0), Coord::from_mils(v.from.1)),
                to: CoordPoint::new(Coord::from_mils(v.to.0), Coord::from_mils(v.to.1)),
                corner_x_radius_mils: v.corner_x_radius_mils,
                corner_y_radius_mils: v.corner_y_radius_mils,
                color: v.color,
                area_color: v.area_color,
                is_solid: v.is_solid,
                line_width: v.line_width,
                owner_part_id: v.owner_part_id,
                owner_part_display_mode: v.owner_part_display_mode,
            }),
        )),
        HighOp::AddLabel(v) => out.push(ComposedOp::AddLabel(AddLabelNode(
            altium_format::sch_ops_core::AddLabelOp {
                opid: base_opid.to_owned(),
                component_ref: v.component_ref.clone(),
                x_mils: v.x_mils,
                y_mils: v.y_mils,
                text: v.text.clone(),
                color: v.color,
                font_id: v.font_id,
                orientation: v.orientation,
                justification: v.justification,
                is_mirrored: v.is_mirrored,
                owner_part_id: v.owner_part_id,
                owner_part_display_mode: v.owner_part_display_mode,
            },
        ))),
        HighOp::AddTextFrame(v) => out.push(ComposedOp::AddTextFrame(AddTextFrameNode(
            altium_format::sch_ops_core::AddTextFrameOp {
                opid: base_opid.to_owned(),
                component_ref: v.component_ref.clone(),
                from: CoordPoint::new(Coord::from_mils(v.from.0), Coord::from_mils(v.from.1)),
                to: CoordPoint::new(Coord::from_mils(v.to.0), Coord::from_mils(v.to.1)),
                text: v.text.clone(),
                color: v.color,
                area_color: v.area_color,
                font_id: v.font_id,
                alignment: v.alignment,
                word_wrap: v.word_wrap,
                show_border: v.show_border,
                is_solid: v.is_solid,
                clip_to_rect: v.clip_to_rect,
                owner_part_id: v.owner_part_id,
                owner_part_display_mode: v.owner_part_display_mode,
            },
        ))),
        HighOp::AddImage(v) => out.push(ComposedOp::AddImage(AddImageNode(
            altium_format::sch_ops_core::AddImageOp {
                opid: base_opid.to_owned(),
                component_ref: v.component_ref.clone(),
                from: CoordPoint::new(Coord::from_mils(v.from.0), Coord::from_mils(v.from.1)),
                to: CoordPoint::new(Coord::from_mils(v.to.0), Coord::from_mils(v.to.1)),
                file_name: v.file_name.clone(),
                image_data: v.image_data.clone(),
                keep_aspect: v.keep_aspect,
                owner_part_id: v.owner_part_id,
                owner_part_display_mode: v.owner_part_display_mode,
            },
        ))),
        HighOp::AddTrack(v) => out.push(ComposedOp::AddTrack(AddTrackNode(
            altium_format::pcb_ops_core::AddTrackOp {
                opid: base_opid.to_owned(),
                footprint_ref: v.footprint_ref.clone(),
                start: CoordPoint::new(Coord::from_mils(v.start.0), Coord::from_mils(v.start.1)),
                end: CoordPoint::new(Coord::from_mils(v.end.0), Coord::from_mils(v.end.1)),
                width: v.width_mils.map(Coord::from_mils),
                layer: v.layer.clone(),
            },
        ))),
        HighOp::AddFootprint(v) => out.push(ComposedOp::AddFootprint(AddFootprintNode(
            altium_format::pcb_ops_core::AddFootprintOp {
                opid: base_opid.to_owned(),
                id: v.id.clone(),
                name: v.name.clone(),
                pattern: v.pattern.clone(),
                description: v.description.clone(),
            },
        ))),
    }
}

fn child(base: &str, segment: &str, idx: usize) -> String {
    format!("{base}/{segment}[{idx}]")
}

fn lower_add_component(op: &AddComponentOp, base_opid: &str, out: &mut Vec<ComposedOp>) {
    out.push(ComposedOp::CreateComponentRoot(ComponentRoot {
        opid: child(base_opid, "create_component_root", 0),
        id: op.id.clone(),
        lib_reference: op.lib_reference.clone(),
        designator: op.designator.clone(),
        value: op.value.clone(),
    }));

    if let Some(designator) = &op.designator {
        out.push(ComposedOp::CreateComponentDesignator(ComponentText {
            opid: child(base_opid, "create_component_designator", 0),
            component_ref: op.component_ref.clone(),
            text: designator.clone(),
        }));
    }

    if let Some(value) = &op.value {
        out.push(ComposedOp::CreateComponentComment(ComponentText {
            opid: child(base_opid, "create_component_comment", 0),
            component_ref: op.component_ref.clone(),
            text: value.clone(),
        }));
    }

    for (i, pin) in op.pins.iter().enumerate() {
        let pid = child(base_opid, "add_pin", i);
        let mut sub = pin.clone();
        sub.opid = Some(pid.clone());
        out.push(ComposedOp::AddPin(pin_from_add_pin(&sub, &pid)));
    }

    if let Some(footprint) = &op.footprint {
        let cref = op.component_ref.clone();
        out.push(ComposedOp::CreateImplementationList(ComponentRefNode {
            opid: child(base_opid, "create_implementation_list", 0),
            component_ref: cref.clone(),
        }));
        out.push(ComposedOp::CreateImplementation(ImplementationNode {
            opid: child(base_opid, "create_implementation", 0),
            component_ref: cref.clone(),
            model_name: footprint.model_name.clone(),
        }));
        out.push(ComposedOp::CreateImplementationMap(ComponentRefNode {
            opid: child(base_opid, "create_implementation_map", 0),
            component_ref: cref.clone(),
        }));
        for (i, map) in footprint.map.iter().enumerate() {
            out.push(ComposedOp::CreateMapDefiner(MapDefinerNode {
                opid: child(base_opid, "create_map_definer", i),
                component_ref: cref.clone(),
                pin_designator: map.pin.clone(),
                pad_designator: map.pad.clone(),
            }));
        }
        out.push(ComposedOp::CreateParameterList(ComponentRefNode {
            opid: child(base_opid, "create_parameter_list", 0),
            component_ref: cref,
        }));
    }
}

fn pin_from_add_pin(v: &AddPinOp, fallback_opid: &str) -> PinNode {
    PinNode {
        opid: v.opid.clone().unwrap_or_else(|| fallback_opid.to_owned()),
        component_ref: v.component_ref.clone(),
        designator: v.designator.clone(),
        name: v.name.clone(),
        electrical: v.electrical.clone(),
        length_mils: v.length_mils,
        at: v.at,
        rotation: v.rotation,
    }
}

trait HighOpExt {
    fn opid(&self) -> Option<&str>;
}

impl HighOpExt for HighOp {
    fn opid(&self) -> Option<&str> {
        match self {
            HighOp::AddComponent(v) => v.opid.as_deref(),
            HighOp::AddPin(v) => v.opid.as_deref(),
            HighOp::AddParameter(v) => v.opid.as_deref(),
            HighOp::AddAlias(v) => v.opid.as_deref(),
            HighOp::RemoveAlias(v) => v.opid.as_deref(),
            HighOp::RemoveComponent(v) => v.opid.as_deref(),
            HighOp::EditComponent(v) => v.opid.as_deref(),
            HighOp::EditRecord(v) => v.opid.as_deref(),
            HighOp::RemoveRecords(v) => v.opid.as_deref(),
            HighOp::Query(v) => v.opid.as_deref(),
            HighOp::QueryComponents(v) => v.opid.as_deref(),
            HighOp::QueryPins(v) => v.opid.as_deref(),
            HighOp::QueryRecords(v) => v.opid.as_deref(),
            HighOp::AddLine(v) => v.opid.as_deref(),
            HighOp::AddRectangle(v) => v.opid.as_deref(),
            HighOp::AddArc(v) => v.opid.as_deref(),
            HighOp::AddEllipticalArc(v) => v.opid.as_deref(),
            HighOp::AddEllipse(v) => v.opid.as_deref(),
            HighOp::AddPolyline(v) => v.opid.as_deref(),
            HighOp::AddPolygon(v) => v.opid.as_deref(),
            HighOp::AddBezier(v) => v.opid.as_deref(),
            HighOp::AddPie(v) => v.opid.as_deref(),
            HighOp::AddRoundRectangle(v) => v.opid.as_deref(),
            HighOp::AddLabel(v) => v.opid.as_deref(),
            HighOp::AddTextFrame(v) => v.opid.as_deref(),
            HighOp::AddImage(v) => v.opid.as_deref(),
            HighOp::AddTrack(v) => v.opid.as_deref(),
            HighOp::AddFootprint(v) => v.opid.as_deref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::model::{FootprintMapEntry, FootprintOp};

    #[test]
    fn add_component_lowers_with_hierarchical_opids() {
        let high = vec![HighOp::AddComponent(AddComponentOp {
            opid: Some("create_comp".to_owned()),
            id: Some("R1".to_owned()),
            component_ref: None,
            lib_reference: "R".to_owned(),
            designator: Some("R1".to_owned()),
            value: Some("10K".to_owned()),
            pins: vec![
                AddPinOp {
                    opid: None,
                    id: None,
                    component_ref: None,
                    designator: "1".to_owned(),
                    name: None,
                    electrical: Some("passive".to_owned()),
                    length_mils: Some(25),
                    at: None,
                    rotation: None,
                },
                AddPinOp {
                    opid: None,
                    id: None,
                    component_ref: None,
                    designator: "2".to_owned(),
                    name: None,
                    electrical: Some("passive".to_owned()),
                    length_mils: Some(25),
                    at: None,
                    rotation: None,
                },
            ],
            footprint: Some(FootprintOp {
                model_name: "0805".to_owned(),
                map: vec![
                    FootprintMapEntry {
                        pin: "1".to_owned(),
                        pad: "1".to_owned(),
                    },
                    FootprintMapEntry {
                        pin: "2".to_owned(),
                        pad: "2".to_owned(),
                    },
                ],
            }),
        })];

        let composed = lower_high_ops(&high);
        assert_eq!(composed.len(), 11);
        match &composed[0] {
            ComposedOp::CreateComponentRoot(v) => {
                assert_eq!(v.opid, "create_comp/create_component_root[0]")
            }
            _ => panic!("unexpected op"),
        }
        match &composed[8] {
            ComposedOp::CreateMapDefiner(v) => {
                assert_eq!(v.opid, "create_comp/create_map_definer[0]")
            }
            _ => panic!("unexpected op"),
        }
    }
}

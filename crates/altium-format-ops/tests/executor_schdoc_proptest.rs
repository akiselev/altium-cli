use altium_format::SchDoc;
use altium_format::sch_ops_core::{RecordPatch, RecordSelector};
use altium_format_ops::{
    AddComponentOp, AddLineHighOp, AddParameterOp, AddPinOp, AddRectangleHighOp, ApplySpec,
    EditComponentHighOp, EditRecordHighOp, HighOp, QueryComponentsHighOp, QueryHighOp,
    QueryPinsHighOp, QueryRecordsHighOp, RemoveRecordsHighOp, apply_schdoc, parse_apply_spec_json,
};
use altium_format_types::SchRecordType;
use proptest::prelude::*;

mod harness;
use harness::{list_len_field, save_reopen_schdoc, schdoc_fixture_path};

fn build_ops(plans: Vec<Vec<(u8, i32, i32)>>) -> Vec<HighOp> {
    let mut out = Vec::new();
    let mut op_counter = 0usize;

    for (component_idx, actions) in plans.into_iter().enumerate() {
        let comp_base = format!("comp_{component_idx}");
        let designator = format!("PBT_R_{component_idx}");
        out.push(HighOp::AddComponent(AddComponentOp {
            opid: Some(comp_base.clone()),
            id: None,
            component_ref: None,
            lib_reference: format!("PBT_LIB_{component_idx}"),
            designator: Some(designator.clone()),
            value: Some(format!("V{component_idx}")),
            pins: Vec::new(),
            footprint: None,
        }));

        let cref = altium_format_ops::RefExpr::op(format!("{comp_base}/create_component_root[0]"))
            .member("ref");

        for (code, a, b) in actions {
            let opid = format!("op_{op_counter:04}");
            op_counter += 1;
            match code % 10 {
                0 => out.push(HighOp::AddPin(AddPinOp {
                    opid: Some(opid),
                    id: None,
                    component_ref: Some(cref.clone()),
                    designator: format!("{}", (a.abs() % 200) + 1),
                    name: Some(format!("N{}", b.abs() % 200)),
                    electrical: Some("passive".to_owned()),
                    length_mils: Some((a.abs() % 300) + 10),
                    at: Some((a.abs() % 200, b.abs() % 120)),
                    rotation: Some(0),
                })),
                1 => out.push(HighOp::AddParameter(AddParameterOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    name: "Manufacturer".to_owned(),
                    text: format!("V{}", a.abs() % 500),
                    is_hidden: Some((b & 1) == 1),
                })),
                2 => out.push(HighOp::AddLine(AddLineHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    from: (a.abs() % 200, b.abs() % 200),
                    to: ((a.abs() % 200) + 20, (b.abs() % 200) + 20),
                    color: Some(0),
                    line_width: Some((a.abs() % 3) + 1),
                    line_style: Some(a.abs() % 4),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                3 => out.push(HighOp::AddRectangle(AddRectangleHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    from: (a.abs() % 200, b.abs() % 200),
                    to: ((a.abs() % 200) + 40, (b.abs() % 200) + 30),
                    color: Some(0),
                    area_color: Some(0x00FF_FFFF),
                    is_solid: Some(true),
                    transparent: Some(false),
                    line_width: Some((a.abs() % 3) + 1),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                4 => out.push(HighOp::EditComponent(EditComponentHighOp {
                    opid: Some(opid),
                    component_ref: cref.clone(),
                    description: Some(format!("DESC_{a}")),
                    part_count: Some((a.abs() % 3) + 1),
                    display_mode_count: None,
                    component_kind: Some(0),
                    show_hidden_pins: Some((b & 1) == 1),
                })),
                5 => out.push(HighOp::EditRecord(EditRecordHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    selector: RecordSelector::ByName("Manufacturer".to_owned()),
                    patch: RecordPatch {
                        text: Some(format!("M{}", a.abs() % 1000)),
                        ..RecordPatch::default()
                    },
                })),
                6 => out.push(HighOp::RemoveRecords(RemoveRecordsHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    selector: RecordSelector::ByRecordType(SchRecordType::Parameter as i32),
                })),
                7 => out.push(HighOp::QueryPins(QueryPinsHighOp {
                    opid: Some(opid),
                    component_ref: cref.clone(),
                })),
                8 => out.push(HighOp::QueryRecords(QueryRecordsHighOp {
                    opid: Some(opid),
                    component_ref: cref.clone(),
                    record_type: Some(SchRecordType::Parameter as i32),
                })),
                _ => out.push(HighOp::Query(QueryHighOp {
                    opid: Some(opid),
                    selector: format!("component[designator={designator}]"),
                })),
            }
        }
    }
    out.push(HighOp::QueryComponents(QueryComponentsHighOp {
        opid: Some("tail_query_components".to_owned()),
        pattern: Some("PBT_LIB_".to_owned()),
    }));
    out
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 40,
        .. ProptestConfig::default()
    })]
    #[test]
    fn schdoc_generated_programs_are_stable_and_roundtrip(
        plans in prop::collection::vec(
            prop::collection::vec((0u8..=9, -500i32..=500, -500i32..=500), 0..=14),
            1..=3
        )
    ) {
        let ops = build_ops(plans);
        let json = serde_json::to_string(&ApplySpec::Wrapped { ops: ops.clone() })
            .expect("serialize generated JSON");
        let parsed = parse_apply_spec_json(&json).expect("parse generated JSON");

        let input = schdoc_fixture_path(
            "myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc",
        );
        let mut doc_direct = SchDoc::open(&input).expect("open fixture direct");
        let mut doc_parsed = SchDoc::open(&input).expect("open fixture parsed");

        let report_direct = apply_schdoc(&mut doc_direct, &ops).expect("apply direct generated ops");
        let report_parsed =
            apply_schdoc(&mut doc_parsed, &parsed).expect("apply parsed generated ops");

        prop_assert_eq!(report_direct.high_op_count, report_parsed.high_op_count);
        prop_assert_eq!(report_direct.composed_op_count, report_parsed.composed_op_count);
        prop_assert_eq!(report_direct.low_op_count, report_parsed.low_op_count);
        prop_assert_eq!(report_direct.results.len(), report_parsed.results.len());
        prop_assert!(report_direct.results.len() >= report_direct.low_op_count);
        prop_assert_eq!(
            list_len_field(&report_direct, "tail_query_components", "components"),
            list_len_field(&report_parsed, "tail_query_components", "components")
        );

        doc_direct.validate_invariants().expect("direct invariants");
        doc_parsed.validate_invariants().expect("parsed invariants");
        save_reopen_schdoc(&doc_direct);
    }
}

#[test]
fn schdoc_model_based_manual_vs_json_equivalent() {
    let ops = vec![
        HighOp::AddComponent(AddComponentOp {
            opid: Some("c0".to_owned()),
            id: None,
            component_ref: None,
            lib_reference: "MODEL_EQ_SCHDOC".to_owned(),
            designator: Some("R?".to_owned()),
            value: Some("10k".to_owned()),
            pins: vec![],
            footprint: None,
        }),
        HighOp::AddPin(AddPinOp {
            opid: Some("p0".to_owned()),
            id: None,
            component_ref: Some(
                altium_format_ops::RefExpr::op("c0/create_component_root[0]").member("ref"),
            ),
            designator: "1".to_owned(),
            name: Some("P1".to_owned()),
            electrical: Some("passive".to_owned()),
            length_mils: Some(120),
            at: Some((0, 0)),
            rotation: Some(0),
        }),
        HighOp::Query(QueryHighOp {
            opid: Some("q0".to_owned()),
            selector: "component[designator=R?]".to_owned(),
        }),
    ];

    let json = serde_json::to_string(&ApplySpec::Wrapped { ops: ops.clone() }).expect("serialize");
    let parsed = parse_apply_spec_json(&json).expect("parse");

    let input = schdoc_fixture_path(
        "myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc",
    );
    let mut doc_manual = SchDoc::open(&input).expect("open fixture manual");
    let mut doc_json = SchDoc::open(&input).expect("open fixture json");

    let report_manual = apply_schdoc(&mut doc_manual, &ops).expect("apply manual");
    let report_json = apply_schdoc(&mut doc_json, &parsed).expect("apply json");

    assert_eq!(report_manual.low_op_count, report_json.low_op_count);
    assert_eq!(
        format!("{:?}", report_manual.results.get("q0").expect("q0 manual")),
        format!("{:?}", report_json.results.get("q0").expect("q0 json"))
    );
}

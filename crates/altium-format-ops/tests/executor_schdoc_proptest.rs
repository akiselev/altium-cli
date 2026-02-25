#![cfg(feature = "proptest")]

use altium_format::SchDoc;
use altium_format::sch_ops_core::{RecordPatch, RecordSelector};
use altium_format_ops::{
    AddArcHighOp, AddBezierHighOp, AddComponentOp, AddEllipseHighOp, AddLabelHighOp, AddLineHighOp,
    AddParameterOp, AddPieHighOp, AddPinOp, AddPolygonHighOp, AddPolylineHighOp,
    AddRectangleHighOp, AddRoundRectangleHighOp, ApplySpec, EditComponentHighOp, EditRecordHighOp,
    HighOp, QueryComponentsHighOp, QueryHighOp, QueryPinsHighOp, QueryRecordsHighOp,
    RemoveRecordsHighOp, apply_schdoc, parse_apply_spec_json,
};
use altium_format_types::SchRecordType;
use proptest::prelude::*;

mod harness;
use harness::{list_len_field, save_reopen_schdoc, schdoc_fixture_path};

fn edge_i32(v: i32, fallback: i32) -> i32 {
    match v.rem_euclid(9) {
        0 => -5000,
        1 => -1000,
        2 => -1,
        3 => 0,
        4 => 1,
        5 => 1000,
        6 => 5000,
        _ => fallback,
    }
}

fn norm_u8(v: i32, max: i32) -> i32 {
    v.abs() % max.max(1)
}

fn poly_points(a: i32, b: i32, c: i32) -> Vec<(i32, i32)> {
    let x0 = edge_i32(a, a % 1500);
    let y0 = edge_i32(b, b % 1500);
    vec![
        (x0, y0),
        (x0 + 20 + norm_u8(c, 80), y0 + 10),
        (x0 + 40, y0 + 20 + norm_u8(a, 80)),
        (x0 + 70, y0 + 5 + norm_u8(b, 70)),
    ]
}

fn build_ops(plans: Vec<Vec<(u8, i32, i32, i32)>>) -> Vec<HighOp> {
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

        for (code, a, b, c) in actions {
            let opid = format!("op_{op_counter:04}");
            op_counter += 1;
            match code % 18 {
                0 => out.push(HighOp::AddPin(AddPinOp {
                    opid: Some(opid),
                    id: None,
                    component_ref: Some(cref.clone()),
                    designator: format!("{}", (a.abs() % 200) + 1),
                    name: Some(format!("N{}", b.abs() % 200)),
                    electrical: Some("passive".to_owned()),
                    length_mils: Some((a.abs() % 300) + 10),
                    at: Some((edge_i32(a, a % 1500), edge_i32(b, b % 1500))),
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
                    from: (edge_i32(a, a % 1500), edge_i32(b, b % 1500)),
                    to: (
                        edge_i32(a + 20 + norm_u8(c, 70), (a + 20) % 1500),
                        edge_i32(b + 20 + norm_u8(a, 70), (b + 20) % 1500),
                    ),
                    color: Some(0),
                    line_width: Some((a.abs() % 3) + 1),
                    line_style: Some(norm_u8(c, 4)),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                3 => out.push(HighOp::AddRectangle(AddRectangleHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    from: (edge_i32(a, a % 1400), edge_i32(b, b % 1400)),
                    to: (
                        edge_i32(a + 40 + norm_u8(c, 90), (a + 40) % 1400),
                        edge_i32(b + 30 + norm_u8(a, 90), (b + 30) % 1400),
                    ),
                    color: Some(0),
                    area_color: Some(0x00FF_FFFF),
                    is_solid: Some((c & 1) == 0),
                    transparent: Some((c & 2) != 0),
                    line_width: Some((a.abs() % 3) + 1),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                4 => out.push(HighOp::EditComponent(EditComponentHighOp {
                    opid: Some(opid),
                    component_ref: cref.clone(),
                    description: Some(format!("DESC_{a}")),
                    part_count: Some((a.abs() % 4) + 1),
                    display_mode_count: None,
                    component_kind: Some(0),
                    show_hidden_pins: Some((b & 1) == 1),
                })),
                5 => out.push(HighOp::EditRecord(EditRecordHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    selector: RecordSelector::ByName("Manufacturer".to_owned()),
                    patch: RecordPatch {
                        text: Some(format!("M{}_{}", a.abs() % 1000, c.abs() % 100)),
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
                9 => out.push(HighOp::Query(QueryHighOp {
                    opid: Some(opid),
                    selector: format!("component[designator={designator}]"),
                })),
                10 => out.push(HighOp::AddArc(AddArcHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    cx_mils: edge_i32(a, a % 1800),
                    cy_mils: edge_i32(b, b % 1800),
                    radius_mils: 1 + norm_u8(c, 500),
                    start_angle: Some((a % 720) as f64 - 360.0),
                    end_angle: Some((b % 720) as f64),
                    color: Some(0),
                    line_width: Some(1 + norm_u8(c, 3)),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                11 => out.push(HighOp::AddEllipse(AddEllipseHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    cx_mils: edge_i32(a, a % 1800),
                    cy_mils: edge_i32(b, b % 1800),
                    radius_mils: 1 + norm_u8(c, 500),
                    secondary_radius_mils: 1 + norm_u8(a, 500),
                    color: Some(0),
                    area_color: Some(0x00ff_ffff),
                    is_solid: Some((c & 1) == 0),
                    line_width: Some(1 + norm_u8(a, 3)),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                12 => out.push(HighOp::AddPolyline(AddPolylineHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    points_mils: poly_points(a, b, c),
                    color: Some(0),
                    line_width: Some(1 + norm_u8(c, 3)),
                    line_style: Some(norm_u8(a, 4)),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                13 => out.push(HighOp::AddPolygon(AddPolygonHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    points_mils: poly_points(a + 5, b + 7, c + 11),
                    color: Some(0),
                    area_color: Some(0x00ff_ffff),
                    is_solid: Some((c & 1) == 0),
                    line_width: Some(1 + norm_u8(c, 3)),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                14 => out.push(HighOp::AddBezier(AddBezierHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    points_mils: poly_points(a + 17, b + 19, c + 23),
                    color: Some(0),
                    line_width: Some(1 + norm_u8(c, 3)),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                15 => out.push(HighOp::AddPie(AddPieHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    cx_mils: edge_i32(a, a % 1400),
                    cy_mils: edge_i32(b, b % 1400),
                    radius_mils: 1 + norm_u8(c, 500),
                    start_angle: Some((a % 360) as f64),
                    end_angle: Some((b % 540) as f64 - 180.0),
                    color: Some(0),
                    area_color: Some(0x00ff_ffff),
                    is_solid: Some((c & 1) == 0),
                    line_width: Some(1 + norm_u8(a, 3)),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                16 => out.push(HighOp::AddRoundRectangle(AddRoundRectangleHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    from: (edge_i32(a, a % 1400), edge_i32(b, b % 1400)),
                    to: (
                        edge_i32(a + 30 + norm_u8(c, 60), (a + 30) % 1400),
                        edge_i32(b + 25 + norm_u8(a, 60), (b + 25) % 1400),
                    ),
                    corner_x_radius_mils: 1 + norm_u8(c, 40),
                    corner_y_radius_mils: 1 + norm_u8(a, 40),
                    color: Some(0),
                    area_color: Some(0x00ff_ffff),
                    is_solid: Some((b & 1) == 0),
                    line_width: Some(1 + norm_u8(a, 3)),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                _ => out.push(HighOp::AddLabel(AddLabelHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    x_mils: edge_i32(a, a % 1500),
                    y_mils: edge_i32(b, b % 1500),
                    text: format!("LBL_{}_{}", norm_u8(a, 100), norm_u8(c, 100)),
                    color: Some(0),
                    font_id: Some(1),
                    orientation: Some(norm_u8(c, 4)),
                    justification: Some(norm_u8(a, 9)),
                    is_mirrored: Some((b & 1) != 0),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
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

fn run_schdoc_stability_program(plans: Vec<Vec<(u8, i32, i32, i32)>>) {
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
    let report_parsed = apply_schdoc(&mut doc_parsed, &parsed).expect("apply parsed generated ops");

    assert_eq!(report_direct.high_op_count, report_parsed.high_op_count);
    assert_eq!(
        report_direct.composed_op_count,
        report_parsed.composed_op_count
    );
    assert_eq!(report_direct.low_op_count, report_parsed.low_op_count);
    assert_eq!(report_direct.results.len(), report_parsed.results.len());
    assert!(report_direct.results.len() >= report_direct.low_op_count);
    assert_eq!(
        list_len_field(&report_direct, "tail_query_components", "components"),
        list_len_field(&report_parsed, "tail_query_components", "components")
    );

    doc_direct.validate_invariants().expect("direct invariants");
    doc_parsed.validate_invariants().expect("parsed invariants");
    save_reopen_schdoc(&doc_direct);
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 48,
        .. ProptestConfig::default()
    })]
    #[test]
    fn schdoc_generated_programs_are_stable_and_roundtrip_smoke(
        plans in prop::collection::vec(
            prop::collection::vec((0u8..=17, -6000i32..=6000, -6000i32..=6000, -2000i32..=2000), 0..=24),
            1..=4
        )
    ) {
        run_schdoc_stability_program(plans);
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 180,
        .. ProptestConfig::default()
    })]
    #[test]
    #[ignore = "nightly stress property suite"]
    fn schdoc_generated_programs_are_stable_and_roundtrip_nightly(
        plans in prop::collection::vec(
            prop::collection::vec((0u8..=17, -20000i32..=20000, -20000i32..=20000, -5000i32..=5000), 0..=80),
            1..=8
        )
    ) {
        run_schdoc_stability_program(plans);
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 24,
        .. ProptestConfig::default()
    })]
    #[test]
    fn schdoc_query_ops_are_state_noops_metamorphic_smoke(
        plans in prop::collection::vec(
            prop::collection::vec((0u8..=17, -4000i32..=4000, -4000i32..=4000, -1500i32..=1500), 0..=16),
            1..=3
        )
    ) {
        let input = schdoc_fixture_path(
            "myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc",
        );
        let mut doc = SchDoc::open(&input).expect("open fixture");
        let mut_ops = build_ops(plans);
        apply_schdoc(&mut doc, &mut_ops).expect("apply mutating ops");
        doc.validate_invariants().expect("mutated invariants");

        let query_ops = [
            HighOp::QueryComponents(QueryComponentsHighOp {
                opid: Some("q_components".to_owned()),
                pattern: Some("PBT_LIB_".to_owned()),
            }),
            HighOp::Query(QueryHighOp {
                opid: Some("q_selector".to_owned()),
                selector: "component".to_owned(),
            }),
        ];
        let report_a = apply_schdoc(&mut doc, &query_ops).expect("apply query ops A");
        let report_b = apply_schdoc(&mut doc, &query_ops).expect("apply query ops B");
        doc.validate_invariants().expect("post-query invariants");

        prop_assert_eq!(
            list_len_field(&report_a, "q_components", "components"),
            list_len_field(&report_b, "q_components", "components")
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 20,
        .. ProptestConfig::default()
    })]
    #[test]
    fn schdoc_remove_records_is_idempotent_metamorphic_smoke(
        salt in 0u16..2000u16,
        text_seed in -2000i32..=2000
    ) {
        let input = schdoc_fixture_path(
            "myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc",
        );
        let mut once = SchDoc::open(&input).expect("open once");
        let mut twice = SchDoc::open(&input).expect("open twice");
        let mut add_and_remove_once = vec![
            HighOp::AddComponent(AddComponentOp {
                opid: Some("rm_comp".to_owned()),
                id: None,
                component_ref: None,
                lib_reference: format!("RM_{salt}"),
                designator: Some("U?".to_owned()),
                value: Some("1".to_owned()),
                pins: Vec::new(),
                footprint: None,
            }),
            HighOp::AddParameter(AddParameterOp {
                opid: Some("rm_param".to_owned()),
                component_ref: Some(
                    altium_format_ops::RefExpr::op("rm_comp/create_component_root[0]")
                        .member("ref"),
                ),
                name: "Manufacturer".to_owned(),
                text: format!("T{}", text_seed.abs()),
                is_hidden: Some(false),
            }),
            HighOp::RemoveRecords(RemoveRecordsHighOp {
                opid: Some("rm1".to_owned()),
                component_ref: Some(
                    altium_format_ops::RefExpr::op("rm_comp/create_component_root[0]")
                        .member("ref"),
                ),
                selector: RecordSelector::ByRecordType(SchRecordType::Parameter as i32),
            }),
        ];
        let mut add_and_remove_twice = add_and_remove_once.clone();
        add_and_remove_twice.push(HighOp::RemoveRecords(RemoveRecordsHighOp {
            opid: Some("rm2".to_owned()),
            component_ref: Some(
                altium_format_ops::RefExpr::op("rm_comp/create_component_root[0]")
                    .member("ref"),
            ),
            selector: RecordSelector::ByRecordType(SchRecordType::Parameter as i32),
        }));

        add_and_remove_once.push(HighOp::QueryRecords(QueryRecordsHighOp {
            opid: Some("q_rm".to_owned()),
            component_ref: altium_format_ops::RefExpr::op("rm_comp/create_component_root[0]")
                .member("ref"),
            record_type: Some(SchRecordType::Parameter as i32),
        }));
        add_and_remove_twice.push(HighOp::QueryRecords(QueryRecordsHighOp {
            opid: Some("q_rm".to_owned()),
            component_ref: altium_format_ops::RefExpr::op("rm_comp/create_component_root[0]")
                .member("ref"),
            record_type: Some(SchRecordType::Parameter as i32),
        }));

        let report_once = apply_schdoc(&mut once, &add_and_remove_once).expect("remove once batch");
        let report_twice =
            apply_schdoc(&mut twice, &add_and_remove_twice).expect("remove twice batch");

        once.validate_invariants().expect("once invariants");
        twice.validate_invariants().expect("twice invariants");
        prop_assert_eq!(
            list_len_field(&report_once, "q_rm", "records"),
            list_len_field(&report_twice, "q_rm", "records")
        );
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

#![cfg(feature = "proptest")]

use altium_format::SchLib;
use altium_format::sch_ops_core::{RecordPatch, RecordSelector};
use altium_format_ops::{
    AddAliasOp, AddArcHighOp, AddBezierHighOp, AddComponentOp, AddEllipseHighOp, AddLabelHighOp,
    AddLineHighOp, AddParameterOp, AddPieHighOp, AddPinOp, AddPolygonHighOp, AddPolylineHighOp,
    AddRectangleHighOp, AddRoundRectangleHighOp, AddTextFrameHighOp, ApplySpec,
    EditComponentHighOp, EditRecordHighOp, HighOp, QueryComponentsHighOp, QueryHighOp,
    QueryPinsHighOp, QueryRecordsHighOp, RefExpr, RemoveRecordsHighOp, apply_schlib,
    parse_apply_spec_json,
};
use proptest::prelude::*;

mod harness;
use harness::{list_len_field, save_reopen_schlib, schlib_fixture_path};

fn component_root_ref(base_opid: &str) -> RefExpr {
    RefExpr::op(format!("{base_opid}/create_component_root[0]")).member("ref")
}

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

fn build_ops(plans: Vec<Vec<(u8, i32, i32, i32, i32)>>) -> Vec<HighOp> {
    let mut out = Vec::new();
    let mut op_counter = 0usize;

    for (component_idx, actions) in plans.into_iter().enumerate() {
        let comp_base = format!("comp_{component_idx}");
        let lib_ref = format!("PROP_{component_idx}");
        out.push(HighOp::AddComponent(AddComponentOp {
            opid: Some(comp_base.clone()),
            id: None,
            component_ref: None,
            lib_reference: lib_ref,
            designator: Some(format!("U{component_idx}?")),
            value: None,
            pins: Vec::new(),
            footprint: None,
        }));

        let cref = component_root_ref(&comp_base);

        for (code, a, b, c, d) in actions {
            let opid = format!("op_{op_counter:04}");
            op_counter += 1;
            match code % 19 {
                0 => out.push(HighOp::AddPin(AddPinOp {
                    opid: Some(opid),
                    id: None,
                    component_ref: Some(cref.clone()),
                    designator: format!("{}", (a.abs() % 200) + 1),
                    name: Some(format!("P{}", b.abs() % 200)),
                    electrical: Some("passive".to_owned()),
                    length_mils: Some(5 + norm_u8(c, 800)),
                    at: Some((edge_i32(a, a % 1500), edge_i32(b, b % 1500))),
                    rotation: Some((norm_u8(d, 4) * 90) as i32),
                })),
                1 => out.push(HighOp::AddParameter(AddParameterOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    name: if (c & 1) == 0 {
                        "Manufacturer".to_owned()
                    } else {
                        format!("P{}", norm_u8(a, 32))
                    },
                    text: format!("V{}_{}", a.abs() % 500, d.abs() % 17),
                    is_hidden: Some((b & 1) == 1),
                })),
                2 => out.push(HighOp::AddLine(AddLineHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    from: (edge_i32(a, a % 1500), edge_i32(b, b % 1500)),
                    to: (
                        edge_i32(a + 20 + norm_u8(c, 70), (a + 20) % 1500),
                        edge_i32(b + 20 + norm_u8(d, 70), (b + 20) % 1500),
                    ),
                    color: Some(norm_u8(d, 2) * 0x00ff_ffff),
                    line_width: Some((c.abs() % 3) + 1),
                    line_style: Some(c.abs() % 4),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                3 => out.push(HighOp::AddRectangle(AddRectangleHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    from: (edge_i32(a, a % 1400), edge_i32(b, b % 1400)),
                    to: (
                        edge_i32(a + 40 + norm_u8(c, 90), (a + 40) % 1400),
                        edge_i32(b + 30 + norm_u8(d, 90), (b + 30) % 1400),
                    ),
                    color: Some(0),
                    area_color: Some(0x00ff_ffff),
                    is_solid: Some((d & 1) == 0),
                    transparent: Some((d & 2) != 0),
                    line_width: Some((c.abs() % 3) + 1),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                4 => out.push(HighOp::AddAlias(AddAliasOp {
                    opid: Some(opid),
                    component_ref: cref.clone(),
                    alias_name: format!("ALIAS_{component_idx}_{op_counter}"),
                })),
                5 => out.push(HighOp::EditComponent(EditComponentHighOp {
                    opid: Some(opid),
                    component_ref: cref.clone(),
                    description: Some(format!("DESC_{a}")),
                    part_count: Some((a.abs() % 4) + 1),
                    display_mode_count: None,
                    component_kind: Some(0),
                    show_hidden_pins: Some((b & 1) == 1),
                })),
                6 => out.push(HighOp::EditRecord(EditRecordHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    selector: RecordSelector::ByName("Manufacturer".to_owned()),
                    patch: RecordPatch {
                        text: Some(format!("M{}_{}", a.abs() % 1000, d.abs() % 50)),
                        ..RecordPatch::default()
                    },
                })),
                7 => out.push(HighOp::QueryPins(QueryPinsHighOp {
                    opid: Some(opid),
                    component_ref: cref.clone(),
                })),
                8 => out.push(HighOp::QueryRecords(QueryRecordsHighOp {
                    opid: Some(opid),
                    component_ref: cref.clone(),
                    record_type: Some(41),
                })),
                9 => out.push(HighOp::RemoveRecords(RemoveRecordsHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    selector: RecordSelector::ByRecordType(41),
                })),
                10 => out.push(HighOp::AddArc(AddArcHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    cx_mils: edge_i32(a, a % 1800),
                    cy_mils: edge_i32(b, b % 1800),
                    radius_mils: 1 + norm_u8(c, 600),
                    start_angle: Some((d % 720) as f64 - 360.0),
                    end_angle: Some((a % 720) as f64),
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
                    radius_mils: 1 + norm_u8(c, 600),
                    secondary_radius_mils: 1 + norm_u8(d, 600),
                    color: Some(0),
                    area_color: Some(0x00ff_ffff),
                    is_solid: Some((d & 1) == 0),
                    line_width: Some(1 + norm_u8(a, 3)),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                12 => out.push(HighOp::AddPolyline(AddPolylineHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    points_mils: poly_points(a, b, c),
                    color: Some(0),
                    line_width: Some(1 + norm_u8(d, 3)),
                    line_style: Some(norm_u8(c, 4)),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                13 => out.push(HighOp::AddPolygon(AddPolygonHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    points_mils: poly_points(a + 3, b + 5, c + 7),
                    color: Some(0),
                    area_color: Some(0x00ff_ffff),
                    is_solid: Some((d & 1) == 0),
                    line_width: Some(1 + norm_u8(d, 3)),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                14 => out.push(HighOp::AddBezier(AddBezierHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    points_mils: poly_points(a + 11, b + 13, c + 17),
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
                    end_angle: Some((d % 540) as f64 - 180.0),
                    color: Some(0),
                    area_color: Some(0x00ff_ffff),
                    is_solid: Some((d & 1) == 0),
                    line_width: Some(1 + norm_u8(b, 3)),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                16 => out.push(HighOp::AddRoundRectangle(AddRoundRectangleHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    from: (edge_i32(a, a % 1400), edge_i32(b, b % 1400)),
                    to: (
                        edge_i32(a + 30 + norm_u8(c, 60), (a + 30) % 1400),
                        edge_i32(b + 25 + norm_u8(d, 60), (b + 25) % 1400),
                    ),
                    corner_x_radius_mils: 1 + norm_u8(c, 40),
                    corner_y_radius_mils: 1 + norm_u8(d, 40),
                    color: Some(0),
                    area_color: Some(0x00ff_ffff),
                    is_solid: Some((a & 1) == 0),
                    line_width: Some(1 + norm_u8(a, 3)),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                17 => out.push(HighOp::AddLabel(AddLabelHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    x_mils: edge_i32(a, a % 1500),
                    y_mils: edge_i32(b, b % 1500),
                    text: format!("LBL_{}_{}", norm_u8(c, 100), norm_u8(d, 100)),
                    color: Some(0),
                    font_id: Some(1),
                    orientation: Some(norm_u8(c, 4)),
                    justification: Some(norm_u8(d, 9)),
                    is_mirrored: Some((a & 1) != 0),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                18 => out.push(HighOp::AddTextFrame(AddTextFrameHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    from: (edge_i32(a, a % 1500), edge_i32(b, b % 1500)),
                    to: (
                        edge_i32(a + 60 + norm_u8(c, 80), (a + 60) % 1500),
                        edge_i32(b + 35 + norm_u8(d, 80), (b + 35) % 1500),
                    ),
                    text: format!("TF_{}_{}", norm_u8(a, 999), norm_u8(b, 999)),
                    color: Some(0),
                    area_color: Some(0x00ff_ffff),
                    font_id: Some(1),
                    alignment: Some(norm_u8(c, 9)),
                    word_wrap: Some((d & 1) == 0),
                    show_border: Some((d & 2) == 0),
                    is_solid: Some((d & 4) == 0),
                    clip_to_rect: Some((d & 8) == 0),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                _ => unreachable!("code % 19 must be in 0..=18"),
            }
        }
    }

    out.push(HighOp::AddTextFrame(AddTextFrameHighOp {
        opid: Some("tail_textframe".to_owned()),
        component_ref: Some(component_root_ref("comp_0")),
        from: (0, 0),
        to: (60, 30),
        text: "tail".to_owned(),
        color: Some(0),
        area_color: Some(0x00FF_FFFF),
        font_id: Some(1),
        alignment: Some(0),
        word_wrap: Some(false),
        show_border: Some(true),
        is_solid: Some(true),
        clip_to_rect: Some(false),
        owner_part_id: Some(0),
        owner_part_display_mode: Some(0),
    }));
    out.push(HighOp::QueryComponents(QueryComponentsHighOp {
        opid: Some("tail_query_components".to_owned()),
        pattern: Some("PROP_".to_owned()),
    }));
    out
}

fn run_schlib_stability_program(plans: Vec<Vec<(u8, i32, i32, i32, i32)>>) {
    let ops = build_ops(plans);

    let wrapped = ApplySpec::Wrapped { ops: ops.clone() };
    let json = serde_json::to_string(&wrapped).expect("serialize generated JSON");
    let parsed = parse_apply_spec_json(&json).expect("parse generated JSON");

    let input = schlib_fixture_path("Resistors_Caps.SchLib");
    let mut lib_direct = SchLib::open(&input).expect("open fixture direct");
    let mut lib_parsed = SchLib::open(&input).expect("open fixture parsed");

    let report_direct = apply_schlib(&mut lib_direct, &ops).expect("apply direct generated ops");
    let report_parsed = apply_schlib(&mut lib_parsed, &parsed).expect("apply parsed generated ops");

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

    save_reopen_schlib(&lib_direct);
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 36,
        .. ProptestConfig::default()
    })]
    #[test]
    fn schlib_generated_programs_are_stable_and_roundtrip_smoke(
        plans in prop::collection::vec(
            prop::collection::vec(
                (0u8..=18, -5000i32..=5000, -5000i32..=5000, -2000i32..=2000, -720i32..=720),
                0..=22
            ),
            1..=4
        )
    ) {
        run_schlib_stability_program(plans);
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 180,
        .. ProptestConfig::default()
    })]
    #[test]
    #[ignore = "nightly stress property suite"]
    fn schlib_generated_programs_are_stable_and_roundtrip_nightly(
        plans in prop::collection::vec(
            prop::collection::vec(
                (0u8..=18, -20000i32..=20000, -20000i32..=20000, -4000i32..=4000, -1440i32..=1440),
                0..=70
            ),
            1..=8
        )
    ) {
        run_schlib_stability_program(plans);
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 28,
        .. ProptestConfig::default()
    })]
    #[test]
    fn schlib_query_ops_are_state_noops_metamorphic_smoke(
        plans in prop::collection::vec(
            prop::collection::vec(
                (0u8..=18, -3000i32..=3000, -3000i32..=3000, -1200i32..=1200, -360i32..=360),
                0..=16
            ),
            1..=3
        )
    ) {
        let input = schlib_fixture_path("Resistors_Caps.SchLib");
        let mut lib = SchLib::open(&input).expect("open fixture");
        let mut_ops = build_ops(plans);
        apply_schlib(&mut lib, &mut_ops).expect("apply mutating ops");
        lib.validate_invariants().expect("mutated invariants");

        let query_ops = [
            HighOp::QueryComponents(QueryComponentsHighOp {
                opid: Some("q_components".to_owned()),
                pattern: Some("PROP_".to_owned()),
            }),
            HighOp::Query(QueryHighOp {
                opid: Some("q_selector".to_owned()),
                selector: "component".to_owned(),
            }),
        ];
        let report_a = apply_schlib(&mut lib, &query_ops).expect("apply query ops A");
        let report_b = apply_schlib(&mut lib, &query_ops).expect("apply query ops B");
        lib.validate_invariants().expect("post-query invariants");

        prop_assert_eq!(
            list_len_field(&report_a, "q_components", "components"),
            list_len_field(&report_b, "q_components", "components")
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 24,
        .. ProptestConfig::default()
    })]
    #[test]
    fn schlib_remove_records_is_idempotent_metamorphic_smoke(
        salt in 0u16..2000u16,
        text_seed in -2000i32..=2000
    ) {
        let input = schlib_fixture_path("Resistors_Caps.SchLib");
        let mut once = SchLib::open(&input).expect("open fixture once");
        let mut twice = SchLib::open(&input).expect("open fixture twice");
        let mut add_and_remove_once = vec![
            HighOp::AddComponent(AddComponentOp {
                opid: Some("rm_comp".to_owned()),
                id: None,
                component_ref: None,
                lib_reference: format!("RM_{salt}"),
                designator: Some("U?".to_owned()),
                value: None,
                pins: Vec::new(),
                footprint: None,
            }),
            HighOp::AddParameter(AddParameterOp {
                opid: Some("rm_param".to_owned()),
                component_ref: Some(component_root_ref("rm_comp")),
                name: "Manufacturer".to_owned(),
                text: format!("T{}", text_seed.abs()),
                is_hidden: Some(false),
            }),
            HighOp::RemoveRecords(RemoveRecordsHighOp {
                opid: Some("rm1".to_owned()),
                component_ref: Some(component_root_ref("rm_comp")),
                selector: RecordSelector::ByRecordType(41),
            }),
        ];
        let mut add_and_remove_twice = add_and_remove_once.clone();
        add_and_remove_twice.push(HighOp::RemoveRecords(RemoveRecordsHighOp {
            opid: Some("rm2".to_owned()),
            component_ref: Some(component_root_ref("rm_comp")),
            selector: RecordSelector::ByRecordType(41),
        }));

        add_and_remove_once.push(HighOp::QueryRecords(QueryRecordsHighOp {
            opid: Some("q_rm".to_owned()),
            component_ref: component_root_ref("rm_comp"),
            record_type: Some(41),
        }));
        add_and_remove_twice.push(HighOp::QueryRecords(QueryRecordsHighOp {
            opid: Some("q_rm".to_owned()),
            component_ref: component_root_ref("rm_comp"),
            record_type: Some(41),
        }));

        let report_once = apply_schlib(&mut once, &add_and_remove_once).expect("remove once batch");
        let report_twice =
            apply_schlib(&mut twice, &add_and_remove_twice).expect("remove twice batch");

        once.validate_invariants().expect("once invariants");
        twice.validate_invariants().expect("twice invariants");

        prop_assert_eq!(
            list_len_field(&report_once, "q_rm", "records"),
            list_len_field(&report_twice, "q_rm", "records")
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 120,
        .. ProptestConfig::default()
    })]
    #[test]
    #[ignore = "nightly stress property suite"]
    fn schlib_metamorphic_nightly(
        plans in prop::collection::vec(
            prop::collection::vec(
                (0u8..=18, -12000i32..=12000, -12000i32..=12000, -3000i32..=3000, -1080i32..=1080),
                0..=40
            ),
            1..=6
        )
    ) {
        let input = schlib_fixture_path("Resistors_Caps.SchLib");
        let mut lib = SchLib::open(&input).expect("open fixture");
        apply_schlib(&mut lib, &build_ops(plans)).expect("apply base");
        let report_a = apply_schlib(
            &mut lib,
            &[HighOp::QueryComponents(QueryComponentsHighOp {
                opid: Some("night_q".to_owned()),
                pattern: Some("PROP_".to_owned()),
            })],
        )
        .expect("apply query A");
        let report_b = apply_schlib(
            &mut lib,
            &[HighOp::QueryComponents(QueryComponentsHighOp {
                opid: Some("night_q".to_owned()),
                pattern: Some("PROP_".to_owned()),
            })],
        )
        .expect("apply query B");
        prop_assert_eq!(
            list_len_field(&report_a, "night_q", "components"),
            list_len_field(&report_b, "night_q", "components")
        );
    }
}

#[test]
fn schlib_model_based_manual_vs_yaml_equivalent() {
    let ops = vec![
        HighOp::AddComponent(AddComponentOp {
            opid: Some("c0".to_owned()),
            id: None,
            component_ref: None,
            lib_reference: "MODEL_EQ_0".to_owned(),
            designator: Some("U?".to_owned()),
            value: None,
            pins: vec![AddPinOp {
                opid: None,
                id: None,
                component_ref: None,
                designator: "1".to_owned(),
                name: Some("P1".to_owned()),
                electrical: Some("passive".to_owned()),
                length_mils: Some(100),
                at: Some((0, 0)),
                rotation: Some(0),
            }],
            footprint: None,
        }),
        HighOp::AddParameter(AddParameterOp {
            opid: Some("p0".to_owned()),
            component_ref: Some(component_root_ref("c0")),
            name: "Manufacturer".to_owned(),
            text: "ACME".to_owned(),
            is_hidden: Some(false),
        }),
        HighOp::QueryRecords(QueryRecordsHighOp {
            opid: Some("q0".to_owned()),
            component_ref: component_root_ref("c0"),
            record_type: Some(41),
        }),
    ];

    let json = serde_json::to_string(&ApplySpec::Wrapped { ops: ops.clone() }).expect("serialize");
    let parsed = parse_apply_spec_json(&json).expect("parse");

    let input = schlib_fixture_path("Resistors_Caps.SchLib");
    let mut lib_manual = SchLib::open(&input).expect("open fixture manual");
    let mut lib_yaml = SchLib::open(&input).expect("open fixture yaml");

    let report_manual = apply_schlib(&mut lib_manual, &ops).expect("apply manual");
    let report_yaml = apply_schlib(&mut lib_yaml, &parsed).expect("apply yaml");

    assert_eq!(report_manual.low_op_count, report_yaml.low_op_count);
    assert_eq!(
        format!(
            "{:?}",
            report_manual
                .results
                .get("q0")
                .expect("q0 manual")
                .fields
                .get("records")
        ),
        format!(
            "{:?}",
            report_yaml
                .results
                .get("q0")
                .expect("q0 yaml")
                .fields
                .get("records")
        )
    );
}

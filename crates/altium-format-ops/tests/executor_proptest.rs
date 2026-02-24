use altium_format::SchLib;
use altium_format::sch_ops_core::{RecordPatch, RecordSelector};
use altium_format_ops::{
    AddAliasOp, AddComponentOp, AddLineHighOp, AddParameterOp, AddPinOp, AddRectangleHighOp,
    AddTextFrameHighOp, ApplySpec, EditComponentHighOp, EditRecordHighOp, HighOp,
    QueryComponentsHighOp, QueryPinsHighOp, QueryRecordsHighOp, RefExpr, RemoveRecordsHighOp,
    apply_schlib, parse_apply_spec_json,
};
use proptest::prelude::*;

mod harness;
use harness::{list_len_field, save_reopen_schlib, schlib_fixture_path};

fn component_root_ref(base_opid: &str) -> RefExpr {
    RefExpr::op(format!("{base_opid}/create_component_root[0]")).member("ref")
}

fn build_ops(plans: Vec<Vec<(u8, i32, i32, i32)>>) -> Vec<HighOp> {
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

        for (code, a, b, c) in actions {
            let opid = format!("op_{op_counter:04}");
            op_counter += 1;
            match code % 10 {
                0 => out.push(HighOp::AddPin(AddPinOp {
                    opid: Some(opid),
                    id: None,
                    component_ref: Some(cref.clone()),
                    designator: format!("{}", (a.abs() % 200) + 1),
                    name: Some(format!("P{}", b.abs() % 200)),
                    electrical: Some("passive".to_owned()),
                    length_mils: Some((c.abs() % 300) + 10),
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
                    x1_mils: a.abs() % 200,
                    y1_mils: b.abs() % 200,
                    x2_mils: (a.abs() % 200) + 20,
                    y2_mils: (b.abs() % 200) + 20,
                    color: Some(0),
                    line_width: Some((c.abs() % 3) + 1),
                    line_style: Some(c.abs() % 4),
                    owner_part_id: Some(0),
                    owner_part_display_mode: Some(0),
                })),
                3 => out.push(HighOp::AddRectangle(AddRectangleHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    x1_mils: a.abs() % 200,
                    y1_mils: b.abs() % 200,
                    x2_mils: (a.abs() % 200) + 40,
                    y2_mils: (b.abs() % 200) + 30,
                    color: Some(0),
                    area_color: Some(0x00FF_FFFF),
                    is_solid: Some(true),
                    transparent: Some(false),
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
                    part_count: Some((a.abs() % 3) + 1),
                    display_mode_count: None,
                    component_kind: Some(0),
                    show_hidden_pins: Some((b & 1) == 1),
                })),
                6 => out.push(HighOp::EditRecord(EditRecordHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    selector: RecordSelector::ByName("Manufacturer".to_owned()),
                    patch: RecordPatch {
                        text: Some(format!("M{}", a.abs() % 1000)),
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
                _ => out.push(HighOp::RemoveRecords(RemoveRecordsHighOp {
                    opid: Some(opid),
                    component_ref: Some(cref.clone()),
                    selector: RecordSelector::ByRecordType(41),
                })),
            }
        }
    }

    out.push(HighOp::AddTextFrame(AddTextFrameHighOp {
        opid: Some("tail_textframe".to_owned()),
        component_ref: Some(component_root_ref("comp_0")),
        x1_mils: 0,
        y1_mils: 0,
        x2_mils: 60,
        y2_mils: 30,
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

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 24,
        .. ProptestConfig::default()
    })]
    #[test]
    fn schlib_generated_programs_are_stable_and_roundtrip(
        plans in prop::collection::vec(
            prop::collection::vec((0u8..=9, -300i32..=300, -300i32..=300, -10i32..=10), 0..=10),
            1..=3
        )
    ) {
        let ops = build_ops(plans);

        let wrapped = ApplySpec::Wrapped { ops: ops.clone() };
        let json = serde_json::to_string(&wrapped).expect("serialize generated JSON");
        let parsed = parse_apply_spec_json(&json).expect("parse generated JSON");

        let input = schlib_fixture_path("Resistors_Caps.SchLib");
        let mut lib_direct = SchLib::open(&input).expect("open fixture direct");
        let mut lib_parsed = SchLib::open(&input).expect("open fixture parsed");

        let report_direct = apply_schlib(&mut lib_direct, &ops).expect("apply direct generated ops");
        let report_parsed = apply_schlib(&mut lib_parsed, &parsed).expect("apply parsed generated ops");

        prop_assert_eq!(report_direct.high_op_count, report_parsed.high_op_count);
        prop_assert_eq!(report_direct.composed_op_count, report_parsed.composed_op_count);
        prop_assert_eq!(report_direct.low_op_count, report_parsed.low_op_count);
        prop_assert_eq!(report_direct.results.len(), report_parsed.results.len());
        prop_assert!(report_direct.results.len() >= report_direct.low_op_count);
        prop_assert_eq!(
            list_len_field(&report_direct, "tail_query_components", "components"),
            list_len_field(&report_parsed, "tail_query_components", "components")
        );

        save_reopen_schlib(&lib_direct);
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

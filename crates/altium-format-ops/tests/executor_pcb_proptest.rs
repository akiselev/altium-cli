use altium_format::{PcbDoc, PcbLib};
use altium_format_ops::{
    AddFootprintHighOp, AddTrackHighOp, ApplySpec, HighOp, QueryHighOp, RefExpr, apply_pcbdoc,
    apply_pcblib, parse_apply_spec_json,
};
use proptest::prelude::*;

mod harness;
use harness::{
    pcbdoc_fixture_path, pcblib_fixture_path, save_reopen_pcblib, validate_pcbdoc, validate_pcblib,
};

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

fn norm(v: i32, max: i32) -> i32 {
    v.abs() % max.max(1)
}

fn layer_name(v: i32) -> String {
    if (v & 1) == 0 {
        "TopLayer".to_owned()
    } else {
        "BottomLayer".to_owned()
    }
}

fn pick_supported_pcbdoc_fixture() -> std::path::PathBuf {
    let dir = pcbdoc_fixture_path("");
    let entries = std::fs::read_dir(&dir).expect("read pcbdoc fixture directory");
    let mut paths: Vec<std::path::PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("pcbdoc"))
                .unwrap_or(false)
        })
        .collect();
    paths.sort();
    for path in paths {
        if PcbDoc::open(&path).is_ok() {
            return path;
        }
    }
    panic!("no currently-supported pcbdoc fixture found in {}", dir.display());
}

fn count_field(report: &altium_format_ops::ApplyReport, opid: &str) -> i64 {
    let q = report.results.get(opid).expect("query result exists");
    let value = q.fields.get("count").expect("count field exists");
    match value {
        altium_format_ops::Value::I64(v) => *v,
        _ => panic!("count field must be integer"),
    }
}

fn build_pcbdoc_ops(plans: Vec<Vec<(u8, i32, i32, i32, i32)>>) -> Vec<HighOp> {
    let mut out = Vec::new();
    let mut op_counter = 0usize;

    for actions in plans {
        for (code, a, b, c, d) in actions {
            let opid = format!("op_{op_counter:04}");
            op_counter += 1;
            match code % 3 {
                0 | 1 => out.push(HighOp::AddTrack(AddTrackHighOp {
                    opid: Some(opid),
                    footprint_ref: None,
                    start: (edge_i32(a, a % 2000), edge_i32(b, b % 2000)),
                    end: (
                        edge_i32(a + 20 + norm(c, 200), (a + 20) % 2000),
                        edge_i32(b + 15 + norm(d, 200), (b + 15) % 2000),
                    ),
                    width_mils: Some(1 + norm(c, 80)),
                    layer: Some(layer_name(d)),
                })),
                _ => out.push(HighOp::Query(QueryHighOp {
                    opid: Some(opid),
                    selector: "track".to_owned(),
                })),
            }
        }
    }

    out.push(HighOp::Query(QueryHighOp {
        opid: Some("tail_query_tracks".to_owned()),
        selector: "track".to_owned(),
    }));
    out
}

fn run_pcbdoc_stability_program(plans: Vec<Vec<(u8, i32, i32, i32, i32)>>) {
    let ops = build_pcbdoc_ops(plans);
    let json = serde_json::to_string(&ApplySpec::Wrapped { ops: ops.clone() })
        .expect("serialize generated JSON");
    let parsed = parse_apply_spec_json(&json).expect("parse generated JSON");

    let input = pick_supported_pcbdoc_fixture();
    let mut doc_direct = PcbDoc::open(&input).expect("open fixture direct");
    let mut doc_parsed = PcbDoc::open(&input).expect("open fixture parsed");

    let report_direct = apply_pcbdoc(&mut doc_direct, &ops).expect("apply direct generated ops");
    let report_parsed = apply_pcbdoc(&mut doc_parsed, &parsed).expect("apply parsed generated ops");

    assert_eq!(report_direct.high_op_count, report_parsed.high_op_count);
    assert_eq!(report_direct.composed_op_count, report_parsed.composed_op_count);
    assert_eq!(report_direct.low_op_count, report_parsed.low_op_count);
    assert_eq!(report_direct.results.len(), report_parsed.results.len());
    assert_eq!(
        count_field(&report_direct, "tail_query_tracks"),
        count_field(&report_parsed, "tail_query_tracks")
    );

    validate_pcbdoc(&doc_direct);
    validate_pcbdoc(&doc_parsed);
}

fn build_pcblib_ops(plans: Vec<Vec<(u8, i32, i32, i32, i32)>>) -> Vec<HighOp> {
    let mut out = Vec::new();
    let mut op_counter = 0usize;

    for (group_idx, actions) in plans.into_iter().enumerate() {
        let fp_opid = format!("fp_{group_idx}");
        let fp_name = format!("PBT_FP_{group_idx}_{:04}", op_counter);
        out.push(HighOp::AddFootprint(AddFootprintHighOp {
            opid: Some(fp_opid.clone()),
            id: None,
            name: fp_name.clone(),
            pattern: Some(fp_name.clone()),
            description: Some(format!("generated_{group_idx}")),
        }));

        let fp_ref = RefExpr::op(fp_opid).member("ref");

        for (code, a, b, c, d) in actions {
            let opid = format!("op_{op_counter:04}");
            op_counter += 1;
            match code % 4 {
                0 | 1 => out.push(HighOp::AddTrack(AddTrackHighOp {
                    opid: Some(opid),
                    footprint_ref: Some(fp_ref.clone()),
                    start: (edge_i32(a, a % 2000), edge_i32(b, b % 2000)),
                    end: (
                        edge_i32(a + 25 + norm(c, 120), (a + 25) % 2000),
                        edge_i32(b + 25 + norm(d, 120), (b + 25) % 2000),
                    ),
                    width_mils: Some(1 + norm(c, 80)),
                    layer: Some(layer_name(d)),
                })),
                2 => out.push(HighOp::Query(QueryHighOp {
                    opid: Some(opid),
                    selector: format!("footprint[name={fp_name}]"),
                })),
                _ => out.push(HighOp::Query(QueryHighOp {
                    opid: Some(opid),
                    selector: format!("track[footprint={fp_name}]"),
                })),
            }
        }
    }

    out.push(HighOp::Query(QueryHighOp {
        opid: Some("tail_query_footprints".to_owned()),
        selector: "footprint".to_owned(),
    }));
    out.push(HighOp::Query(QueryHighOp {
        opid: Some("tail_query_tracks".to_owned()),
        selector: "track".to_owned(),
    }));
    out
}

fn run_pcblib_stability_program(plans: Vec<Vec<(u8, i32, i32, i32, i32)>>) {
    let ops = build_pcblib_ops(plans);
    let json = serde_json::to_string(&ApplySpec::Wrapped { ops: ops.clone() })
        .expect("serialize generated JSON");
    let parsed = parse_apply_spec_json(&json).expect("parse generated JSON");

    let input = pcblib_fixture_path("28Pins_Project.PcbLib");
    let mut lib_direct = PcbLib::open(&input).expect("open fixture direct");
    let mut lib_parsed = PcbLib::open(&input).expect("open fixture parsed");

    let report_direct = apply_pcblib(&mut lib_direct, &ops).expect("apply direct generated ops");
    let report_parsed = apply_pcblib(&mut lib_parsed, &parsed).expect("apply parsed generated ops");

    assert_eq!(report_direct.high_op_count, report_parsed.high_op_count);
    assert_eq!(report_direct.composed_op_count, report_parsed.composed_op_count);
    assert_eq!(report_direct.low_op_count, report_parsed.low_op_count);
    assert_eq!(report_direct.results.len(), report_parsed.results.len());
    assert_eq!(
        count_field(&report_direct, "tail_query_footprints"),
        count_field(&report_parsed, "tail_query_footprints")
    );
    assert_eq!(
        count_field(&report_direct, "tail_query_tracks"),
        count_field(&report_parsed, "tail_query_tracks")
    );

    validate_pcblib(&lib_direct);
    validate_pcblib(&lib_parsed);
    save_reopen_pcblib(&lib_direct);
    save_reopen_pcblib(&lib_parsed);
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 28,
        .. ProptestConfig::default()
    })]
    #[test]
    fn pcbdoc_generated_programs_are_stable_smoke(
        plans in prop::collection::vec(
            prop::collection::vec((0u8..=2, -6000i32..=6000, -6000i32..=6000, -2000i32..=2000, -720i32..=720), 0..=28),
            1..=4
        )
    ) {
        run_pcbdoc_stability_program(plans);
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 28,
        .. ProptestConfig::default()
    })]
    #[test]
    fn pcblib_generated_programs_are_stable_smoke(
        plans in prop::collection::vec(
            prop::collection::vec((0u8..=3, -6000i32..=6000, -6000i32..=6000, -2000i32..=2000, -720i32..=720), 0..=18),
            1..=4
        )
    ) {
        run_pcblib_stability_program(plans);
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 20,
        .. ProptestConfig::default()
    })]
    #[test]
    fn pcbdoc_query_ops_are_state_noops_metamorphic_smoke(
        plans in prop::collection::vec(
            prop::collection::vec((0u8..=2, -4000i32..=4000, -4000i32..=4000, -1500i32..=1500, -360i32..=360), 0..=16),
            1..=3
        )
    ) {
        let input = pick_supported_pcbdoc_fixture();
        let mut doc = PcbDoc::open(&input).expect("open fixture");
        let mut_ops = build_pcbdoc_ops(plans);
        apply_pcbdoc(&mut doc, &mut_ops).expect("apply mutating ops");

        let query_ops = [HighOp::Query(QueryHighOp {
            opid: Some("q_tracks".to_owned()),
            selector: "track".to_owned(),
        })];
        let report_a = apply_pcbdoc(&mut doc, &query_ops).expect("apply query ops A");
        let report_b = apply_pcbdoc(&mut doc, &query_ops).expect("apply query ops B");
        validate_pcbdoc(&doc);

        prop_assert_eq!(
            count_field(&report_a, "q_tracks"),
            count_field(&report_b, "q_tracks")
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 20,
        .. ProptestConfig::default()
    })]
    #[test]
    fn pcblib_query_ops_are_state_noops_metamorphic_smoke(
        plans in prop::collection::vec(
            prop::collection::vec((0u8..=3, -4000i32..=4000, -4000i32..=4000, -1500i32..=1500, -360i32..=360), 0..=12),
            1..=3
        )
    ) {
        let input = pcblib_fixture_path("28Pins_Project.PcbLib");
        let mut lib = PcbLib::open(&input).expect("open fixture");
        let mut_ops = build_pcblib_ops(plans);
        apply_pcblib(&mut lib, &mut_ops).expect("apply mutating ops");

        let query_ops = [
            HighOp::Query(QueryHighOp {
                opid: Some("q_footprints".to_owned()),
                selector: "footprint".to_owned(),
            }),
            HighOp::Query(QueryHighOp {
                opid: Some("q_tracks".to_owned()),
                selector: "track".to_owned(),
            }),
        ];
        let report_a = apply_pcblib(&mut lib, &query_ops).expect("apply query ops A");
        let report_b = apply_pcblib(&mut lib, &query_ops).expect("apply query ops B");
        validate_pcblib(&lib);

        prop_assert_eq!(
            count_field(&report_a, "q_footprints"),
            count_field(&report_b, "q_footprints")
        );
        prop_assert_eq!(
            count_field(&report_a, "q_tracks"),
            count_field(&report_b, "q_tracks")
        );
    }
}

#[test]
fn pcbdoc_model_based_manual_vs_json_equivalent() {
    let ops = vec![
        HighOp::AddTrack(AddTrackHighOp {
            opid: Some("t0".to_owned()),
            footprint_ref: None,
            start: (0, 0),
            end: (120, 60),
            width_mils: Some(8),
            layer: Some("TopLayer".to_owned()),
        }),
        HighOp::Query(QueryHighOp {
            opid: Some("q0".to_owned()),
            selector: "track".to_owned(),
        }),
    ];

    let json = serde_json::to_string(&ApplySpec::Wrapped { ops: ops.clone() }).expect("serialize");
    let parsed = parse_apply_spec_json(&json).expect("parse");

    let input = pick_supported_pcbdoc_fixture();
    let mut doc_manual = PcbDoc::open(&input).expect("open fixture manual");
    let mut doc_json = PcbDoc::open(&input).expect("open fixture json");

    let report_manual = apply_pcbdoc(&mut doc_manual, &ops).expect("apply manual");
    let report_json = apply_pcbdoc(&mut doc_json, &parsed).expect("apply json");

    assert_eq!(report_manual.low_op_count, report_json.low_op_count);
    assert_eq!(
        count_field(&report_manual, "q0"),
        count_field(&report_json, "q0")
    );
}

#[test]
fn pcblib_model_based_manual_vs_json_equivalent() {
    let ops = vec![
        HighOp::AddFootprint(AddFootprintHighOp {
            opid: Some("f0".to_owned()),
            id: None,
            name: "MODEL_EQ_FP0".to_owned(),
            pattern: Some("MODEL_EQ_FP0".to_owned()),
            description: Some("model based".to_owned()),
        }),
        HighOp::AddTrack(AddTrackHighOp {
            opid: Some("t0".to_owned()),
            footprint_ref: Some(RefExpr::op("f0").member("ref")),
            start: (0, 0),
            end: (100, 100),
            width_mils: Some(6),
            layer: Some("TopLayer".to_owned()),
        }),
        HighOp::Query(QueryHighOp {
            opid: Some("q0".to_owned()),
            selector: "track[footprint=MODEL_EQ_FP0]".to_owned(),
        }),
    ];

    let json = serde_json::to_string(&ApplySpec::Wrapped { ops: ops.clone() }).expect("serialize");
    let parsed = parse_apply_spec_json(&json).expect("parse");

    let input = pcblib_fixture_path("28Pins_Project.PcbLib");
    let mut lib_manual = PcbLib::open(&input).expect("open fixture manual");
    let mut lib_json = PcbLib::open(&input).expect("open fixture json");

    let report_manual = apply_pcblib(&mut lib_manual, &ops).expect("apply manual");
    let report_json = apply_pcblib(&mut lib_json, &parsed).expect("apply json");

    assert_eq!(report_manual.low_op_count, report_json.low_op_count);
    assert_eq!(
        count_field(&report_manual, "q0"),
        count_field(&report_json, "q0")
    );

    save_reopen_pcblib(&lib_manual);
    save_reopen_pcblib(&lib_json);
}

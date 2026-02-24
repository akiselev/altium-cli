use altium_format::SchLib;
use altium_format_ops::apply_ops_source_schlib;

mod harness;
use harness::schlib_fixture_path;

#[test]
fn schlib_ops_source_broad_flow_passes() {
    let input = schlib_fixture_path("Resistors_Caps.SchLib");
    let mut lib = SchLib::open(&input).expect("open schlib fixture");

    let src = r#"
r1 = add_component { lib_reference: "OPS_E2E_U1", designator: "U?" }
add_pin $r1 { designator: "1", electrical: passive, length_mils: 25mil }
add_parameter $r1 { name: "MFG", text: "ACME" }
add_line $r1 { x1_mils: 0, y1_mils: 0, x2_mils: 10, y2_mils: 10 }
add_rectangle $r1 { x1_mils: 0, y1_mils: 0, x2_mils: 40, y2_mils: 20 }
add_arc $r1 { cx_mils: 0, cy_mils: 0, radius_mils: 10 }
add_text_frame $r1 { x1_mils: 0, y1_mils: 0, x2_mils: 100, y2_mils: 50, text: "hello" }
query component[lib_reference=OPS_E2E_U1]
"#;

    let report = apply_ops_source_schlib(&mut lib, src).expect("apply ops source schlib");
    assert!(report.results.contains_key("r1/create_component_root[0]"));
    assert!(report.results.contains_key("r1"));
    let q = report.results.get("op_0007").expect("query result");
    assert_eq!(q.kind, "query");
    assert!(q.ref_.is_some());
}

#[test]
fn schlib_ops_source_runtime_rejects_selector_outside_runtime_subset() {
    let input = schlib_fixture_path("Resistors_Caps.SchLib");
    let mut lib = SchLib::open(&input).expect("open schlib fixture");

    let src = r#"query component[value=10K]"#;
    let err = apply_ops_source_schlib(&mut lib, src).expect_err("runtime selector subset");
    let _ = err;
}

#[test]
fn schlib_ops_source_ref_equivalence_dollar_name_and_dot_ref() {
    let input = schlib_fixture_path("Resistors_Caps.SchLib");
    let mut lib_a = SchLib::open(&input).expect("open schlib fixture A");
    let mut lib_b = SchLib::open(&input).expect("open schlib fixture B");

    let src_a = r#"
r1 = add_component { lib_reference: "OPS_EQV_U1", designator: "U?" }
add_pin $r1 { designator: "1", electrical: passive }
query component[lib_reference=OPS_EQV_U1]
"#;
    let src_b = r#"
r1 = add_component { lib_reference: "OPS_EQV_U2", designator: "U?" }
add_pin $r1.ref { designator: "1", electrical: passive }
query component[lib_reference=OPS_EQV_U2]
"#;

    let report_a = apply_ops_source_schlib(&mut lib_a, src_a).expect("apply A");
    let report_b = apply_ops_source_schlib(&mut lib_b, src_b).expect("apply B");

    assert_eq!(
        report_a.results.get("op_0002").expect("query A").kind,
        report_b.results.get("op_0002").expect("query B").kind
    );
}

use altium_format::SchDoc;
use altium_format_ops::apply_ops_source_schdoc;

mod harness;
use harness::schdoc_fixture_path;

#[test]
fn schdoc_ops_source_pass_now_subset() {
    let input = schdoc_fixture_path(
        "myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc",
    );
    let mut doc = SchDoc::open(&input).expect("open schdoc fixture");

    let src = r#"
r1 = add_component {
  lib_reference: "OPS_E2E_R"
  designator: "R901"
  value: "10K"
}
add_pin $r1 { designator: "1", electrical: passive }
add_parameter $r1 { name: "MFG", text: "ACME" }
query component[designator=R901]
"#;

    let report = apply_ops_source_schdoc(&mut doc, src).expect("apply ops source");
    assert!(report.results.contains_key("r1/create_component_root[0]"));
    assert!(report.results.contains_key("r1"));
    let q = report.results.get("op_0003").expect("query result");
    assert_eq!(q.kind, "query");
}

#[test]
fn schdoc_ops_source_compile_rejects_schlib_alias_ops() {
    let input = schdoc_fixture_path(
        "myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc",
    );
    let mut doc = SchDoc::open(&input).expect("open schdoc fixture");

    let src = r#"add_alias $last { alias_name: "R_ALIAS" }"#;
    let err = apply_ops_source_schdoc(&mut doc, src).expect_err("should fail at schdoc compile");
    let msg = err.to_string();
    assert!(msg.contains("ops parse/typecheck failed"));
    assert!(msg.contains("add_alias is a SchLib-only operation"));
}

#[test]
fn schdoc_ops_source_runtime_rejects_selector_outside_runtime_subset() {
    let input = schdoc_fixture_path(
        "myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc",
    );
    let mut doc = SchDoc::open(&input).expect("open schdoc fixture");

    let src = r#"query component[value=10K]"#;
    let err = apply_ops_source_schdoc(&mut doc, src).expect_err("runtime selector subset");
    let _ = err;
}

#[test]
fn schdoc_ops_source_compile_rejects_generic_edit_selector() {
    let input = schdoc_fixture_path(
        "myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc",
    );
    let mut doc = SchDoc::open(&input).expect("open schdoc fixture");

    let src = r#"edit component[designator=R1] { description: "x" }"#;
    let err = apply_ops_source_schdoc(&mut doc, src).expect_err("compile should fail");
    let msg = err.to_string();
    assert!(msg.contains("ops parse/typecheck failed"));
    assert!(msg.contains("E2008"));
}

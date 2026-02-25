#![cfg(feature = "test-fixtures")]
use altium_format::{SchDoc, SchLib};
use altium_format_ops::{apply_schdoc, apply_schlib, parse_apply_spec_yaml};

mod harness;
use harness::{save_reopen_schdoc, save_reopen_schlib, schdoc_fixture_path, schlib_fixture_path};

#[test]
fn schdoc_executor_feedback_loop_results_and_reopen() {
    let input = schdoc_fixture_path(
        "myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc",
    );
    let mut doc = SchDoc::open(&input).expect("open schdoc fixture");

    let spec = r#"
ops:
  - opid: create_comp
    op: add_component
    id: RNEW
    lib_reference: OPS_TEST_R
    designator: R777
    value: 22K
    pins:
      - designator: "1"
        electrical: passive
      - designator: "2"
        electrical: passive
    footprint:
      model_name: "0603"
      map:
        - pin: "1"
          pad: "1"
        - pin: "2"
          pad: "2"
  - opid: q1
    op: query
    selector: component[designator=R777]
"#;

    let ops = parse_apply_spec_yaml(spec).expect("parse yaml spec");
    let report = apply_schdoc(&mut doc, &ops).expect("apply schdoc ops");

    assert_eq!(report.high_op_count, 2);
    assert_eq!(report.composed_op_count, 12);
    assert_eq!(report.low_op_count, 12);
    assert!(
        report
            .results
            .contains_key("create_comp/create_component_root[0]")
    );

    let q = report.results.get("q1").expect("q1 result exists");
    assert_eq!(q.kind, "query");
    assert!(q.ref_.is_some(), "query should return a primary ref");

    doc.validate_invariants().expect("invariants after apply");

    save_reopen_schdoc(&doc);
}

#[test]
fn schdoc_typed_ref_from_prior_op_result() {
    let input = schdoc_fixture_path(
        "myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc",
    );
    let mut doc = SchDoc::open(&input).expect("open schdoc fixture");

    let spec = r#"
ops:
  - opid: create_comp
    op: add_component
    lib_reference: OPS_REF_R
    designator: R200
  - opid: add_pin_again
    op: add_pin
    component_ref:
      root:
        OpId: create_comp/create_component_root[0]
      steps:
        - Member: ref
    designator: "9"
    electrical: passive
  - opid: q2
    op: query
    selector: component[designator=R200]
"#;

    let ops = parse_apply_spec_yaml(spec).expect("parse yaml spec");
    let report = apply_schdoc(&mut doc, &ops).expect("apply schdoc ops");

    let pin_res = report
        .results
        .get("add_pin_again")
        .expect("add_pin_again result exists");
    assert_eq!(pin_res.kind, "add_pin");
    let pin_ref = pin_res.ref_.as_ref().expect("pin op has primary ref");
    assert!(pin_ref.display_path.contains("pin[9]"));

    let q = report.results.get("q2").expect("q2 result exists");
    assert!(q.ref_.is_some(), "query should resolve created component");

    doc.validate_invariants()
        .expect("invariants after typed ref flow");
}

#[test]
fn schlib_executor_query_and_save_roundtrip() {
    let input = schlib_fixture_path("Resistors_Caps.SchLib");
    let mut lib = SchLib::open(&input).expect("open schlib fixture");

    let spec = r#"
ops:
  - opid: create_comp
    op: add_component
    id: CNEW
    lib_reference: OPS_LIB_U1
    designator: U?
    value: TEST
    pins:
      - designator: "1"
        electrical: passive
      - designator: "2"
        electrical: passive
    footprint:
      model_name: "DIP-2"
      map:
        - pin: "1"
          pad: "1"
        - pin: "2"
          pad: "2"
  - opid: q1
    op: query
    selector: component[lib_reference=OPS_LIB_U1]
"#;

    let ops = parse_apply_spec_yaml(spec).expect("parse yaml spec");
    let report = apply_schlib(&mut lib, &ops).expect("apply schlib ops");

    assert_eq!(report.high_op_count, 2);
    assert_eq!(report.composed_op_count, 12);
    assert_eq!(report.low_op_count, 12);

    let q = report.results.get("q1").expect("q1 result exists");
    assert!(q.ref_.is_some(), "query should return created component");
    assert_eq!(
        q.ref_.as_ref().expect("primary ref").display_path,
        "OPS_LIB_U1"
    );

    save_reopen_schlib(&lib);
}

#[test]
fn schdoc_ref_root_last_resolves_previous_component() {
    let input = schdoc_fixture_path(
        "myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc",
    );
    let mut doc = SchDoc::open(&input).expect("open schdoc fixture");

    let spec = r#"
ops:
  - opid: create_comp
    op: add_component
    lib_reference: OPS_LAST_REF_R
  - opid: add_pin_last
    op: add_pin
    component_ref:
      root: Last
      steps:
        - Member: ref
    designator: "5"
    electrical: passive
"#;

    let ops = parse_apply_spec_yaml(spec).expect("parse yaml spec");
    let report = apply_schdoc(&mut doc, &ops).expect("apply schdoc ops");

    let pin = report
        .results
        .get("add_pin_last")
        .expect("add_pin_last result exists");
    assert_eq!(pin.kind, "add_pin");
    assert!(
        pin.ref_
            .as_ref()
            .expect("pin ref exists")
            .display_path
            .contains("pin[5]")
    );
}

#[test]
fn schlib_query_cardinality_sets_primary_ref_only_for_single_match() {
    let input = schlib_fixture_path("Resistors_Caps.SchLib");
    let mut lib = SchLib::open(&input).expect("open schlib fixture");

    let zero_spec = r#"
ops:
  - opid: q_zero
    op: query
    selector: component[lib_reference=OPS_DOES_NOT_EXIST]
"#;
    let ops = parse_apply_spec_yaml(zero_spec).expect("parse zero yaml spec");
    let zero_report = apply_schlib(&mut lib, &ops).expect("apply zero query");
    let q_zero = zero_report
        .results
        .get("q_zero")
        .expect("q_zero result exists");
    assert!(q_zero.refs.is_empty());
    assert!(q_zero.ref_.is_none());

    let many_spec = r#"
ops:
  - opid: q_many
    op: query
    selector: component
"#;
    let ops = parse_apply_spec_yaml(many_spec).expect("parse many yaml spec");
    let many_report = apply_schlib(&mut lib, &ops).expect("apply many query");
    let q_many = many_report
        .results
        .get("q_many")
        .expect("q_many result exists");
    assert!(
        q_many.refs.len() > 1,
        "fixture should contain multiple components"
    );
    assert!(
        q_many.ref_.is_none(),
        "primary ref must be None for multi-match"
    );
}

#[test]
fn schdoc_invalid_query_selector_fails_fast() {
    let input = schdoc_fixture_path(
        "myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc",
    );
    let mut doc = SchDoc::open(&input).expect("open schdoc fixture");

    let spec = r#"
ops:
  - opid: bad_selector
    op: query
    selector: pin[designator=1]
"#;
    let ops = parse_apply_spec_yaml(spec).expect("parse yaml spec");
    let err = apply_schdoc(&mut doc, &ops).expect_err("invalid selector should fail");
    assert!(
        err.to_string().contains("unsupported query selector"),
        "unexpected error: {err}"
    );
}

#[test]
fn schdoc_add_pin_without_ref_or_prior_component_fails() {
    let input = schdoc_fixture_path(
        "myriadrf_LimeSDR-XTRX__hardware_1v0_Schematics__03_Clock_Diagram.SchDoc",
    );
    let mut doc = SchDoc::open(&input).expect("open schdoc fixture");

    let spec = r#"
ops:
  - opid: lone_pin
    op: add_pin
    designator: "1"
    electrical: passive
"#;
    let ops = parse_apply_spec_yaml(spec).expect("parse yaml spec");
    let err = apply_schdoc(&mut doc, &ops).expect_err("pin without component_ref should fail");
    assert!(
        err.to_string().contains("component_ref is required"),
        "unexpected error: {err}"
    );
}

#[test]
fn schlib_alias_edit_component_and_query_components() {
    let input = schlib_fixture_path("Resistors_Caps.SchLib");
    let mut lib = SchLib::open(&input).expect("open schlib fixture");

    let spec = r#"
ops:
  - opid: create_comp
    op: add_component
    lib_reference: OPS_MUT_U
  - opid: add_alias
    op: add_alias
    component_ref:
      root:
        OpId: create_comp/create_component_root[0]
      steps:
        - Member: ref
    alias_name: OPS_MUT_ALIAS
  - opid: edit_comp
    op: edit_component
    component_ref:
      root:
        OpId: create_comp/create_component_root[0]
      steps:
        - Member: ref
    description: OPS_DESC
    part_count: 2
  - opid: q_comp
    op: query_components
    pattern: OPS_MUT_U
"#;

    let ops = parse_apply_spec_yaml(spec).expect("parse yaml spec");
    let report = apply_schlib(&mut lib, &ops).expect("apply schlib ops");

    let q = report.results.get("q_comp").expect("q_comp result");
    assert_eq!(q.kind, "query_components");
    let components = q.fields.get("components").expect("components field");
    match components {
        altium_format_ops::Value::List(rows) => assert_eq!(rows.len(), 1),
        _ => panic!("unexpected components value"),
    }
}

#[test]
fn schlib_edit_and_remove_records_feedback_loop() {
    let input = schlib_fixture_path("Resistors_Caps.SchLib");
    let mut lib = SchLib::open(&input).expect("open schlib fixture");

    let spec = r#"
ops:
  - opid: create_comp
    op: add_component
    lib_reference: OPS_REC_U
    designator: U?
    pins:
      - designator: "1"
        electrical: passive
  - opid: add_param
    op: add_parameter
    name: Manufacturer
    text: ACME
  - opid: edit_param
    op: edit_record
    selector:
      ByName: Manufacturer
    patch:
      text: ACME2
  - opid: q_params_before_remove
    op: query_records
    component_ref:
      root:
        OpId: create_comp/create_component_root[0]
      steps:
        - Member: ref
    record_type: 41
  - opid: rm_params
    op: remove_records
    selector:
      ByRecordType: 41
  - opid: q_params_after_remove
    op: query_records
    component_ref:
      root:
        OpId: create_comp/create_component_root[0]
      steps:
        - Member: ref
    record_type: 41
"#;

    let ops = parse_apply_spec_yaml(spec).expect("parse yaml spec");
    let report = apply_schlib(&mut lib, &ops).expect("apply schlib ops");

    let before = report
        .results
        .get("q_params_before_remove")
        .expect("before result");
    let after = report
        .results
        .get("q_params_after_remove")
        .expect("after result");

    let before_len = match before.fields.get("records").expect("records before") {
        altium_format_ops::Value::List(v) => v.len(),
        _ => 0,
    };
    let after_len = match after.fields.get("records").expect("records after") {
        altium_format_ops::Value::List(v) => v.len(),
        _ => usize::MAX,
    };
    assert!(
        before_len > after_len,
        "remove_records should reduce record count"
    );
}

#[test]
fn schlib_graphics_and_query_pins_and_invalid_bezier() {
    let input = schlib_fixture_path("Resistors_Caps.SchLib");
    let mut lib = SchLib::open(&input).expect("open schlib fixture");

    let ok_spec = r#"
ops:
  - opid: create_comp
    op: add_component
    lib_reference: OPS_GRAPH_U
    pins:
      - designator: "1"
        name: P1
        electrical: passive
  - opid: add_line
    op: add_line
    from: [0, 0]
    to: [100, 100]
  - opid: add_rect
    op: add_rectangle
    from: [0, 0]
    to: [80, 40]
  - opid: q_pins
    op: query_pins
    component_ref:
      root:
        OpId: create_comp/create_component_root[0]
      steps:
        - Member: ref
"#;
    let ops = parse_apply_spec_yaml(ok_spec).expect("parse ok yaml spec");
    let report = apply_schlib(&mut lib, &ops).expect("apply ok schlib ops");
    let pins = report.results.get("q_pins").expect("pins result");
    match pins.fields.get("pins").expect("pins field") {
        altium_format_ops::Value::List(v) => assert!(!v.is_empty()),
        _ => panic!("unexpected pins field"),
    }

    let bad_spec = r#"
ops:
  - opid: create_comp
    op: add_component
    lib_reference: OPS_BAD_BEZ
  - opid: bad_bezier
    op: add_bezier
    points_mils:
      - [0, 0]
      - [10, 10]
      - [20, 20]
"#;
    let ops = parse_apply_spec_yaml(bad_spec).expect("parse bad yaml spec");
    let err = apply_schlib(&mut lib, &ops).expect_err("bad bezier should fail");
    assert!(
        err.to_string().contains("bezier requires exactly 4 points"),
        "unexpected error: {err}"
    );
}

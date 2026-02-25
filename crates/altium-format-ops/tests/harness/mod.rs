use std::path::PathBuf;

use altium_format::{PcbDoc, PcbLib, SchDoc, SchLib};
use altium_format_ops::ApplyReport;

#[allow(dead_code)]
pub fn schdoc_fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/schdoc")
        .join(name)
}

#[allow(dead_code)]
pub fn schlib_fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/schlib")
        .join(name)
}

#[allow(dead_code)]
pub fn pcbdoc_fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/pcbdoc")
        .join(name)
}

#[allow(dead_code)]
pub fn pcblib_fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/pcblib")
        .join(name)
}

#[allow(dead_code)]
pub fn save_reopen_schdoc(doc: &SchDoc) {
    let tmp = tempfile::NamedTempFile::new().expect("create temp file");
    doc.save(tmp.path()).expect("save schdoc output");
    let reopened = SchDoc::open(tmp.path()).expect("reopen saved schdoc");
    reopened
        .validate_invariants()
        .expect("reopened schdoc validates");
}

#[allow(dead_code)]
pub fn save_bytes_schdoc(doc: &SchDoc) -> Vec<u8> {
    let tmp = tempfile::NamedTempFile::new().expect("create temp file");
    doc.save(tmp.path()).expect("save schdoc output");
    std::fs::read(tmp.path()).expect("read saved schdoc")
}

#[allow(dead_code)]
pub fn save_reopen_schlib(lib: &SchLib) {
    use altium_format::test_utils::assert_cfb_files_semantic_eq;

    lib.validate_invariants()
        .expect("schlib validates before save");
    let tmp1 = tempfile::NamedTempFile::new().expect("create temp file");
    lib.save(tmp1.path()).expect("save schlib output");
    let reopened = SchLib::open(tmp1.path()).expect("reopen saved schlib");
    reopened
        .validate_invariants()
        .expect("reopened schlib validates");
    let tmp2 = tempfile::NamedTempFile::new().expect("create second temp file");
    reopened
        .save(tmp2.path())
        .expect("save reopened schlib output");
    assert_cfb_files_semantic_eq(tmp1.path(), tmp2.path());
}

#[allow(dead_code)]
pub fn save_bytes_schlib(lib: &SchLib) -> Vec<u8> {
    let tmp = tempfile::NamedTempFile::new().expect("create temp file");
    lib.save(tmp.path()).expect("save schlib output");
    std::fs::read(tmp.path()).expect("read saved schlib")
}

#[allow(dead_code)]
pub fn validate_pcbdoc(doc: &PcbDoc) {
    doc.validate_invariants().expect("pcbdoc validates");
}

#[allow(dead_code)]
pub fn validate_pcblib(lib: &PcbLib) {
    lib.validate_invariants().expect("pcblib validates");
}

#[allow(dead_code)]
pub fn save_reopen_pcblib(lib: &PcbLib) {
    use altium_format::test_utils::assert_cfb_files_semantic_eq;

    lib.validate_invariants()
        .expect("pcblib validates before save");
    let tmp1 = tempfile::NamedTempFile::new().expect("create temp file");
    lib.save(tmp1.path()).expect("save pcblib output");
    let reopened = PcbLib::open(tmp1.path()).expect("reopen saved pcblib");
    reopened
        .validate_invariants()
        .expect("reopened pcblib validates");
    let tmp2 = tempfile::NamedTempFile::new().expect("create second temp file");
    reopened
        .save(tmp2.path())
        .expect("save reopened pcblib output");
    assert_cfb_files_semantic_eq(tmp1.path(), tmp2.path());
}

#[allow(dead_code)]
pub fn list_len_field(report: &ApplyReport, opid: &str, field: &str) -> usize {
    let q = report.results.get(opid).expect("query result exists");
    let value = q.fields.get(field).expect("list field exists");
    match value {
        altium_format_ops::Value::List(v) => v.len(),
        _ => 0,
    }
}

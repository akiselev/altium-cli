use std::path::PathBuf;

use altium_format::{SchDoc, SchLib};
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
pub fn save_reopen_schdoc(doc: &SchDoc) {
    let tmp = tempfile::NamedTempFile::new().expect("create temp file");
    doc.save(tmp.path()).expect("save schdoc output");
    let reopened = SchDoc::open(tmp.path()).expect("reopen saved schdoc");
    reopened
        .validate_invariants()
        .expect("reopened schdoc validates");
}

#[allow(dead_code)]
pub fn save_reopen_schlib(lib: &SchLib) {
    let tmp = tempfile::NamedTempFile::new().expect("create temp file");
    lib.save(tmp.path()).expect("save schlib output");
    SchLib::open(tmp.path()).expect("reopen saved schlib");
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

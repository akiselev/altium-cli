// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Tests for CFB stream comparison utilities.

mod common;

use common::cfb_compare::*;
use std::io::{Cursor, Write};

/// Create a minimal CFB with given streams.
fn make_cfb(streams: &[(&str, &[u8])]) -> Vec<u8> {
    let buf = Cursor::new(Vec::new());
    let mut cfb = cfb::CompoundFile::create(buf).unwrap();
    for (path, data) in streams {
        let p = std::path::Path::new(path);
        if let Some(parent) = p.parent() {
            if parent != std::path::Path::new("/") {
                let _ = cfb.create_storage(parent);
            }
        }
        let mut stream = cfb.create_stream(path).unwrap();
        stream.write_all(data).unwrap();
    }
    cfb.flush().unwrap();
    cfb.into_inner().into_inner()
}

#[test]
fn identical_cfbs_report_match() {
    let data = make_cfb(&[("/FileHeader", b"|RECORD=1|NAME=Test|")]);
    let report = compare_cfb_files(&data, &data);
    assert!(report.is_match());
    assert_eq!(report.matched.len(), 1);
}

#[test]
fn text_param_reordering_is_equal() {
    let orig = make_cfb(&[("/FileHeader", b"|A=1|B=2|C=3|")]);
    let rebuilt = make_cfb(&[("/FileHeader", b"|C=3|A=1|B=2|")]);
    let report = compare_cfb_files(&orig, &rebuilt);
    assert!(
        report.is_match(),
        "Reordered params should match: {}",
        report
    );
}

#[test]
fn text_param_value_diff_detected() {
    let orig = make_cfb(&[("/FileHeader", b"|A=1|B=2|")]);
    let rebuilt = make_cfb(&[("/FileHeader", b"|A=1|B=99|")]);
    let report = compare_cfb_files(&orig, &rebuilt);
    assert!(!report.is_match());
    assert_eq!(report.text_diffs.len(), 1);
    assert_eq!(report.text_diffs[0].param_diffs.len(), 1);
    assert_eq!(report.text_diffs[0].param_diffs[0].0, "B");
}

#[test]
fn missing_stream_detected() {
    let orig = make_cfb(&[("/FileHeader", b"|A=1|"), ("/Extra", b"|X=Y|")]);
    let rebuilt = make_cfb(&[("/FileHeader", b"|A=1|")]);
    let report = compare_cfb_files(&orig, &rebuilt);
    assert!(!report.is_match());
    assert_eq!(report.only_in_original.len(), 1);
}

#[test]
fn binary_diff_detected() {
    let orig = make_cfb(&[("/Data", &[0x01, 0x02, 0x03, 0x04])]);
    let rebuilt = make_cfb(&[("/Data", &[0x01, 0x02, 0xFF, 0x04])]);
    let report = compare_cfb_files(&orig, &rebuilt);
    assert!(!report.is_match());
    assert_eq!(report.binary_diffs.len(), 1);
    assert_eq!(report.binary_diffs[0].first_diff_offset, Some(2));
}

#[test]
fn is_text_heuristic_works() {
    assert!(is_text_stream(b"|RECORD=1|NAME=Test|"));
    assert!(is_text_stream(b"Hello World"));
    assert!(!is_text_stream(&[0x00, 0x01, 0x02, 0x80, 0xFF]));
    assert!(is_text_stream(b"")); // empty is text
}

#[test]
fn diff_count_sums_all_differences() {
    let orig = make_cfb(&[
        ("/A", b"|X=1|"),
        ("/B", &[0x00, 0x01]),
        ("/C", b"|Y=2|"),
    ]);
    let rebuilt = make_cfb(&[("/A", b"|X=99|"), ("/B", &[0x00, 0xFF])]);
    let report = compare_cfb_files(&orig, &rebuilt);
    assert_eq!(report.diff_count(), 3); // 1 text diff + 1 binary diff + 1 only_in_original
}

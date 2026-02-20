// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! High-level from-scratch rebuild command support.
//!
//! Rebuilds supported Altium documents into a temp file using typed record
//! getters/setters and templates, then diffs original vs rebuilt CFB streams.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use altium_format::v2::documents::{PcbLib, SchDoc, SchLib};
use altium_format::v2::handles::{
    PcbArcHandle, PcbComponentBodyHandle, PcbFillHandle, PcbPadHandle, PcbRegionHandle,
    PcbTextHandle, PcbTrackHandle, PcbViaHandle, SchArcHandle, SchBezierHandle, SchBlanketHandle,
    SchBusEntryHandle, SchBusHandle, SchComponent, SchDesignatorHandle, SchEllipseHandle,
    SchEllipticalArcHandle, SchImageHandle, SchImplementationHandle, SchImplementationListHandle,
    SchImplementationParametersHandle, SchJunctionHandle, SchLabelHandle, SchLineHandle,
    SchMapDefinerHandle, SchMapDefinerListHandle, SchNetLabelHandle, SchNoERCHandle, SchNoteHandle,
    SchParameterHandle, SchPieHandle, SchPinHandle, SchPolygonHandle, SchPolylineHandle,
    SchPortHandle, SchPowerHandle, SchRectangleHandle, SchRoundRectangleHandle,
    SchSheetEntryHandle, SchSheetFileNameHandle, SchSheetHandle, SchSheetNameHandle,
    SchSheetSymbolHandle, SchSymbolHandle, SchTextFrameHandle, SchWireHandle,
};
use altium_format::v2::records::{
    PcbArcRecord, PcbComponentBodyRecord, PcbFillRecord, PcbPadRecord, PcbRegionRecord,
    PcbTextRecord, PcbTrackRecord, PcbViaRecord, SchArcRecord, SchBezierRecord, SchBlanketRecord,
    SchBusEntryRecord, SchBusRecord, SchDesignatorRecord, SchEllipseRecord, SchEllipticalArcRecord,
    SchImageRecord, SchImplementationListRecord, SchImplementationParametersRecord,
    SchImplementationRecord, SchJunctionRecord, SchLabelRecord, SchLineRecord,
    SchMapDefinerListRecord, SchMapDefinerRecord, SchNetLabelRecord, SchNoERCRecord, SchNoteRecord,
    SchParameterRecord, SchPieRecord, SchPinRecord, SchPolygonRecord, SchPolylineRecord,
    SchPortRecord, SchPowerRecord, SchRectangleRecord, SchRoundRectangleRecord,
    SchSheetEntryRecord, SchSheetFileNameRecord, SchSheetNameRecord, SchSheetRecord,
    SchSheetSymbolRecord, SchSymbolRecord, SchTextFrameRecord, SchWireRecord,
};
use altium_format::v2::templates;
use altium_format::v2::traits::{DocumentQuery, RecordType};

use crate::cfb_diff::{CfbDiffReport, compare_cfb_files};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkippedRecordSummary {
    pub context: String,
    pub record_id: u8,
    pub reason: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuildReport {
    pub file_type: String,
    pub source_path: String,
    pub rebuilt_path: String,
    pub skipped_records: Vec<SkippedRecordSummary>,
    pub diff: CfbDiffReport,
}

struct PanicHookSilencer {
    previous: Option<Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Send + Sync + 'static>>,
}

impl PanicHookSilencer {
    fn install() -> Self {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        Self {
            previous: Some(previous),
        }
    }
}

impl Drop for PanicHookSilencer {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            std::panic::set_hook(previous);
        }
    }
}

fn classify_extension(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match ext.as_str() {
        "schlib" => Some("SchLib"),
        "pcblib" => Some("PcbLib"),
        "schdoc" => Some("SchDoc"),
        "pcbdoc" => Some("PcbDoc"),
        _ => None,
    }
}

fn make_temp_rebuild_path(src: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "altium".to_string());
    let ext = src
        .extension()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "bin".to_string());
    let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let pid = std::process::id();
    Ok(std::env::temp_dir().join(format!("{}-rebuild-{}-{}.{}", stem, pid, ts, ext)))
}

fn add_skip(
    skips: &mut BTreeMap<(String, u8, String), usize>,
    context: &str,
    record_id: u8,
    reason: &str,
) {
    let key = (context.to_string(), record_id, reason.to_string());
    *skips.entry(key).or_insert(0) += 1;
}

fn finalize_skips(skips: BTreeMap<(String, u8, String), usize>) -> Vec<SkippedRecordSummary> {
    skips
        .into_iter()
        .map(
            |((context, record_id, reason), count)| SkippedRecordSummary {
                context,
                record_id,
                reason,
                count,
            },
        )
        .collect()
}

macro_rules! copy_sch_record {
    ($type_id:expr, $rid:expr, $src_store:expr, $emit:ident, $skips:expr, $context:expr) => {{
        match $type_id {
            <SchPinRecord as RecordType>::RECORD_ID => {
                let src = SchPinHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchPinRecord::from_origin(templates::sch_pin_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchArcRecord as RecordType>::RECORD_ID => {
                let src = SchArcHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchArcRecord::from_origin(templates::sch_arc_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchLineRecord as RecordType>::RECORD_ID => {
                let src = SchLineHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchLineRecord::from_origin(templates::sch_line_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchRectangleRecord as RecordType>::RECORD_ID => {
                let src = SchRectangleHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchRectangleRecord::from_origin(templates::sch_rectangle_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchBezierRecord as RecordType>::RECORD_ID => {
                let src = SchBezierHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchBezierRecord::from_origin(templates::sch_bezier_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchPolylineRecord as RecordType>::RECORD_ID => {
                let src = SchPolylineHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchPolylineRecord::from_origin(templates::sch_polyline_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchPolygonRecord as RecordType>::RECORD_ID => {
                let src = SchPolygonHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchPolygonRecord::from_origin(templates::sch_polygon_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchEllipseRecord as RecordType>::RECORD_ID => {
                let src = SchEllipseHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchEllipseRecord::from_origin(templates::sch_ellipse_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchPieRecord as RecordType>::RECORD_ID => {
                let src = SchPieHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchPieRecord::from_origin(templates::sch_pie_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchRoundRectangleRecord as RecordType>::RECORD_ID => {
                let src = SchRoundRectangleHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchRoundRectangleRecord::from_origin(templates::sch_round_rectangle_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchEllipticalArcRecord as RecordType>::RECORD_ID => {
                let src = SchEllipticalArcHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchEllipticalArcRecord::from_origin(templates::sch_elliptical_arc_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchImageRecord as RecordType>::RECORD_ID => {
                let src = SchImageHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchImageRecord::from_origin(templates::sch_image_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchDesignatorRecord as RecordType>::RECORD_ID => {
                let src = SchDesignatorHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchDesignatorRecord::from_origin(templates::sch_designator_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchParameterRecord as RecordType>::RECORD_ID => {
                let src = SchParameterHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchParameterRecord::from_origin(templates::sch_parameter_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchSymbolRecord as RecordType>::RECORD_ID => {
                let src = SchSymbolHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchSymbolRecord::from_origin(templates::sch_symbol_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchLabelRecord as RecordType>::RECORD_ID => {
                let src = SchLabelHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchLabelRecord::from_origin(templates::sch_label_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchPowerRecord as RecordType>::RECORD_ID => {
                let src = SchPowerHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchPowerRecord::from_origin(templates::sch_power_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchPortRecord as RecordType>::RECORD_ID => {
                let src = SchPortHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchPortRecord::from_origin(templates::sch_port_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchNoERCRecord as RecordType>::RECORD_ID => {
                let src = SchNoERCHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchNoERCRecord::from_origin(templates::sch_no_erc_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchNetLabelRecord as RecordType>::RECORD_ID => {
                let src = SchNetLabelHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchNetLabelRecord::from_origin(templates::sch_net_label_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchBusRecord as RecordType>::RECORD_ID => {
                let src = SchBusHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchBusRecord::from_origin(templates::sch_bus_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchWireRecord as RecordType>::RECORD_ID => {
                let src = SchWireHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchWireRecord::from_origin(templates::sch_wire_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchTextFrameRecord as RecordType>::RECORD_ID => {
                let src = SchTextFrameHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchTextFrameRecord::from_origin(templates::sch_text_frame_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchJunctionRecord as RecordType>::RECORD_ID => {
                let src = SchJunctionHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchJunctionRecord::from_origin(templates::sch_junction_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchSheetRecord as RecordType>::RECORD_ID => {
                let src = SchSheetHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchSheetRecord::from_origin(templates::sch_sheet_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchSheetNameRecord as RecordType>::RECORD_ID => {
                let src = SchSheetNameHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchSheetNameRecord::from_origin(templates::sch_sheet_name_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchSheetFileNameRecord as RecordType>::RECORD_ID => {
                let src = SchSheetFileNameHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchSheetFileNameRecord::from_origin(templates::sch_sheet_filename_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchBusEntryRecord as RecordType>::RECORD_ID => {
                let src = SchBusEntryHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchBusEntryRecord::from_origin(templates::sch_bus_entry_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchSheetSymbolRecord as RecordType>::RECORD_ID => {
                let src = SchSheetSymbolHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchSheetSymbolRecord::from_origin(templates::sch_sheet_symbol_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchSheetEntryRecord as RecordType>::RECORD_ID => {
                let src = SchSheetEntryHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchSheetEntryRecord::from_origin(templates::sch_sheet_entry_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchImplementationListRecord as RecordType>::RECORD_ID => {
                let src = SchImplementationListHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchImplementationListRecord::from_origin(
                    templates::sch_implementation_list_default(),
                );
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchImplementationRecord as RecordType>::RECORD_ID => {
                let src = SchImplementationHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchImplementationRecord::from_origin(templates::sch_implementation_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchMapDefinerListRecord as RecordType>::RECORD_ID => {
                let src = SchMapDefinerListHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchMapDefinerListRecord::from_origin(templates::sch_map_definer_list_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchMapDefinerRecord as RecordType>::RECORD_ID => {
                let src = SchMapDefinerHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchMapDefinerRecord::from_origin(templates::sch_map_definer_default());
                dst.copy_modeled_fields_from(&src);
                dst.set_implementation_designators(&src.implementation_designators());
                $emit!(dst);
            }
            <SchImplementationParametersRecord as RecordType>::RECORD_ID => {
                let src = SchImplementationParametersHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchImplementationParametersRecord::from_origin(
                    templates::sch_implementation_parameters_default(),
                );
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchNoteRecord as RecordType>::RECORD_ID => {
                let src = SchNoteHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchNoteRecord::from_origin(templates::sch_note_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <SchBlanketRecord as RecordType>::RECORD_ID => {
                let src = SchBlanketHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchBlanketRecord::from_origin(templates::sch_blanket_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            _ => {
                add_skip(
                    $skips,
                    $context,
                    $type_id,
                    "no typed template copier implemented",
                );
            }
        }
    }};
}

macro_rules! copy_pcb_record {
    ($type_id:expr, $rid:expr, $src_store:expr, $emit:ident, $skips:expr, $context:expr) => {{
        match $type_id {
            <PcbArcRecord as RecordType>::RECORD_ID => {
                let src = PcbArcHandle::new($src_store.clone(), $rid).read();
                let mut dst = PcbArcRecord::from_origin(templates::pcb_arc_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <PcbPadRecord as RecordType>::RECORD_ID => {
                let src = PcbPadHandle::new($src_store.clone(), $rid).read();
                let mut dst = PcbPadRecord::from_origin(templates::pcb_pad_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <PcbViaRecord as RecordType>::RECORD_ID => {
                let src = PcbViaHandle::new($src_store.clone(), $rid).read();
                let mut dst = PcbViaRecord::from_origin(templates::pcb_via_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <PcbTrackRecord as RecordType>::RECORD_ID => {
                let src = PcbTrackHandle::new($src_store.clone(), $rid).read();
                let mut dst = PcbTrackRecord::from_origin(templates::pcb_track_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <PcbTextRecord as RecordType>::RECORD_ID => {
                let src = PcbTextHandle::new($src_store.clone(), $rid).read();
                let mut dst = PcbTextRecord::from_origin(templates::pcb_text_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <PcbFillRecord as RecordType>::RECORD_ID => {
                let src = PcbFillHandle::new($src_store.clone(), $rid).read();
                let mut dst = PcbFillRecord::from_origin(templates::pcb_fill_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <PcbRegionRecord as RecordType>::RECORD_ID => {
                let src = PcbRegionHandle::new($src_store.clone(), $rid).read();
                let mut dst = PcbRegionRecord::from_origin(templates::pcb_region_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            <PcbComponentBodyRecord as RecordType>::RECORD_ID => {
                let src = PcbComponentBodyHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    PcbComponentBodyRecord::from_origin(templates::pcb_component_body_default());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
            }
            _ => {
                add_skip(
                    $skips,
                    $context,
                    $type_id,
                    "no typed template copier implemented",
                );
            }
        }
    }};
}

fn rebuild_schlib(
    path: &Path,
    skips: &mut BTreeMap<(String, u8, String), usize>,
) -> Result<PathBuf, Box<dyn Error>> {
    let src = SchLib::open_file(path).map_err(|e| e.to_string())?;
    let dst = SchLib::new_empty();

    let header = src.header();
    dst.set_header(&header);

    let src_store = src.store().clone();
    let components =
        DocumentQuery::<SchComponent>::query_all(&src, "#1").map_err(|e| e.to_string())?;

    for src_comp in components {
        let src_parent = src_comp.read();
        let dst_comp = dst.build_component(templates::sch_component_default, |builder| {
            builder.with_component(|comp| {
                comp.copy_modeled_fields_from(&src_parent);
            });
        });

        for (type_id, rid) in src_comp.all_children() {
            let is_binary = {
                let store = src_store.borrow();
                store.record(rid).origin.is_binary()
            };
            if is_binary {
                if type_id == <SchPinRecord as RecordType>::RECORD_ID {
                    let decoded = {
                        let store = src_store.borrow();
                        store.record(rid).origin.as_binary().and_then(|b| {
                            SchPinRecord::from_legacy_binary_record_data(&b.raw_block)
                        })
                    };
                    if let Some(pin) = decoded {
                        dst_comp.add_child_record(pin);
                    } else {
                        add_skip(
                            skips,
                            "schlib:component-child",
                            type_id,
                            "failed to decode legacy binary sch pin",
                        );
                    }
                } else {
                    add_skip(
                        skips,
                        "schlib:component-child",
                        type_id,
                        "record origin is binary but schematic copier expects params",
                    );
                }
                continue;
            }
            macro_rules! emit_child {
                ($rec:expr) => {{
                    dst_comp.add_child_record($rec);
                }};
            }
            let copied = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                copy_sch_record!(
                    type_id,
                    rid,
                    src_store,
                    emit_child,
                    skips,
                    "schlib:component-child"
                );
            }));
            if copied.is_err() {
                add_skip(
                    skips,
                    "schlib:component-child",
                    type_id,
                    "panic while copying typed record",
                );
            }
        }
    }

    let out = make_temp_rebuild_path(path)?;
    dst.save_file(&out).map_err(|e| e.to_string())?;
    Ok(out)
}

fn rebuild_pcblib(
    path: &Path,
    skips: &mut BTreeMap<(String, u8, String), usize>,
) -> Result<PathBuf, Box<dyn Error>> {
    let src = PcbLib::open_file(path).map_err(|e| e.to_string())?;
    let dst = PcbLib::new_empty();

    let src_store = src.store().clone();
    let names = src.names();
    for name in names {
        let Some(src_fp) = src.find_footprint(&name) else {
            add_skip(
                skips,
                "pcblib:footprint",
                0,
                "footprint not found by name during rebuild",
            );
            continue;
        };

        let src_meta = src_fp.read();
        let dst_fp = dst.build_footprint(&name, templates::pcb_footprint_default, |builder| {
            builder.with_metadata(|meta| {
                meta.copy_modeled_fields_from(&src_meta);
            });
        });

        for (type_id, rid) in src_fp.all_children() {
            let is_binary = {
                let store = src_store.borrow();
                store.record(rid).origin.is_binary()
            };
            if !is_binary {
                add_skip(
                    skips,
                    "pcblib:primitive",
                    type_id,
                    "record origin is params but pcb copier expects binary",
                );
                continue;
            }
            macro_rules! emit_prim {
                ($rec:expr) => {{
                    dst_fp.add_primitive_record($rec);
                }};
            }
            let copied = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                copy_pcb_record!(
                    type_id,
                    rid,
                    src_store,
                    emit_prim,
                    skips,
                    "pcblib:primitive"
                );
            }));
            if copied.is_err() {
                add_skip(
                    skips,
                    "pcblib:primitive",
                    type_id,
                    "panic while copying typed record",
                );
            }
        }
    }

    let out = make_temp_rebuild_path(path)?;
    dst.save_file(&out).map_err(|e| e.to_string())?;
    Ok(out)
}

fn rebuild_schdoc(
    path: &Path,
    skips: &mut BTreeMap<(String, u8, String), usize>,
) -> Result<PathBuf, Box<dyn Error>> {
    let src = SchDoc::open_file(path).map_err(|e| e.to_string())?;
    let dst = SchDoc::new_empty();

    let src_store = src.store().clone();

    for src_comp in src.components() {
        let src_parent = src_comp.read();
        let dst_comp = dst.build_component(templates::sch_component_default, |builder| {
            builder.with_component(|comp| {
                comp.copy_modeled_fields_from(&src_parent);
            });
        });

        for (type_id, rid) in src_comp.all_children() {
            let is_binary = {
                let store = src_store.borrow();
                store.record(rid).origin.is_binary()
            };
            if is_binary {
                if type_id == <SchPinRecord as RecordType>::RECORD_ID {
                    let decoded = {
                        let store = src_store.borrow();
                        store.record(rid).origin.as_binary().and_then(|b| {
                            SchPinRecord::from_legacy_binary_record_data(&b.raw_block)
                        })
                    };
                    if let Some(pin) = decoded {
                        dst_comp.add_child_record(pin);
                    } else {
                        add_skip(
                            skips,
                            "schdoc:component-child",
                            type_id,
                            "failed to decode legacy binary sch pin",
                        );
                    }
                } else {
                    add_skip(
                        skips,
                        "schdoc:component-child",
                        type_id,
                        "record origin is binary but schematic copier expects params",
                    );
                }
                continue;
            }
            macro_rules! emit_child {
                ($rec:expr) => {{
                    dst_comp.add_child_record($rec);
                }};
            }
            let copied = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                copy_sch_record!(
                    type_id,
                    rid,
                    src_store,
                    emit_child,
                    skips,
                    "schdoc:component-child"
                );
            }));
            if copied.is_err() {
                add_skip(
                    skips,
                    "schdoc:component-child",
                    type_id,
                    "panic while copying typed record",
                );
            }
        }
    }

    for (type_id, rid) in src.orphan_records() {
        let is_binary = {
            let store = src_store.borrow();
            store.record(rid).origin.is_binary()
        };
        if is_binary {
            if type_id == <SchPinRecord as RecordType>::RECORD_ID {
                let decoded =
                    {
                        let store = src_store.borrow();
                        store.record(rid).origin.as_binary().and_then(|b| {
                            SchPinRecord::from_legacy_binary_record_data(&b.raw_block)
                        })
                    };
                if let Some(pin) = decoded {
                    dst.add_orphan_record(pin);
                } else {
                    add_skip(
                        skips,
                        "schdoc:orphan",
                        type_id,
                        "failed to decode legacy binary sch pin",
                    );
                }
            } else {
                add_skip(
                    skips,
                    "schdoc:orphan",
                    type_id,
                    "record origin is binary but schematic copier expects params",
                );
            }
            continue;
        }
        macro_rules! emit_orphan {
            ($rec:expr) => {{
                dst.add_orphan_record($rec);
            }};
        }
        let copied = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            copy_sch_record!(type_id, rid, src_store, emit_orphan, skips, "schdoc:orphan");
        }));
        if copied.is_err() {
            add_skip(
                skips,
                "schdoc:orphan",
                type_id,
                "panic while copying typed record",
            );
        }
    }

    let out = make_temp_rebuild_path(path)?;
    dst.save_file(&out).map_err(|e| e.to_string())?;
    Ok(out)
}

/// Rebuild a supported Altium document from high-level types and diff against
/// the original CFB stream layout.
pub fn cmd_rebuild(path: &Path) -> Result<RebuildReport, Box<dyn Error>> {
    let Some(file_type) = classify_extension(path) else {
        return Err(format!("Unsupported file extension for {}", path.display()).into());
    };

    if file_type == "PcbDoc" {
        return Err("PcbDoc v2 rebuild is not implemented yet".into());
    }

    let _panic_hook_silencer = PanicHookSilencer::install();

    let mut skips: BTreeMap<(String, u8, String), usize> = BTreeMap::new();

    let rebuilt_path = match file_type {
        "SchLib" => rebuild_schlib(path, &mut skips)?,
        "PcbLib" => rebuild_pcblib(path, &mut skips)?,
        "SchDoc" => rebuild_schdoc(path, &mut skips)?,
        _ => return Err(format!("Unsupported file type: {}", file_type).into()),
    };

    let original_bytes = fs::read(path)?;
    let rebuilt_bytes = fs::read(&rebuilt_path)?;
    let diff = compare_cfb_files(&original_bytes, &rebuilt_bytes);

    Ok(RebuildReport {
        file_type: file_type.to_string(),
        source_path: path.display().to_string(),
        rebuilt_path: rebuilt_path.display().to_string(),
        skipped_records: finalize_skips(skips),
        diff,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_extension_case_insensitive() {
        assert_eq!(classify_extension(Path::new("x.SchLib")), Some("SchLib"));
        assert_eq!(classify_extension(Path::new("x.pcBlib")), Some("PcbLib"));
        assert_eq!(classify_extension(Path::new("x.SCHDOC")), Some("SchDoc"));
        assert_eq!(classify_extension(Path::new("x.txt")), None);
    }

    #[test]
    fn pcbdoc_returns_not_implemented() {
        let err = cmd_rebuild(Path::new("dummy.PcbDoc")).unwrap_err();
        assert!(
            err.to_string()
                .contains("PcbDoc v2 rebuild is not implemented yet"),
            "unexpected error: {}",
            err
        );
    }
}

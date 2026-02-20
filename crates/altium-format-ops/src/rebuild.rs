// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! High-level from-scratch rebuild command support.
//!
//! Rebuilds supported Altium documents into a temp file using typed record
//! getters/setters and templates, then diffs original vs rebuilt CFB streams.

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
    SchSheetSymbolHandle, SchSymbolHandle, SchTaskHolderHandle, SchTextFrameHandle, SchWireHandle,
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
    SchSheetSymbolRecord, SchSymbolRecord, SchTaskHolderRecord, SchTextFrameRecord, SchWireRecord,
};
use altium_format::v2::store::DocumentMeta;
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

fn resolve_rebuild_path(src: &Path, output: Option<&Path>) -> Result<PathBuf, Box<dyn Error>> {
    let Some(output) = output else {
        return make_temp_rebuild_path(src);
    };

    let src_abs = if src.is_absolute() {
        src.to_path_buf()
    } else {
        std::env::current_dir()?.join(src)
    };
    let out_abs = if output.is_absolute() {
        output.to_path_buf()
    } else {
        std::env::current_dir()?.join(output)
    };
    if src_abs == out_abs {
        return Err("output path must differ from source path".into());
    }
    Ok(output.to_path_buf())
}

/// Preserve source text-record trailing-NUL affinity for keys whose parsed
/// value includes `\0` in the source backing store.
fn copy_param_nul_suffix_from_source<T: RecordType>(
    dst: &mut T,
    src_store: &altium_format::v2::store::DocRef,
    rid: altium_format::v2::ids::RecordId,
) {
    if T::IS_BINARY {
        return;
    }

    let nul_values: Vec<(String, String)> = {
        let store = src_store.borrow();
        let Some(src_param) = store.record(rid).origin.as_param() else {
            return;
        };
        src_param
            .params
            .iter()
            .filter_map(|(k, v)| {
                if v.as_str().contains('\0') {
                    Some((k.to_string(), v.as_str().to_string()))
                } else {
                    None
                }
            })
            .collect()
    };

    if nul_values.is_empty() {
        return;
    }

    let dst_params = &mut dst.origin_mut().param_mut().params;
    for (k, v) in nul_values {
        dst_params.add(&k, &v);
    }
}

/// Copy full param origin from source record to destination record, preserving
/// duplicate keys and original entry order in the parsed parameter collection.
fn copy_param_origin_lossless_from_source<T: RecordType>(
    dst: &mut T,
    src_store: &altium_format::v2::store::DocRef,
    rid: altium_format::v2::ids::RecordId,
) {
    if T::IS_BINARY {
        return;
    }

    let src_param = {
        let store = src_store.borrow();
        store.record(rid).origin.as_param().cloned()
    };

    if let Some(src_param) = src_param {
        *dst.origin_mut().param_mut() = src_param;
    }
}

/// Replace destination param key/value entries with the full parsed key/value
/// set from the source record, preserving source textual forms.
fn copy_all_param_values_from_source<T: RecordType>(
    dst: &mut T,
    src_store: &altium_format::v2::store::DocRef,
    rid: altium_format::v2::ids::RecordId,
) {
    if T::IS_BINARY {
        return;
    }

    let src_values: Vec<(String, String)> = {
        let store = src_store.borrow();
        let Some(src_param) = store.record(rid).origin.as_param() else {
            return;
        };
        src_param
            .params
            .iter()
            .map(|(k, v)| (k.to_string(), v.as_str().to_string()))
            .collect()
    };

    let dst_params = &mut dst.origin_mut().param_mut().params;
    let existing: Vec<String> = dst_params.iter().map(|(k, _)| k.to_string()).collect();
    for key in existing {
        dst_params.remove(&key);
    }
    for (k, v) in src_values {
        dst_params.add(&k, &v);
    }
}

macro_rules! copy_sch_record {
    ($type_id:expr, $rid:expr, $src_store:expr, $emit:ident, $context:expr) => {{
        macro_rules! emit_sch {
            ($dst:ident) => {{
                copy_param_origin_lossless_from_source(&mut $dst, &$src_store, $rid);
                $emit!($dst);
            }};
        }
        let copy_result: std::result::Result<(), String> = match $type_id {
            <SchPinRecord as RecordType>::RECORD_ID => {
                let src = SchPinHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchPinRecord::from_origin(templates::sch_pin_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchArcRecord as RecordType>::RECORD_ID => {
                let src = SchArcHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchArcRecord::from_origin(templates::sch_arc_default());
                dst.copy_modeled_fields_from(&src);
                dst.copy_geometry_encoding_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchLineRecord as RecordType>::RECORD_ID => {
                let src = SchLineHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchLineRecord::from_origin(templates::sch_line_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchRectangleRecord as RecordType>::RECORD_ID => {
                let src = SchRectangleHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchRectangleRecord::from_origin(templates::sch_rectangle_default());
                dst.copy_modeled_fields_from(&src);
                dst.copy_coordinate_encoding_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchBezierRecord as RecordType>::RECORD_ID => {
                let src = SchBezierHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchBezierRecord::from_origin(templates::sch_bezier_default());
                dst.copy_modeled_fields_from(&src);
                dst.copy_vertices_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchPolylineRecord as RecordType>::RECORD_ID => {
                let src = SchPolylineHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchPolylineRecord::from_origin(templates::sch_polyline_default());
                dst.copy_modeled_fields_from(&src);
                dst.copy_vertices_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchPolygonRecord as RecordType>::RECORD_ID => {
                let src = SchPolygonHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchPolygonRecord::from_origin(templates::sch_polygon_default());
                dst.copy_modeled_fields_from(&src);
                dst.copy_vertices_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchEllipseRecord as RecordType>::RECORD_ID => {
                let src = SchEllipseHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchEllipseRecord::from_origin(templates::sch_ellipse_default());
                dst.copy_modeled_fields_from(&src);
                dst.copy_coordinate_encoding_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchPieRecord as RecordType>::RECORD_ID => {
                let src = SchPieHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchPieRecord::from_origin(templates::sch_pie_default());
                dst.copy_modeled_fields_from(&src);
                dst.copy_geometry_encoding_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchRoundRectangleRecord as RecordType>::RECORD_ID => {
                let src = SchRoundRectangleHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchRoundRectangleRecord::from_origin(templates::sch_round_rectangle_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchEllipticalArcRecord as RecordType>::RECORD_ID => {
                let src = SchEllipticalArcHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchEllipticalArcRecord::from_origin(templates::sch_elliptical_arc_default());
                dst.copy_modeled_fields_from(&src);
                dst.copy_geometry_encoding_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchImageRecord as RecordType>::RECORD_ID => {
                let src = SchImageHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchImageRecord::from_origin(templates::sch_image_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchDesignatorRecord as RecordType>::RECORD_ID => {
                let src = SchDesignatorHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchDesignatorRecord::from_origin(templates::sch_designator_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchParameterRecord as RecordType>::RECORD_ID => {
                let src = SchParameterHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchParameterRecord::from_origin(templates::sch_parameter_default());
                dst.copy_modeled_fields_from(&src);
                dst.append_hidden_duplicate_for_export();
                emit_sch!(dst);
                Ok(())
            }
            <SchSymbolRecord as RecordType>::RECORD_ID => {
                let src = SchSymbolHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchSymbolRecord::from_origin(templates::sch_symbol_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchLabelRecord as RecordType>::RECORD_ID => {
                let src = SchLabelHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchLabelRecord::from_origin(templates::sch_label_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchPowerRecord as RecordType>::RECORD_ID => {
                let src = SchPowerHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchPowerRecord::from_origin(templates::sch_power_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchPortRecord as RecordType>::RECORD_ID => {
                let src = SchPortHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchPortRecord::from_origin(templates::sch_port_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchNoERCRecord as RecordType>::RECORD_ID => {
                let src = SchNoERCHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchNoERCRecord::from_origin(templates::sch_no_erc_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchNetLabelRecord as RecordType>::RECORD_ID => {
                let src = SchNetLabelHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchNetLabelRecord::from_origin(templates::sch_net_label_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchBusRecord as RecordType>::RECORD_ID => {
                let src = SchBusHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchBusRecord::from_origin(templates::sch_bus_default());
                dst.copy_modeled_fields_from(&src);
                dst.copy_vertices_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchWireRecord as RecordType>::RECORD_ID => {
                let src = SchWireHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchWireRecord::from_origin(templates::sch_wire_default());
                dst.copy_modeled_fields_from(&src);
                dst.copy_vertices_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchTextFrameRecord as RecordType>::RECORD_ID => {
                let src = SchTextFrameHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchTextFrameRecord::from_origin(templates::sch_text_frame_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchJunctionRecord as RecordType>::RECORD_ID => {
                let src = SchJunctionHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchJunctionRecord::from_origin(templates::sch_junction_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchSheetRecord as RecordType>::RECORD_ID => {
                let src = SchSheetHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchSheetRecord::from_origin(templates::sch_sheet_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchSheetNameRecord as RecordType>::RECORD_ID => {
                let src = SchSheetNameHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchSheetNameRecord::from_origin(templates::sch_sheet_name_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchSheetFileNameRecord as RecordType>::RECORD_ID => {
                let src = SchSheetFileNameHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchSheetFileNameRecord::from_origin(templates::sch_sheet_filename_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchBusEntryRecord as RecordType>::RECORD_ID => {
                let src = SchBusEntryHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchBusEntryRecord::from_origin(templates::sch_bus_entry_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchSheetSymbolRecord as RecordType>::RECORD_ID => {
                let src = SchSheetSymbolHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchSheetSymbolRecord::from_origin(templates::sch_sheet_symbol_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchSheetEntryRecord as RecordType>::RECORD_ID => {
                let src = SchSheetEntryHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchSheetEntryRecord::from_origin(templates::sch_sheet_entry_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchImplementationListRecord as RecordType>::RECORD_ID => {
                let src = SchImplementationListHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchImplementationListRecord::from_origin(
                    templates::sch_implementation_list_default(),
                );
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchImplementationRecord as RecordType>::RECORD_ID => {
                let src = SchImplementationHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchImplementationRecord::from_origin(templates::sch_implementation_default());
                dst.copy_modeled_fields_from(&src);
                dst.set_datafile_links(&src.datafile_links());
                emit_sch!(dst);
                Ok(())
            }
            <SchMapDefinerListRecord as RecordType>::RECORD_ID => {
                let src = SchMapDefinerListHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchMapDefinerListRecord::from_origin(templates::sch_map_definer_list_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchMapDefinerRecord as RecordType>::RECORD_ID => {
                let src = SchMapDefinerHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchMapDefinerRecord::from_origin(templates::sch_map_definer_default());
                dst.copy_modeled_fields_from(&src);
                dst.set_implementation_designators(&src.implementation_designators());
                emit_sch!(dst);
                Ok(())
            }
            <SchImplementationParametersRecord as RecordType>::RECORD_ID => {
                let src = SchImplementationParametersHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchImplementationParametersRecord::from_origin(
                    templates::sch_implementation_parameters_default(),
                );
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchNoteRecord as RecordType>::RECORD_ID => {
                let src = SchNoteHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchNoteRecord::from_origin(templates::sch_note_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchTaskHolderRecord as RecordType>::RECORD_ID => {
                let src = SchTaskHolderHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchTaskHolderRecord::from_origin(templates::sch_task_holder_default());
                dst.copy_modeled_fields_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            <SchBlanketRecord as RecordType>::RECORD_ID => {
                let src = SchBlanketHandle::new($src_store.clone(), $rid).read();
                let mut dst = SchBlanketRecord::from_origin(templates::sch_blanket_default());
                dst.copy_modeled_fields_from(&src);
                dst.copy_vertices_from(&src);
                emit_sch!(dst);
                Ok(())
            }
            _ => Err(format!(
                "{}: unimplemented schematic record_id={}",
                $context, $type_id
            )),
        };
        copy_result
    }};
}

macro_rules! copy_pcb_record {
    ($type_id:expr, $rid:expr, $src_store:expr, $emit:ident, $context:expr) => {{
        let copy_result: std::result::Result<(), String> = match $type_id {
            <PcbArcRecord as RecordType>::RECORD_ID => {
                let src = PcbArcHandle::new($src_store.clone(), $rid).read();
                let mut dst = PcbArcRecord::from_origin(src.origin().clone());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
                Ok(())
            }
            <PcbPadRecord as RecordType>::RECORD_ID => {
                let src = PcbPadHandle::new($src_store.clone(), $rid).read();
                let mut dst = PcbPadRecord::from_origin(src.origin().clone());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
                Ok(())
            }
            <PcbViaRecord as RecordType>::RECORD_ID => {
                let src = PcbViaHandle::new($src_store.clone(), $rid).read();
                let mut dst = PcbViaRecord::from_origin(src.origin().clone());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
                Ok(())
            }
            <PcbTrackRecord as RecordType>::RECORD_ID => {
                let src = PcbTrackHandle::new($src_store.clone(), $rid).read();
                let mut dst = PcbTrackRecord::from_origin(src.origin().clone());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
                Ok(())
            }
            <PcbTextRecord as RecordType>::RECORD_ID => {
                let src = PcbTextHandle::new($src_store.clone(), $rid).read();
                let mut dst = PcbTextRecord::from_origin(src.origin().clone());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
                Ok(())
            }
            <PcbFillRecord as RecordType>::RECORD_ID => {
                let src = PcbFillHandle::new($src_store.clone(), $rid).read();
                let mut dst = PcbFillRecord::from_origin(src.origin().clone());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
                Ok(())
            }
            <PcbRegionRecord as RecordType>::RECORD_ID => {
                let src = PcbRegionHandle::new($src_store.clone(), $rid).read();
                let mut dst = PcbRegionRecord::from_origin(src.origin().clone());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
                Ok(())
            }
            <PcbComponentBodyRecord as RecordType>::RECORD_ID => {
                let src = PcbComponentBodyHandle::new($src_store.clone(), $rid).read();
                let mut dst = PcbComponentBodyRecord::from_origin(src.origin().clone());
                dst.copy_modeled_fields_from(&src);
                $emit!(dst);
                Ok(())
            }
            _ => Err(format!(
                "{}: unimplemented pcb object_id={}",
                $context, $type_id
            )),
        };
        copy_result
    }};
}

fn rebuild_schlib(path: &Path, out: &Path) -> Result<(), Box<dyn Error>> {
    let src = SchLib::open_file(path).map_err(|e| e.to_string())?;
    let dst = SchLib::new_empty();

    let header = src.header();
    dst.set_header(&header);
    dst.set_storage_meta(src.storage_meta());
    dst.set_redirection_streams(src.redirection_streams());

    let src_store = src.store().clone();
    let components =
        DocumentQuery::<SchComponent>::query_all(&src, "#1").map_err(|e| e.to_string())?;

    for src_comp in components {
        let src_parent = src_comp.read();
        let src_parent_id = {
            let store = src_store.borrow();
            store.group(src_comp.group_id()).parent_id()
        };
        let dst_comp = dst.build_component(templates::sch_component_default, |builder| {
            builder.with_component(|comp| {
                comp.copy_modeled_fields_from(&src_parent);
                copy_param_nul_suffix_from_source(comp, &src_store, src_parent_id);
            });
        });
        dst_comp.set_sidecar_streams(src_comp.sidecar_streams());

        for (type_id, rid) in src_comp.all_children() {
            let is_binary = {
                let store = src_store.borrow();
                store.record(rid).origin.is_binary()
            };
            if is_binary {
                if type_id != <SchPinRecord as RecordType>::RECORD_ID {
                    return Err(format!(
                        "schlib:component-child: binary origin only supported for RECORD=2 pins, got record_id={}",
                        type_id
                    )
                    .into());
                }
                let raw_pin = {
                    let store = src_store.borrow();
                    let node = store.record(rid);
                    let Some(bin) = node.origin.as_binary() else {
                        return Err("schlib:component-child: expected binary origin for pin".into());
                    };
                    bin.raw_block.clone()
                };
                let pin = SchPinRecord::from_legacy_binary_record_data(&raw_pin).ok_or_else(
                    || {
                        format!(
                            "schlib:component-child: failed to decode legacy binary pin payload ({} bytes)",
                            raw_pin.len()
                        )
                    },
                )?;
                dst_comp.add_child_record(pin);
                continue;
            }
            macro_rules! emit_child {
                ($rec:expr) => {{
                    dst_comp.add_child_record($rec);
                }};
            }
            copy_sch_record!(
                type_id,
                rid,
                src_store,
                emit_child,
                "schlib:component-child"
            )
            .map_err(|e| -> Box<dyn Error> { e.into() })?;
        }
    }

    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    dst.save_file(&out).map_err(|e| e.to_string())?;
    Ok(())
}

fn rebuild_pcblib(path: &Path, out: &Path) -> Result<(), Box<dyn Error>> {
    let src = PcbLib::open_file(path).map_err(|e| e.to_string())?;
    let dst = PcbLib::new_empty();
    dst.set_section_keys(src.section_keys());
    dst.set_file_header_meta(src.file_header_meta());
    dst.set_file_version_info_meta(src.file_version_info_meta());
    dst.set_library_meta(src.library_meta());

    let src_store = src.store().clone();
    let names = src.names();
    for name in names {
        let Some(src_fp) = src.find_footprint(&name) else {
            return Err(format!(
                "pcblib:footprint: footprint '{}' not found by name during rebuild",
                name
            )
            .into());
        };

        let src_meta = src_fp.read();
        let src_meta_id = {
            let store = src_store.borrow();
            store.group(src_fp.group_id()).parent_id()
        };
        let dst_fp = dst.build_footprint(&name, templates::pcb_footprint_default, |builder| {
            builder.with_metadata(|meta| {
                meta.copy_modeled_fields_from(&src_meta);
                copy_all_param_values_from_source(meta, &src_store, src_meta_id);
                copy_param_nul_suffix_from_source(meta, &src_store, src_meta_id);
            });
        });

        for (type_id, rid) in src_fp.all_children() {
            let is_binary = {
                let store = src_store.borrow();
                store.record(rid).origin.is_binary()
            };
            if !is_binary {
                return Err(format!(
                    "pcblib:primitive: object_id={} has params origin (expected binary)",
                    type_id
                )
                .into());
            }
            macro_rules! emit_prim {
                ($rec:expr) => {{
                    dst_fp.add_primitive_record($rec);
                }};
            }
            copy_pcb_record!(type_id, rid, src_store, emit_prim, "pcblib:primitive")
                .map_err(|e| -> Box<dyn Error> { e.into() })?;
        }

        dst_fp.set_storage_passthrough(src_fp.storage_passthrough());
    }

    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    dst.save_file(&out).map_err(|e| e.to_string())?;
    Ok(())
}

fn rebuild_schdoc(path: &Path, out: &Path) -> Result<(), Box<dyn Error>> {
    let src = SchDoc::open_file(path).map_err(|e| e.to_string())?;
    let dst = SchDoc::new_empty();

    let src_store = src.store().clone();
    // Preserve typed stream metadata for SchDoc stream headers and Storage entries.
    let (src_file_header_meta, src_additional_meta, src_storage_meta) = {
        let store = src_store.borrow();
        match store.meta() {
            DocumentMeta::SchDoc {
                file_header_meta,
                additional_meta,
                storage_meta,
            } => (
                file_header_meta.clone(),
                additional_meta.clone(),
                storage_meta.clone(),
            ),
            _ => (Default::default(), None, Default::default()),
        }
    };
    {
        let mut dst_store = dst.store().borrow_mut();
        if let DocumentMeta::SchDoc {
            file_header_meta,
            additional_meta,
            storage_meta,
        } = dst_store.meta_mut()
        {
            *file_header_meta = src_file_header_meta;
            *additional_meta = src_additional_meta;
            *storage_meta = src_storage_meta;
        }
    }

    for src_comp in src.components() {
        let src_parent = src_comp.read();
        let (
            src_parent_id,
            src_parent_original_index,
            src_parent_stream_name,
            src_parent_snapshot,
            src_child_indices,
        ) = {
            let store = src_store.borrow();
            let group = store.group(src_comp.group_id());
            let parent_id = group.parent_id();
            (
                parent_id,
                group.parent_original_index(),
                store.record(parent_id).stream_name.clone(),
                store.record(parent_id).snapshot_bytes().to_vec(),
                group.original_indices().to_vec(),
            )
        };
        let dst_comp = dst.build_component(templates::sch_component_default, |builder| {
            builder.with_component(|comp| {
                comp.copy_modeled_fields_from(&src_parent);
                copy_param_origin_lossless_from_source(comp, &src_store, src_parent_id);
            });
        });
        {
            let dst_parent_id = {
                let mut dst_store = dst.store().borrow_mut();
                let group = dst_store.group_mut(dst_comp.group_id());
                group.set_parent_original_index(src_parent_original_index);
                group.parent_id()
            };
            let mut dst_store = dst.store().borrow_mut();
            let dst_parent = dst_store.record_mut(dst_parent_id);
            dst_parent.stream_name = src_parent_stream_name;
            dst_parent.original_snapshot = src_parent_snapshot;
            dst_parent.dirty = false;
        }

        for (child_pos, (type_id, rid)) in src_comp.all_children().into_iter().enumerate() {
            let is_binary = {
                let store = src_store.borrow();
                store.record(rid).origin.is_binary()
            };
            if is_binary {
                return Err(format!(
                    "schdoc:component-child: record_id={} has binary origin (strict AD26 mode)",
                    type_id
                )
                .into());
            }
            let src_child_stream_name = {
                let store = src_store.borrow();
                store.record(rid).stream_name.clone()
            };
            let src_child_snapshot = {
                let store = src_store.borrow();
                store.record(rid).snapshot_bytes().to_vec()
            };
            let src_child_original_index = src_child_indices
                .get(child_pos)
                .copied()
                .unwrap_or(usize::MAX);
            macro_rules! emit_child {
                ($rec:expr) => {{
                    let dst_rid = dst_comp.add_child_record($rec);
                    let mut dst_store = dst.store().borrow_mut();
                    {
                        let group = dst_store.group_mut(dst_comp.group_id());
                        group.set_child_original_index(child_pos, src_child_original_index);
                    }
                    let dst_node = dst_store.record_mut(dst_rid);
                    dst_node.stream_name = src_child_stream_name.clone();
                    dst_node.original_snapshot = src_child_snapshot.clone();
                    dst_node.dirty = false;
                }};
            }
            copy_sch_record!(
                type_id,
                rid,
                src_store,
                emit_child,
                "schdoc:component-child"
            )
            .map_err(|e| -> Box<dyn Error> { e.into() })?;
        }
    }

    for (orphan_pos, (type_id, rid)) in src.orphan_records().into_iter().enumerate() {
        let is_binary = {
            let store = src_store.borrow();
            store.record(rid).origin.is_binary()
        };
        if is_binary {
            return Err(format!(
                "schdoc:orphan: record_id={} has binary origin (strict AD26 mode)",
                type_id
            )
            .into());
        }
        let (src_orphan_original_index, src_orphan_stream_name, src_orphan_snapshot) = {
            let store = src_store.borrow();
            (
                store
                    .orphan_original_indices()
                    .get(orphan_pos)
                    .copied()
                    .unwrap_or(usize::MAX),
                store.record(rid).stream_name.clone(),
                store.record(rid).snapshot_bytes().to_vec(),
            )
        };
        macro_rules! emit_orphan {
            ($rec:expr) => {{
                let dst_rid = dst.add_orphan_record($rec);
                let mut dst_store = dst.store().borrow_mut();
                dst_store.set_orphan_original_index(orphan_pos, src_orphan_original_index);
                let dst_node = dst_store.record_mut(dst_rid);
                dst_node.stream_name = src_orphan_stream_name.clone();
                dst_node.original_snapshot = src_orphan_snapshot.clone();
                dst_node.dirty = false;
            }};
        }
        copy_sch_record!(type_id, rid, src_store, emit_orphan, "schdoc:orphan")
            .map_err(|e| -> Box<dyn Error> { e.into() })?;
    }

    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    dst.save_file(&out).map_err(|e| e.to_string())?;
    Ok(())
}

/// Rebuild a supported Altium document from high-level types and diff against
/// the original CFB stream layout.
pub fn cmd_rebuild(path: &Path, output: Option<&Path>) -> Result<RebuildReport, Box<dyn Error>> {
    let Some(file_type) = classify_extension(path) else {
        return Err(format!("Unsupported file extension for {}", path.display()).into());
    };

    if file_type == "PcbDoc" {
        return Err("PcbDoc v2 rebuild is not implemented yet".into());
    }

    let _panic_hook_silencer = PanicHookSilencer::install();

    let rebuilt_path = resolve_rebuild_path(path, output)?;

    match file_type {
        "SchLib" => rebuild_schlib(path, &rebuilt_path)?,
        "PcbLib" => rebuild_pcblib(path, &rebuilt_path)?,
        "SchDoc" => rebuild_schdoc(path, &rebuilt_path)?,
        _ => return Err(format!("Unsupported file type: {}", file_type).into()),
    };

    let original_bytes = fs::read(path)?;
    let rebuilt_bytes = fs::read(&rebuilt_path)?;
    let diff = compare_cfb_files(&original_bytes, &rebuilt_bytes);

    Ok(RebuildReport {
        file_type: file_type.to_string(),
        source_path: path.display().to_string(),
        rebuilt_path: rebuilt_path.display().to_string(),
        skipped_records: Vec::new(),
        diff,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use altium_format::v2::documents::pcblib_streams::{
        PcbLibCountedDataStreamMeta, PcbLibFileHeaderStreamMeta, PcbLibFootprintSidecarStreamsMeta,
        PcbLibLibraryStorageMeta, PcbLibModelsStorageMeta, PcbLibParamTableStreamMeta,
        PcbLibPrimitiveGuidEntry, PcbLibPrimitiveGuidsStreamMeta, PcbLibWideStringsStreamMeta,
    };
    use altium_format::v2::documents::schdoc_streams::{SchDocStorageEntry, SchDocStorageStreamMeta};
    use altium_format::v2::documents::schlib_streams::{
        SchLibRedirectionStreamMeta,
    };
    use altium_format::v2::handles::PcbFootprintStoragePassthrough;
    use altium_format::v2::parameters::ParameterCollection;
    use altium_format::v2::records::PcbFootprintRecord;
    use altium_format::v2::templates;

    #[test]
    fn classify_extension_case_insensitive() {
        assert_eq!(classify_extension(Path::new("x.SchLib")), Some("SchLib"));
        assert_eq!(classify_extension(Path::new("x.pcBlib")), Some("PcbLib"));
        assert_eq!(classify_extension(Path::new("x.SCHDOC")), Some("SchDoc"));
        assert_eq!(classify_extension(Path::new("x.txt")), None);
    }

    #[test]
    fn pcbdoc_returns_not_implemented() {
        let err = cmd_rebuild(Path::new("dummy.PcbDoc"), None).unwrap_err();
        assert!(
            err.to_string()
                .contains("PcbDoc v2 rebuild is not implemented yet"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn output_path_must_differ_from_source_path() {
        let err =
            resolve_rebuild_path(Path::new("a.SchLib"), Some(Path::new("a.SchLib"))).unwrap_err();
        assert!(
            err.to_string().contains("output path must differ"),
            "unexpected error: {}",
            err
        );
    }

    fn temp_file_path(ext: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "altium-rebuild-test-{}-{}.{}",
            std::process::id(),
            ts,
            ext
        ))
    }

    #[test]
    fn rebuild_schlib_preserves_typed_document_streams() {
        let src_path = temp_file_path("SchLib");
        let out_path = temp_file_path("SchLib");

        let src = SchLib::new_empty();
        let mut header = src.header();
        header.unique_id = "SCHLIB-UNITTEST-UID".to_string();
        src.set_header(&header);

        let storage_meta = SchDocStorageStreamMeta {
            header_block: Default::default(),
            entries: vec![SchDocStorageEntry {
                id: "icon-0".to_string(),
                compressed_flags: 0,
                compressed_data: vec![1, 2, 3, 4],
            }],
        };
        src.set_storage_meta(storage_meta.clone());

        let mut redirection_params = ParameterCollection::new();
        redirection_params.add("SECTIONNAME", "U_REAL");
        redirection_params.add("EXTRA", "1");
        let mut redirection_streams = BTreeMap::new();
        redirection_streams.insert(
            "U_ALIAS".to_string(),
            SchLibRedirectionStreamMeta {
                section_name: "U_REAL".to_string(),
                params: redirection_params,
            },
        );
        src.set_redirection_streams(redirection_streams.clone());

        src.build_component(templates::sch_component_default, |builder| {
            builder.with_component(|record| {
                record.set_lib_reference("U_REAL");
                record.set_component_description("test");
                record.set_part_count(1);
            });
        });

        src.save_file(&src_path)
            .expect("failed to save source SchLib fixture");
        rebuild_schlib(&src_path, &out_path).expect("failed to rebuild SchLib fixture");

        let rebuilt = SchLib::open_file(&out_path).expect("failed to open rebuilt SchLib fixture");
        assert_eq!(rebuilt.storage_meta().entries.len(), storage_meta.entries.len());
        assert_eq!(
            rebuilt
                .storage_meta()
                .entries
                .first()
                .map(|e| e.compressed_data.clone()),
            storage_meta
                .entries
                .first()
                .map(|e| e.compressed_data.clone())
        );
        assert_eq!(rebuilt.redirection_streams().len(), redirection_streams.len());
        assert_eq!(
            rebuilt
                .redirection_streams()
                .get("U_ALIAS")
                .map(|m| m.section_name.clone()),
            Some("U_REAL".to_string())
        );

        let _ = fs::remove_file(&src_path);
        let _ = fs::remove_file(&out_path);
    }

    #[test]
    fn rebuild_pcblib_preserves_typed_document_and_footprint_sidecar_streams() {
        let src_path = temp_file_path("PcbLib");
        let out_path = temp_file_path("PcbLib");

        let src = PcbLib::new_empty();
        let mut section_keys = altium_format::v2::documents::section_keys::SectionKeyList::new();
        section_keys.insert_mapping("FP1", "FP1K");
        src.set_section_keys(section_keys.clone());
        src.set_file_header_meta(PcbLibFileHeaderStreamMeta {
            header_text: "PCB 6.0 Binary Library File".to_string(),
            file_version: 5.01,
            key: "ZZZZZZZZ".to_string(),
        });
        src.set_file_version_info_meta(PcbLibCountedDataStreamMeta {
            header_count: 1,
            data: b"version-info".to_vec(),
        });
        src.set_library_meta(PcbLibLibraryStorageMeta {
            header_count: 1,
            data: b"library-data".to_vec(),
            embedded_fonts: b"fonts".to_vec(),
            component_params_toc: PcbLibCountedDataStreamMeta {
                header_count: 1,
                data: b"toc".to_vec(),
            },
            layer_kind_mapping: PcbLibCountedDataStreamMeta {
                header_count: 1,
                data: b"layer-kind".to_vec(),
            },
            models: PcbLibModelsStorageMeta {
                header_count: 1,
                data: b"models".to_vec(),
                entries: {
                    let mut m = BTreeMap::new();
                    m.insert(0, b"model-0".to_vec());
                    m
                },
            },
            models_no_embed: PcbLibCountedDataStreamMeta {
                header_count: 1,
                data: b"models-no-embed".to_vec(),
            },
            pad_via_library: PcbLibCountedDataStreamMeta {
                header_count: 1,
                data: b"pad-via".to_vec(),
            },
            textures: PcbLibCountedDataStreamMeta {
                header_count: 1,
                data: b"textures".to_vec(),
            },
        });

        let fp = src.build_footprint("FP1", templates::pcb_footprint_default, |builder| {
            builder.with_metadata(|meta: &mut PcbFootprintRecord| {
                meta.set_pattern("FP1");
            });
        });

        let mut ws_params = ParameterCollection::new();
        ws_params.add("ENCODEDTEXT0", "65,66,67");
        let mut uid_params = ParameterCollection::new();
        uid_params.add("PRIMITIVEINDEX", "0");
        uid_params.add("UNIQUEID", "{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}");
        let sidecars = PcbLibFootprintSidecarStreamsMeta {
            wide_strings: Some(PcbLibWideStringsStreamMeta {
                entries: vec![ws_params],
            }),
            primitive_guids: Some(PcbLibPrimitiveGuidsStreamMeta {
                entries: vec![PcbLibPrimitiveGuidEntry {
                    tag: 2,
                    index: 0,
                    guid: [7u8; 16],
                }],
            }),
            unique_id_primitive_information: Some(PcbLibParamTableStreamMeta {
                entries: vec![uid_params.clone()],
            }),
            extended_primitive_information: Some(PcbLibParamTableStreamMeta {
                entries: vec![uid_params],
            }),
        };

        fp.set_storage_passthrough(PcbFootprintStoragePassthrough {
            raw_pattern_name_block: b"FP1".to_vec(),
            raw_header: 1u32.to_le_bytes().to_vec(),
            original_primitive_order: Vec::new(),
            sidecar_streams: sidecars.clone(),
        });

        src.save_file(&src_path)
            .expect("failed to save source PcbLib fixture");
        rebuild_pcblib(&src_path, &out_path).expect("failed to rebuild PcbLib fixture");

        let rebuilt = PcbLib::open_file(&out_path).expect("failed to open rebuilt PcbLib fixture");
        assert_eq!(rebuilt.file_header_meta().key, "ZZZZZZZZ");
        assert_eq!(rebuilt.section_keys().len(), section_keys.len());
        assert_eq!(rebuilt.section_keys().get_key("FP1"), "FP1K");
        assert_eq!(rebuilt.file_version_info_meta().data, b"version-info".to_vec());
        assert_eq!(rebuilt.library_meta().data, b"library-data".to_vec());
        assert_eq!(
            rebuilt
                .library_meta()
                .models
                .entries
                .get(&0)
                .cloned(),
            Some(b"model-0".to_vec())
        );

        let rebuilt_fp = rebuilt
            .find_footprint("FP1")
            .expect("rebuilt footprint should exist");
        let rebuilt_pass = rebuilt_fp.storage_passthrough();
        assert_eq!(rebuilt_pass.raw_pattern_name_block, b"FP1".to_vec());
        assert_eq!(rebuilt_pass.raw_header, 1u32.to_le_bytes().to_vec());
        assert_eq!(
            rebuilt_pass
                .sidecar_streams
                .wide_strings
                .as_ref()
                .map(|m| m.entries.len()),
            Some(1)
        );
        assert_eq!(
            rebuilt_pass
                .sidecar_streams
                .primitive_guids
                .as_ref()
                .map(|m| m.entries.len()),
            Some(1)
        );
        assert_eq!(
            rebuilt_pass
                .sidecar_streams
                .unique_id_primitive_information
                .as_ref()
                .map(|m| m.entries.len()),
            Some(1)
        );
        assert_eq!(
            rebuilt_pass
                .sidecar_streams
                .extended_primitive_information
                .as_ref()
                .map(|m| m.entries.len()),
            Some(1)
        );

        let _ = fs::remove_file(&src_path);
        let _ = fs::remove_file(&out_path);
    }

    #[test]
    fn rebuild_schlib_accepts_legacy_binary_pin_children() {
        let src_path = temp_file_path("SchLib");
        let out_path = temp_file_path("SchLib");

        let src = SchLib::new_empty();
        let comp = src.build_component(templates::sch_component_default, |builder| {
            builder.with_component(|record| {
                record.set_lib_reference("BINPIN");
                record.set_component_description("binary pin fixture");
                record.set_part_count(1);
            });
            builder.add_pin(templates::sch_pin_default, |pin| {
                pin.set_designator("1");
                pin.set_name("IO1");
            });
        });

        // Force the pin child origin to binary to mirror real AD SchLib files.
        let pin_rid = comp
            .all_children()
            .into_iter()
            .find(|(record_id, _)| *record_id == <SchPinRecord as RecordType>::RECORD_ID)
            .map(|(_, rid)| rid)
            .expect("component should contain a pin child");
        let pin_raw = SchPinHandle::new(src.store().clone(), pin_rid)
            .read()
            .to_legacy_binary_record_data();
        {
            let mut store = src.store().borrow_mut();
            let node = store.record_mut(pin_rid);
            node.origin = altium_format::v2::backing_store::RecordOrigin::Binary(
                altium_format::v2::backing_store::BinaryOrigin::new(pin_raw),
            );
            node.mark_dirty();
        }

        src.save_file(&src_path)
            .expect("failed to save source SchLib fixture");
        rebuild_schlib(&src_path, &out_path)
            .expect("rebuild should decode and accept binary pin children");

        let rebuilt = SchLib::open_file(&out_path).expect("failed to open rebuilt SchLib fixture");
        let rebuilt_gid = rebuilt
            .find_component("BINPIN")
            .expect("rebuilt component should exist");
        let rebuilt_pin_count = {
            let store = rebuilt.store().borrow();
            let group = store.group(rebuilt_gid);
            group
                .child_ids()
                .iter()
                .filter(|&&rid| store.record(rid).key == <SchPinRecord as RecordType>::RECORD_ID)
                .count()
        };
        assert_eq!(rebuilt_pin_count, 1);

        let _ = fs::remove_file(&src_path);
        let _ = fs::remove_file(&out_path);
    }
}

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

use altium_format::documents::pcbdoc_streams::PcbDocSectionMeta;
use altium_format::documents::{PcbDoc, PcbLib, SchDoc, SchLib};
use altium_format::handles::{
    PcbArcHandle, PcbComponentBodyHandle, PcbFillHandle, PcbPadHandle, PcbRegionHandle,
    PcbTextHandle, PcbTrackHandle, PcbViaHandle, SchArcHandle, SchBusEntryHandle, SchComponent,
    SchDesignatorHandle, SchEllipseHandle, SchEllipticalArcHandle, SchImageHandle,
    SchImplementationHandle, SchImplementationListHandle, SchImplementationParametersHandle,
    SchJunctionHandle, SchLabelHandle, SchLineHandle, SchMapDefinerHandle,
    SchMapDefinerListHandle, SchNetLabelHandle, SchNoERCHandle, SchNoteHandle,
    SchParameterHandle, SchPieHandle, SchPinHandle, SchPortHandle, SchPowerHandle,
    SchRectangleHandle, SchRoundRectangleHandle, SchSheetEntryHandle, SchSheetFileNameHandle,
    SchSheetHandle, SchSheetNameHandle, SchSheetSymbolHandle, SchSymbolHandle,
    SchTaskHolderHandle, SchTextFrameHandle,
};
use altium_format::records::{
    PcbArcRecord, PcbComponentBodyRecord, PcbConnectionRecord, PcbFillRecord, PcbPadRecord,
    PcbRegionRecord, PcbTextRecord, PcbTrackRecord, PcbViaRecord, SchArcRecord, SchBezierRecord,
    SchBlanketRecord, SchBusEntryRecord, SchBusRecord, SchDesignatorRecord, SchEllipseRecord,
    SchEllipticalArcRecord, SchImageRecord, SchImplementationListRecord,
    SchImplementationParametersRecord, SchImplementationRecord, SchJunctionRecord, SchLabelRecord,
    SchLineRecord, SchMapDefinerListRecord, SchMapDefinerRecord, SchNetLabelRecord, SchNoERCRecord,
    SchNoteRecord, SchParameterRecord, SchPieRecord, SchPinRecord, SchPolygonRecord,
    SchPolylineRecord, SchPortRecord, SchPowerRecord, SchRectangleRecord, SchRoundRectangleRecord,
    SchSheetEntryRecord, SchSheetFileNameRecord, SchSheetNameRecord, SchSheetRecord,
    SchSheetSymbolRecord, SchSymbolRecord, SchTaskHolderRecord, SchTextFrameRecord, SchWireRecord,
};
use altium_format::templates;

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

macro_rules! copy_sch_record {
    ($type_id:expr, $rid:expr, $src_store:expr, $emit:ident, $context:expr) => {{
        let copy_result: std::result::Result<(), String> = match $type_id {
            SchPinRecord::RECORD_ID => {
                let src = SchPinHandle::new($src_store.clone(), $rid).read_normalized();
                let dst = SchPinRecord::builder_from(templates::sch_pin_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            SchArcRecord::RECORD_ID => {
                let src = SchArcHandle::new($src_store.clone(), $rid).read();
                let dst = SchArcRecord::builder_from(templates::sch_arc_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            SchLineRecord::RECORD_ID => {
                let src = SchLineHandle::new($src_store.clone(), $rid).read();
                let dst = SchLineRecord::builder_from(templates::sch_line_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            SchRectangleRecord::RECORD_ID => {
                let src = SchRectangleHandle::new($src_store.clone(), $rid).read();
                let dst =
                    SchRectangleRecord::builder_from(templates::sch_rectangle_default, &src)
                        .build();
                $emit!(dst);
                Ok(())
            }
            SchBezierRecord::RECORD_ID => {
                Err(format!(
                    "{}: strict rebuild does not support RECORD={} (Bezier vertices not fully modeled)",
                    $context,
                    SchBezierRecord::RECORD_ID
                ))
            }
            SchPolylineRecord::RECORD_ID => {
                Err(format!(
                    "{}: strict rebuild does not support RECORD={} (Polyline vertices not fully modeled)",
                    $context,
                    SchPolylineRecord::RECORD_ID
                ))
            }
            SchPolygonRecord::RECORD_ID => {
                Err(format!(
                    "{}: strict rebuild does not support RECORD={} (Polygon vertices not fully modeled)",
                    $context,
                    SchPolygonRecord::RECORD_ID
                ))
            }
            SchEllipseRecord::RECORD_ID => {
                let src = SchEllipseHandle::new($src_store.clone(), $rid).read();
                let dst =
                    SchEllipseRecord::builder_from(templates::sch_ellipse_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            SchPieRecord::RECORD_ID => {
                let src = SchPieHandle::new($src_store.clone(), $rid).read();
                let dst = SchPieRecord::builder_from(templates::sch_pie_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            SchRoundRectangleRecord::RECORD_ID => {
                let src = SchRoundRectangleHandle::new($src_store.clone(), $rid).read();
                let dst = SchRoundRectangleRecord::builder_from(
                    templates::sch_round_rectangle_default,
                    &src,
                )
                .build();
                $emit!(dst);
                Ok(())
            }
            SchEllipticalArcRecord::RECORD_ID => {
                let src = SchEllipticalArcHandle::new($src_store.clone(), $rid).read();
                let dst = SchEllipticalArcRecord::builder_from(
                    templates::sch_elliptical_arc_default,
                    &src,
                )
                .build();
                $emit!(dst);
                Ok(())
            }
            SchImageRecord::RECORD_ID => {
                let src = SchImageHandle::new($src_store.clone(), $rid).read();
                let dst =
                    SchImageRecord::builder_from(templates::sch_image_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            SchDesignatorRecord::RECORD_ID => {
                let src = SchDesignatorHandle::new($src_store.clone(), $rid).read();
                let dst =
                    SchDesignatorRecord::builder_from(templates::sch_designator_default, &src)
                        .build();
                $emit!(dst);
                Ok(())
            }
            SchParameterRecord::RECORD_ID => {
                let src = SchParameterHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchParameterRecord::builder_from(templates::sch_parameter_default, &src)
                        .build();
                dst.append_hidden_duplicate_for_export();
                $emit!(dst);
                Ok(())
            }
            SchSymbolRecord::RECORD_ID => {
                let src = SchSymbolHandle::new($src_store.clone(), $rid).read();
                let dst =
                    SchSymbolRecord::builder_from(templates::sch_symbol_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            SchLabelRecord::RECORD_ID => {
                let src = SchLabelHandle::new($src_store.clone(), $rid).read();
                let dst = SchLabelRecord::builder_from(templates::sch_label_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            SchPowerRecord::RECORD_ID => {
                let src = SchPowerHandle::new($src_store.clone(), $rid).read();
                let dst = SchPowerRecord::builder_from(templates::sch_power_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            SchPortRecord::RECORD_ID => {
                let src = SchPortHandle::new($src_store.clone(), $rid).read();
                let dst = SchPortRecord::builder_from(templates::sch_port_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            SchNoERCRecord::RECORD_ID => {
                let src = SchNoERCHandle::new($src_store.clone(), $rid).read();
                let dst =
                    SchNoERCRecord::builder_from(templates::sch_no_erc_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            SchNetLabelRecord::RECORD_ID => {
                let src = SchNetLabelHandle::new($src_store.clone(), $rid).read();
                let dst =
                    SchNetLabelRecord::builder_from(templates::sch_net_label_default, &src)
                        .build();
                $emit!(dst);
                Ok(())
            }
            SchBusRecord::RECORD_ID => {
                Err(format!(
                    "{}: strict rebuild does not support RECORD={} (Bus vertices not fully modeled)",
                    $context,
                    SchBusRecord::RECORD_ID
                ))
            }
            SchWireRecord::RECORD_ID => {
                Err(format!(
                    "{}: strict rebuild does not support RECORD={} (Wire vertices not fully modeled)",
                    $context,
                    SchWireRecord::RECORD_ID
                ))
            }
            SchTextFrameRecord::RECORD_ID => {
                let src = SchTextFrameHandle::new($src_store.clone(), $rid).read();
                let dst =
                    SchTextFrameRecord::builder_from(templates::sch_text_frame_default, &src)
                        .build();
                $emit!(dst);
                Ok(())
            }
            SchJunctionRecord::RECORD_ID => {
                let src = SchJunctionHandle::new($src_store.clone(), $rid).read();
                let dst =
                    SchJunctionRecord::builder_from(templates::sch_junction_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            SchSheetRecord::RECORD_ID => {
                let src = SchSheetHandle::new($src_store.clone(), $rid).read();
                let dst = SchSheetRecord::builder_from(templates::sch_sheet_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            SchSheetNameRecord::RECORD_ID => {
                let src = SchSheetNameHandle::new($src_store.clone(), $rid).read();
                let dst =
                    SchSheetNameRecord::builder_from(templates::sch_sheet_name_default, &src)
                        .build();
                $emit!(dst);
                Ok(())
            }
            SchSheetFileNameRecord::RECORD_ID => {
                let src = SchSheetFileNameHandle::new($src_store.clone(), $rid).read();
                let dst = SchSheetFileNameRecord::builder_from(
                    templates::sch_sheet_filename_default,
                    &src,
                )
                .build();
                $emit!(dst);
                Ok(())
            }
            SchBusEntryRecord::RECORD_ID => {
                let src = SchBusEntryHandle::new($src_store.clone(), $rid).read();
                let dst =
                    SchBusEntryRecord::builder_from(templates::sch_bus_entry_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            SchSheetSymbolRecord::RECORD_ID => {
                let src = SchSheetSymbolHandle::new($src_store.clone(), $rid).read();
                let dst = SchSheetSymbolRecord::builder_from(
                    templates::sch_sheet_symbol_default,
                    &src,
                )
                .build();
                $emit!(dst);
                Ok(())
            }
            SchSheetEntryRecord::RECORD_ID => {
                let src = SchSheetEntryHandle::new($src_store.clone(), $rid).read();
                let dst =
                    SchSheetEntryRecord::builder_from(templates::sch_sheet_entry_default, &src)
                        .build();
                $emit!(dst);
                Ok(())
            }
            SchImplementationListRecord::RECORD_ID => {
                let src = SchImplementationListHandle::new($src_store.clone(), $rid).read();
                let dst = SchImplementationListRecord::builder_from(
                    templates::sch_implementation_list_default,
                    &src,
                )
                .build();
                $emit!(dst);
                Ok(())
            }
            SchImplementationRecord::RECORD_ID => {
                let src = SchImplementationHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchImplementationRecord::builder_from(templates::sch_implementation_default, &src)
                        .build();
                dst.set_datafile_links(&src.datafile_links());
                $emit!(dst);
                Ok(())
            }
            SchMapDefinerListRecord::RECORD_ID => {
                let src = SchMapDefinerListHandle::new($src_store.clone(), $rid).read();
                let dst = SchMapDefinerListRecord::builder_from(
                    templates::sch_map_definer_list_default,
                    &src,
                )
                .build();
                $emit!(dst);
                Ok(())
            }
            SchMapDefinerRecord::RECORD_ID => {
                let src = SchMapDefinerHandle::new($src_store.clone(), $rid).read();
                let mut dst =
                    SchMapDefinerRecord::builder_from(templates::sch_map_definer_default, &src)
                        .build();
                dst.set_implementation_designators(&src.implementation_designators());
                $emit!(dst);
                Ok(())
            }
            SchImplementationParametersRecord::RECORD_ID => {
                let src = SchImplementationParametersHandle::new($src_store.clone(), $rid).read();
                let dst = SchImplementationParametersRecord::builder_from(
                    templates::sch_implementation_parameters_default,
                    &src,
                )
                .build();
                $emit!(dst);
                Ok(())
            }
            SchNoteRecord::RECORD_ID => {
                let src = SchNoteHandle::new($src_store.clone(), $rid).read();
                let dst = SchNoteRecord::builder_from(templates::sch_note_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            SchTaskHolderRecord::RECORD_ID => {
                let src = SchTaskHolderHandle::new($src_store.clone(), $rid).read();
                let dst =
                    SchTaskHolderRecord::builder_from(templates::sch_task_holder_default, &src)
                        .build();
                $emit!(dst);
                Ok(())
            }
            SchBlanketRecord::RECORD_ID => {
                Err(format!(
                    "{}: strict rebuild does not support RECORD={} (Blanket vertices not fully modeled)",
                    $context,
                    SchBlanketRecord::RECORD_ID
                ))
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
            PcbArcRecord::RECORD_ID => {
                let src = PcbArcHandle::new($src_store.clone(), $rid).read();
                let dst = PcbArcRecord::builder_from(templates::pcb_arc_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            PcbPadRecord::RECORD_ID => {
                let src = PcbPadHandle::new($src_store.clone(), $rid).read();
                let dst = PcbPadRecord::builder_from(templates::pcb_pad_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            PcbViaRecord::RECORD_ID => {
                let src = PcbViaHandle::new($src_store.clone(), $rid).read();
                let dst = PcbViaRecord::builder_from(templates::pcb_via_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            PcbTrackRecord::RECORD_ID => {
                let src = PcbTrackHandle::new($src_store.clone(), $rid).read();
                let dst = PcbTrackRecord::builder_from(templates::pcb_track_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            PcbTextRecord::RECORD_ID => {
                let src = PcbTextHandle::new($src_store.clone(), $rid).read();
                let dst = PcbTextRecord::builder_from(templates::pcb_text_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            PcbFillRecord::RECORD_ID => {
                let src = PcbFillHandle::new($src_store.clone(), $rid).read();
                let dst = PcbFillRecord::builder_from(templates::pcb_fill_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            PcbConnectionRecord::RECORD_ID => {
                Err(format!(
                    "{}: strict rebuild does not support pcb object_id={} (no high-level template)",
                    $context,
                    PcbConnectionRecord::RECORD_ID
                ))
            }
            PcbRegionRecord::RECORD_ID => {
                let src = PcbRegionHandle::new($src_store.clone(), $rid).read();
                let dst = PcbRegionRecord::builder_from(templates::pcb_region_default, &src).build();
                $emit!(dst);
                Ok(())
            }
            PcbComponentBodyRecord::RECORD_ID => {
                let src = PcbComponentBodyHandle::new($src_store.clone(), $rid).read();
                let dst = PcbComponentBodyRecord::builder_from(
                    templates::pcb_component_body_default,
                    &src,
                )
                .build();
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

    let components = src.query_all::<SchComponent>("#1").map_err(|e| e.to_string())?;

    for src_comp in components {
        let src_parent = src_comp.read();
        let dst_comp = dst.build_component(templates::sch_component_default, |builder| {
            let rebuilt = altium_format::records::SchComponentRecord::builder_from(
                templates::sch_component_default,
                &src_parent,
            )
            .build();
            builder.with_component(|comp| *comp = rebuilt);
        });
        dst_comp.set_sidecar_streams(src_comp.sidecar_streams());

        for (type_id, rid) in src_comp.all_children() {
            macro_rules! emit_child {
                ($rec:expr) => {{
                    dst_comp.add_child_record($rec);
                }};
            }
            copy_sch_record!(
                type_id,
                rid,
                src.store().clone(),
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
        let dst_fp = dst.build_footprint(&name, templates::pcb_footprint_default, |builder| {
            let rebuilt = altium_format::records::PcbFootprintRecord::builder_from(
                templates::pcb_footprint_default,
                &src_meta,
            )
            .build();
            builder.with_metadata(|meta| *meta = rebuilt);
        });

        for (type_id, rid) in src_fp.all_children() {
            macro_rules! emit_prim {
                ($rec:expr) => {{
                    dst_fp.add_primitive_record($rec);
                }};
            }
            copy_pcb_record!(
                type_id,
                rid,
                src.store().clone(),
                emit_prim,
                "pcblib:primitive"
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

fn rebuild_schdoc(path: &Path, out: &Path) -> Result<(), Box<dyn Error>> {
    let src = SchDoc::open_file(path).map_err(|e| e.to_string())?;
    let dst = SchDoc::new_empty();

    dst.set_file_header_meta(src.file_header_meta());
    dst.set_additional_meta(src.additional_meta());
    dst.set_storage_meta(src.storage_meta());

    for src_comp in src.components() {
        let src_parent = src_comp.read();
        let dst_comp = dst.build_component(templates::sch_component_default, |builder| {
            let rebuilt = altium_format::records::SchComponentRecord::builder_from(
                templates::sch_component_default,
                &src_parent,
            )
            .build();
            builder.with_component(|comp| *comp = rebuilt);
        });

        for (type_id, rid) in src_comp.all_children().into_iter() {
            macro_rules! emit_child {
                ($rec:expr) => {{
                    dst_comp.add_child_record($rec);
                }};
            }
            copy_sch_record!(
                type_id,
                rid,
                src.store().clone(),
                emit_child,
                "schdoc:component-child"
            )
            .map_err(|e| -> Box<dyn Error> { e.into() })?;
        }
    }

    for (type_id, rid) in src.orphan_records().into_iter() {
        macro_rules! emit_orphan {
            ($rec:expr) => {{
                dst.add_orphan_record($rec);
            }};
        }
        copy_sch_record!(
            type_id,
            rid,
            src.store().clone(),
            emit_orphan,
            "schdoc:orphan"
        )
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

fn rebuild_pcbdoc(path: &Path, out: &Path) -> Result<(), Box<dyn Error>> {
    let src = PcbDoc::open_file(path).map_err(|e| e.to_string())?;
    let dst = PcbDoc::new_empty();

    let mut streams_meta = src.streams_meta();
    let primitive_section_names: Vec<String> = streams_meta
        .sections
        .iter_mut()
        .filter_map(|(section_name, section_meta)| match section_meta {
            PcbDocSectionMeta::Primitive(primitive_meta) => {
                primitive_meta.record_ids.clear();
                Some(section_name.clone())
            }
            _ => None,
        })
        .collect();
    dst.set_streams_meta(streams_meta);

    for section_name in primitive_section_names {
        for (type_id, rid) in src.primitive_records(&section_name) {
            macro_rules! emit_prim {
                ($rec:expr) => {{
                    dst.add_primitive_record(&section_name, $rec)
                        .map_err(|e| -> Box<dyn Error> { e.to_string().into() })?;
                }};
            }
            copy_pcb_record!(
                type_id,
                rid,
                src.store().clone(),
                emit_prim,
                "pcbdoc:primitive"
            )
                .map_err(|e| -> Box<dyn Error> { e.into() })?;
        }
    }

    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    dst.save_file(out).map_err(|e| e.to_string())?;
    Ok(())
}

/// Rebuild a supported Altium document from high-level types and diff against
/// the original CFB stream layout.
pub fn cmd_rebuild(path: &Path, output: Option<&Path>) -> Result<RebuildReport, Box<dyn Error>> {
    let Some(file_type) = classify_extension(path) else {
        return Err(format!("Unsupported file extension for {}", path.display()).into());
    };

    let _panic_hook_silencer = PanicHookSilencer::install();

    let rebuilt_path = resolve_rebuild_path(path, output)?;

    match file_type {
        "SchLib" => rebuild_schlib(path, &rebuilt_path)?,
        "PcbLib" => rebuild_pcblib(path, &rebuilt_path)?,
        "SchDoc" => rebuild_schdoc(path, &rebuilt_path)?,
        "PcbDoc" => rebuild_pcbdoc(path, &rebuilt_path)?,
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

    use altium_format::documents::pcblib_streams::{
        PcbLibCountedDataStreamMeta, PcbLibFileHeaderStreamMeta, PcbLibFootprintSidecarStreamsMeta,
        PcbLibLibraryStorageMeta, PcbLibModelsStorageMeta, PcbLibParamTableStreamMeta,
        PcbLibPrimitiveGuidEntry, PcbLibPrimitiveGuidsStreamMeta, PcbLibWideStringsStreamMeta,
    };
    use altium_format::documents::schdoc_streams::{
        SchDocStorageEntry, SchDocStorageStreamMeta,
    };
    use altium_format::documents::schlib_streams::SchLibRedirectionStreamMeta;
    use altium_format::handles::PcbFootprintStoragePassthrough;
    use altium_format::parameters::ParameterCollection;
    use altium_format::records::PcbFootprintRecord;
    use altium_format::templates;

    #[test]
    fn classify_extension_case_insensitive() {
        assert_eq!(classify_extension(Path::new("x.SchLib")), Some("SchLib"));
        assert_eq!(classify_extension(Path::new("x.pcBlib")), Some("PcbLib"));
        assert_eq!(classify_extension(Path::new("x.SCHDOC")), Some("SchDoc"));
        assert_eq!(classify_extension(Path::new("x.txt")), None);
    }

    #[test]
    fn pcbdoc_rebuild_requires_existing_source_file() {
        let err = cmd_rebuild(Path::new("dummy.PcbDoc"), None).unwrap_err();
        assert!(
            err.to_string().contains("I/O error"),
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
        assert_eq!(
            rebuilt.storage_meta().entries.len(),
            storage_meta.entries.len()
        );
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
        assert_eq!(
            rebuilt.redirection_streams().len(),
            redirection_streams.len()
        );
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
    fn rebuild_pcblib_preserves_typed_document_streams_but_not_raw_passthrough() {
        let src_path = temp_file_path("PcbLib");
        let out_path = temp_file_path("PcbLib");

        let src = PcbLib::new_empty();
        let mut section_keys = altium_format::documents::section_keys::SectionKeyList::new();
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
        assert_eq!(
            rebuilt.file_version_info_meta().data,
            b"version-info".to_vec()
        );
        assert_eq!(rebuilt.library_meta().data, b"library-data".to_vec());
        assert_eq!(
            rebuilt.library_meta().models.entries.get(&0).cloned(),
            Some(b"model-0".to_vec())
        );

        let rebuilt_fp = rebuilt
            .find_footprint("FP1")
            .expect("rebuilt footprint should exist");
        let rebuilt_pass = rebuilt_fp.storage_passthrough();
        assert!(
            rebuilt_pass.sidecar_streams.wide_strings.is_none(),
            "strict rebuild should not passthrough sidecar streams"
        );

        let _ = fs::remove_file(&src_path);
        let _ = fs::remove_file(&out_path);
    }

}

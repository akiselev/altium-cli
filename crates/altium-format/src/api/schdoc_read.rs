//! Read path: convert internal SchDoc flat records → public API types.
//!
//! The core challenge is resolving the flat `OWNERINDEX`-linked record list into
//! a nested `SchDocSheet` tree. The algorithm:
//!
//! 1. Build ownership map: `parent_index → Vec<child_index>`
//! 2. Extract Sheet (record 0): fonts, display settings
//! 3. Find Template among Sheet's children, collect template-owned graphics
//! 4. Walk remaining Sheet children in order, converting each to a `SheetObject`
//! 5. For container types (Component, SheetSymbol, etc.), recursively collect children

use std::collections::HashMap;

use crate::api::sch_common::{
    build_footprint_maps_schdoc, graphic_from_record, parameter_from_internal, pin_from_internal,
};
use crate::api::schdoc_types::*;
use crate::api::schlib_types::Parameter;
use crate::sch_records::SchRecord;
use crate::{AltiumFormatError, Result, ResultExt};

use altium_format_types::constants::record_structure::RECORD;
use altium_format_types::sch::SheetStyle;

/// Convert the flat SchDoc record lists into a structured `SchDocSheet`.
pub(crate) fn sheet_from_internal(
    records: &[SchRecord],
    additional_records: &[SchRecord],
) -> Result<SchDocSheet> {
    if records.is_empty() {
        return Err(AltiumFormatError::InvalidParamValue {
            key: RECORD.to_owned(),
            detail: "SchDoc has no records".to_owned(),
        });
    }

    // Step 1: Build ownership map (parent_index → child_indices)
    let ownership_map = build_ownership_map(records, additional_records);

    // Step 2: Extract Sheet (record 0)
    let sheet = match &records[0] {
        SchRecord::Sheet(s) => s,
        other => {
            return Err(AltiumFormatError::InvalidParamValue {
                key: RECORD.to_owned(),
                detail: format!(
                    "expected Sheet (RECORD=31) at index 0, found {:?}",
                    std::mem::discriminant(other)
                ),
            });
        }
    };

    // Extract fonts
    let fonts: Vec<Font> = sheet
        .fonts
        .iter()
        .map(|f| Font {
            id: f.id,
            name: f.name.clone(),
            size: f.size,
            bold: f.bold,
            italic: f.italic,
            underline: f.underline,
            strikeout: f.strikeout,
            rotation: f.rotation,
        })
        .collect();

    // Extract display settings
    let ds = &sheet.display_settings;

    // Step 3: Find Template among Sheet's children
    let sheet_children = ownership_map.get(&0).cloned().unwrap_or_default();
    let mut template = Template {
        file_name: String::new(),
        children: Vec::new(),
    };
    let mut template_idx: Option<usize> = None;

    for &child_idx in &sheet_children {
        let child_rec = resolve_record(records, additional_records, child_idx);
        if let SchRecord::Template(t) = child_rec {
            template.file_name = t.file_name.clone();
            template_idx = Some(child_idx);

            // Collect template-owned graphics
            if let Some(tmpl_children) = ownership_map.get(&child_idx) {
                for &tmpl_child_idx in tmpl_children {
                    let tmpl_rec = resolve_record(records, additional_records, tmpl_child_idx);
                    if let Some(graphic) = graphic_from_record(tmpl_rec) {
                        template.children.push(graphic);
                    }
                    // Template children can also be Parameters, Labels, etc.
                    // that are part of the title block
                    else if let SchRecord::Parameter(_) = tmpl_rec {
                        // Template parameters are just graphics for the title block
                        // Skip them as they're format-internal
                    }
                    // Designator records within template (title block fields)
                    else if let SchRecord::Designator(_) = tmpl_rec {
                        // Format-internal title block designator, skip
                    }
                }
            }
            break;
        }
    }

    // Step 4: Walk remaining Sheet children in order, building SheetObjects
    let mut objects = Vec::new();

    for &child_idx in &sheet_children {
        // Skip the template — it's extracted as a separate field
        if Some(child_idx) == template_idx {
            continue;
        }

        let obj = convert_sheet_child(records, additional_records, child_idx, &ownership_map)
            .with_context(|| format!("converting sheet child at record index {child_idx}"))?;

        if let Some(o) = obj {
            objects.push(o);
        }
    }

    Ok(SchDocSheet {
        fonts,
        snap_grid_size: ds.snap_grid_size.unwrap_or_default(),
        visible_grid_size: ds.visible_grid_size.unwrap_or_default(),
        hot_spot_grid_size: ds.hot_spot_grid_size.unwrap_or_default(),
        snap_grid_on: ds.snap_grid_on.unwrap_or(true),
        visible_grid_on: ds.visible_grid_on.unwrap_or(true),
        hot_spot_grid_on: ds.hot_spot_grid_on.unwrap_or(true),
        sheet_style: ds.sheet_style.unwrap_or(SheetStyle::A4),
        use_custom_sheet: ds.use_custom_sheet.unwrap_or(false),
        custom_width: ds.custom_x.unwrap_or_default(),
        custom_height: ds.custom_y.unwrap_or_default(),
        area_color: ds.area_color.unwrap_or(altium_format_types::Color::WHITE),
        border_on: ds.border_on.unwrap_or(true),
        title_block_on: ds.title_block_on.unwrap_or(true),
        show_template_graphics: ds.show_template_graphics.unwrap_or(true),
        template_file_name: ds.template_file_name.clone().unwrap_or_default(),
        display_unit: ds.display_unit.map(|u| u as i32).unwrap_or(0),
        workspace_orientation: ds.workspace_orientation.map(|o| o as i32).unwrap_or(0),
        show_hidden_pins: ds.show_hidden_pins.unwrap_or(false),
        template,
        objects,
    })
}

/// Build the ownership map from flat records.
///
/// Returns `parent_index → Vec<child_index>` using a virtual index scheme:
/// - Main records (from `/FileHeader`): indices `0..records.len()`
/// - Additional records (from `/Additional`): indices `records.len()..records.len()+additional.len()`
///
/// This ensures every record from both streams is represented in the tree.
fn build_ownership_map(
    records: &[SchRecord],
    additional_records: &[SchRecord],
) -> HashMap<usize, Vec<usize>> {
    let mut map: HashMap<usize, Vec<usize>> = HashMap::new();
    let base_len = records.len();

    // Main records: owner_index always points into the main records list.
    // Skip self-references (e.g. Sheet at index 0 has owner_index=0).
    for (idx, record) in records.iter().enumerate() {
        let (owner_index, _is_additional) = owner_ref(record);
        if owner_index >= 0 && owner_index as usize != idx {
            map.entry(owner_index as usize).or_default().push(idx);
        }
    }

    // Additional records reference either the main list or the additional list
    // depending on the `owner_index_additional_list` flag (pins, harness
    // entries/types, signal harness children, …: every record carries it).
    for (idx, record) in additional_records.iter().enumerate() {
        let (owner_index, owner_is_additional) = owner_ref(record);
        if owner_index >= 0 {
            let parent_virtual_idx = if owner_is_additional {
                // Owner is in the additional records list
                base_len + owner_index as usize
            } else {
                // Owner is in the main records list
                owner_index as usize
            };
            map.entry(parent_virtual_idx)
                .or_default()
                .push(base_len + idx);
        }
    }

    map
}

/// Resolve a virtual index to the actual `SchRecord` reference.
///
/// Virtual indices `0..records.len()` map to `records`, and
/// `records.len()..` map to `additional_records`.
fn resolve_record<'a>(
    records: &'a [SchRecord],
    additional_records: &'a [SchRecord],
    virtual_idx: usize,
) -> &'a SchRecord {
    if virtual_idx < records.len() {
        &records[virtual_idx]
    } else {
        &additional_records[virtual_idx - records.len()]
    }
}

use crate::schdoc::owner_ref;

/// Convert a single Sheet child record into a `SheetObject`.
///
/// Returns `None` for records that are invisible to the API (e.g.,
/// ImplementationList, ParameterList which are children of Components
/// but may appear at sheet level in some edge cases).
fn convert_sheet_child(
    records: &[SchRecord],
    additional_records: &[SchRecord],
    idx: usize,
    ownership_map: &HashMap<usize, Vec<usize>>,
) -> Result<Option<SheetObject>> {
    let record = resolve_record(records, additional_records, idx);
    match record {
        SchRecord::Component(comp) => {
            let children = ownership_map.get(&idx).cloned().unwrap_or_default();
            let component = component_from_schdoc_internal(
                records,
                additional_records,
                comp,
                &children,
                ownership_map,
            )?;
            Ok(Some(SheetObject::Component(component)))
        }
        SchRecord::Wire(w) => Ok(Some(SheetObject::Wire(wire_from_internal(w)))),
        SchRecord::Bus(b) => Ok(Some(SheetObject::Bus(bus_from_internal(b)))),
        SchRecord::NetLabel(n) => Ok(Some(SheetObject::NetLabel(net_label_from_internal(n)))),
        SchRecord::PowerObject(p) => Ok(Some(SheetObject::PowerObject(
            power_object_from_internal(p),
        ))),
        SchRecord::Port(p) => Ok(Some(SheetObject::Port(port_from_internal(p)))),
        SchRecord::Junction(j) => Ok(Some(SheetObject::Junction(junction_from_internal(j)))),
        SchRecord::NoConnect(n) => Ok(Some(SheetObject::NoConnect(no_connect_from_internal(n)))),
        SchRecord::BusEntry(b) => Ok(Some(SheetObject::BusEntry(bus_entry_from_internal(b)))),
        SchRecord::SheetSymbol(ss) => {
            let children = ownership_map.get(&idx).cloned().unwrap_or_default();
            let sym = sheet_symbol_from_internal(
                records,
                additional_records,
                ss,
                &children,
                ownership_map,
            )?;
            Ok(Some(SheetObject::SheetSymbol(sym)))
        }
        SchRecord::ParameterSet(ps) => {
            let children = ownership_map.get(&idx).cloned().unwrap_or_default();
            let pset = parameter_set_from_internal(records, additional_records, ps, &children)?;
            Ok(Some(SheetObject::ParameterSet(pset)))
        }
        SchRecord::Note(n) => Ok(Some(SheetObject::Note(note_from_internal(n)))),
        SchRecord::Probe(p) => Ok(Some(SheetObject::Probe(probe_from_internal(p)))),
        SchRecord::CompileMask(c) => Ok(Some(SheetObject::CompileMask(
            compile_mask_from_internal(c),
        ))),
        SchRecord::Blanket(b) => Ok(Some(SheetObject::Blanket(blanket_from_internal(b)))),
        SchRecord::HarnessConnector(hc) => {
            let children = ownership_map.get(&idx).cloned().unwrap_or_default();
            let conn = harness_connector_from_internal(records, additional_records, hc, &children)?;
            Ok(Some(SheetObject::HarnessConnector(conn)))
        }
        SchRecord::SignalHarness(sh) => Ok(Some(SheetObject::SignalHarness(
            signal_harness_from_internal(sh),
        ))),
        // Sheet-level parameters
        SchRecord::Parameter(p) => Ok(Some(SheetObject::Parameter(parameter_from_internal(p)))),
        // Sheet-level designators (title block fields)
        SchRecord::Designator(d) => {
            // Designators at sheet level are title block fields — treat as parameters
            Ok(Some(SheetObject::Parameter(Parameter {
                name: d.name.clone(),
                text: d.text.clone(),
                is_hidden: d.is_hidden,
                read_only: d.read_only_state,
                location: d.location,
                orientation: d.orientation,
                color: d.color,
                font_id: d.font_id,
                justification: d.justification,
                is_mirrored: d.is_mirrored,
                show_name: d.show_name,
                unique_id: d.unique_id.clone(),
                not_auto_position: d.not_auto_position,
                param_type: d.param_type,
                description: d.description.clone(),
            })))
        }
        // Container records that are sheet-owned but invisible to API
        SchRecord::ImplementationList(_)
        | SchRecord::Implementation(_)
        | SchRecord::ImplementationMap(_)
        | SchRecord::MapDefiner(_)
        | SchRecord::ParameterList(_) => Ok(None),
        // SheetName/SheetFileName at sheet level (shouldn't happen, but handle gracefully)
        SchRecord::SheetName(_) | SchRecord::SheetFileName(_) => Ok(None),
        // HighLevelCode variants (treated like SheetSymbol)
        SchRecord::HighLevelCodeSymbol(ss) => {
            let children = ownership_map.get(&idx).cloned().unwrap_or_default();
            let sym = sheet_symbol_from_internal(
                records,
                additional_records,
                ss,
                &children,
                ownership_map,
            )?;
            Ok(Some(SheetObject::SheetSymbol(sym)))
        }
        other => match graphic_from_record(other) {
            Some(graphic) => Ok(Some(SheetObject::Graphic(graphic))),
            None => Err(AltiumFormatError::InvalidParamValue {
                key: RECORD.to_owned(),
                detail: format!(
                    "unexpected record type {:?} as sheet child at index {}",
                    std::mem::discriminant(other),
                    idx
                ),
            }),
        },
    }
}

// ── Container converters ─────────────────────────────────────────────────────

/// Convert a SchComponent and its children into a SchDocComponent.
///
/// `child_indices` uses virtual indices: values `< records.len()` refer to main
/// records, values `>= records.len()` refer to additional records (offset by
/// `records.len()`).
fn component_from_schdoc_internal(
    records: &[SchRecord],
    additional_records: &[SchRecord],
    comp: &crate::sch_records::SchComponent,
    child_indices: &[usize],
    ownership_map: &HashMap<usize, Vec<usize>>,
) -> Result<SchDocComponent> {
    let mut designator = String::new();
    let mut children = Vec::new();

    // Process children in order (from both main and additional records)
    for &child_idx in child_indices {
        let child_rec = resolve_record(records, additional_records, child_idx);
        match child_rec {
            SchRecord::Designator(d) => {
                designator = d.text.clone();
                // Designator is extracted to a field, not added to children
            }
            SchRecord::Pin(p) => {
                children.push(ComponentChild::Pin(pin_from_internal(p)));
            }
            SchRecord::Parameter(p) => {
                children.push(ComponentChild::Parameter(parameter_from_internal(p)));
            }
            // Container records (ImplementationList, Implementation, etc.) — skip,
            // they're consumed via build_footprint_maps_schdoc
            SchRecord::ImplementationList(_)
            | SchRecord::Implementation(_)
            | SchRecord::ImplementationMap(_)
            | SchRecord::MapDefiner(_)
            | SchRecord::ParameterList(_)
            | SchRecord::Symbol(_) => {}
            other => match graphic_from_record(other) {
                Some(graphic) => {
                    children.push(ComponentChild::Graphic(graphic));
                }
                None => {
                    return Err(AltiumFormatError::NotImplemented(format!(
                        "record type {:?} as Component child",
                        std::mem::discriminant(other)
                    )));
                }
            },
        }
    }

    // Build footprint maps from the implementation chain
    let footprint_maps =
        build_footprint_maps_schdoc(records, additional_records, child_indices, ownership_map)?;
    for fm in footprint_maps {
        children.push(ComponentChild::FootprintMap(fm));
    }

    Ok(SchDocComponent {
        designator,
        unique_id: comp.unique_id.clone(),
        lib_reference: comp.lib_reference.clone(),
        source_library_name: comp.source_library_name.clone(),
        design_item_id: comp.design_item_id.clone(),
        library_path: comp.library_path.clone(),
        location: comp.location,
        orientation: comp.orientation,
        is_mirrored: comp.is_mirrored,
        description: if comp.component_description.is_empty() {
            None
        } else {
            Some(comp.component_description.clone())
        },
        component_kind: comp.component_kind,
        part_count: comp.part_count,
        current_part_id: comp.current_part_id,
        display_mode_count: comp.display_mode_count,
        display_mode: checked_display_mode(comp.display_mode, comp.display_mode_count)?,
        show_hidden_pins: comp.show_hidden_pins,
        children,
    })
}

/// A component's placed display mode must name one of its display modes:
/// `0 <= mode < max(count, 1)` (all 33,816 components of the SchDoc corpus do).
pub(crate) fn checked_display_mode(mode: i32, count: i32) -> Result<i32> {
    if (0..count.max(1)).contains(&mode) {
        Ok(mode)
    } else {
        Err(crate::AltiumFormatError::InvalidParamValue {
            key: "DisplayMode".to_owned(),
            detail: format!("display mode {mode} outside the component's {count} display modes"),
        })
    }
}

/// Convert a SchSheetSymbol and its children into a SheetSymbol.
fn sheet_symbol_from_internal(
    records: &[SchRecord],
    additional_records: &[SchRecord],
    ss: &crate::sch_records::SchSheetSymbol,
    child_indices: &[usize],
    _ownership_map: &HashMap<usize, Vec<usize>>,
) -> Result<SheetSymbol> {
    let mut sheet_name = ss.sheet_name.clone();
    let mut file_name = ss.file_name.clone();
    let mut children = Vec::new();

    for &child_idx in child_indices {
        let child_rec = resolve_record(records, additional_records, child_idx);
        match child_rec {
            SchRecord::SheetName(sn) => {
                // SheetName child overrides the parent's sheet_name field
                sheet_name = sn.text.clone();
            }
            SchRecord::SheetFileName(sf) => {
                // SheetFileName child overrides the parent's file_name field
                file_name = sf.text.clone();
            }
            SchRecord::SheetEntry(se) => {
                children.push(SheetSymbolChild::Entry(sheet_entry_from_internal(se)));
            }
            SchRecord::Parameter(p) => {
                children.push(SheetSymbolChild::Parameter(parameter_from_internal(p)));
            }
            // HighLevelCode variants as children
            SchRecord::HighLevelCodeName(sn) => {
                sheet_name = sn.text.clone();
            }
            SchRecord::HighLevelCodeFileName(sf) => {
                file_name = sf.text.clone();
            }
            SchRecord::HighLevelCodeEntry(se) => {
                children.push(SheetSymbolChild::Entry(sheet_entry_from_internal(se)));
            }
            SchRecord::ImplementationList(_)
            | SchRecord::Implementation(_)
            | SchRecord::ImplementationMap(_)
            | SchRecord::MapDefiner(_) => {}
            other => {
                return Err(AltiumFormatError::NotImplemented(format!(
                    "record type {:?} as SheetSymbol child",
                    std::mem::discriminant(other)
                )));
            }
        }
    }

    Ok(SheetSymbol {
        unique_id: ss.unique_id.clone(),
        location: ss.location,
        x_size: ss.x_size,
        y_size: ss.y_size,
        color: ss.color,
        area_color: ss.area_color,
        line_width: ss.line_width,
        is_solid: ss.is_solid,
        symbol_type: ss.symbol_type,
        sheet_name,
        file_name,
        children,
    })
}

/// Convert a SchParameterSet and its children into a ParameterSet.
fn parameter_set_from_internal(
    records: &[SchRecord],
    additional_records: &[SchRecord],
    ps: &crate::sch_records::SchParameterSet,
    child_indices: &[usize],
) -> Result<ParameterSet> {
    let mut parameters = Vec::new();

    for &child_idx in child_indices {
        let child_rec = resolve_record(records, additional_records, child_idx);
        if let SchRecord::Parameter(p) = child_rec {
            parameters.push(parameter_from_internal(p));
        }
    }

    Ok(ParameterSet {
        unique_id: ps.unique_id.clone(),
        location: ps.location,
        color: ps.color,
        orientation: ps.orientation,
        name: ps.name.clone(),
        style: ps.style,
        parameters,
    })
}

/// Convert a SchHarnessConnector and its children into a HarnessConnector.
fn harness_connector_from_internal(
    records: &[SchRecord],
    additional_records: &[SchRecord],
    hc: &crate::sch_records::SchHarnessConnector,
    child_indices: &[usize],
) -> Result<HarnessConnector> {
    let mut children = Vec::new();

    for &child_idx in child_indices {
        let child_rec = resolve_record(records, additional_records, child_idx);
        match child_rec {
            SchRecord::HarnessEntry(se) => {
                children.push(HarnessChild::Entry(sheet_entry_from_internal(se)));
            }
            SchRecord::HarnessConnectorType(sn) => {
                children.push(HarnessChild::ConnectorType(sn.text.clone()));
            }
            SchRecord::Parameter(p) => {
                children.push(HarnessChild::Parameter(parameter_from_internal(p)));
            }
            other => {
                return Err(AltiumFormatError::NotImplemented(format!(
                    "record type {:?} as HarnessConnector child",
                    std::mem::discriminant(other)
                )));
            }
        }
    }

    Ok(HarnessConnector {
        unique_id: hc.unique_id.clone(),
        location: hc.location,
        x_size: hc.x_size,
        y_size: hc.y_size,
        color: hc.color,
        area_color: hc.area_color,
        line_width: hc.line_width,
        children,
    })
}

// ── Per-type converters (leaf records) ───────────────────────────────────────

fn wire_from_internal(w: &crate::sch_records::SchWire) -> Wire {
    Wire {
        unique_id: w.unique_id.clone(),
        vertices: w.vertices.clone(),
        color: w.color,
        line_width: w.line_width,
        line_style: w.line_style,
    }
}

fn bus_from_internal(b: &crate::sch_records::SchBus) -> Bus {
    Bus {
        unique_id: b.unique_id.clone(),
        vertices: b.vertices.clone(),
        color: b.color,
        line_width: b.line_width,
    }
}

fn net_label_from_internal(n: &crate::sch_records::SchNetLabel) -> NetLabel {
    NetLabel {
        unique_id: n.unique_id.clone(),
        text: n.text.clone(),
        location: n.location,
        orientation: n.orientation,
        justification: n.justification,
        font_id: n.font_id,
        color: n.color,
        is_mirrored: n.is_mirrored,
    }
}

fn power_object_from_internal(p: &crate::sch_records::SchPowerObject) -> PowerObject {
    PowerObject {
        unique_id: p.unique_id.clone(),
        text: p.text.clone(),
        location: p.location,
        orientation: p.orientation,
        style: p.style,
        show_net_name: p.show_net_name,
        font_id: p.font_id,
        color: p.color,
        is_cross_sheet_connector: p.is_cross_sheet_connector,
    }
}

fn port_from_internal(p: &crate::sch_records::SchPort) -> Port {
    Port {
        unique_id: p.unique_id.clone(),
        name: p.name.clone(),
        location: p.location,
        io_type: p.io_type,
        style: p.style,
        width: p.width,
        height: p.height,
        color: p.color,
        area_color: p.area_color,
        text_color: p.text_color,
        font_id: p.font_id,
        alignment: p.alignment,
        harness_type: p.harness_type.clone(),
        border_width: p.border_width,
        auto_size: p.auto_size,
        port_name_is_hidden: p.port_name_is_hidden,
    }
}

fn junction_from_internal(j: &crate::sch_records::SchJunction) -> Junction {
    Junction {
        unique_id: j.unique_id.clone(),
        location: j.location,
        color: j.color,
    }
}

fn no_connect_from_internal(n: &crate::sch_records::SchNoConnect) -> NoConnect {
    NoConnect {
        unique_id: n.unique_id.clone(),
        location: n.location,
        color: n.color,
        orientation: n.orientation,
        symbol: n.symbol.clone(),
        is_active: n.is_active,
        suppress_all: n.suppress_all,
    }
}

fn bus_entry_from_internal(b: &crate::sch_records::SchBusEntry) -> BusEntry {
    BusEntry {
        unique_id: b.unique_id.clone(),
        location: b.location,
        corner: b.corner,
        color: b.color,
        line_width: b.line_width,
    }
}

fn sheet_entry_from_internal(se: &crate::sch_records::SchSheetEntry) -> SheetEntry {
    SheetEntry {
        unique_id: se.unique_id.clone(),
        name: se.name.clone(),
        io_type: se.io_type,
        side: se.side,
        distance_from_top: se.distance_from_top,
        style: se.style,
        color: se.color,
        area_color: se.area_color,
        text_color: se.text_color,
        text_font_id: se.text_font_id,
    }
}

fn note_from_internal(n: &crate::sch_records::SchNote) -> Note {
    Note {
        unique_id: n.unique_id.clone(),
        location: n.location,
        corner: n.corner,
        text: n.text.clone(),
        author: n.author.clone(),
        font_id: n.font_id,
        color: n.color,
        area_color: n.area_color,
        text_color: n.text_color,
        is_solid: n.is_solid,
        show_border: n.show_border,
        alignment: n.alignment,
        word_wrap: n.word_wrap,
        clip_to_rect: n.clip_to_rect,
        text_margin: n.text_margin,
        collapsed: n.collapsed,
    }
}

fn probe_from_internal(p: &crate::sch_records::SchProbe) -> Probe {
    Probe {
        unique_id: p.unique_id.clone(),
        location: p.location,
        color: p.color,
        orientation: p.orientation,
        name: p.name.clone(),
    }
}

fn compile_mask_from_internal(c: &crate::sch_records::SchCompileMask) -> CompileMask {
    CompileMask {
        unique_id: c.unique_id.clone(),
        location: c.location,
        corner: c.corner,
        color: c.color,
        area_color: c.area_color,
        line_width: c.line_width,
        collapsed: c.collapsed,
    }
}

fn blanket_from_internal(b: &crate::sch_records::SchBlanket) -> Blanket {
    Blanket {
        unique_id: b.unique_id.clone(),
        location: b.location,
        corner: b.corner,
        color: b.color,
        area_color: b.area_color,
        line_style: b.line_style,
        line_width: b.line_width,
        vertices: b.vertices.clone(),
        collapsed: b.collapsed,
    }
}

fn signal_harness_from_internal(sh: &crate::sch_records::SchBus) -> SignalHarness {
    SignalHarness {
        unique_id: sh.unique_id.clone(),
        vertices: sh.vertices.clone(),
        color: sh.color,
        line_width: sh.line_width,
    }
}

#[cfg(test)]
mod tests {
    use super::checked_display_mode;

    #[test]
    fn display_mode_must_be_one_of_the_components_modes() {
        assert_eq!(checked_display_mode(0, 0).unwrap(), 0);
        assert_eq!(checked_display_mode(0, 1).unwrap(), 0);
        assert_eq!(checked_display_mode(2, 3).unwrap(), 2);
        assert!(checked_display_mode(1, 1).is_err());
        assert!(checked_display_mode(3, 3).is_err());
        assert!(checked_display_mode(-1, 2).is_err());
    }
}

#[cfg(all(test, feature = "test-fixtures"))]
mod corpus_tests {
    use crate::sch_records::SchRecord;
    use crate::schdoc::SchDoc;
    use std::path::Path;

    /// Batch 5 item 5: every corpus SchDoc's first error, grouped by cause, and
    /// ASCII SchDocs read like binary ones (an ASCII sheet saved as CFB reopens
    /// with the same records).
    #[test]
    fn corpus_schdoc_open_errors() {
        const OPENED: usize = 1293;
        use std::collections::BTreeMap;
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/schdoc");
        let mut paths: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
            .map(|e| e.expect("dir entry").path())
            .filter(|p| {
                p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("schdoc"))
            })
            .collect();
        paths.sort();
        let mut causes: BTreeMap<String, usize> = BTreeMap::new();
        let mut examples: BTreeMap<String, String> = BTreeMap::new();
        let (mut opened, mut sheets, mut ascii_opened, mut ascii_resaved) = (0, 0, 0, 0);
        for path in &paths {
            let ascii = std::fs::read(path)
                .unwrap()
                .starts_with(b"|HEADER=Protel for Windows - Schematic Capture Ascii");
            let doc = match SchDoc::open(path) {
                Ok(doc) => doc,
                Err(e) => {
                    // The innermost cause, without file-specific values.
                    // The two innermost causes (record kind + reason), digits masked.
                    let msg = format!("{e:#}");
                    let parts: Vec<&str> = msg.rsplitn(3, ": ").take(2).collect();
                    let cause: String = parts
                        .iter()
                        .rev()
                        .copied()
                        .collect::<Vec<_>>()
                        .join(": ")
                        .chars()
                        .map(|c| if c.is_ascii_digit() { '#' } else { c })
                        .take(110)
                        .collect();
                    examples
                        .entry(format!("open: {cause}"))
                        .or_insert_with(|| format!("{}: {msg}", path.display()));
                    *causes.entry(format!("open: {cause}")).or_default() += 1;
                    continue;
                }
            };
            opened += 1;
            ascii_opened += usize::from(ascii);
            match doc.sheet() {
                Ok(_) => sheets += 1,
                Err(e) => {
                    let msg = format!("{e:#}");
                    let cause = msg.rsplit(": ").next().unwrap_or(&msg);
                    let cause: String = cause
                        .chars()
                        .map(|c| if c.is_ascii_digit() { '#' } else { c })
                        .take(90)
                        .collect();
                    *causes.entry(format!("sheet: {cause}")).or_default() += 1;
                }
            }
            if ascii {
                let tmp = tempfile::NamedTempFile::new().unwrap();
                doc.save(tmp.path())
                    .unwrap_or_else(|e| panic!("{}: save: {e:#}", path.display()));
                let again = SchDoc::open(tmp.path())
                    .unwrap_or_else(|e| panic!("{}: reopen: {e:#}", path.display()));
                assert_eq!(
                    format!("{:?}", doc.records),
                    format!("{:?}", again.records),
                    "{}: records changed by ASCII -> CFB save",
                    path.display()
                );
                ascii_resaved += 1;
            }
        }
        for (cause, n) in &causes {
            eprintln!("{n:5}  {cause}");
            if let Some(example) = examples.get(cause) {
                eprintln!("       e.g. {example}");
            }
        }
        eprintln!(
            "files {} opened {opened} sheets {sheets} ascii opened {ascii_opened} resaved {ascii_resaved}",
            paths.len()
        );
        // dd200d4: 1,202 opened, 1,130 sheets. Every opened SchDoc now converts.
        assert_eq!(paths.len(), 1617);
        assert_eq!((opened, sheets), (OPENED, OPENED));
        assert_eq!(ascii_resaved, ascii_opened);
    }

    /// R9: every corpus component's placed display mode is in range and
    /// reaches the API (137 components on readable sheets are placed in an
    /// alternate mode), and survives `update_sheet` → `sheet()` together with
    /// every child's display mode (the writer stored 0 for both).
    #[test]
    fn corpus_r9_display_mode() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/schdoc");
        let mut paths: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
            .map(|e| e.expect("dir entry").path())
            .filter(|p| {
                p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("schdoc"))
            })
            .collect();
        paths.sort();
        let mut alternate_children = 0;
        let (mut docs, mut sheets, mut alternate, mut written, mut not_written) = (0, 0, 0, 0, 0);
        for path in &paths {
            let Ok(mut doc) = SchDoc::open(path) else {
                continue;
            };
            docs += 1;
            let stored: Vec<i32> = doc
                .records
                .iter()
                .filter_map(|r| match r {
                    SchRecord::Component(c) => Some(c.display_mode),
                    _ => None,
                })
                .collect();
            let Ok(sheet) = doc.sheet() else {
                continue;
            };
            sheets += 1;
            let modes = |s: &crate::api::SchDocSheet| -> Vec<i32> {
                s.components().iter().map(|c| c.display_mode).collect()
            };
            let child_modes = |s: &crate::api::SchDocSheet| -> Vec<(i32, i32)> {
                s.components()
                    .iter()
                    .flat_map(|c| c.children.iter())
                    .filter_map(|child| match child {
                        crate::api::ComponentChild::Graphic(g) => {
                            Some((g.owner_part_id(), g.owner_part_display_mode()))
                        }
                        crate::api::ComponentChild::Pin(p) => {
                            Some((p.owner_part_id, i32::from(p.owner_part_display_mode)))
                        }
                        _ => None,
                    })
                    .collect()
            };
            let read = modes(&sheet);
            let mut sorted_read = read.clone();
            let mut sorted_stored = stored.clone();
            sorted_read.sort_unstable();
            sorted_stored.sort_unstable();
            assert_eq!(sorted_read, sorted_stored, "{}", path.display());
            alternate += read.iter().filter(|&&m| m != 0).count();
            if read.iter().any(|&m| m != 0) {
                // update_sheet fails on some corpus sheets for reasons unrelated
                // to display modes; those are counted, not checked.
                if doc.update_sheet(&sheet).is_ok() {
                    let after = doc
                        .sheet()
                        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                    assert_eq!(
                        modes(&after),
                        read,
                        "{}: display modes changed",
                        path.display()
                    );
                    assert_eq!(
                        child_modes(&after),
                        child_modes(&sheet),
                        "{}: child display modes changed",
                        path.display()
                    );
                    alternate_children +=
                        child_modes(&sheet).iter().filter(|(_, m)| *m != 0).count();
                    written += 1;
                } else {
                    not_written += 1;
                }
            }
        }
        eprintln!(
            "corpus_r9_display_mode: {docs} SchDocs open, {sheets} sheets read, {alternate} \
             components in an alternate mode, {written} sheets with one written and re-read, \
             {not_written} where update_sheet failed"
        );
        eprintln!("corpus_r9_display_mode: {alternate_children} alternate-mode children");
        assert!(alternate_children > 0);
        // 137 at dd200d4; 157 once ASCII, IgnoreOnLoad and harness owners read (batch 5).
        assert_eq!(alternate, 157);
        assert!(written > 0);
    }

    /// R10: UniqueIDs are the same on every load (missing ones used to be
    /// filled with random ids); every SchDoc of the corpus is opened twice.
    #[test]
    fn corpus_r10_unique_ids_reproducible() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/schdoc");
        let mut paths: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
            .map(|e| e.expect("dir entry").path())
            .filter(|p| {
                p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("schdoc"))
            })
            .collect();
        paths.sort();
        let ids = |doc: &SchDoc| -> Vec<String> {
            doc.records
                .iter()
                .chain(doc.additional_records.iter())
                .filter_map(|r| r.unique_id().map(str::to_owned))
                .collect()
        };
        let (mut docs, mut records_with_ids) = (0, 0);
        for path in &paths {
            let Ok(first) = SchDoc::open(path) else {
                continue;
            };
            let second = SchDoc::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let (a, b) = (ids(&first), ids(&second));
            assert_eq!(a, b, "{}: ids differ between loads", path.display());
            assert!(a.iter().all(|id| !id.is_empty()), "{}", path.display());
            docs += 1;
            records_with_ids += a.len();
        }
        eprintln!(
            "corpus_r10_unique_ids_reproducible: {docs} SchDocs opened twice, {records_with_ids} \
             record ids identical"
        );
        // 1,202 at dd200d4; batch 5 reads 91 more (ASCII, IgnoreOnLoad).
        assert_eq!(docs, 1_293);
    }
}

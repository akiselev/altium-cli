//! Write path: convert public SchDoc API types → internal flat records.
//!
//! Flattens the nested `SchDocSheet` tree back into `Vec<SchRecord>` with
//! correct `OWNERINDEX` values. Records are emitted in depth-first order
//! matching Altium's canonical ordering.
//!
//! **UniqueId-based field preservation**: When `existing_records` is provided,
//! a `HashMap<unique_id, &SchRecord>` index is built. For every emitted record,
//! we look up the old record by its `unique_id` and copy format-internal fields
//! (`index_in_sheet`, `selection_memory`, `style_id`, `graphically_locked`,
//! `union_index`) from the old base. This ensures roundtrips preserve fields
//! that aren't exposed in the public API.

use std::collections::HashMap;

use crate::api::sch_common::{
    default_base, pin_to_internal, parameter_to_internal, graphic_to_record,
};
use crate::api::schdoc_types::*;
use crate::sch_records::{
    SchRecord, SchPrimitiveBase,
    SchWire as InternalWire, SchBus as InternalBus,
    SchNetLabel as InternalNetLabel, SchPowerObject as InternalPowerObject,
    SchPort as InternalPort, SchJunction as InternalJunction,
    SchNoConnect as InternalNoConnect, SchBusEntry as InternalBusEntry,
    SchSheetSymbol as InternalSheetSymbol, SchSheetEntry as InternalSheetEntry,
    SchSheetName as InternalSheetName, SchSheetFileName as InternalSheetFileName,
    SchParameterSet as InternalParameterSet, SchNote as InternalNote,
    SchProbe as InternalProbe, SchCompileMask as InternalCompileMask,
    SchBlanket as InternalBlanket, SchHarnessConnector as InternalHarnessConnector,
    SchComponent, SchDesignator, SchTemplate,
    SchImplementationList, SchImplementation, SchImplementationMap, SchMapDefiner,
};
use crate::util::generate_unique_id;
use crate::Result;

use altium_format_types::color::Color;
use altium_format_types::common::RotationBy90;
use altium_format_types::coord::CoordPoint;
use altium_format_types::sch::{
    LeftRightSide, ParameterReadOnlyState, ParameterType,
    PenWidth, TextHorzAnchor, TextJustification, TextVertAnchor,
};

// ── UniqueId index ───────────────────────────────────────────────────────────

/// Build a lookup index from unique_id → &SchRecord for field preservation.
///
/// Only records with non-empty unique_ids are indexed. When multiple records
/// share the same unique_id (shouldn't happen, but defensively handle it),
/// the first one wins.
fn build_unique_id_index<'a>(records: &'a [SchRecord]) -> HashMap<&'a str, &'a SchRecord> {
    let mut map = HashMap::new();
    for rec in records {
        if let Some(uid) = rec.unique_id() {
            if !uid.is_empty() {
                map.entry(uid).or_insert(rec);
            }
        }
    }
    map
}

/// Copy format-internal base fields from an existing record to a new record.
///
/// Preserves: `index_in_sheet`, `selection_memory`, `style_id`,
/// `graphically_locked`, `union_index`. Does NOT preserve `owner_index`
/// (recomputed from tree structure) or `owner_part_id`/`owner_part_display_mode`
/// (only relevant within components and set explicitly).
fn preserve_base_fields(new_base: &mut SchPrimitiveBase, old_base: &SchPrimitiveBase) {
    new_base.index_in_sheet = old_base.index_in_sheet;
    new_base.selection_memory = old_base.selection_memory;
    new_base.style_id = old_base.style_id;
    new_base.graphically_locked = old_base.graphically_locked;
    new_base.union_index = old_base.union_index;
    new_base.is_not_accessible = old_base.is_not_accessible;
}

/// Look up an existing record by unique_id and preserve its base fields.
fn try_preserve_base(rec: &mut SchRecord, uid: &str, index: &HashMap<&str, &SchRecord>) {
    if uid.is_empty() {
        return;
    }
    if let Some(&old_rec) = index.get(uid) {
        if let (Some(new_base), Some(old_base)) = (rec.base_mut(), old_rec.base()) {
            preserve_base_fields(new_base, old_base);
        }
    }
}

// ── Top-level conversion ─────────────────────────────────────────────────────

/// Convert a `SchDocSheet` into flat records suitable for serialization.
///
/// When `existing_records` is provided, format-internal fields are preserved
/// from matching existing records (by unique_id).
///
/// Returns `(records, additional_records)`.
pub(crate) fn sheet_to_internal(
    sheet: &SchDocSheet,
    existing_records: Option<&[SchRecord]>,
) -> Result<(Vec<SchRecord>, Vec<SchRecord>)> {
    let mut records = Vec::new();

    // Build the unique_id index from existing records for field preservation
    let uid_index = existing_records
        .map(build_unique_id_index)
        .unwrap_or_default();

    // Record 0: Sheet (RECORD=31) — will be filled by the caller (SchDoc::update_sheet)
    // since it needs the existing Sheet record's full internal state.
    // We build records starting from index 1 (after Sheet at 0).
    // The caller is responsible for keeping record 0 (Sheet) intact.

    // Template at index 1
    let template_idx = 1;
    records.push(SchRecord::Template(SchTemplate {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..default_base()
        },
        file_name: sheet.template.file_name.clone(),
    }));

    // Template children
    for graphic in &sheet.template.children {
        let mut rec = graphic_to_record(graphic);
        set_owner_index(&mut rec, template_idx as i32);
        // Preserve base fields for template graphics too
        if let Some(uid) = rec.unique_id() {
            let uid_owned = uid.to_owned();
            try_preserve_base(&mut rec, &uid_owned, &uid_index);
        }
        records.push(rec);
    }

    // Sheet-level objects
    for obj in &sheet.objects {
        emit_sheet_object(&mut records, obj, existing_records, &uid_index)?;
    }

    Ok((records, Vec::new()))
}

/// Emit a SheetObject and its children into the records vec.
///
/// All sheet-level objects have owner_index = 0.
fn emit_sheet_object(
    records: &mut Vec<SchRecord>,
    obj: &SheetObject,
    existing_records: Option<&[SchRecord]>,
    uid_index: &HashMap<&str, &SchRecord>,
) -> Result<()> {
    match obj {
        SheetObject::Component(comp) => {
            emit_component(records, comp, existing_records, uid_index)?;
        }
        SheetObject::Wire(w) => {
            let mut rec = wire_to_internal(w);
            try_preserve_base(&mut rec, &w.unique_id, uid_index);
            preserve_wire_fields(&mut rec, &w.unique_id, uid_index);
            records.push(rec);
        }
        SheetObject::Bus(b) => {
            let mut rec = bus_to_internal(b);
            try_preserve_base(&mut rec, &b.unique_id, uid_index);
            preserve_bus_fields(&mut rec, &b.unique_id, uid_index);
            records.push(rec);
        }
        SheetObject::NetLabel(n) => {
            let mut rec = net_label_to_internal(n);
            try_preserve_base(&mut rec, &n.unique_id, uid_index);
            records.push(rec);
        }
        SheetObject::PowerObject(p) => {
            let mut rec = power_object_to_internal(p);
            try_preserve_base(&mut rec, &p.unique_id, uid_index);
            preserve_power_object_fields(&mut rec, &p.unique_id, uid_index);
            records.push(rec);
        }
        SheetObject::Port(p) => {
            let mut rec = port_to_internal(p);
            try_preserve_base(&mut rec, &p.unique_id, uid_index);
            preserve_port_fields(&mut rec, &p.unique_id, uid_index);
            records.push(rec);
        }
        SheetObject::Junction(j) => {
            let mut rec = junction_to_internal(j);
            try_preserve_base(&mut rec, &j.unique_id, uid_index);
            preserve_junction_fields(&mut rec, &j.unique_id, uid_index);
            records.push(rec);
        }
        SheetObject::NoConnect(n) => {
            let mut rec = no_connect_to_internal(n);
            try_preserve_base(&mut rec, &n.unique_id, uid_index);
            preserve_no_connect_fields(&mut rec, &n.unique_id, uid_index);
            records.push(rec);
        }
        SheetObject::BusEntry(b) => {
            let mut rec = bus_entry_to_internal(b);
            try_preserve_base(&mut rec, &b.unique_id, uid_index);
            records.push(rec);
        }
        SheetObject::SheetSymbol(ss) => {
            emit_sheet_symbol(records, ss, uid_index)?;
        }
        SheetObject::ParameterSet(ps) => {
            emit_parameter_set(records, ps, uid_index)?;
        }
        SheetObject::Note(n) => {
            let mut rec = note_to_internal(n);
            try_preserve_base(&mut rec, &n.unique_id, uid_index);
            preserve_note_fields(&mut rec, &n.unique_id, uid_index);
            records.push(rec);
        }
        SheetObject::Probe(p) => {
            let mut rec = probe_to_internal(p);
            try_preserve_base(&mut rec, &p.unique_id, uid_index);
            records.push(rec);
        }
        SheetObject::CompileMask(c) => {
            let mut rec = compile_mask_to_internal(c);
            try_preserve_base(&mut rec, &c.unique_id, uid_index);
            records.push(rec);
        }
        SheetObject::Blanket(b) => {
            let mut rec = blanket_to_internal(b);
            try_preserve_base(&mut rec, &b.unique_id, uid_index);
            records.push(rec);
        }
        SheetObject::Graphic(g) => {
            let mut rec = graphic_to_record(g);
            set_owner_index(&mut rec, 0);
            if let Some(uid) = rec.unique_id() {
                let uid_owned = uid.to_owned();
                try_preserve_base(&mut rec, &uid_owned, uid_index);
            }
            records.push(rec);
        }
        SheetObject::Parameter(p) => {
            let mut rec = SchRecord::Parameter(parameter_to_internal(p));
            set_owner_index(&mut rec, 0);
            try_preserve_base(&mut rec, &p.unique_id, uid_index);
            records.push(rec);
        }
        SheetObject::HarnessConnector(hc) => {
            emit_harness_connector(records, hc, uid_index)?;
        }
        SheetObject::SignalHarness(sh) => {
            let mut rec = signal_harness_to_internal(sh);
            try_preserve_base(&mut rec, &sh.unique_id, uid_index);
            preserve_signal_harness_fields(&mut rec, &sh.unique_id, uid_index);
            records.push(rec);
        }
    }
    Ok(())
}

// ── Type-specific field preservation ─────────────────────────────────────────
//
// These preserve format-internal fields that are NOT in SchPrimitiveBase but
// ARE on the specific record types. Without these, fields like `underline_color`,
// `locked`, `symbol_type`, etc. would be reset to defaults on every save.

fn preserve_wire_fields(rec: &mut SchRecord, uid: &str, index: &HashMap<&str, &SchRecord>) {
    if let (SchRecord::Wire(new), Some(SchRecord::Wire(old))) =
        (rec, index.get(uid).copied())
    {
        new.underline_color = old.underline_color;
        new.assigned_interface = old.assigned_interface.clone();
        new.assigned_interface_signal = old.assigned_interface_signal.clone();
    }
}

fn preserve_bus_fields(rec: &mut SchRecord, uid: &str, index: &HashMap<&str, &SchRecord>) {
    if let (SchRecord::Bus(new), Some(SchRecord::Bus(old))) =
        (rec, index.get(uid).copied())
    {
        new.underline_color = old.underline_color;
        new.assigned_interface = old.assigned_interface.clone();
        new.assigned_interface_signal = old.assigned_interface_signal.clone();
    }
}

fn preserve_power_object_fields(rec: &mut SchRecord, uid: &str, index: &HashMap<&str, &SchRecord>) {
    if let (SchRecord::PowerObject(new), Some(SchRecord::PowerObject(old))) =
        (rec, index.get(uid).copied())
    {
        new.symbol_type = old.symbol_type;
    }
}

fn preserve_port_fields(rec: &mut SchRecord, uid: &str, index: &HashMap<&str, &SchRecord>) {
    if let (SchRecord::Port(new), Some(SchRecord::Port(old))) =
        (rec, index.get(uid).copied())
    {
        new.object_definition_id = old.object_definition_id.clone();
    }
}

fn preserve_junction_fields(rec: &mut SchRecord, uid: &str, index: &HashMap<&str, &SchRecord>) {
    if let (SchRecord::Junction(new), Some(SchRecord::Junction(old))) =
        (rec, index.get(uid).copied())
    {
        new.size = old.size;
        new.locked = old.locked;
    }
}

fn preserve_no_connect_fields(rec: &mut SchRecord, uid: &str, index: &HashMap<&str, &SchRecord>) {
    if let (SchRecord::NoConnect(new), Some(SchRecord::NoConnect(old))) =
        (rec, index.get(uid).copied())
    {
        new.error_kind_set_to_suppress = old.error_kind_set_to_suppress.clone();
        new.connection_pairs_to_suppress = old.connection_pairs_to_suppress.clone();
    }
}

fn preserve_note_fields(rec: &mut SchRecord, uid: &str, index: &HashMap<&str, &SchRecord>) {
    if let (SchRecord::Note(new), Some(SchRecord::Note(old))) =
        (rec, index.get(uid).copied())
    {
        new.line_width = old.line_width;
    }
}

fn preserve_signal_harness_fields(rec: &mut SchRecord, uid: &str, index: &HashMap<&str, &SchRecord>) {
    if let (SchRecord::SignalHarness(new), Some(SchRecord::SignalHarness(old))) =
        (rec, index.get(uid).copied())
    {
        new.underline_color = old.underline_color;
        new.assigned_interface = old.assigned_interface.clone();
        new.assigned_interface_signal = old.assigned_interface_signal.clone();
    }
}

// ── Container emitters ───────────────────────────────────────────────────────

/// Emit a component and all its children.
fn emit_component(
    records: &mut Vec<SchRecord>,
    comp: &SchDocComponent,
    existing_records: Option<&[SchRecord]>,
    uid_index: &HashMap<&str, &SchRecord>,
) -> Result<()> {
    // The component record index in the FINAL output is records.len() + 1
    // (since record 0 is the Sheet which the caller prepends).
    let comp_final_idx = (records.len() + 1) as i32;

    // Find existing component for field preservation (component-specific fields
    // beyond base fields are handled here since Component doesn't have a base)
    let existing_comp = existing_records.and_then(|recs| {
        recs.iter().find(|r| {
            if let SchRecord::Component(c) = r {
                c.unique_id == comp.unique_id || c.lib_reference == comp.lib_reference
            } else {
                false
            }
        })
    });

    let mut sch_comp = SchComponent {
        lib_reference: comp.lib_reference.clone(),
        component_description: comp.description.clone().unwrap_or_default(),
        part_count: comp.part_count,
        display_mode_count: comp.display_mode_count,
        owner_index: 0, // Sheet-owned
        is_not_accessible: false,
        index_in_sheet: 0,
        owner_part_id: 0,
        owner_part_display_mode: 0,
        graphically_locked: false,
        union_index: 0,
        location: comp.location,
        display_mode: 0,
        is_mirrored: comp.is_mirrored,
        orientation: comp.orientation,
        current_part_id: comp.current_part_id,
        show_hidden_fields: false,
        show_hidden_pins: comp.show_hidden_pins,
        library_path: comp.library_path.clone(),
        source_library_name: comp.source_library_name.clone(),
        database_table_name: String::new(),
        sheet_part_file_name: String::new(),
        target_file_name: String::new(),
        unique_id: comp.unique_id.clone(),
        area_color: Color::new(11_599_871),
        color: Color::new(12_800_000),
        pin_color: Color::new(8_388_608),
        override_colors: false,
        display_field_names: false,
        designator_locked: false,
        part_id_locked: false,
        pins_moveable: false,
        alias_list: String::new(),
        not_use_library_name: false,
        not_use_db_table_name: false,
        design_item_id: comp.design_item_id.clone(),
        vault_guid: String::new(),
        item_guid: String::new(),
        revision_guid: String::new(),
        symbol_vault_guid: String::new(),
        symbol_item_guid: String::new(),
        symbol_revision_guid: String::new(),
        generic_component_template_guid: String::new(),
        has_only_current_part_info: false,
        all_pin_count: comp.children.iter().filter(|c| matches!(c, ComponentChild::Pin(_))).count() as i32,
        key_component_unique_id: String::new(),
        component_kind: comp.component_kind,
        component_kind_version2: comp.component_kind,
        component_kind_version3: comp.component_kind,
        custom_display_mode_names: Vec::new(),
    };

    // Preserve format-internal fields from existing component
    if let Some(SchRecord::Component(existing)) = existing_comp {
        // Base-equivalent fields (Component has its own rather than SchPrimitiveBase)
        sch_comp.index_in_sheet = existing.index_in_sheet;
        sch_comp.graphically_locked = existing.graphically_locked;
        sch_comp.union_index = existing.union_index;
        sch_comp.is_not_accessible = existing.is_not_accessible;

        // Component-specific format-internal fields
        sch_comp.area_color = existing.area_color;
        sch_comp.color = existing.color;
        sch_comp.pin_color = existing.pin_color;
        sch_comp.override_colors = existing.override_colors;
        sch_comp.display_field_names = existing.display_field_names;
        sch_comp.designator_locked = existing.designator_locked;
        sch_comp.part_id_locked = existing.part_id_locked;
        sch_comp.pins_moveable = existing.pins_moveable;
        sch_comp.database_table_name = existing.database_table_name.clone();
        sch_comp.vault_guid = existing.vault_guid.clone();
        sch_comp.item_guid = existing.item_guid.clone();
        sch_comp.revision_guid = existing.revision_guid.clone();
        sch_comp.symbol_vault_guid = existing.symbol_vault_guid.clone();
        sch_comp.symbol_item_guid = existing.symbol_item_guid.clone();
        sch_comp.symbol_revision_guid = existing.symbol_revision_guid.clone();
        sch_comp.generic_component_template_guid = existing.generic_component_template_guid.clone();
        sch_comp.has_only_current_part_info = existing.has_only_current_part_info;
        sch_comp.key_component_unique_id = existing.key_component_unique_id.clone();
        sch_comp.custom_display_mode_names = existing.custom_display_mode_names.clone();
    }

    records.push(SchRecord::Component(sch_comp));

    // Emit Designator (RECORD=34) re-synthesized from comp.designator
    if !comp.designator.is_empty() {
        records.push(SchRecord::Designator(SchDesignator {
            base: SchPrimitiveBase {
                owner_index: comp_final_idx,
                ..default_base()
            },
            location: CoordPoint::zero(),
            color: Color::new(0x00000080),
            font_id: 1,
            text: comp.designator.clone(),
            name: "Designator".to_owned(),
            is_hidden: false,
            orientation: RotationBy90::Rotate0,
            justification: TextJustification::BottomLeft,
            is_mirrored: false,
            unique_id: generate_unique_id(),
            show_name: false,
            read_only_state: ParameterReadOnlyState::Name,
            not_auto_position: false,
            override_not_auto_position: false,
            not_allow_library_synchronize: false,
            not_allow_database_synchronize: false,
            description: String::new(),
            param_type: ParameterType::String,
            text_horz_anchor: TextHorzAnchor::None,
            text_vert_anchor: TextVertAnchor::None,
            is_image_parameter: false,
        }));
    }

    // Emit children in order (Parameters, Pins, Graphics)
    let mut footprint_maps = Vec::new();
    for child in &comp.children {
        match child {
            ComponentChild::Pin(pin) => {
                let mut internal = pin_to_internal(pin);
                internal.owner_index = comp_final_idx;
                records.push(SchRecord::Pin(internal));
            }
            ComponentChild::Parameter(param) => {
                let mut internal = parameter_to_internal(param);
                internal.base.owner_index = comp_final_idx;
                let mut rec = SchRecord::Parameter(internal);
                try_preserve_base(&mut rec, &param.unique_id, uid_index);
                records.push(rec);
            }
            ComponentChild::Graphic(graphic) => {
                let mut rec = graphic_to_record(graphic);
                set_owner_index(&mut rec, comp_final_idx);
                if let Some(uid) = rec.unique_id() {
                    let uid_owned = uid.to_owned();
                    try_preserve_base(&mut rec, &uid_owned, uid_index);
                }
                records.push(rec);
            }
            ComponentChild::FootprintMap(fm) => {
                footprint_maps.push(fm);
            }
        }
    }

    // Emit implementation chain for footprint maps
    if !footprint_maps.is_empty() {
        let impl_list_idx = (records.len() + 1) as i32;
        records.push(SchRecord::ImplementationList(SchImplementationList {
            base: SchPrimitiveBase {
                owner_index: comp_final_idx,
                ..default_base()
            },
        }));

        for fp in &footprint_maps {
            let impl_idx = (records.len() + 1) as i32;
            records.push(SchRecord::Implementation(SchImplementation {
                base: SchPrimitiveBase {
                    owner_index: impl_list_idx,
                    ..default_base()
                },
                description: fp.description.clone(),
                use_component_library: true,
                model_name: fp.model_name.clone(),
                model_type: "PCBLIB".to_owned(),
                model_vault_guid: String::new(),
                model_item_guid: String::new(),
                model_revision_guid: String::new(),
                datafile_links: Vec::new(),
                is_current: fp.is_current,
                datalinks_locked: false,
                database_datalinks_locked: false,
                integrated_model: false,
                database_model: false,
                unique_id: generate_unique_id(),
                model_location: String::new(),
            }));

            let map_idx = (records.len() + 1) as i32;
            records.push(SchRecord::ImplementationMap(SchImplementationMap {
                base: SchPrimitiveBase {
                    owner_index: impl_idx,
                    ..default_base()
                },
                unique_id: generate_unique_id(),
            }));

            for ppm in &fp.pin_pad_maps {
                records.push(SchRecord::MapDefiner(SchMapDefiner {
                    base: SchPrimitiveBase {
                        owner_index: map_idx,
                        ..default_base()
                    },
                    des_intf: ppm.pin.clone(),
                    des_imps: if ppm.pad.is_empty() {
                        Vec::new()
                    } else {
                        vec![ppm.pad.clone()]
                    },
                }));
            }
        }
    }

    Ok(())
}

/// Emit a SheetSymbol and its children.
fn emit_sheet_symbol(
    records: &mut Vec<SchRecord>,
    ss: &SheetSymbol,
    uid_index: &HashMap<&str, &SchRecord>,
) -> Result<()> {
    let sym_final_idx = (records.len() + 1) as i32;

    let mut sym_rec = SchRecord::SheetSymbol(InternalSheetSymbol {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..default_base()
        },
        location: ss.location,
        x_size: ss.x_size,
        y_size: ss.y_size,
        line_width: ss.line_width,
        color: ss.color,
        area_color: ss.area_color,
        is_solid: ss.is_solid,
        unique_id: ss.unique_id.clone(),
        symbol_type: ss.symbol_type,
        sheet_name: ss.sheet_name.clone(),
        file_name: ss.file_name.clone(),
        show_hidden_fields: false,
        design_item_id: String::new(),
        source_library_name: String::new(),
        vault_guid: String::new(),
        item_guid: String::new(),
        revision_guid: String::new(),
        revision_name: String::new(),
    });

    // Preserve base fields and SheetSymbol-specific format-internal fields
    try_preserve_base(&mut sym_rec, &ss.unique_id, uid_index);
    if let (SchRecord::SheetSymbol(new), Some(SchRecord::SheetSymbol(old))) =
        (&mut sym_rec, uid_index.get(ss.unique_id.as_str()).copied())
    {
        new.show_hidden_fields = old.show_hidden_fields;
        new.design_item_id = old.design_item_id.clone();
        new.source_library_name = old.source_library_name.clone();
        new.vault_guid = old.vault_guid.clone();
        new.item_guid = old.item_guid.clone();
        new.revision_guid = old.revision_guid.clone();
        new.revision_name = old.revision_name.clone();
    }

    records.push(sym_rec);

    // Emit SheetName (RECORD=32)
    records.push(SchRecord::SheetName(InternalSheetName {
        base: SchPrimitiveBase {
            owner_index: sym_final_idx,
            ..default_base()
        },
        location: CoordPoint::zero(),
        orientation: RotationBy90::Rotate0,
        justification: TextJustification::BottomLeft,
        color: Color::BLACK,
        font_id: 1,
        is_hidden: false,
        text: ss.sheet_name.clone(),
        is_mirrored: false,
        not_auto_position: false,
        text_horz_anchor: TextHorzAnchor::None,
        text_vert_anchor: TextVertAnchor::None,
        unique_id: generate_unique_id(),
        selection: false,
    }));

    // Emit SheetFileName (RECORD=33)
    records.push(SchRecord::SheetFileName(InternalSheetFileName {
        base: SchPrimitiveBase {
            owner_index: sym_final_idx,
            ..default_base()
        },
        location: CoordPoint::zero(),
        orientation: RotationBy90::Rotate0,
        justification: TextJustification::BottomLeft,
        color: Color::BLACK,
        font_id: 1,
        is_hidden: false,
        text: ss.file_name.clone(),
        is_mirrored: false,
        not_auto_position: false,
        text_horz_anchor: TextHorzAnchor::None,
        text_vert_anchor: TextVertAnchor::None,
        unique_id: generate_unique_id(),
        selection: false,
    }));

    // Emit children (entries and parameters)
    for child in &ss.children {
        match child {
            SheetSymbolChild::Entry(e) => {
                let mut rec = SchRecord::SheetEntry(
                    sheet_entry_to_internal(e, sym_final_idx),
                );
                try_preserve_base(&mut rec, &e.unique_id, uid_index);
                records.push(rec);
            }
            SheetSymbolChild::Parameter(p) => {
                let mut internal = parameter_to_internal(p);
                internal.base.owner_index = sym_final_idx;
                let mut rec = SchRecord::Parameter(internal);
                try_preserve_base(&mut rec, &p.unique_id, uid_index);
                records.push(rec);
            }
        }
    }

    Ok(())
}

/// Emit a ParameterSet and its child parameters.
fn emit_parameter_set(
    records: &mut Vec<SchRecord>,
    ps: &ParameterSet,
    uid_index: &HashMap<&str, &SchRecord>,
) -> Result<()> {
    let ps_final_idx = (records.len() + 1) as i32;

    let mut ps_rec = SchRecord::ParameterSet(InternalParameterSet {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..default_base()
        },
        location: ps.location,
        color: ps.color,
        orientation: ps.orientation,
        name: ps.name.clone(),
        style: ps.style,
        unique_id: ps.unique_id.clone(),
    });
    try_preserve_base(&mut ps_rec, &ps.unique_id, uid_index);
    records.push(ps_rec);

    for param in &ps.parameters {
        let mut internal = parameter_to_internal(param);
        internal.base.owner_index = ps_final_idx;
        let mut rec = SchRecord::Parameter(internal);
        try_preserve_base(&mut rec, &param.unique_id, uid_index);
        records.push(rec);
    }

    Ok(())
}

/// Emit a HarnessConnector and its children.
fn emit_harness_connector(
    records: &mut Vec<SchRecord>,
    hc: &HarnessConnector,
    uid_index: &HashMap<&str, &SchRecord>,
) -> Result<()> {
    let hc_final_idx = (records.len() + 1) as i32;

    let mut hc_rec = SchRecord::HarnessConnector(InternalHarnessConnector {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..default_base()
        },
        location: hc.location,
        x_size: hc.x_size,
        y_size: hc.y_size,
        line_width: hc.line_width,
        color: hc.color,
        area_color: hc.area_color,
        primary_connection_position: 1_000_000,
        harness_connector_side: LeftRightSide::Left,
        unique_id: hc.unique_id.clone(),
    });

    // Preserve base fields and HarnessConnector-specific format-internal fields
    try_preserve_base(&mut hc_rec, &hc.unique_id, uid_index);
    if let (SchRecord::HarnessConnector(new), Some(SchRecord::HarnessConnector(old))) =
        (&mut hc_rec, uid_index.get(hc.unique_id.as_str()).copied())
    {
        new.primary_connection_position = old.primary_connection_position;
        new.harness_connector_side = old.harness_connector_side;
    }

    records.push(hc_rec);

    for child in &hc.children {
        match child {
            HarnessChild::Entry(e) => {
                let mut rec = SchRecord::HarnessEntry(
                    sheet_entry_to_internal(e, hc_final_idx),
                );
                try_preserve_base(&mut rec, &e.unique_id, uid_index);
                records.push(rec);
            }
            HarnessChild::ConnectorType(name) => {
                records.push(SchRecord::HarnessConnectorType(InternalSheetName {
                    base: SchPrimitiveBase {
                        owner_index: hc_final_idx,
                        ..default_base()
                    },
                    location: CoordPoint::zero(),
                    orientation: RotationBy90::Rotate0,
                    justification: TextJustification::BottomLeft,
                    color: Color::BLACK,
                    font_id: 1,
                    is_hidden: false,
                    text: name.clone(),
                    is_mirrored: false,
                    not_auto_position: false,
                    text_horz_anchor: TextHorzAnchor::None,
                    text_vert_anchor: TextVertAnchor::None,
                    unique_id: generate_unique_id(),
                    selection: false,
                }));
            }
            HarnessChild::Parameter(p) => {
                let mut internal = parameter_to_internal(p);
                internal.base.owner_index = hc_final_idx;
                let mut rec = SchRecord::Parameter(internal);
                try_preserve_base(&mut rec, &p.unique_id, uid_index);
                records.push(rec);
            }
        }
    }

    Ok(())
}

// ── Per-type write converters ────────────────────────────────────────────────

fn wire_to_internal(w: &Wire) -> SchRecord {
    SchRecord::Wire(InternalWire {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..default_base()
        },
        color: w.color,
        line_width: w.line_width,
        line_style: w.line_style,
        vertices: w.vertices.clone(),
        unique_id: if w.unique_id.is_empty() { generate_unique_id() } else { w.unique_id.clone() },
        underline_color: Color::BLACK,
        assigned_interface: String::new(),
        assigned_interface_signal: String::new(),
    })
}

fn bus_to_internal(b: &Bus) -> SchRecord {
    SchRecord::Bus(InternalBus {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..default_base()
        },
        color: b.color,
        line_width: b.line_width,
        vertices: b.vertices.clone(),
        unique_id: if b.unique_id.is_empty() { generate_unique_id() } else { b.unique_id.clone() },
        underline_color: Color::BLACK,
        assigned_interface: String::new(),
        assigned_interface_signal: String::new(),
    })
}

fn net_label_to_internal(n: &NetLabel) -> SchRecord {
    SchRecord::NetLabel(InternalNetLabel {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..default_base()
        },
        location: n.location,
        orientation: n.orientation,
        justification: n.justification,
        color: n.color,
        font_id: n.font_id,
        text: n.text.clone(),
        is_mirrored: n.is_mirrored,
        unique_id: if n.unique_id.is_empty() { generate_unique_id() } else { n.unique_id.clone() },
    })
}

fn power_object_to_internal(p: &PowerObject) -> SchRecord {
    SchRecord::PowerObject(InternalPowerObject {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..default_base()
        },
        location: p.location,
        color: p.color,
        text: p.text.clone(),
        symbol_type: 0,
        style: p.style,
        show_net_name: p.show_net_name,
        orientation: p.orientation,
        font_id: p.font_id,
        is_cross_sheet_connector: p.is_cross_sheet_connector,
        unique_id: if p.unique_id.is_empty() { generate_unique_id() } else { p.unique_id.clone() },
    })
}

fn port_to_internal(p: &Port) -> SchRecord {
    SchRecord::Port(InternalPort {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..default_base()
        },
        location: p.location,
        color: p.color,
        area_color: p.area_color,
        name: p.name.clone(),
        io_type: p.io_type,
        style: p.style,
        width: p.width,
        height: p.height,
        text_color: p.text_color,
        font_id: p.font_id,
        alignment: p.alignment,
        unique_id: if p.unique_id.is_empty() { generate_unique_id() } else { p.unique_id.clone() },
        harness_type: p.harness_type.clone(),
        border_width: p.border_width,
        auto_size: p.auto_size,
        port_name_is_hidden: p.port_name_is_hidden,
        object_definition_id: String::new(),
    })
}

fn junction_to_internal(j: &Junction) -> SchRecord {
    SchRecord::Junction(InternalJunction {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..default_base()
        },
        location: j.location,
        size: PenWidth::Zero,
        color: j.color,
        locked: true,
        unique_id: if j.unique_id.is_empty() { generate_unique_id() } else { j.unique_id.clone() },
    })
}

fn no_connect_to_internal(n: &NoConnect) -> SchRecord {
    SchRecord::NoConnect(InternalNoConnect {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..default_base()
        },
        location: n.location,
        color: n.color,
        orientation: n.orientation,
        symbol: n.symbol.clone(),
        is_active: n.is_active,
        suppress_all: n.suppress_all,
        error_kind_set_to_suppress: String::new(),
        connection_pairs_to_suppress: String::new(),
        unique_id: if n.unique_id.is_empty() { generate_unique_id() } else { n.unique_id.clone() },
    })
}

fn bus_entry_to_internal(b: &BusEntry) -> SchRecord {
    SchRecord::BusEntry(InternalBusEntry {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..default_base()
        },
        unique_id: if b.unique_id.is_empty() { generate_unique_id() } else { b.unique_id.clone() },
        location: b.location,
        corner: b.corner,
        line_width: b.line_width,
        color: b.color,
    })
}

fn note_to_internal(n: &Note) -> SchRecord {
    SchRecord::Note(InternalNote {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..default_base()
        },
        location: n.location,
        corner: n.corner,
        line_width: PenWidth::Zero,
        color: n.color,
        area_color: n.area_color,
        text_color: n.text_color,
        font_id: n.font_id,
        is_solid: n.is_solid,
        show_border: n.show_border,
        alignment: n.alignment,
        word_wrap: n.word_wrap,
        clip_to_rect: n.clip_to_rect,
        text: n.text.clone(),
        text_margin: n.text_margin,
        collapsed: n.collapsed,
        author: n.author.clone(),
        unique_id: if n.unique_id.is_empty() { generate_unique_id() } else { n.unique_id.clone() },
    })
}

fn probe_to_internal(p: &Probe) -> SchRecord {
    SchRecord::Probe(InternalProbe {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..default_base()
        },
        location: p.location,
        color: p.color,
        orientation: p.orientation,
        name: p.name.clone(),
        unique_id: if p.unique_id.is_empty() { generate_unique_id() } else { p.unique_id.clone() },
    })
}

fn compile_mask_to_internal(c: &CompileMask) -> SchRecord {
    SchRecord::CompileMask(InternalCompileMask {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..default_base()
        },
        unique_id: if c.unique_id.is_empty() { generate_unique_id() } else { c.unique_id.clone() },
        location: c.location,
        corner: c.corner,
        color: c.color,
        area_color: c.area_color,
        collapsed: c.collapsed,
        line_width: c.line_width,
    })
}

fn blanket_to_internal(b: &Blanket) -> SchRecord {
    SchRecord::Blanket(InternalBlanket {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..default_base()
        },
        location: b.location,
        corner: b.corner,
        color: b.color,
        area_color: b.area_color,
        line_style: b.line_style,
        line_style_ext: b.line_style,
        line_width: b.line_width,
        vertices: b.vertices.clone(),
        collapsed: b.collapsed,
        unique_id: if b.unique_id.is_empty() { generate_unique_id() } else { b.unique_id.clone() },
    })
}

fn signal_harness_to_internal(sh: &SignalHarness) -> SchRecord {
    SchRecord::SignalHarness(InternalBus {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..default_base()
        },
        color: sh.color,
        line_width: sh.line_width,
        vertices: sh.vertices.clone(),
        unique_id: if sh.unique_id.is_empty() { generate_unique_id() } else { sh.unique_id.clone() },
        underline_color: Color::BLACK,
        assigned_interface: String::new(),
        assigned_interface_signal: String::new(),
    })
}

fn sheet_entry_to_internal(e: &SheetEntry, owner_index: i32) -> InternalSheetEntry {
    InternalSheetEntry {
        base: SchPrimitiveBase {
            owner_index,
            ..default_base()
        },
        location: CoordPoint::zero(),
        side: e.side,
        distance_from_top: e.distance_from_top,
        color: e.color,
        area_color: e.area_color,
        text_color: e.text_color,
        text_font_id: e.text_font_id,
        text_style: String::new(),
        name: e.name.clone(),
        harness_type: String::new(),
        io_type: e.io_type,
        style: e.style,
        arrow_kind: String::new(),
        unique_id: if e.unique_id.is_empty() { generate_unique_id() } else { e.unique_id.clone() },
    }
}

// ── Helper ───────────────────────────────────────────────────────────────────

/// Set the owner_index on a SchRecord.
fn set_owner_index(rec: &mut SchRecord, owner_index: i32) {
    match rec {
        SchRecord::Sheet(v) => v.base.owner_index = owner_index,
        SchRecord::Template(v) => v.base.owner_index = owner_index,
        SchRecord::Wire(v) => v.base.owner_index = owner_index,
        SchRecord::Bus(v) => v.base.owner_index = owner_index,
        SchRecord::NetLabel(v) => v.base.owner_index = owner_index,
        SchRecord::PowerObject(v) => v.base.owner_index = owner_index,
        SchRecord::Port(v) => v.base.owner_index = owner_index,
        SchRecord::NoConnect(v) => v.base.owner_index = owner_index,
        SchRecord::Junction(v) => v.base.owner_index = owner_index,
        SchRecord::SheetName(v) => v.base.owner_index = owner_index,
        SchRecord::SheetFileName(v) => v.base.owner_index = owner_index,
        SchRecord::SheetSymbol(v) => v.base.owner_index = owner_index,
        SchRecord::SheetEntry(v) => v.base.owner_index = owner_index,
        SchRecord::BusEntry(v) => v.base.owner_index = owner_index,
        SchRecord::ParameterSet(v) => v.base.owner_index = owner_index,
        SchRecord::Note(v) => v.base.owner_index = owner_index,
        SchRecord::Probe(v) => v.base.owner_index = owner_index,
        SchRecord::CompileMask(v) => v.base.owner_index = owner_index,
        SchRecord::Blanket(v) => v.base.owner_index = owner_index,
        SchRecord::Component(v) => v.owner_index = owner_index,
        SchRecord::Pin(v) => v.owner_index = owner_index,
        SchRecord::Symbol(v) => v.base.owner_index = owner_index,
        SchRecord::Line(v) => v.base.owner_index = owner_index,
        SchRecord::Rectangle(v) => v.base.owner_index = owner_index,
        SchRecord::RoundRectangle(v) => v.base.owner_index = owner_index,
        SchRecord::Arc(v) => v.base.owner_index = owner_index,
        SchRecord::EllipticalArc(v) => v.base.owner_index = owner_index,
        SchRecord::Ellipse(v) => v.base.owner_index = owner_index,
        SchRecord::Pie(v) => v.base.owner_index = owner_index,
        SchRecord::Polyline(v) => v.base.owner_index = owner_index,
        SchRecord::Polygon(v) => v.base.owner_index = owner_index,
        SchRecord::Bezier(v) => v.base.owner_index = owner_index,
        SchRecord::Image(v) => v.base.owner_index = owner_index,
        SchRecord::Label(v) => v.base.owner_index = owner_index,
        SchRecord::Hyperlink(v) => v.base.owner_index = owner_index,
        SchRecord::Designator(v) => v.base.owner_index = owner_index,
        SchRecord::Parameter(v) => v.base.owner_index = owner_index,
        SchRecord::TextFrame(v) => v.base.owner_index = owner_index,
        SchRecord::ImplementationList(v) => v.base.owner_index = owner_index,
        SchRecord::Implementation(v) => v.base.owner_index = owner_index,
        SchRecord::ImplementationMap(v) => v.base.owner_index = owner_index,
        SchRecord::MapDefiner(v) => v.base.owner_index = owner_index,
        SchRecord::ParameterList(v) => v.base.owner_index = owner_index,
        SchRecord::HarnessConnector(v) => v.base.owner_index = owner_index,
        SchRecord::HarnessEntry(v) => v.base.owner_index = owner_index,
        SchRecord::HarnessConnectorType(v) => v.base.owner_index = owner_index,
        SchRecord::SignalHarness(v) => v.base.owner_index = owner_index,
        SchRecord::HighLevelCodeSymbol(v) => v.base.owner_index = owner_index,
        SchRecord::HighLevelCodeEntry(v) => v.base.owner_index = owner_index,
        SchRecord::HighLevelCodeName(v) => v.base.owner_index = owner_index,
        SchRecord::HighLevelCodeFileName(v) => v.base.owner_index = owner_index,
    }
}

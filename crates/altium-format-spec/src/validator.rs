//! Phase 3 consistency checks for compiled spec models.
//!
//! Each validate function returns `Ok(warnings)` when the spec is structurally
//! valid (warnings are non-fatal) or `Err(errors)` when hard errors are found.
//!
//! Duplicate-ID detection here is the authoritative cross-file check; the
//! compiler also detects within-file duplicates as a fast-fail first pass.

use std::collections::HashMap;

use crate::eval::{SpecError, SpecErrorCode, Severity};
use crate::model::{SchDocSpec, PcbDocSpec};

/// Phase 3 consistency checks for a compiled [`SchDocSpec`].
///
/// Returns `Ok(warnings)` when the spec is valid (possibly with non-fatal
/// warnings such as unresolved pin references), or `Err(errors)` when one or
/// more hard errors are detected.
pub fn validate_schdoc_spec(spec: &SchDocSpec) -> Result<Vec<SpecError>, Vec<SpecError>> {
    let mut errors: Vec<SpecError> = Vec::new();
    let mut warnings: Vec<SpecError> = Vec::new();

    // Collect all designators across all sheets, detect duplicates.
    let mut seen_designators: HashMap<&str, usize> = HashMap::new();
    for sheet in &spec.sheets {
        for component in &sheet.components {
            let des = component.designator.as_str();
            if let Some(first_sheet_idx) = seen_designators.get(des) {
                errors.push(SpecError::no_span(
                    SpecErrorCode::DuplicateDesignator,
                    format!(
                        "duplicate designator '{}': first seen in sheet {}, also in sheet {}",
                        des,
                        first_sheet_idx,
                        spec.sheets.iter().position(|s| std::ptr::eq(s, sheet)).unwrap_or(0),
                    ),
                ));
            } else {
                seen_designators.insert(des, spec.sheets.iter().position(|s| std::ptr::eq(s, sheet)).unwrap_or(0));
            }
        }
    }

    // Validate net pin references: every component designator in a net's pin
    // list must exist in the designator set we just collected.
    for sheet in &spec.sheets {
        for net in &sheet.nets {
            for pin_ref in &net.pins {
                if !seen_designators.contains_key(pin_ref.component.as_str()) {
                    errors.push(SpecError::no_span(
                        SpecErrorCode::DanglingNetRef,
                        format!(
                            "net '{}' references component '{}' which does not exist in any sheet",
                            net.name, pin_ref.component,
                        ),
                    ));
                }
            }
        }
        // Also check power objects' pin lists.
        for power in &sheet.powers {
            for pin_ref in &power.pins {
                if !seen_designators.contains_key(pin_ref.component.as_str()) {
                    errors.push(SpecError::no_span(
                        SpecErrorCode::DanglingNetRef,
                        format!(
                            "power net '{}' references component '{}' which does not exist in any sheet",
                            power.name, pin_ref.component,
                        ),
                    ));
                }
            }
        }
    }

    // Authoritative cross-file annotation ID uniqueness check.
    let mut seen_annotation_ids: HashMap<&str, &str> = HashMap::new();

    // Check sheet annotations.
    for sheet in &spec.sheets {
        if let Some(ann) = &sheet.annotation {
            check_annotation_id(ann.id.as_str(), "sheet", &mut seen_annotation_ids, &mut errors);
        }
        // Check component annotations.
        for component in &sheet.components {
            if let Some(ann) = &component.annotation {
                check_annotation_id(ann.id.as_str(), &component.designator, &mut seen_annotation_ids, &mut errors);
            }
        }
        // Check net annotations.
        for net in &sheet.nets {
            if let Some(ann) = &net.annotation {
                check_annotation_id(ann.id.as_str(), &net.name, &mut seen_annotation_ids, &mut errors);
            }
        }
        // Check power annotations.
        for power in &sheet.powers {
            if let Some(ann) = &power.annotation {
                check_annotation_id(ann.id.as_str(), &power.name, &mut seen_annotation_ids, &mut errors);
            }
        }
    }

    // Pin references to non-existent pins are warnings (pins may come from
    // library which is not yet resolved at Phase 3).
    // The pin name/existence check is deferred to Phase 4 (resolver); here we
    // emit a warning for any net pin_ref whose pin designator we cannot verify
    // statically.  Since SchDocSpec has no pin inventory (that lives in
    // SchLibSpec), every net pin reference that names a real component is a
    // candidate warning.  We skip this check if we already errored on the
    // component reference (dangling refs) to avoid noise.
    for sheet in &spec.sheets {
        for net in &sheet.nets {
            for pin_ref in &net.pins {
                // Only warn when the component IS known (otherwise already an error).
                if seen_designators.contains_key(pin_ref.component.as_str()) {
                    // Pin existence cannot be checked without the library — emit warning.
                    warnings.push(
                        SpecError::no_span(
                            SpecErrorCode::UnresolvedPinRef,
                            format!(
                                "pin reference '{}.{}' in net '{}' cannot be verified without library (pin list not yet resolved)",
                                pin_ref.component, pin_ref.pin, net.name,
                            ),
                        )
                        .with_severity(Severity::Warning),
                    );
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(warnings)
    } else {
        Err(errors)
    }
}

/// Phase 3 consistency checks for a compiled [`PcbDocSpec`].
///
/// Returns `Ok(warnings)` when the spec is valid, or `Err(errors)` on hard
/// failures.
pub fn validate_pcbdoc_spec(spec: &PcbDocSpec) -> Result<Vec<SpecError>, Vec<SpecError>> {
    let mut errors: Vec<SpecError> = Vec::new();

    // Collect all designators across all boards, detect duplicates.
    let mut seen_designators: HashMap<&str, usize> = HashMap::new();
    for (board_idx, board) in spec.boards.iter().enumerate() {
        for component in &board.components {
            let des = component.designator.as_str();
            if let Some(first_board_idx) = seen_designators.get(des) {
                errors.push(SpecError::no_span(
                    SpecErrorCode::DuplicateDesignator,
                    format!(
                        "duplicate designator '{}': first seen in board {}, also in board {}",
                        des, first_board_idx, board_idx,
                    ),
                ));
            } else {
                seen_designators.insert(des, board_idx);
            }
        }
    }

    // Authoritative cross-file annotation ID uniqueness check.
    let mut seen_annotation_ids: HashMap<&str, &str> = HashMap::new();

    for board in &spec.boards {
        if let Some(ann) = &board.annotation {
            check_annotation_id(ann.id.as_str(), &board.name, &mut seen_annotation_ids, &mut errors);
        }
        for component in &board.components {
            if let Some(ann) = &component.annotation {
                check_annotation_id(ann.id.as_str(), &component.designator, &mut seen_annotation_ids, &mut errors);
            }
        }
        for net in &board.nets {
            if let Some(ann) = &net.annotation {
                check_annotation_id(ann.id.as_str(), &net.name, &mut seen_annotation_ids, &mut errors);
            }
        }
        for polygon in &board.polygons {
            if let Some(ann) = &polygon.annotation {
                check_annotation_id(ann.id.as_str(), &polygon.name, &mut seen_annotation_ids, &mut errors);
            }
        }
        for rule in &board.rules {
            if let Some(ann) = &rule.annotation {
                check_annotation_id(ann.id.as_str(), &rule.name, &mut seen_annotation_ids, &mut errors);
            }
        }
        for class in &board.classes {
            if let Some(ann) = &class.annotation {
                check_annotation_id(ann.id.as_str(), &class.name, &mut seen_annotation_ids, &mut errors);
            }
        }
        for dp in &board.differential_pairs {
            if let Some(ann) = &dp.annotation {
                check_annotation_id(ann.id.as_str(), &dp.name, &mut seen_annotation_ids, &mut errors);
            }
        }
    }

    if errors.is_empty() {
        Ok(Vec::new())
    } else {
        Err(errors)
    }
}

/// Push `id` into `seen_ids`, recording `owner_name` as the canonical owner.
/// Emits a [`SpecErrorCode::DuplicateAnnotationId`] error if the ID was
/// already registered.
fn check_annotation_id<'a>(
    id: &'a str,
    owner_name: &'a str,
    seen_ids: &mut HashMap<&'a str, &'a str>,
    errors: &mut Vec<SpecError>,
) {
    if let Some(first_owner) = seen_ids.get(id) {
        errors.push(SpecError::no_span(
            SpecErrorCode::DuplicateAnnotationId,
            format!(
                "duplicate annotation ID '{}': first used by '{}', also used by '{}'",
                id, first_owner, owner_name,
            ),
        ));
    } else {
        seen_ids.insert(id, owner_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{SchDocSpec, SheetSpec, SchDocComponentSpec, NetSpec, PinRef, SymbolRef};
    use crate::annotation::CompiledAnnotation;
    use altium_format_types::{Coord, CoordPoint};

    fn make_component(designator: &str) -> SchDocComponentSpec {
        SchDocComponentSpec {
            annotation: None,
            designator: designator.to_string(),
            symbol: SymbolRef::Literal(designator.to_string()),
            location: CoordPoint { x: Coord::ZERO, y: Coord::ZERO },
            orientation: None,
            is_mirrored: None,
            description: None,
            parameters: Vec::new(),
        }
    }

    fn make_net(name: &str, pins: Vec<(&str, &str)>) -> NetSpec {
        NetSpec {
            annotation: None,
            name: name.to_string(),
            pins: pins
                .into_iter()
                .map(|(comp, pin)| PinRef {
                    component: comp.to_string(),
                    pin: pin.to_string(),
                })
                .collect(),
        }
    }

    fn empty_sheet() -> SheetSpec {
        SheetSpec {
            annotation: None,
            fonts: Vec::new(),
            custom_width: None,
            custom_height: None,
            snap_grid_on: None,
            visible_grid_on: None,
            hot_spot_grid_on: None,
            show_hidden_pins: None,
            border_on: None,
            title_block_on: None,
            components: Vec::new(),
            nets: Vec::new(),
            powers: Vec::new(),
            objects: Vec::new(),
        }
    }

    #[test]
    fn valid_spec_passes_with_no_errors() {
        let spec = SchDocSpec {
            sheets: vec![{
                let mut sheet = empty_sheet();
                sheet.components.push(make_component("U1"));
                sheet.components.push(make_component("R1"));
                sheet
            }],
        };
        let result = validate_schdoc_spec(&spec);
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
    }

    #[test]
    fn duplicate_designator_is_caught() {
        let spec = SchDocSpec {
            sheets: vec![
                {
                    let mut s = empty_sheet();
                    s.components.push(make_component("U1"));
                    s
                },
                {
                    let mut s = empty_sheet();
                    s.components.push(make_component("U1"));
                    s
                },
            ],
        };
        let result = validate_schdoc_spec(&spec);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.code == SpecErrorCode::DuplicateDesignator),
            "expected DuplicateDesignator error, got: {:?}",
            errors,
        );
    }

    #[test]
    fn dangling_net_ref_is_caught() {
        let spec = SchDocSpec {
            sheets: vec![{
                let mut sheet = empty_sheet();
                sheet.components.push(make_component("R1"));
                sheet.nets.push(make_net("VCC", vec![("U99", "1")]));
                sheet
            }],
        };
        let result = validate_schdoc_spec(&spec);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.code == SpecErrorCode::DanglingNetRef),
            "expected DanglingNetRef error, got: {:?}",
            errors,
        );
    }

    #[test]
    fn duplicate_annotation_id_is_caught() {
        let shared_id = "ABCD1234";
        let spec = SchDocSpec {
            sheets: vec![{
                let mut sheet = empty_sheet();
                let mut c1 = make_component("R1");
                c1.annotation = Some(CompiledAnnotation {
                    id: shared_id.to_string(),
                    stable: false,
                    group: None,
                });
                let mut c2 = make_component("R2");
                c2.annotation = Some(CompiledAnnotation {
                    id: shared_id.to_string(),
                    stable: false,
                    group: None,
                });
                sheet.components.push(c1);
                sheet.components.push(c2);
                sheet
            }],
        };
        let result = validate_schdoc_spec(&spec);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.code == SpecErrorCode::DuplicateAnnotationId),
            "expected DuplicateAnnotationId error, got: {:?}",
            errors,
        );
    }

    #[test]
    fn pin_ref_to_known_component_emits_warning_not_error() {
        let spec = SchDocSpec {
            sheets: vec![{
                let mut sheet = empty_sheet();
                sheet.components.push(make_component("R1"));
                sheet.nets.push(make_net("VCC", vec![("R1", "1")]));
                sheet
            }],
        };
        let result = validate_schdoc_spec(&spec);
        // Valid spec: R1 exists, so it is Ok(warnings)
        assert!(result.is_ok(), "expected Ok(warnings), got {:?}", result);
        let warnings = result.unwrap();
        assert!(
            warnings
                .iter()
                .any(|w| w.severity == Severity::Warning && w.code == SpecErrorCode::UnresolvedPinRef),
            "expected UnresolvedPinRef warning, got: {:?}",
            warnings,
        );
    }

    // ── PcbDocSpec tests ──────────────────────────────────────────────────────

    fn make_pcbdoc_component(designator: &str) -> crate::model::PcbDocComponentSpec {
        crate::model::PcbDocComponentSpec {
            annotation: None,
            designator: designator.to_string(),
            pattern: None,
            comment: None,
            location: None,
            rotation: None,
            layer: None,
            source_library: None,
        }
    }

    fn empty_board(name: &str) -> crate::model::BoardSpec {
        crate::model::BoardSpec {
            annotation: None,
            name: name.to_string(),
            signal_layer_count: None,
            snap_grid_size: None,
            visible_grid_size: None,
            display_unit: None,
            nets: Vec::new(),
            components: Vec::new(),
            tracks: Vec::new(),
            arcs: Vec::new(),
            vias: Vec::new(),
            pads: Vec::new(),
            fills: Vec::new(),
            texts: Vec::new(),
            regions: Vec::new(),
            component_bodies: Vec::new(),
            dimensions: Vec::new(),
            polygons: Vec::new(),
            rules: Vec::new(),
            classes: Vec::new(),
            differential_pairs: Vec::new(),
        }
    }

    #[test]
    fn pcbdoc_valid_spec_passes() {
        let spec = PcbDocSpec {
            boards: vec![{
                let mut b = empty_board("main");
                b.components.push(make_pcbdoc_component("U1"));
                b
            }],
            placement: None,
            placement_rules: Vec::new(),
        };
        let result = validate_pcbdoc_spec(&spec);
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
    }

    #[test]
    fn pcbdoc_duplicate_designator_is_caught() {
        let spec = PcbDocSpec {
            boards: vec![
                {
                    let mut b = empty_board("board_a");
                    b.components.push(make_pcbdoc_component("U1"));
                    b
                },
                {
                    let mut b = empty_board("board_b");
                    b.components.push(make_pcbdoc_component("U1"));
                    b
                },
            ],
            placement: None,
            placement_rules: Vec::new(),
        };
        let result = validate_pcbdoc_spec(&spec);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.code == SpecErrorCode::DuplicateDesignator),
        );
    }

    #[test]
    fn pcbdoc_duplicate_annotation_id_is_caught() {
        let shared_id = "EFGH5678";
        let spec = PcbDocSpec {
            boards: vec![{
                let mut b = empty_board("main");
                let mut c1 = make_pcbdoc_component("U1");
                c1.annotation = Some(CompiledAnnotation {
                    id: shared_id.to_string(),
                    stable: false,
                    group: None,
                });
                let mut c2 = make_pcbdoc_component("U2");
                c2.annotation = Some(CompiledAnnotation {
                    id: shared_id.to_string(),
                    stable: false,
                    group: None,
                });
                b.components.push(c1);
                b.components.push(c2);
                b
            }],
            placement: None,
            placement_rules: Vec::new(),
        };
        let result = validate_pcbdoc_spec(&spec);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.code == SpecErrorCode::DuplicateAnnotationId),
        );
    }
}

//! Phase 4 library resolution for SchDoc specs.
//!
//! Resolves component symbol references against a set of [`SchLibSpec`]
//! instances to build a designator → footprint name mapping.
//!
//! If a component explicitly references a library that is not present in the
//! provided slice, the function returns a hard error.  Components with no
//! library reference (bare designators) are valid and produce no footprint
//! mapping entry.

use std::collections::HashMap;

use crate::eval::{SpecError, SpecErrorCode};
use crate::model::{SchDocSpec, SchLibSpec, SymbolRef};

/// Output of Phase 4 library resolution.
///
/// `footprint_map` maps each component designator to its footprint model name,
/// populated only for components whose symbol could be resolved in the provided
/// libraries.  Designators with no library reference are absent from the map
/// (this is valid — not an error).
#[derive(Debug)]
pub struct FootprintResolvedSpec {
    /// designator → footprint model name
    pub footprint_map: HashMap<String, String>,
}

/// Resolve component symbol references in `model` against `libraries`.
///
/// For each component in every sheet of `model`:
/// - If the symbol reference is a named import alias (`SymbolRef::Import`), the
///   alias must match one of the library aliases that the caller has pre-loaded
///   into `libraries`.  Currently this function matches by `lib_reference` name
///   within the provided library slice.
/// - If the symbol reference is a bare literal (`SymbolRef::Literal`), the
///   function searches all provided libraries for a component with a matching
///   `lib_reference`.
///
/// When a library is explicitly referenced by alias but is not present in
/// `libraries`, this function returns a hard error: the caller must supply all
/// referenced libraries.  Components with no symbol → library resolution are
/// silently omitted from the footprint map.
pub fn resolve_schdoc_spec(
    model: &SchDocSpec,
    libraries: &[SchLibSpec],
) -> Result<FootprintResolvedSpec, SpecError> {
    let mut footprint_map: HashMap<String, String> = HashMap::new();

    for sheet in &model.sheets {
        for component in &sheet.components {
            match &component.symbol {
                SymbolRef::Import { alias, name } => {
                    // The caller must have provided the library referenced by `alias`.
                    // We match libraries positionally (no alias metadata on SchLibSpec
                    // itself), so we search by lib_reference name across all libraries.
                    //
                    // LIMITATION: This resolver ignores the library alias entirely — it
                    // searches all provided libraries for a component whose `lib_reference`
                    // matches `name`, regardless of which alias the spec declares. Because
                    // `SchLibSpec` does not carry library identity (filename or alias),
                    // alias-based disambiguation is not possible at this layer. When library
                    // identity is added to `SchLibSpec`, this lookup must be updated to
                    // filter by alias first before falling back to a name-only search.
                    let resolved = libraries
                        .iter()
                        .flat_map(|lib| &lib.components)
                        .find(|c| &c.lib_reference == name);

                    match resolved {
                        Some(lib_component) => {
                            // Use the first footprint mapping, if any.
                            if let Some(fp) = lib_component.footprints.first() {
                                footprint_map
                                    .insert(component.designator.clone(), fp.model_name.clone());
                            }
                        }
                        None => {
                            // The alias was referenced but no matching component was found
                            // in the provided libraries.
                            return Err(SpecError::no_span(
                                SpecErrorCode::UnresolvableLibrary,
                                format!(
                                    "cannot resolve library '{}' referenced by component '{}': \
                                     no matching component '{}' found in provided libraries",
                                    alias, component.designator, name,
                                ),
                            ));
                        }
                    }
                }

                SymbolRef::Literal(lib_ref) => {
                    // Search all provided libraries for a matching lib_reference.
                    let resolved = libraries
                        .iter()
                        .flat_map(|lib| &lib.components)
                        .find(|c| &c.lib_reference == lib_ref);

                    if let Some(lib_component) = resolved {
                        if let Some(fp) = lib_component.footprints.first() {
                            footprint_map
                                .insert(component.designator.clone(), fp.model_name.clone());
                        }
                    }
                    // No library reference found and SymbolRef::Literal → valid;
                    // component simply has no footprint mapping.
                }
            }
        }
    }

    Ok(FootprintResolvedSpec { footprint_map })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ComponentSpec, FootprintMapSpec, SchDocComponentSpec, SchDocSpec, SchLibSpec, SheetSpec,
        SymbolRef,
    };
    use altium_format_types::{Coord, CoordPoint};

    fn empty_sheet() -> SheetSpec {
        SheetSpec {
            annotation: None,
            fonts: Vec::new(),
            power_declarations: std::collections::HashMap::new(),
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
            constraints: Vec::new(),
        }
    }

    fn make_schdoc_component(designator: &str, lib_ref: &str) -> SchDocComponentSpec {
        SchDocComponentSpec {
            annotation: None,
            designator: designator.to_string(),
            symbol: SymbolRef::Literal(lib_ref.to_string()),
            location: CoordPoint {
                x: Coord::ZERO,
                y: Coord::ZERO,
            },
            orientation: None,
            is_mirrored: None,
            description: None,
            parameters: Vec::new(),
            pin_connections: Vec::new(),
        }
    }

    fn make_schdoc_component_import(
        designator: &str,
        alias: &str,
        name: &str,
    ) -> SchDocComponentSpec {
        SchDocComponentSpec {
            annotation: None,
            designator: designator.to_string(),
            symbol: SymbolRef::Import {
                alias: alias.to_string(),
                name: name.to_string(),
            },
            location: CoordPoint {
                x: Coord::ZERO,
                y: Coord::ZERO,
            },
            orientation: None,
            is_mirrored: None,
            description: None,
            parameters: Vec::new(),
            pin_connections: Vec::new(),
        }
    }

    fn make_schlib_component(lib_ref: &str, footprint: &str) -> ComponentSpec {
        ComponentSpec {
            annotation: None,
            lib_reference: lib_ref.to_string(),
            designator: None,
            description: None,
            component_kind: None,
            part_count: None,
            show_hidden_pins: None,
            pins: Vec::new(),
            parameters: Vec::new(),
            aliases: Vec::new(),
            footprints: vec![FootprintMapSpec {
                model_name: footprint.to_string(),
                maps: Vec::new(),
                source: None,
            }],
            graphics: Vec::new(),
            parts: Vec::new(),
        }
    }

    fn make_schlib(components: Vec<ComponentSpec>) -> SchLibSpec {
        SchLibSpec { components }
    }

    #[test]
    fn resolver_with_library_populates_footprint_map() {
        let mut sheet = empty_sheet();
        sheet
            .components
            .push(make_schdoc_component("R1", "Resistor"));
        let model = SchDocSpec {
            sheets: vec![sheet],
        };

        let lib = make_schlib(vec![make_schlib_component("Resistor", "0402")]);
        let result = resolve_schdoc_spec(&model, &[lib]);
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        let resolved = result.unwrap();
        assert_eq!(
            resolved.footprint_map.get("R1").map(|s| s.as_str()),
            Some("0402")
        );
    }

    #[test]
    fn resolver_without_library_produces_empty_map_no_error() {
        let mut sheet = empty_sheet();
        sheet
            .components
            .push(make_schdoc_component("R1", "Resistor"));
        let model = SchDocSpec {
            sheets: vec![sheet],
        };

        let result = resolve_schdoc_spec(&model, &[]);
        assert!(
            result.is_ok(),
            "expected Ok with empty map, got {:?}",
            result.err()
        );
        let resolved = result.unwrap();
        assert!(
            resolved.footprint_map.is_empty(),
            "expected empty footprint map, got {:?}",
            resolved.footprint_map,
        );
    }

    #[test]
    fn resolver_import_alias_found_populates_footprint() {
        let mut sheet = empty_sheet();
        sheet
            .components
            .push(make_schdoc_component_import("U1", "mylib", "OpAmp"));
        let model = SchDocSpec {
            sheets: vec![sheet],
        };

        let lib = make_schlib(vec![make_schlib_component("OpAmp", "SOIC8")]);
        let result = resolve_schdoc_spec(&model, &[lib]);
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(
            resolved.footprint_map.get("U1").map(|s| s.as_str()),
            Some("SOIC8")
        );
    }

    #[test]
    fn resolver_import_alias_not_found_returns_hard_error() {
        let mut sheet = empty_sheet();
        sheet
            .components
            .push(make_schdoc_component_import("U1", "missing_lib", "OpAmp"));
        let model = SchDocSpec {
            sheets: vec![sheet],
        };

        // Libraries provided but don't contain "OpAmp"
        let lib = make_schlib(vec![make_schlib_component("Resistor", "0402")]);
        let result = resolve_schdoc_spec(&model, &[lib]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, SpecErrorCode::UnresolvableLibrary);
        assert!(
            err.message.contains("missing_lib"),
            "error should mention the alias, got: {}",
            err.message,
        );
        assert!(
            err.message.contains("U1"),
            "error should mention the component designator, got: {}",
            err.message,
        );
    }

    #[test]
    fn resolver_import_alias_uses_first_match_not_alias() {
        // LIMITATION: alias is ignored; first library with matching lib_reference wins.
        // Both libraries contain "OpAmp" but with different footprints.  The resolver
        // searches all libraries in order and picks the first match, ignoring which
        // library alias the spec declared.
        let mut sheet = empty_sheet();
        sheet
            .components
            .push(make_schdoc_component_import("U1", "second_lib", "OpAmp"));
        let model = SchDocSpec {
            sheets: vec![sheet],
        };

        let first_lib = make_schlib(vec![make_schlib_component("OpAmp", "SOIC8")]);
        let second_lib = make_schlib(vec![make_schlib_component("OpAmp", "DIP8")]);

        // Even though the alias says "second_lib", the first library wins because
        // SchLibSpec carries no identity metadata and the resolver searches in order.
        let result = resolve_schdoc_spec(&model, &[first_lib, second_lib]);
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(
            resolved.footprint_map.get("U1").map(|s| s.as_str()),
            Some("SOIC8"),
            "expected first library's footprint (SOIC8), alias disambiguation is not implemented",
        );
    }

    #[test]
    fn resolver_component_without_footprint_in_library_is_not_mapped() {
        let mut sheet = empty_sheet();
        sheet
            .components
            .push(make_schdoc_component("R1", "Resistor"));
        let model = SchDocSpec {
            sheets: vec![sheet],
        };

        // Library component has no footprints
        let lib = make_schlib(vec![ComponentSpec {
            annotation: None,
            lib_reference: "Resistor".to_string(),
            designator: None,
            description: None,
            component_kind: None,
            part_count: None,
            show_hidden_pins: None,
            pins: Vec::new(),
            parameters: Vec::new(),
            aliases: Vec::new(),
            footprints: Vec::new(),
            graphics: Vec::new(),
            parts: Vec::new(),
        }]);
        let result = resolve_schdoc_spec(&model, &[lib]);
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert!(
            resolved.footprint_map.is_empty(),
            "expected no mapping when library component has no footprints",
        );
    }
}

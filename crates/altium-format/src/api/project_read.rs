//! Read path: converts internal AltiumProject storage into public API types.

use indexmap::IndexMap;

use altium_format_types::project::{
    DifferenceCheckLevel, DocAnnotationScope, DocAutoNetClassScope, ERC_ERROR_KIND_COUNT,
    ERC_LEVEL_NAMED_KEYS, ErrorLevel, VariationKind,
};

use crate::api::{
    AnnotationMatchParameter, AnnotationSettings, BuildConfiguration, ClassGenSettings,
    ComparisonOption, ComponentVariation, DatabaseUpdateSettings, DiffPairSuffix, DifferenceLevel,
    DocumentRef, ErcConnectionMatrix, ErcLevel, LibraryUpdateSettings, ModificationLevel,
    OutputGroup, OutputJob, ParameterVariation, Project, ProjectParameter, ProjectVariant,
};
use crate::project::AltiumProject;
use crate::{AltiumFormatError, ResultExt};

fn get_str<'a>(map: &'a IndexMap<String, String>, key: &str) -> &'a str {
    map.get(key).map(|s| s.as_str()).unwrap_or("")
}

fn get_bool(map: &IndexMap<String, String>, key: &str) -> bool {
    map.get(key).map(|s| s == "1").unwrap_or(false)
}

fn get_int(map: &IndexMap<String, String>, key: &str) -> i32 {
    map.get(key)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
}

fn get_enum<E>(map: &IndexMap<String, String>, key: &str) -> crate::Result<E>
where
    E: TryFrom<i32, Error = altium_format_types::InvalidEnumValue>,
{
    let val = get_int(map, key);
    E::try_from(val)
        .map_err(AltiumFormatError::InvalidEnumValue)
        .with_context(|| format!("parsing key '{key}'"))
}

pub(crate) fn project_from_internal(proj: &AltiumProject) -> crate::Result<Project> {
    let design = proj.design();

    let auto_cross_refs = match design.get("AutoCrossReferences").map(|s| s.as_str()) {
        Some("1") => Some(true),
        Some("0") => Some(false),
        _ => None,
    };

    Ok(Project {
        name: proj.name().to_owned(),
        hierarchy_mode: get_enum(design, "HierarchyMode")?,
        channel_room_naming_style: get_enum(design, "ChannelRoomNamingStyle")?,
        channel_designator_format: get_str(design, "ChannelDesignatorFormatString").to_owned(),
        channel_room_level_separator: get_str(design, "ChannelRoomLevelSeperator").to_owned(),
        allow_port_net_names: get_bool(design, "AllowPortNetNames"),
        allow_sheet_entry_net_names: get_bool(design, "AllowSheetEntryNetNames"),
        netlist_single_pin_nets: get_bool(design, "NetlistSinglePinNets"),
        append_sheet_number_to_local_nets: get_bool(design, "AppendSheetNumberToLocalNets"),
        name_nets_hierarchically: get_bool(design, "NameNetsHierarchically"),
        power_port_names_take_priority: get_bool(design, "PowerPortNamesTakePriority"),
        pin_swap_by_netlabel: get_bool(design, "PinSwapBy_Netlabel"),
        pin_swap_by_pin: get_bool(design, "PinSwapBy_Pin"),
        cross_ref_sheet_style: get_enum(design, "CrossRefSheetStyle")?,
        cross_ref_location_style: get_enum(design, "CrossRefLocationStyle")?,
        cross_ref_ports: get_enum(design, "CrossRefPorts")?,
        cross_ref_cross_sheets: get_bool(design, "CrossRefCrossSheets"),
        cross_ref_sheet_entries: get_bool(design, "CrossRefSheetEntries"),
        cross_ref_follow_from_main_settings: get_bool(design, "CrossRefFollowFromMainSettings"),
        auto_sheet_numbering: get_bool(design, "AutoSheetNumbering"),
        auto_cross_references: auto_cross_refs,
        new_indexing_of_sheet_symbols: get_bool(design, "NewIndexingOfSheetSymbols"),
        output_path: get_str(design, "OutputPath").to_owned(),
        default_configuration: get_str(design, "DefaultConfiguration").to_owned(),
        documents: parse_documents(proj)?,
        configurations: parse_configurations(proj),
        output_groups: parse_output_groups(proj),
        annotation: parse_annotation(proj)?,
        class_gen: parse_class_gen(proj),
        library_update: parse_library_update(proj),
        database_update: parse_database_update(proj),
        comparison_options: parse_comparison_options(proj),
        erc_matrix: parse_erc_matrix(proj)?,
        erc_levels: parse_erc_levels(proj)?,
        modification_levels: parse_modification_levels(proj)?,
        difference_levels: parse_difference_levels(proj)?,
        variants: parse_variants(proj)?,
        parameters: parse_parameters(proj),
        diff_pair_suffixes: parse_diff_pair_suffixes(proj),
        net_infos: Vec::new(),
        smart_pdf_page_options: proj.smart_pdf().get("PageOptions").cloned(),
    })
}

fn parse_documents(proj: &AltiumProject) -> crate::Result<Vec<DocumentRef>> {
    proj.documents()
        .iter()
        .enumerate()
        .map(|(i, map)| {
            let scope_str = get_str(map, "AnnotateScope");
            let scope = if scope_str.is_empty() {
                DocAnnotationScope::All
            } else {
                scope_str
                    .parse::<DocAnnotationScope>()
                    .map_err(AltiumFormatError::InvalidEnumValue)
                    .with_context(|| format!("Document{}: parsing AnnotateScope", i + 1))?
            };
            let net_class_str = get_str(map, "ClassGenNCAutoScope");
            let net_class_scope = if net_class_str.is_empty() {
                DocAutoNetClassScope::None
            } else {
                net_class_str
                    .parse::<DocAutoNetClassScope>()
                    .map_err(AltiumFormatError::InvalidEnumValue)
                    .with_context(|| format!("Document{}: parsing ClassGenNCAutoScope", i + 1))?
            };
            Ok(DocumentRef {
                path: get_str(map, "DocumentPath").to_owned(),
                unique_id: get_str(map, "DocumentUniqueId").to_owned(),
                annotation_enabled: get_bool(map, "AnnotationEnabled"),
                annotate_start_value: get_int(map, "AnnotateStartValue"),
                annotation_index_control_enabled: get_bool(map, "AnnotationIndexControlEnabled"),
                annotate_suffix: get_str(map, "AnnotateSuffix").to_owned(),
                annotate_scope: scope,
                annotate_order: get_int(map, "AnnotateOrder"),
                do_library_update: get_bool(map, "DoLibraryUpdate"),
                do_database_update: get_bool(map, "DoDatabaseUpdate"),
                class_gen_cc_auto_enabled: get_bool(map, "ClassGenCCAutoEnabled"),
                class_gen_cc_auto_room_enabled: get_bool(map, "ClassGenCCAutoRoomEnabled"),
                class_gen_nc_auto_scope: net_class_scope,
                generate_class_cluster: get_bool(map, "GenerateClassCluster"),
            })
        })
        .collect()
}

fn parse_configurations(proj: &AltiumProject) -> Vec<BuildConfiguration> {
    proj.configurations()
        .iter()
        .map(|map| BuildConfiguration {
            name: get_str(map, "Name").to_owned(),
            variant: get_str(map, "Variant").to_owned(),
            content_type_guid: get_str(map, "ContentTypeGUID").to_owned(),
            configuration_type: get_str(map, "ConfigurationType").to_owned(),
            parameter_count: get_int(map, "ParameterCount"),
            constraint_file_count: get_int(map, "ConstraintFileCount"),
            output_jobs_count: get_int(map, "OutputJobsCount"),
            release_item_id: get_str(map, "ReleaseItemId").to_owned(),
        })
        .collect()
}

fn parse_output_groups(proj: &AltiumProject) -> Vec<OutputGroup> {
    proj.output_groups()
        .iter()
        .map(|raw| {
            let keys = raw.keys();
            let outputs = raw
                .outputs()
                .iter()
                .map(|out| OutputJob {
                    name: get_str(out, "OutputName").to_owned(),
                    output_type: get_str(out, "OutputType").to_owned(),
                    document_path: get_str(out, "OutputDocumentPath").to_owned(),
                    variant_name: get_str(out, "OutputVariantName").to_owned(),
                    is_default: get_bool(out, "OutputDefault"),
                    page_options: out.get("PageOptions").cloned(),
                })
                .collect();
            OutputGroup {
                name: get_str(keys, "Name").to_owned(),
                description: get_str(keys, "Description").to_owned(),
                target_printer: get_str(keys, "TargetPrinter").to_owned(),
                printer_options: get_str(keys, "PrinterOptions").to_owned(),
                outputs,
            }
        })
        .collect()
}

fn parse_erc_matrix(proj: &AltiumProject) -> crate::Result<ErcConnectionMatrix> {
    let matrix_map = proj.erc_matrix();
    let mut matrix = ErcConnectionMatrix::default();
    if matrix_map.is_empty() {
        // No section: Altium's matrix is all NoReport (Psch2-erc ParseFromStrings).
        return Ok(matrix);
    }
    if let Some(stray) = matrix_map
        .keys()
        .find(|k| !(1..=17).any(|n| **k == format!("L{n}")))
    {
        return Err(AltiumFormatError::InvalidParamValue {
            key: stray.clone(),
            detail: "unexpected key in [ERC Connection Matrix] (rows are L1..L17)".to_owned(),
        })
        .context("parsing ERC connection matrix");
    }
    for row in 0..17 {
        let key = format!("L{}", row + 1);
        let Some(row_str) = matrix_map.get(&key) else {
            return Err(AltiumFormatError::InvalidParamValue {
                key,
                detail: "missing row".to_owned(),
            })
            .context("parsing ERC connection matrix");
        };
        {
            let chars: Vec<char> = row_str.chars().collect();
            if chars.len() != 17 {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: key.clone(),
                    detail: format!("expected 17 characters, got {}", chars.len()),
                })
                .context("parsing ERC connection matrix");
            }
            for col in 0..17 {
                matrix.cells[row][col] = ErrorLevel::from_matrix_char(chars[col])
                    .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                        key: key.clone(),
                        detail: format!("invalid matrix char '{}' at column {}", chars[col], col),
                    })
                    .context("parsing ERC connection matrix")?;
            }
        }
    }
    Ok(matrix)
}

/// `[Electrical Rules Check]`: `Type<v+1>=<level>` (or a named key) per ERC
/// error kind `v`. An unknown key or a level that is not 0..=3 is an error
/// (Altium itself ignores such lines; here they would be silently lost).
fn parse_erc_levels(proj: &AltiumProject) -> crate::Result<Vec<ErcLevel>> {
    proj.erc_levels()
        .iter()
        .map(|(key, value)| {
            let known = match key.strip_prefix("Type") {
                Some(n) => {
                    n.parse::<u16>().is_ok_and(|n| {
                        (1..=ERC_ERROR_KIND_COUNT).contains(&n)
                            && !ERC_LEVEL_NAMED_KEYS.iter().any(|(_, v)| *v + 1 == n)
                    }) && !n.starts_with(['0', '+'])
                }
                None => ERC_LEVEL_NAMED_KEYS.iter().any(|(k, _)| k == key),
            };
            if !known {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: key.clone(),
                    detail: "not an ERC error kind key".to_owned(),
                })
                .context("parsing [Electrical Rules Check]");
            }
            let level_int =
                value
                    .parse::<i32>()
                    .map_err(|e| AltiumFormatError::InvalidParamValue {
                        key: key.clone(),
                        detail: format!("level '{value}': {e}"),
                    });
            let level = level_int
                .and_then(|v| ErrorLevel::try_from(v).map_err(AltiumFormatError::InvalidEnumValue))
                .with_context(|| format!("parsing [Electrical Rules Check] '{key}'"))?;
            Ok(ErcLevel {
                key: key.clone(),
                level,
            })
        })
        .collect()
}

/// A `Type<n>` key (n >= 1, no sign or leading zero) of the level sections.
fn level_type_index(section: &str, key: &str) -> crate::Result<u16> {
    key.strip_prefix("Type")
        .filter(|n| !n.starts_with(['0', '+']))
        .and_then(|n| n.parse::<u16>().ok())
        .ok_or_else(|| AltiumFormatError::InvalidParamValue {
            key: key.to_owned(),
            detail: "expected Type<n>".to_owned(),
        })
        .with_context(|| format!("parsing [{section}]"))
}

fn parse_modification_levels(proj: &AltiumProject) -> crate::Result<Vec<ModificationLevel>> {
    proj.modification_levels()
        .iter()
        .map(|(key, value)| {
            let index = level_type_index("Modification Levels", key)?;
            let enabled = match value.as_str() {
                "1" => true,
                "0" => false,
                other => {
                    return Err(AltiumFormatError::InvalidParamValue {
                        key: key.clone(),
                        detail: format!("expected 0 or 1, got '{other}'"),
                    })
                    .context("parsing [Modification Levels]");
                }
            };
            Ok(ModificationLevel {
                difference_kind_index: index,
                enabled,
            })
        })
        .collect()
}

fn parse_difference_levels(proj: &AltiumProject) -> crate::Result<Vec<DifferenceLevel>> {
    proj.difference_levels()
        .iter()
        .map(|(key, value)| {
            let index = level_type_index("Difference Levels", key)?;
            let level = value
                .parse::<i32>()
                .map_err(|e| AltiumFormatError::InvalidParamValue {
                    key: key.clone(),
                    detail: format!("level '{value}': {e}"),
                })
                .and_then(|v| {
                    DifferenceCheckLevel::try_from(v).map_err(AltiumFormatError::InvalidEnumValue)
                })
                .with_context(|| format!("parsing difference level '{key}'"))?;
            Ok(DifferenceLevel {
                difference_kind_index: index,
                level,
            })
        })
        .collect()
}

fn parse_annotation(proj: &AltiumProject) -> crate::Result<AnnotationSettings> {
    let map = proj.annotate();
    let mut match_params = Vec::new();
    for i in 1.. {
        let param_key = format!("MatchParameter{i}");
        if let Some(name) = map.get(&param_key) {
            let strict_key = format!("MatchStrictly{i}");
            let strict = map.get(&strict_key).map(|s| s == "1").unwrap_or(false);
            match_params.push(AnnotationMatchParameter {
                name: name.clone(),
                strict,
            });
        } else {
            break;
        }
    }
    Ok(AnnotationSettings {
        sort_order: get_enum(map, "SortOrder")?,
        sort_location: get_enum(map, "SortLocation")?,
        replace_subparts: get_bool(map, "ReplaceSubparts"),
        physical_naming_format: get_str(map, "PhysicalNamingFormat").to_owned(),
        global_index_sort_order: get_enum(map, "GlobalIndexSortOrder")?,
        global_index_sort_location: get_enum(map, "GlobalIndexSortLocation")?,
        match_parameters: match_params,
    })
}

fn parse_class_gen(proj: &AltiumProject) -> ClassGenSettings {
    let map = proj.class_gen();
    ClassGenSettings {
        comp_class_manual_enabled: get_bool(map, "CompClassManualEnabled"),
        comp_class_manual_room_enabled: get_bool(map, "CompClassManualRoomEnabled"),
        net_class_auto_bus_enabled: get_bool(map, "NetClassAutoBusEnabled"),
        net_class_auto_comp_enabled: get_bool(map, "NetClassAutoCompEnabled"),
        net_class_auto_named_harness_enabled: get_bool(map, "NetClassAutoNamedHarnessEnabled"),
        net_class_manual_enabled: get_bool(map, "NetClassManualEnabled"),
        net_class_separate_for_bus_sections: get_bool(map, "NetClassSeparateForBusSections"),
    }
}

fn parse_library_update(proj: &AltiumProject) -> LibraryUpdateSettings {
    let map = proj.library_update_options();
    LibraryUpdateSettings {
        selected_only: get_bool(map, "SelectedOnly"),
        update_variants: get_bool(map, "UpdateVariants"),
        update_to_latest_revision: get_bool(map, "UpdateToLatestRevision"),
        full_replace: get_bool(map, "FullReplace"),
        update_designator_lock: get_bool(map, "UpdateDesignatorLock"),
        update_part_id_lock: get_bool(map, "UpdatePartIDLock"),
        preserve_parameter_locations: get_bool(map, "PreserveParameterLocations"),
        preserve_parameter_visibility: get_bool(map, "PreserveParameterVisibility"),
        do_graphics: get_bool(map, "DoGraphics"),
        do_parameters: get_bool(map, "DoParameters"),
        do_models: get_bool(map, "DoModels"),
        add_parameters: get_bool(map, "AddParameters"),
        remove_parameters: get_bool(map, "RemoveParameters"),
        add_models: get_bool(map, "AddModels"),
        remove_models: get_bool(map, "RemoveModels"),
        update_current_models: get_bool(map, "UpdateCurrentModels"),
    }
}

fn parse_database_update(proj: &AltiumProject) -> DatabaseUpdateSettings {
    let map = proj.database_update_options();
    DatabaseUpdateSettings {
        selected_only: get_bool(map, "SelectedOnly"),
        update_variants: get_bool(map, "UpdateVariants"),
        update_to_latest_revision: get_bool(map, "UpdateToLatestRevision"),
        part_types: get_int(map, "PartTypes"),
    }
}

fn parse_comparison_options(proj: &AltiumProject) -> Vec<ComparisonOption> {
    proj.comparison_options()
        .iter()
        .map(|(_key, value)| {
            let pairs = parse_pipe_pairs(value);
            ComparisonOption {
                kind: pipe_get_str(&pairs, "Kind").to_owned(),
                min_percent: pipe_get_int(&pairs, "MinPercent"),
                min_match: pipe_get_int(&pairs, "MinMatch"),
                show_match: pipe_get_str(&pairs, "ShowMatch") == "1",
                use_name: pipe_get_int(&pairs, "UseName"),
                include_all_rules: pipe_get_str(&pairs, "InclAllRules") == "1",
            }
        })
        .collect()
}

/// Parse `Key=Val|Key=Val|...` into a list of (key, value) pairs.
fn parse_pipe_pairs(s: &str) -> Vec<(&str, &str)> {
    s.split('|')
        .filter(|seg| !seg.is_empty())
        .filter_map(|seg| {
            let eq = seg.find('=')?;
            Some((&seg[..eq], &seg[eq + 1..]))
        })
        .collect()
}

fn pipe_get_str<'a>(pairs: &[(&str, &'a str)], key: &str) -> &'a str {
    pairs
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| *v)
        .unwrap_or("")
}

fn pipe_get_int(pairs: &[(&str, &str)], key: &str) -> i32 {
    pipe_get_str(pairs, key).parse().unwrap_or(0)
}

/// Pipe pairs of a variation line; every pair must be `KEY=VALUE` with a key
/// from `allowed`, each at most once.
fn strict_pipe_pairs<'a>(
    key: &str,
    value: &'a str,
    allowed: &[&str],
) -> crate::Result<Vec<(&'a str, &'a str)>> {
    let mut pairs: Vec<(&str, &str)> = Vec::new();
    for seg in value.split('|').filter(|seg| !seg.is_empty()) {
        let (k, v) = seg
            .split_once('=')
            .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!("'{seg}' is not KEY=VALUE"),
            })?;
        if !allowed.iter().any(|a| a.eq_ignore_ascii_case(k)) {
            return Err(AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!("unknown field '{k}' (known: {allowed:?})"),
            });
        }
        if pairs.iter().any(|(p, _)| p.eq_ignore_ascii_case(k)) {
            return Err(AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!("field '{k}' repeated"),
            });
        }
        pairs.push((k, v));
    }
    Ok(pairs)
}

fn required_pipe<'a>(pairs: &[(&str, &'a str)], line: &str, field: &str) -> crate::Result<&'a str> {
    pairs
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(field))
        .map(|(_, v)| *v)
        .ok_or_else(|| AltiumFormatError::InvalidParamValue {
            key: line.to_owned(),
            detail: format!("missing field '{field}'"),
        })
}

/// A `0`/`1` flag of a variant section.
fn variant_flag(map: &IndexMap<String, String>, key: &str) -> crate::Result<Option<bool>> {
    map.get(key)
        .map(|v| match v.as_str() {
            "1" => Ok(true),
            "0" => Ok(false),
            other => Err(AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!("expected 0 or 1, got '{other}'"),
            }),
        })
        .transpose()
}

/// A count key of a variant section; it must equal the number of indexed lines.
fn check_variant_count(
    map: &IndexMap<String, String>,
    key: &str,
    found: usize,
) -> crate::Result<()> {
    if let Some(v) = map.get(key) {
        let n: usize = v
            .parse()
            .map_err(|e| AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!("'{v}': {e}"),
            })?;
        if n != found {
            return Err(AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!("count {n} but {found} indexed lines"),
            });
        }
    }
    Ok(())
}

/// `[ProjectVariantN]`: `UniqueId`, `Description`, `AllowFabrication`,
/// `OverwritePcbFootprint`, `VariationCount` + `Variation1..n`
/// (`Designator|UniqueId|Kind|AlternatePart`), `ParamVariation1..n`
/// (`Designator|ParameterName|VariantValue`) and `ParameterCount`. No PrjPcb
/// with variants is in the test corpora, so every other key, a gap in an
/// indexed list, a count that disagrees, or variant-level parameters
/// (`ParameterCount > 0`, layout unknown) are errors.
fn parse_variants(proj: &AltiumProject) -> crate::Result<Vec<ProjectVariant>> {
    proj.variants()
        .iter()
        .enumerate()
        .map(|(i, map)| {
            let section = format!("[ProjectVariant{}]", i + 1);
            (|| -> crate::Result<ProjectVariant> {
                let mut variations = Vec::new();
                while let Some(val) = map.get(&format!("Variation{}", variations.len() + 1)) {
                    let line = format!("Variation{}", variations.len() + 1);
                    let pairs = strict_pipe_pairs(
                        &line,
                        val,
                        &["Designator", "UniqueId", "Kind", "AlternatePart"],
                    )?;
                    let kind_raw = required_pipe(&pairs, &line, "Kind")?;
                    let kind_int: i32 =
                        kind_raw
                            .parse()
                            .map_err(|e| AltiumFormatError::InvalidParamValue {
                                key: line.clone(),
                                detail: format!("Kind '{kind_raw}': {e}"),
                            })?;
                    let kind = VariationKind::try_from(kind_int)
                        .map_err(AltiumFormatError::InvalidEnumValue)
                        .with_context(|| line.clone())?;
                    variations.push(ComponentVariation {
                        designator: required_pipe(&pairs, &line, "Designator")?.to_owned(),
                        unique_id: required_pipe(&pairs, &line, "UniqueId")?.to_owned(),
                        kind,
                        alternate_part: required_pipe(&pairs, &line, "AlternatePart")?.to_owned(),
                    });
                }
                let mut param_variations = Vec::new();
                while let Some(val) =
                    map.get(&format!("ParamVariation{}", param_variations.len() + 1))
                {
                    let line = format!("ParamVariation{}", param_variations.len() + 1);
                    let pairs = strict_pipe_pairs(
                        &line,
                        val,
                        &["Designator", "ParameterName", "VariantValue"],
                    )?;
                    param_variations.push(ParameterVariation {
                        designator: required_pipe(&pairs, &line, "Designator")?.to_owned(),
                        parameter_name: required_pipe(&pairs, &line, "ParameterName")?.to_owned(),
                        variant_value: required_pipe(&pairs, &line, "VariantValue")?.to_owned(),
                    });
                }
                check_variant_count(map, "VariationCount", variations.len())?;
                check_variant_count(map, "ParameterCount", 0).context(
                    "variant-level parameters are not supported (their layout is unverified)",
                )?;
                let indexed = |k: &str, prefix: &str| {
                    k.strip_prefix(prefix)
                        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
                };
                for key in map.keys() {
                    let known = matches!(
                        key.as_str(),
                        "UniqueId"
                            | "Description"
                            | "AllowFabrication"
                            | "OverwritePcbFootprint"
                            | "VariationCount"
                            | "ParameterCount"
                    ) || (indexed(key, "Variation")
                        && key[9..]
                            .parse::<usize>()
                            .is_ok_and(|n| n >= 1 && n <= variations.len()))
                        || (indexed(key, "ParamVariation")
                            && key[14..]
                                .parse::<usize>()
                                .is_ok_and(|n| n >= 1 && n <= param_variations.len()));
                    if !known {
                        return Err(AltiumFormatError::InvalidParamValue {
                            key: key.clone(),
                            detail: "unknown key, or an indexed line after a gap".to_owned(),
                        });
                    }
                }
                Ok(ProjectVariant {
                    unique_id: get_str(map, "UniqueId").to_owned(),
                    description: get_str(map, "Description").to_owned(),
                    allow_fabrication: variant_flag(map, "AllowFabrication")?,
                    overwrite_pcb_footprint: variant_flag(map, "OverwritePcbFootprint")?,
                    variations,
                    param_variations,
                })
            })()
            .with_context(|| section.clone())
        })
        .collect()
}

fn parse_parameters(proj: &AltiumProject) -> Vec<ProjectParameter> {
    proj.parameters_sections()
        .iter()
        .map(|map| ProjectParameter {
            name: get_str(map, "Name").to_owned(),
            value: get_str(map, "Value").to_owned(),
        })
        .collect()
}

fn parse_diff_pair_suffixes(proj: &AltiumProject) -> Vec<DiffPairSuffix> {
    proj.diff_pair_suffixes()
        .iter()
        .map(|map| DiffPairSuffix {
            positive: get_str(map, "Positive").to_owned(),
            negative: get_str(map, "Negative").to_owned(),
        })
        .collect()
}

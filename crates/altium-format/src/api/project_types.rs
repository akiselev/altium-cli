//! Public API types for PrjPcb project files.
//!
//! These types provide a clean, domain-typed interface for querying and mutating
//! Altium Designer project files. They abstract away the INI key-value storage
//! and provide structured access to project settings, documents, ERC configuration,
//! output groups, and build configurations.

use altium_format_types::color::Color;
use altium_format_types::project::{
    ChannelRoomNamingStyle, CrossRefLocationStyle, CrossRefPorts, CrossRefSheetStyle,
    DifferenceCheckLevel, DocAnnotationScope, DocAutoNetClassScope, ErrorLevel, FlattenMode,
    SortLocation, SortOrder, VariationKind,
};

// ── Project ─────────────────────────────────────────────────────────────────

/// Top-level project settings and children.
///
/// Natural key: project name (filename stem of the `.PrjPcb` file).
/// Contains all project-level settings from `[Design]` plus child collections
/// from numbered and singleton sections.
#[derive(Debug, Clone)]
pub struct Project {
    pub name: String,

    // ── [Design] section ────────────────────────────────────
    pub hierarchy_mode: FlattenMode,
    pub channel_room_naming_style: ChannelRoomNamingStyle,
    pub channel_designator_format: String,
    pub channel_room_level_separator: String,

    // Net naming
    pub allow_port_net_names: bool,
    pub allow_sheet_entry_net_names: bool,
    pub netlist_single_pin_nets: bool,
    pub append_sheet_number_to_local_nets: bool,
    pub name_nets_hierarchically: bool,
    pub power_port_names_take_priority: bool,

    // Pin swap
    pub pin_swap_by_netlabel: bool,
    pub pin_swap_by_pin: bool,

    // Cross-references
    pub cross_ref_sheet_style: CrossRefSheetStyle,
    pub cross_ref_location_style: CrossRefLocationStyle,
    pub cross_ref_ports: CrossRefPorts,
    pub cross_ref_cross_sheets: bool,
    pub cross_ref_sheet_entries: bool,
    pub cross_ref_follow_from_main_settings: bool,

    // Sheet numbering
    pub auto_sheet_numbering: bool,
    /// `None` maps to -1 (undefined) in the INI file.
    pub auto_cross_references: Option<bool>,
    pub new_indexing_of_sheet_symbols: bool,

    // Build / output
    pub output_path: String,
    pub default_configuration: String,

    // ── Children ────────────────────────────────────────────
    pub documents: Vec<DocumentRef>,
    pub configurations: Vec<BuildConfiguration>,
    pub output_groups: Vec<OutputGroup>,
    pub annotation: AnnotationSettings,
    pub class_gen: ClassGenSettings,
    pub library_update: LibraryUpdateSettings,
    pub database_update: DatabaseUpdateSettings,
    pub comparison_options: Vec<ComparisonOption>,
    pub erc_matrix: ErcConnectionMatrix,
    pub erc_levels: Vec<ErcLevel>,
    pub modification_levels: Vec<ModificationLevel>,
    pub difference_levels: Vec<DifferenceLevel>,
    pub variants: Vec<ProjectVariant>,
    pub parameters: Vec<ProjectParameter>,
    pub diff_pair_suffixes: Vec<DiffPairSuffix>,
    pub net_infos: Vec<NetInfo>,

    // ── [SmartPDF] ──────────────────────────────────────────
    pub smart_pdf_page_options: Option<String>,
}

// ── DocumentRef ─────────────────────────────────────────────────────────────

/// A document referenced by the project.
///
/// Natural key: `path` (relative path from the project directory).
#[derive(Debug, Clone)]
pub struct DocumentRef {
    pub path: String,
    pub unique_id: String,
    pub annotation_enabled: bool,
    pub annotate_start_value: i32,
    pub annotation_index_control_enabled: bool,
    pub annotate_suffix: String,
    pub annotate_scope: DocAnnotationScope,
    pub annotate_order: i32,
    pub do_library_update: bool,
    pub do_database_update: bool,
    pub class_gen_cc_auto_enabled: bool,
    pub class_gen_cc_auto_room_enabled: bool,
    pub class_gen_nc_auto_scope: DocAutoNetClassScope,
    pub generate_class_cluster: bool,
}

// ── BuildConfiguration ──────────────────────────────────────────────────────

/// A build configuration (sources, releases, etc.).
///
/// Natural key: `name`.
#[derive(Debug, Clone)]
pub struct BuildConfiguration {
    pub name: String,
    pub variant: String,
    pub content_type_guid: String,
    pub configuration_type: String,
    pub parameter_count: i32,
    pub constraint_file_count: i32,
    pub output_jobs_count: i32,
    pub release_item_id: String,
}

// ── OutputGroup / OutputJob ─────────────────────────────────────────────────

/// An output job group containing related outputs.
///
/// Natural key: `name`.
#[derive(Debug, Clone)]
pub struct OutputGroup {
    pub name: String,
    pub description: String,
    pub target_printer: String,
    /// Raw pipe-delimited printer options string (preserved as-is).
    pub printer_options: String,
    pub outputs: Vec<OutputJob>,
}

/// An individual output within a group.
///
/// Natural key: `name`.
#[derive(Debug, Clone)]
pub struct OutputJob {
    pub name: String,
    pub output_type: String,
    pub document_path: String,
    pub variant_name: String,
    pub is_default: bool,
    /// Raw pipe-delimited page options string (preserved as-is).
    pub page_options: Option<String>,
}

// ── Annotation ──────────────────────────────────────────────────────────────

/// Annotation settings (singleton `[Annotate]` section).
#[derive(Debug, Clone)]
pub struct AnnotationSettings {
    pub sort_order: SortOrder,
    pub sort_location: SortLocation,
    pub replace_subparts: bool,
    pub physical_naming_format: String,
    pub global_index_sort_order: SortOrder,
    pub global_index_sort_location: SortLocation,
    pub match_parameters: Vec<AnnotationMatchParameter>,
}

/// A match parameter entry within annotation settings.
#[derive(Debug, Clone)]
pub struct AnnotationMatchParameter {
    pub name: String,
    pub strict: bool,
}

// ── ClassGen ────────────────────────────────────────────────────────────────

/// Class generation settings (singleton `[PrjClassGen]` section).
#[derive(Debug, Clone)]
pub struct ClassGenSettings {
    pub comp_class_manual_enabled: bool,
    pub comp_class_manual_room_enabled: bool,
    pub net_class_auto_bus_enabled: bool,
    pub net_class_auto_comp_enabled: bool,
    pub net_class_auto_named_harness_enabled: bool,
    pub net_class_manual_enabled: bool,
    pub net_class_separate_for_bus_sections: bool,
}

// ── LibraryUpdate ───────────────────────────────────────────────────────────

/// Library update settings (singleton `[LibraryUpdateOptions]` section).
#[derive(Debug, Clone)]
pub struct LibraryUpdateSettings {
    pub selected_only: bool,
    pub update_variants: bool,
    pub update_to_latest_revision: bool,
    pub full_replace: bool,
    pub update_designator_lock: bool,
    pub update_part_id_lock: bool,
    pub preserve_parameter_locations: bool,
    pub preserve_parameter_visibility: bool,
    pub do_graphics: bool,
    pub do_parameters: bool,
    pub do_models: bool,
    pub add_parameters: bool,
    pub remove_parameters: bool,
    pub add_models: bool,
    pub remove_models: bool,
    pub update_current_models: bool,
}

// ── DatabaseUpdate ──────────────────────────────────────────────────────────

/// Database update settings (singleton `[DatabaseUpdateOptions]` section).
#[derive(Debug, Clone)]
pub struct DatabaseUpdateSettings {
    pub selected_only: bool,
    pub update_variants: bool,
    pub update_to_latest_revision: bool,
    pub part_types: i32,
}

// ── ComparisonOption ────────────────────────────────────────────────────────

/// ECO comparison option (parsed from pipe-delimited `ComparisonOptions{N}` values).
///
/// Natural key: `kind`.
#[derive(Debug, Clone)]
pub struct ComparisonOption {
    pub kind: String,
    pub min_percent: i32,
    pub min_match: i32,
    pub show_match: bool,
    /// -1=auto, 0=no, 1=yes.
    pub use_name: i32,
    pub include_all_rules: bool,
}

// ── ERC ─────────────────────────────────────────────────────────────────────

/// 17x17 ERC connection matrix.
///
/// `cells[row][col]` where row/col indices correspond to `ConnectionCode` values (0..16).
/// Serialized as `L1..L17` rows of 17 N/W/E/F characters.
#[derive(Debug, Clone)]
pub struct ErcConnectionMatrix {
    pub cells: [[ErrorLevel; 17]; 17],
}

impl Default for ErcConnectionMatrix {
    fn default() -> Self {
        Self {
            cells: [[ErrorLevel::NoReport; 17]; 17],
        }
    }
}

/// Per-error-kind ERC check level.
#[derive(Debug, Clone)]
pub struct ErcLevel {
    /// 1-based `Type{N}` key, or named key (e.g. `MultiChannelAlternate`).
    pub key: String,
    pub level: ErrorLevel,
}

/// Per-difference-kind modification level.
#[derive(Debug, Clone)]
pub struct ModificationLevel {
    /// 1-based `Type{N}` key.
    pub difference_kind_index: u16,
    pub enabled: bool,
}

/// Per-difference-kind difference check level.
#[derive(Debug, Clone)]
pub struct DifferenceLevel {
    /// 1-based `Type{N}` key.
    pub difference_kind_index: u16,
    pub level: DifferenceCheckLevel,
}

// ── Variants ────────────────────────────────────────────────────────────────

/// A project variant.
///
/// Natural key: `description` (or `unique_id`).
#[derive(Debug, Clone)]
pub struct ProjectVariant {
    pub unique_id: String,
    pub description: String,
    pub overwrite_pcb_footprint: bool,
    pub variations: Vec<ComponentVariation>,
    pub param_variations: Vec<ParameterVariation>,
}

/// A component variation within a project variant.
#[derive(Debug, Clone)]
pub struct ComponentVariation {
    pub designator: String,
    pub unique_id: String,
    pub kind: VariationKind,
    pub alternate_part: String,
}

/// A parameter variation within a project variant.
#[derive(Debug, Clone)]
pub struct ParameterVariation {
    pub designator: String,
    pub parameter_name: String,
    pub variant_value: String,
}

// ── Parameters / DiffPairs / NetInfo ────────────────────────────────────────

/// Project-level parameter.
///
/// Natural key: `name`.
#[derive(Debug, Clone)]
pub struct ProjectParameter {
    pub name: String,
    pub value: String,
}

/// Differential pair suffix.
#[derive(Debug, Clone)]
pub struct DiffPairSuffix {
    pub positive: String,
    pub negative: String,
}

/// Net color assignment.
///
/// Natural key: `net_name`.
#[derive(Debug, Clone)]
pub struct NetInfo {
    pub net_name: String,
    pub net_color: Color,
}

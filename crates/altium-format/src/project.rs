use std::path::Path;

use indexmap::IndexMap;

use crate::ResultExt;

/// Raw internal storage for a parsed `.PrjPcb` project file.
///
/// Stores all sections as ordered key-value maps, preserving insertion order
/// for roundtrip fidelity. Domain-typed interpretation is handled by the
/// read path (`api::project_read`).
pub struct AltiumProject {
    /// Project file name (stem from path).
    name: String,
    /// `[Design]` section key-value pairs.
    design: IndexMap<String, String>,
    /// `[Preferences]` section.
    preferences: IndexMap<String, String>,
    /// `[Document{N}]` sections, one per document.
    documents: Vec<IndexMap<String, String>>,
    /// `[Configuration{N}]` sections.
    configurations: Vec<IndexMap<String, String>>,
    /// `[OutputGroup{N}]` sections with nested per-output indexed keys.
    output_groups: Vec<OutputGroupRaw>,
    /// `[Modification Levels]` section.
    modification_levels: IndexMap<String, String>,
    /// `[Difference Levels]` section.
    difference_levels: IndexMap<String, String>,
    /// `[Electrical Rules Check]` section.
    erc_levels: IndexMap<String, String>,
    /// `[ERC Connection Matrix]` section (L1..L17 rows).
    erc_matrix: IndexMap<String, String>,
    /// `[Annotate]` section.
    annotate: IndexMap<String, String>,
    /// `[PrjClassGen]` section.
    class_gen: IndexMap<String, String>,
    /// `[LibraryUpdateOptions]` section.
    library_update_options: IndexMap<String, String>,
    /// `[DatabaseUpdateOptions]` section.
    database_update_options: IndexMap<String, String>,
    /// `[Comparison Options]` section.
    comparison_options: IndexMap<String, String>,
    /// `[SmartPDF]` section.
    smart_pdf: IndexMap<String, String>,
    /// `[Variant{N}]` sections.
    variants: Vec<IndexMap<String, String>>,
    /// `[Parameter{N}]` sections.
    parameters: Vec<IndexMap<String, String>>,
    /// `[DiffPairSuffix{N}]` sections.
    diff_pair_suffixes: Vec<IndexMap<String, String>>,
    /// `[Net Info]` section.
    net_infos: IndexMap<String, String>,
    /// `[UniqueIDMappings]` section.
    unique_ids_mappings: IndexMap<String, String>,
}

/// Raw storage for an output group with its nested per-output entries.
pub(crate) struct OutputGroupRaw {
    /// Group-level keys (Name, Description, TargetPrinter, PrinterOptions).
    keys: IndexMap<String, String>,
    /// Per-output entries, each containing the de-indexed keys.
    outputs: Vec<IndexMap<String, String>>,
}

impl OutputGroupRaw {
    pub(crate) fn keys(&self) -> &IndexMap<String, String> {
        &self.keys
    }

    pub(crate) fn outputs(&self) -> &[IndexMap<String, String>] {
        &self.outputs
    }
}

/// Known output-level key prefixes for `[OutputGroup{N}]` sections.
const OUTPUT_KEY_PREFIXES: &[&str] = &[
    "OutputType",
    "OutputName",
    "OutputDocumentPath",
    "OutputVariantName",
    "OutputDefault",
    "PageOptions",
];

fn split_output_key(key: &str) -> Option<(&str, usize)> {
    for prefix in OUTPUT_KEY_PREFIXES {
        if let Some(suffix) = key.strip_prefix(prefix) {
            if let Ok(idx) = suffix.parse::<usize>() {
                return Some((prefix, idx));
            }
        }
    }
    None
}

/// The current parser target section.
enum Section {
    None,
    Design,
    Preferences,
    Document,
    Configuration,
    OutputGroup,
    ModificationLevels,
    DifferenceLevels,
    ErcLevels,
    ErcMatrix,
    Annotate,
    ClassGen,
    LibraryUpdateOptions,
    DatabaseUpdateOptions,
    ComparisonOptions,
    SmartPdf,
    Variant,
    Parameter,
    DiffPairSuffix,
    NetInfo,
    UniqueIdMappings,
    Unknown,
}

/// Default INI content for a blank AD26 project (no document references).
const BLANK_AD26_INI: &str = include_str!("blank_project_ad26.ini");

impl AltiumProject {
    /// Create a new blank project with Altium Designer 26 defaults.
    ///
    /// The project contains all default sections (Design, ERC, Output Groups,
    /// etc.) but no document references — documents are added after creation.
    pub fn new_blank_ad26() -> Self {
        Self::parse("unnamed".to_owned(), BLANK_AD26_INI)
            .expect("embedded blank project content should always parse")
    }

    pub fn open(path: impl AsRef<Path>) -> crate::Result<Self> {
        let path = path.as_ref();
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_string();

        let content = std::fs::read_to_string(path)
            .map_err(crate::AltiumFormatError::Io)
            .with_context(|| format!("reading project file '{}'", path.display()))?;

        let content = content.strip_prefix('\u{FEFF}').unwrap_or(&content);

        // Need to own the stripped content before returning from this function.
        let content = content.to_owned();

        Self::parse(name, &content)
            .with_context(|| format!("parsing project file '{}'", path.display()))
    }

    fn parse(name: String, content: &str) -> crate::Result<Self> {
        let mut design = IndexMap::new();
        let mut preferences = IndexMap::new();
        let mut documents: Vec<IndexMap<String, String>> = Vec::new();
        let mut configurations: Vec<IndexMap<String, String>> = Vec::new();
        let mut output_groups: Vec<OutputGroupRaw> = Vec::new();
        let mut modification_levels = IndexMap::new();
        let mut difference_levels = IndexMap::new();
        let mut erc_levels = IndexMap::new();
        let mut erc_matrix = IndexMap::new();
        let mut annotate = IndexMap::new();
        let mut class_gen = IndexMap::new();
        let mut library_update_options = IndexMap::new();
        let mut database_update_options = IndexMap::new();
        let mut comparison_options = IndexMap::new();
        let mut smart_pdf = IndexMap::new();
        let mut variants: Vec<IndexMap<String, String>> = Vec::new();
        let mut parameters: Vec<IndexMap<String, String>> = Vec::new();
        let mut diff_pair_suffixes: Vec<IndexMap<String, String>> = Vec::new();
        let mut net_infos = IndexMap::new();
        let mut unique_ids_mappings = IndexMap::new();

        let mut current_section = Section::None;

        for raw_line in content.lines() {
            let line = raw_line.trim();

            // Skip empty lines and comment lines.
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }

            // Section header.
            if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                current_section = detect_section(header);
                match &current_section {
                    Section::Document => documents.push(IndexMap::new()),
                    Section::Configuration => configurations.push(IndexMap::new()),
                    Section::OutputGroup => output_groups.push(OutputGroupRaw {
                        keys: IndexMap::new(),
                        outputs: Vec::new(),
                    }),
                    Section::Variant => variants.push(IndexMap::new()),
                    Section::Parameter => parameters.push(IndexMap::new()),
                    Section::DiffPairSuffix => diff_pair_suffixes.push(IndexMap::new()),
                    _ => {}
                }
                continue;
            }

            // Key=Value line — split on first `=` only.
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].to_owned();
                let value = line[eq_pos + 1..].to_owned();

                match &current_section {
                    Section::None | Section::Unknown => {}
                    Section::Design => {
                        design.insert(key, value);
                    }
                    Section::Preferences => {
                        preferences.insert(key, value);
                    }
                    Section::Document => {
                        if let Some(map) = documents.last_mut() {
                            map.insert(key, value);
                        }
                    }
                    Section::Configuration => {
                        if let Some(map) = configurations.last_mut() {
                            map.insert(key, value);
                        }
                    }
                    Section::OutputGroup => {
                        if let Some(group) = output_groups.last_mut() {
                            if let Some((prefix, idx)) = split_output_key(&key) {
                                // Indices are 1-based in the file; convert to 0-based.
                                let slot = idx.saturating_sub(1);
                                if group.outputs.len() <= slot {
                                    group.outputs.resize_with(slot + 1, IndexMap::new);
                                }
                                group.outputs[slot].insert(prefix.to_owned(), value);
                            } else {
                                group.keys.insert(key, value);
                            }
                        }
                    }
                    Section::ModificationLevels => {
                        modification_levels.insert(key, value);
                    }
                    Section::DifferenceLevels => {
                        difference_levels.insert(key, value);
                    }
                    Section::ErcLevels => {
                        erc_levels.insert(key, value);
                    }
                    Section::ErcMatrix => {
                        erc_matrix.insert(key, value);
                    }
                    Section::Annotate => {
                        annotate.insert(key, value);
                    }
                    Section::ClassGen => {
                        class_gen.insert(key, value);
                    }
                    Section::LibraryUpdateOptions => {
                        library_update_options.insert(key, value);
                    }
                    Section::DatabaseUpdateOptions => {
                        database_update_options.insert(key, value);
                    }
                    Section::ComparisonOptions => {
                        comparison_options.insert(key, value);
                    }
                    Section::SmartPdf => {
                        smart_pdf.insert(key, value);
                    }
                    Section::Variant => {
                        if let Some(map) = variants.last_mut() {
                            map.insert(key, value);
                        }
                    }
                    Section::Parameter => {
                        if let Some(map) = parameters.last_mut() {
                            map.insert(key, value);
                        }
                    }
                    Section::DiffPairSuffix => {
                        if let Some(map) = diff_pair_suffixes.last_mut() {
                            map.insert(key, value);
                        }
                    }
                    Section::NetInfo => {
                        net_infos.insert(key, value);
                    }
                    Section::UniqueIdMappings => {
                        unique_ids_mappings.insert(key, value);
                    }
                }
            }
            // Lines with no `=` and not a section header are silently ignored
            // (not valid INI key-value pairs).
        }

        Ok(AltiumProject {
            name,
            design,
            preferences,
            documents,
            configurations,
            output_groups,
            modification_levels,
            difference_levels,
            erc_levels,
            erc_matrix,
            annotate,
            class_gen,
            library_update_options,
            database_update_options,
            comparison_options,
            smart_pdf,
            variants,
            parameters,
            diff_pair_suffixes,
            net_infos,
            unique_ids_mappings,
        })
    }
}

fn detect_section(header: &str) -> Section {
    // Exact matches first.
    match header {
        "Design" => return Section::Design,
        "Preferences" => return Section::Preferences,
        "Modification Levels" => return Section::ModificationLevels,
        "Difference Levels" => return Section::DifferenceLevels,
        "Electrical Rules Check" => return Section::ErcLevels,
        "ERC Connection Matrix" => return Section::ErcMatrix,
        "Annotate" => return Section::Annotate,
        "PrjClassGen" => return Section::ClassGen,
        "LibraryUpdateOptions" => return Section::LibraryUpdateOptions,
        "DatabaseUpdateOptions" => return Section::DatabaseUpdateOptions,
        "Comparison Options" => return Section::ComparisonOptions,
        "SmartPDF" => return Section::SmartPdf,
        "Net Info" => return Section::NetInfo,
        "UniqueIDMappings" => return Section::UniqueIdMappings,
        _ => {}
    }

    // Prefix + numeric suffix matches.
    if is_numbered_prefix(header, "Document") {
        return Section::Document;
    }
    if is_numbered_prefix(header, "Configuration") {
        return Section::Configuration;
    }
    if is_numbered_prefix(header, "OutputGroup") {
        return Section::OutputGroup;
    }
    if is_numbered_prefix(header, "Variant") {
        return Section::Variant;
    }
    if is_numbered_prefix(header, "Parameter") {
        return Section::Parameter;
    }
    if is_numbered_prefix(header, "DiffPairSuffix") {
        return Section::DiffPairSuffix;
    }

    Section::Unknown
}

fn is_numbered_prefix(header: &str, prefix: &str) -> bool {
    if let Some(suffix) = header.strip_prefix(prefix) {
        !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit())
    } else {
        false
    }
}

impl AltiumProject {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn design(&self) -> &IndexMap<String, String> {
        &self.design
    }

    pub(crate) fn preferences(&self) -> &IndexMap<String, String> {
        &self.preferences
    }

    pub(crate) fn documents(&self) -> &[IndexMap<String, String>] {
        &self.documents
    }

    pub(crate) fn configurations(&self) -> &[IndexMap<String, String>] {
        &self.configurations
    }

    pub(crate) fn output_groups(&self) -> &[OutputGroupRaw] {
        &self.output_groups
    }

    pub(crate) fn modification_levels(&self) -> &IndexMap<String, String> {
        &self.modification_levels
    }

    pub(crate) fn difference_levels(&self) -> &IndexMap<String, String> {
        &self.difference_levels
    }

    pub(crate) fn erc_levels(&self) -> &IndexMap<String, String> {
        &self.erc_levels
    }

    pub(crate) fn erc_matrix(&self) -> &IndexMap<String, String> {
        &self.erc_matrix
    }

    pub(crate) fn annotate(&self) -> &IndexMap<String, String> {
        &self.annotate
    }

    pub(crate) fn class_gen(&self) -> &IndexMap<String, String> {
        &self.class_gen
    }

    pub(crate) fn library_update_options(&self) -> &IndexMap<String, String> {
        &self.library_update_options
    }

    pub(crate) fn database_update_options(&self) -> &IndexMap<String, String> {
        &self.database_update_options
    }

    pub(crate) fn comparison_options(&self) -> &IndexMap<String, String> {
        &self.comparison_options
    }

    pub(crate) fn smart_pdf(&self) -> &IndexMap<String, String> {
        &self.smart_pdf
    }

    pub(crate) fn variants(&self) -> &[IndexMap<String, String>] {
        &self.variants
    }

    pub(crate) fn parameters_sections(&self) -> &[IndexMap<String, String>] {
        &self.parameters
    }

    pub(crate) fn diff_pair_suffixes(&self) -> &[IndexMap<String, String>] {
        &self.diff_pair_suffixes
    }

    pub(crate) fn net_infos(&self) -> &IndexMap<String, String> {
        &self.net_infos
    }

    pub(crate) fn unique_ids_mappings(&self) -> &IndexMap<String, String> {
        &self.unique_ids_mappings
    }

    /// Mutable access to the `[Design]` section for writing back spec values.
    pub fn design_mut(&mut self) -> &mut IndexMap<String, String> {
        &mut self.design
    }

    /// Mutable access to the `[ERC Connection Matrix]` section.
    pub fn erc_matrix_mut(&mut self) -> &mut IndexMap<String, String> {
        &mut self.erc_matrix
    }

    /// Mutable access to the `[Electrical Rules Check]` section.
    pub fn erc_levels_mut(&mut self) -> &mut IndexMap<String, String> {
        &mut self.erc_levels
    }

    /// Save the project to an INI file with UTF-8 BOM prefix.
    pub fn save(&self, path: impl AsRef<Path>) -> crate::Result<()> {
        let path = path.as_ref();
        let ini = crate::api::project_write::write_ini(self);
        // Write UTF-8 BOM + content
        let content = format!("\u{FEFF}{}", ini);
        std::fs::write(path, content)
            .map_err(crate::AltiumFormatError::Io)
            .with_context(|| format!("saving project file '{}'", path.display()))
    }

    /// Convert internal storage to a typed `Project` API value.
    pub fn project(&self) -> crate::Result<crate::api::Project> {
        crate::api::project_read::project_from_internal(self)
    }

    /// List document paths referenced in the project.
    pub fn document_paths(&self) -> Vec<&str> {
        self.documents
            .iter()
            .filter_map(|d| d.get("DocumentPath").map(|s| s.as_str()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_ini() {
        let input = "[Design]\nVersion=1.0\nHierarchyMode=0\n\n[Preferences]\nPrefsVaultGUID=\n";
        let project = AltiumProject::parse("TestProject".to_owned(), input).unwrap();

        assert_eq!(project.name(), "TestProject");
        assert_eq!(project.design().get("Version").map(|s| s.as_str()), Some("1.0"));
        assert_eq!(project.design().get("HierarchyMode").map(|s| s.as_str()), Some("0"));
        assert_eq!(
            project.preferences().get("PrefsVaultGUID").map(|s| s.as_str()),
            Some("")
        );
    }

    #[test]
    fn test_bom_stripping() {
        let bom = "\u{FEFF}";
        let input = format!("{}[Design]\nVersion=2.0\n", bom);
        let content = input.strip_prefix('\u{FEFF}').unwrap_or(&input);
        let project = AltiumProject::parse("BomTest".to_owned(), content).unwrap();
        assert_eq!(project.design().get("Version").map(|s| s.as_str()), Some("2.0"));
    }

    #[test]
    fn test_split_output_key() {
        assert_eq!(split_output_key("OutputType1"), Some(("OutputType", 1)));
        assert_eq!(split_output_key("OutputName21"), Some(("OutputName", 21)));
        assert_eq!(split_output_key("PageOptions3"), Some(("PageOptions", 3)));
        assert_eq!(split_output_key("OutputDefault10"), Some(("OutputDefault", 10)));
        assert_eq!(split_output_key("OutputVariantName5"), Some(("OutputVariantName", 5)));
        assert_eq!(split_output_key("OutputDocumentPath2"), Some(("OutputDocumentPath", 2)));
        // Non-output keys return None.
        assert_eq!(split_output_key("Name"), None);
        assert_eq!(split_output_key("PrinterOptions"), None);
        assert_eq!(split_output_key("Description"), None);
        // Prefix without a numeric suffix returns None.
        assert_eq!(split_output_key("OutputType"), None);
    }

    #[test]
    fn test_document_paths() {
        let input = "[Document1]\nDocumentPath=Schematic.SchLib\n\n[Document2]\nDocumentPath=Board.PcbLib\n";
        let project = AltiumProject::parse("DocPaths".to_owned(), input).unwrap();
        let paths = project.document_paths();
        assert_eq!(paths, vec!["Schematic.SchLib", "Board.PcbLib"]);
    }

    #[test]
    fn test_output_group_indexed_keys() {
        let input = "[OutputGroup1]\nName=Netlist Outputs\nDescription=\nOutputType1=CadnetixNetlist\nOutputName1=Cadnetix Netlist\nOutputDefault1=0\nOutputType2=CalayNetlist\nOutputName2=Calay Netlist\nOutputDefault2=0\n";
        let project = AltiumProject::parse("OGTest".to_owned(), input).unwrap();
        let groups = project.output_groups();
        assert_eq!(groups.len(), 1);
        let g = &groups[0];
        assert_eq!(g.keys().get("Name").map(|s| s.as_str()), Some("Netlist Outputs"));
        assert_eq!(g.keys().get("Description").map(|s| s.as_str()), Some(""));
        assert_eq!(g.outputs().len(), 2);
        assert_eq!(
            g.outputs()[0].get("OutputType").map(|s| s.as_str()),
            Some("CadnetixNetlist")
        );
        assert_eq!(
            g.outputs()[1].get("OutputName").map(|s| s.as_str()),
            Some("Calay Netlist")
        );
    }

    #[test]
    fn test_comment_and_empty_lines_skipped() {
        let input = "\n; this is a comment\n# another comment\n[Design]\nVersion=1.0\n\n; ignore me\nHierarchyMode=0\n";
        let project = AltiumProject::parse("CommentTest".to_owned(), input).unwrap();
        assert_eq!(project.design().len(), 2);
    }

    #[test]
    fn test_value_with_equals_sign() {
        let input = "[Design]\nFoo=a=b=c\n";
        let project = AltiumProject::parse("EqTest".to_owned(), input).unwrap();
        assert_eq!(project.design().get("Foo").map(|s| s.as_str()), Some("a=b=c"));
    }

    #[test]
    fn test_unknown_section_ignored() {
        let input = "[UnknownFutureSection]\nKey=Value\n[Design]\nVersion=1.0\n";
        let project = AltiumProject::parse("UnkTest".to_owned(), input).unwrap();
        assert_eq!(project.design().get("Version").map(|s| s.as_str()), Some("1.0"));
    }

    #[test]
    fn new_blank_ad26_roundtrip() {
        let proj = AltiumProject::new_blank_ad26();

        // Verify key structural properties of the blank project.
        assert_eq!(proj.design().get("Version").map(|s| s.as_str()), Some("1.0"));
        assert_eq!(proj.documents().len(), 0, "blank project should have no documents");
        assert_eq!(proj.configurations().len(), 1);
        assert_eq!(proj.output_groups().len(), 10);
        assert_eq!(proj.erc_matrix().len(), 17, "ERC matrix should have 17 rows");
        assert!(!proj.modification_levels().is_empty());
        assert!(!proj.difference_levels().is_empty());
        assert!(!proj.erc_levels().is_empty());

        // Save to a temp file and reopen.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        proj.save(tmp.path()).unwrap();

        let reopened = AltiumProject::open(tmp.path()).unwrap();

        // Verify the reopened project matches the original.
        assert_eq!(reopened.design().len(), proj.design().len());
        assert_eq!(reopened.documents().len(), 0);
        assert_eq!(reopened.configurations().len(), 1);
        assert_eq!(reopened.output_groups().len(), 10);
        assert_eq!(reopened.erc_matrix().len(), 17);
        assert_eq!(reopened.modification_levels().len(), proj.modification_levels().len());
        assert_eq!(reopened.difference_levels().len(), proj.difference_levels().len());
        assert_eq!(reopened.erc_levels().len(), proj.erc_levels().len());

        // Verify save→reopen produces identical INI content.
        let ini1 = crate::api::project_write::write_ini(&proj);
        let ini2 = crate::api::project_write::write_ini(&reopened);
        assert_eq!(ini1, ini2, "roundtrip INI content should be identical");
    }

    #[cfg(feature = "test-fixtures")]
    #[test]
    fn test_open_blank_project() {
        let project = AltiumProject::open("data/BlankProject.PrjPcb").unwrap();
        assert_eq!(project.name(), "BlankProject");
        assert_eq!(
            project.design().get("Version").map(|s| s.as_str()),
            Some("1.0")
        );
        let paths = project.document_paths();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"BlankSchlibComponent.SchLib"));
        assert!(paths.contains(&"BlankPcbLibComponent.PcbLib"));
        assert!(project.output_groups().len() >= 10);
        assert!(!project.erc_matrix().is_empty());
    }
}

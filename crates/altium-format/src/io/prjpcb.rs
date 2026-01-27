//! PrjPcb reader/writer for Altium project files.
//!
//! Altium project files (.PrjPcb) are INI-style text files that define:
//! - Project metadata and settings
//! - List of documents (schematics, PCB, libraries)
//! - ERC connection matrix settings
//! - Output configurations
//! - Project parameters

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

use crate::dump::{DumpTree, TreeBuilder};
use crate::error::{AltiumError, Result};

/// A project document reference.
#[derive(Debug, Clone, Default)]
pub struct ProjectDocument {
    /// Path to the document (relative to project)
    pub path: String,
    /// Document type inferred from extension
    pub doc_type: DocumentType,
    /// Whether annotation is enabled
    pub annotation_enabled: bool,
    /// Annotation start value
    pub annotation_start_value: i32,
    /// Whether to do library update
    pub do_library_update: bool,
    /// Whether to do database update
    pub do_database_update: bool,
    /// All parameters for this document
    pub params: HashMap<String, String>,
}

/// Document type based on file extension.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum DocumentType {
    /// Schematic document (.SchDoc)
    Schematic,
    /// PCB document (.PcbDoc)
    Pcb,
    /// Schematic library (.SchLib)
    SchLib,
    /// PCB library (.PcbLib)
    PcbLib,
    /// Integrated library (.IntLib)
    IntLib,
    /// Output job (.OutJob)
    OutputJob,
    /// Other/unknown type
    #[default]
    Other,
}

impl DocumentType {
    /// Infer document type from file path.
    pub fn from_path(path: &str) -> Self {
        let lower = path.to_lowercase();
        if lower.ends_with(".schdoc") {
            DocumentType::Schematic
        } else if lower.ends_with(".pcbdoc") {
            DocumentType::Pcb
        } else if lower.ends_with(".schlib") {
            DocumentType::SchLib
        } else if lower.ends_with(".pcblib") {
            DocumentType::PcbLib
        } else if lower.ends_with(".intlib") {
            DocumentType::IntLib
        } else if lower.ends_with(".outjob") {
            DocumentType::OutputJob
        } else {
            DocumentType::Other
        }
    }

    /// Get the display name for this document type.
    pub fn display_name(&self) -> &'static str {
        match self {
            DocumentType::Schematic => "Schematic",
            DocumentType::Pcb => "PCB",
            DocumentType::SchLib => "Schematic Library",
            DocumentType::PcbLib => "PCB Library",
            DocumentType::IntLib => "Integrated Library",
            DocumentType::OutputJob => "Output Job",
            DocumentType::Other => "Other",
        }
    }
}

impl std::fmt::Display for DocumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// A project parameter.
#[derive(Debug, Clone, Default)]
pub struct ProjectParameter {
    /// Parameter name
    pub name: String,
    /// Parameter value
    pub value: String,
}

/// ERC (Electrical Rule Check) connection matrix.
#[derive(Debug, Clone, Default)]
pub struct ErcMatrix {
    /// Matrix rows (L1-L17), each containing violation levels
    pub rows: Vec<String>,
}

impl ErcMatrix {
    /// Get the violation level between two pin types.
    pub fn get_level(&self, row: usize, col: usize) -> Option<char> {
        if row < self.rows.len() {
            self.rows[row].chars().nth(col)
        } else {
            None
        }
    }

    /// Decode ERC level character to description.
    pub fn decode_level(c: char) -> &'static str {
        match c {
            'N' => "No Report",
            'W' => "Warning",
            'E' => "Error",
            'A' => "ActiveLow Warning",
            'B' => "Bidirectional",
            'O' => "Open",
            'R' => "Report",
            _ => "Unknown",
        }
    }
}

/// Project variant for multi-variant designs.
#[derive(Debug, Clone, Default)]
pub struct ProjectVariant {
    /// Variant name
    pub name: String,
    /// Variant description
    pub description: String,
    /// Parameter overrides for this variant
    pub parameter_overrides: HashMap<String, String>,
}

/// Output group configuration.
#[derive(Debug, Clone, Default)]
pub struct OutputGroup {
    /// Output group name
    pub name: String,
    /// Output type
    pub output_type: String,
    /// Settings
    pub settings: HashMap<String, String>,
}

/// An Altium project file.
#[derive(Debug, Clone, Default)]
pub struct PrjPcb {
    /// Project file path (if loaded from file)
    pub path: Option<PathBuf>,
    /// Project version
    pub version: String,
    /// Hierarchy mode (0 = flat, 1 = hierarchical)
    pub hierarchy_mode: i32,
    /// Output path
    pub output_path: String,
    /// Annotation start value
    pub annotation_start_value: i32,
    /// Documents in the project
    pub documents: Vec<ProjectDocument>,
    /// Project parameters
    pub parameters: HashMap<String, String>,
    /// ERC connection matrix
    pub erc_matrix: ErcMatrix,
    /// Project variants
    pub variants: Vec<ProjectVariant>,
    /// Output groups
    pub output_groups: Vec<OutputGroup>,
    /// All raw sections (for round-trip preservation)
    pub sections: HashMap<String, HashMap<String, String>>,
}

impl PrjPcb {
    /// Create a new empty project.
    pub fn new() -> Self {
        PrjPcb {
            version: "1.0".to_string(),
            ..Default::default()
        }
    }

    /// Open and read a PrjPcb file.
    pub fn open<R: Read>(reader: R) -> Result<Self> {
        let buf_reader = BufReader::new(reader);
        let mut prj = PrjPcb::default();

        let mut current_section = String::new();
        let mut current_section_data: HashMap<String, String> = HashMap::new();

        for line_result in buf_reader.lines() {
            let line = line_result.map_err(AltiumError::Io)?;
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }

            // Check for section header
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                // Save previous section
                if !current_section.is_empty() {
                    prj.process_section(&current_section, &current_section_data);
                    prj.sections
                        .insert(current_section.clone(), current_section_data.clone());
                }

                // Start new section
                current_section = trimmed[1..trimmed.len() - 1].to_string();
                current_section_data = HashMap::new();
            } else if let Some(eq_pos) = trimmed.find('=') {
                // Key=Value pair
                let key = trimmed[..eq_pos].to_string();
                let value = trimmed[eq_pos + 1..].to_string();
                current_section_data.insert(key, value);
            }
        }

        // Process last section
        if !current_section.is_empty() {
            prj.process_section(&current_section, &current_section_data);
            prj.sections.insert(current_section, current_section_data);
        }

        Ok(prj)
    }

    /// Open and read a PrjPcb file from a path.
    pub fn open_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        let file = File::open(path_ref)?;
        let mut prj = Self::open(file)?;
        prj.path = Some(path_ref.to_path_buf());
        Ok(prj)
    }

    /// Process a section and populate appropriate fields.
    fn process_section(&mut self, section: &str, data: &HashMap<String, String>) {
        match section {
            "Design" => self.process_design_section(data),
            "Parameters" => {
                self.parameters = data.clone();
            }
            "ERC Connection Matrix" => self.process_erc_section(data),
            _ => {
                // Check for DocumentN sections
                if let Some(suffix) = section.strip_prefix("Document") {
                    if suffix.parse::<i32>().is_ok() {
                        self.process_document_section(data);
                    }
                }
                // Check for OutputGroupN sections
                if let Some(suffix) = section.strip_prefix("OutputGroup") {
                    if suffix.parse::<i32>().is_ok() {
                        self.process_output_group_section(section, data);
                    }
                }
            }
        }
    }

    /// Process the [Design] section.
    fn process_design_section(&mut self, data: &HashMap<String, String>) {
        if let Some(v) = data.get("Version") {
            self.version = v.clone();
        }
        if let Some(v) = data.get("HierarchyMode") {
            self.hierarchy_mode = v.parse().unwrap_or(0);
        }
        if let Some(v) = data.get("OutputPath") {
            self.output_path = v.clone();
        }
        if let Some(v) = data.get("AnnotationStartValue") {
            self.annotation_start_value = v.parse().unwrap_or(1);
        }
    }

    /// Process a [DocumentN] section.
    fn process_document_section(&mut self, data: &HashMap<String, String>) {
        let path = data.get("DocumentPath").cloned().unwrap_or_default();
        if path.is_empty() {
            return;
        }

        let doc = ProjectDocument {
            doc_type: DocumentType::from_path(&path),
            path,
            annotation_enabled: data
                .get("AnnotationEnabled")
                .map(|v| v == "1")
                .unwrap_or(true),
            annotation_start_value: data
                .get("AnnotateStartValue")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
            do_library_update: data
                .get("DoLibraryUpdate")
                .map(|v| v == "1")
                .unwrap_or(true),
            do_database_update: data
                .get("DoDatabaseUpdate")
                .map(|v| v == "1")
                .unwrap_or(true),
            params: data.clone(),
        };

        self.documents.push(doc);
    }

    /// Process the [ERC Connection Matrix] section.
    fn process_erc_section(&mut self, data: &HashMap<String, String>) {
        let mut rows = Vec::new();
        for i in 1..=17 {
            let key = format!("L{}", i);
            if let Some(v) = data.get(&key) {
                rows.push(v.clone());
            }
        }
        self.erc_matrix.rows = rows;
    }

    /// Process an [OutputGroupN] section.
    fn process_output_group_section(&mut self, section: &str, data: &HashMap<String, String>) {
        let group = OutputGroup {
            name: data
                .get("Name")
                .cloned()
                .unwrap_or_else(|| section.to_string()),
            output_type: data.get("OutputType").cloned().unwrap_or_default(),
            settings: data.clone(),
        };
        self.output_groups.push(group);
    }

    /// Save the project to a writer.
    pub fn save<W: Write>(&self, mut writer: W) -> Result<()> {
        // [Design] section
        writeln!(writer, "[Design]")?;
        writeln!(writer, "Version={}", self.version)?;
        writeln!(writer, "HierarchyMode={}", self.hierarchy_mode)?;
        writeln!(writer, "ChannelRoomNamingStyle=0")?;
        writeln!(writer, "OutputPath={}", self.output_path)?;
        writeln!(writer, "LogFolderPath=")?;
        writeln!(
            writer,
            "AnnotationStartValue={}",
            self.annotation_start_value
        )?;
        writeln!(writer, "OpenOutputs=1")?;
        writeln!(writer, "ArchiveProject=0")?;
        writeln!(writer, "TimestampOutput=0")?;
        writeln!(writer, "ManagedProjectGuid=")?;
        writeln!(writer, "Variants=")?;
        writeln!(writer)?;

        // Document sections
        for (i, doc) in self.documents.iter().enumerate() {
            writeln!(writer, "[Document{}]", i + 1)?;
            writeln!(writer, "DocumentPath={}", doc.path)?;
            writeln!(
                writer,
                "AnnotationEnabled={}",
                if doc.annotation_enabled { "1" } else { "0" }
            )?;
            writeln!(writer, "AnnotateStartValue={}", doc.annotation_start_value)?;
            writeln!(writer, "AnnotationIndexControlEnabled=0")?;
            writeln!(writer, "AnnotateSuffix=")?;
            writeln!(writer, "AnnotateScope=0")?;
            writeln!(writer, "AnnotateOrder=-1")?;
            writeln!(
                writer,
                "DoLibraryUpdate={}",
                if doc.do_library_update { "1" } else { "0" }
            )?;
            writeln!(
                writer,
                "DoDatabaseUpdate={}",
                if doc.do_database_update { "1" } else { "0" }
            )?;
            writeln!(writer, "ClassGenCCAutoEnabled=1")?;
            writeln!(writer, "ClassGenCCAutoRoomEnabled=1")?;
            writeln!(writer, "ClassGenNCAutoScope=0")?;
            writeln!(writer, "DItemRevisionGUID=")?;
            writeln!(writer)?;
        }

        // [GeneratedDocuments]
        writeln!(writer, "[GeneratedDocuments]")?;
        writeln!(writer)?;

        // [ProjectVariantGroups]
        writeln!(writer, "[ProjectVariantGroups]")?;
        writeln!(writer)?;

        // [ERC Connection Matrix]
        writeln!(writer, "[ERC Connection Matrix]")?;
        let default_erc = vec![
            "NNNNNNNNNNNWNNNWW",
            "NNWNNNNWNWNWNWNWN",
            "NWEABOROBWBWRORNB",
            "NNAABOROBWBWBORNB",
            "NNBBNNBNNWNWBNNNN",
            "NNOOOROOONNWOONOO",
            "NNRBBNRNNWNWRBNRN",
            "NWOOOOOOOWOWNOOOO",
            "NNBBNNBNNWNWBNNNN",
            "NWWWWNWWWNWWWWNWW",
            "WBBBBOBBBBBWBBBBB",
            "NWWWWNWWWNWWWWNWW",
            "WWRRRORRRWRWRRNRR",
            "NNBBNNBNNWNWBNNNN",
            "NNOOOROOONNWOONOO",
            "WWNRNNRNRWRWRNRRR",
            "WNNNNNNNNNBNNNNRN",
        ];
        let rows = if self.erc_matrix.rows.is_empty() {
            &default_erc
        } else {
            &self
                .erc_matrix
                .rows
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
        };
        for (i, row) in rows.iter().enumerate() {
            writeln!(writer, "L{}={}", i + 1, row)?;
        }
        writeln!(writer)?;

        // [ProjectOptions]
        writeln!(writer, "[ProjectOptions]")?;
        writeln!(writer, "IncludeDesignatorInPinUniqueIDNumber=0")?;
        writeln!(writer, "IncludePartNumberInPinUniqueIDNumber=0")?;
        writeln!(writer, "EnableConstraintManager=0")?;
        writeln!(writer, "ComponentNamingScheme=0")?;
        writeln!(writer, "PadNamingScheme=0")?;
        writeln!(writer, "ComponentAutoZoom=1")?;
        writeln!(writer, "ShowSheetNumberInSheetSymbolPad=0")?;
        writeln!(writer, "OpenSchematicInServerForce=0")?;
        writeln!(writer, "LocalCompilerGUID=")?;
        writeln!(writer)?;

        // [Parameters]
        writeln!(writer, "[Parameters]")?;
        for (key, value) in &self.parameters {
            writeln!(writer, "{}={}", key, value)?;
        }
        writeln!(writer)?;

        // [Workspace]
        writeln!(writer, "[Workspace]")?;
        writeln!(writer)?;

        // [Configuration Constraints]
        writeln!(writer, "[Configuration Constraints]")?;
        writeln!(writer)?;

        Ok(())
    }

    /// Save the project to a file path.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        self.save(file)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DOCUMENT MANAGEMENT
    // ═══════════════════════════════════════════════════════════════════════════

    /// Add a document to the project.
    pub fn add_document(&mut self, path: impl Into<String>) -> &mut ProjectDocument {
        let path_str = path.into();
        let doc = ProjectDocument {
            doc_type: DocumentType::from_path(&path_str),
            path: path_str,
            annotation_enabled: true,
            annotation_start_value: 1,
            do_library_update: true,
            do_database_update: true,
            params: HashMap::new(),
        };
        self.documents.push(doc);
        self.documents.last_mut().unwrap()
    }

    /// Remove a document from the project by path.
    pub fn remove_document(&mut self, path: &str) -> bool {
        let original_len = self.documents.len();
        self.documents.retain(|d| d.path != path);
        self.documents.len() != original_len
    }

    /// Get a document by path.
    pub fn get_document(&self, path: &str) -> Option<&ProjectDocument> {
        self.documents.iter().find(|d| d.path == path)
    }

    /// Get a mutable document by path.
    pub fn get_document_mut(&mut self, path: &str) -> Option<&mut ProjectDocument> {
        self.documents.iter_mut().find(|d| d.path == path)
    }

    /// Get all schematic documents.
    pub fn schematics(&self) -> Vec<&ProjectDocument> {
        self.documents
            .iter()
            .filter(|d| d.doc_type == DocumentType::Schematic)
            .collect()
    }

    /// Get all PCB documents.
    pub fn pcb_documents(&self) -> Vec<&ProjectDocument> {
        self.documents
            .iter()
            .filter(|d| d.doc_type == DocumentType::Pcb)
            .collect()
    }

    /// Get the primary PCB document (first one found).
    pub fn primary_pcb(&self) -> Option<&ProjectDocument> {
        self.documents
            .iter()
            .find(|d| d.doc_type == DocumentType::Pcb)
    }

    /// Get the project name from parameters or file name.
    pub fn name(&self) -> String {
        if let Some(name) = self.parameters.get("Name") {
            name.clone()
        } else if let Some(ref path) = self.path {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unnamed")
                .to_string()
        } else {
            "Unnamed".to_string()
        }
    }

    /// Set the project name.
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.parameters.insert("Name".to_string(), name.into());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PARAMETER MANAGEMENT
    // ═══════════════════════════════════════════════════════════════════════════

    /// Get a project parameter.
    pub fn get_parameter(&self, key: &str) -> Option<&String> {
        self.parameters.get(key)
    }

    /// Set a project parameter.
    pub fn set_parameter(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.parameters.insert(key.into(), value.into());
    }

    /// Remove a project parameter.
    pub fn remove_parameter(&mut self, key: &str) -> Option<String> {
        self.parameters.remove(key)
    }
}

impl DumpTree for PrjPcb {
    fn dump(&self, tree: &mut TreeBuilder) {
        tree.root(&format!(
            "Project: {} ({} documents)",
            self.name(),
            self.documents.len()
        ));

        // Info section
        tree.push(!self.documents.is_empty());
        let info_props = vec![
            ("version", self.version.clone()),
            (
                "hierarchy",
                if self.hierarchy_mode == 0 {
                    "Flat".to_string()
                } else {
                    "Hierarchical".to_string()
                },
            ),
            ("output_path", self.output_path.clone()),
        ];
        tree.add_leaf("Info", &info_props);
        tree.pop();

        // Documents section
        if !self.documents.is_empty() {
            tree.push(!self.parameters.is_empty());
            tree.begin_node(&format!("Documents ({})", self.documents.len()));
            for (i, doc) in self.documents.iter().enumerate() {
                tree.push(i < self.documents.len() - 1);
                let doc_props = vec![
                    ("type", doc.doc_type.display_name().to_string()),
                    (
                        "annotation",
                        if doc.annotation_enabled {
                            "enabled".to_string()
                        } else {
                            "disabled".to_string()
                        },
                    ),
                ];
                tree.add_leaf(&doc.path, &doc_props);
                tree.pop();
            }
            tree.pop();
        }

        // Parameters section
        if !self.parameters.is_empty() {
            tree.push(false);
            tree.begin_node(&format!("Parameters ({})", self.parameters.len()));
            let mut params: Vec<_> = self.parameters.iter().collect();
            params.sort_by_key(|(k, _)| k.as_str());
            for (i, (key, value)) in params.iter().enumerate() {
                tree.push(i < params.len() - 1);
                tree.add_leaf(key, &[("value", value.to_string())]);
                tree.pop();
            }
            tree.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_type_from_path() {
        assert_eq!(
            DocumentType::from_path("Sheet1.SchDoc"),
            DocumentType::Schematic
        );
        assert_eq!(DocumentType::from_path("PCB1.PcbDoc"), DocumentType::Pcb);
        assert_eq!(
            DocumentType::from_path("Library.SchLib"),
            DocumentType::SchLib
        );
        assert_eq!(
            DocumentType::from_path("Library.PcbLib"),
            DocumentType::PcbLib
        );
        assert_eq!(DocumentType::from_path("test.txt"), DocumentType::Other);
    }

    #[test]
    fn test_parse_project() {
        let content = r#"[Design]
Version=1.0
HierarchyMode=0
OutputPath=Project Outputs\
AnnotationStartValue=1

[Document1]
DocumentPath=Sheet1.SchDoc
AnnotationEnabled=1

[Document2]
DocumentPath=PCB1.PcbDoc
AnnotationEnabled=1

[Parameters]
Name=TestProject
"#;

        let prj = PrjPcb::open(content.as_bytes()).unwrap();
        assert_eq!(prj.version, "1.0");
        assert_eq!(prj.hierarchy_mode, 0);
        assert_eq!(prj.documents.len(), 2);
        assert_eq!(prj.documents[0].path, "Sheet1.SchDoc");
        assert_eq!(prj.documents[0].doc_type, DocumentType::Schematic);
        assert_eq!(prj.documents[1].path, "PCB1.PcbDoc");
        assert_eq!(prj.documents[1].doc_type, DocumentType::Pcb);
        assert_eq!(prj.name(), "TestProject");
    }
}

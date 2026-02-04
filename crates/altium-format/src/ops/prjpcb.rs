// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Project file operations.
//!
//! Provides high-level operations for exploring Altium project (.PrjPcb) files.
//! Mirrors the schdoc/schlib/pcblib module patterns to maintain consistency
//! across the codebase.
//!
//! **Key Feature**: Cross-document loading is handled gracefully. Missing
//! referenced documents produce warnings in output, not crashes. Option<T>
//! is used for data from potentially missing files.

// cmd_* functions mix presentation and business logic; separation punted until
// usage patterns clarify abstraction boundaries (premature abstraction risk)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::io::prjpcb::{DocumentType, PrjPcb};
use crate::ops::output::*;

// ===========================================================================
// HELPER FUNCTIONS
// ===========================================================================

/// Opens and parses a PrjPcb file from the given path.
/// Returns the parsed PrjPcb structure or an error if the file cannot be read.
pub fn open_prjpcb(path: &Path) -> Result<PrjPcb, Box<dyn std::error::Error>> {
    PrjPcb::open_file(path).map_err(|e| e.to_string().into())
}

/// Resolves a document path relative to the project directory.
///
/// Project files store document paths relative to the project file location.
/// This function resolves those paths to absolute paths that can be used
/// to open the referenced files.
///
/// # Arguments
/// * `project_dir` - The directory containing the project file
/// * `doc_path` - The relative path from the project file
///
/// # Returns
/// The resolved absolute path to the document
pub fn resolve_document_path(project_dir: &Path, doc_path: &str) -> PathBuf {
    // Handle Windows-style path separators
    let normalized_path = doc_path.replace('\\', "/");
    project_dir.join(normalized_path)
}

/// Converts hierarchy mode integer to human-readable string.
fn hierarchy_mode_to_string(mode: i32) -> String {
    match mode {
        0 => "Flat".to_string(),
        1 => "Hierarchical".to_string(),
        _ => format!("Unknown ({})", mode),
    }
}

/// Sorts strings with embedded numbers naturally (e.g., "Sheet2" < "Sheet10").
///
/// TODO: Consolidate with schlib/schdoc/pcblib::alphanumeric_sort after all 4 ops modules exist
/// to validate the pattern before abstracting.
fn alphanumeric_sort(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(&ac), Some(&bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    // Extract and compare numbers
                    let a_num: String = a_chars
                        .by_ref()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    let b_num: String = b_chars
                        .by_ref()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    let a_val: u64 = a_num.parse().unwrap_or(0);
                    let b_val: u64 = b_num.parse().unwrap_or(0);
                    match a_val.cmp(&b_val) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    a_chars.next();
                    b_chars.next();
                    match ac.cmp(&bc) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }
            }
        }
    }
}

/// Get the project directory from the project file path.
fn get_project_dir(project_path: &Path) -> PathBuf {
    project_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Check if a document exists at the resolved path.
#[allow(dead_code)]
fn check_document_exists(project_dir: &Path, doc_path: &str) -> bool {
    resolve_document_path(project_dir, doc_path).exists()
}

// ===========================================================================
// BROWSE COMMANDS
// ===========================================================================

/// Returns project overview with statistics and document counts.
///
/// Provides a high-level summary of the project including:
/// - Project metadata (name, version, hierarchy mode)
/// - Document counts by type (schematic, PCB, libraries)
/// - Project parameters
/// - Component summary (if schematic documents can be loaded)
pub fn cmd_overview(path: &Path) -> Result<PrjPcbOverview, Box<dyn std::error::Error>> {
    let prj = open_prjpcb(path)?;
    let project_dir = get_project_dir(path);

    // Build document summary
    let mut schematics = Vec::new();
    let mut pcb_documents = Vec::new();
    let mut libraries = Vec::new();
    let mut other = Vec::new();

    for doc in &prj.documents {
        let resolved_path = resolve_document_path(&project_dir, &doc.path);
        let exists = resolved_path.exists();
        let doc_info = DocumentInfo {
            path: doc.path.clone(),
            doc_type: doc.doc_type.display_name().to_string(),
            exists,
        };

        match doc.doc_type {
            DocumentType::Schematic => schematics.push(doc_info),
            DocumentType::Pcb => pcb_documents.push(doc_info),
            DocumentType::SchLib | DocumentType::PcbLib | DocumentType::IntLib => {
                libraries.push(doc_info)
            }
            _ => other.push(doc_info),
        }
    }

    // Sort documents
    schematics.sort_by(|a, b| alphanumeric_sort(&a.path, &b.path));
    pcb_documents.sort_by(|a, b| alphanumeric_sort(&a.path, &b.path));
    libraries.sort_by(|a, b| alphanumeric_sort(&a.path, &b.path));
    other.sort_by(|a, b| alphanumeric_sort(&a.path, &b.path));

    let document_summary = DocumentSummary {
        total_documents: prj.documents.len(),
        schematics,
        pcb_documents,
        libraries,
        other,
    };

    // Note: Component summary would require loading and parsing all schematic documents,
    // which is expensive and may fail for missing files. For now, we return None.
    // A future enhancement could add a `--deep` flag to enable this.
    let component_summary = None;

    Ok(PrjPcbOverview {
        path: path.display().to_string(),
        name: prj.name(),
        version: prj.version.clone(),
        hierarchy_mode: hierarchy_mode_to_string(prj.hierarchy_mode),
        document_summary,
        parameters: prj.parameters.clone(),
        component_summary,
    })
}

/// Returns detailed project metadata and configuration.
///
/// Provides comprehensive project information including:
/// - File path and project name
/// - Version and hierarchy mode
/// - Output path configuration
/// - Annotation settings
/// - Document and parameter counts
/// - ERC matrix configuration status
pub fn cmd_info(path: &Path) -> Result<PrjPcbInfo, Box<dyn std::error::Error>> {
    let prj = open_prjpcb(path)?;

    // Count documents by type
    let mut type_counts: HashMap<&str, usize> = HashMap::new();
    for doc in &prj.documents {
        *type_counts.entry(doc.doc_type.display_name()).or_insert(0) += 1;
    }

    let mut document_counts: Vec<(String, usize)> = type_counts
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    document_counts.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(PrjPcbInfo {
        path: path.display().to_string(),
        name: prj.name(),
        version: prj.version.clone(),
        hierarchy_mode: hierarchy_mode_to_string(prj.hierarchy_mode),
        output_path: prj.output_path.clone(),
        annotation_start: prj.annotation_start_value,
        document_counts,
        parameter_count: prj.parameters.len(),
        erc_matrix_rows: prj.erc_matrix.rows.len(),
    })
}

/// Lists referenced documents in the project.
///
/// Returns all documents referenced by the project file, optionally filtered
/// by document type. Each document entry includes:
/// - Relative path from project file
/// - Document type (Schematic, PCB, etc.)
/// - Whether the file exists on disk
/// - Annotation and library update settings
///
/// # Arguments
/// * `path` - Path to the project file
/// * `doc_type` - Optional filter for document type (e.g., "Schematic", "PCB")
pub fn cmd_documents(
    path: &Path,
    doc_type: Option<String>,
) -> Result<PrjPcbDocumentList, Box<dyn std::error::Error>> {
    let prj = open_prjpcb(path)?;
    let project_dir = get_project_dir(path);

    // Build document list with existence check
    let mut documents: Vec<DocumentDetailInfo> = prj
        .documents
        .iter()
        .filter(|doc| {
            if let Some(ref filter) = doc_type {
                let filter_lower = filter.to_lowercase();
                let type_name = doc.doc_type.display_name().to_lowercase();
                type_name.contains(&filter_lower) || {
                    // Also match common shorthand names
                    match doc.doc_type {
                        DocumentType::Schematic => {
                            filter_lower == "sch" || filter_lower == "schdoc"
                        }
                        DocumentType::Pcb => filter_lower == "pcb" || filter_lower == "pcbdoc",
                        DocumentType::SchLib => filter_lower == "schlib",
                        DocumentType::PcbLib => filter_lower == "pcblib",
                        DocumentType::IntLib => filter_lower == "intlib",
                        DocumentType::OutputJob => filter_lower == "outjob",
                        DocumentType::Other => false,
                    }
                }
            } else {
                true
            }
        })
        .map(|doc| {
            let resolved_path = resolve_document_path(&project_dir, &doc.path);
            DocumentDetailInfo {
                path: doc.path.clone(),
                doc_type: doc.doc_type.display_name().to_string(),
                exists: resolved_path.exists(),
                annotation_enabled: doc.annotation_enabled,
                library_update: doc.do_library_update,
            }
        })
        .collect();

    documents.sort_by(|a, b| alphanumeric_sort(&a.path, &b.path));

    Ok(PrjPcbDocumentList {
        path: path.display().to_string(),
        filter: doc_type,
        total_documents: documents.len(),
        documents,
    })
}

/// Aggregates BOM from all schematic sheets in the project.
///
/// This function collects component information from all schematic documents
/// in the project. Components can be returned either as individual items or
/// grouped by lib_reference (part number).
///
/// # Arguments
/// * `path` - Path to the project file
/// * `grouped` - If true, group components by lib_reference with quantity
///
/// # Notes
/// - Missing schematic files produce warnings, not errors
/// - Components are collected from SchDoc files only
/// - Due to the complexity of cross-document loading, this implementation
///   returns a placeholder BOM with project document info. Full BOM extraction
///   would require loading and parsing each schematic document.
pub fn cmd_bom(path: &Path, grouped: bool) -> Result<PrjPcbBom, Box<dyn std::error::Error>> {
    let prj = open_prjpcb(path)?;
    let project_dir = get_project_dir(path);

    // Collect schematic documents
    let schematic_docs: Vec<_> = prj
        .documents
        .iter()
        .filter(|d| d.doc_type == DocumentType::Schematic)
        .collect();

    // Track warnings for missing files
    let mut warnings = Vec::new();
    let all_components: Vec<SchematicComponentInfo> = Vec::new();

    for doc in &schematic_docs {
        let resolved_path = resolve_document_path(&project_dir, &doc.path);
        if !resolved_path.exists() {
            warnings.push(format!(
                "Warning: Schematic document not found: {}",
                doc.path
            ));
            continue;
        }

        // Note: Actually loading and parsing SchDoc files would require
        // importing crate::v2::io::schdoc::SchDocV2 and extracting components.
        // For now, we note that the file exists but cannot extract BOM data
        // without a more complex implementation that handles all record types.
        //
        // A full implementation would:
        // 1. Open each SchDoc with SchDocV2::open()
        // 2. Extract ComponentData records
        // 3. Find associated Designator and Parameter records
        // 4. Build the component info
        //
        // This is left as a TODO for when cross-document operations are needed.
    }

    // For now, return an empty BOM with the count of schematics found
    // This allows the command to work without crashing on missing files
    let items = if grouped {
        BomItems::Grouped(Vec::new())
    } else {
        BomItems::Individual(all_components)
    };

    Ok(PrjPcbBom {
        path: path.display().to_string(),
        total_components: 0,
        unique_parts: if grouped { Some(0) } else { None },
        items,
    })
}

/// Validates the project by checking for missing referenced documents.
///
/// Performs the following checks:
/// - All referenced documents exist on disk
/// - Project has at least one schematic document
/// - Project has at least one PCB document (warning if missing)
/// - ERC matrix is defined
///
/// Returns validation results with categorized errors and warnings.
pub fn cmd_validate(path: &Path) -> Result<PrjPcbValidation, Box<dyn std::error::Error>> {
    let prj = open_prjpcb(path)?;
    let project_dir = get_project_dir(path);

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Check for missing documents
    for doc in &prj.documents {
        let resolved_path = resolve_document_path(&project_dir, &doc.path);
        if !resolved_path.exists() {
            errors.push(format!(
                "Missing document: {} ({})",
                doc.path,
                doc.doc_type.display_name()
            ));
        }
    }

    // Check for required document types
    let has_schematic = prj
        .documents
        .iter()
        .any(|d| d.doc_type == DocumentType::Schematic);
    let has_pcb = prj
        .documents
        .iter()
        .any(|d| d.doc_type == DocumentType::Pcb);

    if !has_schematic {
        errors.push("Project has no schematic documents".to_string());
    }

    if !has_pcb {
        warnings.push("Project has no PCB document".to_string());
    }

    // Check project parameters
    if prj.name() == "Unnamed" && prj.parameters.get("Name").is_none() {
        warnings.push("Project has no name defined in parameters".to_string());
    }

    // Check ERC matrix
    if prj.erc_matrix.rows.is_empty() {
        warnings.push("ERC connection matrix is not defined".to_string());
    }

    // Check output path
    if prj.output_path.is_empty() {
        warnings.push("Output path is not configured".to_string());
    }

    // Check for duplicate document paths
    let mut seen_paths = std::collections::HashSet::new();
    for doc in &prj.documents {
        let path_lower = doc.path.to_lowercase();
        if !seen_paths.insert(path_lower.clone()) {
            warnings.push(format!("Duplicate document path: {}", doc.path));
        }
    }

    // Check annotation settings consistency
    let docs_with_disabled_annotation: Vec<_> = prj
        .documents
        .iter()
        .filter(|d| d.doc_type == DocumentType::Schematic && !d.annotation_enabled)
        .map(|d| d.path.clone())
        .collect();

    if !docs_with_disabled_annotation.is_empty() && docs_with_disabled_annotation.len() < prj.schematics().len() {
        warnings.push(format!(
            "Inconsistent annotation settings: {} of {} schematics have annotation disabled",
            docs_with_disabled_annotation.len(),
            prj.schematics().len()
        ));
    }

    Ok(PrjPcbValidation {
        path: path.display().to_string(),
        errors,
        warnings,
    })
}

// ===========================================================================
// EXPORT COMMANDS
// ===========================================================================

/// Serializes the project structure to JSON for external tools.
///
/// Returns a JSON representation of the project including:
/// - Project metadata (name, version, hierarchy mode)
/// - Document list with existence status
/// - Project parameters
/// - ERC matrix configuration
///
/// This output is suitable for LLM processing or integration with external
/// build systems and documentation tools.
pub fn cmd_json(path: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let prj = open_prjpcb(path)?;
    let project_dir = get_project_dir(path);

    // Build document list with resolved paths
    let documents: Vec<serde_json::Value> = prj
        .documents
        .iter()
        .map(|doc| {
            let resolved_path = resolve_document_path(&project_dir, &doc.path);
            serde_json::json!({
                "path": doc.path,
                "doc_type": doc.doc_type.display_name(),
                "exists": resolved_path.exists(),
                "annotation_enabled": doc.annotation_enabled,
                "annotation_start_value": doc.annotation_start_value,
                "do_library_update": doc.do_library_update,
                "do_database_update": doc.do_database_update,
            })
        })
        .collect();

    // Build ERC matrix (if present)
    let erc_matrix = if !prj.erc_matrix.rows.is_empty() {
        Some(serde_json::json!({
            "row_count": prj.erc_matrix.rows.len(),
            "rows": prj.erc_matrix.rows,
        }))
    } else {
        None
    };

    // Build output groups (if present)
    let output_groups: Vec<serde_json::Value> = prj
        .output_groups
        .iter()
        .map(|g| {
            serde_json::json!({
                "name": g.name,
                "output_type": g.output_type,
            })
        })
        .collect();

    // Build variants (if present)
    let variants: Vec<serde_json::Value> = prj
        .variants
        .iter()
        .map(|v| {
            serde_json::json!({
                "name": v.name,
                "description": v.description,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "file": path.display().to_string(),
        "name": prj.name(),
        "version": prj.version,
        "hierarchy_mode": prj.hierarchy_mode,
        "hierarchy_mode_display": hierarchy_mode_to_string(prj.hierarchy_mode),
        "output_path": prj.output_path,
        "annotation_start_value": prj.annotation_start_value,
        "documents": documents,
        "parameters": prj.parameters,
        "erc_matrix": erc_matrix,
        "output_groups": output_groups,
        "variants": variants,
    }))
}

// ===========================================================================
// TESTS
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_document_path_simple() {
        let project_dir = PathBuf::from("/home/user/project");
        let doc_path = "Sheet1.SchDoc";
        let resolved = resolve_document_path(&project_dir, doc_path);
        assert_eq!(resolved, PathBuf::from("/home/user/project/Sheet1.SchDoc"));
    }

    #[test]
    fn test_resolve_document_path_windows_separator() {
        let project_dir = PathBuf::from("/home/user/project");
        let doc_path = "Schematics\\Sheet1.SchDoc";
        let resolved = resolve_document_path(&project_dir, doc_path);
        assert_eq!(
            resolved,
            PathBuf::from("/home/user/project/Schematics/Sheet1.SchDoc")
        );
    }

    #[test]
    fn test_resolve_document_path_nested() {
        let project_dir = PathBuf::from("/home/user/project");
        let doc_path = "Source/Schematics/Sheet1.SchDoc";
        let resolved = resolve_document_path(&project_dir, doc_path);
        assert_eq!(
            resolved,
            PathBuf::from("/home/user/project/Source/Schematics/Sheet1.SchDoc")
        );
    }

    #[test]
    fn test_hierarchy_mode_to_string() {
        assert_eq!(hierarchy_mode_to_string(0), "Flat");
        assert_eq!(hierarchy_mode_to_string(1), "Hierarchical");
        assert_eq!(hierarchy_mode_to_string(2), "Unknown (2)");
        assert_eq!(hierarchy_mode_to_string(-1), "Unknown (-1)");
    }

    #[test]
    fn test_alphanumeric_sort() {
        let mut items = vec!["Sheet10", "Sheet2", "Sheet1", "PCB1"];
        items.sort_by(|a, b| alphanumeric_sort(a, b));
        assert_eq!(items, vec!["PCB1", "Sheet1", "Sheet2", "Sheet10"]);
    }

    #[test]
    fn test_alphanumeric_sort_mixed() {
        let mut items = vec![
            "Sheet10.SchDoc",
            "Sheet2.SchDoc",
            "Sheet1.SchDoc",
            "PCB1.PcbDoc",
        ];
        items.sort_by(|a, b| alphanumeric_sort(a, b));
        assert_eq!(
            items,
            vec![
                "PCB1.PcbDoc",
                "Sheet1.SchDoc",
                "Sheet2.SchDoc",
                "Sheet10.SchDoc"
            ]
        );
    }

    #[test]
    fn test_get_project_dir() {
        let path = PathBuf::from("/home/user/project/MyProject.PrjPcb");
        let dir = get_project_dir(&path);
        assert_eq!(dir, PathBuf::from("/home/user/project"));
    }

    #[test]
    fn test_get_project_dir_no_parent() {
        let path = PathBuf::from("MyProject.PrjPcb");
        let dir = get_project_dir(&path);
        assert_eq!(dir, PathBuf::from("."));
    }

    #[test]
    fn test_open_prjpcb_from_data() {
        // Use the test fixture in the data directory
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/Project1.PrjPcb");
        if path.exists() {
            let result = open_prjpcb(&path);
            assert!(result.is_ok());
            let prj = result.unwrap();
            assert_eq!(prj.name(), "Project1");
            assert_eq!(prj.documents.len(), 2);
        }
    }

    #[test]
    fn test_cmd_overview_from_data() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/Project1.PrjPcb");
        if path.exists() {
            let result = cmd_overview(&path);
            assert!(result.is_ok());
            let overview = result.unwrap();
            assert_eq!(overview.name, "Project1");
            assert_eq!(overview.document_summary.total_documents, 2);
            assert_eq!(overview.document_summary.schematics.len(), 1);
            assert_eq!(overview.document_summary.pcb_documents.len(), 1);
        }
    }

    #[test]
    fn test_cmd_info_from_data() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/Project1.PrjPcb");
        if path.exists() {
            let result = cmd_info(&path);
            assert!(result.is_ok());
            let info = result.unwrap();
            assert_eq!(info.name, "Project1");
            assert_eq!(info.version, "1.0");
            assert_eq!(info.hierarchy_mode, "Flat");
        }
    }

    #[test]
    fn test_cmd_documents_from_data() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/Project1.PrjPcb");
        if path.exists() {
            let result = cmd_documents(&path, None);
            assert!(result.is_ok());
            let docs = result.unwrap();
            assert_eq!(docs.total_documents, 2);
        }
    }

    #[test]
    fn test_cmd_documents_filtered() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/Project1.PrjPcb");
        if path.exists() {
            let result = cmd_documents(&path, Some("Schematic".to_string()));
            assert!(result.is_ok());
            let docs = result.unwrap();
            assert_eq!(docs.total_documents, 1);
            assert!(docs.documents[0].path.ends_with(".SchDoc"));
        }
    }

    #[test]
    fn test_cmd_validate_from_data() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/Project1.PrjPcb");
        if path.exists() {
            let result = cmd_validate(&path);
            assert!(result.is_ok());
            let validation = result.unwrap();
            // The test fixture references files that don't exist in the data directory
            assert!(!validation.errors.is_empty()); // Missing documents
        }
    }

    #[test]
    fn test_cmd_json_from_data() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/Project1.PrjPcb");
        if path.exists() {
            let result = cmd_json(&path);
            assert!(result.is_ok());
            let json = result.unwrap();
            assert_eq!(json["name"], "Project1");
            assert_eq!(json["version"], "1.0");
            assert!(json["documents"].is_array());
        }
    }

    #[test]
    fn test_cmd_bom_grouped() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/Project1.PrjPcb");
        if path.exists() {
            let result = cmd_bom(&path, true);
            assert!(result.is_ok());
            let bom = result.unwrap();
            assert!(matches!(bom.items, BomItems::Grouped(_)));
        }
    }

    #[test]
    fn test_cmd_bom_individual() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/Project1.PrjPcb");
        if path.exists() {
            let result = cmd_bom(&path, false);
            assert!(result.is_ok());
            let bom = result.unwrap();
            assert!(matches!(bom.items, BomItems::Individual(_)));
        }
    }
}

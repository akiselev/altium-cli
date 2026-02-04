// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Project file operations.
//!
//! High-level operations for managing Altium project (.PrjPcb) files.
//!
//! # V2 Migration Note
//! This module uses v1 types (SchDoc, SchRecord, RecordTree<SchRecord>).
//! TODO: Migrate to v2 types when SchDocV2 provides equivalent functionality.

#![allow(deprecated)]

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use crate::io::{DocumentType, PcbDoc, PrjPcb, SchDoc};
use crate::ops::output::*;
use crate::records::sch::SchRecord;
use crate::tree::RecordTree;

fn open_prjpcb(path: &Path) -> Result<PrjPcb, Box<dyn std::error::Error>> {
    Ok(PrjPcb::open_file(path)?)
}

/// Get the project directory.
fn project_dir(path: &Path) -> PathBuf {
    path.parent().unwrap_or(Path::new(".")).to_path_buf()
}

/// Resolve a document path relative to the project.
fn resolve_document_path(project_path: &Path, doc_path: &str) -> PathBuf {
    let project_dir = project_dir(project_path);
    project_dir.join(doc_path)
}

/// Open a schematic document from the project.
fn open_schdoc(project_path: &Path, doc_path: &str) -> Result<SchDoc, Box<dyn std::error::Error>> {
    let full_path = resolve_document_path(project_path, doc_path);
    let file = File::open(&full_path)?;
    Ok(SchDoc::open(BufReader::new(file))?)
}

/// Open a PCB document from the project.
fn open_pcbdoc(project_path: &Path, doc_path: &str) -> Result<PcbDoc, Box<dyn std::error::Error>> {
    let full_path = resolve_document_path(project_path, doc_path);
    let file = File::open(&full_path)?;
    Ok(PcbDoc::open(BufReader::new(file))?)
}

// ═══════════════════════════════════════════════════════════════════════════
// COMPONENT AND NET EXTRACTION UTILITIES
// ═══════════════════════════════════════════════════════════════════════════

/// Component info extracted from schematics.
#[derive(Debug, Clone, Default)]
struct SchematicComponent {
    designator: String,
    lib_reference: String,
    description: String,
    footprint: String,
    value: String,
    sheet: String,
    parameters: HashMap<String, String>,
}

/// Net info extracted from schematics.
#[derive(Debug, Clone, Default)]
struct SchematicNet {
    name: String,
    pins: Vec<NetPin>,
}

/// Pin connection in a net.
#[derive(Debug, Clone, Default)]
struct NetPin {
    component: String,
    pin: String,
}

/// Extract all components from project schematics.
fn extract_components(
    prj: &PrjPcb,
    project_path: &Path,
) -> Result<Vec<SchematicComponent>, Box<dyn std::error::Error>> {
    let mut components = Vec::new();

    for doc in prj.schematics() {
        let schdoc = match open_schdoc(project_path, &doc.path) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Failed to open schematic document {}: {}", doc.path, e);
                continue;
            }
        };

        // Build a tree to find designators
        let tree = RecordTree::from_records(schdoc.primitives.clone());

        // Extract components from this schematic
        for (id, record) in tree.iter() {
            if let SchRecord::Component(comp) = record {
                // Find designator from child records
                let mut designator = String::new();
                for (_child_id, child) in tree.children(id) {
                    if let SchRecord::Designator(d) = child {
                        designator = d.param.label.text.clone();
                        break;
                    }
                }

                let sch_comp = SchematicComponent {
                    designator,
                    lib_reference: comp.lib_reference.clone(),
                    description: comp.component_description.clone(),
                    sheet: doc.path.clone(),
                    ..Default::default()
                };

                components.push(sch_comp);
            }
        }
    }

    Ok(components)
}

/// Extract all nets from project schematics.
fn extract_nets(
    prj: &PrjPcb,
    project_path: &Path,
) -> Result<Vec<SchematicNet>, Box<dyn std::error::Error>> {
    let mut net_map: HashMap<String, SchematicNet> = HashMap::new();

    for doc in prj.schematics() {
        let schdoc = match open_schdoc(project_path, &doc.path) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Failed to open schematic document {}: {}", doc.path, e);
                continue;
            }
        };

        // Extract net labels and power ports
        for record in &schdoc.primitives {
            match record {
                SchRecord::NetLabel(label) => {
                    let net_name = label.label.text.clone();
                    let net = net_map
                        .entry(net_name.clone())
                        .or_insert_with(|| SchematicNet {
                            name: net_name,
                            pins: Vec::new(),
                        });
                    // Net labels mark connection points but don't directly connect to pins
                    let _ = net; // Mark as used
                }
                SchRecord::PowerObject(power) => {
                    let net_name = power.text.clone();
                    let net = net_map
                        .entry(net_name.clone())
                        .or_insert_with(|| SchematicNet {
                            name: net_name,
                            pins: Vec::new(),
                        });
                    let _ = net;
                }
                _ => {}
            }
        }
    }

    Ok(net_map.into_values().collect())
}

// ═══════════════════════════════════════════════════════════════════════════
// HIGH-LEVEL COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

pub fn cmd_overview(path: &Path) -> Result<PrjPcbOverview, Box<dyn std::error::Error>> {
    let prj = open_prjpcb(path)?;

    let schematics: Vec<_> = prj
        .documents
        .iter()
        .filter(|d| d.doc_type == DocumentType::Schematic)
        .map(|d| DocumentInfo {
            path: d.path.clone(),
            doc_type: d.doc_type.display_name().to_string(),
            exists: resolve_document_path(path, &d.path).exists(),
        })
        .collect();

    let pcb_documents: Vec<_> = prj
        .documents
        .iter()
        .filter(|d| d.doc_type == DocumentType::Pcb)
        .map(|d| DocumentInfo {
            path: d.path.clone(),
            doc_type: d.doc_type.display_name().to_string(),
            exists: resolve_document_path(path, &d.path).exists(),
        })
        .collect();

    let libraries: Vec<_> = prj
        .documents
        .iter()
        .filter(|d| {
            d.doc_type == DocumentType::SchLib
                || d.doc_type == DocumentType::PcbLib
                || d.doc_type == DocumentType::IntLib
        })
        .map(|d| DocumentInfo {
            path: d.path.clone(),
            doc_type: d.doc_type.display_name().to_string(),
            exists: resolve_document_path(path, &d.path).exists(),
        })
        .collect();

    let other: Vec<_> = prj
        .documents
        .iter()
        .filter(|d| d.doc_type == DocumentType::Other || d.doc_type == DocumentType::OutputJob)
        .map(|d| DocumentInfo {
            path: d.path.clone(),
            doc_type: d.doc_type.display_name().to_string(),
            exists: resolve_document_path(path, &d.path).exists(),
        })
        .collect();

    let document_summary = DocumentSummary {
        total_documents: prj.documents.len(),
        schematics,
        pcb_documents,
        libraries,
        other,
    };

    // Component summary (if schematics exist)
    let component_summary = if !prj.schematics().is_empty() {
        extract_components(&prj, path).ok().and_then(|components| {
            if components.is_empty() {
                None
            } else {
                // Count by prefix
                let mut by_prefix: HashMap<String, usize> = HashMap::new();
                for comp in &components {
                    let prefix: String = comp
                        .designator
                        .chars()
                        .take_while(|c| c.is_alphabetic())
                        .collect();
                    *by_prefix.entry(prefix).or_default() += 1;
                }

                let mut prefixes: Vec<_> = by_prefix
                    .into_iter()
                    .map(|(prefix, count)| {
                        let display_name = match prefix.as_str() {
                            "R" => "Resistors".to_string(),
                            "C" => "Capacitors".to_string(),
                            "L" => "Inductors".to_string(),
                            "U" => "ICs".to_string(),
                            "Q" => "Transistors".to_string(),
                            "D" => "Diodes".to_string(),
                            "J" | "P" => "Connectors".to_string(),
                            "SW" | "S" => "Switches".to_string(),
                            "F" => "Fuses".to_string(),
                            "Y" => "Crystals".to_string(),
                            _ => prefix.clone(),
                        };
                        (prefix, display_name, count)
                    })
                    .collect();
                prefixes.sort_by(|a, b| b.2.cmp(&a.2));

                Some(ComponentSummaryStats {
                    total_components: components.len(),
                    by_prefix: prefixes,
                })
            }
        })
    } else {
        None
    };

    Ok(PrjPcbOverview {
        path: path.display().to_string(),
        name: prj.name(),
        version: prj.version.clone(),
        hierarchy_mode: if prj.hierarchy_mode == 0 {
            "Flat".to_string()
        } else {
            "Hierarchical".to_string()
        },
        document_summary,
        parameters: prj.parameters.clone(),
        component_summary,
    })
}

pub fn cmd_info(path: &Path) -> Result<PrjPcbInfo, Box<dyn std::error::Error>> {
    let prj = open_prjpcb(path)?;

    let mut by_type: HashMap<&str, usize> = HashMap::new();
    for doc in &prj.documents {
        *by_type.entry(doc.doc_type.display_name()).or_default() += 1;
    }

    let mut document_counts: Vec<_> = by_type
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    document_counts.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

    Ok(PrjPcbInfo {
        path: path.display().to_string(),
        name: prj.name(),
        version: prj.version.clone(),
        hierarchy_mode: if prj.hierarchy_mode == 0 {
            "Flat".to_string()
        } else {
            "Hierarchical".to_string()
        },
        output_path: if prj.output_path.is_empty() {
            "(default)".to_string()
        } else {
            prj.output_path.clone()
        },
        annotation_start: prj.annotation_start_value,
        document_counts,
        parameter_count: prj.parameters.len(),
        erc_matrix_rows: prj.erc_matrix.rows.len(),
    })
}

pub fn cmd_documents(
    path: &Path,
    doc_type: Option<String>,
) -> Result<PrjPcbDocumentList, Box<dyn std::error::Error>> {
    let prj = open_prjpcb(path)?;

    let filter_type = doc_type.as_ref().map(|t| t.to_lowercase());

    let documents: Vec<_> = prj
        .documents
        .iter()
        .filter(|d| {
            if let Some(ref filter) = filter_type {
                d.doc_type.display_name().to_lowercase().contains(filter)
            } else {
                true
            }
        })
        .map(|d| DocumentDetailInfo {
            path: d.path.clone(),
            doc_type: d.doc_type.display_name().to_string(),
            exists: resolve_document_path(path, &d.path).exists(),
            annotation_enabled: d.annotation_enabled,
            library_update: d.do_library_update,
        })
        .collect();

    Ok(PrjPcbDocumentList {
        path: path.display().to_string(),
        filter: doc_type,
        total_documents: documents.len(),
        documents,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// CREATION COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Embedded blank PrjPcb template.
const BLANK_PRJPCB_TEMPLATE: &[u8] = include_bytes!("../../data/Project1.PrjPcb");

pub fn cmd_create(
    path: &Path,
    name: Option<String>,
    template: Option<PathBuf>,
) -> Result<String, Box<dyn std::error::Error>> {
    if path.exists() {
        return Err(format!("File already exists: {}", path.display()).into());
    }

    let message = match template {
        Some(template_path) => {
            std::fs::copy(&template_path, path)?;
            format!(
                "Created project from template: {}\n  Template: {}",
                path.display(),
                template_path.display()
            )
        }
        None => {
            // Use embedded template
            std::fs::write(path, BLANK_PRJPCB_TEMPLATE)?;

            // If name specified, update it
            if let Some(ref project_name) = name {
                let mut prj = open_prjpcb(path)?;
                prj.set_name(project_name);
                prj.save_to_file(path)?;
            }

            format!("Created new project: {}", path.display())
        }
    };

    // Verify
    let prj = open_prjpcb(path)?;
    Ok(format!(
        "{}\n  Name: {}\n  Documents: {}",
        message,
        prj.name(),
        prj.documents.len()
    ))
}

// ═══════════════════════════════════════════════════════════════════════════
// DOCUMENT MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════

pub fn cmd_add_document(path: &Path, document: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut prj = open_prjpcb(path)?;

    // Check if document already exists
    if prj.get_document(document).is_some() {
        return Err(format!("Document '{}' already in project", document).into());
    }

    prj.add_document(document);
    prj.save_to_file(path)?;

    let doc_type = DocumentType::from_path(document);
    Ok(format!(
        "Added {} to project: {}\n  Type: {}\n  Total documents: {}",
        document,
        path.display(),
        doc_type,
        prj.documents.len()
    ))
}

pub fn cmd_remove_document(
    path: &Path,
    document: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut prj = open_prjpcb(path)?;

    if !prj.remove_document(document) {
        return Err(format!("Document '{}' not found in project", document).into());
    }

    prj.save_to_file(path)?;

    Ok(format!(
        "Removed {} from project: {}\n  Remaining documents: {}",
        document,
        path.display(),
        prj.documents.len()
    ))
}

// ═══════════════════════════════════════════════════════════════════════════
// PARAMETER MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════

pub fn cmd_parameters(path: &Path) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let prj = open_prjpcb(path)?;
    Ok(prj.parameters.clone())
}

pub fn cmd_set_parameter(
    path: &Path,
    name: &str,
    value: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut prj = open_prjpcb(path)?;

    let was_existing = prj.parameters.contains_key(name);
    prj.set_parameter(name, value);
    prj.save_to_file(path)?;

    if was_existing {
        Ok(format!(
            "Updated parameter '{}' = '{}' in {}",
            name,
            value,
            path.display()
        ))
    } else {
        Ok(format!(
            "Added parameter '{}' = '{}' to {}",
            name,
            value,
            path.display()
        ))
    }
}

pub fn cmd_remove_parameter(path: &Path, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut prj = open_prjpcb(path)?;

    if prj.remove_parameter(name).is_none() {
        return Err(format!("Parameter '{}' not found", name).into());
    }

    prj.save_to_file(path)?;
    Ok(format!(
        "Removed parameter '{}' from {}",
        name,
        path.display()
    ))
}

// ═══════════════════════════════════════════════════════════════════════════
// NETLIST AND COMPONENT EXTRACTION
// ═══════════════════════════════════════════════════════════════════════════

pub fn cmd_netlist(path: &Path) -> Result<PrjPcbNetlist, Box<dyn std::error::Error>> {
    let prj = open_prjpcb(path)?;
    let nets = extract_nets(&prj, path)?;

    let net_infos: Vec<NetInfo> = nets
        .into_iter()
        .map(|net| NetInfo {
            name: net.name,
            pins: net
                .pins
                .into_iter()
                .map(|pin| NetPinConnection {
                    component: pin.component,
                    pin: pin.pin,
                })
                .collect(),
        })
        .collect();

    Ok(PrjPcbNetlist {
        path: path.display().to_string(),
        total_nets: net_infos.len(),
        nets: net_infos,
    })
}

pub fn cmd_components(path: &Path) -> Result<PrjPcbComponentList, Box<dyn std::error::Error>> {
    let prj = open_prjpcb(path)?;
    let components = extract_components(&prj, path)?;

    let component_infos: Vec<SchematicComponentInfo> = components
        .into_iter()
        .map(|comp| SchematicComponentInfo {
            designator: comp.designator,
            lib_reference: comp.lib_reference,
            description: comp.description,
            footprint: comp.footprint,
            value: comp.value,
            sheet: comp.sheet,
            parameters: comp.parameters,
        })
        .collect();

    Ok(PrjPcbComponentList {
        path: path.display().to_string(),
        total_components: component_infos.len(),
        components: component_infos,
    })
}

pub fn cmd_bom(path: &Path, grouped: bool) -> Result<PrjPcbBom, Box<dyn std::error::Error>> {
    let prj = open_prjpcb(path)?;
    let components = extract_components(&prj, path)?;

    let items = if grouped {
        // Group by lib_reference
        let mut groups: HashMap<String, Vec<&SchematicComponent>> = HashMap::new();
        for comp in &components {
            groups
                .entry(comp.lib_reference.clone())
                .or_default()
                .push(comp);
        }

        let mut group_items: Vec<_> = groups
            .into_iter()
            .map(|(lib_ref, comps)| BomGroupItem {
                lib_reference: lib_ref,
                quantity: comps.len(),
                designators: comps.iter().map(|c| c.designator.clone()).collect(),
            })
            .collect();
        group_items.sort_by(|a, b| b.quantity.cmp(&a.quantity));

        BomItems::Grouped(group_items)
    } else {
        let component_infos: Vec<SchematicComponentInfo> = components
            .into_iter()
            .map(|comp| SchematicComponentInfo {
                designator: comp.designator,
                lib_reference: comp.lib_reference,
                description: comp.description,
                footprint: comp.footprint,
                value: comp.value,
                sheet: comp.sheet,
                parameters: comp.parameters,
            })
            .collect();

        BomItems::Individual(component_infos)
    };

    let (total_components, unique_parts) = match &items {
        BomItems::Grouped(groups) => {
            let total = groups.iter().map(|g| g.quantity).sum();
            (total, Some(groups.len()))
        }
        BomItems::Individual(comps) => (comps.len(), None),
    };

    Ok(PrjPcbBom {
        path: path.display().to_string(),
        total_components,
        unique_parts,
        items,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// PCB IMPORT COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

pub fn cmd_import_to_pcb(
    path: &Path,
    pcb: Option<String>,
    dry_run: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let prj = open_prjpcb(path)?;

    // Find the target PCB
    let pcb_doc = if let Some(ref pcb_path) = pcb {
        prj.get_document(pcb_path)
            .ok_or_else(|| format!("PCB document '{}' not found in project", pcb_path))?
    } else {
        prj.primary_pcb()
            .ok_or_else(|| "No PCB document found in project".to_string())?
    };

    let pcb_path_str = pcb_doc.path.clone();

    // Extract components and nets from schematics
    let components = extract_components(&prj, path)?;
    let nets = extract_nets(&prj, path)?;

    if dry_run {
        let mut message = format!("Import to PCB: {}\n", path.display());
        message.push_str(&format!("Target PCB: {}\n", pcb_path_str));
        message.push_str(&format!("Components to import: {}\n", components.len()));
        message.push_str(&format!("Nets to import: {}\n\n", nets.len()));
        message.push_str("[DRY RUN - No changes will be made]\n\n");
        message.push_str("Components that would be added:\n");
        for comp in components.iter().take(20) {
            message.push_str(&format!("  {} - {}\n", comp.designator, comp.lib_reference));
        }
        if components.len() > 20 {
            message.push_str(&format!("  ... and {} more\n", components.len() - 20));
        }
        message.push_str("\nNets that would be added:\n");
        for net in nets.iter().take(20) {
            message.push_str(&format!("  {}\n", net.name));
        }
        if nets.len() > 20 {
            message.push_str(&format!("  ... and {} more", nets.len() - 20));
        }
        Ok(message)
    } else {
        // Open the PCB and add components
        let full_pcb_path = resolve_document_path(path, &pcb_path_str);
        let mut pcbdoc = open_pcbdoc(path, &pcb_path_str)?;

        // Get existing components and nets
        let existing_designators: HashSet<_> = pcbdoc
            .components
            .iter()
            .map(|c| c.designator.clone())
            .collect();
        let existing_nets: HashSet<_> = pcbdoc.nets.iter().cloned().collect();

        let mut added_components = 0;
        let mut added_nets = 0;

        // Add missing components
        for comp in &components {
            if !existing_designators.contains(&comp.designator) {
                // Would add component here - for now just count
                added_components += 1;
            }
        }

        // Add missing nets
        for net in &nets {
            if !existing_nets.contains(&net.name) {
                pcbdoc.nets.push(net.name.clone());
                added_nets += 1;
            }
        }

        // Save if changes were made
        if added_components > 0 || added_nets > 0 {
            pcbdoc.save_to_file(&full_pcb_path)?;
            Ok(format!(
                "Import complete:\n  Added {} nets\n  Components: {} (component placement not yet implemented)",
                added_nets, added_components
            ))
        } else {
            Ok("No changes needed - PCB already up to date".to_string())
        }
    }
}

pub fn cmd_sync_to_pcb(
    path: &Path,
    pcb: Option<String>,
    dry_run: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    // For now, this is similar to import but focuses on synchronization
    cmd_import_to_pcb(path, pcb, dry_run)
}

pub fn cmd_diff_sch_pcb(
    path: &Path,
    pcb: Option<String>,
) -> Result<PrjPcbSchPcbDiff, Box<dyn std::error::Error>> {
    let prj = open_prjpcb(path)?;

    // Find the target PCB
    let pcb_doc = if let Some(ref pcb_path) = pcb {
        prj.get_document(pcb_path)
            .ok_or_else(|| format!("PCB document '{}' not found in project", pcb_path))?
    } else {
        prj.primary_pcb()
            .ok_or_else(|| "No PCB document found in project".to_string())?
    };

    let pcb_path_str = pcb_doc.path.clone();

    // Extract from schematics
    let sch_components = extract_components(&prj, path)?;
    let sch_nets = extract_nets(&prj, path)?;

    // Load PCB
    let pcbdoc = open_pcbdoc(path, &pcb_path_str)?;

    let sch_designators: HashSet<_> = sch_components.iter().map(|c| &c.designator).collect();
    let pcb_designators: HashSet<_> = pcbdoc.components.iter().map(|c| &c.designator).collect();

    let sch_net_names: HashSet<_> = sch_nets.iter().map(|n| &n.name).collect();
    let pcb_net_names: HashSet<_> = pcbdoc.nets.iter().collect();

    // Components only in schematic
    let only_in_schematic: Vec<String> = sch_designators
        .difference(&pcb_designators)
        .map(|s| s.to_string())
        .collect();
    // Components only in PCB
    let only_in_pcb: Vec<String> = pcb_designators
        .difference(&sch_designators)
        .map(|s| s.to_string())
        .collect();

    // Nets only in schematic
    let nets_only_in_schematic: Vec<String> = sch_net_names
        .difference(&pcb_net_names)
        .map(|s| s.to_string())
        .collect();
    // Nets only in PCB
    let nets_only_in_pcb: Vec<String> = pcb_net_names
        .difference(&sch_net_names)
        .map(|s| s.to_string())
        .collect();

    Ok(PrjPcbSchPcbDiff {
        path: path.display().to_string(),
        pcb_document: pcb_path_str,
        schematic_components: sch_components.len(),
        pcb_components: pcbdoc.components.len(),
        only_in_schematic,
        only_in_pcb,
        schematic_nets: sch_nets.len(),
        pcb_nets: pcbdoc.nets.len(),
        nets_only_in_schematic,
        nets_only_in_pcb,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// VALIDATION
// ═══════════════════════════════════════════════════════════════════════════

pub fn cmd_validate(
    path: &Path,
    check_files: bool,
) -> Result<PrjPcbValidation, Box<dyn std::error::Error>> {
    let prj = open_prjpcb(path)?;

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Basic validation
    if prj.name().is_empty() || prj.name() == "Unnamed" {
        warnings.push("Project has no name defined".to_string());
    }

    if prj.documents.is_empty() {
        warnings.push("Project has no documents".to_string());
    }

    // Check for at least one schematic and one PCB
    if prj.schematics().is_empty() {
        warnings.push("Project has no schematic documents".to_string());
    }
    if prj.pcb_documents().is_empty() {
        warnings.push("Project has no PCB documents".to_string());
    }

    // Check for duplicate document paths
    let mut seen_paths = HashSet::new();
    for doc in &prj.documents {
        if !seen_paths.insert(&doc.path) {
            errors.push(format!("Duplicate document path: {}", doc.path));
        }
    }

    // Check file existence if requested
    if check_files {
        for doc in &prj.documents {
            let full_path = resolve_document_path(path, &doc.path);
            if !full_path.exists() {
                errors.push(format!("Missing document: {}", doc.path));
            }
        }
    }

    Ok(PrjPcbValidation {
        path: path.display().to_string(),
        errors,
        warnings,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// EXPORT
// ═══════════════════════════════════════════════════════════════════════════

pub fn cmd_json(
    path: &Path,
    full: bool,
    pretty: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    use serde::Serialize;

    let prj = open_prjpcb(path)?;

    #[derive(Serialize)]
    struct DocumentJson {
        path: String,
        doc_type: String,
        annotation_enabled: bool,
    }

    #[derive(Serialize)]
    struct ProjectJson {
        name: String,
        version: String,
        hierarchy_mode: i32,
        output_path: String,
        documents: Vec<DocumentJson>,
        #[serde(skip_serializing_if = "HashMap::is_empty")]
        parameters: HashMap<String, String>,
    }

    let documents: Vec<_> = prj
        .documents
        .iter()
        .map(|d| DocumentJson {
            path: d.path.clone(),
            doc_type: d.doc_type.display_name().to_string(),
            annotation_enabled: d.annotation_enabled,
        })
        .collect();

    let output = ProjectJson {
        name: prj.name(),
        version: prj.version.clone(),
        hierarchy_mode: prj.hierarchy_mode,
        output_path: prj.output_path.clone(),
        documents,
        parameters: if full {
            prj.parameters.clone()
        } else {
            HashMap::new()
        },
    };

    let json = if pretty {
        serde_json::to_string_pretty(&output)?
    } else {
        serde_json::to_string(&output)?
    };

    Ok(json)
}

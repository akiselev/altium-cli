// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Query command implementation.
//!
//! Provides selector-based record querying using two query languages:
//! - Record Selector syntax: `U1`, `R*`, `$LM358`, `~VCC`, `@10K`, `U1:VCC`
//! - SchQL syntax: `component[part*=7805]`, `pin[type=input]`, `net:power`

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use serde::Serialize;

use crate::output::{self, TextFormat};
use altium_format::io::{SchDoc, SchLib};
use altium_format::query::{
    QueryMatch as SchqlMatch, QueryResult as SchqlResult, RecordQueryMatch,
    query_records_with_doc_name, query_schdoc,
};
use altium_format::records::sch::SchRecord;
use altium_format::tree::RecordTree;

/// Result structure for query command output.
#[derive(Serialize)]
pub struct QueryCommandResult {
    /// Path to the queried file.
    pub file: String,
    /// The query string that was executed.
    pub query: String,
    /// Query language used (selector or schql).
    pub language: String,
    /// Number of matches found.
    pub match_count: usize,
    /// Execution time in microseconds.
    pub execution_time_us: u64,
    /// The matched records.
    pub matches: Vec<QueryMatchOutput>,
}

impl TextFormat for QueryCommandResult {
    fn format_text(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("Query: {} ({})\n", self.query, self.language));
        output.push_str(&format!("File: {}\n", self.file));
        output.push_str(&format!(
            "Matches: {} ({}µs)\n\n",
            self.match_count, self.execution_time_us
        ));

        for (i, m) in self.matches.iter().enumerate() {
            output.push_str(&format!("{}. {} ({})\n", i + 1, m.unique_id, m.record_type));
            if let Some(desc) = &m.description {
                output.push_str(&format!("   {}\n", desc));
            }
            if let Some(loc) = &m.location {
                output.push_str(&format!("   Location: {}\n", loc));
            }
            if let Some(net) = &m.net {
                output.push_str(&format!("   Net: {}\n", net));
            }
            if let Some(children) = m.children {
                output.push_str(&format!("   Children: {}\n", children));
            }
        }

        output
    }
}

/// A single query match for output.
#[derive(Serialize)]
pub struct QueryMatchOutput {
    /// Type of the matched record.
    pub record_type: String,
    /// Primary identifier (designator, net name, etc.).
    pub unique_id: String,
    /// Additional description or context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Location if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// Associated net (for pins, ports).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net: Option<String>,
    /// Child count (for components).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<usize>,
}

/// Run the query command.
///
/// Detects query language based on syntax:
/// - Starts with `component`, `pin`, `net`, `wire`, etc. with brackets -> SchQL
/// - Otherwise -> Record Selector syntax
pub fn run(path: &Path, selector: &str, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "schdoc" => run_schdoc_query(path, selector, format),
        "schlib" => run_schlib_query(path, selector, format),
        _ => Err(format!(
            "Unsupported file type for query: {}. Supported: .SchDoc, .SchLib",
            extension
        )
        .into()),
    }
}

/// Query a schematic document.
fn run_schdoc_query(
    path: &Path,
    selector: &str,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();

    let file = File::open(path)?;
    let doc = SchDoc::open(BufReader::new(file))?;

    let (matches, language) = if is_schql_query(selector) {
        let result = query_schdoc(&doc, selector)?;
        let matches = convert_schql_matches(&result);
        (matches, "schql")
    } else {
        let tree = RecordTree::from_records(doc.primitives.clone());
        let doc_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let results = query_records_with_doc_name(&tree, selector, Some(doc_name))?;
        let matches = convert_record_matches(&tree, &results);
        (matches, "selector")
    };

    let elapsed = start.elapsed();

    let result = QueryCommandResult {
        file: path.display().to_string(),
        query: selector.to_string(),
        language: language.to_string(),
        match_count: matches.len(),
        execution_time_us: elapsed.as_micros() as u64,
        matches,
    };

    output::print(&result, format)?;
    Ok(())
}

/// Query a schematic library.
fn run_schlib_query(
    path: &Path,
    selector: &str,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();

    let file = File::open(path)?;
    let lib = SchLib::open(BufReader::new(file))?;

    let mut all_matches = Vec::new();

    for component in lib.components.iter() {
        let tree = RecordTree::from_records(component.primitives.clone());
        let results = query_records_with_doc_name(&tree, selector, Some(component.name()))?;

        for result in results {
            if let Some(record) = tree.get(result.id) {
                let match_output =
                    record_to_output(record, &tree, result.id, Some(component.name()));
                all_matches.push(match_output);
            }
        }
    }

    let elapsed = start.elapsed();

    let result = QueryCommandResult {
        file: path.display().to_string(),
        query: selector.to_string(),
        language: "selector".to_string(),
        match_count: all_matches.len(),
        execution_time_us: elapsed.as_micros() as u64,
        matches: all_matches,
    };

    output::print(&result, format)?;
    Ok(())
}

/// Detect if the query uses SchQL syntax.
fn is_schql_query(query: &str) -> bool {
    let trimmed = query.trim().to_lowercase();

    let schql_prefixes = [
        "component",
        "pin",
        "net",
        "wire",
        "port",
        "power",
        "junction",
        "label",
        "#",
    ];

    for prefix in schql_prefixes {
        if trimmed.starts_with(prefix)
            && (trimmed.contains('[')
                || trimmed.contains(">")
                || trimmed.contains("::")
                || (trimmed.contains(':') && !trimmed.contains(":#")))
        {
            return true;
        }
    }

    false
}

/// Convert SchQL query results to output format.
fn convert_schql_matches(result: &SchqlResult) -> Vec<QueryMatchOutput> {
    result
        .matches
        .iter()
        .map(|m| match m {
            SchqlMatch::Component {
                designator,
                part,
                description,
                value,
                pin_count,
                ..
            } => QueryMatchOutput {
                record_type: "Component".to_string(),
                unique_id: designator.clone(),
                description: Some(format!(
                    "{} - {}{}",
                    part,
                    description,
                    value
                        .as_ref()
                        .map(|v| format!(" ({})", v))
                        .unwrap_or_default()
                )),
                location: None,
                net: None,
                children: Some(*pin_count),
            },
            SchqlMatch::Pin {
                component_designator,
                designator,
                name,
                electrical_type,
                connected_net,
                is_hidden,
            } => QueryMatchOutput {
                record_type: "Pin".to_string(),
                unique_id: format!("{}.{}", component_designator, designator),
                description: Some(format!(
                    "{} [{}]{}",
                    name,
                    electrical_type,
                    if *is_hidden { " (hidden)" } else { "" }
                )),
                location: None,
                net: connected_net.clone(),
                children: None,
            },
            SchqlMatch::Net {
                name,
                is_power,
                is_ground,
                connection_count,
                connections,
            } => QueryMatchOutput {
                record_type: if *is_ground {
                    "Ground"
                } else if *is_power {
                    "PowerNet"
                } else {
                    "Net"
                }
                .to_string(),
                unique_id: name.clone(),
                description: Some(format!(
                    "{} connections: {}",
                    connection_count,
                    connections.join(", ")
                )),
                location: None,
                net: Some(name.clone()),
                children: None,
            },
            SchqlMatch::Port {
                name,
                io_type,
                connected_net,
            } => QueryMatchOutput {
                record_type: "Port".to_string(),
                unique_id: name.clone(),
                description: Some(format!("[{}]", io_type)),
                location: None,
                net: connected_net.clone(),
                children: None,
            },
            SchqlMatch::Wire {
                index,
                vertex_count,
                start,
                end,
            } => QueryMatchOutput {
                record_type: "Wire".to_string(),
                unique_id: format!("Wire#{}", index),
                description: Some(format!(
                    "{} vertices from ({},{}) to ({},{})",
                    vertex_count,
                    start.0 / 10000,
                    start.1 / 10000,
                    end.0 / 10000,
                    end.1 / 10000
                )),
                location: Some(format!("({}, {})", start.0 / 10000, start.1 / 10000)),
                net: None,
                children: None,
            },
            SchqlMatch::Power {
                net_name,
                style,
                is_ground,
            } => QueryMatchOutput {
                record_type: if *is_ground { "Ground" } else { "Power" }.to_string(),
                unique_id: net_name.clone(),
                description: Some(format!("[{}]", style)),
                location: None,
                net: Some(net_name.clone()),
                children: None,
            },
            SchqlMatch::Label { text, location } => QueryMatchOutput {
                record_type: "Label".to_string(),
                unique_id: text.clone(),
                description: None,
                location: Some(format!("({}, {})", location.0 / 10000, location.1 / 10000)),
                net: None,
                children: None,
            },
            SchqlMatch::Junction { location } => QueryMatchOutput {
                record_type: "Junction".to_string(),
                unique_id: format!("@({},{})", location.0 / 10000, location.1 / 10000),
                description: None,
                location: Some(format!("({}, {})", location.0 / 10000, location.1 / 10000)),
                net: None,
                children: None,
            },
            SchqlMatch::Parameter {
                component_designator,
                name,
                value,
            } => QueryMatchOutput {
                record_type: "Parameter".to_string(),
                unique_id: format!("{}.{}", component_designator, name),
                description: Some(value.clone()),
                location: None,
                net: None,
                children: None,
            },
            SchqlMatch::Count(n) => QueryMatchOutput {
                record_type: "Count".to_string(),
                unique_id: n.to_string(),
                description: None,
                location: None,
                net: None,
                children: None,
            },
        })
        .collect()
}

/// Convert record selector query results to output format.
fn convert_record_matches(
    tree: &RecordTree<SchRecord>,
    results: &[RecordQueryMatch],
) -> Vec<QueryMatchOutput> {
    results
        .iter()
        .filter_map(|result| {
            tree.get(result.id)
                .map(|record| record_to_output(record, tree, result.id, None))
        })
        .collect()
}

/// Convert a single record to output format.
fn record_to_output(
    record: &SchRecord,
    tree: &RecordTree<SchRecord>,
    id: altium_format::tree::RecordId,
    component_context: Option<&str>,
) -> QueryMatchOutput {
    match record {
        SchRecord::Component(c) => {
            let designator = get_designator_for_component(tree, id);
            QueryMatchOutput {
                record_type: "Component".to_string(),
                unique_id: designator.unwrap_or_else(|| "<none>".to_string()),
                description: Some(format!("{} - {}", c.lib_reference, c.component_description)),
                location: Some(format!(
                    "({}, {})",
                    c.graphical.location_x / 10000,
                    c.graphical.location_y / 10000
                )),
                net: None,
                children: Some(tree.child_count(id)),
            }
        }
        SchRecord::Pin(p) => {
            let comp_name = component_context.unwrap_or("?");
            QueryMatchOutput {
                record_type: "Pin".to_string(),
                unique_id: format!("{}.{}", comp_name, p.designator),
                description: Some(format!("{} [{:?}]", p.name, p.electrical)),
                location: Some(format!(
                    "({}, {})",
                    p.graphical.location_x / 10000,
                    p.graphical.location_y / 10000
                )),
                net: if p.hidden_net_name.is_empty() {
                    None
                } else {
                    Some(p.hidden_net_name.clone())
                },
                children: None,
            }
        }
        SchRecord::NetLabel(nl) => QueryMatchOutput {
            record_type: "NetLabel".to_string(),
            unique_id: nl.label.text.clone(),
            description: None,
            location: Some(format!(
                "({}, {})",
                nl.label.graphical.location_x / 10000,
                nl.label.graphical.location_y / 10000
            )),
            net: Some(nl.label.text.clone()),
            children: None,
        },
        SchRecord::PowerObject(p) => {
            let is_ground =
                p.text.to_uppercase().contains("GND") || p.text.to_uppercase().contains("VSS");
            QueryMatchOutput {
                record_type: if is_ground { "Ground" } else { "Power" }.to_string(),
                unique_id: p.text.clone(),
                description: Some(format!("[{:?}]", p.style)),
                location: Some(format!(
                    "({}, {})",
                    p.graphical.location_x / 10000,
                    p.graphical.location_y / 10000
                )),
                net: Some(p.text.clone()),
                children: None,
            }
        }
        SchRecord::Port(p) => QueryMatchOutput {
            record_type: "Port".to_string(),
            unique_id: p.name.clone(),
            description: Some(format!("[{:?}]", p.io_type)),
            location: Some(format!(
                "({}, {})",
                p.graphical.location_x / 10000,
                p.graphical.location_y / 10000
            )),
            net: None,
            children: None,
        },
        SchRecord::Wire(w) => {
            let vertex_count = w.vertices.len();
            let (start, end) = if vertex_count >= 2 {
                (
                    format!("({}, {})", w.vertices[0].0 / 10000, w.vertices[0].1 / 10000),
                    format!(
                        "({}, {})",
                        w.vertices[vertex_count - 1].0 / 10000,
                        w.vertices[vertex_count - 1].1 / 10000
                    ),
                )
            } else {
                ("?".to_string(), "?".to_string())
            };
            QueryMatchOutput {
                record_type: "Wire".to_string(),
                unique_id: format!("{} -> {}", start, end),
                description: Some(format!("{} vertices", vertex_count)),
                location: Some(start),
                net: None,
                children: None,
            }
        }
        SchRecord::Junction(j) => QueryMatchOutput {
            record_type: "Junction".to_string(),
            unique_id: format!(
                "@({},{})",
                j.graphical.location_x / 10000,
                j.graphical.location_y / 10000
            ),
            description: None,
            location: Some(format!(
                "({}, {})",
                j.graphical.location_x / 10000,
                j.graphical.location_y / 10000
            )),
            net: None,
            children: None,
        },
        SchRecord::Parameter(p) => QueryMatchOutput {
            record_type: "Parameter".to_string(),
            unique_id: p.name.clone(),
            description: Some(p.label.text.clone()),
            location: None,
            net: None,
            children: None,
        },
        SchRecord::Designator(d) => QueryMatchOutput {
            record_type: "Designator".to_string(),
            unique_id: d.param.label.text.clone(),
            description: None,
            location: None,
            net: None,
            children: None,
        },
        SchRecord::Label(l) => QueryMatchOutput {
            record_type: "Label".to_string(),
            unique_id: l.text.clone(),
            description: None,
            location: Some(format!(
                "({}, {})",
                l.graphical.location_x / 10000,
                l.graphical.location_y / 10000
            )),
            net: None,
            children: None,
        },
        SchRecord::SheetHeader(_) => QueryMatchOutput {
            record_type: "Sheet".to_string(),
            unique_id: component_context.unwrap_or("Sheet").to_string(),
            description: None,
            location: None,
            net: None,
            children: None,
        },
        _ => QueryMatchOutput {
            record_type: format!("{:?}", std::mem::discriminant(record)),
            unique_id: format!("[{}]", id.index()),
            description: None,
            location: None,
            net: None,
            children: None,
        },
    }
}

/// Get designator text for a component by looking up its Designator child.
fn get_designator_for_component(
    tree: &RecordTree<SchRecord>,
    component_id: altium_format::tree::RecordId,
) -> Option<String> {
    for (_child_id, child) in tree.children(component_id) {
        if let SchRecord::Designator(d) = child {
            return Some(d.param.label.text.clone());
        }
    }
    None
}

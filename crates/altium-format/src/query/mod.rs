//! Query systems for schematic records and documents.
//!
//! This module provides two complementary query systems:
//!
//! ## 1. Record Selector System (Low-Level)
//!
//! Domain-specific selector syntax for querying raw schematic records in a `RecordTree`.
//! Optimized for engineers working directly with component designators and part numbers.
//!
//! ### Syntax Overview
//!
//! | Pattern | Meaning | Example |
//! |---------|---------|---------|
//! | `U1` | Component by designator | Match component U1 |
//! | `R*` | Designator pattern | Match R1, R2, R100, etc. |
//! | `$LM358` | Component by part number | Match by lib_reference |
//! | `~VCC` | Net by name | Match net labels/power objects |
//! | `@10K` | Component by value | Match by Value parameter |
//! | `#Power` | Sheet by name | Match sheet (reserved) |
//! | `U1:3` | Pin by number | Pin 3 of component U1 |
//! | `U1:VCC` | Pin by name | VCC pin of component U1 |
//! | `R*@10K` | Combined query | 10K resistors |
//!
//! ```ignore
//! use altium_format::query::{query_records, SelectorParser, SelectorEngine};
//! let results = query_records(&tree, "R*@10K").unwrap();
//! ```
//!
//! ## 2. SchQL System (High-Level)
//!
//! CSS-style query language for querying schematic documents with computed connectivity.
//! Provides a `SchematicView` abstraction with nets, connections, and relationship queries.
//!
//! ### Syntax Overview
//!
//! | Pattern | Meaning |
//! |---------|---------|
//! | `component` | All components |
//! | `#U1` | Component with ID "U1" |
//! | `pin[type=input]` | Input pins |
//! | `net:power` | Power nets |
//! | `#U1 > pin` | Direct children (pins of U1) |
//! | `#U1 >> component` | Electrically connected components |
//! | `#VCC :: pin` | Pins on VCC net |
//! | `component:count` | Count of components |
//!
//! ```ignore
//! use altium_format::query::{SchematicView, SchematicQuery};
//! let view = SchematicView::from_schdoc(&doc);
//! let query = SchematicQuery::new(&view);
//! let result = query.query("component[part*=7805] > pin[type=input]").unwrap();
//! ```

// =============================================================================
// Shared Types (used by both systems)
// =============================================================================

mod common;

pub use common::{
    ElectricalFilter, ElectricalType, FilterOp, FilterValue as CommonFilterValue, VisibilityFilter,
    compare_filter,
};

// =============================================================================
// Record Selector System (raw RecordTree queries)
// =============================================================================

mod engine;
mod parser;
mod pattern;
mod selector;

pub use engine::{
    QueryMatch as RecordQueryMatch, SelectorEngine, query_records, query_records_with_doc_name,
};
pub use parser::{SelectorParser, parse as parse_selector};
pub use pattern::Pattern;
pub use selector::{
    Combinator, FilterOperator, FilterValue, NetConnectedTarget, PropertyFilter,
    PseudoSelector as RecordPseudoSelector, RecordMatcher, RecordType, Selector as RecordSelector,
    SelectorChain, SelectorSegment,
};

// =============================================================================
// SchQL System (high-level SchematicView queries)
// =============================================================================

mod ast;
mod executor;
mod schql_parser;
mod view;

pub use ast::*;
pub use executor::QueryExecutor;
pub use schql_parser::{QueryError, QueryParser};
pub use view::{
    ComponentView, ConnectionPoint, NetView, PinView, PortView, PowerView, SchematicView,
};

use crate::io::schdoc::SchDoc;

/// Query result containing matched elements from SchQL
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Matched elements
    pub matches: Vec<QueryMatch>,
    /// Original query string
    pub query: String,
    /// Execution time in microseconds
    pub execution_time_us: u64,
}

impl QueryResult {
    /// Check if query returned any results
    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    /// Get number of matches
    pub fn len(&self) -> usize {
        self.matches.len()
    }

    /// Render as concise list
    pub fn to_text(&self) -> String {
        if self.matches.is_empty() {
            return format!("Query `{}`: No matches\n", self.query);
        }

        // Check for count result
        if let Some(QueryMatch::Count(n)) = self.matches.first() {
            return format!("{}\n", n);
        }

        let mut output = format!(
            "Query `{}`: {} match{}\n",
            self.query,
            self.matches.len(),
            if self.matches.len() == 1 { "" } else { "es" }
        );
        for m in &self.matches {
            output.push_str(&format!("  {}\n", m.to_short_text()));
        }
        output
    }

    /// Render with full details
    pub fn to_detail_text(&self) -> String {
        if self.matches.is_empty() {
            return format!("Query `{}`: No matches\n", self.query);
        }

        let mut output = format!(
            "Query `{}`: {} match{}\n\n",
            self.query,
            self.matches.len(),
            if self.matches.len() == 1 { "" } else { "es" }
        );
        for m in &self.matches {
            output.push_str(&m.to_detail_text());
            output.push('\n');
        }
        output
    }
}

/// A matched element from a SchQL query
#[derive(Debug, Clone)]
pub enum QueryMatch {
    /// Component match
    Component {
        designator: String,
        part: String,
        description: String,
        value: Option<String>,
        footprint: Option<String>,
        pin_count: usize,
    },

    /// Pin match
    Pin {
        component_designator: String,
        designator: String,
        name: String,
        electrical_type: String,
        connected_net: Option<String>,
        is_hidden: bool,
    },

    /// Net match
    Net {
        name: String,
        is_power: bool,
        is_ground: bool,
        connection_count: usize,
        connections: Vec<String>,
    },

    /// Port match
    Port {
        name: String,
        io_type: String,
        connected_net: Option<String>,
    },

    /// Wire match
    Wire {
        index: usize,
        vertex_count: usize,
        start: (i32, i32),
        end: (i32, i32),
    },

    /// Power symbol match
    Power {
        net_name: String,
        style: String,
        is_ground: bool,
    },

    /// Net label match
    Label { text: String, location: (i32, i32) },

    /// Junction match
    Junction { location: (i32, i32) },

    /// Parameter match
    Parameter {
        component_designator: String,
        name: String,
        value: String,
    },

    /// Count result (for :count pseudo-selector)
    Count(usize),
}

impl QueryMatch {
    /// Render match as concise text
    pub fn to_short_text(&self) -> String {
        match self {
            QueryMatch::Component {
                designator,
                part,
                value,
                ..
            } => {
                if let Some(v) = value {
                    format!("{} ({}, {})", designator, part, v)
                } else {
                    format!("{} ({})", designator, part)
                }
            }
            QueryMatch::Pin {
                component_designator,
                designator,
                name,
                electrical_type,
                connected_net,
                ..
            } => {
                let net_str = connected_net.as_deref().unwrap_or("NC");
                format!(
                    "{}.{} \"{}\" [{}] -> {}",
                    component_designator, designator, name, electrical_type, net_str
                )
            }
            QueryMatch::Net {
                name,
                connection_count,
                is_power,
                is_ground,
                ..
            } => {
                let suffix = if *is_power {
                    " [PWR]"
                } else if *is_ground {
                    " [GND]"
                } else {
                    ""
                };
                format!("{}{} ({} connections)", name, suffix, connection_count)
            }
            QueryMatch::Port {
                name,
                io_type,
                connected_net,
            } => {
                let net_str = connected_net.as_deref().unwrap_or("?");
                format!("PORT {} [{}] -> {}", name, io_type, net_str)
            }
            QueryMatch::Wire {
                index,
                vertex_count,
                ..
            } => {
                format!("Wire #{} ({} vertices)", index, vertex_count)
            }
            QueryMatch::Power {
                net_name,
                style,
                is_ground,
            } => {
                let kind = if *is_ground { "GND" } else { "PWR" };
                format!("{} [{}] ({})", net_name, kind, style)
            }
            QueryMatch::Label { text, .. } => {
                format!("Label \"{}\"", text)
            }
            QueryMatch::Junction { location } => {
                format!(
                    "Junction @ ({}, {})",
                    location.0 / 10000,
                    location.1 / 10000
                )
            }
            QueryMatch::Parameter {
                component_designator,
                name,
                value,
            } => {
                format!("{}.{} = \"{}\"", component_designator, name, value)
            }
            QueryMatch::Count(n) => {
                format!("{}", n)
            }
        }
    }

    /// Render match as detailed text
    pub fn to_detail_text(&self) -> String {
        match self {
            QueryMatch::Component {
                designator,
                part,
                description,
                value,
                footprint,
                pin_count,
            } => {
                let mut s = format!("Component {}\n", designator);
                s.push_str(&format!("  Part: {}\n", part));
                if !description.is_empty() {
                    s.push_str(&format!("  Description: {}\n", description));
                }
                if let Some(v) = value {
                    s.push_str(&format!("  Value: {}\n", v));
                }
                if let Some(fp) = footprint {
                    s.push_str(&format!("  Footprint: {}\n", fp));
                }
                s.push_str(&format!("  Pins: {}\n", pin_count));
                s
            }
            QueryMatch::Net {
                name,
                connections,
                is_power,
                is_ground,
                ..
            } => {
                let mut s = format!("Net: {}", name);
                if *is_power {
                    s.push_str(" [POWER]");
                }
                if *is_ground {
                    s.push_str(" [GROUND]");
                }
                s.push('\n');
                for conn in connections {
                    s.push_str(&format!("  - {}\n", conn));
                }
                s
            }
            QueryMatch::Pin {
                component_designator,
                designator,
                name,
                electrical_type,
                connected_net,
                is_hidden,
            } => {
                let mut s = format!("Pin {}.{}\n", component_designator, designator);
                s.push_str(&format!("  Name: {}\n", name));
                s.push_str(&format!("  Type: {}\n", electrical_type));
                if let Some(net) = connected_net {
                    s.push_str(&format!("  Net: {}\n", net));
                } else {
                    s.push_str("  Net: (unconnected)\n");
                }
                if *is_hidden {
                    s.push_str("  Hidden: yes\n");
                }
                s
            }
            _ => self.to_short_text(),
        }
    }
}

/// Main query engine for schematic documents using SchQL
pub struct SchematicQuery<'a> {
    view: &'a SchematicView,
}

impl<'a> SchematicQuery<'a> {
    /// Create a new query engine for a schematic view
    pub fn new(view: &'a SchematicView) -> Self {
        Self { view }
    }

    /// Execute a query and return results
    pub fn query(&self, query_str: &str) -> Result<QueryResult, QueryError> {
        let start = std::time::Instant::now();

        // Parse the query
        let parser = QueryParser::new();
        let selector = parser.parse(query_str)?;

        // Execute the query
        let executor = QueryExecutor::new(self.view);
        let matches = executor.execute(&selector)?;

        Ok(QueryResult {
            matches,
            query: query_str.to_string(),
            execution_time_us: start.elapsed().as_micros() as u64,
        })
    }

    /// Execute multiple queries and return combined results
    pub fn query_batch(&self, queries: &[&str]) -> Vec<Result<QueryResult, QueryError>> {
        queries.iter().map(|q| self.query(q)).collect()
    }
}

/// Convenience function to query a SchDoc directly using SchQL
pub fn query_schdoc(doc: &SchDoc, query_str: &str) -> Result<QueryResult, QueryError> {
    let view = SchematicView::from_schdoc(doc);
    let engine = SchematicQuery::new(&view);
    engine.query(query_str)
}

#[cfg(test)]
mod tests;

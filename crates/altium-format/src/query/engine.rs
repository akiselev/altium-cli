//! Query execution engine for selector-based record queries.
//!
//! The `SelectorEngine` executes parsed selectors against record collections,
//! supporting all selector features including property filters, pseudo-selectors,
//! and parent/child combinators.
//!
//! # V2 Migration Note
//! This module uses v1 types (SchRecord, PinElectricalType, RecordTree<SchRecord>).
//! TODO: Migrate to v2 types when SchDocV2 provides equivalent query functionality.

#![allow(deprecated)]

use std::collections::HashMap;

use crate::error::Result;
use crate::records::sch::{PinElectricalType, SchRecord};
use crate::tree::{RecordId, RecordTree};

use super::common::{FilterValue as CommonFilterValue, compare_filter};
use super::pattern::Pattern;
use super::selector::{
    Combinator, FilterOperator, FilterValue, NetConnectedTarget, PropertyFilter, PseudoSelector,
    RecordMatcher, RecordType, Selector, SelectorChain, SelectorSegment,
};

/// Result of a query: record ID with optional metadata.
#[derive(Debug, Clone)]
pub struct QueryMatch {
    /// The ID of the matched record.
    pub id: RecordId,
    /// Depth in the tree (0 for roots).
    pub depth: usize,
}

impl QueryMatch {
    /// Create a new query match.
    pub fn new(id: RecordId, depth: usize) -> Self {
        Self { id, depth }
    }
}

/// Query execution engine for schematic records.
///
/// The engine operates on a `RecordTree<SchRecord>` and provides methods to
/// execute selector queries against the tree.
pub struct SelectorEngine<'a> {
    tree: &'a RecordTree<SchRecord>,
    /// Cache of component designators: component_id -> designator text
    designator_cache: HashMap<RecordId, String>,
    /// Cache of component values: component_id -> value text
    value_cache: HashMap<RecordId, String>,
    /// Cache of component part numbers: component_id -> lib_reference
    part_number_cache: HashMap<RecordId, String>,
    /// Cache of sheet names: sheet_header_id -> title text
    sheet_name_cache: HashMap<RecordId, String>,
}

impl<'a> SelectorEngine<'a> {
    /// Create a new selector engine for the given record tree.
    pub fn new(tree: &'a RecordTree<SchRecord>) -> Self {
        let mut engine = Self {
            tree,
            designator_cache: HashMap::new(),
            value_cache: HashMap::new(),
            part_number_cache: HashMap::new(),
            sheet_name_cache: HashMap::new(),
        };
        engine.build_caches();
        engine
    }

    /// Build lookup caches for efficient querying.
    fn build_caches(&mut self) {
        for (id, record) in self.tree.iter() {
            match record {
                SchRecord::Component(comp) => {
                    // Cache part number (lib_reference)
                    if !comp.lib_reference.is_empty() {
                        self.part_number_cache
                            .insert(id, comp.lib_reference.clone());
                    }
                }
                SchRecord::Designator(des) => {
                    // Cache designator text -> parent component
                    if let Some(parent_id) = self.tree.parent_id(id) {
                        self.designator_cache
                            .insert(parent_id, des.text().to_string());
                    }
                }
                SchRecord::Parameter(param) => {
                    // Cache "Value" parameter -> parent component
                    if param.name.eq_ignore_ascii_case("value") {
                        if let Some(parent_id) = self.tree.parent_id(id) {
                            self.value_cache
                                .insert(parent_id, param.value().to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Set the sheet name for a specific sheet header.
    /// This should be called with the document name (filename without extension),
    /// not the Title parameter which is typically the project title.
    pub fn set_sheet_name(&mut self, sheet_id: RecordId, name: String) {
        self.sheet_name_cache.insert(sheet_id, name);
    }

    /// Set the sheet name for all sheet headers in the tree.
    /// Convenience method for single-sheet documents where the document name
    /// applies to the single SheetHeader record.
    pub fn set_document_name(&mut self, name: String) {
        for (id, record) in self.tree.iter() {
            if matches!(record, SchRecord::SheetHeader(_)) {
                self.sheet_name_cache.insert(id, name.clone());
            }
        }
    }

    /// Execute a selector query and return matching record IDs.
    pub fn query(&self, selector: &Selector) -> Vec<QueryMatch> {
        if selector.is_empty() {
            return vec![];
        }

        let mut results = Vec::new();

        // Union of all alternatives (OR)
        for chain in &selector.alternatives {
            let chain_results = self.execute_chain(chain);
            for result in chain_results {
                // Avoid duplicates
                if !results.iter().any(|r: &QueryMatch| r.id == result.id) {
                    results.push(result);
                }
            }
        }

        results
    }

    /// Execute a single selector chain.
    fn execute_chain(&self, chain: &SelectorChain) -> Vec<QueryMatch> {
        if chain.segments.is_empty() {
            return vec![];
        }

        // Start with the first segment - find all matching records
        let mut current_matches: Vec<RecordId> = self.find_matching_records(&chain.segments[0]);

        // Process remaining segments with combinators
        for segment in chain.segments.iter().skip(1) {
            let combinator = chain.segments[chain
                .segments
                .iter()
                .position(|s| std::ptr::eq(s, segment))
                .map(|i| i.saturating_sub(1))
                .unwrap_or(0)]
            .combinator
            .unwrap_or(Combinator::Descendant);

            current_matches = self.apply_combinator(current_matches, segment, combinator);

            if current_matches.is_empty() {
                break;
            }
        }

        current_matches
            .into_iter()
            .map(|id| QueryMatch::new(id, self.tree.depth(id)))
            .collect()
    }

    /// Find all records matching a segment (without considering combinators).
    fn find_matching_records(&self, segment: &SelectorSegment) -> Vec<RecordId> {
        self.tree
            .iter()
            .filter(|(id, record)| self.matches_segment(*id, record, segment))
            .map(|(id, _)| id)
            .collect()
    }

    /// Apply a combinator to narrow down matches.
    fn apply_combinator(
        &self,
        from_ids: Vec<RecordId>,
        segment: &SelectorSegment,
        combinator: Combinator,
    ) -> Vec<RecordId> {
        let mut results = Vec::new();

        for from_id in from_ids {
            match combinator {
                Combinator::DirectChild => {
                    // Only match direct children
                    for (child_id, child_record) in self.tree.children(from_id) {
                        if self.matches_segment(child_id, child_record, segment) {
                            results.push(child_id);
                        }
                    }
                }
                Combinator::Descendant => {
                    // Match any descendant
                    for (desc_id, desc_record) in self.tree.descendants(from_id) {
                        if self.matches_segment(desc_id, desc_record, segment) {
                            results.push(desc_id);
                        }
                    }
                }
            }
        }

        results
    }

    /// Check if a record matches a selector segment.
    fn matches_segment(&self, id: RecordId, record: &SchRecord, segment: &SelectorSegment) -> bool {
        // Check main matcher
        if !self.matches_record_matcher(id, record, &segment.matcher) {
            return false;
        }

        // Check property filters
        for filter in &segment.filters {
            if !self.matches_property_filter(id, record, filter) {
                return false;
            }
        }

        // Check pseudo-selectors
        for pseudo in &segment.pseudo {
            if !self.matches_pseudo_selector(id, record, pseudo) {
                return false;
            }
        }

        true
    }

    /// Check if a record matches a record matcher.
    fn matches_record_matcher(
        &self,
        id: RecordId,
        record: &SchRecord,
        matcher: &RecordMatcher,
    ) -> bool {
        match matcher {
            RecordMatcher::Any => true,

            RecordMatcher::Type(record_type) => self.record_has_type(record, *record_type),

            RecordMatcher::Designator(pattern) => {
                // Match component by its designator
                if let SchRecord::Component(_) = record {
                    if let Some(designator) = self.designator_cache.get(&id) {
                        return pattern.matches(designator);
                    }
                }
                false
            }

            RecordMatcher::PartNumber(pattern) => {
                // Match component by lib_reference
                if let SchRecord::Component(comp) = record {
                    return pattern.matches(&comp.lib_reference);
                }
                false
            }

            RecordMatcher::Net(pattern) => {
                // Match net labels and power objects by net name
                match record {
                    SchRecord::NetLabel(nl) => pattern.matches(&nl.label.text),
                    SchRecord::PowerObject(po) => pattern.matches(&po.text),
                    _ => false,
                }
            }

            RecordMatcher::Value(pattern) => {
                // Match component by its Value parameter
                if let SchRecord::Component(_) = record {
                    if let Some(value) = self.value_cache.get(&id) {
                        return pattern.matches(value);
                    }
                }
                false
            }

            RecordMatcher::Sheet(pattern) => {
                // Match sheet header by name/title
                if let SchRecord::SheetHeader(_header) = record {
                    // Check if we have a cached title for this sheet
                    if let Some(title) = self.sheet_name_cache.get(&id) {
                        return pattern.matches(title);
                    }
                    // If no title found, match against empty string (matches wildcard "*")
                    return pattern.matches("");
                }
                false
            }

            RecordMatcher::Pin { component, pin } => {
                // Match pins by component designator and pin designator/name
                if let SchRecord::Pin(pin_record) = record {
                    // Get parent component
                    if let Some(parent_id) = self.tree.parent_id(id) {
                        // First check if parent is a component
                        if let Some(SchRecord::Component(_)) = self.tree.get(parent_id) {
                            // Check component designator
                            let comp_matches =
                                if let Some(designator) = self.designator_cache.get(&parent_id) {
                                    component.matches(designator)
                                } else {
                                    // No designator, only match if component pattern is wildcard
                                    component.as_str() == "*"
                                };

                            if comp_matches {
                                // Check pin designator or name
                                return pin.matches(&pin_record.designator)
                                    || pin.matches(&pin_record.name);
                            }
                        }
                    }
                }
                false
            }

            RecordMatcher::NetConnected { net, target } => {
                // This is a complex query that requires net connectivity analysis
                // For now, we return records connected to the named net
                self.matches_net_connected(id, record, net, *target)
            }

            RecordMatcher::DesignatorWithValue { designator, value } => {
                // Match component by both designator and value
                if let SchRecord::Component(_) = record {
                    let des_matches = self
                        .designator_cache
                        .get(&id)
                        .map(|d| designator.matches(d))
                        .unwrap_or(false);

                    let val_matches = self
                        .value_cache
                        .get(&id)
                        .map(|v| value.matches(v))
                        .unwrap_or(false);

                    return des_matches && val_matches;
                }
                false
            }
        }
    }

    /// Check if a record has the specified type.
    fn record_has_type(&self, record: &SchRecord, record_type: RecordType) -> bool {
        matches!(
            (record, record_type),
            (SchRecord::Component(_), RecordType::Component)
                | (SchRecord::Pin(_), RecordType::Pin)
                | (SchRecord::Wire(_), RecordType::Wire)
                | (SchRecord::NetLabel(_), RecordType::NetLabel)
                | (SchRecord::Port(_), RecordType::Port)
                | (SchRecord::PowerObject(_), RecordType::PowerObject)
                | (SchRecord::Junction(_), RecordType::Junction)
                | (SchRecord::Label(_), RecordType::Label)
                | (SchRecord::Rectangle(_), RecordType::Rectangle)
                | (SchRecord::Line(_), RecordType::Line)
                | (SchRecord::Arc(_), RecordType::Arc)
                | (SchRecord::Ellipse(_), RecordType::Ellipse)
                | (SchRecord::Polygon(_), RecordType::Polygon)
                | (SchRecord::Polyline(_), RecordType::Polyline)
                | (SchRecord::Bezier(_), RecordType::Bezier)
                | (SchRecord::Image(_), RecordType::Image)
                | (SchRecord::Parameter(_), RecordType::Parameter)
                | (SchRecord::SheetHeader(_), RecordType::Sheet)
                | (SchRecord::Symbol(_), RecordType::Symbol)
                | (SchRecord::Designator(_), RecordType::Designator)
                | (SchRecord::TextFrame(_), RecordType::TextFrame)
                | (SchRecord::TextFrameVariant(_), RecordType::TextFrame)
        )
    }

    /// Check if a record matches a net connectivity query.
    fn matches_net_connected(
        &self,
        id: RecordId,
        record: &SchRecord,
        net_pattern: &Pattern,
        target: NetConnectedTarget,
    ) -> bool {
        // Find all net labels/power objects matching the pattern
        let net_names: Vec<String> = self
            .tree
            .iter()
            .filter_map(|(_, r)| match r {
                SchRecord::NetLabel(nl) if net_pattern.matches(&nl.label.text) => {
                    Some(nl.label.text.clone())
                }
                SchRecord::PowerObject(po) if net_pattern.matches(&po.text) => {
                    Some(po.text.clone())
                }
                _ => None,
            })
            .collect();

        if net_names.is_empty() {
            return false;
        }

        // Check if current record is connected to one of these nets
        // This is a simplified implementation - full net connectivity requires
        // geometric analysis
        match (target, record) {
            (NetConnectedTarget::Pins, SchRecord::Pin(pin)) => {
                // Check if pin has a hidden net name matching
                net_names
                    .iter()
                    .any(|n| n.eq_ignore_ascii_case(&pin.hidden_net_name))
            }
            (NetConnectedTarget::Components, SchRecord::Component(_)) => {
                // Check if any child pins are connected to the net
                for (_child_id, child) in self.tree.children(id) {
                    if let SchRecord::Pin(pin) = child {
                        if net_names
                            .iter()
                            .any(|n| n.eq_ignore_ascii_case(&pin.hidden_net_name))
                        {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Check if a record matches a property filter.
    fn matches_property_filter(
        &self,
        id: RecordId,
        record: &SchRecord,
        filter: &PropertyFilter,
    ) -> bool {
        // Get the property value from the record
        let prop_value = self.get_property(id, record, &filter.property);

        match prop_value {
            Some(value) => self.compare_filter_value(&value, &filter.operator, &filter.value),
            None => false,
        }
    }

    /// Get a property value from a record.
    fn get_property(&self, id: RecordId, record: &SchRecord, property: &str) -> Option<String> {
        let prop_lower = property.to_lowercase();

        // Common properties across record types
        match prop_lower.as_str() {
            "x" | "location.x" => return self.get_location_x(record),
            "y" | "location.y" => return self.get_location_y(record),
            "owner_index" | "ownerindex" => return Some(self.get_owner_index(record).to_string()),
            "hidden" => return Some(self.is_hidden(record).to_string()),
            _ => {}
        }

        // Record-specific properties
        match record {
            SchRecord::Component(comp) => match prop_lower.as_str() {
                "lib_reference" | "libreference" => Some(comp.lib_reference.clone()),
                "description" | "componentdescription" => Some(comp.component_description.clone()),
                "unique_id" | "uniqueid" => Some(comp.unique_id.clone()),
                "part_count" | "partcount" => Some(comp.part_count.to_string()),
                "designator" => self.designator_cache.get(&id).cloned(),
                "value" => self.value_cache.get(&id).cloned(),
                _ => None,
            },
            SchRecord::Pin(pin) => match prop_lower.as_str() {
                "designator" => Some(pin.designator.clone()),
                "name" => Some(pin.name.clone()),
                "electrical" => Some(format!("{:?}", pin.electrical)),
                "length" | "pin_length" | "pinlength" => Some(pin.pin_length.to_string()),
                "description" => Some(pin.description.clone()),
                _ => None,
            },
            SchRecord::NetLabel(nl) => match prop_lower.as_str() {
                "text" | "net" | "name" => Some(nl.label.text.clone()),
                _ => None,
            },
            SchRecord::PowerObject(po) => match prop_lower.as_str() {
                "text" | "net" | "name" => Some(po.text.clone()),
                "style" => Some(format!("{:?}", po.style)),
                _ => None,
            },
            SchRecord::Parameter(param) => match prop_lower.as_str() {
                "name" => Some(param.name.clone()),
                "value" | "text" => Some(param.value().to_string()),
                _ => None,
            },
            SchRecord::Label(label) => match prop_lower.as_str() {
                "text" => Some(label.text.clone()),
                _ => None,
            },
            SchRecord::Port(port) => match prop_lower.as_str() {
                "name" => Some(port.name.clone()),
                "io_type" | "iotype" => Some(format!("{:?}", port.io_type)),
                "style" => Some(format!("{:?}", port.style)),
                _ => None,
            },
            SchRecord::SheetHeader(_header) => match prop_lower.as_str() {
                "name" | "title" => self.sheet_name_cache.get(&id).cloned(),
                _ => None,
            },
            _ => None,
        }
    }

    /// Get location X from a record.
    fn get_location_x(&self, record: &SchRecord) -> Option<String> {
        match record {
            SchRecord::Component(c) => Some(c.graphical.location_x.to_string()),
            SchRecord::Pin(p) => Some(p.graphical.location_x.to_string()),
            SchRecord::Symbol(s) => Some(s.graphical.location_x.to_string()),
            SchRecord::Label(l) => Some(l.graphical.location_x.to_string()),
            SchRecord::NetLabel(nl) => Some(nl.label.graphical.location_x.to_string()),
            SchRecord::PowerObject(po) => Some(po.graphical.location_x.to_string()),
            SchRecord::Port(p) => Some(p.graphical.location_x.to_string()),
            SchRecord::Junction(j) => Some(j.graphical.location_x.to_string()),
            SchRecord::Rectangle(r) => Some(r.graphical.location_x.to_string()),
            SchRecord::Line(l) => Some(l.graphical.location_x.to_string()),
            SchRecord::Arc(a) => Some(a.graphical.location_x.to_string()),
            SchRecord::Ellipse(e) => Some(e.graphical.location_x.to_string()),
            _ => None,
        }
    }

    /// Get location Y from a record.
    fn get_location_y(&self, record: &SchRecord) -> Option<String> {
        match record {
            SchRecord::Component(c) => Some(c.graphical.location_y.to_string()),
            SchRecord::Pin(p) => Some(p.graphical.location_y.to_string()),
            SchRecord::Symbol(s) => Some(s.graphical.location_y.to_string()),
            SchRecord::Label(l) => Some(l.graphical.location_y.to_string()),
            SchRecord::NetLabel(nl) => Some(nl.label.graphical.location_y.to_string()),
            SchRecord::PowerObject(po) => Some(po.graphical.location_y.to_string()),
            SchRecord::Port(p) => Some(p.graphical.location_y.to_string()),
            SchRecord::Junction(j) => Some(j.graphical.location_y.to_string()),
            SchRecord::Rectangle(r) => Some(r.graphical.location_y.to_string()),
            SchRecord::Line(l) => Some(l.graphical.location_y.to_string()),
            SchRecord::Arc(a) => Some(a.graphical.location_y.to_string()),
            SchRecord::Ellipse(e) => Some(e.graphical.location_y.to_string()),
            _ => None,
        }
    }

    /// Get owner index from a record.
    fn get_owner_index(&self, record: &SchRecord) -> i32 {
        match record {
            SchRecord::Component(c) => c.graphical.base.owner_index,
            SchRecord::Pin(p) => p.graphical.base.owner_index,
            SchRecord::Symbol(s) => s.graphical.base.owner_index,
            SchRecord::Label(l) => l.graphical.base.owner_index,
            SchRecord::NetLabel(nl) => nl.label.graphical.base.owner_index,
            SchRecord::PowerObject(po) => po.graphical.base.owner_index,
            SchRecord::Port(p) => p.graphical.base.owner_index,
            SchRecord::Junction(j) => j.graphical.base.owner_index,
            _ => -1,
        }
    }

    /// Check if a record is hidden.
    fn is_hidden(&self, record: &SchRecord) -> bool {
        match record {
            SchRecord::Pin(p) => p.is_hidden(),
            SchRecord::Label(l) => l.is_hidden,
            _ => false,
        }
    }

    /// Compare a property value against a filter.
    ///
    /// Uses the shared comparison logic from common.rs.
    fn compare_filter_value(
        &self,
        actual: &str,
        operator: &FilterOperator,
        expected: &FilterValue,
    ) -> bool {
        // Convert to common types
        let filter_op = operator.to_filter_op();
        let common_value = match expected {
            FilterValue::String(s) => CommonFilterValue::String(s.clone()),
            FilterValue::Number(n) => CommonFilterValue::Number(*n),
            FilterValue::Bool(b) => CommonFilterValue::Bool(*b),
            FilterValue::Pattern(p) => CommonFilterValue::Pattern(p.clone()),
        };

        // Special case: Wildcard with Pattern uses pattern matching directly
        if matches!(operator, FilterOperator::Wildcard) {
            if let FilterValue::Pattern(p) = expected {
                return p.matches(actual);
            }
            if let FilterValue::String(s) = expected {
                return Pattern::new(s).map(|p| p.matches(actual)).unwrap_or(false);
            }
        }

        compare_filter(Some(actual), filter_op, &common_value, true)
    }

    /// Check if a record matches a pseudo-selector.
    fn matches_pseudo_selector(
        &self,
        id: RecordId,
        record: &SchRecord,
        pseudo: &PseudoSelector,
    ) -> bool {
        match pseudo {
            PseudoSelector::Root => self.tree.is_root(id),
            PseudoSelector::Empty => !self.tree.has_children(id),
            PseudoSelector::FirstChild => self.is_first_child(id),
            PseudoSelector::LastChild => self.is_last_child(id),
            PseudoSelector::NthChild(n) => self.is_nth_child(id, *n),
            PseudoSelector::OnlyChild => self.is_only_child(id),

            // Electrical state (for pins)
            PseudoSelector::Connected => self.is_pin_connected(record),
            PseudoSelector::Unconnected => !self.is_pin_connected(record),
            PseudoSelector::Input => self.pin_has_electrical(record, PinElectricalType::Input),
            PseudoSelector::Output => self.pin_has_electrical(record, PinElectricalType::Output),
            PseudoSelector::Bidirectional => {
                self.pin_has_electrical(record, PinElectricalType::InputOutput)
            }
            PseudoSelector::Power => self.pin_has_electrical(record, PinElectricalType::Power),
            PseudoSelector::Passive => self.pin_has_electrical(record, PinElectricalType::Passive),
            PseudoSelector::OpenCollector => {
                self.pin_has_electrical(record, PinElectricalType::OpenCollector)
            }
            PseudoSelector::OpenEmitter => {
                self.pin_has_electrical(record, PinElectricalType::OpenEmitter)
            }
            PseudoSelector::HiZ => self.pin_has_electrical(record, PinElectricalType::HiZ),

            // Visibility
            PseudoSelector::Visible => !self.is_hidden(record),
            PseudoSelector::Hidden => self.is_hidden(record),
            PseudoSelector::Selected => false, // UI state - not supported in query engine

            // Combinatorial
            PseudoSelector::Not(selector) => {
                let matches = self.query(selector);
                !matches.iter().any(|m| m.id == id)
            }
            PseudoSelector::Has(selector) => {
                // Check if any descendant matches
                for (desc_id, _) in self.tree.descendants(id) {
                    let desc_matches = self.query(selector);
                    if desc_matches.iter().any(|m| m.id == desc_id) {
                        return true;
                    }
                }
                false
            }
            PseudoSelector::Is(selector) => {
                let matches = self.query(selector);
                matches.iter().any(|m| m.id == id)
            }
        }
    }

    /// Check if record is the first child of its parent.
    fn is_first_child(&self, id: RecordId) -> bool {
        if let Some(parent_id) = self.tree.parent_id(id) {
            if let Some((first_id, _)) = self.tree.children(parent_id).next() {
                return first_id == id;
            }
        }
        false
    }

    /// Check if record is the last child of its parent.
    fn is_last_child(&self, id: RecordId) -> bool {
        if let Some(parent_id) = self.tree.parent_id(id) {
            if let Some((last_id, _)) = self.tree.children(parent_id).last() {
                return last_id == id;
            }
        }
        false
    }

    /// Check if record is the nth child (1-indexed).
    fn is_nth_child(&self, id: RecordId, n: usize) -> bool {
        if let Some(parent_id) = self.tree.parent_id(id) {
            if let Some((nth_id, _)) = self.tree.children(parent_id).nth(n.saturating_sub(1)) {
                return nth_id == id;
            }
        }
        false
    }

    /// Check if record is the only child of its parent.
    fn is_only_child(&self, id: RecordId) -> bool {
        if let Some(parent_id) = self.tree.parent_id(id) {
            return self.tree.child_count(parent_id) == 1;
        }
        false
    }

    /// Check if a pin is connected to a net.
    fn is_pin_connected(&self, record: &SchRecord) -> bool {
        if let SchRecord::Pin(pin) = record {
            // A pin is considered connected if it has a hidden net name
            return !pin.hidden_net_name.is_empty();
        }
        false
    }

    /// Check if a pin has a specific electrical type.
    fn pin_has_electrical(&self, record: &SchRecord, electrical: PinElectricalType) -> bool {
        if let SchRecord::Pin(pin) = record {
            return pin.electrical == electrical;
        }
        false
    }

    /// Get the designator of a component by ID.
    pub fn get_component_designator(&self, id: RecordId) -> Option<&str> {
        self.designator_cache.get(&id).map(|s| s.as_str())
    }

    /// Get the value of a component by ID.
    pub fn get_component_value(&self, id: RecordId) -> Option<&str> {
        self.value_cache.get(&id).map(|s| s.as_str())
    }

    /// Get the part number (lib_reference) of a component by ID.
    pub fn get_component_part_number(&self, id: RecordId) -> Option<&str> {
        self.part_number_cache.get(&id).map(|s| s.as_str())
    }
}

/// Convenience function to parse and execute a selector query.
pub fn query_records(tree: &RecordTree<SchRecord>, selector_str: &str) -> Result<Vec<QueryMatch>> {
    query_records_with_doc_name(tree, selector_str, None)
}

/// Convenience function to parse and execute a selector query with optional document name.
/// The document name is used as the sheet name for sheet queries (not the Title parameter).
pub fn query_records_with_doc_name(
    tree: &RecordTree<SchRecord>,
    selector_str: &str,
    document_name: Option<&str>,
) -> Result<Vec<QueryMatch>> {
    use super::parser::SelectorParser;

    let parser = SelectorParser::new(selector_str);
    let selector = parser.parse()?;
    let mut engine = SelectorEngine::new(tree);

    // Set document name for all sheet headers if provided
    if let Some(name) = document_name {
        engine.set_document_name(name.to_string());
    }

    Ok(engine.query(&selector))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::sch::{SchComponent, SchDesignator, SchNetLabel, SchParameter, SchPin};

    // Helper to create a test tree
    fn create_test_tree() -> RecordTree<SchRecord> {
        let mut records = Vec::new();

        // Component U1 at index 0
        let mut comp = SchComponent::default();
        comp.lib_reference = "LM358".to_string();
        comp.graphical.base.owner_index = -1;
        records.push(SchRecord::Component(comp));

        // Designator for U1 at index 1
        let mut des = SchDesignator::default();
        des.param.label.text = "U1".to_string();
        des.param.label.graphical.base.owner_index = 0;
        records.push(SchRecord::Designator(des));

        // Value parameter at index 2
        let mut param = SchParameter::default();
        param.name = "Value".to_string();
        param.label.text = "OpAmp".to_string();
        param.label.graphical.base.owner_index = 0;
        records.push(SchRecord::Parameter(param));

        // Pin 1 at index 3
        let mut pin1 = SchPin::default();
        pin1.designator = "1".to_string();
        pin1.name = "IN+".to_string();
        pin1.electrical = PinElectricalType::Input;
        pin1.graphical.base.owner_index = 0;
        records.push(SchRecord::Pin(pin1));

        // Pin 2 at index 4
        let mut pin2 = SchPin::default();
        pin2.designator = "2".to_string();
        pin2.name = "OUT".to_string();
        pin2.electrical = PinElectricalType::Output;
        pin2.graphical.base.owner_index = 0;
        records.push(SchRecord::Pin(pin2));

        // Component R1 at index 5
        let mut comp2 = SchComponent::default();
        comp2.lib_reference = "Resistor".to_string();
        comp2.graphical.base.owner_index = -1;
        records.push(SchRecord::Component(comp2));

        // Designator for R1 at index 6
        let mut des2 = SchDesignator::default();
        des2.param.label.text = "R1".to_string();
        des2.param.label.graphical.base.owner_index = 5;
        records.push(SchRecord::Designator(des2));

        // Value parameter at index 7
        let mut param2 = SchParameter::default();
        param2.name = "Value".to_string();
        param2.label.text = "10K".to_string();
        param2.label.graphical.base.owner_index = 5;
        records.push(SchRecord::Parameter(param2));

        // NetLabel at index 8
        let mut netlabel = SchNetLabel::default();
        netlabel.label.text = "VCC".to_string();
        netlabel.label.graphical.base.owner_index = -1;
        records.push(SchRecord::NetLabel(netlabel));

        RecordTree::from_records(records)
    }

    #[test]
    fn test_query_by_designator() {
        let tree = create_test_tree();
        let results = query_records(&tree, "U1").unwrap();

        assert_eq!(results.len(), 1);
        if let Some(SchRecord::Component(comp)) = tree.get(results[0].id) {
            assert_eq!(comp.lib_reference, "LM358");
        } else {
            panic!("Expected component");
        }
    }

    #[test]
    fn test_query_by_designator_pattern() {
        let tree = create_test_tree();
        let results = query_records(&tree, "R*").unwrap();

        assert_eq!(results.len(), 1);
        if let Some(SchRecord::Component(comp)) = tree.get(results[0].id) {
            assert_eq!(comp.lib_reference, "Resistor");
        } else {
            panic!("Expected component");
        }
    }

    #[test]
    fn test_query_by_part_number() {
        let tree = create_test_tree();
        let results = query_records(&tree, "$LM358").unwrap();

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_query_by_value() {
        let tree = create_test_tree();
        let results = query_records(&tree, "@10K").unwrap();

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_query_by_net() {
        let tree = create_test_tree();
        let results = query_records(&tree, "~VCC").unwrap();

        assert_eq!(results.len(), 1);
        if let Some(SchRecord::NetLabel(nl)) = tree.get(results[0].id) {
            assert_eq!(nl.label.text, "VCC");
        } else {
            panic!("Expected net label");
        }
    }

    #[test]
    fn test_query_pin() {
        let tree = create_test_tree();
        let results = query_records(&tree, "U1:1").unwrap();

        assert_eq!(results.len(), 1);
        if let Some(SchRecord::Pin(pin)) = tree.get(results[0].id) {
            assert_eq!(pin.designator, "1");
        } else {
            panic!("Expected pin");
        }
    }

    #[test]
    fn test_query_pin_by_name() {
        let tree = create_test_tree();
        let results = query_records(&tree, "U1:OUT").unwrap();

        assert_eq!(results.len(), 1);
        if let Some(SchRecord::Pin(pin)) = tree.get(results[0].id) {
            assert_eq!(pin.name, "OUT");
        } else {
            panic!("Expected pin");
        }
    }

    #[test]
    fn test_query_by_type() {
        let tree = create_test_tree();
        let results = query_records(&tree, "pin").unwrap();

        assert_eq!(results.len(), 2); // Two pins in the test tree
    }

    #[test]
    fn test_query_alternatives() {
        let tree = create_test_tree();
        let results = query_records(&tree, "U1, R1").unwrap();

        assert_eq!(results.len(), 2); // Both components
    }

    #[test]
    fn test_query_with_pseudo_selector() {
        let tree = create_test_tree();
        let results = query_records(&tree, "pin:input").unwrap();

        assert_eq!(results.len(), 1);
        if let Some(SchRecord::Pin(pin)) = tree.get(results[0].id) {
            assert_eq!(pin.electrical, PinElectricalType::Input);
        } else {
            panic!("Expected pin");
        }
    }

    #[test]
    fn test_query_sheet_by_name() {
        use crate::records::sch::SchSheetHeader;

        let mut records = Vec::new();

        // Create a sheet header at index 0
        let mut sheet = SchSheetHeader::default();
        sheet.base.owner_index = -1;
        records.push(SchRecord::SheetHeader(sheet));

        let tree = RecordTree::from_records(records);

        // Test 1: Match by exact name with property filter
        let results =
            query_records_with_doc_name(&tree, "sheet[name='PowerSupply']", Some("PowerSupply"))
                .unwrap();
        assert_eq!(results.len(), 1);

        // Test 2: Match by wildcard pattern with property filter
        let results =
            query_records_with_doc_name(&tree, "sheet[name~='Power*']", Some("PowerSupply"))
                .unwrap();
        assert_eq!(results.len(), 1);

        // Test 3: No match with wrong name
        let results =
            query_records_with_doc_name(&tree, "sheet[name='Other']", Some("PowerSupply")).unwrap();
        assert_eq!(results.len(), 0);

        // Test 4: Match all sheets with type selector
        let results = query_records(&tree, "sheet").unwrap();
        assert_eq!(results.len(), 1);

        // Test 5: Match by exact name with # shortcut (SheetName matcher)
        let results =
            query_records_with_doc_name(&tree, "#PowerSupply", Some("PowerSupply")).unwrap();
        assert_eq!(results.len(), 1);

        // Test 6: Match by wildcard with # shortcut
        let results = query_records_with_doc_name(&tree, "#Power*", Some("PowerSupply")).unwrap();
        assert_eq!(results.len(), 1);

        // Test 7: No match with # shortcut
        let results = query_records_with_doc_name(&tree, "#Other", Some("PowerSupply")).unwrap();
        assert_eq!(results.len(), 0);

        // Test 8: Without document name set, sheet has no name (empty string)
        let results = query_records(&tree, "sheet[name='PowerSupply']").unwrap();
        assert_eq!(
            results.len(),
            0,
            "Should not match when document name not set"
        );

        // Test 9: Wildcard "*" matches sheets even without name
        let results = query_records(&tree, "#*").unwrap();
        assert_eq!(
            results.len(),
            1,
            "Wildcard should match sheets without name"
        );
    }
}

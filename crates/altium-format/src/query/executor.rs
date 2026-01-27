//! Query executor - matches selectors against schematic elements.

use std::collections::HashSet;

use super::QueryMatch;
use super::ast::*;
use super::schql_parser::QueryError;
use super::view::*;

/// Query executor that matches selectors against a schematic view
pub struct QueryExecutor<'a> {
    view: &'a SchematicView,
}

impl<'a> QueryExecutor<'a> {
    /// Create a new executor for a schematic view
    pub fn new(view: &'a SchematicView) -> Self {
        Self { view }
    }

    /// Execute a selector and return matches
    pub fn execute(&self, selector: &Selector) -> Result<Vec<QueryMatch>, QueryError> {
        let matches = self.execute_selector(selector)?;

        // Apply result modifiers
        let matches = self.apply_result_modifiers(matches, selector);

        Ok(matches)
    }

    fn execute_selector(&self, selector: &Selector) -> Result<Vec<QueryMatch>, QueryError> {
        match selector {
            Selector::Element(elem_type) => self.select_element(*elem_type),
            Selector::Id(id) => self.select_id(id),
            Selector::Universal => self.select_all(),
            Selector::Attribute(attr) => self.filter_by_attribute(&self.select_all()?, attr),
            Selector::Pseudo(pseudo) => self.filter_by_pseudo(&self.select_all()?, pseudo),
            Selector::Compound(parts) => self.select_compound(parts),
            Selector::Combinator {
                left,
                combinator,
                right,
            } => self.select_combinator(left, *combinator, right),
            Selector::Union(selectors) => self.select_union(selectors),
            Selector::Not(inner) => self.select_not(inner),
            Selector::Has(inner) => self.select_has(inner),
        }
    }

    fn select_element(&self, elem_type: ElementType) -> Result<Vec<QueryMatch>, QueryError> {
        match elem_type {
            ElementType::Component => Ok(self
                .view
                .components
                .iter()
                .map(|c| self.component_to_match(c))
                .collect()),
            ElementType::Pin => Ok(self.all_pins()),
            ElementType::Net => Ok(self
                .view
                .nets
                .iter()
                .map(|n| self.net_to_match(n))
                .collect()),
            ElementType::Port => Ok(self
                .view
                .ports
                .iter()
                .map(|p| self.port_to_match(p))
                .collect()),
            ElementType::Wire => Ok(self
                .view
                .wires
                .iter()
                .enumerate()
                .map(|(i, w)| self.wire_to_match(i, w))
                .collect()),
            ElementType::Power => Ok(self
                .view
                .power_symbols
                .iter()
                .filter(|p| !p.is_ground)
                .map(|p| self.power_to_match(p))
                .collect()),
            ElementType::Ground => Ok(self
                .view
                .power_symbols
                .iter()
                .filter(|p| p.is_ground)
                .map(|p| self.power_to_match(p))
                .collect()),
            ElementType::Label => Ok(self
                .view
                .labels
                .iter()
                .map(|l| self.label_to_match(l))
                .collect()),
            ElementType::Junction => Ok(self
                .view
                .junctions
                .iter()
                .map(|j| self.junction_to_match(j))
                .collect()),
            ElementType::Parameter => Ok(self.all_parameters()),
            ElementType::Designator => Ok(self
                .view
                .components
                .iter()
                .map(|c| QueryMatch::Parameter {
                    component_designator: c.designator.clone(),
                    name: "Designator".to_string(),
                    value: c.designator.clone(),
                })
                .collect()),
            ElementType::Sheet => {
                // Return sheet info - just one match
                Ok(vec![QueryMatch::Component {
                    designator: "Sheet".to_string(),
                    part: self.view.sheet_name.clone().unwrap_or_default(),
                    description: format!(
                        "{} components, {} nets",
                        self.view.components.len(),
                        self.view.nets.len()
                    ),
                    value: None,
                    footprint: None,
                    pin_count: 0,
                }])
            }
        }
    }

    fn select_id(&self, id: &str) -> Result<Vec<QueryMatch>, QueryError> {
        let id_upper = id.to_uppercase();

        // Try component designator
        if let Some(comp) = self.view.get_component(id) {
            return Ok(vec![self.component_to_match(comp)]);
        }
        // Case-insensitive search
        for comp in &self.view.components {
            if comp.designator.to_uppercase() == id_upper {
                return Ok(vec![self.component_to_match(comp)]);
            }
        }

        // Try net name
        if let Some(net) = self.view.get_net(id) {
            return Ok(vec![self.net_to_match(net)]);
        }
        // Case-insensitive search
        for net in &self.view.nets {
            if net.name.to_uppercase() == id_upper {
                return Ok(vec![self.net_to_match(net)]);
            }
        }

        // Try port name
        for port in &self.view.ports {
            if port.name.to_uppercase() == id_upper {
                return Ok(vec![self.port_to_match(port)]);
            }
        }

        // Try power symbol net name
        for pwr in &self.view.power_symbols {
            if pwr.net_name.to_uppercase() == id_upper {
                return Ok(vec![self.power_to_match(pwr)]);
            }
        }

        Ok(vec![])
    }

    fn select_all(&self) -> Result<Vec<QueryMatch>, QueryError> {
        let mut matches = Vec::new();
        matches.extend(
            self.view
                .components
                .iter()
                .map(|c| self.component_to_match(c)),
        );
        matches.extend(self.all_pins());
        matches.extend(self.view.nets.iter().map(|n| self.net_to_match(n)));
        matches.extend(self.view.ports.iter().map(|p| self.port_to_match(p)));
        matches.extend(
            self.view
                .power_symbols
                .iter()
                .map(|p| self.power_to_match(p)),
        );
        Ok(matches)
    }

    fn select_compound(&self, parts: &[Selector]) -> Result<Vec<QueryMatch>, QueryError> {
        if parts.is_empty() {
            return Ok(vec![]);
        }

        // Start with first selector
        let mut matches = self.execute_selector(&parts[0])?;

        // Apply remaining selectors as filters
        for part in &parts[1..] {
            matches = self.filter_matches(matches, part)?;
        }

        Ok(matches)
    }

    fn filter_matches(
        &self,
        matches: Vec<QueryMatch>,
        selector: &Selector,
    ) -> Result<Vec<QueryMatch>, QueryError> {
        match selector {
            Selector::Attribute(attr) => self.filter_by_attribute(&matches, attr),
            Selector::Pseudo(pseudo) => self.filter_by_pseudo(&matches, pseudo),
            Selector::Element(elem_type) => Ok(matches
                .into_iter()
                .filter(|m| self.match_has_type(m, *elem_type))
                .collect()),
            Selector::Id(id) => {
                let id_upper = id.to_uppercase();
                Ok(matches
                    .into_iter()
                    .filter(|m| match m {
                        QueryMatch::Component { designator, .. } => {
                            designator.to_uppercase() == id_upper
                        }
                        QueryMatch::Net { name, .. } => name.to_uppercase() == id_upper,
                        QueryMatch::Port { name, .. } => name.to_uppercase() == id_upper,
                        QueryMatch::Power { net_name, .. } => net_name.to_uppercase() == id_upper,
                        _ => false,
                    })
                    .collect())
            }
            Selector::Not(inner) => {
                let excluded = self.execute_selector(inner)?;
                let excluded_set: HashSet<String> = excluded
                    .iter()
                    .map(|m: &QueryMatch| m.to_short_text())
                    .collect();
                Ok(matches
                    .into_iter()
                    .filter(|m: &QueryMatch| !excluded_set.contains(&m.to_short_text()))
                    .collect())
            }
            _ => Ok(matches),
        }
    }

    fn select_combinator(
        &self,
        left: &Selector,
        combinator: CombinatorType,
        right: &Selector,
    ) -> Result<Vec<QueryMatch>, QueryError> {
        match combinator {
            CombinatorType::Child | CombinatorType::Descendant => {
                // Get left side matches (parents)
                let parents = self.execute_selector(left)?;
                let mut results = Vec::new();

                for parent in parents {
                    // For each parent, get children that match right selector
                    let children = self.get_children(&parent, right)?;
                    results.extend(children);
                }

                Ok(results)
            }
            CombinatorType::OnNet => {
                // Get net matches from left
                let nets = self.execute_selector(left)?;
                let mut results = Vec::new();

                for net_match in nets {
                    if let QueryMatch::Net { ref name, .. } = net_match {
                        // Get all pins on this net
                        if let Some(net) = self.view.get_net(name) {
                            for conn in &net.connections {
                                if let ConnectionPoint::Pin {
                                    component_designator,
                                    pin_designator,
                                    pin_name,
                                } = conn
                                {
                                    // Check if this pin matches the right selector
                                    let pin_match = QueryMatch::Pin {
                                        component_designator: component_designator.clone(),
                                        designator: pin_designator.clone(),
                                        name: pin_name.clone(),
                                        electrical_type: "".to_string(),
                                        connected_net: Some(name.clone()),
                                        is_hidden: false,
                                    };
                                    if self.match_satisfies_selector(&pin_match, right)? {
                                        results.push(pin_match);
                                    }
                                }
                            }
                        }
                    }
                }

                Ok(results)
            }
            CombinatorType::Connected => {
                // Get left side matches and find everything connected to them
                let sources = self.execute_selector(left)?;
                let mut connected_nets: HashSet<String> = HashSet::new();

                // Find all nets connected to source elements
                for source in &sources {
                    match source {
                        QueryMatch::Component { designator, .. } => {
                            if let Some(comp) = self.view.get_component(designator) {
                                for pin in &comp.pins {
                                    if let Some(net) = &pin.connected_net {
                                        connected_nets.insert(net.clone());
                                    }
                                }
                            }
                        }
                        QueryMatch::Pin {
                            connected_net: Some(net),
                            ..
                        } => {
                            connected_nets.insert(net.clone());
                        }
                        QueryMatch::Net { name, .. } => {
                            connected_nets.insert(name.clone());
                        }
                        _ => {}
                    }
                }

                // Find all elements connected to those nets
                let mut results = Vec::new();
                for net_name in connected_nets {
                    if let Some(net) = self.view.get_net(&net_name) {
                        for conn in &net.connections {
                            if let ConnectionPoint::Pin {
                                component_designator,
                                ..
                            } = conn
                            {
                                // Get the component
                                if let Some(comp) = self.view.get_component(component_designator) {
                                    let comp_match = self.component_to_match(comp);
                                    if self.match_satisfies_selector(&comp_match, right)? {
                                        // Avoid duplicates
                                        if !results.iter().any(|m| {
                                            if let QueryMatch::Component { designator, .. } = m {
                                                designator == component_designator
                                            } else {
                                                false
                                            }
                                        }) {
                                            results.push(comp_match);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                Ok(results)
            }
            CombinatorType::Sibling => {
                // Find elements with same parent
                let left_matches = self.execute_selector(left)?;
                let mut results = Vec::new();

                for left_match in left_matches {
                    if let QueryMatch::Pin {
                        component_designator,
                        ..
                    } = &left_match
                    {
                        // Get all other pins of the same component
                        if let Some(comp) = self.view.get_component(component_designator) {
                            for pin in &comp.pins {
                                let pin_match = self.pin_to_match(comp, pin);
                                if self.match_satisfies_selector(&pin_match, right)? {
                                    results.push(pin_match);
                                }
                            }
                        }
                    }
                }

                Ok(results)
            }
            CombinatorType::Adjacent => {
                // For now, treat same as descendant
                self.select_combinator(left, CombinatorType::Descendant, right)
            }
        }
    }

    fn select_union(&self, selectors: &[Selector]) -> Result<Vec<QueryMatch>, QueryError> {
        let mut all_matches: Vec<QueryMatch> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        for sel in selectors {
            let matches: Vec<QueryMatch> = self.execute_selector(sel)?;
            for m in matches {
                let key = m.to_short_text();
                if !seen.contains(&key) {
                    seen.insert(key);
                    all_matches.push(m);
                }
            }
        }

        Ok(all_matches)
    }

    fn select_not(&self, inner: &Selector) -> Result<Vec<QueryMatch>, QueryError> {
        let all: Vec<QueryMatch> = self.select_all()?;
        let excluded: Vec<QueryMatch> = self.execute_selector(inner)?;
        let excluded_set: HashSet<String> = excluded
            .iter()
            .map(|m: &QueryMatch| m.to_short_text())
            .collect();

        Ok(all
            .into_iter()
            .filter(|m: &QueryMatch| !excluded_set.contains(&m.to_short_text()))
            .collect())
    }

    fn select_has(&self, inner: &Selector) -> Result<Vec<QueryMatch>, QueryError> {
        // Find components that have children matching inner
        let mut results = Vec::new();

        for comp in &self.view.components {
            // Check if any pin matches inner selector
            for pin in &comp.pins {
                let pin_match = self.pin_to_match(comp, pin);
                if self.match_satisfies_selector(&pin_match, inner)? {
                    results.push(self.component_to_match(comp));
                    break;
                }
            }
        }

        Ok(results)
    }

    fn get_children(
        &self,
        parent: &QueryMatch,
        child_selector: &Selector,
    ) -> Result<Vec<QueryMatch>, QueryError> {
        match parent {
            QueryMatch::Component { designator, .. } => {
                if let Some(comp) = self.view.get_component(designator) {
                    let mut children = Vec::new();

                    // Pins are children of components
                    for pin in &comp.pins {
                        let pin_match = self.pin_to_match(comp, pin);
                        if self.match_satisfies_selector(&pin_match, child_selector)? {
                            children.push(pin_match);
                        }
                    }

                    // Parameters are also children
                    for (name, value) in &comp.parameters {
                        let param_match = QueryMatch::Parameter {
                            component_designator: designator.clone(),
                            name: name.clone(),
                            value: value.clone(),
                        };
                        if self.match_satisfies_selector(&param_match, child_selector)? {
                            children.push(param_match);
                        }
                    }

                    Ok(children)
                } else {
                    Ok(vec![])
                }
            }
            _ => Ok(vec![]),
        }
    }

    fn match_satisfies_selector(
        &self,
        m: &QueryMatch,
        selector: &Selector,
    ) -> Result<bool, QueryError> {
        match selector {
            Selector::Element(elem_type) => Ok(self.match_has_type(m, *elem_type)),
            Selector::Id(id) => {
                let id_upper = id.to_uppercase();
                Ok(match m {
                    QueryMatch::Component { designator, .. } => {
                        designator.to_uppercase() == id_upper
                    }
                    QueryMatch::Pin {
                        component_designator,
                        designator,
                        ..
                    } => {
                        designator.to_uppercase() == id_upper
                            || format!("{}.{}", component_designator, designator).to_uppercase()
                                == id_upper
                    }
                    QueryMatch::Net { name, .. } => name.to_uppercase() == id_upper,
                    QueryMatch::Port { name, .. } => name.to_uppercase() == id_upper,
                    _ => false,
                })
            }
            Selector::Universal => Ok(true),
            Selector::Attribute(attr) => Ok(self.match_has_attribute(m, attr)),
            Selector::Pseudo(pseudo) => Ok(self.match_has_pseudo(m, pseudo)),
            Selector::Compound(parts) => {
                for part in parts {
                    if !self.match_satisfies_selector(m, part)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Selector::Not(inner) => Ok(!self.match_satisfies_selector(m, inner)?),
            _ => Ok(true),
        }
    }

    fn match_has_type(&self, m: &QueryMatch, elem_type: ElementType) -> bool {
        matches!(
            (m, elem_type),
            (QueryMatch::Component { .. }, ElementType::Component)
                | (QueryMatch::Pin { .. }, ElementType::Pin)
                | (QueryMatch::Net { .. }, ElementType::Net)
                | (QueryMatch::Port { .. }, ElementType::Port)
                | (QueryMatch::Wire { .. }, ElementType::Wire)
                | (
                    QueryMatch::Power {
                        is_ground: false,
                        ..
                    },
                    ElementType::Power
                )
                | (
                    QueryMatch::Power {
                        is_ground: true,
                        ..
                    },
                    ElementType::Ground
                )
                | (QueryMatch::Label { .. }, ElementType::Label)
                | (QueryMatch::Junction { .. }, ElementType::Junction)
                | (QueryMatch::Parameter { .. }, ElementType::Parameter)
        )
    }

    fn filter_by_attribute(
        &self,
        matches: &[QueryMatch],
        attr: &AttributeSelector,
    ) -> Result<Vec<QueryMatch>, QueryError> {
        Ok(matches
            .iter()
            .filter(|m| self.match_has_attribute(m, attr))
            .cloned()
            .collect())
    }

    fn match_has_attribute(&self, m: &QueryMatch, attr: &AttributeSelector) -> bool {
        let value = self.get_attribute_value(m, &attr.name);
        attr.matches(value)
    }

    fn get_attribute_value<'b>(&'b self, m: &'b QueryMatch, attr_name: &str) -> Option<&'b str> {
        let attr_lower = attr_name.to_lowercase();

        match m {
            QueryMatch::Component {
                designator,
                part,
                description,
                value,
                footprint,
                ..
            } => {
                match attr_lower.as_str() {
                    "designator" | "des" | "ref" => Some(designator.as_str()),
                    "part" | "partname" | "libref" => Some(part.as_str()),
                    "description" | "desc" => Some(description.as_str()),
                    "value" | "val" => value.as_deref(),
                    "footprint" | "fp" | "package" => footprint.as_deref(),
                    "pins" | "pincount" => None, // Numeric, handled separately
                    _ => None,
                }
            }
            QueryMatch::Pin {
                component_designator,
                designator,
                name,
                electrical_type,
                connected_net,
                is_hidden,
                ..
            } => match attr_lower.as_str() {
                "designator" | "des" | "num" | "number" | "pin" => Some(designator.as_str()),
                "name" | "pinname" => Some(name.as_str()),
                "type" | "electrical" | "elec" => Some(electrical_type.as_str()),
                "net" | "netname" => connected_net.as_deref(),
                "component" | "comp" => Some(component_designator.as_str()),
                "hidden" => {
                    if *is_hidden {
                        Some("true")
                    } else {
                        Some("false")
                    }
                }
                _ => None,
            },
            QueryMatch::Net {
                name,
                is_power,
                is_ground,
                ..
            } => match attr_lower.as_str() {
                "name" | "netname" => Some(name.as_str()),
                "power" => {
                    if *is_power {
                        Some("true")
                    } else {
                        Some("false")
                    }
                }
                "ground" | "gnd" => {
                    if *is_ground {
                        Some("true")
                    } else {
                        Some("false")
                    }
                }
                _ => None,
            },
            QueryMatch::Port {
                name,
                io_type,
                connected_net,
                ..
            } => match attr_lower.as_str() {
                "name" => Some(name.as_str()),
                "io" | "iotype" | "type" | "direction" => Some(io_type.as_str()),
                "net" | "netname" => connected_net.as_deref(),
                _ => None,
            },
            QueryMatch::Power {
                net_name,
                style,
                is_ground,
                ..
            } => match attr_lower.as_str() {
                "name" | "netname" | "net" => Some(net_name.as_str()),
                "style" => Some(style.as_str()),
                "ground" | "gnd" => {
                    if *is_ground {
                        Some("true")
                    } else {
                        Some("false")
                    }
                }
                _ => None,
            },
            QueryMatch::Parameter {
                component_designator,
                name,
                value,
            } => match attr_lower.as_str() {
                "name" => Some(name.as_str()),
                "value" | "val" => Some(value.as_str()),
                "component" | "comp" => Some(component_designator.as_str()),
                _ => None,
            },
            QueryMatch::Label { text, .. } => match attr_lower.as_str() {
                "text" | "name" | "net" => Some(text.as_str()),
                _ => None,
            },
            _ => None,
        }
    }

    fn filter_by_pseudo(
        &self,
        matches: &[QueryMatch],
        pseudo: &PseudoSelector,
    ) -> Result<Vec<QueryMatch>, QueryError> {
        // Result modifiers are handled separately
        if pseudo.is_result_modifier() {
            return Ok(matches.to_vec());
        }

        Ok(matches
            .iter()
            .filter(|m| self.match_has_pseudo(m, pseudo))
            .cloned()
            .collect())
    }

    fn match_has_pseudo(&self, m: &QueryMatch, pseudo: &PseudoSelector) -> bool {
        match pseudo {
            PseudoSelector::Connected => match m {
                QueryMatch::Pin { connected_net, .. } => connected_net.is_some(),
                QueryMatch::Net {
                    connection_count, ..
                } => *connection_count > 0,
                QueryMatch::Port { connected_net, .. } => connected_net.is_some(),
                _ => true,
            },
            PseudoSelector::Unconnected => match m {
                QueryMatch::Pin { connected_net, .. } => connected_net.is_none(),
                QueryMatch::Net {
                    connection_count, ..
                } => *connection_count == 0,
                _ => false,
            },
            PseudoSelector::Power => match m {
                QueryMatch::Net { is_power, .. } => *is_power,
                QueryMatch::Pin {
                    electrical_type, ..
                } => electrical_type == "Power",
                QueryMatch::Power { is_ground, .. } => !*is_ground,
                _ => false,
            },
            PseudoSelector::Ground => match m {
                QueryMatch::Net { is_ground, .. } => *is_ground,
                QueryMatch::Power { is_ground, .. } => *is_ground,
                _ => false,
            },
            PseudoSelector::Input => match m {
                QueryMatch::Pin {
                    electrical_type, ..
                } => electrical_type == "Input",
                QueryMatch::Port { io_type, .. } => io_type.to_uppercase().contains("INPUT"),
                _ => false,
            },
            PseudoSelector::Output => match m {
                QueryMatch::Pin {
                    electrical_type, ..
                } => electrical_type == "Output",
                QueryMatch::Port { io_type, .. } => io_type.to_uppercase().contains("OUTPUT"),
                _ => false,
            },
            PseudoSelector::Bidirectional => match m {
                QueryMatch::Pin {
                    electrical_type, ..
                } => electrical_type == "Bidirectional",
                QueryMatch::Port { io_type, .. } => io_type.to_uppercase().contains("BIDIR"),
                _ => false,
            },
            PseudoSelector::Passive => match m {
                QueryMatch::Pin {
                    electrical_type, ..
                } => electrical_type == "Passive",
                _ => false,
            },
            PseudoSelector::OpenCollector => match m {
                QueryMatch::Pin {
                    electrical_type, ..
                } => electrical_type == "OpenCollector",
                _ => false,
            },
            PseudoSelector::OpenEmitter => match m {
                QueryMatch::Pin {
                    electrical_type, ..
                } => electrical_type == "OpenEmitter",
                _ => false,
            },
            PseudoSelector::HiZ => match m {
                QueryMatch::Pin {
                    electrical_type, ..
                } => electrical_type == "HiZ",
                _ => false,
            },
            PseudoSelector::Hidden => match m {
                QueryMatch::Pin { is_hidden, .. } => *is_hidden,
                _ => false,
            },
            PseudoSelector::Visible => match m {
                QueryMatch::Pin { is_hidden, .. } => !*is_hidden,
                _ => true,
            },
            // Result modifiers handled in apply_result_modifiers
            _ => true,
        }
    }

    fn apply_result_modifiers(
        &self,
        mut matches: Vec<QueryMatch>,
        selector: &Selector,
    ) -> Vec<QueryMatch> {
        let modifiers = selector.get_result_modifiers();

        for modifier in modifiers {
            match modifier {
                PseudoSelector::Count => {
                    return vec![QueryMatch::Count(matches.len())];
                }
                PseudoSelector::First => {
                    return matches.into_iter().take(1).collect();
                }
                PseudoSelector::Last => {
                    return matches.into_iter().last().into_iter().collect();
                }
                PseudoSelector::Nth(n) => {
                    if *n > 0 && *n <= matches.len() {
                        return vec![matches.remove(*n - 1)];
                    }
                    return vec![];
                }
                PseudoSelector::NthLast(n) => {
                    if *n > 0 && *n <= matches.len() {
                        let idx = matches.len() - *n;
                        return vec![matches.remove(idx)];
                    }
                    return vec![];
                }
                PseudoSelector::Even => {
                    return matches
                        .into_iter()
                        .enumerate()
                        .filter(|(i, _)| i % 2 == 1)
                        .map(|(_, m)| m)
                        .collect();
                }
                PseudoSelector::Odd => {
                    return matches
                        .into_iter()
                        .enumerate()
                        .filter(|(i, _)| i % 2 == 0)
                        .map(|(_, m)| m)
                        .collect();
                }
                PseudoSelector::Limit(n) => {
                    matches = matches.into_iter().take(*n).collect();
                }
                PseudoSelector::Offset(n) => {
                    matches = matches.into_iter().skip(*n).collect();
                }
                _ => {}
            }
        }

        matches
    }

    // Helper methods to create QueryMatch from views

    fn component_to_match(&self, comp: &ComponentView) -> QueryMatch {
        QueryMatch::Component {
            designator: comp.designator.clone(),
            part: comp.part_name.clone(),
            description: comp.description.clone(),
            value: comp.value.clone(),
            footprint: comp.footprint.clone(),
            pin_count: comp.pins.len(),
        }
    }

    fn pin_to_match(&self, comp: &ComponentView, pin: &PinView) -> QueryMatch {
        QueryMatch::Pin {
            component_designator: comp.designator.clone(),
            designator: pin.designator.clone(),
            name: pin.name.clone(),
            electrical_type: pin.electrical_type.as_str().to_string(),
            connected_net: pin.connected_net.clone(),
            is_hidden: pin.is_hidden,
        }
    }

    fn all_pins(&self) -> Vec<QueryMatch> {
        let mut pins = Vec::new();
        for comp in &self.view.components {
            for pin in &comp.pins {
                pins.push(self.pin_to_match(comp, pin));
            }
        }
        pins
    }

    fn net_to_match(&self, net: &NetView) -> QueryMatch {
        QueryMatch::Net {
            name: net.name.clone(),
            is_power: net.is_power,
            is_ground: net.is_ground,
            connection_count: net.connections.len(),
            connections: net
                .connections
                .iter()
                .map(|c| c.to_short_string())
                .collect(),
        }
    }

    fn port_to_match(&self, port: &PortView) -> QueryMatch {
        QueryMatch::Port {
            name: port.name.clone(),
            io_type: port.io_type.clone(),
            connected_net: port.connected_net.clone(),
        }
    }

    fn wire_to_match(&self, index: usize, wire: &WireView) -> QueryMatch {
        let start = wire.vertices.first().copied().unwrap_or((0, 0));
        let end = wire.vertices.last().copied().unwrap_or((0, 0));
        QueryMatch::Wire {
            index,
            vertex_count: wire.vertices.len(),
            start,
            end,
        }
    }

    fn power_to_match(&self, power: &PowerView) -> QueryMatch {
        QueryMatch::Power {
            net_name: power.net_name.clone(),
            style: power.style.clone(),
            is_ground: power.is_ground,
        }
    }

    fn label_to_match(&self, label: &LabelView) -> QueryMatch {
        QueryMatch::Label {
            text: label.text.clone(),
            location: label.location,
        }
    }

    fn junction_to_match(&self, junction: &JunctionView) -> QueryMatch {
        QueryMatch::Junction {
            location: junction.location,
        }
    }

    fn all_parameters(&self) -> Vec<QueryMatch> {
        let mut params = Vec::new();
        for comp in &self.view.components {
            for (name, value) in &comp.parameters {
                params.push(QueryMatch::Parameter {
                    component_designator: comp.designator.clone(),
                    name: name.clone(),
                    value: value.clone(),
                });
            }
        }
        params
    }
}

#[cfg(test)]
mod tests {
    // These tests would require a mock SchematicView
    // For now, just verify the executor compiles
    #[test]
    fn test_executor_creation() {
        // Would need a test SchDoc here
    }
}

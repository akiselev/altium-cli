//! High-level schematic view with connectivity analysis.
//!
//! This module provides a semantic representation of schematic documents,
//! computing net connectivity from raw primitives.

use crate::io::schdoc::SchDoc;
use crate::records::sch::*;
use crate::tree::RecordTree;
use std::collections::{HashMap, HashSet};

// Re-export ElectricalType from common module
pub use super::common::ElectricalType;

/// A connection point in the schematic
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionPoint {
    /// Component pin
    Pin {
        component_designator: String,
        pin_designator: String,
        pin_name: String,
    },
    /// Inter-sheet port
    Port { name: String, io_type: String },
    /// Power/ground symbol
    PowerRail { net_name: String, is_ground: bool },
    /// Net label
    NetLabel { name: String },
}

impl ConnectionPoint {
    /// Get a short string representation
    pub fn to_short_string(&self) -> String {
        match self {
            ConnectionPoint::Pin {
                component_designator,
                pin_designator,
                pin_name,
            } => {
                if pin_name.is_empty() {
                    format!("{}.{}", component_designator, pin_designator)
                } else {
                    format!("{}.{} ({})", component_designator, pin_designator, pin_name)
                }
            }
            ConnectionPoint::Port { name, .. } => format!("PORT:{}", name),
            ConnectionPoint::PowerRail {
                net_name,
                is_ground,
            } => {
                if *is_ground {
                    format!("GND:{}", net_name)
                } else {
                    format!("PWR:{}", net_name)
                }
            }
            ConnectionPoint::NetLabel { name } => format!("LABEL:{}", name),
        }
    }
}

/// High-level view of a component
#[derive(Debug, Clone)]
pub struct ComponentView {
    /// Reference designator (e.g., "U1", "R1")
    pub designator: String,
    /// Library reference / part name
    pub part_name: String,
    /// Component description
    pub description: String,
    /// Value parameter if present
    pub value: Option<String>,
    /// Footprint name
    pub footprint: Option<String>,
    /// All pins on this component
    pub pins: Vec<PinView>,
    /// Additional parameters
    pub parameters: HashMap<String, String>,
    /// Record index in primitives list
    pub record_index: usize,
}

/// High-level view of a pin
#[derive(Debug, Clone)]
pub struct PinView {
    /// Pin designator/number
    pub designator: String,
    /// Pin name
    pub name: String,
    /// Electrical type
    pub electrical_type: ElectricalType,
    /// Net this pin connects to
    pub connected_net: Option<String>,
    /// Whether pin is hidden
    pub is_hidden: bool,
    /// Hidden net name (for power pins with implicit connection)
    pub hidden_net: Option<String>,
    /// Parent component designator
    pub component_designator: String,
    /// Location for connectivity analysis
    pub location: (i32, i32),
    /// Pin end location (corner)
    pub corner: (i32, i32),
}

/// High-level view of a net
#[derive(Debug, Clone)]
pub struct NetView {
    /// Net name
    pub name: String,
    /// Whether this is a power net
    pub is_power: bool,
    /// Whether this is a ground net
    pub is_ground: bool,
    /// All connection points on this net
    pub connections: Vec<ConnectionPoint>,
}

/// High-level view of a port
#[derive(Debug, Clone)]
pub struct PortView {
    /// Port name
    pub name: String,
    /// I/O type
    pub io_type: String,
    /// Harness type
    pub harness: Option<String>,
    /// Connected net
    pub connected_net: Option<String>,
    /// Location
    pub location: (i32, i32),
    /// Record index
    pub record_index: usize,
}

/// High-level view of a power symbol
#[derive(Debug, Clone)]
pub struct PowerView {
    /// Net name
    pub net_name: String,
    /// Power style
    pub style: String,
    /// Whether this is a ground symbol
    pub is_ground: bool,
    /// Location
    pub location: (i32, i32),
    /// Record index
    pub record_index: usize,
}

/// Wire segment information
#[derive(Debug, Clone)]
pub struct WireView {
    /// Vertices of the wire
    pub vertices: Vec<(i32, i32)>,
    /// Record index
    pub record_index: usize,
}

/// Net label information
#[derive(Debug, Clone)]
pub struct LabelView {
    /// Label text (net name)
    pub text: String,
    /// Location
    pub location: (i32, i32),
    /// Record index
    pub record_index: usize,
}

/// Junction information
#[derive(Debug, Clone)]
pub struct JunctionView {
    /// Location
    pub location: (i32, i32),
    /// Record index
    pub record_index: usize,
}

/// High-level schematic document view with connectivity
#[derive(Debug)]
pub struct SchematicView {
    /// Sheet name/title
    pub sheet_name: Option<String>,
    /// All components
    pub components: Vec<ComponentView>,
    /// All nets (computed from connectivity)
    pub nets: Vec<NetView>,
    /// All ports
    pub ports: Vec<PortView>,
    /// All power symbols
    pub power_symbols: Vec<PowerView>,
    /// All wires
    pub wires: Vec<WireView>,
    /// All net labels
    pub labels: Vec<LabelView>,
    /// All junctions
    pub junctions: Vec<JunctionView>,
    /// Component index by designator
    component_index: HashMap<String, usize>,
    /// Net index by name
    net_index: HashMap<String, usize>,
}

impl SchematicView {
    /// Build a schematic view from a SchDoc
    pub fn from_schdoc(doc: &SchDoc) -> Self {
        let mut builder = SchematicViewBuilder::new(doc);
        builder.build()
    }

    /// Get component by designator
    pub fn get_component(&self, designator: &str) -> Option<&ComponentView> {
        self.component_index
            .get(designator)
            .map(|&i| &self.components[i])
    }

    /// Get net by name
    pub fn get_net(&self, name: &str) -> Option<&NetView> {
        self.net_index.get(name).map(|&i| &self.nets[i])
    }

    /// Get all power net names
    pub fn power_nets(&self) -> Vec<&str> {
        self.nets
            .iter()
            .filter(|n| n.is_power && !n.is_ground)
            .map(|n| n.name.as_str())
            .collect()
    }

    /// Get all ground net names
    pub fn ground_nets(&self) -> Vec<&str> {
        self.nets
            .iter()
            .filter(|n| n.is_ground)
            .map(|n| n.name.as_str())
            .collect()
    }

    /// Find components by part name pattern
    pub fn find_components_by_part(&self, pattern: &str) -> Vec<&ComponentView> {
        let pattern_lower = pattern.to_lowercase();
        self.components
            .iter()
            .filter(|c| c.part_name.to_lowercase().contains(&pattern_lower))
            .collect()
    }

    /// Get all pins for a component
    pub fn get_pins(&self, designator: &str) -> Vec<&PinView> {
        self.get_component(designator)
            .map(|c| c.pins.iter().collect())
            .unwrap_or_default()
    }
}

/// Builder for SchematicView that handles connectivity analysis
struct SchematicViewBuilder<'a> {
    doc: &'a SchDoc,
    #[allow(dead_code)] // Reserved for future hierarchical record queries
    tree: RecordTree<SchRecord>,
    /// Coordinate to connection points map
    coord_map: HashMap<(i32, i32), Vec<CoordEntry>>,
    /// Union-find for net connectivity
    net_union: HashMap<(i32, i32), (i32, i32)>,
    /// Net names at coordinates
    net_names: HashMap<(i32, i32), String>,
    /// Power/ground flags at coordinates
    power_coords: HashSet<(i32, i32)>,
    ground_coords: HashSet<(i32, i32)>,
}

#[derive(Debug, Clone)]
enum CoordEntry {
    PinEnd {
        component_designator: String,
        pin_designator: String,
        pin_name: String,
        #[allow(dead_code)] // Reserved for future ERC checks based on pin electrical types
        electrical_type: ElectricalType,
    },
    WireVertex {
        #[allow(dead_code)] // Reserved for future wire segment highlighting
        wire_index: usize,
    },
    Junction,
    NetLabel {
        name: String,
    },
    Port {
        name: String,
        io_type: String,
    },
    Power {
        name: String,
        is_ground: bool,
    },
}

impl<'a> SchematicViewBuilder<'a> {
    fn new(doc: &'a SchDoc) -> Self {
        let tree = RecordTree::from_records(doc.primitives.clone());
        Self {
            doc,
            tree,
            coord_map: HashMap::new(),
            net_union: HashMap::new(),
            net_names: HashMap::new(),
            power_coords: HashSet::new(),
            ground_coords: HashSet::new(),
        }
    }

    fn build(&mut self) -> SchematicView {
        // Extract components and pins
        let (components, component_index) = self.extract_components();

        // Extract other elements
        let ports = self.extract_ports();
        let power_symbols = self.extract_power_symbols();
        let wires = self.extract_wires();
        let labels = self.extract_labels();
        let junctions = self.extract_junctions();

        // Build coordinate map for connectivity
        self.build_coord_map(
            &components,
            &ports,
            &power_symbols,
            &wires,
            &labels,
            &junctions,
        );

        // Trace connectivity through wires
        self.trace_connectivity(&wires);

        // Build nets from traced connectivity
        let nets = self.build_nets(&components);
        let net_index: HashMap<String, usize> = nets
            .iter()
            .enumerate()
            .map(|(i, n)| (n.name.clone(), i))
            .collect();

        // Update pins with their connected net names
        let mut components = components;
        self.update_pin_connections(&mut components, &nets);

        SchematicView {
            sheet_name: self.extract_sheet_name(),
            components,
            nets,
            ports,
            power_symbols,
            wires,
            labels,
            junctions,
            component_index,
            net_index,
        }
    }

    fn extract_components(&self) -> (Vec<ComponentView>, HashMap<String, usize>) {
        let mut components = Vec::new();
        let mut component_index = HashMap::new();

        for (idx, record) in self.doc.primitives.iter().enumerate() {
            if let SchRecord::Component(comp) = record {
                // Find designator and parameters
                let mut designator = String::new();
                let mut value = None;
                let mut footprint = None;
                let mut parameters = HashMap::new();
                let mut pins = Vec::new();

                // Scan children for parameters and pins
                for child in self.doc.primitives.iter() {
                    let owner = child.owner_index();
                    if owner == idx as i32 {
                        match child {
                            SchRecord::Designator(des) => {
                                designator = des.param.label.text.clone();
                            }
                            SchRecord::Parameter(param) => {
                                let name = param.name.to_uppercase();
                                let val = param.label.text.clone();
                                if name == "VALUE" {
                                    value = Some(val.clone());
                                }
                                parameters.insert(param.name.clone(), val);
                            }
                            SchRecord::Pin(pin) => {
                                let corner = pin.get_corner();
                                pins.push(PinView {
                                    designator: pin.designator.clone(),
                                    name: pin.name.clone(),
                                    electrical_type: ElectricalType::from_pin_electrical(
                                        pin.electrical,
                                    ),
                                    connected_net: None,
                                    is_hidden: pin.is_hidden(),
                                    hidden_net: if pin.hidden_net_name.is_empty() {
                                        None
                                    } else {
                                        Some(pin.hidden_net_name.clone())
                                    },
                                    component_designator: String::new(), // Set later
                                    location: (pin.graphical.location_x, pin.graphical.location_y),
                                    corner,
                                });
                            }
                            SchRecord::Implementation(impl_rec) => {
                                if impl_rec.model_type.to_uppercase() == "PCBLIB"
                                    && impl_rec.is_current
                                {
                                    footprint = Some(impl_rec.model_name.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }

                // Use lib_reference if designator not found
                if designator.is_empty() {
                    designator = format!("?{}", components.len());
                }

                // Update pin component designators
                for pin in &mut pins {
                    pin.component_designator = designator.clone();
                }

                let comp_view = ComponentView {
                    designator: designator.clone(),
                    part_name: comp.lib_reference.clone(),
                    description: comp.component_description.clone(),
                    value,
                    footprint,
                    pins,
                    parameters,
                    record_index: idx,
                };

                component_index.insert(designator.clone(), components.len());
                components.push(comp_view);
            }
        }

        (components, component_index)
    }

    fn extract_ports(&self) -> Vec<PortView> {
        self.doc
            .primitives
            .iter()
            .enumerate()
            .filter_map(|(idx, record)| {
                if let SchRecord::Port(port) = record {
                    Some(PortView {
                        name: port.name.clone(),
                        io_type: format!("{:?}", port.io_type),
                        harness: if port.harness_type.is_empty() {
                            None
                        } else {
                            Some(port.harness_type.clone())
                        },
                        connected_net: None,
                        location: (port.graphical.location_x, port.graphical.location_y),
                        record_index: idx,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn extract_power_symbols(&self) -> Vec<PowerView> {
        self.doc
            .primitives
            .iter()
            .enumerate()
            .filter_map(|(idx, record)| {
                if let SchRecord::PowerObject(pwr) = record {
                    let is_ground = matches!(
                        pwr.style,
                        PowerObjectStyle::Ground
                            | PowerObjectStyle::SignalGround
                            | PowerObjectStyle::EarthGround
                            | PowerObjectStyle::PowerGround
                    );
                    Some(PowerView {
                        net_name: pwr.text.clone(),
                        style: format!("{:?}", pwr.style),
                        is_ground,
                        location: (pwr.graphical.location_x, pwr.graphical.location_y),
                        record_index: idx,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn extract_wires(&self) -> Vec<WireView> {
        self.doc
            .primitives
            .iter()
            .enumerate()
            .filter_map(|(idx, record)| {
                if let SchRecord::Wire(wire) = record {
                    Some(WireView {
                        vertices: wire.vertices.clone(),
                        record_index: idx,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn extract_labels(&self) -> Vec<LabelView> {
        self.doc
            .primitives
            .iter()
            .enumerate()
            .filter_map(|(idx, record)| {
                if let SchRecord::NetLabel(label) = record {
                    Some(LabelView {
                        text: label.label.text.clone(),
                        location: (
                            label.label.graphical.location_x,
                            label.label.graphical.location_y,
                        ),
                        record_index: idx,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn extract_junctions(&self) -> Vec<JunctionView> {
        self.doc
            .primitives
            .iter()
            .enumerate()
            .filter_map(|(idx, record)| {
                if let SchRecord::Junction(junc) = record {
                    Some(JunctionView {
                        location: (junc.graphical.location_x, junc.graphical.location_y),
                        record_index: idx,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn extract_sheet_name(&self) -> Option<String> {
        // Try to find sheet header or use filename
        for record in &self.doc.primitives {
            if let SchRecord::SheetHeader(_) = record {
                // Sheet header doesn't have name, would need to get from file
                return None;
            }
        }
        None
    }

    fn build_coord_map(
        &mut self,
        components: &[ComponentView],
        ports: &[PortView],
        power_symbols: &[PowerView],
        wires: &[WireView],
        labels: &[LabelView],
        junctions: &[JunctionView],
    ) {
        // Add pin connection points (at pin corner, not base)
        for comp in components {
            for pin in &comp.pins {
                let entry = CoordEntry::PinEnd {
                    component_designator: comp.designator.clone(),
                    pin_designator: pin.designator.clone(),
                    pin_name: pin.name.clone(),
                    electrical_type: pin.electrical_type,
                };
                self.coord_map.entry(pin.corner).or_default().push(entry);
            }
        }

        // Add wire vertices
        for (wire_idx, wire) in wires.iter().enumerate() {
            for &vertex in &wire.vertices {
                self.coord_map
                    .entry(vertex)
                    .or_default()
                    .push(CoordEntry::WireVertex {
                        wire_index: wire_idx,
                    });
            }
        }

        // Add junctions
        for junc in junctions {
            self.coord_map
                .entry(junc.location)
                .or_default()
                .push(CoordEntry::Junction);
        }

        // Add net labels
        for label in labels {
            self.coord_map
                .entry(label.location)
                .or_default()
                .push(CoordEntry::NetLabel {
                    name: label.text.clone(),
                });
            self.net_names.insert(label.location, label.text.clone());
        }

        // Add ports
        for port in ports {
            self.coord_map
                .entry(port.location)
                .or_default()
                .push(CoordEntry::Port {
                    name: port.name.clone(),
                    io_type: port.io_type.clone(),
                });
        }

        // Add power symbols
        for pwr in power_symbols {
            self.coord_map
                .entry(pwr.location)
                .or_default()
                .push(CoordEntry::Power {
                    name: pwr.net_name.clone(),
                    is_ground: pwr.is_ground,
                });
            self.net_names.insert(pwr.location, pwr.net_name.clone());
            if pwr.is_ground {
                self.ground_coords.insert(pwr.location);
            } else {
                self.power_coords.insert(pwr.location);
            }
        }
    }

    fn trace_connectivity(&mut self, wires: &[WireView]) {
        // Initialize union-find: each coordinate is its own parent
        for &coord in self.coord_map.keys() {
            self.net_union.insert(coord, coord);
        }

        // Union wire vertices (each wire connects all its vertices)
        for wire in wires {
            if wire.vertices.len() >= 2 {
                let first = wire.vertices[0];
                for &vertex in &wire.vertices[1..] {
                    self.union(first, vertex);
                }
            }
        }

        // Union coordinates that share the same location
        // (already done implicitly through coord_map)
    }

    fn find(&mut self, coord: (i32, i32)) -> (i32, i32) {
        if let std::collections::hash_map::Entry::Vacant(entry) = self.net_union.entry(coord) {
            entry.insert(coord);
            return coord;
        }

        let parent = self.net_union[&coord];
        if parent == coord {
            return coord;
        }

        let root = self.find(parent);
        self.net_union.insert(coord, root);
        root
    }

    fn union(&mut self, a: (i32, i32), b: (i32, i32)) {
        let root_a = self.find(a);
        let root_b = self.find(b);
        if root_a != root_b {
            self.net_union.insert(root_b, root_a);
        }
    }

    fn build_nets(&mut self, _components: &[ComponentView]) -> Vec<NetView> {
        // Collect coordinates first to avoid borrow issues
        let coords: Vec<(i32, i32)> = self.coord_map.keys().copied().collect();

        // Group coordinates by their root (same net)
        let mut net_groups: HashMap<(i32, i32), Vec<(i32, i32)>> = HashMap::new();
        for coord in coords {
            let root = self.find(coord);
            net_groups.entry(root).or_default().push(coord);
        }

        // Build net views
        let mut nets = Vec::new();
        let mut auto_net_id = 0;

        for (_root, coords) in net_groups {
            // Determine net name from labels or power symbols
            let mut net_name = None;
            let mut is_power = false;
            let mut is_ground = false;

            for &coord in &coords {
                if let Some(name) = self.net_names.get(&coord) {
                    net_name = Some(name.clone());
                }
                if self.power_coords.contains(&coord) {
                    is_power = true;
                }
                if self.ground_coords.contains(&coord) {
                    is_ground = true;
                }
            }

            // Generate auto name if no label
            let name = net_name.unwrap_or_else(|| {
                auto_net_id += 1;
                format!("Net{}", auto_net_id)
            });

            // Collect connection points
            let mut connections = Vec::new();
            for &coord in &coords {
                if let Some(entries) = self.coord_map.get(&coord) {
                    for entry in entries {
                        match entry {
                            CoordEntry::PinEnd {
                                component_designator,
                                pin_designator,
                                pin_name,
                                ..
                            } => {
                                connections.push(ConnectionPoint::Pin {
                                    component_designator: component_designator.clone(),
                                    pin_designator: pin_designator.clone(),
                                    pin_name: pin_name.clone(),
                                });
                            }
                            CoordEntry::Port { name, io_type } => {
                                connections.push(ConnectionPoint::Port {
                                    name: name.clone(),
                                    io_type: io_type.clone(),
                                });
                            }
                            CoordEntry::Power { name, is_ground } => {
                                connections.push(ConnectionPoint::PowerRail {
                                    net_name: name.clone(),
                                    is_ground: *is_ground,
                                });
                            }
                            CoordEntry::NetLabel { name } => {
                                connections.push(ConnectionPoint::NetLabel { name: name.clone() });
                            }
                            _ => {} // Skip wire vertices and junctions
                        }
                    }
                }
            }

            // Only add nets that have actual connections (pins, ports, etc.)
            let has_connections = connections
                .iter()
                .any(|c| matches!(c, ConnectionPoint::Pin { .. }));
            if has_connections || !connections.is_empty() {
                nets.push(NetView {
                    name,
                    is_power,
                    is_ground,
                    connections,
                });
            }
        }

        nets
    }

    fn update_pin_connections(&self, components: &mut [ComponentView], nets: &[NetView]) {
        // Build a map from (component, pin) to net name
        let mut pin_to_net: HashMap<(String, String), String> = HashMap::new();
        for net in nets {
            for conn in &net.connections {
                if let ConnectionPoint::Pin {
                    component_designator,
                    pin_designator,
                    ..
                } = conn
                {
                    pin_to_net.insert(
                        (component_designator.clone(), pin_designator.clone()),
                        net.name.clone(),
                    );
                }
            }
        }

        // Update pins
        for comp in components {
            for pin in &mut comp.pins {
                let key = (comp.designator.clone(), pin.designator.clone());
                pin.connected_net = pin_to_net.get(&key).cloned();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_electrical_type_display() {
        assert_eq!(ElectricalType::Input.as_str(), "Input");
        assert_eq!(ElectricalType::Power.as_str(), "Power");
    }

    #[test]
    fn test_connection_point_short_string() {
        let pin = ConnectionPoint::Pin {
            component_designator: "U1".to_string(),
            pin_designator: "1".to_string(),
            pin_name: "VCC".to_string(),
        };
        assert_eq!(pin.to_short_string(), "U1.1 (VCC)");

        let power = ConnectionPoint::PowerRail {
            net_name: "VCC".to_string(),
            is_ground: false,
        };
        assert_eq!(power.to_short_string(), "PWR:VCC");
    }
}

//! Netlist builder for analyzing and managing net connectivity.

use std::collections::{HashMap, HashSet};

use crate::records::sch::{SchPin, SchRecord};
use crate::types::Coord;

/// A connection point on the schematic.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConnectionPoint {
    /// Location of the connection.
    pub location: (i32, i32),
    /// Type of connection.
    pub kind: ConnectionKind,
}

/// Types of connection points.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConnectionKind {
    /// Pin connection with component designator and pin designator.
    Pin { component: String, pin: String },
    /// Wire endpoint.
    WireEnd { wire_index: usize },
    /// Junction.
    Junction { index: usize },
    /// Net label.
    NetLabel { name: String },
    /// Power port.
    PowerPort { name: String },
    /// Sheet port.
    Port { name: String },
}

/// A net connecting multiple points.
#[derive(Debug, Clone)]
pub struct Net {
    /// Net name (may be auto-generated).
    pub name: String,
    /// Whether the name was explicitly assigned (via net label).
    pub named: bool,
    /// All connection points in this net.
    pub connections: Vec<ConnectionPoint>,
}

impl Net {
    /// Get all pin connections in this net.
    pub fn pins(&self) -> Vec<(&str, &str)> {
        self.connections
            .iter()
            .filter_map(|c| match &c.kind {
                ConnectionKind::Pin { component, pin } => Some((component.as_str(), pin.as_str())),
                _ => None,
            })
            .collect()
    }

    /// Check if this net connects to a specific component.
    pub fn connects_to(&self, component: &str) -> bool {
        self.connections.iter().any(|c| match &c.kind {
            ConnectionKind::Pin { component: c, .. } => c == component,
            _ => false,
        })
    }
}

/// Netlist containing all nets in a schematic.
#[derive(Debug, Clone, Default)]
pub struct Netlist {
    /// All nets.
    pub nets: Vec<Net>,
    /// Map from location to net index.
    location_to_net: HashMap<(i32, i32), usize>,
}

impl Netlist {
    /// Get a net by name.
    pub fn get_by_name(&self, name: &str) -> Option<&Net> {
        self.nets.iter().find(|n| n.name == name)
    }

    /// Get all nets connected to a component.
    pub fn nets_for_component(&self, designator: &str) -> Vec<&Net> {
        self.nets
            .iter()
            .filter(|n| n.connects_to(designator))
            .collect()
    }

    /// Get the net at a specific location.
    pub fn net_at(&self, x: i32, y: i32) -> Option<&Net> {
        self.location_to_net.get(&(x, y)).map(|&i| &self.nets[i])
    }
}

/// Builder for constructing netlists from schematic primitives.
pub struct NetlistBuilder {
    /// Tolerance for matching connection points (in internal units).
    tolerance: i32,
}

impl Default for NetlistBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NetlistBuilder {
    /// Create a new netlist builder.
    pub fn new() -> Self {
        Self {
            tolerance: Coord::from_mils(1.0).to_raw(), // 1 mil tolerance
        }
    }

    /// Set the tolerance for matching connection points.
    pub fn with_tolerance(mut self, mils: f64) -> Self {
        self.tolerance = Coord::from_mils(mils).to_raw();
        self
    }

    /// Build a netlist from schematic primitives.
    pub fn build(&self, primitives: &[SchRecord]) -> Netlist {
        // Step 1: Collect all connection points
        let mut points: Vec<ConnectionPoint> = Vec::new();
        let mut designators: HashMap<usize, String> = HashMap::new();

        // First pass: collect designators
        for record in primitives.iter() {
            if let SchRecord::Designator(d) = record {
                let owner = d.param.label.graphical.base.owner_index as usize;
                designators.insert(owner, d.text().to_string());
            }
        }

        // Second pass: collect connection points
        for (i, record) in primitives.iter().enumerate() {
            match record {
                SchRecord::Pin(pin) => {
                    let owner = pin.graphical.base.owner_index as usize;
                    let component = designators.get(&owner).cloned().unwrap_or_default();
                    let endpoint = self.get_pin_endpoint(pin);

                    points.push(ConnectionPoint {
                        location: endpoint,
                        kind: ConnectionKind::Pin {
                            component,
                            pin: pin.designator.clone(),
                        },
                    });
                }
                SchRecord::Wire(wire) => {
                    // Add both endpoints
                    for (j, vertex) in wire.vertices.iter().enumerate() {
                        if j == 0 || j == wire.vertices.len() - 1 {
                            points.push(ConnectionPoint {
                                location: *vertex,
                                kind: ConnectionKind::WireEnd { wire_index: i },
                            });
                        }
                    }
                }
                SchRecord::Junction(junction) => {
                    points.push(ConnectionPoint {
                        location: (junction.graphical.location_x, junction.graphical.location_y),
                        kind: ConnectionKind::Junction { index: i },
                    });
                }
                SchRecord::NetLabel(label) => {
                    points.push(ConnectionPoint {
                        location: (
                            label.label.graphical.location_x,
                            label.label.graphical.location_y,
                        ),
                        kind: ConnectionKind::NetLabel {
                            name: label.label.text.clone(),
                        },
                    });
                }
                SchRecord::PowerObject(power) => {
                    points.push(ConnectionPoint {
                        location: (power.graphical.location_x, power.graphical.location_y),
                        kind: ConnectionKind::PowerPort {
                            name: power.text.clone(),
                        },
                    });
                }
                SchRecord::Port(port) => {
                    points.push(ConnectionPoint {
                        location: (port.graphical.location_x, port.graphical.location_y),
                        kind: ConnectionKind::Port {
                            name: port.name.clone(),
                        },
                    });
                }
                _ => {}
            }
        }

        // Step 2: Collect wire segments for connectivity
        let mut wire_segments: Vec<((i32, i32), (i32, i32))> = Vec::new();
        for record in primitives {
            if let SchRecord::Wire(wire) = record {
                for i in 0..wire.vertices.len().saturating_sub(1) {
                    wire_segments.push((wire.vertices[i], wire.vertices[i + 1]));
                }
            }
        }

        // Step 3: Build connectivity graph using union-find
        let mut union_find = UnionFind::new(points.len());

        // Connect points at the same location
        let mut location_map: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (i, point) in points.iter().enumerate() {
            // Snap to grid for matching
            let snapped = self.snap_location(point.location);
            location_map.entry(snapped).or_default().push(i);
        }

        for indices in location_map.values() {
            for i in 1..indices.len() {
                union_find.union(indices[0], indices[i]);
            }
        }

        // Connect points along wire segments
        for (start, end) in &wire_segments {
            let snapped_start = self.snap_location(*start);
            let snapped_end = self.snap_location(*end);

            if let (Some(start_indices), Some(end_indices)) = (
                location_map.get(&snapped_start),
                location_map.get(&snapped_end),
            ) {
                if !start_indices.is_empty() && !end_indices.is_empty() {
                    union_find.union(start_indices[0], end_indices[0]);
                }
            }

            // Also check for points along the wire segment
            for (loc, indices) in &location_map {
                if self.point_on_segment(*loc, *start, *end) && !indices.is_empty() {
                    if let Some(start_idx) =
                        location_map.get(&snapped_start).and_then(|v| v.first())
                    {
                        union_find.union(*start_idx, indices[0]);
                    }
                }
            }
        }

        // Step 4: Group points into nets
        let mut net_groups: HashMap<usize, Vec<usize>> = HashMap::new();
        for i in 0..points.len() {
            let root = union_find.find(i);
            net_groups.entry(root).or_default().push(i);
        }

        // Step 5: Create nets
        let mut nets = Vec::new();
        let mut net_counter = 1;

        for (_, group) in net_groups {
            let connections: Vec<ConnectionPoint> =
                group.iter().map(|&i| points[i].clone()).collect();

            // Find net name
            let (name, named) = self.find_net_name(&connections, &mut net_counter);

            nets.push(Net {
                name,
                named,
                connections,
            });
        }

        // Sort nets by name for consistent output
        nets.sort_by(|a, b| a.name.cmp(&b.name));

        // Build location-to-net map
        let mut location_to_net = HashMap::new();
        for (i, net) in nets.iter().enumerate() {
            for conn in &net.connections {
                location_to_net.insert(conn.location, i);
            }
        }

        Netlist {
            nets,
            location_to_net,
        }
    }

    /// Snap a location to tolerance grid for matching.
    fn snap_location(&self, loc: (i32, i32)) -> (i32, i32) {
        let tol = self.tolerance.max(1);
        ((loc.0 / tol) * tol, (loc.1 / tol) * tol)
    }

    /// Get the endpoint of a pin (where wires connect).
    fn get_pin_endpoint(&self, pin: &SchPin) -> (i32, i32) {
        use crate::records::sch::PinConglomerateFlags;

        let base_x = pin.graphical.location_x;
        let base_y = pin.graphical.location_y;
        let length = pin.pin_length;

        let rotated = pin.pin_conglomerate.contains(PinConglomerateFlags::ROTATED);
        let flipped = pin.pin_conglomerate.contains(PinConglomerateFlags::FLIPPED);

        let (dx, dy) = match (rotated, flipped) {
            (false, false) => (length, 0),
            (true, false) => (0, length),
            (false, true) => (-length, 0),
            (true, true) => (0, -length),
        };

        (base_x + dx, base_y + dy)
    }

    /// Check if a point lies on a wire segment.
    fn point_on_segment(&self, point: (i32, i32), start: (i32, i32), end: (i32, i32)) -> bool {
        let (px, py) = point;
        let (sx, sy) = start;
        let (ex, ey) = end;

        // Check if point is within bounding box
        let min_x = sx.min(ex) - self.tolerance;
        let max_x = sx.max(ex) + self.tolerance;
        let min_y = sy.min(ey) - self.tolerance;
        let max_y = sy.max(ey) + self.tolerance;

        if px < min_x || px > max_x || py < min_y || py > max_y {
            return false;
        }

        // For horizontal segment
        if sy == ey && (py - sy).abs() <= self.tolerance {
            return true;
        }

        // For vertical segment
        if sx == ex && (px - sx).abs() <= self.tolerance {
            return true;
        }

        false
    }

    /// Find the net name from connections.
    fn find_net_name(&self, connections: &[ConnectionPoint], counter: &mut u32) -> (String, bool) {
        // Priority: Net label > Power port > Port > Auto-generated

        // Check for net label
        for conn in connections {
            if let ConnectionKind::NetLabel { name } = &conn.kind {
                if !name.is_empty() {
                    return (name.clone(), true);
                }
            }
        }

        // Check for power port
        for conn in connections {
            if let ConnectionKind::PowerPort { name } = &conn.kind {
                if !name.is_empty() {
                    return (name.clone(), true);
                }
            }
        }

        // Check for port
        for conn in connections {
            if let ConnectionKind::Port { name } = &conn.kind {
                if !name.is_empty() {
                    return (name.clone(), true);
                }
            }
        }

        // Auto-generate
        let name = format!("Net{}", counter);
        *counter += 1;
        (name, false)
    }

    /// Find unconnected pins in the schematic.
    pub fn find_unconnected_pins(
        &self,
        primitives: &[SchRecord],
    ) -> Vec<(String, String, (i32, i32))> {
        let netlist = self.build(primitives);
        let mut unconnected = Vec::new();

        // Find nets with only a single pin connection
        for net in &netlist.nets {
            let pins: Vec<_> = net
                .connections
                .iter()
                .filter(|c| matches!(c.kind, ConnectionKind::Pin { .. }))
                .collect();

            if pins.len() == 1 {
                // Check if net has any wire connections
                let has_wires = net
                    .connections
                    .iter()
                    .any(|c| matches!(c.kind, ConnectionKind::WireEnd { .. }));

                if !has_wires {
                    if let ConnectionKind::Pin { component, pin } = &pins[0].kind {
                        unconnected.push((component.clone(), pin.clone(), pins[0].location));
                    }
                }
            }
        }

        unconnected
    }

    /// Find locations where junctions might be needed.
    pub fn find_missing_junctions(&self, primitives: &[SchRecord]) -> Vec<(i32, i32)> {
        let mut wire_points: HashMap<(i32, i32), usize> = HashMap::new();
        let mut junction_locations: HashSet<(i32, i32)> = HashSet::new();

        // Collect existing junction locations
        for record in primitives {
            if let SchRecord::Junction(j) = record {
                junction_locations.insert((j.graphical.location_x, j.graphical.location_y));
            }
        }

        // Count wire connections at each point
        for record in primitives {
            if let SchRecord::Wire(wire) = record {
                for i in 0..wire.vertices.len().saturating_sub(1) {
                    let start = wire.vertices[i];
                    let end = wire.vertices[i + 1];

                    // Count endpoints
                    *wire_points.entry(start).or_insert(0) += 1;
                    *wire_points.entry(end).or_insert(0) += 1;

                    // Check for T-junctions (wire crossing another wire's midpoint)
                    for other_record in primitives {
                        if let SchRecord::Wire(other_wire) = other_record {
                            for j in 0..other_wire.vertices.len().saturating_sub(1) {
                                let other_start = other_wire.vertices[j];
                                let other_end = other_wire.vertices[j + 1];

                                // Check if our wire's endpoint lies on another wire's segment
                                if self.point_on_segment(start, other_start, other_end)
                                    && start != other_start
                                    && start != other_end
                                {
                                    *wire_points.entry(start).or_insert(0) += 1;
                                }
                                if self.point_on_segment(end, other_start, other_end)
                                    && end != other_start
                                    && end != other_end
                                {
                                    *wire_points.entry(end).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Find points with 3+ connections that don't have junctions
        let mut missing = Vec::new();
        for (loc, count) in wire_points {
            if count >= 3 && !junction_locations.contains(&loc) {
                missing.push(loc);
            }
        }

        missing
    }
}

/// Union-Find data structure for connectivity.
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
            rank: vec![0; size],
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn union(&mut self, x: usize, y: usize) {
        let root_x = self.find(x);
        let root_y = self.find(y);

        if root_x != root_y {
            if self.rank[root_x] < self.rank[root_y] {
                self.parent[root_x] = root_y;
            } else if self.rank[root_x] > self.rank[root_y] {
                self.parent[root_y] = root_x;
            } else {
                self.parent[root_y] = root_x;
                self.rank[root_x] += 1;
            }
        }
    }
}

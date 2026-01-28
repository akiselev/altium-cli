# Layout and Routing Engine Implementation Plan

This document outlines a comprehensive plan to implement a robust automatic layout and routing engine for Altium schematic documents (SchDoc). The engine will automatically place components and route wire connections using state-of-the-art algorithms.

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Recommended Rust Crates](#recommended-rust-crates)
4. [Phase 1: Foundation](#phase-1-foundation)
5. [Phase 2: Layout Engine](#phase-2-layout-engine)
6. [Phase 3: Routing Engine](#phase-3-routing-engine)
7. [Phase 4: Optimization](#phase-4-optimization)
8. [Phase 5: Integration](#phase-5-integration)
9. [API Design](#api-design)
10. [Testing Strategy](#testing-strategy)

---

## Overview

### Goals

1. **Automatic Component Placement**: Given a netlist of components and connections, automatically arrange components on a schematic sheet with optimal placement
2. **Automatic Wire Routing**: Connect all pins according to the netlist with minimal wire crossings and clean routing
3. **Optimization**: Use simulated annealing, genetic algorithms, and force-directed layout to achieve high-quality results
4. **Incremental Updates**: Support adding/moving single components without re-laying out the entire schematic

### Current State

The codebase already has:
- Basic `LayoutEngine` with collision detection and placement suggestions (`crates/altium-format/src/edit/layout.rs`)
- Basic `RoutingEngine` with A* pathfinding (`crates/altium-format/src/edit/routing.rs`)
- `EditSession` for managing schematic modifications (`crates/altium-format/src/edit/session.rs`)
- `NetlistBuilder` for connectivity analysis

### What's Needed

1. Graph representation of netlist connectivity
2. Force-directed layout for initial placement
3. Simulated annealing for placement optimization
4. Advanced routing with rip-up and reroute
5. Multi-net routing optimization
6. Spatial indexing for performance

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         AutoLayoutEngine                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐              │
│  │  NetlistGraph │───▶│ PlacementSolver│───▶│RoutingSolver │              │
│  └──────────────┘    └──────────────┘    └──────────────┘              │
│         │                   │                    │                       │
│         ▼                   ▼                    ▼                       │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐              │
│  │   petgraph   │    │Force-Directed│    │   A* / HPA*  │              │
│  │   (graph)    │    │  + Sim. Ann. │    │   Routing    │              │
│  └──────────────┘    └──────────────┘    └──────────────┘              │
│                             │                    │                       │
│                             ▼                    ▼                       │
│                      ┌──────────────┐    ┌──────────────┐              │
│                      │    rstar     │    │    rstar     │              │
│                      │ (R*-tree)    │    │ (obstacles)  │              │
│                      └──────────────┘    └──────────────┘              │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Module Structure

```
crates/altium-format/src/
├── edit/
│   ├── mod.rs
│   ├── layout.rs          # Existing - extend
│   ├── routing.rs         # Existing - extend
│   ├── session.rs         # Existing
│   └── autolayout/        # NEW
│       ├── mod.rs
│       ├── graph.rs       # Netlist graph representation
│       ├── placement.rs   # Placement algorithms
│       ├── force.rs       # Force-directed layout
│       ├── annealing.rs   # Simulated annealing
│       ├── genetic.rs     # Genetic algorithm (optional)
│       ├── router.rs      # Advanced routing
│       ├── spatial.rs     # R*-tree spatial index
│       └── config.rs      # Configuration options
```

---

## Recommended Rust Crates

### Core Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| [petgraph](https://crates.io/crates/petgraph) | 0.6 | Graph data structures and algorithms |
| [rstar](https://crates.io/crates/rstar) | 0.12 | R*-tree spatial indexing for collision detection |
| [pathfinding](https://crates.io/crates/pathfinding) | 4.8 | A*, Dijkstra, and other pathfinding algorithms |
| [argmin](https://crates.io/crates/argmin) | 0.10 | Optimization framework (includes simulated annealing) |
| [rand](https://crates.io/crates/rand) | 0.8 | Random number generation for stochastic algorithms |

### Optional/Alternative Crates

| Crate | Purpose | When to Use |
|-------|---------|-------------|
| [forceatlas2](https://crates.io/crates/forceatlas2) | Force-directed graph layout | If petgraph layout insufficient |
| [fdg-sim](https://crates.io/crates/fdg-sim) | Force-directed simulation | Alternative force layout |
| [simulated_annealing](https://crates.io/crates/simulated_annealing) | Standalone SA | Simpler than argmin for SA-only |
| [genevo](https://crates.io/crates/genevo) | Genetic algorithms | For GA-based placement optimization |
| [hierarchical_pathfinding](https://crates.io/crates/hierarchical_pathfinding) | HPA* routing | Large schematics with many nets |
| [rayon](https://crates.io/crates/rayon) | Parallel iteration | Multi-threaded optimization |

### Cargo.toml Additions

```toml
[dependencies]
# Graph representation
petgraph = "0.6"

# Spatial indexing
rstar = "0.12"

# Pathfinding algorithms
pathfinding = "4.8"

# Optimization framework
argmin = "0.10"
argmin-math = { version = "0.4", features = ["ndarray_latest-serde"] }

# Random number generation
rand = "0.8"
rand_chacha = "0.3"  # Reproducible RNG for testing

# Parallel processing
rayon = "1.10"

# Optional: Genetic algorithms
genevo = { version = "0.7", optional = true }

[features]
default = []
genetic = ["genevo"]
```

---

## Phase 1: Foundation

### 1.1 Netlist Graph Representation

Create a graph structure representing component connectivity:

```rust
// crates/altium-format/src/edit/autolayout/graph.rs

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::connected_components;
use std::collections::HashMap;

/// A node in the netlist graph
#[derive(Debug, Clone)]
pub enum NetlistNode {
    /// A component (U1, R1, C1, etc.)
    Component {
        index: usize,           // Index in SchDoc primitives
        designator: String,
        lib_reference: String,
        pin_count: usize,
        bounds_width: i32,
        bounds_height: i32,
    },
    /// A net (named connection point)
    Net {
        name: String,
        is_power: bool,         // VCC, GND, etc.
    },
}

/// An edge representing a pin connection
#[derive(Debug, Clone)]
pub struct NetlistEdge {
    pub pin_designator: String,
    pub pin_name: String,
    pub pin_electrical_type: PinElectricalType,
}

/// Graph representing schematic connectivity
pub struct NetlistGraph {
    pub graph: DiGraph<NetlistNode, NetlistEdge>,
    pub component_nodes: HashMap<String, NodeIndex>,  // designator -> node
    pub net_nodes: HashMap<String, NodeIndex>,        // net name -> node
}

impl NetlistGraph {
    /// Build graph from SchDoc primitives
    pub fn from_primitives(primitives: &[SchRecord]) -> Self { ... }

    /// Get all components connected to a net
    pub fn components_on_net(&self, net: &str) -> Vec<&str> { ... }

    /// Get all nets connected to a component
    pub fn nets_on_component(&self, designator: &str) -> Vec<&str> { ... }

    /// Calculate connectivity strength between two components
    /// (number of shared nets)
    pub fn connectivity_strength(&self, comp1: &str, comp2: &str) -> usize { ... }

    /// Find connected subgraphs (for multi-sheet schematics)
    pub fn connected_subgraphs(&self) -> Vec<Vec<NodeIndex>> { ... }
}
```

### 1.2 Spatial Index

Implement R*-tree based spatial indexing:

```rust
// crates/altium-format/src/edit/autolayout/spatial.rs

use rstar::{RTree, RTreeObject, AABB};
use crate::types::{Coord, CoordPoint, CoordRect};

/// A component bounding box for spatial indexing
#[derive(Debug, Clone)]
pub struct ComponentBounds {
    pub index: usize,
    pub designator: String,
    pub bounds: CoordRect,
}

impl RTreeObject for ComponentBounds {
    type Envelope = AABB<[i32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(
            [self.bounds.location1.x.to_raw(), self.bounds.location1.y.to_raw()],
            [self.bounds.location2.x.to_raw(), self.bounds.location2.y.to_raw()],
        )
    }
}

/// Spatial index for fast collision detection and nearest neighbor queries
pub struct SpatialIndex {
    tree: RTree<ComponentBounds>,
    wire_tree: RTree<WireSegmentBounds>,  // For routing
}

impl SpatialIndex {
    pub fn new() -> Self { ... }

    /// Build from current schematic state
    pub fn from_primitives(primitives: &[SchRecord], layout: &LayoutEngine) -> Self { ... }

    /// Find all components intersecting a rectangle
    pub fn query_rect(&self, rect: CoordRect) -> Vec<&ComponentBounds> { ... }

    /// Find k nearest components to a point
    pub fn nearest_k(&self, point: CoordPoint, k: usize) -> Vec<&ComponentBounds> { ... }

    /// Check if placement is valid (no collisions)
    pub fn is_valid_placement(&self, bounds: CoordRect, exclude: Option<usize>) -> bool { ... }

    /// Update after moving a component
    pub fn update_component(&mut self, index: usize, new_bounds: CoordRect) { ... }
}
```

### 1.3 Configuration

```rust
// crates/altium-format/src/edit/autolayout/config.rs

/// Configuration for automatic layout
#[derive(Debug, Clone)]
pub struct AutoLayoutConfig {
    // Sheet settings
    pub sheet_width: Coord,
    pub sheet_height: Coord,
    pub margin: Coord,

    // Grid settings
    pub grid_spacing: Coord,
    pub snap_to_grid: bool,

    // Placement settings
    pub component_spacing: Coord,
    pub group_by_function: bool,        // Group ICs, passives, etc.
    pub align_power_rails: bool,        // Align VCC/GND connections

    // Force-directed settings
    pub force_iterations: usize,
    pub force_strength: f64,
    pub force_damping: f64,

    // Simulated annealing settings
    pub sa_initial_temp: f64,
    pub sa_cooling_rate: f64,
    pub sa_iterations_per_temp: usize,
    pub sa_min_temp: f64,

    // Routing settings
    pub prefer_orthogonal: bool,        // Prefer horizontal/vertical wires
    pub max_routing_iterations: usize,
    pub allow_wire_crossings: bool,
    pub crossing_penalty: f64,
}

impl Default for AutoLayoutConfig {
    fn default() -> Self {
        Self {
            sheet_width: Coord::from_mils(11000.0),
            sheet_height: Coord::from_mils(8500.0),
            margin: Coord::from_mils(500.0),
            grid_spacing: Coord::from_mils(10.0),
            snap_to_grid: true,
            component_spacing: Coord::from_mils(100.0),
            group_by_function: true,
            align_power_rails: true,
            force_iterations: 500,
            force_strength: 1.0,
            force_damping: 0.9,
            sa_initial_temp: 1000.0,
            sa_cooling_rate: 0.995,
            sa_iterations_per_temp: 100,
            sa_min_temp: 1.0,
            max_routing_iterations: 10000,
            prefer_orthogonal: true,
            allow_wire_crossings: true,
            crossing_penalty: 5.0,
        }
    }
}
```

---

## Phase 2: Layout Engine

### 2.1 Force-Directed Layout

Initial placement using force-directed algorithm:

```rust
// crates/altium-format/src/edit/autolayout/force.rs

use crate::types::{Coord, CoordPoint};
use super::graph::NetlistGraph;

/// Force-directed layout using Fruchterman-Reingold algorithm
pub struct ForceDirectedLayout {
    positions: HashMap<String, (f64, f64)>,  // designator -> (x, y)
    velocities: HashMap<String, (f64, f64)>,
    config: ForceConfig,
}

#[derive(Debug, Clone)]
pub struct ForceConfig {
    pub area_width: f64,
    pub area_height: f64,
    pub optimal_distance: f64,      // k = sqrt(area / |V|)
    pub attraction_strength: f64,   // Edge spring constant
    pub repulsion_strength: f64,    // Node repulsion constant
    pub damping: f64,               // Velocity damping
    pub max_displacement: f64,      // Limit movement per iteration
}

impl ForceDirectedLayout {
    pub fn new(graph: &NetlistGraph, config: ForceConfig) -> Self { ... }

    /// Run one iteration of force calculation
    pub fn step(&mut self, graph: &NetlistGraph) {
        // Calculate repulsive forces (all pairs)
        for (v, pos_v) in &self.positions {
            let mut force = (0.0, 0.0);
            for (u, pos_u) in &self.positions {
                if v != u {
                    let delta = (pos_v.0 - pos_u.0, pos_v.1 - pos_u.1);
                    let distance = (delta.0.powi(2) + delta.1.powi(2)).sqrt().max(0.01);
                    let repulsion = self.config.repulsion_strength.powi(2) / distance;
                    force.0 += delta.0 / distance * repulsion;
                    force.1 += delta.1 / distance * repulsion;
                }
            }
            // Store force for later application
        }

        // Calculate attractive forces (edges only)
        for edge in graph.graph.edge_indices() {
            // Spring force between connected components
        }

        // Apply forces with damping
        // Constrain to bounds
    }

    /// Run until convergence or max iterations
    pub fn run(&mut self, graph: &NetlistGraph, max_iterations: usize) -> bool { ... }

    /// Get final positions
    pub fn positions(&self) -> &HashMap<String, (f64, f64)> { ... }

    /// Convert to grid-snapped CoordPoints
    pub fn to_coord_positions(&self, grid: &Grid) -> HashMap<String, CoordPoint> { ... }
}
```

### 2.2 Simulated Annealing Placement

Optimize placement using simulated annealing:

```rust
// crates/altium-format/src/edit/autolayout/annealing.rs

use argmin::core::{CostFunction, State};
use rand::Rng;

/// Placement state for simulated annealing
#[derive(Clone)]
pub struct PlacementState {
    pub positions: Vec<(i32, i32)>,     // Component positions
    pub orientations: Vec<Orientation>, // Component rotations
}

/// Cost function for placement optimization
pub struct PlacementCost {
    graph: NetlistGraph,
    component_sizes: Vec<(i32, i32)>,   // Width, height for each component
    config: AutoLayoutConfig,
}

impl PlacementCost {
    /// Calculate total cost of a placement
    pub fn evaluate(&self, state: &PlacementState) -> f64 {
        let mut cost = 0.0;

        // 1. Wire length cost (HPWL - Half-Perimeter Wire Length)
        cost += self.wire_length_cost(state) * 1.0;

        // 2. Overlap penalty
        cost += self.overlap_penalty(state) * 1000.0;

        // 3. Boundary violation penalty
        cost += self.boundary_penalty(state) * 500.0;

        // 4. Alignment bonus (reduce cost for aligned components)
        cost -= self.alignment_bonus(state) * 0.5;

        // 5. Congestion estimate (routing difficulty)
        cost += self.congestion_estimate(state) * 2.0;

        cost
    }

    /// Half-Perimeter Wire Length estimation
    fn wire_length_cost(&self, state: &PlacementState) -> f64 {
        let mut total = 0.0;
        for net in self.graph.net_nodes.keys() {
            let pins = self.get_pins_on_net(net, state);
            if pins.len() >= 2 {
                let (min_x, max_x, min_y, max_y) = self.bounding_box(&pins);
                total += (max_x - min_x + max_y - min_y) as f64;
            }
        }
        total
    }

    /// Overlap penalty - heavily penalize overlapping components
    fn overlap_penalty(&self, state: &PlacementState) -> f64 { ... }

    /// Penalty for components outside sheet boundaries
    fn boundary_penalty(&self, state: &PlacementState) -> f64 { ... }

    /// Bonus for well-aligned components
    fn alignment_bonus(&self, state: &PlacementState) -> f64 { ... }

    /// Estimate routing congestion in regions
    fn congestion_estimate(&self, state: &PlacementState) -> f64 { ... }
}

/// Simulated annealing placement optimizer
pub struct SimulatedAnnealingPlacer {
    cost_fn: PlacementCost,
    config: AnnealingConfig,
    rng: rand_chacha::ChaCha8Rng,
}

#[derive(Debug, Clone)]
pub struct AnnealingConfig {
    pub initial_temp: f64,
    pub cooling_rate: f64,
    pub iterations_per_temp: usize,
    pub min_temp: f64,
    pub move_types: Vec<MoveType>,
}

#[derive(Debug, Clone)]
pub enum MoveType {
    Translate { max_distance: i32 },
    Rotate,
    Swap,                              // Swap two components
    Mirror,
}

impl SimulatedAnnealingPlacer {
    pub fn new(graph: NetlistGraph, config: AnnealingConfig) -> Self { ... }

    /// Generate a neighbor state by making a random move
    fn generate_neighbor(&mut self, current: &PlacementState) -> PlacementState {
        let move_type = self.config.move_types
            .choose(&mut self.rng)
            .unwrap();

        match move_type {
            MoveType::Translate { max_distance } => {
                // Move random component by random offset
            }
            MoveType::Rotate => {
                // Rotate random component 90°
            }
            MoveType::Swap => {
                // Swap positions of two similar-sized components
            }
            MoveType::Mirror => {
                // Mirror component horizontally
            }
        }
    }

    /// Run simulated annealing optimization
    pub fn optimize(&mut self, initial: PlacementState) -> PlacementState {
        let mut current = initial;
        let mut current_cost = self.cost_fn.evaluate(&current);
        let mut best = current.clone();
        let mut best_cost = current_cost;

        let mut temp = self.config.initial_temp;

        while temp > self.config.min_temp {
            for _ in 0..self.config.iterations_per_temp {
                let neighbor = self.generate_neighbor(&current);
                let neighbor_cost = self.cost_fn.evaluate(&neighbor);

                let delta = neighbor_cost - current_cost;

                // Accept if better, or probabilistically if worse
                let accept = delta < 0.0 ||
                    self.rng.gen::<f64>() < (-delta / temp).exp();

                if accept {
                    current = neighbor;
                    current_cost = neighbor_cost;

                    if current_cost < best_cost {
                        best = current.clone();
                        best_cost = current_cost;
                    }
                }
            }

            temp *= self.config.cooling_rate;
        }

        best
    }
}
```

### 2.3 Component Grouping

Group related components for better organization:

```rust
// crates/altium-format/src/edit/autolayout/grouping.rs

/// Strategy for grouping components
pub enum GroupingStrategy {
    /// Group by component type (ICs, resistors, capacitors, etc.)
    ByType,
    /// Group by function (identified by net connectivity)
    ByFunction,
    /// Group by hierarchical sheet
    BySheet,
    /// Custom grouping function
    Custom(Box<dyn Fn(&SchComponent) -> String>),
}

/// Identifies functional blocks in a schematic
pub struct FunctionalGrouper {
    graph: NetlistGraph,
}

impl FunctionalGrouper {
    /// Identify power supply section
    pub fn find_power_section(&self) -> Vec<String> { ... }

    /// Identify clock/oscillator section
    pub fn find_clock_section(&self) -> Vec<String> { ... }

    /// Identify I/O interface section
    pub fn find_io_section(&self) -> Vec<String> { ... }

    /// Identify analog section (op-amps, ADCs, etc.)
    pub fn find_analog_section(&self) -> Vec<String> { ... }

    /// Use community detection algorithm to find groups
    pub fn detect_communities(&self) -> Vec<Vec<String>> {
        // Use modularity-based clustering on the netlist graph
    }
}
```

---

## Phase 3: Routing Engine

### 3.1 Enhanced A* Router

Improve existing A* with better heuristics:

```rust
// crates/altium-format/src/edit/autolayout/router.rs

use pathfinding::prelude::*;

/// Enhanced A* router with schematic-specific optimizations
pub struct SchematicRouter {
    spatial_index: SpatialIndex,
    config: RoutingConfig,
}

#[derive(Debug, Clone)]
pub struct RoutingConfig {
    pub grid_spacing: Coord,
    pub prefer_orthogonal: bool,
    pub turn_cost: f64,
    pub crossing_cost: f64,
    pub via_cost: f64,              // For multi-layer (future)
    pub hug_component_bonus: f64,   // Prefer routes along component edges
}

impl SchematicRouter {
    /// Route a single net (may have multiple pins)
    pub fn route_net(&self, pins: &[CoordPoint]) -> Option<Vec<WireSegment>> {
        if pins.len() < 2 {
            return None;
        }

        // For 2 pins: direct A* routing
        if pins.len() == 2 {
            return self.route_two_points(pins[0], pins[1]);
        }

        // For multi-pin nets: use Steiner tree approximation
        self.route_steiner(pins)
    }

    /// A* routing between two points
    fn route_two_points(&self, start: CoordPoint, end: CoordPoint) -> Option<Vec<WireSegment>> {
        let result = astar(
            &start,
            |p| self.successors(*p, end),
            |p| self.heuristic(*p, end),
            |p| *p == end,
        )?;

        Some(self.path_to_segments(result.0))
    }

    /// Steiner tree routing for multi-pin nets
    fn route_steiner(&self, pins: &[CoordPoint]) -> Option<Vec<WireSegment>> {
        // 1. Build minimum spanning tree of pins
        // 2. For each edge in MST, route with A*
        // 3. Add Steiner points at intersections
    }

    /// Generate valid successors for A* node
    fn successors(&self, pos: CoordPoint, target: CoordPoint) -> Vec<(CoordPoint, i64)> {
        let mut successors = Vec::new();
        let step = self.config.grid_spacing.to_raw();

        // Orthogonal moves
        for (dx, dy) in [(step, 0), (-step, 0), (0, step), (0, -step)] {
            let next = CoordPoint::from_raw(pos.x.to_raw() + dx, pos.y.to_raw() + dy);
            if self.is_valid_position(next, pos, target) {
                let cost = self.move_cost(pos, next);
                successors.push((next, cost));
            }
        }

        // Optional diagonal moves (45°)
        if !self.config.prefer_orthogonal {
            for (dx, dy) in [(step, step), (step, -step), (-step, step), (-step, -step)] {
                // ...
            }
        }

        successors
    }
}
```

### 3.2 Rip-Up and Reroute

Handle routing failures by removing conflicting routes:

```rust
/// Rip-up and reroute strategy for handling routing failures
pub struct RipUpRouter {
    base_router: SchematicRouter,
    max_rip_iterations: usize,
}

impl RipUpRouter {
    /// Route all nets with rip-up and reroute
    pub fn route_all(&mut self, nets: &[Net]) -> RoutingResult {
        let mut routed: HashMap<String, Vec<WireSegment>> = HashMap::new();
        let mut failed: Vec<String> = Vec::new();

        // Sort nets by difficulty (longer nets first)
        let mut sorted_nets = nets.to_vec();
        sorted_nets.sort_by_key(|n| std::cmp::Reverse(n.pins.len()));

        for net in &sorted_nets {
            match self.try_route_net(net, &routed) {
                Some(segments) => {
                    routed.insert(net.name.clone(), segments);
                }
                None => {
                    // Try rip-up
                    if let Some(segments) = self.rip_up_and_route(net, &mut routed) {
                        routed.insert(net.name.clone(), segments);
                    } else {
                        failed.push(net.name.clone());
                    }
                }
            }
        }

        RoutingResult { routed, failed }
    }

    /// Rip up conflicting nets and try to reroute
    fn rip_up_and_route(
        &self,
        net: &Net,
        routed: &mut HashMap<String, Vec<WireSegment>>,
    ) -> Option<Vec<WireSegment>> {
        // Find nets that block this route
        let blocking_nets = self.find_blocking_nets(net, routed);

        for iteration in 0..self.max_rip_iterations {
            // Rip up blocking net with lowest priority
            let to_rip = self.select_net_to_rip(&blocking_nets, routed)?;
            let ripped_segments = routed.remove(&to_rip)?;

            // Try to route our net
            if let Some(segments) = self.try_route_net(net, routed) {
                // Try to re-route the ripped net
                if let Some(re_routed) = self.try_route_net_by_name(&to_rip, routed) {
                    routed.insert(to_rip, re_routed);
                    return Some(segments);
                }
            }

            // Restore if failed
            routed.insert(to_rip, ripped_segments);
        }

        None
    }
}
```

### 3.3 Wire Bundling

Group parallel wires for cleaner schematics:

```rust
/// Bundle parallel wires for cleaner appearance
pub struct WireBundler {
    bundle_spacing: Coord,
}

impl WireBundler {
    /// Identify wires that can be bundled
    pub fn find_bundles(&self, wires: &[Vec<WireSegment>]) -> Vec<WireBundle> {
        // Find wires with parallel segments
        // Group by direction and proximity
    }

    /// Adjust wire positions to create clean bundles
    pub fn create_bundles(&self, bundles: &[WireBundle]) -> Vec<Vec<WireSegment>> {
        // Space wires evenly within bundle
        // Align entry/exit points
    }
}
```

---

## Phase 4: Optimization

### 4.1 Genetic Algorithm (Optional)

For very complex layouts, genetic algorithms can explore more solution space:

```rust
// crates/altium-format/src/edit/autolayout/genetic.rs
// (Only with `genetic` feature flag)

use genevo::prelude::*;

/// Genetic algorithm for placement optimization
pub struct GeneticPlacer {
    graph: NetlistGraph,
    config: GeneticConfig,
}

#[derive(Debug, Clone)]
pub struct GeneticConfig {
    pub population_size: usize,
    pub generations: usize,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
    pub elitism_count: usize,
}

/// Chromosome representing a placement solution
#[derive(Clone)]
pub struct PlacementChromosome {
    // Encoded as sequence of component positions and orientations
    genes: Vec<PlacementGene>,
}

impl Genotype for PlacementChromosome {
    type Dna = PlacementGene;
}

impl GeneticPlacer {
    pub fn optimize(&self, initial: PlacementState) -> PlacementState {
        // Initialize population from initial state with variations
        // Run evolutionary loop
        // Return best individual
    }
}
```

### 4.2 Incremental Layout

Support for modifying existing layouts:

```rust
/// Incremental layout engine for modifying existing schematics
pub struct IncrementalLayout {
    current_state: PlacementState,
    spatial_index: SpatialIndex,
}

impl IncrementalLayout {
    /// Add a new component to existing layout
    pub fn add_component(
        &mut self,
        component: &SchComponent,
        connected_to: &[String],  // Existing component designators
    ) -> CoordPoint {
        // Find centroid of connected components
        // Use local optimization to find good position
        // Run limited simulated annealing in region
    }

    /// Move a component and adjust neighbors
    pub fn move_component(
        &mut self,
        designator: &str,
        new_position: CoordPoint,
    ) -> Vec<(String, CoordPoint)> {
        // Move component
        // Check for overlaps
        // Push overlapping components away
        // Re-optimize local region
    }

    /// Re-layout a subset of components
    pub fn relayout_region(&mut self, region: CoordRect) -> Vec<(String, CoordPoint)> {
        // Extract components in region
        // Run simulated annealing on subset
        // Return new positions
    }
}
```

---

## Phase 5: Integration

### 5.1 AutoLayoutEngine

Main entry point combining all algorithms:

```rust
// crates/altium-format/src/edit/autolayout/mod.rs

pub struct AutoLayoutEngine {
    config: AutoLayoutConfig,
}

impl AutoLayoutEngine {
    pub fn new(config: AutoLayoutConfig) -> Self {
        Self { config }
    }

    /// Perform complete auto-layout of schematic
    pub fn auto_layout(&self, session: &mut EditSession) -> Result<AutoLayoutResult> {
        // 1. Build netlist graph
        let graph = NetlistGraph::from_primitives(&session.doc.primitives);

        // 2. Initial placement using force-directed layout
        let force_layout = ForceDirectedLayout::new(&graph, self.force_config());
        let initial = force_layout.run(&graph, self.config.force_iterations);

        // 3. Optimize placement using simulated annealing
        let sa_placer = SimulatedAnnealingPlacer::new(graph.clone(), self.sa_config());
        let optimized = sa_placer.optimize(initial.to_placement_state());

        // 4. Apply placement to schematic
        self.apply_placement(session, &optimized)?;

        // 5. Route all nets
        let router = SchematicRouter::new(self.routing_config());
        let routing_result = router.route_all(&graph)?;

        // 6. Apply routing to schematic
        self.apply_routing(session, &routing_result)?;

        Ok(AutoLayoutResult {
            components_placed: graph.component_nodes.len(),
            wires_routed: routing_result.routed.len(),
            failed_routes: routing_result.failed.clone(),
            total_wire_length: routing_result.total_length(),
            wire_crossings: routing_result.crossing_count(),
        })
    }

    /// Layout only (no routing)
    pub fn auto_place(&self, session: &mut EditSession) -> Result<PlacementResult> { ... }

    /// Route only (keep existing placement)
    pub fn auto_route(&self, session: &mut EditSession) -> Result<RoutingResult> { ... }

    /// Add single component with smart placement
    pub fn place_component(
        &self,
        session: &mut EditSession,
        lib_reference: &str,
        connected_pins: &[(String, String)],  // (component, pin) pairs
    ) -> Result<CoordPoint> { ... }
}
```

### 5.2 CLI Integration

Add commands to altium-cli:

```rust
// In crates/altium-cli/src/commands/autolayout.rs

/// Auto-layout command
#[derive(Parser)]
pub struct AutoLayoutCmd {
    /// Input schematic file
    input: PathBuf,

    /// Output schematic file
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Only place components, don't route
    #[arg(long)]
    place_only: bool,

    /// Only route wires, keep existing placement
    #[arg(long)]
    route_only: bool,

    /// Component spacing in mils
    #[arg(long, default_value = "100")]
    spacing: f64,

    /// Number of optimization iterations
    #[arg(long, default_value = "1000")]
    iterations: usize,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

impl AutoLayoutCmd {
    pub fn run(&self) -> Result<()> {
        let mut session = EditSession::open(&self.input)?;

        let config = AutoLayoutConfig {
            component_spacing: Coord::from_mils(self.spacing),
            sa_iterations_per_temp: self.iterations,
            ..Default::default()
        };

        let engine = AutoLayoutEngine::new(config);

        let result = if self.place_only {
            engine.auto_place(&mut session)?
        } else if self.route_only {
            engine.auto_route(&mut session)?
        } else {
            engine.auto_layout(&mut session)?
        };

        if self.verbose {
            println!("Components placed: {}", result.components_placed);
            println!("Wires routed: {}", result.wires_routed);
            println!("Wire crossings: {}", result.wire_crossings);
        }

        let output = self.output.as_ref().unwrap_or(&self.input);
        session.save(output)?;

        Ok(())
    }
}
```

---

## API Design

### Public API

```rust
// Re-export main types
pub use autolayout::{
    AutoLayoutEngine,
    AutoLayoutConfig,
    AutoLayoutResult,
    PlacementResult,
    RoutingResult,
};

// EditSession extensions
impl EditSession {
    /// Automatically layout all components
    pub fn auto_layout(&mut self, config: AutoLayoutConfig) -> Result<AutoLayoutResult> {
        AutoLayoutEngine::new(config).auto_layout(self)
    }

    /// Automatically place components without routing
    pub fn auto_place(&mut self, config: AutoLayoutConfig) -> Result<PlacementResult> {
        AutoLayoutEngine::new(config).auto_place(self)
    }

    /// Automatically route all nets
    pub fn auto_route(&mut self, config: RoutingConfig) -> Result<RoutingResult> {
        AutoLayoutEngine::new(config.into()).auto_route(self)
    }

    /// Smart placement for new component
    pub fn smart_add_component(
        &mut self,
        lib_reference: &str,
        connected_to: &[(String, String)],
    ) -> Result<usize> {
        let engine = AutoLayoutEngine::new(AutoLayoutConfig::default());
        let position = engine.place_component(self, lib_reference, connected_to)?;
        self.add_component(lib_reference, position, Orientation::Normal, None)
    }
}
```

### Example Usage

```rust
use altium_format::edit::{EditSession, AutoLayoutConfig};

fn main() -> Result<()> {
    // Create new schematic
    let mut session = EditSession::new();

    // Load component library
    session.load_library("components.SchLib")?;

    // Add components (positions don't matter yet)
    session.add_component("ATmega328P", CoordPoint::ZERO, Orientation::Normal, Some("U1"))?;
    session.add_component("Crystal", CoordPoint::ZERO, Orientation::Normal, Some("Y1"))?;
    session.add_component("22pF", CoordPoint::ZERO, Orientation::Normal, Some("C1"))?;
    session.add_component("22pF", CoordPoint::ZERO, Orientation::Normal, Some("C2"))?;
    session.add_component("10uF", CoordPoint::ZERO, Orientation::Normal, Some("C3"))?;

    // Define net connections (alternatively, parse from netlist)
    // This would typically come from the SchDoc's existing wire/net structure
    // or be defined programmatically

    // Auto-layout with custom config
    let config = AutoLayoutConfig {
        component_spacing: Coord::from_mils(150.0),
        sa_iterations_per_temp: 200,
        ..Default::default()
    };

    let result = session.auto_layout(config)?;

    println!("Placed {} components", result.components_placed);
    println!("Routed {} nets with {} crossings",
             result.wires_routed,
             result.wire_crossings);

    // Save result
    session.save("output.SchDoc")?;

    Ok(())
}
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_netlist_graph_construction() {
        // Test graph building from primitives
    }

    #[test]
    fn test_force_directed_converges() {
        // Test that force-directed layout converges
    }

    #[test]
    fn test_simulated_annealing_improves_cost() {
        // Test that SA reduces cost function
    }

    #[test]
    fn test_router_finds_path() {
        // Test A* routing finds valid path
    }

    #[test]
    fn test_router_avoids_obstacles() {
        // Test that router respects obstacles
    }

    #[test]
    fn test_rip_up_recovers_from_deadlock() {
        // Test rip-up and reroute
    }
}
```

### Integration Tests

```rust
#[test]
fn test_full_auto_layout_simple() {
    // Test complete auto-layout on simple schematic
    let mut session = create_test_schematic_with_components(5);
    let result = session.auto_layout(AutoLayoutConfig::default()).unwrap();

    assert!(result.failed_routes.is_empty());
    assert_eq!(result.components_placed, 5);
}

#[test]
fn test_auto_layout_complex() {
    // Test on larger schematic
    let mut session = load_test_schematic("complex_design.SchDoc");
    let result = session.auto_layout(AutoLayoutConfig::default()).unwrap();

    // Verify no overlaps
    let errors = session.validate();
    assert!(errors.iter().all(|e| e.kind != ValidationErrorKind::ComponentOverlap));
}
```

### Benchmarks

```rust
#[bench]
fn bench_force_directed_100_components(b: &mut Bencher) {
    let graph = create_random_graph(100, 150);
    b.iter(|| {
        let mut layout = ForceDirectedLayout::new(&graph, ForceConfig::default());
        layout.run(&graph, 500);
    });
}

#[bench]
fn bench_simulated_annealing_50_components(b: &mut Bencher) {
    let graph = create_random_graph(50, 75);
    b.iter(|| {
        let placer = SimulatedAnnealingPlacer::new(graph.clone(), AnnealingConfig::default());
        placer.optimize(random_placement(50));
    });
}
```

---

## Implementation Timeline

### Milestone 1: Foundation (Weeks 1-2)
- [ ] Add crate dependencies to Cargo.toml
- [ ] Implement `NetlistGraph` using petgraph
- [ ] Implement `SpatialIndex` using rstar
- [ ] Add configuration structures

### Milestone 2: Placement (Weeks 3-4)
- [ ] Implement `ForceDirectedLayout`
- [ ] Implement `SimulatedAnnealingPlacer`
- [ ] Add placement cost functions
- [ ] Integrate with `EditSession`

### Milestone 3: Routing (Weeks 5-6)
- [ ] Enhance A* router with better heuristics
- [ ] Implement multi-pin net routing (Steiner tree)
- [ ] Add rip-up and reroute capability
- [ ] Wire simplification and bundling

### Milestone 4: Integration (Week 7)
- [ ] Create `AutoLayoutEngine` facade
- [ ] Add CLI commands
- [ ] Comprehensive testing
- [ ] Documentation

### Milestone 5: Optimization (Week 8)
- [ ] Performance profiling and optimization
- [ ] Optional genetic algorithm support
- [ ] Incremental layout support
- [ ] Edge cases and robustness

---

## References

### Academic Papers

1. **Force-Directed Layout**: Fruchterman, T. M., & Reingold, E. M. (1991). Graph drawing by force-directed placement. Software: Practice and experience, 21(11), 1129-1164.

2. **Simulated Annealing Placement**: Kirkpatrick, S., Gelatt, C. D., & Vecchi, M. P. (1983). Optimization by simulated annealing. Science, 220(4598), 671-680.

3. **A* Pathfinding**: Hart, P. E., Nilsson, N. J., & Raphael, B. (1968). A formal basis for the heuristic determination of minimum cost paths. IEEE transactions on Systems Science and Cybernetics, 4(2), 100-107.

4. **Hierarchical Pathfinding**: Botea, A., Müller, M., & Schaeffer, J. (2004). Near optimal hierarchical path-finding. Journal of game development, 1(1), 7-28.

5. **VLSI Placement**: Shahookar, K., & Mazumder, P. (1991). VLSI cell placement techniques. ACM Computing Surveys (CSUR), 23(2), 143-220.

### Related Projects

- [KiCad Automated Routing Tools](https://hackaday.io/project/204891-kicad-automated-routing-tools) - Rust-accelerated autorouter for KiCad
- [FPGA SA Placer](https://stefanabikaram.com/writing/fpga-sa-placer/) - Simulated annealing FPGA placer in Rust

---

## Sources

- [pathfinding crate](https://crates.io/crates/pathfinding) - A*, Dijkstra, BFS, DFS algorithms
- [petgraph](https://github.com/petgraph/petgraph) - Graph data structure library
- [rstar](https://github.com/georust/rstar) - R*-tree spatial index
- [argmin](https://github.com/argmin-rs/argmin) - Numerical optimization framework
- [forceatlas2](https://crates.io/crates/forceatlas2) - Force-directed graph layout
- [fdg](https://github.com/grantshandy/fdg) - Force-directed graph library
- [genevo](https://github.com/innoave/genevo) - Genetic algorithm framework
- [hierarchical_pathfinding](https://github.com/mich101mich/hierarchical_pathfinding) - HPA* implementation
- [simulated_annealing](https://crates.io/crates/simulated_annealing) - SA optimization

# Phase 0: Netlist Clustering & Preprocessing — Implementation Specification

Preprocessing stage for the PCB autoplacer pipeline. Phase 0 runs before Phase 1
(global analytical placement via Solverang). Its job is to examine the netlist,
detect component clusters automatically, classify components into functional domains,
assign clusters to board regions, and produce a set of `UserConstraint` values that
are fed directly into the existing `solve_placement()` function.

**Pipeline position:**

```
PcbIr + user spec
        │
        ▼
[Phase 0: phase0_preprocess()]   ← this spec
        │  auto_clusters, final_groups, region_assignments,
        │  suggested_constraints, metadata
        ▼
[Phase 1: solve_placement()]     ← existing, in autopcb-placement/src/lib.rs
        │  PlacementResult
        ▼
[Phase 2: SA detailed placement] ← separate spec
```

---

## 1. Data Structures

All types live in `crates/autopcb-placement/src/clustering.rs` unless noted
otherwise.

### 1.1 Component Graph Node

```rust
/// Metadata attached to each node in the component graph.
/// Mirrors `IrComponent` fields needed for clustering decisions.
#[derive(Debug, Clone)]
pub struct ComponentNode {
    /// Designator string ("U1", "C3", "J2").
    pub designator: String,

    /// Footprint pattern name ("QFP-100", "0402", "USB-C-GCT").
    pub pattern: String,

    /// Bounding box half-dimensions in mm (from `IrComponent.local_bounds`).
    pub half_w: f64,
    pub half_h: f64,

    /// Total pad count for this component.
    pub pin_count: usize,

    /// Whether this component is a board connector.
    /// True if: pattern contains "USB", "CONN", "HDR", "DF", or "GCT"
    /// OR designator starts with 'J' or 'P'
    /// OR footprint has a pad on the board edge layer.
    pub is_connector: bool,

    /// Functional domain inferred from net names and component type.
    pub domain: Domain,
}
```

### 1.2 Functional Domain

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Domain {
    /// Power supply and regulation: voltage regulators, large capacitors on
    /// power nets, ferrite beads, power inductors.
    Power,

    /// Analog signal processing: op-amps, ADCs, DACs, analog filters,
    /// crystal oscillators, precision resistors on analog nets.
    Analog,

    /// Digital logic: MCUs, FPGAs, memories, digital ICs, passive
    /// components on digital nets.
    Digital,

    /// Not classifiable — default for components with no clear signals.
    Unknown,
}
```

### 1.3 Board Region

```rust
/// A named axis-aligned rectangular region on the board.
#[derive(Debug, Clone)]
pub struct BoardRegion {
    /// Short identifier used in constraint generation, e.g. "left_edge",
    /// "power_corner", "digital_center".
    pub name: String,

    /// Bounding rectangle in mm (board-relative coordinates).
    pub rect: RectRegion,  // from autopcb-placement: {min_x, min_y, max_x, max_y}
}

/// Canonical region names produced by `assign_regions()`.
/// Matches the named region shortcuts already in `lib.rs` where possible.
pub enum CanonicalRegion {
    TopEdge,
    BottomEdge,
    LeftEdge,
    RightEdge,
    TopLeftQuadrant,
    TopRightQuadrant,
    BottomLeftQuadrant,
    BottomRightQuadrant,
    Center,
}
```

### 1.4 Raw Cluster (from graph algorithm)

```rust
/// A cluster produced by BFS or spectral bisection before user merging.
#[derive(Debug, Clone)]
pub struct Cluster {
    /// Sequential integer ID assigned during construction.
    pub id: usize,

    /// Designators of all components in the cluster.
    pub members: Vec<String>,

    /// Dominant domain across all members (majority vote).
    pub domain: Domain,

    /// Estimated total bounding box area if packed without clearance (mm²).
    /// = Σ (2·half_w × 2·half_h) for all members.
    pub total_area_mm2: f64,

    /// Total shared-net edge weight within the cluster
    /// (sum of all intra-cluster edge weights).
    pub internal_cohesion: f64,
}
```

### 1.5 Auto-cluster (annotated cluster)

```rust
/// A cluster annotated with a board region assignment.
#[derive(Debug, Clone)]
pub struct AutoCluster {
    pub cluster: Cluster,

    /// Assigned board region. None if region assignment was skipped or
    /// this cluster contains only user-constrained components.
    pub region: Option<BoardRegion>,

    /// Whether this cluster contains any connector components.
    pub has_connector: bool,
}
```

### 1.6 Hierarchical Cluster Tree

```rust
/// Hierarchical grouping produced by `classify_domains()`.
pub enum ClusterNode {
    /// A named domain group that can be further subdivided.
    Branch {
        domain: Domain,
        children: Vec<ClusterNode>,
    },
    /// A leaf containing a flat list of component designators.
    Leaf {
        components: Vec<String>,
        domain: Domain,
    },
}
```

### 1.7 Final Group

```rust
/// The resolved group after merging auto-clusters with user constraints.
/// Each component appears in exactly one FinalGroup.
#[derive(Debug, Clone)]
pub struct FinalGroup {
    /// Unique name for the group, e.g. "auto_digital_0", "user_J1_J2".
    pub name: String,

    /// Designators in the group.
    pub members: Vec<String>,

    /// Whether the group was created or modified by a user constraint.
    pub user_defined: bool,

    /// Dominant domain.
    pub domain: Domain,
}
```

### 1.8 Clustering Metadata

```rust
#[derive(Debug, Clone)]
pub struct ClusteringMetadata {
    /// Total component count in the IR.
    pub component_count: usize,

    /// Total net count in the IR.
    pub net_count: usize,

    /// Total edge count in the weighted component graph.
    pub graph_edge_count: usize,

    /// Algorithm that was used: "bfs" or "spectral".
    pub algorithm_used: String,

    /// Edge weight threshold used by BFS clustering.
    pub bfs_threshold: f64,

    /// Number of power-iteration rounds until convergence (spectral only).
    pub spectral_iterations: Option<usize>,

    /// Whether spectral failed to converge and BFS fallback was used.
    pub spectral_fallback: bool,

    /// Wall-clock time for Phase 0 in milliseconds.
    pub duration_ms: u128,
}
```

### 1.9 Phase 0 Output

```rust
/// Complete output of Phase 0, ready for consumption by Phase 1.
#[derive(Debug, Clone)]
pub struct Phase0Output {
    /// Raw auto-detected clusters (one per graph partition).
    pub auto_clusters: Vec<AutoCluster>,

    /// Final groups after merging user constraints with auto-clusters.
    /// Every component from `PcbIr::components` appears in exactly one group.
    pub final_groups: Vec<FinalGroup>,

    /// Cluster-name → board region assignment.
    /// Key is `FinalGroup::name`.
    pub region_assignments: HashMap<String, BoardRegion>,

    /// Concrete `UserConstraint` values ready to pass to `solve_placement()`.
    /// These are derived from auto-clusters (Near, RegionContainment) plus
    /// connector edge placements. They are APPENDED to any caller-supplied
    /// user constraints, never replacing them.
    pub suggested_constraints: Vec<UserConstraint>,

    /// Diagnostic information about the run.
    pub metadata: ClusteringMetadata,
}
```

---

## 2. Graph Construction

**Function signature:**

```rust
pub fn build_component_graph(ir: &PcbIr) -> ComponentGraph
```

where `ComponentGraph` is:

```rust
pub struct ComponentGraph {
    pub graph: petgraph::Graph<ComponentNode, f64>,

    /// Mapping from designator string to petgraph NodeIndex.
    pub node_index: HashMap<String, petgraph::graph::NodeIndex>,
}
```

### 2.1 Algorithm

The component graph is a weighted undirected graph. Each component is a node;
two components share a weighted edge if they have one or more nets in common.

**Edge weight = number of shared nets between two components.**

This is the simplest and most interpretable metric. It is equivalent to the
pin-connectivity count when each net is counted once (not multiplied by pad
count). For MVP this is sufficient; see Section 10 for alternatives.

**Construction is O(N + E)** where N = component count and E = edge count:

```
1. For each net in ir.nets:
   a. Collect all (component_id, pad_id) pairs from net.pins.
   b. For each unique pair of component_ids (i, j) with i < j:
      - If edge (i, j) exists: increment weight by 1.
      - Else: insert edge (i, j) with weight 1.
```

This is O(nets × pins_per_net²) in the worst case, but nets with more than
~20 pins are rare on PCB-scale designs. Power nets (GND, VCC) that connect
to many components must be handled specially:

**Power net exclusion:** If a net has `pin_count > N/4` (connects to more than
25% of components), treat it as a power net and exclude it from edge-weight
computation. Power nets would otherwise dominate all edge weights and cause
every component to cluster together. The threshold `N/4` is a heuristic;
record it in `ClusteringMetadata`.

**Pseudocode:**

```
function build_component_graph(ir):
    graph = new petgraph::Graph<ComponentNode, f64>(Undirected)
    node_index = HashMap::new()

    for (id, comp) in ir.components.iter():
        node = ComponentNode {
            designator: comp.designator,
            pattern: comp.pattern,
            half_w: comp.local_bounds.width() / 2.0,
            half_h: comp.local_bounds.height() / 2.0,
            pin_count: comp.pads.len(),
            is_connector: classify_connector(comp),
            domain: Domain::Unknown,   // filled by classify_domains()
        }
        idx = graph.add_node(node)
        node_index.insert(comp.designator.clone(), idx)

    power_net_threshold = ir.components.len() / 4

    for (_, net) in ir.nets.iter():
        if net.pins.len() > power_net_threshold:
            continue   // skip power/ground planes

        component_ids: Vec<&str> = net.pins
            .iter()
            .map(|p| ir.components[p.component].designator.as_str())
            .collect()
        component_ids.dedup()   // deduplicate in case multiple pads on same net/comp

        for i in 0..component_ids.len():
            for j in (i+1)..component_ids.len():
                a = node_index[component_ids[i]]
                b = node_index[component_ids[j]]
                if let Some(edge) = graph.find_edge(a, b):
                    *graph.edge_weight_mut(edge) += 1.0
                else:
                    graph.add_edge(a, b, 1.0)

    // Tag domain for each node
    domains = classify_domains_inner(ir, &graph)
    for (designator, domain) in domains:
        let idx = node_index[&designator]
        graph[idx].domain = domain

    ComponentGraph { graph, node_index }
```

### 2.2 Connector Classification

```rust
fn classify_connector(comp: &IrComponent) -> bool {
    let des = comp.designator.to_ascii_uppercase();
    let pat = comp.pattern.to_ascii_uppercase();

    // Designator prefix heuristic
    let connector_prefixes = ["J", "P", "CN", "X"];
    if connector_prefixes.iter().any(|p| des.starts_with(p)) {
        return true;
    }

    // Pattern keyword heuristic
    let connector_keywords = ["USB", "CONN", "HDR", "DF", "GCT", "DSUB",
                               "MOLEX", "JST", "MICRO-B", "TYPE-C"];
    if connector_keywords.iter().any(|k| pat.contains(k)) {
        return true;
    }

    false
}
```

---

## 3. Domain Classification

**Function signature:**

```rust
pub fn classify_domains(ir: &PcbIr) -> HashMap<String, Domain>
```

Returns a map of designator → `Domain`.

### 3.1 Net-Name Heuristics

Scan the net name for keywords using case-insensitive substring matching:

| Net name pattern | Domain |
|-----------------|--------|
| Contains "GND", "AGND", "DGND", "PGND" | Power |
| Contains "VCC", "VDD", "V3V3", "V5V", "V12", "VBAT", "PWR", "POWER", "VIN", "VOUT", "VREG" | Power |
| Contains "AIN", "AOUT", "ANALOG", "VREF", "AVDD", "AVCC", "AGND" | Analog |
| Contains "SPI", "I2C", "UART", "GPIO", "CLK", "DATA", "MOSI", "MISO", "SCL", "SDA", "TX", "RX" | Digital |
| Contains "USB_D", "HSYNC", "VSYNC", "DDR", "ETH" | Digital |

A component's domain is the majority vote across all its connected net domains.
If no nets match, the domain is `Unknown`.

### 3.2 Component-Type Heuristics

Applied AFTER net-name classification to resolve `Unknown` components or
break ties:

| Designator prefix | Domain override |
|------------------|----------------|
| U (IC) — with pattern containing "LDO", "VREG", "REG", "DCDC", "PWM_CTRL" | Power |
| U (IC) — with pattern containing "AMP", "OPAMP", "ADC", "DAC", "MUX_A", "FILTER" | Analog |
| U (IC) — with pattern containing "MCU", "STM32", "ESP", "FPGA", "PIC", "AVR", "ARM", "ZYNQ" | Digital |
| L (inductor) — connected to power net | Power |
| C (capacitor) > 10µF (from pattern, if parseable) — connected to power net | Power |
| C (capacitor) — connected to analog net | Analog |
| R, C (small, unlabeled) — majority of connected components | inherit majority |

**Cascade priority:**
1. Connector classification (`is_connector = true`) → no domain change (connectors stay `Unknown` for domain purposes and are handled separately)
2. Component-type override (explicit pattern match)
3. Net-name majority vote
4. Inherit majority domain of neighboring nodes in the component graph (BFS depth 1)
5. If still `Unknown`, leave as `Unknown`

### 3.3 Algorithm

```
function classify_domains_inner(ir, graph):
    comp_domain = HashMap::new()   // designator → Domain

    // Pass 1: net-name classification
    for (_, comp) in ir.components.iter():
        domain_votes = {Power:0, Analog:0, Digital:0, Unknown:0}
        for pad in comp.pads:
            if let Some(net_id) = pad.net:
                net_name = ir.nets[net_id].name.to_uppercase()
                d = classify_net_name(net_name)
                domain_votes[d] += 1
        comp_domain[comp.designator] = majority_vote(domain_votes)

    // Pass 2: component-type override
    for (_, comp) in ir.components.iter():
        if let Some(d) = component_type_override(comp):
            comp_domain[comp.designator] = d

    // Pass 3: propagate via BFS for remaining Unknown
    for (_, comp) in ir.components.iter():
        if comp_domain[comp.designator] == Domain::Unknown:
            neighbor_domains: Vec<Domain> = graph.neighbors(node_index[comp.designator])
                .map(|n| comp_domain[graph[n].designator])
                .filter(|d| *d != Domain::Unknown)
                .collect()
            if !neighbor_domains.is_empty():
                comp_domain[comp.designator] = majority_vote_list(neighbor_domains)

    comp_domain
```

---

## 4. Clustering Algorithms

### 4.1 Algorithm Selection

```rust
pub fn select_algorithm(component_count: usize) -> ClusteringAlgorithm {
    if component_count < 100 {
        ClusteringAlgorithm::Bfs
    } else {
        ClusteringAlgorithm::Spectral
    }
}

pub enum ClusteringAlgorithm { Bfs, Spectral }
```

**Rule:** Always attempt BFS as the fallback if spectral fails to converge.

### 4.2 BFS High-Weight Edge Clustering

**Function signature:**

```rust
pub fn cluster_bfs(graph: &ComponentGraph, threshold: f64) -> Vec<Cluster>
```

**Threshold auto-computation:** `threshold = median edge weight across all edges`.

The median is computed by collecting all edge weights into a `Vec<f64>`,
sorting, and taking the middle element. For an even count, take the lower
median (index `n/2 - 1`). A threshold of 0 is invalid; in that case use 1.0.

**Pseudocode:**

```
function cluster_bfs(graph, threshold):
    visited = HashSet::new()
    clusters = Vec::new()
    cluster_id = 0

    for each node n in graph (in NodeIndex order):
        if n in visited:
            continue

        // Start a new cluster from n
        members = Vec::new()
        queue = VecDeque::new()
        queue.push_back(n)
        visited.insert(n)

        while queue is not empty:
            current = queue.pop_front()
            members.push(graph[current].designator.clone())

            for each neighbor m of current:
                if m in visited:
                    continue
                edge_weight = graph.edge_weight(current, m)
                if edge_weight >= threshold:
                    visited.insert(m)
                    queue.push_back(m)

        cluster = build_cluster(cluster_id, members, graph)
        clusters.push(cluster)
        cluster_id += 1

    clusters
```

**Complexity:** O(N + E) — each node and edge is visited at most once.

**Determinism:** The traversal order depends on petgraph's `NodeIndex` ordering,
which is insertion order. Since `build_component_graph` inserts nodes in the
order `ir.components.iter()` returns them (stable across runs for the same IR),
this is deterministic.

**`build_cluster` helper:**

```
function build_cluster(id, members, graph):
    domain_votes = {Power:0, Analog:0, Digital:0, Unknown:0}
    total_area = 0.0
    internal_cohesion = 0.0

    for m in members:
        node = graph[node_index[m]]
        domain_votes[node.domain] += 1
        total_area += 4.0 * node.half_w * node.half_h

    for each edge (a, b, w) internal to members:
        internal_cohesion += w

    Cluster {
        id,
        members,
        domain: majority_vote(domain_votes),
        total_area_mm2: total_area,
        internal_cohesion,
    }
```

### 4.3 Spectral Bisection (Fiedler Vector)

**Function signature:**

```rust
pub fn cluster_spectral(graph: &ComponentGraph, min_size: usize) -> Vec<Cluster>
```

Recursively bisects the component graph using the Fiedler vector (second
smallest eigenvector of the graph Laplacian). Stops when a partition reaches
`min_size` components or the Fiedler value is approximately zero (indicating
no meaningful cut exists).

**This is implemented in pure Rust with no matrix library dependencies.** The
Laplacian eigenvector is computed via power iteration on the shifted inverse
Laplacian.

#### 4.3.1 Laplacian Matrix

For a weighted graph with N nodes and edge weights `w(i,j)`:

```
Degree matrix D:  D[i][i] = Σ_j w(i,j)   (sum of all edge weights for node i)
                  D[i][j] = 0              (off-diagonal)

Laplacian L = D - W
  L[i][i] = D[i][i]
  L[i][j] = -w(i,j)  for i ≠ j
  L[i][j] = 0         if (i,j) is not an edge
```

The Laplacian is stored as a `Vec<Vec<f64>>` dense matrix (N×N). At PCB scale
(N ≤ 500), this costs 500² × 8 = 2 MB, which is acceptable. For N > 300 a
sparse representation (CSR format as `Vec<(usize, f64)>` per row) is preferred.

**Threshold for sparse vs dense:** If `N > 300`, use sparse row format.

#### 4.3.2 Power Iteration for Fiedler Vector

The Fiedler vector is the eigenvector corresponding to the second smallest
eigenvalue of L. Direct computation via power iteration on L itself finds the
largest eigenvalue, not the smallest. Instead, use the **deflated shifted
inverse** approach:

**Step 1: Shift.** Form `M = λ_max · I - L` where `λ_max` is the largest
diagonal entry of D (an upper bound on the largest eigenvalue). The largest
eigenvalue of M corresponds to the smallest eigenvalue of L.

**Step 2: Deflate.** To find the SECOND smallest eigenvalue of L (i.e., skip
the all-ones zero eigenvector), project out the constant vector at each
iteration:

```
v = v - (v · ones / N) · ones
```

This ensures the iteration converges to the Fiedler vector rather than the
trivial zero-eigenvalue vector.

**Pseudocode:**

```
function compute_fiedler_vector(L: &[[f64]], max_iters: usize, tol: f64)
    -> (Vec<f64>, f64, usize):  // (vector, eigenvalue, iters)

    N = L.len()
    assert N >= 2

    // Estimate λ_max as max diagonal entry of L
    lambda_max = L.iter().enumerate().map(|(i, row)| row[i]).fold(0.0, f64::max)
    // Small epsilon to avoid division near zero
    lambda_max = lambda_max.max(1e-10)

    // Shift matrix M = λ_max·I - L  (formed implicitly)
    // Mv = λ_max·v - L·v

    // Initialize random vector
    v = vec![1.0; N]
    for i in 0..N:
        v[i] = (i as f64 * 1.6180339887) % 1.0 - 0.5   // quasi-random, deterministic

    // Deflate: remove component along all-ones vector
    project_out_ones(&mut v)
    normalize(&mut v)

    prev_eigenvalue = 0.0

    for iter in 0..max_iters:
        // Matrix-vector product: w = (λ_max·I - L)·v = λ_max·v - L·v
        Lv = matvec(L, &v)
        w = Vec::with_capacity(N)
        for i in 0..N:
            w.push(lambda_max * v[i] - Lv[i])

        // Deflate: remove all-ones component
        project_out_ones(&mut w)

        // Rayleigh quotient to estimate eigenvalue of M
        // (which is λ_max - fiedler_value for L)
        eigenvalue_M = dot(&w, &v) / dot(&v, &v)
        fiedler_value = lambda_max - eigenvalue_M

        normalize(&mut w)

        // Convergence check
        if (eigenvalue_M - prev_eigenvalue).abs() < tol:
            return (w, fiedler_value, iter + 1)

        prev_eigenvalue = eigenvalue_M
        v = w

    // Return best estimate (did not converge)
    (v, lambda_max - prev_eigenvalue, max_iters)

function project_out_ones(v: &mut Vec<f64>):
    mean = v.iter().sum::<f64>() / v.len() as f64
    for x in v.iter_mut():
        *x -= mean

function normalize(v: &mut Vec<f64>):
    norm = v.iter().map(|x| x * x).sum::<f64>().sqrt()
    if norm > 1e-15:
        for x in v.iter_mut():
            *x /= norm

function matvec(L: &[[f64]], v: &[f64]) -> Vec<f64>:
    N = L.len()
    result = vec![0.0; N]
    for i in 0..N:
        for j in 0..N:
            result[i] += L[i][j] * v[j]
    result

function dot(a: &[f64], b: &[f64]) -> f64:
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
```

**Parameters:**
- `max_iters = 100` (sufficient for PCB-scale Laplacians)
- `tol = 1e-6`

#### 4.3.3 Bisection and Recursion

```
function cluster_spectral(graph, min_size) -> Vec<Cluster>:
    all_nodes = all NodeIndices in graph
    return spectral_bisect_recursive(graph, all_nodes, min_size, &mut counter=0)

function spectral_bisect_recursive(graph, nodes, min_size, counter) -> Vec<Cluster>:
    N = nodes.len()

    // Base cases
    if N <= min_size:
        return [build_cluster(*counter++, designators(nodes), graph)]

    // Build subgraph Laplacian for `nodes`
    (L, index_map) = build_subgraph_laplacian(graph, nodes)

    // Compute Fiedler vector
    (fiedler_vec, fiedler_value, iters) = compute_fiedler_vector(&L, 100, 1e-6)

    // Stop recursion if graph is nearly disconnected (Fiedler ≈ 0)
    if fiedler_value < 1e-6:
        return [build_cluster(*counter++, designators(nodes), graph)]

    // Partition by sign of Fiedler vector (median split)
    median = median_of(fiedler_vec)
    left_nodes  = [nodes[i] for i if fiedler_vec[i] <= median]
    right_nodes = [nodes[i] for i if fiedler_vec[i] >  median]

    // Stop recursion if partition is trivially unbalanced
    // (one side has 0 nodes — can happen with isolated nodes)
    if left_nodes.is_empty() || right_nodes.is_empty():
        return [build_cluster(*counter++, designators(nodes), graph)]

    // Count edge-cut between partitions
    edge_cut = count_edges_between(graph, left_nodes, right_nodes)
    if edge_cut < 3:   // tiny cut: not worth splitting further
        return [build_cluster(*counter++, designators(nodes), graph)]

    // Recurse
    left_clusters  = spectral_bisect_recursive(graph, left_nodes,  min_size, counter)
    right_clusters = spectral_bisect_recursive(graph, right_nodes, min_size, counter)

    left_clusters + right_clusters

function build_subgraph_laplacian(graph, nodes) -> ([[f64]], HashMap<NodeIndex, usize>):
    N = nodes.len()
    L = vec![vec![0.0; N]; N]
    index_map = HashMap::new()
    for (i, node) in nodes.iter().enumerate():
        index_map[*node] = i

    for (i, node_a) in nodes.iter().enumerate():
        for (j, node_b) in nodes.iter().enumerate():
            if i == j:
                continue
            if let Some(edge) = graph.find_edge(*node_a, *node_b):
                w = *graph.edge_weight(edge)
                L[i][i] += w
                L[i][j] -= w

    (L, index_map)
```

**Stopping criteria summary:**

| Condition | Stop? | Reason |
|-----------|-------|--------|
| `N <= min_size` | Yes | Partition small enough |
| `fiedler_value < 1e-6` | Yes | Graph is approximately disconnected; no natural cut |
| `edge_cut < 3` | Yes | Less than 3 nets cross the partition; not meaningful |
| One partition is empty | Yes | Degenerate: isolated node or numerical artifact |

**Convergence fallback to BFS:**

```rust
fn cluster_spectral_with_fallback(
    graph: &ComponentGraph,
    min_size: usize,
) -> (Vec<Cluster>, bool /* used_fallback */) {
    // Spectral bisection is tried first. If it produces a single cluster
    // containing all components (failed to bisect at the top level),
    // that indicates convergence failure — fall back to BFS.
    let clusters = cluster_spectral(graph, min_size);
    if clusters.len() == 1 && clusters[0].members.len() == graph.graph.node_count() {
        let threshold = compute_bfs_threshold(graph);
        (cluster_bfs(graph, threshold), true)
    } else {
        (clusters, false)
    }
}
```

---

## 5. Hierarchical Domain Grouping

**Function signature:**

```rust
pub fn build_domain_hierarchy(
    clusters: &[Cluster],
    graph: &ComponentGraph,
) -> Vec<ClusterNode>
```

Returns a two-level hierarchy:

```
Level 1 (coarse): one Branch per Domain that has any clusters
Level 2 (fine):   Leaf nodes within each domain, one per cluster
```

**Algorithm:**

```
function build_domain_hierarchy(clusters, graph) -> Vec<ClusterNode>:
    by_domain: HashMap<Domain, Vec<&Cluster>> = group_by(clusters, |c| c.domain)
    result = Vec::new()

    for domain in [Power, Analog, Digital, Unknown]:
        if let Some(domain_clusters) = by_domain.get(domain):
            children = domain_clusters
                .iter()
                .map(|c| ClusterNode::Leaf {
                    components: c.members.clone(),
                    domain,
                })
                .collect()
            result.push(ClusterNode::Branch { domain, children })

    result
```

A flattened view of the hierarchy is used for region assignment and constraint
generation. The tree structure is preserved in the output for debugging and
future use (e.g., for hierarchical constraint generation in Phase 3).

---

## 6. Board Region Assignment

**Function signature:**

```rust
pub fn assign_regions(
    clusters: &[Cluster],
    board: &BoardGeometry,
) -> HashMap<String, BoardRegion>
```

Maps cluster name (`"cluster_<id>"`) → `BoardRegion`.

### 6.1 Assignment Rules

**Rule 1: Connectors → board edges.**

For each cluster with `has_connector = true` OR for each cluster whose sole
member is a connector component, assign to the nearest board edge.

If the cluster contains a mix of connectors and non-connectors, assign to the
board edge on the side with the most connectors (by count, ties broken by
alphabetical order of designator).

Edge selection uses the cluster's centroid in the current IR (if available)
vs. the board center to determine which edge is nearest. In the absence of
current positions, round-robin across {Top, Bottom, Left, Right} in that order.

**Rule 2: Power clusters → near the power-input connector.**

The power-input connector is heuristically identified as the cluster with the
most `VIN`, `VCC`, or `VBAT` labeled nets AND `is_connector = true`. If no
such connector is found, place the power cluster in the `BottomLeftQuadrant`.

**Rule 3: Remaining clusters → balance across board quadrants.**

Assign clusters to quadrants using round-robin, ordered by cluster area
(largest first). The intent is to distribute component density evenly.

**Quadrant assignment order:** TopLeft, TopRight, BottomLeft, BottomRight,
cycling back to TopLeft for the 5th cluster, etc.

**Algorithm:**

```
function assign_regions(clusters, board) -> HashMap<String, BoardRegion>:
    assignments = HashMap::new()
    connector_clusters = clusters filtered by has_connector
    power_clusters = clusters filtered by domain == Power
    other_clusters = remaining

    edge_queue = [TopEdge, BottomEdge, LeftEdge, RightEdge]  // round-robin index

    for c in connector_clusters:
        edge = edge_queue.next_round_robin()
        assignments[cluster_key(c)] = board_edge_region(board, edge)

    power_target = board_quadrant_region(board, BottomLeftQuadrant)
    for c in power_clusters:
        assignments[cluster_key(c)] = power_target

    quadrant_queue = [TopLeftQuadrant, TopRightQuadrant,
                      BottomLeftQuadrant, BottomRightQuadrant]
    for c in other_clusters.sorted_by_area_descending():
        q = quadrant_queue.next_round_robin()
        assignments[cluster_key(c)] = board_quadrant_region(board, q)

    assignments

function board_edge_region(board, edge) -> BoardRegion:
    // Returns a thin rectangular strip along the specified edge.
    // Strip depth = 20% of the perpendicular board dimension.
    bounds = board.bounds
    w = bounds.max.x - bounds.min.x
    h = bounds.max.y - bounds.min.y
    match edge:
        TopEdge    => RectRegion { min_x: bounds.min.x, max_x: bounds.max.x,
                                    min_y: bounds.max.y - h*0.2, max_y: bounds.max.y }
        BottomEdge => RectRegion { min_x: bounds.min.x, max_x: bounds.max.x,
                                    min_y: bounds.min.y, max_y: bounds.min.y + h*0.2 }
        LeftEdge   => RectRegion { min_x: bounds.min.x, max_x: bounds.min.x + w*0.2,
                                    min_y: bounds.min.y, max_y: bounds.max.y }
        RightEdge  => RectRegion { min_x: bounds.max.x - w*0.2, max_x: bounds.max.x,
                                    min_y: bounds.min.y, max_y: bounds.max.y }

function board_quadrant_region(board, quadrant) -> BoardRegion:
    // Returns one of the four board quadrants (or center).
    // Delegates to named_region_from_board() from lib.rs.
    let name = match quadrant { ... };
    named_region_from_board_bounds(board.bounds, name)
```

### 6.2 Overlap Between Power and Connector Regions

If a power cluster is assigned to `BottomLeftQuadrant` but a connector cluster
is also assigned to `BottomEdge`, the regions overlap. This is intentional —
the solver (Phase 1) will resolve the spatial conflict via clearance constraints.
Region assignments are hints, not hard exclusion zones.

---

## 7. User Group Merging

**Function signature:**

```rust
pub fn merge_with_user_groups(
    auto: Vec<Cluster>,
    user: &[UserConstraint],
) -> Vec<FinalGroup>
```

### 7.1 Merging Algorithm

User-specified constraints take absolute priority. A component that appears in
any `UserConstraint` is removed from auto-clusters before generating
`suggested_constraints`. Auto-clusters that become empty after removal are
discarded.

```
function merge_with_user_groups(auto, user) -> Vec<FinalGroup>:
    user_component_set = HashSet::new()

    // Collect all designators that appear in any user constraint
    for uc in user:
        match uc:
            UserConstraint::EdgePlacement { designator, .. }    => user_component_set.insert(designator)
            UserConstraint::Directional { a, b, .. }            => { user_component_set.insert(a); user_component_set.insert(b) }
            UserConstraint::Near { a, b, .. }                   => { user_component_set.insert(a); user_component_set.insert(b) }
            UserConstraint::RegionContainment { designator, .. }=> user_component_set.insert(designator)
            UserConstraint::FixedPosition { designator, .. }    => user_component_set.insert(designator)

    final_groups = Vec::new()

    // Auto clusters: remove user-constrained components
    for (i, cluster) in auto.iter().enumerate():
        remaining = cluster.members.iter()
            .filter(|d| !user_component_set.contains(*d))
            .cloned()
            .collect::<Vec<_>>()
        if remaining.is_empty():
            continue
        final_groups.push(FinalGroup {
            name: format!("auto_{}_{}", cluster.domain_str(), i),
            members: remaining,
            user_defined: false,
            domain: cluster.domain,
        })

    // Add a synthetic group for user-constrained components (for bookkeeping)
    if !user_component_set.is_empty():
        final_groups.push(FinalGroup {
            name: "user_constrained".to_string(),
            members: user_component_set.into_iter().sorted().collect(),
            user_defined: true,
            domain: Domain::Unknown,
        })

    final_groups
```

**Invariant:** After `merge_with_user_groups`, every component in the IR
appears in exactly one `FinalGroup`. This must be verified in debug builds via
`debug_assert`.

---

## 8. Constraint Generation from Clusters

**Internal function (called by `phase0_preprocess`):**

```rust
fn generate_suggested_constraints(
    final_groups: &[FinalGroup],
    region_assignments: &HashMap<String, BoardRegion>,
    graph: &ComponentGraph,
    board: &BoardGeometry,
) -> Vec<UserConstraint>
```

### 8.1 NearConstraint for intra-cluster components

For each `FinalGroup` with 2 or more members, generate pairwise `NearConstraint`
values for members whose shared edge weight exceeds the BFS threshold.

**max_spread formula:** `max_spread_mm = sqrt(group.total_area_mm2) * 2.5`

This means members of a cluster can be at most 2.5× the cluster's "diameter"
apart (a loose constraint that guides the solver without being overly restrictive).

Only generate `NearConstraint` for pairs with direct graph edges above the
median threshold (to avoid N² constraints for large groups):

```
for each pair (a, b) in group.members where edge_weight(a,b) >= threshold:
    constraints.push(UserConstraint::Near {
        a: a.clone(),
        b: b.clone(),
        max_distance_mm: max_spread_mm,
    })
```

**Cap:** Generate at most `min(group.members.len() * 3, 20)` Near constraints
per group to avoid solver overload. Sort pairs by edge weight descending and
take the top N.

### 8.2 RegionContainment for region-assigned groups

For each `FinalGroup` that has an entry in `region_assignments`, generate a
`RegionContainment` constraint for each member:

```
if let Some(region) = region_assignments.get(&group.name):
    for member in group.members:
        constraints.push(UserConstraint::RegionContainment {
            designator: member.clone(),
            region: region.rect.clone(),
        })
```

### 8.3 EdgePlacement for connector clusters

For connector components (groups where `has_connector = true` and the
assigned region is an edge strip):

```
for cluster in auto_clusters where has_connector:
    edge = region_to_edge(region_assignments[cluster.name])
    for member in cluster.members where is_connector:
        constraints.push(UserConstraint::EdgePlacement {
            designator: member.clone(),
            edge,
            inset_mm: 1.0,   // 1mm default inset from board edge
        })
```

**Note:** `EdgePlacement` is generated only for `is_connector = true`
components within connector clusters. Non-connector components in the same
cluster get `RegionContainment` for the edge region instead.

---

## 9. Top-Level Entry Point

**Function signature:**

```rust
pub fn phase0_preprocess(
    ir: &PcbIr,
    user: &[UserConstraint],
) -> Result<Phase0Output, Phase0Error>
```

```rust
#[derive(Debug, thiserror::Error)]
pub enum Phase0Error {
    #[error("component graph is empty — IR has no components")]
    NoComponents,
    #[error("board geometry has zero area")]
    InvalidBoardBounds,
}
```

**Complete algorithm:**

```
function phase0_preprocess(ir, user) -> Result<Phase0Output, Phase0Error>:
    if ir.components.is_empty():
        return Err(Phase0Error::NoComponents)

    bounds = ir.board.bounds
    if bounds.max.x <= bounds.min.x || bounds.max.y <= bounds.min.y:
        return Err(Phase0Error::InvalidBoardBounds)

    t_start = Instant::now()

    // Step 1: Build component graph (includes domain classification)
    graph = build_component_graph(ir)

    // Step 2: Select and run clustering algorithm
    N = ir.components.len()
    algo = select_algorithm(N)
    min_size = if N < 20 { 2 } else { 5 }

    (clusters, spectral_fallback, spectral_iters) = match algo:
        ClusteringAlgorithm::Bfs => {
            threshold = compute_bfs_threshold(&graph)
            (cluster_bfs(&graph, threshold), false, None)
        }
        ClusteringAlgorithm::Spectral => {
            (result, fallback) = cluster_spectral_with_fallback(&graph, min_size)
            (result, fallback, Some(last_iter_count))
        }

    // Step 3: Tag connector flag on clusters
    auto_clusters: Vec<AutoCluster> = clusters.iter().map(|c| {
        has_connector = c.members.iter()
            .any(|d| graph.graph[graph.node_index[d]].is_connector)
        AutoCluster { cluster: c.clone(), region: None, has_connector }
    }).collect()

    // Step 4: Assign board regions
    raw_region_assignments = assign_regions(
        &clusters,
        &ir.board,
    )

    // Attach region back to AutoCluster
    for ac in &mut auto_clusters:
        if let Some(r) = raw_region_assignments.get(&cluster_key(&ac.cluster)):
            ac.region = Some(r.clone())

    // Step 5: Merge with user constraints
    final_groups = merge_with_user_groups(clusters.clone(), user)

    // Step 6: Build region_assignments keyed by FinalGroup name
    region_assignments: HashMap<String, BoardRegion> = HashMap::new()
    for group in &final_groups:
        for (i, cluster) in clusters.iter().enumerate():
            if cluster.members.iter().any(|m| group.members.contains(m)):
                if let Some(r) = raw_region_assignments.get(&cluster_key(cluster)):
                    region_assignments.insert(group.name.clone(), r.clone())
                    break

    // Step 7: Generate suggested constraints
    bfs_threshold = compute_bfs_threshold(&graph)
    suggested_constraints = generate_suggested_constraints(
        &final_groups,
        &region_assignments,
        &graph,
        &ir.board,
    )

    duration_ms = t_start.elapsed().as_millis()

    // Step 8: Compute metadata
    metadata = ClusteringMetadata {
        component_count: N,
        net_count: ir.nets.len(),
        graph_edge_count: graph.graph.edge_count(),
        algorithm_used: if spectral_fallback { "bfs_fallback".into() }
                        else { format!("{:?}", algo).to_lowercase() },
        bfs_threshold,
        spectral_iterations: spectral_iters,
        spectral_fallback,
        duration_ms,
    }

    Ok(Phase0Output {
        auto_clusters,
        final_groups,
        region_assignments,
        suggested_constraints,
        metadata,
    })
```

---

## 10. Edge Weight Metrics

Three edge weight formulations are defined below. Only **Metric 1 (pin
connectivity count)** is implemented in the MVP.

### 10.1 Pin Connectivity Count (MVP, recommended)

`w(i, j) = |{nets shared between component i and component j}|`

Each shared net contributes 1 to the edge weight regardless of how many pads
each component has on that net.

**Pros:** Simple, deterministic, intuitive. Two components that share 3 nets
have w=3, correctly indicating they are more strongly related than two
components that share 1 net.

**Cons:** Treats a 2-pin resistor net and a 32-pin data bus net equally.

### 10.2 Normalized Pin Overlap

`w(i, j) = |{shared nets}| / sqrt(pin_count_i × pin_count_j)`

Divides by the geometric mean of pin counts to normalize for component size.

**Pros:** A connector sharing 1 net with an MCU that has 100 pins gets a much
lower weight than a small IC sharing 1 net with another small IC.

**Cons:** Power connectors (few pins, many shared power nets) may be
incorrectly clustered with power ICs.

### 10.3 Weighted Hypergraph Reduction (advanced)

For each shared net n with pin count p(n):

`w(i, j) += 1 / (p(n) - 1)`

This is the standard hypergraph-to-graph reduction used in placement
literature. A net with 2 pins contributes weight 1.0; a net with 10 pins
contributes weight 0.11.

**Pros:** High-fanout nets (power rails, clock distributions) contribute little
weight, naturally de-emphasizing them without explicit exclusion.

**Cons:** Slightly more complex; the threshold computation for BFS changes
character (weights are no longer integers).

**Selection guidance:**

| Scenario | Recommended Metric |
|----------|--------------------|
| N < 100, simple boards | Metric 1 (MVP) |
| N >= 100, boards with large high-fanout buses | Metric 3 |
| Boards with large size-variation between components | Metric 2 |

---

## 11. Module Structure

```
crates/autopcb-placement/src/
├── lib.rs                     (existing Phase 1 solver, solve_placement())
├── simulated_annealing.rs     (Phase 2/3, separate spec)
└── clustering.rs              (Phase 0, this spec)
    │
    ├── pub struct ComponentNode
    ├── pub enum Domain
    ├── pub struct BoardRegion
    ├── pub enum CanonicalRegion
    ├── pub struct Cluster
    ├── pub struct AutoCluster
    ├── pub enum ClusterNode
    ├── pub struct FinalGroup
    ├── pub struct ClusteringMetadata
    ├── pub struct Phase0Output
    ├── pub struct ComponentGraph
    ├── pub enum ClusteringAlgorithm
    ├── pub enum Phase0Error
    │
    ├── pub fn build_component_graph(ir: &PcbIr) -> ComponentGraph
    ├── pub fn classify_domains(ir: &PcbIr) -> HashMap<String, Domain>
    ├── pub fn cluster_bfs(graph: &ComponentGraph, threshold: f64) -> Vec<Cluster>
    ├── pub fn cluster_spectral(graph: &ComponentGraph, min_size: usize) -> Vec<Cluster>
    ├── pub fn build_domain_hierarchy(clusters: &[Cluster], graph: &ComponentGraph) -> Vec<ClusterNode>
    ├── pub fn assign_regions(clusters: &[Cluster], board: &BoardGeometry) -> HashMap<String, BoardRegion>
    ├── pub fn merge_with_user_groups(auto: Vec<Cluster>, user: &[UserConstraint]) -> Vec<FinalGroup>
    ├── pub fn phase0_preprocess(ir: &PcbIr, user: &[UserConstraint]) -> Result<Phase0Output, Phase0Error>
    │
    ├── fn classify_connector(comp: &IrComponent) -> bool
    ├── fn classify_domains_inner(ir: &PcbIr, graph: &ComponentGraph) -> HashMap<String, Domain>
    ├── fn compute_bfs_threshold(graph: &ComponentGraph) -> f64
    ├── fn compute_fiedler_vector(L: &[Vec<f64>], max_iters: usize, tol: f64) -> (Vec<f64>, f64, usize)
    ├── fn build_subgraph_laplacian(graph: &ComponentGraph, nodes: &[NodeIndex]) -> (Vec<Vec<f64>>, HashMap<NodeIndex, usize>)
    ├── fn spectral_bisect_recursive(graph: &ComponentGraph, nodes: &[NodeIndex], min_size: usize, counter: &mut usize) -> Vec<Cluster>
    ├── fn cluster_spectral_with_fallback(graph: &ComponentGraph, min_size: usize) -> (Vec<Cluster>, bool)
    ├── fn generate_suggested_constraints(...) -> Vec<UserConstraint>
    ├── fn build_cluster(id: usize, members: Vec<String>, graph: &ComponentGraph) -> Cluster
    └── fn board_edge_region(board: &BoardGeometry, edge: CanonicalRegion) -> BoardRegion
```

**Cargo.toml addition** (in `[dependencies]` of `autopcb-placement/Cargo.toml`):

```toml
petgraph = "0.8"
```

`petgraph` is already listed as a Phase 1+ dependency in the technology stack
(`implementation-plan.md`). No other new dependencies are required for the MVP.
The power iteration code is ~100 lines of pure Rust with no external math deps.

**Optional future addition** (not in MVP):

```toml
ndarray = "0.16"
ndarray-linalg = { version = "0.16", features = ["openblas"] }
```

For optimized spectral clustering when N > 300. Not required for PCB-scale
problems.

---

## 12. Integration with `solve_placement()`

The caller integrates Phase 0 by prepending `suggested_constraints` to any
existing user constraints before calling `solve_placement()`:

```rust
// Typical call site in CLI or shell:
let phase0 = phase0_preprocess(&ir, &user_constraints)?;

// Merge: user constraints have priority (they are applied first in lib.rs).
// Auto-generated constraints are appended — they only affect unspecified components.
let mut all_constraints = user_constraints.to_vec();
all_constraints.extend(phase0.suggested_constraints);

let result = solve_placement(&ir, &all_constraints, &config)?;
```

**Constraint translation table:**

| Phase 0 Output | `UserConstraint` variant | Affects Phase 1 behavior |
|---------------|--------------------------|--------------------------|
| Intra-cluster pairs above threshold | `Near { a, b, max_distance_mm }` | `NearConstraint` in ConstraintSystem |
| Cluster with board region | `RegionContainment { designator, region }` | `RectRegionContainment` in ConstraintSystem |
| Connector in connector cluster | `EdgePlacement { designator, edge, inset_mm }` | `EdgePlacementConstraint` in ConstraintSystem |

**No new Solverang constraint types are needed for Phase 0.** All three variants
already exist in `lib.rs`.

---

## 13. Testing

All tests live in `crates/autopcb-placement/src/clustering.rs` within
`#[cfg(test)]` blocks.

### 13.1 Unit Tests (no feature flag required)

**Test: graph construction from synthetic netlist**

```rust
#[test]
fn test_build_graph_known_netlist() {
    // Construct a minimal PcbIr with 4 components and 3 nets:
    //   Net A: U1, U2
    //   Net B: U2, U3
    //   Net C: U1, U3, U4
    // Expected edges: U1-U2 (w=1), U2-U3 (w=1), U1-U3 (w=1), U1-U4 (w=1), U3-U4 (w=1)
    let ir = build_synthetic_ir_4comp();
    let graph = build_component_graph(&ir);
    assert_eq!(graph.graph.node_count(), 4);
    assert_eq!(graph.graph.edge_count(), 5);

    let u1 = graph.node_index["U1"];
    let u2 = graph.node_index["U2"];
    let edge = graph.graph.find_edge(u1, u2).expect("edge U1-U2 must exist");
    let w = graph.graph.edge_weight(edge).unwrap();
    assert!((w - 1.0).abs() < 1e-9, "U1-U2 should have weight 1.0");
}
```

**Test: BFS clustering produces expected groups**

```rust
#[test]
fn test_bfs_clustering_two_groups() {
    // Two disconnected cliques: {U1, U2, U3} heavily connected,
    // {U4, U5} heavily connected, one weak edge between U3-U4 (w=1).
    // Expected: BFS at threshold > 1 produces exactly 2 clusters.
    let ir = build_two_clique_ir();
    let graph = build_component_graph(&ir);
    let threshold = 2.0;  // override auto-threshold for determinism
    let clusters = cluster_bfs(&graph, threshold);
    assert_eq!(clusters.len(), 2);
    let all_members: HashSet<_> = clusters.iter()
        .flat_map(|c| c.members.iter().cloned())
        .collect();
    assert_eq!(all_members.len(), 5, "all 5 components must appear in exactly one cluster");
}
```

**Test: every component appears in exactly one cluster (BFS)**

```rust
#[test]
fn test_bfs_partition_complete() {
    let ir = build_synthetic_ir_10comp();
    let graph = build_component_graph(&ir);
    let threshold = compute_bfs_threshold(&graph);
    let clusters = cluster_bfs(&graph, threshold);

    let mut seen = HashSet::new();
    for c in &clusters {
        for m in &c.members {
            assert!(!seen.contains(m), "component {} in multiple clusters", m);
            seen.insert(m.clone());
        }
    }
    assert_eq!(seen.len(), ir.components.len(), "all components must be clustered");
}
```

**Test: every component appears in exactly one FinalGroup**

```rust
#[test]
fn test_final_groups_cover_all_components() {
    let ir = build_synthetic_ir_10comp();
    let graph = build_component_graph(&ir);
    let threshold = compute_bfs_threshold(&graph);
    let clusters = cluster_bfs(&graph, threshold);
    let groups = merge_with_user_groups(clusters, &[]);

    let mut seen = HashSet::new();
    for g in &groups {
        for m in &g.members {
            assert!(!seen.contains(m), "component {} in multiple groups", m);
            seen.insert(m.clone());
        }
    }
    assert_eq!(seen.len(), ir.components.len());
}
```

**Test: Fiedler vector splits a path graph**

```rust
#[test]
fn test_fiedler_vector_path_graph() {
    // Path graph: 1 — 2 — 3 — 4 — 5
    // Fiedler vector of a path is a sine wave; median split separates {1,2} from {3,4,5}
    // or {1,2,3} from {4,5}. Either is valid.
    let n = 5usize;
    let mut L = vec![vec![0.0f64; n]; n];
    for i in 0..(n-1) {
        L[i][i]   += 1.0;
        L[i+1][i+1] += 1.0;
        L[i][i+1] -= 1.0;
        L[i+1][i] -= 1.0;
    }
    let (v, eigenvalue, _iters) = compute_fiedler_vector(&L, 200, 1e-8);
    // Fiedler value for a path graph P_5 is approximately 0.382
    assert!(eigenvalue > 0.1, "Fiedler eigenvalue must be positive for connected graph");
    assert!(eigenvalue < 2.0, "Fiedler eigenvalue must be < 2 for path graph");
    // Vector should have mixed signs (the partition must be non-trivial)
    let positives = v.iter().filter(|&&x| x > 0.0).count();
    let negatives = v.iter().filter(|&&x| x < 0.0).count();
    assert!(positives >= 1 && negatives >= 1, "Fiedler vector must have mixed signs");
}
```

**Test: power net exclusion prevents degenerate clustering**

```rust
#[test]
fn test_power_net_excluded_from_edge_weights() {
    // 10 components, all connected to a common GND net.
    // Without exclusion, all edge weights would be ≥ 1 and everything clusters together.
    // With exclusion (GND has pin_count = 10 = N, threshold = N/4 = 2 → exclude),
    // the graph should have no edges.
    let ir = build_star_gnd_ir(10);
    let graph = build_component_graph(&ir);
    assert_eq!(graph.graph.edge_count(), 0,
        "GND net with N pins should be excluded; graph should have no edges");
}
```

### 13.2 Fixture Tests (gated behind `--features test-fixtures`)

```rust
#[cfg(feature = "test-fixtures")]
#[test]
fn test_stm32_devboard_domain_separation() {
    // Uses a known fixture PcbDoc with an STM32-based design.
    // Expect: analog components (op-amps, precision resistors) cluster separately
    // from digital components (STM32, FLASH, EEPROM).
    // This is a property check, not an exact check.
    let ir = load_fixture_pcbdoc("stm32_devboard");
    let phase0 = phase0_preprocess(&ir, &[]).unwrap();

    let digital_groups: Vec<_> = phase0.final_groups.iter()
        .filter(|g| g.domain == Domain::Digital)
        .collect();
    let analog_groups: Vec<_> = phase0.final_groups.iter()
        .filter(|g| g.domain == Domain::Analog)
        .collect();

    // At least one group of each domain must exist on a mixed board
    assert!(!digital_groups.is_empty(), "expected at least one digital cluster");
    assert!(!analog_groups.is_empty(), "expected at least one analog cluster");

    // No component should appear in both a digital and an analog group
    let digital_members: HashSet<_> = digital_groups.iter()
        .flat_map(|g| g.members.iter().cloned())
        .collect();
    let analog_members: HashSet<_> = analog_groups.iter()
        .flat_map(|g| g.members.iter().cloned())
        .collect();
    let overlap: Vec<_> = digital_members.intersection(&analog_members).collect();
    assert!(overlap.is_empty(),
        "components {:?} appear in both digital and analog groups", overlap);
}
```

### 13.3 Property Tests (gated behind `--features proptest`)

```rust
#[cfg(feature = "proptest")]
proptest! {
    #[test]
    fn prop_all_components_in_exactly_one_cluster(
        component_count in 2usize..50,
        net_count in 1usize..100,
        seed in any::<u64>(),
    ) {
        let ir = generate_random_ir(component_count, net_count, seed);
        let graph = build_component_graph(&ir);
        let threshold = compute_bfs_threshold(&graph);
        let clusters = cluster_bfs(&graph, threshold);

        let mut seen = HashMap::new();
        for c in &clusters {
            for m in &c.members {
                let prev = seen.insert(m.clone(), c.id);
                prop_assert!(
                    prev.is_none(),
                    "component {} appears in both cluster {} and cluster {}",
                    m, prev.unwrap(), c.id
                );
            }
        }
        prop_assert_eq!(seen.len(), ir.components.len());
    }
}
```

---

## 14. Performance Targets

| N (components) | Graph build | BFS cluster | Spectral cluster | Phase 0 total |
|----------------|------------|-------------|-----------------|---------------|
| 20 | <1ms | <1ms | N/A (BFS) | <5ms |
| 100 | <5ms | <5ms | ~20ms (100 iters) | <30ms |
| 300 | <10ms | <10ms | ~100ms | <150ms |
| 500 | <20ms | <20ms | ~300ms | <400ms |

Spectral bisection dominates cost at large N due to O(N²) dense Laplacian
matrix-vector products. At N=300 the inner loop is 300×300 = 90,000 multiplies
per iteration × 100 iterations = 9M multiplies, which takes ~10ms on modern
hardware. The overall `phase0_preprocess` is bounded at <500ms for any
PCB-scale design, well within the 1s budget stated in `architecture.md`.

---

## 15. Invariants and Contracts

The following invariants must hold on all code paths and should be checked
via `debug_assert` in debug builds:

1. Every component in `PcbIr::components` appears in exactly one `Cluster`
   after `cluster_bfs` or `cluster_spectral`.

2. Every component in `PcbIr::components` appears in exactly one `FinalGroup`
   after `merge_with_user_groups`.

3. No component designator that appears in any input `UserConstraint` appears
   in a non-`user_constrained` `FinalGroup`.

4. `Phase0Output::suggested_constraints` contains only `Near`,
   `RegionContainment`, and `EdgePlacement` variants — never `FixedPosition`
   or `Directional` (those are user intent, not auto-generated).

5. `ClusteringMetadata::component_count` equals `ir.components.len()`.

6. If `spectral_fallback = true`, then `algorithm_used = "bfs_fallback"`.

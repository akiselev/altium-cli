# pcb-toolkit Integration with Autorouter

How the autorouter can leverage `pcb-toolkit` (at `~/git/pcb-toolkit`) for
physics-based routing decisions.

## What pcb-toolkit Provides

pcb-toolkit is a **pure calculation library** — analytical formulas only, no
optimization or simulation. All functions are `fn(input) -> Result<output, CalcError>`
with f64 precision. 45 built-in substrate materials with Er, Tg, and roughness data.

| Calculator | What it computes | Router use case |
|-----------|-----------------|-----------------|
| `impedance::microstrip` | Zo, Er_eff, Tpd, Lo, Co | Trace width for target impedance |
| `impedance::stripline` | Same, buried trace | Inner-layer impedance |
| `impedance::embedded` | Same, embedded microstrip | Inner-layer with cover |
| `impedance::coplanar` | Coplanar waveguide Zo | Coplanar routing |
| `differential::edge_coupled_*` | Zdiff, Zodd, Zeven, Kb | Diff pair gap/width validation |
| `differential::broadside_coupled` | Same, vertical coupling | Broadside diff pairs |
| `current::ipc2221a` / `ipc2152` | Current capacity, Vdrop, Pdiss | Power trace width |
| `via::calculate` | C_pf, L_nH, Z_ohms, f_res | Via SI penalty in cost function |
| `crosstalk::calculate` | Kb, NEXT, coupled voltage | Spacing decisions |
| `spacing::spacing` | Min spacing (mils) for voltage | Clearance rules |
| `thermal::calculate` | Junction temperature | Component thermal routing |
| `pdn::calculate` | Target Z, plane capacitance | PDN-aware routing |
| `wavelength::calculate` | Lambda, lambda/4, lambda/10 | Max stub length |

**Not an optimizer**: pcb-toolkit computes forward (geometry -> electrical properties).
The router must wrap it with inverse solvers or pre-computed lookup tables.

---

## Integration Architecture

### Pre-Route: Build Lookup Tables

Before routing starts, pre-compute tables from the stackup + design rules:

```rust
use pcb_toolkit::{impedance, differential, current, materials};

/// For each (layer, impedance_class) pair, find the trace width
/// that achieves the target impedance.
fn build_width_table(
    stackup: &IrLayerStack,
    rules: &[IrDesignRule],
) -> BTreeMap<(LayerId, ImpedanceClass), TraceWidthMm> {
    // For each impedance target (e.g., 50ohm, 90ohm diff):
    //   Binary search over trace width:
    //     width -> impedance::microstrip::calculate(...) -> Zo
    //     Find width where Zo == target within tolerance
}
```

This produces a lookup table the router uses during A*/Bellman-Ford without calling
pcb-toolkit per-cell. The table is small (layers x impedance_classes, typically <100
entries) and computed once.

**Tables to pre-compute:**

| Table | Key | Value | Source calculator |
|-------|-----|-------|-------------------|
| Trace width for impedance | (layer, Zo_target) | width_mm | `impedance::microstrip` / `stripline` |
| Diff pair width+gap | (layer, Zdiff_target) | (width_mm, gap_mm) | `differential::edge_coupled_*` |
| Trace width for current | (layer, I_max) | width_mm | `current::ipc2221a` |
| Min spacing for voltage | (voltage, device_type) | spacing_mm | `spacing::spacing` |
| Via parasitics | (drill, pad, antipad, height) | (L_nH, C_pF, Z) | `via::calculate` |
| Max stub length | (frequency, Er) | length_mm | `wavelength::calculate` (lambda/10) |
| Propagation delay | (layer, width) | ps/mm | `impedance::*` (Tpd field) |

### During Route: Cost Function Terms

The pre-computed tables feed into the Bellman-Ford cost function:

```rust
fn edge_cost(from: GridNode, to: GridNode, net_class: &NetClass) -> u32 {
    let base = BASE_COST;

    // Width-dependent obstacle inflation already baked into obstacle map
    // (different net classes have different effective trace widths)

    // Via transition: add SI penalty from pre-computed via parasitics
    let via_penalty = if from.layer != to.layer {
        let via_props = via_table.get(&(drill, pad, antipad, height));
        // Higher L and C = higher penalty
        (via_props.inductance_nh * VIA_L_WEIGHT
         + via_props.capacitance_pf * VIA_C_WEIGHT) as u32
    } else { 0 };

    // Propagation delay tracking (for length matching)
    // Tpd varies by layer (different Er_eff for microstrip vs stripline)
    let delay_per_cell = delay_table.get(&(to.layer, net_class.width));

    base + via_penalty + history_cost + ...
}
```

### Post-Route: Validation

After routing, validate the solution against electrical constraints:

```rust
fn validate_solution(solution: &RouteSolution, stackup: &IrLayerStack) -> Vec<Violation> {
    let mut violations = vec![];

    for (net_id, routed_net) in &solution.nets {
        // Check impedance
        for seg in &routed_net.segments {
            let input = MicrostripInput {
                width: seg.width_mm * 39.37, // mm to mils
                height: stackup.dielectric_height(seg.layer),
                thickness: stackup.copper_thickness(seg.layer),
                er: stackup.material(seg.layer).er,
                frequency: net_class.frequency,
            };
            let result = impedance::microstrip::calculate(&input)?;
            if (result.zo - target_zo).abs() > tolerance {
                violations.push(Violation::Impedance { net_id, seg, actual: result.zo });
            }
        }

        // Check current capacity
        for seg in &routed_net.segments {
            let input = CurrentInput { width, thickness, length, temperature_rise, .. };
            let result = current::ipc2221a::calculate(&input)?;
            if result.current_capacity < required_current {
                violations.push(Violation::CurrentCapacity { net_id, seg, .. });
            }
        }

        // Check via resonance (ensure f_res >> signal frequency)
        for via in &routed_net.vias {
            let result = via::calculate(&ViaInput { .. })?;
            if result.resonant_freq_mhz < signal_freq_mhz * 3.0 {
                violations.push(Violation::ViaResonance { net_id, via, .. });
            }
        }

        // Check crosstalk between adjacent traces
        // (requires spatial queries on routed segments)
    }

    violations
}
```

### LLM Spec Generation: Pre-Analysis

The LLM agent can use pcb-toolkit during spec generation to pre-compute constraints:

```
// LLM generates this spec block by calling pcb-toolkit:
impedance_targets {
  // microstrip::calculate(width=5mil, h=4.5mil, t=1.4mil, er=4.2, f=1GHz)
  // -> Zo = 50.3 ohms (target: 50)
  net_class "signal_50ohm" {
    layer L1 { width: 0.127mm }  // 5mil, computed for 50ohm on L1
    layer L2 { width: 0.102mm }  // 4mil, computed for 50ohm on L2 (stripline)
  }

  // differential::edge_coupled_external(w=4mil, s=5mil, h=4.5mil, ...)
  // -> Zdiff = 90.1 ohms (target: 90)
  net_class "usb_diff" {
    layer L1 { width: 0.102mm, gap: 0.127mm }  // 4mil/5mil for 90ohm diff
  }
}

// current::ipc2221a(width=40mil, t=1.4mil, rise=10C, internal)
// -> I_capacity = 1.8A (required: 1.5A, margin: 20%)
power_traces {
  net VCC_3V3 { min_width: 1.016mm }  // 40mil for 1.5A with margin
  net GND     { min_width: 1.016mm }
}
```

---

## What pcb-toolkit Can and Cannot Do for the Router

### Can Do (Forward Calculations)

- **"Given this trace geometry on this layer, what's the impedance?"** -> Yes
- **"Given this trace width and copper weight, what current can it carry?"** -> Yes
- **"What are the parasitics of this via geometry?"** -> Yes
- **"What's the minimum spacing for 48V?"** -> Yes
- **"What's the propagation delay per inch on this layer?"** -> Yes
- **"What's the crosstalk between these two traces?"** -> Yes (NEXT estimation)
- **"What material properties does FR-4 have?"** -> Yes (45 materials)

### Cannot Do (Needs Wrapper)

- **"What trace width gives me 50 ohms on L2?"** -> Need binary search wrapper
- **"What gap gives me 90 ohm differential?"** -> Need binary search wrapper
- **"Optimize the stackup for these impedance targets"** -> Need optimization loop
- **"Find the best via size for SI"** -> Need cost function + search
- **"Route this net"** -> Not a router, just a calculator

### Binary Search Wrapper (Simple Inverse Solver)

For the common "find width for target impedance" problem:

```rust
fn find_width_for_impedance(
    target_zo: f64,
    layer: &LayerInfo,
    tolerance: f64, // e.g., 0.5 ohm
) -> Result<f64, CalcError> {
    let mut lo = 0.5;   // mils
    let mut hi = 200.0;  // mils

    for _ in 0..50 {  // converges in ~17 iterations for 0.001 mil precision
        let mid = (lo + hi) / 2.0;
        let input = MicrostripInput {
            width: mid,
            height: layer.dielectric_height_mils,
            thickness: layer.copper_thickness_mils,
            er: layer.material.er,
            frequency: 1e9, // 1 GHz reference
        };
        let result = impedance::microstrip::calculate(&input)?;

        if (result.zo - target_zo).abs() < tolerance {
            return Ok(mid);
        }
        if result.zo > target_zo {
            lo = mid;  // wider trace = lower impedance
        } else {
            hi = mid;
        }
    }
    Ok((lo + hi) / 2.0)
}
```

---

## GPU Considerations

pcb-toolkit runs on CPU (f64, analytical formulas). For GPU routing:

1. **Pre-compute all tables on CPU** before GPU routing starts. The tables are small
   enough to upload as uniform buffers or small storage buffers.

2. **Bake width-dependent data into obstacle maps**: Different net classes have different
   trace widths (from impedance targets). The obstacle map inflation radius depends on
   trace width + clearance. Build per-net-class obstacle maps or parameterize the cost
   function.

3. **Via cost model on GPU**: Upload pre-computed via parasitics as a small lookup table
   (typically <10 via types). The Bellman-Ford shader indexes into this table for layer
   transitions.

4. **Propagation delay on GPU**: Upload per-layer Tpd values. The shader accumulates
   delay along the path for length-matching cost terms.

5. **Post-route validation on CPU**: After GPU routing produces a solution, validate on
   CPU using pcb-toolkit. This is not performance-critical (runs once per solution).

### Buffer Layout

```
Uniform buffer: RoutingPhysics {
    // Per-layer (max 32 layers)
    layer_tpd_ps_per_mm: [f32; 32],      // propagation delay
    layer_trace_width_mm: [f32; 32],       // per net class (need one per class)
    layer_type: [u32; 32],                 // 0=microstrip, 1=stripline

    // Per-via-type (max 8 types)
    via_inductance_cost: [u32; 8],         // fixed-point, pre-scaled
    via_capacitance_cost: [u32; 8],        // fixed-point, pre-scaled

    // Global
    min_spacing_cells: u32,                // from spacing::spacing()
}
```

---

## Dependency Integration

Add `pcb-toolkit` as a workspace dependency:

```toml
# Cargo.toml (workspace root)
[workspace.dependencies]
pcb-toolkit = { path = "../pcb-toolkit/crates/pcb-toolkit" }

# crates/autopcb-router/Cargo.toml
[dependencies]
pcb-toolkit = { workspace = true }
```

pcb-toolkit has minimal dependencies (only `thiserror` + `serde`), so this is
lightweight. It fits naturally as a calculation backend that the router's
`build_workspace()` calls during table construction.

---

## Integration Points in Router Plan

| Router Milestone | pcb-toolkit Use |
|-----------------|-----------------|
| M1 (scaffold) | Add `pcb-toolkit` dependency |
| M3 (rules bridge) | Build width/gap tables from impedance + current calculators |
| M4 (workspace) | Bake width-dependent inflation into obstacle maps |
| M6 (detailed) | Via cost model uses pre-computed parasitics |
| M7 (PathFinder) | Propagation delay tracking for length matching |
| M8 (optimize) | Validate impedance/current after optimization passes |
| M8 (high-speed) | Diff pair gap from differential impedance calculator |
| M9 (spec+CLI) | Report impedance/current metrics in routing stats |
| M11 (co-opt) | Congestion oracle uses trace width from impedance tables |

---

## Summary

pcb-toolkit is a **constraint oracle** for the router. It answers "is this geometry
electrically valid?" and "what geometry achieves this electrical target?" The router
uses it in three phases:

1. **Pre-route**: Build lookup tables (width-for-impedance, current capacity, via
   parasitics, spacing rules, propagation delay)
2. **During route**: Tables feed GPU cost function terms and obstacle map inflation
3. **Post-route**: Full electrical validation of the solution

The LLM spec generator can also use pcb-toolkit to pre-compute constraints during
spec authoring, so the router receives pre-validated targets rather than discovering
them at route time.

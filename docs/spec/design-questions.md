# Spec Language Design Questions

Three open design questions from the SchDoc extension work.


## 1. Import Semantics: All Imports Are References

### Decision

**All imports are named references.** No bare imports. No composition/merge semantics.

The original bare import model (merge multiple `.sym` files into one SchLib output)
adds complexity for marginal benefit. Since a SchDoc can import individual `.sym`
files directly, there's no need to pre-merge them into a single SchLib. Each spec file
produces exactly one output file.

### Before (two import modes)

```
import "passives.sym"              // bare: merge into my output
import "footprints.sym" as fp      // named: reference only
```

### After (one import mode)

```
import "passives.sym" as passives  // reference
import "ics.sym" as ics            // reference
import "footprints.sym" as fp      // reference
```

### Rules

1. **All imports require `as alias`**. No bare imports.
2. **Each spec file produces exactly one output file** (1:1 mapping).
3. **Named imports are read-only references** — definitions available as `$alias.Name`,
   nothing from imports goes into the output.
4. **Both spec files and compiled Altium binaries can be imported:**
   ```
   import "my-parts.sym" as lib        // spec source
   import "vendor-parts.SchLib" as vendor       // compiled binary
   ```

### Compatibility Matrix

| From | Can import (named) | Purpose |
|------|-------------------|---------|
| `.sym` (component) | `.sym`, `.SchLib`, `.PcbLib` | Reuse templates, footprint refs |
| `.sym` (footprint) | `.sym`, `.PcbLib` | Reuse templates |
| `.sch` | `.sym`, `.SchLib`, `.sch` | Component defs, hierarchy refs |
| `.pcb` | `.sch`, `.sym`, `.SchDoc`, `.PcbLib` | Netlist + footprint defs |

### What This Means for Existing SchLib Workflows

Instead of:
```
// old: main.sym merges passives + ics into one SchLib
import "passives.sym"
import "ics.sym"
```

Users write separate spec files that each produce their own output:
```
passives.sym  ->  passives.SchLib
ics.sym       ->  ics.SchLib
```

And SchDoc specs reference them individually:
```
// board.sch
import "passives.sym" as passives
import "ics.sym" as ics

R1 = place $passives.R_0603 { at: (1in, 1in), value: "10K" }
U1 = place $ics.LM358 { at: (2in, 1in) }
```

This is actually a better model — it matches how real Altium projects work (multiple
library files, each focused on a domain).

### Importing Compiled Altium Files

Import real Altium binary files as read-only references:

```
import "vendor-parts.SchLib" as vendor
U1 = place $vendor.STM32F103 { at: (2in, 1in) }
```

When importing a binary file:
1. Parse the file using existing `altium-format` crate
2. Convert to a read-only definition model (component list with pins/params/graphics)
3. Expose as a namespace, same as a spec import
4. The `dump` module already does something similar (spec from binary)

This is key for adoption — users reference existing libraries without needing their spec.

### Implementation Impact

The import resolver simplifies significantly:
- Remove bare import handling (`bare_imports` field, collision detection, topo ordering)
- All imports go into `named_imports`
- `resolve_imports()` returns a flat map of `alias -> (path, definitions)`
- Cross-domain validation uses the matrix above
- Binary file detection by extension (`.SchLib`, `.PcbLib`, `.SchDoc`)


## 2. Connectivity via Net Labels (No Autorouting)

### Decision

**Skip wire autorouting entirely.** Use short wire stubs + net labels on every pin.
Connectivity is purely logical via matching net names. No geometric routing needed.

### How It Works

Every pin gets a short wire stub extending from the pin endpoint, with a net label
at the stub's end. Pins on the same net share the same net label text. Altium's
netlister connects them logically.

```
net VCC {
    $U1.8
    $R1.1
    $C1.1
    power { style: arrow }
}

net SIG_A {
    $R1.2
    $U1.2
}
```

Compiles to (per pin on each net):
1. A short wire stub (2 vertices) extending from the pin tip
2. A net label at the wire endpoint with the net name
3. For power nets: a power object instead of a net label

### Wire Stub Geometry

The stub extends from the pin's connection point in the pin's orientation direction:

```
Pin orientation 0   (points right, connects left):  stub goes LEFT
Pin orientation 180 (points left, connects right):  stub goes RIGHT
Pin orientation 90  (points down, connects top):    stub goes UP
Pin orientation 270 (points up, connects bottom):   stub goes DOWN
```

Stub length: fixed short distance (e.g., 10mil or 0 — just enough for the net label
to attach to the wire endpoint).

Actually, the simplest approach: **zero-length wire** (wire with start == end at the
pin tip) plus net label at the same point. Altium requires a wire for net label
attachment, but it can be degenerate.

Or even simpler: just a net label placed at the pin's connection point. Check whether
Altium requires a wire segment for connectivity or if a net label on a pin tip is
sufficient.

**UPDATE**: Altium requires pins to be connected via wires or direct contact. A net
label attached to a wire (even a short one) creates the net connection. The minimal
approach:

1. Wire: 2 vertices, pin_tip to pin_tip + small_offset_in_pin_direction
2. Net label: placed at the wire's outer endpoint

### Why This Is Better Than Autorouting (For Now)

1. **Trivially idempotent**: Same net declarations -> same stubs + labels. No routing
   algorithm, no path-dependency, no non-determinism risk.

2. **Simple to implement**: No A*, no obstacle avoidance, no MST. Just compute pin
   tip position + offset, emit wire + label.

3. **LLM-friendly**: LLMs declare connectivity (`net VCC { $U1.8, $R1.1 }`), tool
   handles placement. No spatial reasoning needed.

4. **Schematics look normal**: This is actually how many engineers draw schematics —
   net labels everywhere, minimal wires. It's a valid schematic style.

5. **Extensible**: Autorouting can be added later as an optional mode without changing
   the spec syntax. The `net` block stays the same; only the compiler's output changes.

### Power Objects vs Net Labels

For standard nets: emit a **net label** (Record 25).
For power nets: emit a **power object** (Record 17) instead.

The spec syntax makes this explicit:
```
net VCC {
    $U1.8, $R1.1
    power { style: arrow }      // use power symbol, not net label
}

net SIG_A {
    $R1.2, $U1.2               // no power declaration -> use net labels
}
```

If a net has `power { }`, all pins on that net get power objects.
Otherwise, all pins get net labels.

### Identity

Each wire stub and net label gets a deterministic UniqueID from a seed:

| Entity | Seed |
|--------|------|
| Wire stub | `spec:{file}:stub:{net}:{designator}.{pin}` |
| Net label | `spec:{file}:label:{net}:{designator}.{pin}` |
| Power object | `spec:{file}:power:{net}:{designator}.{pin}` |

Example: `spec:power-supply:stub:VCC:U1.8` -> hash to 8-char ID.

### Future: Optional Autorouting

The `net` block syntax is forward-compatible with autorouting:

```
// V1: stub + label (current)
net VCC { $U1.8, $R1.1 }

// Future V2: autorouted wires (same syntax, different compilation mode)
// altium apply --route=auto power-supply.sch
```

The routing mode is a tool flag, not a syntax change. Users who want routed
schematics can opt in later.


## 3. UniqueID Hashing

### Altium's Algorithm (from C# decompilation)

Altium has two UniqueID generation methods:

**Random (interactive use)** — `SchDataUtils.GenerateUniqueID()`:
```csharp
uniqueIdGenerator = new Random(Guid.NewGuid().GetHashCode());
for (int i = 0; i < 8; i++)
    buffer[i] = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"[uniqueIdGenerator.Next(26)];
```
8 random uppercase letters. Not deterministic. Not suitable for spec system.

**Deterministic (seeded)** — `UniqueIdUtils.GenerateUniqueId(seed)`:
```csharp
string md5hex = RtMD5Ex.FullStringMD5Digest(seed);  // 32 uppercase hex chars
StringBuilder result = new StringBuilder();
for (int i = 0; i < 32; i += 4) {
    int hash = 19;
    for (int j = 0; j < 4; j++)
        hash = hash * 31 + HexValue(md5hex[i + j]);  // 0-15
    result.Append("ABCDEFGHIJKLMNOPQRSTUVWXYZ"[hash % 26]);
}
```
MD5 the seed -> take 4 hex chars at a time -> fold into one base-26 char.
Produces 8 uppercase letters deterministically from any seed string.

**Collision resolution** — `UniqueIdUtils.GetNextUniqueId(id)`:
Base-26 increment with carry. "AAAAAAAA" -> "AAAAAAAB", "AAAAAAAZ" -> "AAAAAABA".

### Our Approach: Replicate the Deterministic Algorithm

We replicate `UniqueIdUtils.GenerateUniqueId(seed)` exactly. This means our
generated IDs are compatible with Altium's own deterministic IDs, and the
collision resolution (`GetNextUniqueId`) works identically.

### Rust Implementation

```rust
use md5;

/// Generate an 8-character uppercase alphabetic UniqueID from a seed string.
/// Replicates Altium's `UniqueIdUtils.GenerateUniqueId(seed)` algorithm.
fn generate_unique_id(seed: &str) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";

    let digest = md5::compute(seed.as_bytes());
    let hex = format!("{:032X}", digest);  // 32 uppercase hex chars
    let hex_bytes = hex.as_bytes();

    let mut result = String::with_capacity(8);
    for group in 0..8 {
        let mut hash: i32 = 19;
        for j in 0..4 {
            let ch = hex_bytes[group * 4 + j];
            let val = match ch {
                b'0'..=b'9' => (ch - b'0') as i32,
                b'A'..=b'F' => (ch - b'A') as i32 + 10,
                _ => 0,
            };
            hash = hash.wrapping_mul(31).wrapping_add(val);
        }
        result.push(ALPHABET[hash.rem_euclid(26) as usize] as char);
    }
    result
}

/// Base-26 increment with carry (collision resolution).
/// Replicates Altium's `UniqueIdUtils.GetNextUniqueId(id)`.
fn next_unique_id(id: &str) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut chars: Vec<u8> = id.bytes().collect();
    let mut carry = true;
    for i in (0..8).rev() {
        if !carry { break; }
        let idx = (chars[i] - b'A') as usize;
        if idx == 25 {
            chars[i] = b'A';  // wrap, carry
        } else {
            chars[i] = ALPHABET[idx + 1];
            carry = false;
        }
    }
    String::from_utf8(chars).unwrap()
}
```

### Seed Strings

Each entity type uses a canonical seed format:

| Entity | Seed format | Example |
|--------|-------------|---------|
| SchLib component graphic | `spec:{component}:{binding}` | `spec:R_0603:body` |
| SchLib part graphic | `spec:{component}:part{N}:{binding}` | `spec:LM358:part1:body` |
| SchLib unnamed graphic | `spec:{component}:{type}_{counter}` | `spec:R_0603:line_0` |
| PcbLib footprint graphic | `spec:{footprint}:{binding}` | `spec:SOT23:courtyard` |
| SchDoc component instance | `spec:{file}:inst:{designator}` | `spec:psu:inst:R1` |
| SchDoc wire stub | `spec:{file}:stub:{net}:{desig}.{pin}` | `spec:psu:stub:VCC:U1.8` |
| SchDoc net label | `spec:{file}:label:{net}:{desig}.{pin}` | `spec:psu:label:VCC:U1.8` |
| SchDoc power object | `spec:{file}:power:{net}:{desig}.{pin}` | `spec:psu:power:VCC:U1.8` |
| SchDoc port | `spec:{file}:port:{name}` | `spec:psu:port:DATA` |
| SchDoc sheet symbol | `spec:{file}:sheetsym:{name}` | `spec:psu:sheetsym:Regulators` |

The `{file}` is the spec filename stem (e.g., `psu` from `psu.sch`).

### Collision Handling

Within a single document, maintain a `HashSet<String>` of assigned UniqueIDs.
After generating an ID from a seed, check for collision:

```rust
fn assign_unique_id(seed: &str, assigned: &mut HashSet<String>) -> String {
    let mut id = generate_unique_id(seed);
    while assigned.contains(&id) {
        id = next_unique_id(&id);
    }
    assigned.insert(id.clone());
    id
}
```

This matches Altium's own collision-resolution pattern (seen in `PrjPcbContent.cs`
line 280: `text = UniqueIdUtils.GetNextUniqueId(text)` in a loop).

### Stability Across Edits

The seed-based approach means:
- **Adding** a new entity gets a new deterministic ID (no effect on existing IDs)
- **Removing** an entity frees its ID (existing IDs unaffected)
- **Renaming** a designator (R1 -> R2) changes the seed -> new ID.
  Reconciler treats as "old removed + new added" (additive: old stays, new added)

Existing entity IDs are **stable** as long as their identity key doesn't change.

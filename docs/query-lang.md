# Altium Query Language (AQL)

A unified query language for selecting and filtering records in Altium Designer files.

## Overview

AQL combines simple pattern-based selectors with CSS-like attribute matching to provide both convenience and power:

- **Simple patterns** for quick component lookups (`R*`, `U1`, `~VCC`)
- **Attribute selectors** for complex filtering (`component[value=10K]`)
- **Pseudo-classes** for type and classification (`:power`, `:input`)
- **Combinators** for relational queries (`U1 > pin`)

## Basic Syntax

### Pattern Selectors

Quick lookup patterns for common queries:

| Pattern | Matches | Example | Description |
|---------|---------|---------|-------------|
| `<designator>` | Exact match | `U1` | Single component by designator |
| `<prefix>*` | Wildcard suffix | `R*` | All resistors (R1, R2, ...) |
| `<prefix>??` | Fixed-length wildcard | `C??` | C01-C99 (not C1 or C100) |
| `<prefix>?` | Single wildcard | `U?` | U1-U9 (not U10) |
| `$<part>` | Part number | `$LM358` | By library part number |
| `~<net>` | Net name | `~VCC` | All items on net VCC |
| `@<value>` | Component value | `@10K` | All 10K resistors |
| `#<id>` | Record ID | `#42` | By internal record ID |
| `<comp>:<pin>` | Component pin | `U1:VCC` | Specific pin on component |

**Examples:**
```
R*              # All resistors
C10             # Capacitor C10
U?              # U1-U9
~GND            # Everything connected to GND
@100nF          # All 100nF capacitors
$LM358          # All LM358 components
U1:OUT          # Output pin of U1
```

### Element Type Selectors

Select by record type (case-insensitive):

| Selector | Matches |
|----------|---------|
| `component` | All components |
| `pin` | All pins |
| `net` | All nets |
| `wire` | All wires |
| `bus` | All buses |
| `port` | All ports |
| `power` | All power objects |
| `label` | All labels |
| `netlabel` | All net labels |
| `junction` | All junctions |
| `sheet` | All sheet symbols |
| `parameter` | All parameters |
| `line` | All lines |
| `arc` | All arcs |
| `text` | All text objects |
| `polygon` | All polygons |
| `rectangle` | All rectangles |
| `pad` | All pads (PCB) |
| `via` | All vias (PCB) |
| `track` | All tracks (PCB) |
| `fill` | All fills (PCB) |
| `region` | All regions (PCB) |
| `rule` | All design rules (PCB) |

**Examples:**
```
component       # All components
pin             # All pins
track           # All PCB tracks
```

## Attribute Selectors

Filter by record fields using CSS-like syntax:

| Selector | Operator | Meaning | Example |
|----------|----------|---------|---------|
| `[field=value]` | `=` | Exact match | `[designator=U1]` |
| `[field!=value]` | `!=` | Not equal | `[layer!=Top]` |
| `[field*=value]` | `*=` | Contains | `[description*=resistor]` |
| `[field^=value]` | `^=` | Starts with | `[designator^=R]` |
| `[field$=value]` | `$=` | Ends with | `[footprint$=0603]` |
| `[field~=value]` | `~=` | Word match | `[comment~=DNP]` |
| `[field>value]` | `>` | Greater than | `[x>1000]` |
| `[field<value]` | `<` | Less than | `[y<500]` |
| `[field>=value]` | `>=` | Greater or equal | `[width>=10]` |
| `[field<=value]` | `<=` | Less or equal | `[height<=20]` |

**Value types:**
- **String**: `"value"` or `value` (quoted if contains spaces/special chars)
- **Number**: `123`, `45.67`, `-10`
- **Boolean**: `true`, `false`
- **Coordinate**: `1000mil`, `2.54mm`, `0.1in`

**Examples:**
```
component[value=10K]           # Components with value "10K"
component[footprint*=0603]     # Components with "0603" in footprint
pin[electrical=power]          # Power pins
net[name^=VCC]                 # Nets starting with "VCC"
track[width>=10mil]            # Tracks 10 mils or wider
component[x>1000][y<2000]      # Components in region
```

## Pseudo-Classes

Filter by classification or derived properties:

### Pin Types
```
pin:input               # Input pins
pin:output              # Output pins
pin:io                  # Bidirectional pins
pin:power               # Power pins
pin:passive             # Passive pins
pin:open-collector      # Open collector pins
pin:open-emitter        # Open emitter pins
pin:hiz                 # High impedance pins
```

### Net Classifications
```
net:power               # Power nets (VCC, VDD, etc.)
net:ground              # Ground nets (GND, VSS, etc.)
net:signal              # Signal nets
net:differential        # Differential pair nets
net:unrouted            # Unrouted nets (PCB)
```

### Component States
```
component:placed        # Placed components (PCB)
component:locked        # Locked components
component:virtual       # Virtual components (no PCB footprint)
```

### Geometric
```
:selected               # Currently selected objects
:visible                # Visible on current layer(s)
:on-grid                # On design grid
```

**Examples:**
```
pin:power               # All power pins
net:differential        # All differential pairs
component:virtual       # Schematic-only components
track:selected          # Selected tracks
```

## Combinators

Combine selectors for relational queries:

| Combinator | Meaning | Example |
|------------|---------|---------|
| (space) | Descendant | `component pin` |
| `>` | Direct child | `component > pin` |
| `+` | Adjacent sibling | `wire + junction` |
| `~` | General sibling | `component ~ component` |
| `,` | Union (OR) | `R*, C*` |

**Examples:**
```
component pin                   # All pins of all components
U1 > pin                       # Direct pins of U1
component > pin:power          # Power pins of components
wire + junction                # Junctions adjacent to wires
R*, C*                         # All resistors and capacitors
component[value=10K], @100nF   # 10K resistors OR 100nF caps
```

## Logical Operators

Combine multiple conditions:

| Operator | Meaning | Example |
|----------|---------|---------|
| `AND` | Both true | `component AND [value=10K]` |
| `OR` | Either true | `R* OR C*` |
| `NOT` | Negation | `NOT :virtual` |

**Operator precedence** (high to low):
1. `NOT`
2. `AND`
3. `OR`
4. `,` (union)

**Examples:**
```
component AND [value=10K] AND :placed
R* OR C* OR L*
component AND NOT :virtual
pin:power OR pin:ground
```

## Advanced Queries

### Numeric Ranges
```
component[x>=1000][x<=2000][y>=500][y<=1500]    # Bounding box
track[width>=8mil][width<=12mil]                # Width range
```

### Regular Expressions
Use `/pattern/` for regex matching:
```
component[designator=/^U[0-9]{2}$/]             # U01-U99 exactly
net[name=/VCC_[0-9]+V/]                         # VCC_3V, VCC_5V, etc.
component[comment=/DNP|DNI/]                    # Do not place/install
```

### Wildcard Escaping
Escape special characters with backslash:
```
component[value=10K\*]          # Value is literally "10K*"
net[name=~VCC]                  # Net name is literally "~VCC"
```

### Case Sensitivity
By default, string matching is case-insensitive. Use `i` or `s` suffix:

```
[designator=u1]i                # Case-insensitive (default)
[designator=U1]s                # Case-sensitive
```

## Practical Examples

### Bill of Materials (BOM) Queries

```bash
# All resistors
altium-cli query design.SchDoc "R*"

# All 10K resistors
altium-cli query design.SchDoc "@10K"
altium-cli query design.SchDoc "component[value=10K]"

# All 0603 SMD capacitors
altium-cli query design.SchDoc "C*[footprint*=0603]"

# High-value resistors
altium-cli query design.SchDoc "R*[value>100K]"
```

### Connectivity Queries

```bash
# All power nets
altium-cli query design.SchDoc "net:power"

# Everything connected to VCC
altium-cli query design.SchDoc "~VCC"

# All differential pairs
altium-cli query design.SchDoc "net:differential"

# Unconnected pins
altium-cli query design.SchDoc "pin NOT net"
```

### PCB Layout Queries

```bash
# Top layer components
altium-cli query design.PcbDoc "component[layer=Top]"

# Wide tracks (>= 20 mils)
altium-cli query design.PcbDoc "track[width>=20mil]"

# Vias on specific net
altium-cli query design.PcbDoc "via[net=VCC]"

# Components in region
altium-cli query design.PcbDoc "component[x>=1000][x<=3000][y>=500][y<=2000]"
```

### Design Rule Queries

```bash
# All clearance rules
altium-cli query design.PcbDoc "rule[type=clearance]"

# Strict width constraints
altium-cli query design.PcbDoc "rule[type=width][min>=10mil]"
```

### Library Queries

```bash
# Find LM358 variants
altium-cli query library.SchLib "$LM358*"

# Find all SOICs
altium-cli query library.PcbLib "component[name*=SOIC]"

# Power symbol search
altium-cli query library.SchLib "component:virtual pin:power"
```

## Query Result Operations

Queries return a collection of matching records. Operations can be chained:

```bash
# Count results
altium-cli query design.SchDoc "R*" --count

# Export to JSON
altium-cli query design.SchDoc "component[value=10K]" --json

# Delete matched records (edit mode)
altium-cli edit design.SchDoc -c "delete $(query 'R*[value<100]')"

# Highlight in GUI (if supported)
altium-cli query design.SchDoc "~VCC" --highlight
```

## Implementation Notes

### Field Name Resolution

Field names are resolved in this order:

1. **Native record fields** (designator, x, y, etc.)
2. **Computed properties** (net, electrical, layer)
3. **Parameters** (user-defined parameters on components)

For ambiguity, use prefixes:
```
field.designator        # Native field
param.designator        # Parameter named "designator"
```

### Coordinate Units

Coordinates are internally in 10K units/mil. Query syntax accepts:
- Bare numbers: treated as mils (`1000` = 1000 mils)
- Explicit units: `1000mil`, `1in`, `25.4mm`

### Performance Hints

- **Pattern selectors** (e.g., `R*`) are optimized with indexes
- **Attribute selectors** may require full table scans
- Use **type selectors** to narrow search space: `component[value=10K]` vs `component AND [value=10K]`
- Combine with `--limit N` for large result sets

### Error Handling

Invalid queries produce clear error messages:
```
ERROR: Unknown field 'foo' in component[foo=bar]
       Available fields: designator, x, y, part_id, footprint, ...

ERROR: Invalid operator '^=' for numeric field 'x'
       Use: <, <=, >, >=, =, !=

ERROR: Unterminated string in [description="missing quote]
```

## Grammar (EBNF)

```ebnf
query           = expr
expr            = term (("OR" | ",") term)*
term            = factor ("AND" factor)*
factor          = "NOT" factor | selector
selector        = pattern | element_type | compound_selector
pattern         = designator_pat | net_pat | value_pat | part_pat | pin_pat
designator_pat  = IDENT ("*" | "?" | "??")?
net_pat         = "~" IDENT
value_pat       = "@" VALUE
part_pat        = "$" IDENT
pin_pat         = IDENT ":" IDENT
element_type    = ("component" | "pin" | "net" | "wire" | ...)
compound_selector = (element_type | pattern) (attr_sel | pseudo)* combinator?
attr_sel        = "[" field op value "]"
pseudo          = ":" IDENT
combinator      = (" " | ">" | "+" | "~") selector
field           = IDENT ("." IDENT)?
op              = "=" | "!=" | "*=" | "^=" | "$=" | "~=" | ">" | "<" | ">=" | "<="
value           = STRING | NUMBER | BOOLEAN | COORD
```

## Future Extensions

Potential extensions for future versions:

- **Spatial queries**: `within(polygon)`, `distance(point, 100mil)`
- **Aggregate functions**: `count()`, `sum()`, `max()`
- **Subqueries**: `component[net IN (query 'net:power')]`
- **Variables**: `LET $vcc = 'net:power'; component[net IN $vcc]`
- **Macros**: User-defined query shortcuts

## Version History

- **v2.0** (planned): Initial unified query language
  - Merged pattern and attribute selectors
  - Added pseudo-classes
  - Added combinators and logical operators

# pcbdoc - PCB Document Commands

Analysis and editing commands for PCB documents (.PcbDoc).

## Analysis Commands

| Command | Purpose |
|---------|---------|
| `overview` | Document overview with components, nets, rules |
| `info` | Document info and statistics |
| `layers` | Show layer stack |
| `settings` | Show board settings |
| `outline` | Show board outline |

## Design Rules

| Command | Purpose |
|---------|---------|
| `rules` | List all design rules |
| `rule` | Show specific rule details |
| `add-rule` | Add design rule |
| `modify-rule` | Modify existing rule |
| `delete-rule` | Delete design rule |

## Components

| Command | Purpose |
|---------|---------|
| `components` | List all components |
| `component` | Show component details |
| `place-component` | Place component on board |
| `add-component` | Add new component |

## Routing

| Command | Purpose |
|---------|---------|
| `nets` | List all nets |
| `tracks` | List tracks |
| `add-track` | Add track segment |
| `add-track-path` | Add multi-segment track path |
| `vias` | List vias |
| `add-via` | Add via |
| `arcs` | List arcs |
| `add-arc` | Add arc |

## Copper Features

| Command | Purpose |
|---------|---------|
| `polygons` | List copper pours |
| `polygon` | Show polygon details |
| `add-polygon` | Add copper pour |
| `fills` | List fills |
| `add-fill` | Add fill |
| `regions` | List regions |
| `add-region` | Add region |

## Board Outline

| Command | Purpose |
|---------|---------|
| `keepouts` | List keepout regions |
| `add-keepout` | Add keepout region |
| `cutouts` | List board cutouts |
| `add-cutout` | Add board cutout |
| `set-outline-rect` | Set rectangular board outline |
| `set-outline` | Set board outline from vertices |

## Text & Annotations

| Command | Purpose |
|---------|---------|
| `texts` | List text objects |
| `add-text` | Add text |

## Export

| Command | Purpose |
|---------|---------|
| `json` | Export as JSON |

## Examples

### Analysis

```bash
# Overview
altium-cli pcbdoc overview design.PcbDoc

# Info and settings
altium-cli pcbdoc info design.PcbDoc
altium-cli pcbdoc settings design.PcbDoc
altium-cli pcbdoc layers design.PcbDoc
altium-cli pcbdoc outline design.PcbDoc

# Design rules
altium-cli pcbdoc rules design.PcbDoc
altium-cli pcbdoc rules design.PcbDoc --kind clearance
altium-cli pcbdoc rule design.PcbDoc "Clearance_1"

# Components
altium-cli pcbdoc components design.PcbDoc
altium-cli pcbdoc components design.PcbDoc --layer top
altium-cli pcbdoc component design.PcbDoc U1

# Routing
altium-cli pcbdoc nets design.PcbDoc
altium-cli pcbdoc tracks design.PcbDoc --net VCC
altium-cli pcbdoc vias design.PcbDoc
altium-cli pcbdoc arcs design.PcbDoc

# Copper
altium-cli pcbdoc polygons design.PcbDoc
altium-cli pcbdoc polygon design.PcbDoc GND_POUR
altium-cli pcbdoc fills design.PcbDoc
altium-cli pcbdoc regions design.PcbDoc

# Board features
altium-cli pcbdoc keepouts design.PcbDoc
altium-cli pcbdoc cutouts design.PcbDoc
altium-cli pcbdoc texts design.PcbDoc
```

### Creating PCB

```bash
# Create new PCB
altium-cli pcbdoc create new.PcbDoc

# Set board outline (rectangular)
altium-cli pcbdoc set-outline-rect design.PcbDoc 0 0 4000 3000

# Set board outline (custom shape)
altium-cli pcbdoc set-outline design.PcbDoc "0,0;4000,0;4000,3000;2000,3000;2000,2500;0,2500"

# Update settings
altium-cli pcbdoc set-settings design.PcbDoc --units mm --layers 4
```

### Design Rules

```bash
# Add rules
altium-cli pcbdoc add-rule design.PcbDoc clearance --value 10 --name "Min_Clearance"
altium-cli pcbdoc add-rule design.PcbDoc width --min 8 --preferred 10 --max 50 --name "Signal_Width"

# Modify rule
altium-cli pcbdoc modify-rule design.PcbDoc "Min_Clearance" --value 15

# Delete rule
altium-cli pcbdoc delete-rule design.PcbDoc "Old_Rule"
```

### Components

```bash
# Place existing component
altium-cli pcbdoc place-component design.PcbDoc U1 1000 2000 --rotation 90 --layer top

# Add new component
altium-cli pcbdoc add-component design.PcbDoc U2 SOIC-8 --designator U2
```

### Routing

```bash
# Add single track
altium-cli pcbdoc add-track design.PcbDoc 0 0 100 100 --net VCC --width 10 --layer top

# Add track path
altium-cli pcbdoc add-track-path design.PcbDoc "0,0;100,0;100,100;200,100" --net VCC --width 10

# Add via
altium-cli pcbdoc add-via design.PcbDoc 500 500 --net VCC --size 40 --hole 20

# Add arc
altium-cli pcbdoc add-arc design.PcbDoc 500 500 100 0 90 --layer top --width 10

# Add net
altium-cli pcbdoc add-net design.PcbDoc VCC_3V3
```

### Copper Pours

```bash
# Add polygon pour
altium-cli pcbdoc add-polygon design.PcbDoc GND --layer top --vertices "0,0;4000,0;4000,3000;0,3000"

# Add fill
altium-cli pcbdoc add-fill design.PcbDoc "0,0;100,0;100,100;0,100" --layer top --net GND

# Add region
altium-cli pcbdoc add-region design.PcbDoc "500,500;600,500;600,600;500,600" --layer top
```

### Board Features

```bash
# Add keepout
altium-cli pcbdoc add-keepout design.PcbDoc "100,100;200,100;200,200;100,200" --layers all

# Add cutout (rectangular)
altium-cli pcbdoc add-cutout design.PcbDoc 500 500 100 100

# Add text
altium-cli pcbdoc add-text design.PcbDoc "REV A" 100 100 --layer silkscreen_top --height 50
```

## Command Details

### add-track

Add single track segment.

```bash
altium-cli pcbdoc add-track <PATH> <X1> <Y1> <X2> <Y2> [OPTIONS]

Options:
  --net <NET>      Net name
  --width <WIDTH>  Track width (mils)
  --layer <LAYER>  Layer name (top, bottom, inner1, etc.)
```

### add-polygon

Add copper pour polygon.

```bash
altium-cli pcbdoc add-polygon <PATH> <NET> [OPTIONS]

Options:
  --layer <LAYER>        Layer name
  --vertices <VERTICES>  Polygon vertices as "x1,y1;x2,y2;..."
  --clearance <MILS>     Pour clearance
```

### add-rule

Add design rule.

```bash
altium-cli pcbdoc add-rule <PATH> <KIND> [OPTIONS]

Arguments:
  <KIND>  Rule kind: clearance, width, via, routing, etc.

Options:
  --name <NAME>       Rule name
  --value <VALUE>     Primary value (mils)
  --min <MIN>         Minimum value
  --max <MAX>         Maximum value
  --preferred <PREF>  Preferred value
  --scope <SCOPE>     Rule scope expression
```

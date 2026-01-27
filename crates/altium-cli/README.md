# altium-cli

CLI tool for inspecting and manipulating Altium Designer files.

## Installation

```bash
cargo install altium-cli
```

## Usage

### Inspect File Structure

View component information in a library or document:

```bash
# Replace with your actual file paths
altium-cli inspect path/to/components.SchLib
altium-cli inspect path/to/footprints.PcbLib
```

### Query Records

Find components, nets, and pins using selector syntax:

```bash
# Find all resistors (replace design.SchDoc with your file)
altium-cli query path/to/design.SchDoc "R*"

# Find components by part number
altium-cli query path/to/components.SchLib "$LM358"

# Find nets by name
altium-cli query path/to/design.SchDoc "~VCC"

# CSS-like queries
altium-cli query path/to/components.SchLib "component[part*=7805]"
```

### Edit Schematic

Modify schematic documents:

```bash
# Move component to new position (coordinates in mils)
altium-cli edit path/to/design.SchDoc -c "move U1 1000 2000" -o output.SchDoc

# Delete component
altium-cli edit path/to/design.SchDoc -c "delete R3" -o output.SchDoc
```

### Output Formats

```bash
# Replace file.SchLib with your actual file path
altium-cli inspect path/to/file.SchLib --json        # Compact JSON
altium-cli inspect path/to/file.SchLib --pretty      # Pretty JSON
```

### Shell Completions

```bash
altium-cli completions bash > altium-cli.bash
source altium-cli.bash
```

## Library Usage

For library usage, see [altium-format](../altium-format/README.md).

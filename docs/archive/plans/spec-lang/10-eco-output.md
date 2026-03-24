# 10 - ECO Output (Text and JSON)

## Location

`crates/altium-format-ops/src/spec/eco.rs`

## Purpose

Render the `EngineeringChangeOrder` as human-readable text (for `altium plan`)
and as structured JSON (for `altium plan --json`).

## Text Format

Per spec-lang.md §12.1:

```
╔══════════════════════════════════════════════════════════════════════╗
║  ENGINEERING CHANGE ORDER                                          ║
║  Library: my-parts.SchLib                                          ║
║  Spec:    my-parts.sym                                     ║
║  Date:    2026-02-26 14:30:00 UTC                                  ║
╚══════════════════════════════════════════════════════════════════════╝

SUMMARY
  Components:  2 add, 1 update, 15 unchanged
  Pins:        8 add, 3 update, 42 unchanged
  Parameters:  4 add, 1 update, 30 unchanged
  Graphics:    6 add, 0 update, 45 unchanged

CHANGES

  + ADD component "R_0603_NEW"
    │ designator: "R?"
    │ description: "New 0603 resistor variant"
    ├── + pin "1" at (-30mil, 0) electrical=passive
    ├── + pin "2" at (30mil, 0) electrical=passive
    ├── + parameter "MFG" text="ACME"
    ├── + rectangle "body" (-20mil,-10mil)–(20mil,10mil) solid
    └── + footprint "0603" [2 pin-pad maps]

  ~ UPDATE component "R_0805"
    │ ~ description: "0805 Resistor" → "0805 Resistor (updated)"
    ├── + pin "3" at (0, 50mil) electrical=passive  [NEW]
    ├── = pin "1" (unchanged)
    ├── = pin "2" (unchanged)
    └── ~ parameter "MFG": text "ACME" → "ACME Inc."

  = 15 components unchanged (not shown)

END OF ECO
```

## Implementation

```rust
impl EngineeringChangeOrder {
    /// Render as human-readable text.
    pub fn render_text(&self) -> String {
        let mut out = String::new();
        self.render_header(&mut out);
        self.render_summary(&mut out);
        self.render_changes(&mut out);
        out.push_str("\nEND OF ECO\n");
        out
    }

    /// Render as JSON.
    pub fn render_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
```

### Header Rendering

```rust
fn render_header(&self, out: &mut String) {
    let border = "═".repeat(70);
    writeln!(out, "╔{border}╗").unwrap();
    writeln!(out, "║  ENGINEERING CHANGE ORDER{:>45}║", "").unwrap();
    writeln!(out, "║  Library: {:<60}║", self.library_path.display()).unwrap();
    writeln!(out, "║  Spec:    {:<60}║", self.spec_path.display()).unwrap();
    writeln!(out, "║  Date:    {:<60}║", format_timestamp(self.timestamp)).unwrap();
    writeln!(out, "╚{border}╝").unwrap();
}
```

### Summary Rendering

One line per entity kind that has any changes:

```
SUMMARY
  Components:  2 add, 1 update, 15 unchanged
```

### Change Rendering

Each change is rendered with a prefix:
- `+` for Add (green)
- `~` for Update (yellow)
- `=` for Unchanged (gray, collapsed)

Children are indented with box-drawing characters:
- `├──` for non-last children
- `└──` for last child
- `│` for continuation lines

Unchanged entities at the same level are collapsed into a count line:
```
  = 15 components unchanged (not shown)
```

### Property Change Rendering

For updates, show old → new:
```
~ description: "0805 Resistor" → "0805 Resistor (updated)"
```

For adds, show property values:
```
│ designator: "R?"
│ description: "New 0603 resistor variant"
```

### Entity-Specific Formatting

**Pin**: `pin "1" at (-30mil, 0) electrical=passive`
**Pad**: `pad "1" at (-0.95mm, -1mm) shape=rectangular 0.6mm×0.7mm`
**Parameter**: `parameter "MFG" text="ACME"`
**Graphic**: `rectangle "body" (-20mil,-10mil)–(20mil,10mil) solid`
**Footprint map**: `footprint "0603" [2 pin-pad maps]`
**Alias**: `alias "R0603"`

## JSON Format

Derive `Serialize` on all ECO types for direct serialization:

```rust
#[derive(Serialize)]
pub struct EngineeringChangeOrder {
    pub library_path: PathBuf,
    pub spec_path: PathBuf,
    pub timestamp: String,           // ISO 8601
    pub summary: EcoSummary,
    pub changes: Vec<EntityChange>,
}
```

The JSON includes all fields for each change, before/after values, and summary
statistics. This enables machine consumption by CI/CD pipelines and review tools.

## Test Strategy

- Render text for add-only ECO
- Render text for mixed ECO (add + update + unchanged)
- Render JSON and verify structure
- Verify summary counts match change list
- Verify box-drawing alignment
- Verify collapsed unchanged count

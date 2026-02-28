# Milestone 7: Text + Annotation Records

**Files**: `crates/altium-format/src/sch_records.rs`

**Depends on**: M2 (Base Types)

## Requirements

Implement parameter-based parsing for text and annotation record types. These records display text content on schematic symbols (labels, designators, parameters, text frames).

## Record Types

### SchLabel (RECORD=4)

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| base | SchGraphicalBase | (flatten) | - |
| text | String | `TEXT` | "" |
| font_id | i32 | `FONTID` | 1 |
| justification | i32 | `JUSTIFICATION` | 0 (BottomLeft) |
| orientation | i32 | `ORIENTATION` | 0 |
| is_mirrored | bool | `ISMIRRORED` | false |
| is_hidden | bool | `ISHIDDEN` | false |

### SchDesignator (RECORD=34)

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| base | SchGraphicalBase | (flatten) | - |
| text | String | `TEXT` | "*" |
| name | String | `NAME` | "Designator" |
| font_id | i32 | `FONTID` | 1 |
| unique_id | String | `UNIQUEID` | "" |
| read_only_state | i32 | `READONLYSTATE` | 1 |
| is_hidden | bool | `ISHIDDEN` | false |
| orientation | i32 | `ORIENTATION` | 0 |
| is_mirrored | bool | `ISMIRRORED` | false |
| justification | i32 | `JUSTIFICATION` | 0 |
| autoposition | i32 | `AUTOPOSITION` | 0 |

### SchParameter (RECORD=41)

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| base | SchGraphicalBase | (flatten) | - |
| text | String | `TEXT` | "*" |
| name | String | `NAME` | "Comment" |
| font_id | i32 | `FONTID` | 1 |
| unique_id | String | `UNIQUEID` | "" |
| read_only_state | i32 | `READONLYSTATE` | 0 |
| is_hidden | bool | `ISHIDDEN` | false |
| orientation | i32 | `ORIENTATION` | 0 |
| is_mirrored | bool | `ISMIRRORED` | false |
| justification | i32 | `JUSTIFICATION` | 0 |
| autoposition | i32 | `AUTOPOSITION` | 0 |
| show_name | bool | `SHOWNAME` | false |
| prop_type | i32 | `PROPTYPE` | 0 |

Note: SchParameter is versatile — used for Comment, Value, and user-defined parameters. The `NAME` field determines the parameter's role.

### SchTextFrame (RECORD=28)

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| base | SchGraphicalBase | (flatten) | - |
| corner | CoordPoint | `CORNER.X`/`_FRAC`, `CORNER.Y`/`_FRAC` | - |
| text | String | `TEXT` | "" |
| font_id | i32 | `FONTID` | 1 |
| alignment | i32 | `ALIGNMENT` | 0 |
| word_wrap | bool | `WORDWRAP` | false |
| is_solid | bool | `ISSOLID` | false |
| line_width | i32 | `LINEWIDTH` | 1 |
| text_margin | i32 | `TEXTMARGIN` | 0 |
| show_border | bool | `SHOWBORDER` | true |
| transparent | bool | `TRANSPARENT` | true |
| clip_to_rect | bool | `CLIPTORECT` | false |

## Acceptance Criteria

- All text record types parse via `#[derive(FromParams)]`
- SchDesignator and SchParameter handle all known parameter keys
- Font IDs reference the font table parsed in FileHeader (M3) — validated at assembly time (M11), not during individual record parsing
- `altium validate` progresses past text records in test files

## Tests

- **Test files**: `crates/altium-format/src/sch_records.rs` (inline `#[cfg(test)]`)
- **Test type**: unit + integration
- **Backing**: doc-derived (docs/schlib/record-types.md)
- **Scenarios**:
  - Normal: SchLabel with text and font
  - Normal: SchDesignator with default "Designator" name
  - Normal: SchParameter with NAME="Comment"
  - Normal: SchTextFrame with word wrap and border
  - Edge: hidden parameter (ISHIDDEN=T)
  - Edge: rotated/mirrored text (ORIENTATION bitmask)
  - Error: missing required TEXT field

## Code Intent

- Add to `crates/altium-format/src/sch_records.rs`:
  - `SchLabel`, `SchDesignator`, `SchParameter`, `SchTextFrame` structs with `#[derive(FromParams)]`
  - Add variants to `SchRecord` enum
  - Add arms to `dispatch_record()` match
- Note: the exact set of parameters per record may expand during red/green testing against real files — add fields as `assert_exhausted` reveals them

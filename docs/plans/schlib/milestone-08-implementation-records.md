# Milestone 8: Implementation Records

**Files**: `crates/altium-format/src/sch_records.rs`

**Depends on**: M2 (Base Types)

## Requirements

Implement parameter-based parsing for implementation/model records. These records define footprint assignments and pin-to-pad mappings — the bridge between schematic symbols and PCB footprints.

## Record Types

### SchImplementationList (RECORD=44)

Container that holds Implementation entries for a component.

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| base | SchPrimitiveBase | (flatten) | - |

Note: Uses SchPrimitiveBase (not SchGraphicalBase) — no location or color.

### SchImplementation (RECORD=45)

A single footprint assignment.

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| base | SchPrimitiveBase | (flatten) | - |
| model_name | String | `MODELNAME` | "" |
| model_type | String | `MODELTYPE` | "" |
| datafile_count | i32 | `DATAFILECOUNT` | 0 |
| model_datafile_entity | String | `MODELDATAFILEENTITY0` | "" |
| model_datafile_kind | String | `MODELDATAFILEKIND0` | "" |
| is_current | bool | `ISCURRENT` | false |
| datalinks_locked | bool | `DATALINKSLOCKED` | false |
| database_model | bool | `DATABASEMODEL` | false |
| description | String | `DESCRIPTION` | "" |
| unique_id | String | `UNIQUEID` | "" |
| integrate_model | bool | `INTEGRATEMODEL` | false |

### SchImplementationMap (RECORD=46)

Container for pin-to-pad mapping entries.

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| base | SchPrimitiveBase | (flatten) | - |

### SchMapDefiner (RECORD=47)

A single pin-to-pad mapping.

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| base | SchPrimitiveBase | (flatten) | - |
| owner_index | i32 | `OWNERINDEX` | -1 |
| target_name | String | `TARGETNAME` | "" |
| source_pin_name | String | `SOURCEPINNAME` | "" |
| source_pin_uniqueid | String | `SOURCEPINUNIQUEID` | "" |
| target_pin_name | String | `TARGETPINNAME` | "" |

### SchImplementationParameters (RECORD=48)

Parameters for an implementation (e.g., footprint filter rules).

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| base | SchPrimitiveBase | (flatten) | - |
| text | String | `TEXT` | "" |
| name | String | `NAME` | "" |
| unique_id | String | `UNIQUEID` | "" |

## OwnerIndex Relationships

```
SchComponent (index 0)
  └── SchImplementationList (OwnerIndex → 0)
        └── SchImplementation (OwnerIndex → impl_list_index)
              ├── SchImplementationMap (OwnerIndex → impl_index)
              │     └── SchMapDefiner (OwnerIndex → map_index)
              └── SchImplementationParameters (OwnerIndex → impl_index)
```

## Acceptance Criteria

- All implementation records parse via `#[derive(FromParams)]`
- Implementation records use SchPrimitiveBase (not SchGraphicalBase)
- OwnerIndex chain preserved correctly
- Model name and type parsed for footprint identification
- `altium validate` progresses past implementation records

## Tests

- **Test files**: `crates/altium-format/src/sch_records.rs` (inline `#[cfg(test)]`)
- **Test type**: unit + integration
- **Backing**: doc-derived (docs/schlib/record-types.md)
- **Scenarios**:
  - Normal: ImplementationList with nested Implementation
  - Normal: Implementation with model name and datafile
  - Normal: MapDefiner with source/target pin names
  - Edge: multiple implementations (multiple footprint options)
  - Edge: empty ImplementationMap (no pin mappings)

## Code Intent

- Add to `crates/altium-format/src/sch_records.rs`:
  - `SchImplementationList`, `SchImplementation`, `SchImplementationMap`, `SchMapDefiner`, `SchImplementationParameters` structs
  - ImplementationList, ImplementationMap use SchPrimitiveBase only (no location/color)
  - Add variants to `SchRecord` enum
  - Add arms to `dispatch_record()` match
- Note: the exact parameter set may expand during red/green validation — Implementation records in particular tend to have many optional vault/database parameters

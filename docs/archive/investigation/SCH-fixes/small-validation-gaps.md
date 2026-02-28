# Small Validation Gaps Research Report

Research into remaining small validation gaps blocking ~24 schematic files total.

**Note on task description**: The original task had some record number/name mismatches.
Corrected mappings used below (verified against `SchRecordType` enum in `altium-format-types/src/sch.rs`).

---

## 1. RECORD=31 (Sheet) -- Missing Template/Release Vault GUIDs (6 files)

### Current Implementation
- **File**: `crates/altium-format/src/sch_records.rs:1386` (`SchSheet`)
- **Parser**: Manual `from_params` at line 1392, builds `SchDisplaySettings`
- The `SchDisplaySettings` struct stores sheet display properties

### C# Reference
- **Export**: `FileFormatV5.cs:3206` -- `ExportDocument()` called by `ExportSheet()`
- The export writes these vault/template fields:
  - `TemplateVaultGUID` (line 3238) -- `Export_String`
  - `TemplateItemGUID` (line 3239) -- `Export_String`
  - `TemplateRevisionGUID` (line 3240) -- `Export_String`
  - `TemplateVaultHRID` (line 3241) -- `Export_String`
  - `TemplateRevisionHRID` (line 3242) -- `Export_String`
  - `ReleaseVaultGUID` (line 3245) -- `Export_DynamicString`
  - `ReleaseItemGUID` (line 3246) -- `Export_DynamicString`
  - `ItemRevisionGUID` (line 3247) -- `Export_DynamicString`
  - `PropsVaultGUID` (line 3248) -- `Export_DynamicString`
  - `PropsRevisionGUID` (line 3249) -- `Export_DynamicString`

### Missing Fields
| Parameter Key | Type | Default | Constant |
|---|---|---|---|
| `TemplateVaultGUID` | `String` | `""` | `vault::TEMPLATE_VAULT_GUID` |
| `TemplateItemGUID` | `String` | `""` | `vault::TEMPLATE_ITEM_GUID` |
| `TemplateRevisionGUID` | `String` | `""` | `vault::TEMPLATE_REVISION_GUID` |
| `TemplateVaultHRID` | `String` | `""` | `vault::TEMPLATE_VAULT_HRID` |
| `TemplateRevisionHRID` | `String` | `""` | `vault::TEMPLATE_REVISION_HRID` |
| `ReleaseVaultGUID` | `String` | `""` | `vault::RELEASE_VAULT_GUID` |
| `ReleaseItemGUID` | `String` | `""` | `vault::RELEASE_ITEM_GUID` |
| `ItemRevisionGUID` | `String` | `""` | `vault::ITEM_REVISION_GUID` |
| `PropsVaultGUID` | `String` | `""` | `vault::PROPS_VAULT_GUID` |
| `PropsRevisionGUID` | `String` | `""` | `vault::PROPS_REVISION_GUID` |

### Recommended Fix
Add these fields to `SchDisplaySettings` (or `SchSheet` directly) and consume them
in the manual `from_params` implementation at `sch_records.rs:1392`.
All use `remove_optional` with `String` type, default empty string.
Constants already exist in `altium-format-types/src/constants/vault.rs`.

### Semantics
These are Altium 365 / Vault integration fields. They store GUIDs linking the sheet
to vault-managed templates and release states. Most files won't have them, but files
created via Altium 365 workflows will. All are simple string fields that default to empty.

---

## 2. RECORD=45 (Implementation) -- Missing Indexed Datafile Fields (5 files)

### Current Implementation
- **File**: `crates/altium-format/src/sch_records.rs:1283` (`SchImplementation`)
- Currently has hard-coded fields for index 0 only:
  - `model_datafile0` at line 1302: `#[param(key = "ModelDatafile0")]`
  - `model_datafile_entity0` at line 1304: `#[param(key = "ModelDatafileEntity0")]`
  - `model_datafile_kind0` at line 1306: `#[param(key = "ModelDatafileKind0")]`

### C# Reference
- **Export**: `FileFormatV5.cs:2510` -- `ExportImplementation()`
- Uses a loop (lines 2524-2531) based on `DatafileCount`:
  ```csharp
  int count = datafileLinkList.GetCount();
  for (int i = 0; i < count; i++)
  {
      string text = i.ToString();
      argSerializer.Export_DynamicString(item.GetState_Location(), "ModelDatafile" + text);
      argSerializer.Export_DynamicString(item.GetState_EntityName(), "ModelDatafileEntity" + text);
      argSerializer.Export_DynamicString(item.GetState_FileKind(), "ModelDatafileKind" + text);
  }
  ```
- **Import**: `FileFormatV5.cs:2589-2610` -- loops over `DatafileCount`, reading `ModelDatafile{i}`, `ModelDatafileEntity{i}`, `ModelDatafileKind{i}`

### Missing Fields
When `DatafileCount > 1`, files will have `ModelDatafile1`, `ModelDatafileEntity1`, `ModelDatafileKind1`, etc.
Our current code only handles index 0.

### Recommended Fix
Replace the three hard-coded index-0 fields with a dynamically-indexed structure:
```rust
pub struct ModelDatafileLink {
    pub location: String,    // ModelDatafile{i}
    pub entity_name: String, // ModelDatafileEntity{i}
    pub file_kind: String,   // ModelDatafileKind{i}
}
```
In `SchImplementation`, replace the three `model_datafile0/entity0/kind0` fields with:
```rust
pub datafile_links: Vec<ModelDatafileLink>,
```
And implement manual `from_params`/`to_params` that loops `0..datafile_count`,
consuming `ModelDatafile{i}`, `ModelDatafileEntity{i}`, `ModelDatafileKind{i}` for each.

**Alternative simpler fix**: Keep deriving FromParams/ToParams but add fields for index 1:
```rust
#[param(key = "ModelDatafile1", default = String::new())]
pub model_datafile1: String,
#[param(key = "ModelDatafileEntity1", default = String::new())]
pub model_datafile_entity1: String,
#[param(key = "ModelDatafileKind1", default = String::new())]
pub model_datafile_kind1: String,
```
This is less clean but handles the common case. In practice, `DatafileCount` rarely exceeds 2.
The proper fix with a Vec is better for correctness.

### Semantics
Each `ModelDatafileLink` entry represents a model file associated with the implementation
(e.g., a `.STEP` 3D model file, a simulation model, etc.). The `entity_name` defaults to
the `ModelName` if empty (see C# import line 2602). The `file_kind` describes the type
(e.g., `"3DModel"`, `"SimModel"`).

---

## 3. RECORD=27 (Wire) -- Missing UNDERLINECOLOR (4 files)

### Current Implementation
- **File**: `crates/altium-format/src/sch_records.rs:1487` (`SchWire`)
- Has: `color`, `line_width`, `line_style`, `vertices`, `unique_id`
- Missing: `underline_color`

### C# Reference
- **Export**: `FileFormatV5.cs:1255` -- `ExportWire()`
  ```csharp
  argSerializer.Export_Color(schDataWire.GetUnderlineColor(), "UnderlineColor");
  ```
- **Import**: `FileFormatV5.cs:1282` -- default `0u` (Color, u32)
- **Interface**: `ISchDataWire` has `GetUnderlineColor()`/`SetUnderlineColor(uint)`

### Missing Field
| Parameter Key | Type | Default | Constant |
|---|---|---|---|
| `UnderlineColor` | `Color` (Win32 COLORREF u32) | `Color::BLACK` (0) | `text::UNDERLINE_COLOR` |

### Recommended Fix
Add to `SchWire` struct:
```rust
#[param(key = UNDERLINE_COLOR, default = Color::BLACK)]
pub underline_color: Color,
```
Import `UNDERLINE_COLOR` from `constants::text`.

### Also Missing from Wire
The C# export also writes:
- `AssignedInterface` (DynamicString, default empty) -- for harness interface assignment
- `AssignedInterfaceSignal` (DynamicString, default empty) -- for harness signal assignment

These may also need to be added. Constant: `FileFormatConsts.cs:129-131`.

### Semantics
`UnderlineColor` is the color used to draw the underline decoration on wires when
net class coloring or similar visual cues are active. It's a standard Win32 COLORREF
in BGR format (0x00BBGGRR).

---

## 4. RECORD=34 (Designator) -- Missing NOTALLOWLIBRARYSYNCHRONIZE (3 files)

### Current Implementation
- **File**: `crates/altium-format/src/sch_records.rs:1134` (`SchDesignator`)
- The Designator is serialized via `ExportDesignator` which calls `ExportParameter` (line 1452/1461)
- So the Designator effectively has all the fields of `SchParameter` (RECORD=41)
- `SchParameter` at line 1174 already has `not_allow_library_synchronize` (line 1203)
- But `SchDesignator` does NOT have it -- it has a subset of Parameter fields

### C# Reference
- **Export**: `FileFormatV5.cs:1445` -- `ExportDesignator()` calls `ExportParameter()`
- `ExportParameter` (line 1339) writes `NotAllowLibrarySynchronize` at line 1364
- **Import**: `ImportDesignator` (line 1464) calls `ImportParameter` which reads it at line 1422

### Missing Fields
The Designator is serialized through `ExportParameter`, so it should support ALL fields
that `SchParameter` has. Currently `SchDesignator` is missing several fields that
`SchParameter` has:

| Parameter Key | Type | Default | Constant |
|---|---|---|---|
| `NotAllowLibrarySynchronize` | `bool` | `false` | `vault::NOT_ALLOW_LIBRARY_SYNCHRONIZE` |
| `NotAllowDatabaseSynchronize` | `bool` | `false` | `vault::NOT_ALLOW_DATABASE_SYNCHRONIZE` |
| `Description` | `String` | `""` | `text::DESCRIPTION` |
| `ParamType` | `ParameterType` | `String` | `record_structure::PARAM_TYPE` |
| `TextHorzAnchor` | `TextHorzAnchor` | `None` | `text::TEXT_HORZ_ANCHOR` |
| `TextVertAnchor` | `TextVertAnchor` | `None` | `text::TEXT_VERT_ANCHOR` |
| `IsImageParameter` | `bool` | `false` | `record_structure::IS_IMAGE_PARAMETER` |

The 3 blocking files specifically hit `NotAllowLibrarySynchronize`.

### Recommended Fix
Add the missing fields to `SchDesignator`. At minimum, add `NotAllowLibrarySynchronize`.
For full correctness, add all fields that `SchParameter` has that `SchDesignator` lacks,
since C# serializes Designator through the same `ExportParameter` path.

```rust
#[param(key = NOT_ALLOW_LIBRARY_SYNCHRONIZE, default = false)]
pub not_allow_library_synchronize: bool,
#[param(key = NOT_ALLOW_DATABASE_SYNCHRONIZE, default = false)]
pub not_allow_database_synchronize: bool,
```

---

## 5. RECORD=26 (Bus) -- Missing UNDERLINECOLOR (2 files)

### Current Implementation
- **File**: `crates/altium-format/src/sch_records.rs:1503` (`SchBus`)
- Has: `color`, `line_width`, `vertices`, `unique_id`
- Missing: `underline_color`

### C# Reference
- **Export**: `FileFormatV5.cs:1297` -- `ExportBus()`
  ```csharp
  argSerializer.Export_Color(schDataBus.GetUnderlineColor(), "UnderlineColor");
  ```
- **Import**: `FileFormatV5.cs:1324` -- default `0u`

### Missing Field
| Parameter Key | Type | Default | Constant |
|---|---|---|---|
| `UnderlineColor` | `Color` (Win32 COLORREF) | `Color::BLACK` (0) | `text::UNDERLINE_COLOR` |

### Recommended Fix
Same as Wire. Add to `SchBus`:
```rust
#[param(key = UNDERLINE_COLOR, default = Color::BLACK)]
pub underline_color: Color,
```

### Also Missing from Bus
Like Wire, Bus also has:
- `AssignedInterface` (DynamicString, default empty)
- `AssignedInterfaceSignal` (DynamicString, default empty)

---

## 6. RECORD=25 (NetLabel) -- Missing SelectionMemory (1 file)

### Current Implementation
- **File**: `crates/altium-format/src/sch_records.rs:1519` (`SchNetLabel`)
- The `SchPrimitiveBase` at line 116 does NOT include `SelectionMemory`

### C# Reference
- **Export**: `FileFormatV5.cs:5076` in `ExportGraphicalObject()`:
  ```csharp
  argSerializer.Export_Byte(schDataGraphicalObject.GetSelectionMemoryFlags(), "SelectionMemory");
  ```
- **Import**: `FileFormatV5.cs:5094` -- default `0` (byte)
- `SelectionMemory` is part of `ExportGraphicalObject` which is called for ALL graphical records

### Missing Field
| Parameter Key | Type | Default | Constant |
|---|---|---|---|
| `SelectionMemory` | `u8` (byte) | `0` | `locking::SELECTION_MEMORY` or `record_structure::SELECTION_MEMORY` |

### Recommended Fix
Since `SelectionMemory` is exported via `ExportGraphicalObject` for ALL graphical records,
it should be added to `SchPrimitiveBase` (which all records flatten):
```rust
#[param(key = SELECTION_MEMORY, default = 0u8)]
pub selection_memory: u8,
```
Import `SELECTION_MEMORY` from appropriate constants module.

**IMPORTANT**: Adding this to `SchPrimitiveBase` fixes it for ALL records at once,
not just NetLabel. The reason only 1 file hits this is because most files have
`SelectionMemory=0` which is the default -- but when a user selects objects and saves,
the selection state is persisted and this field becomes non-zero.

### Semantics
`SelectionMemory` stores a bitmask of selection state flags. From the C# interface
`ISchDataGraphicalObject.GetSelectionMemoryFlags()` / `SetSelectionMemoryFlags(byte)`.
This is a per-object selection state that persists across save/load.

---

## 7. RECORD=15 (SheetSymbol) -- Missing ShowHiddenFields (1 file)

### Current Implementation
- **File**: `crates/altium-format/src/sch_records.rs:1704` (`SchSheetSymbol`)
- Has: `base`, `location`, `x_size`, `y_size`, `line_width`, `color`, `area_color`,
  `is_solid`, `unique_id`, `symbol_type`, `sheet_name`, `file_name`
- Missing: `show_hidden_fields`

### C# Reference
- **Export**: `FileFormatV5.cs:2217` in `ExportSheetSymbol()`:
  ```csharp
  argSerializer.Export_Boolean(schDataSheetSymbol.GetShowHiddenFields(), "ShowHiddenFields");
  ```
- **Import**: `FileFormatV5.cs:2238` -- default `false`
- Also missing from our struct but present in C#:
  - `DesignItemId` (DynamicString, default empty)
  - `SourceLibraryName` (DynamicString, default empty)
  - `VaultGUID` (DynamicString, default empty)
  - `ItemGUID` (DynamicString, default empty)
  - `RevisionGUID` (DynamicString, default empty)
  - `RevisionName` (DynamicString, default empty)

### Missing Field (blocking)
| Parameter Key | Type | Default | Constant |
|---|---|---|---|
| `ShowHiddenFields` | `bool` | `false` | `component::SHOW_HIDDEN_FIELDS` |

### Recommended Fix
Add to `SchSheetSymbol`:
```rust
#[param(key = SHOW_HIDDEN_FIELDS, default = false)]
pub show_hidden_fields: bool,
```
Already imported: `SHOW_HIDDEN_FIELDS` is in the `component` constants import.

For full correctness, also add the vault fields:
```rust
#[param(key = DESIGN_ITEM_ID, default = String::new())]
pub design_item_id: String,
#[param(key = SOURCE_LIBRARY_NAME, default = String::new())]
pub source_library_name: String,
#[param(key = VAULT_GUID, default = String::new())]
pub vault_guid: String,
#[param(key = ITEM_GUID, default = String::new())]
pub item_guid: String,
#[param(key = REVISION_GUID, default = String::new())]
pub revision_guid: String,
#[param(key = "RevisionName", default = String::new())]
pub revision_name: String,
```

---

## 8. RECORD=225 (Blanket) -- Missing COLLAPSED (2 files)

### Current Implementation
- **File**: `crates/altium-format/src/sch_records.rs:1877` (`SchBlanket`)
- Has: `base`, `location`, `corner`, `color`, `area_color`, `line_style`, `line_style_ext`,
  `line_width`, `vertices`, `unique_id`
- Missing: `collapsed`
- Note: `SchCompileMask` (RECORD=211) at line 1856 already has `collapsed`

### C# Reference
- **Export**: `FileFormatV5.cs:2751` in `ExportBlanket()`:
  ```csharp
  argSerializer.Export_Boolean(schDataBlanket.GetCollapsed(), "Collapsed");
  ```
- **Import**: default `false`

### Missing Field
| Parameter Key | Type | Default | Constant |
|---|---|---|---|
| `Collapsed` | `bool` | `false` | `record_structure::COLLAPSED` |

### Recommended Fix
Add to `SchBlanket`:
```rust
#[param(key = COLLAPSED, default = false)]
pub collapsed: bool,
```
The `COLLAPSED` constant is already imported in `sch_records.rs` (line 66 from `record_structure`).

### Serialization Order
From the C# export, the order is: Location, Corner, LineWidth, Color, AreaColor, **Collapsed**,
LineStyle, Vertices, LineStyleExt, UniqueID. Place `collapsed` after `area_color`.

---

## Summary -- Priority Order by File Count

| Priority | Record | Missing Field(s) | Files Blocked | Fix Complexity |
|---|---|---|---|---|
| 1 | RECORD=31 Sheet | Template/Release vault GUIDs (10 fields) | 6 | Medium (manual parser) |
| 2 | RECORD=45 Implementation | ModelDatafile{N}/Entity{N}/Kind{N} loop | 5 | Medium (indexed loop) |
| 3 | RECORD=27 Wire | UnderlineColor | 4 | Simple (add field) |
| 4 | RECORD=34 Designator | NotAllowLibrarySynchronize (+ others) | 3 | Simple (add fields) |
| 5 | RECORD=26 Bus | UnderlineColor | 2 | Simple (add field) |
| 6 | RECORD=225 Blanket | Collapsed | 2 | Simple (add field) |
| 7 | RECORD=25 NetLabel (via base) | SelectionMemory | 1 | Simple (add to base) |
| 8 | RECORD=15 SheetSymbol | ShowHiddenFields | 1 | Simple (add field) |

**Total unique files blocked**: ~24 (some files may have multiple issues)

### Quick Wins (simple field additions)
- Wire: add `underline_color` field
- Bus: add `underline_color` field
- Designator: add `not_allow_library_synchronize` (and siblings)
- Blanket: add `collapsed` field
- SheetSymbol: add `show_hidden_fields` field
- SchPrimitiveBase: add `selection_memory` field (fixes NetLabel + all future records)

### Medium Complexity
- Sheet: add 10 vault/template GUID fields to manual `from_params` parser
- Implementation: refactor to support indexed ModelDatafile loop (or add index 1 fields)

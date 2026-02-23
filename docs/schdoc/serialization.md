# SchDoc Serialization (Round-Trip Write)

How to serialize an in-memory `SchDoc` back to a byte-identical CFB file.

Sources: `SchDataExporterSheetV5.cs`, `SchDataExporterDocumentV5.cs`, `SchDataExporterBaseV5.cs`,
`FileFormatV5.cs`, `SchDataSerializerParam.cs`, `SchDataSerializer.cs`, `SchDataObjectComparator.cs`,
`FileFormatConsts.cs`, `BinaryFileCode.cs` (all in AD26-dotnet/Altium.Sch.DataModel/).

---

## 1. Architecture: The Write Pipeline

The write pipeline is simpler than SchLib because SchDoc is a flat file:

```
SchDoc (in-memory)
  -> serialize FileHeader stream (header + sheet + font table + ALL content records)
  -> serialize Storage stream (embedded images, zlib-compressed)
  -> serialize Additional stream (RECORD=225 dashed rectangles)
  -> serialize optional streams (ObjectDefinitions, ReuseBlocks, etc.)
  -> write CFB container to disk
```

Corresponding C# entry points:

- **`SchDataExporterSheetV5`** extends `SchDataExporterDocumentV5` extends `SchDataExporterBaseV5`
- The `Run()` pipeline in `SchDataExporterBaseV5`:

```
1. InitializeForSaving()              -- fix up font IDs
2. FillBaseAndAdditionalWarehouses()  -- collect records into flat lists
3. FillExtendedWarehouse()            -- collect embedded images
4. FixBaseWarehouse()                 -- post-processing (MoveSpecialObjectsToTop, AddAutoJunctions)
5. WriteBaseWarehouse()               -- -> FileHeader stream
6. WriteExtendedWarehouse()           -- -> Storage stream
7. WriteAdditionalWarehouse()         -- -> Additional stream
8. WriteOptionalStreams()             -- -> ObjectDefinitions, ReuseBlocks, etc.
9. FinalizeForSaving()
```

### SchDoc-specific FixBaseWarehouse

SchDoc calls `MoveSpecialObjectsToTop()` + `AddAutoJunctions()` during warehouse fixing.
These are NOT called for SchLib.

---

## 2. Sparse Saving -- The Two-Tier Export System

Identical to SchLib. See `docs/schlib/serialization.md` Section 2 for full details.

**Quick summary:**

| Tier | Methods | Behavior |
|------|---------|----------|
| T1 (default-skipping) | `Export_Boolean`, `Export_Byte`, `Export_LongInt`, `Export_Color`, etc. | Omit when value == default (0, false, empty) |
| T2 (always-write) | `Export_Boolean_WithDefault`, `Export_Byte_WithDefault`, `Export_LongInt_WithDefault` | Always emit |
| COND (call-site) | Explicit `if` guards | Only emitted when condition is true |

---

## 3. Parameter Encoding Rules

Identical to SchLib encoding. See `docs/schlib/serialization.md` Section 3.

Key facts:
- Pipe-delimited `|KEY=VALUE|`, NUL-terminated, Windows-1252
- `%UTF8%` prefix for Unicode keys
- Booleans: `T`/`F`
- Colors: Win32 COLORREF as decimal i32
- Coords: DXP fractional encoding (integer + `_FRAC`)
- Extended records: `RECORD=254` + `RECORDEX=<actual_value>` for types >= 256

---

## 4. Parameter Order Per Record Type

Parameter order is **explicitly hardcoded** in `FileFormatV5.cs`. Each export method
calls serializer methods in a precise order.

### Common Prefix: ExportDataObject

All objects start with this base export (8 parameters):

```
# Field                              Key                              Tier  Notes
1 owner_index                        OwnerIndex                       T1    Skipped if 0
2 is_not_accessible                  IsNotAccesible                   T1    Inverted; skipped if false
3 owner_index_additional_list        OwnerIndexAdditionalList          T1    Skipped if false
4 index_in_sheet                     IndexInSheet                     T1    Skipped if 0
5 ignore_on_load                     IgnoreOnLoad                     COND  Only if true
6 wiring_diagram_origin_unique_id    WiringDiagramOriginUniqueId      COND  Only for containers, if non-empty
7 is_schematic_block_object          IsSchematicBlockObject            T1    Skipped if false
8 unique_id_in_reuse_block           UniqueIDInReuseBlock             COND  Only if non-empty
```

### Common Prefix: ExportGraphicalObject (extends ExportDataObject)

Appends 5 more parameters after ExportDataObject:

```
# Field                       Key                        Tier  Notes
9  owner_part_id               OwnerPartId                T1    (i16 export)
10 owner_part_display_mode     OwnerPartDisplayMode        T1    (byte export)
11 selection_memory             SelectionMemory             T1    (byte export)
12 union_index                  UnionIndex                  T1    Skipped if 0
13 graphically_locked           GraphicallyLocked           T1    Skipped if false
```

---

### RECORD=31: SchSheet (ExportSheet -> ExportDocument)

The sheet record contains the font table and all document-level settings. It is
exported via `ExportDocument()` which calls `ExportStyleAndFontTable()` first.

```
# Field                        Key                              Tier  Notes
-- [ExportStyleAndFontTable]
1  font_id_count               FontIdCount                      T1    (i16 export)
   -- For each font i (1-based), only fonts with save flag set:
   size{i}                     Size{i}                          T1    (i16)
   rotation{i}                 Rotation{i}                      T1    (i16)
   underline{i}                Underline{i}                     T1    bool
   italic{i}                   Italic{i}                        T1    bool
   bold{i}                     Bold{i}                          T1    bool
   strikeout{i}                StrikeOut{i}                     T1    bool
   font_name{i}                FontName{i}                      STR

-- [ExportDocument fields]
2  use_mbcs                    UseMBCS                          T1    Always true
3  is_boc                      IsBOC                            T1    Always true
4  hotspot_grid_on             HotSpotGridOn                    T1    bool
5  hotspot_grid_size           HotSpotGridSize                  COORD
6  sheet_style                 SheetStyle                       T1    (byte export)
7  system_font                 SystemFont                       T1    (FontID)
8  document_border_style       DocumentBorderStyle              T1    enum
9  workspace_orientation       WorkspaceOrientation             T1    enum
10 border_on                   BorderOn                         T1    bool
11 title_block_on              TitleBlockOn                     T1    bool
12 sheet_number_space_size     SheetNumberSpaceSize             T1    (i32)
13 color                       Color                            COLOR
14 area_color                  AreaColor                        COLOR
15 snap_grid_on                SnapGridOn                       T1    bool
16 snap_grid_size              SnapGridSize                     COORD
17 visible_grid_on             VisibleGridOn                    T1    bool
18 visible_grid_size           VisibleGridSize                  COORD
19 custom_x                    CustomX                          COORD
20 custom_y                    CustomY                          COORD
21 use_custom_sheet            UseCustomSheet                   T1    bool
22 show_hidden_pins            ShowHiddenPins                   T1    bool
23 reference_zones_on          ReferenceZonesOn                 T1    Inverted (export !value)
24 custom_x_zones              CustomXZones                     T1    (i32)
25 custom_y_zones              CustomYZones                     T1    (i32)
26 custom_margin_width         CustomMarginWidth                COORD
27 show_template_graphics      ShowTemplateGraphics             T1    bool
28 template_file_name          TemplateFileName                 STR
29 template_vault_guid         TemplateVaultGUID                STR
30 template_item_guid          TemplateItemGUID                 STR
31 template_revision_guid      TemplateRevisionGUID             STR
32 template_vault_hrid         TemplateVaultHRID                STR
33 template_revision_hrid      TemplateRevisionHRID             STR
34 display_unit                Display_Unit                     T1    enum
35 reference_zone_style        ReferenceZoneStyle               T1    enum
36 release_vault_guid          ReleaseVaultGUID                 T1    DynamicString
37 release_item_guid           ReleaseItemGUID                  T1    DynamicString
38 item_revision_guid          ItemRevisionGUID                 T1    DynamicString
39 props_vault_guid            PropsVaultGUID                   T1    DynamicString
40 props_revision_guid         PropsRevisionGUID                T1    DynamicString
41 file_version_info           FileVersionInfo                  T1    DynamicString
```

**Note:** `ReferenceZonesOn` is inverted -- the export writes `!value`.

### RECORD=39: SchTemplate

```
# Field          Key            Tier
-- [ExportGraphicalObject]
1  file_name      FileName       STR
```

### RECORD=1: SchComponent

Same as SchLib. See `docs/schlib/serialization.md` Section 4, RECORD=1.

### RECORD=2: SchPin (Text Parameter Format)

Same as SchLib. See `docs/schlib/serialization.md` Section 4, RECORD=2.

**Critical difference:** In SchDoc, pins are ALWAYS text parameter blocks (flags=0x00).
In SchLib, pins are binary blocks (flags=0x01). The parameter text format is identical.

---

### RECORD=27: SchWire

```
# Field                    Key                              Tier  Notes
-- [ExportGraphicalObject]
1  line_width               LineWidth                        T1    (TSize enum)
2  color                    Color                            COLOR
3  underline_color          UnderlineColor                   COLOR
4  unique_id                UniqueID                         T1    DynamicString
5  assigned_interface       AssignedInterface                T1    DynamicString
6  assigned_interface_sig   AssignedInterfaceSignal           T1    DynamicString
7  vertices                 LOCATIONCOUNT + X{1..N} + Y{1..N}  INDEXED  (via ExportToFile)
```

**Note:** Wire exports UniqueID BEFORE vertices. Compare with Bus which exports
UniqueID AFTER vertices.

### RECORD=26: SchBus

```
# Field                    Key                              Tier  Notes
-- [ExportGraphicalObject]
1  line_width               LineWidth                        T1    (TSize enum)
2  color                    Color                            COLOR
3  underline_color          UnderlineColor                   COLOR
4  vertices                 LOCATIONCOUNT + X{1..N} + Y{1..N}  INDEXED  (via ExportToFile)
5  unique_id                UniqueID                         T1    DynamicString
6  assigned_interface       AssignedInterface                T1    DynamicString
7  assigned_interface_sig   AssignedInterfaceSignal           T1    DynamicString
```

**Note:** Bus exports vertices BEFORE UniqueID (opposite of Wire).

### RECORD=25: SchNetLabel

```
# Field          Key            Tier
-- [ExportGraphicalObject]
1  location.x     Location.X     COORD
2  location.y     Location.Y     COORD
3  orientation    Orientation    T1    (RotationBy90 enum)
4  justification  Justification  T1    (TextJustification enum)
5  color          Color          COLOR
6  font_id        FontID         T1
7  text           Text           T1    DynamicString
8  is_mirrored    IsMirrored     T1    bool
9  unique_id      UniqueID       T1    DynamicString
```

### RECORD=17: SchPowerObject

```
# Field                    Key                              Tier  Notes
-- [ExportGraphicalObject]
1  style                    Style                            T1    (PowerObjectStyle enum)
2  show_net_name            ShowNetName                      T2    Export_Boolean_WithDefault
3  location.x               Location.X                       COORD
4  location.y               Location.Y                       COORD
5  orientation              Orientation                      T1    (RotationBy90)
6  color                    Color                            COLOR
7  font_id                  FontID                           COND  Only if GetFontID() != 0
8  text                     Text                             T1    DynamicString
9  is_cross_sheet_connector IsCrossSheetConnector            T1    bool
10 unique_id                UniqueID                         T1    DynamicString
11 object_definition_id     ObjectDefinitionId               T1    DynamicString
12 default_cross_sheet_hidden DefaultCrossSheetHidden        COND  (via ExportDefaultCrossSheetHidden)
```

### RECORD=18: SchPort

```
# Field                    Key                              Tier  Notes
-- [ExportGraphicalObject]
1  style                    Style                            T1    (PortArrowStyle enum)
2  io_type                  IOType                           T1    (PortIO enum)
3  alignment                Alignment                        T1    (HorizontalAlign enum)
4  width                    Width                            COORD
5  location.x               Location.X                       COORD
6  location.y               Location.Y                       COORD
7  color                    Color                            COLOR
8  font_id                  FontID                           T1
9  area_color               AreaColor                        COLOR
10 text_color               TextColor                        COLOR
11 name                     Name                             T1    DynamicString
12 harness_type             HarnessType                      T1    DynamicString
13 unique_id                UniqueID                         T1    DynamicString
14 height                   Height                           COORD
15 border_width             BorderWidth                      T1    (TSize enum)
16 auto_size                AutoSize                         T1    bool
17 object_definition_id     ObjectDefinitionId               T1    DynamicString
18 port_name_is_hidden      PortNameIsHidden                 T1    Inverted from ShowNetName
19 default_cross_sheet_hidden DefaultCrossSheetHidden        COND  (via ExportDefaultCrossSheetHidden)
```

### RECORD=22: SchNoConnect (NoERC)

```
# Field                         Key                              Tier  Notes
-- [ExportGraphicalObject]
1  location.x                    Location.X                       COORD
2  location.y                    Location.Y                       COORD
3  color                         Color                            COLOR
4  orientation                   Orientation                      T1    (RotationBy90)
5  symbol                        Symbol                           T1    (NoERCSymbol enum)
6  is_active                     IsActive                         T2    Export_Boolean_WithDefault
7  suppress_all                  SuppressAll                      T2    Export_Boolean_WithDefault
8  error_kind_set_to_suppress    ErrorKindSetToSuppress            COND  Only if !SuppressAll
9  connection_pairs_to_suppress  ConnectionPairsToSuppress         COND  Only if !SuppressAll
10 unique_id                     UniqueID                         T1    DynamicString
```

### RECORD=29: SchJunction

```
# Field          Key            Tier
-- [ExportGraphicalObject]
1  location.x     Location.X     COORD
2  location.y     Location.Y     COORD
3  size           Size           T1    (TSize enum)
4  color          Color          COLOR
5  locked         Locked         T1    bool
6  unique_id      UniqueID       T1    DynamicString
```

### RECORD=15: SchSheetSymbol

Uses `ExportRectangularEntryContainer` as base, then adds SheetSymbol-specific fields.

```
# Field                        Key                              Tier  Notes
-- [ExportGraphicalObject]
-- [ExportRectangularEntryContainer]
1  location.x                   Location.X                       COORD
2  location.y                   Location.Y                       COORD
3  x_size                       XSize                            COORD
4  y_size                       YSize                            COORD
5  line_width                   LineWidth                        T1    (TSize enum)
6  color                        Color                            COLOR
7  area_color                   AreaColor                        COLOR
-- [ExportSheetSymbol-specific]
8  is_solid                     IsSolid                          T1    bool
9  show_hidden_fields           ShowHiddenFields                 T1    bool
10 unique_id                    UniqueID                         STR   Note: Export_String (NOT DynamicString!)
11 symbol_type                  SymbolType                       T1    DynamicString (via SchDataUtils)
12 design_item_id               DesignItemId                     T1    DynamicString
13 source_library_name          SourceLibraryName                T1    DynamicString
14 vault_guid                   VaultGUID                        T1    DynamicString
15 item_guid                    ItemGUID                         T1    DynamicString
16 revision_guid                RevisionGUID                     T1    DynamicString
17 revision_name                RevisionName                     T1    DynamicString
```

**UniqueID anomaly:** SchSheetSymbol uses `Export_String` instead of `Export_DynamicString`
for UniqueID. This is unique among all record types. If UniqueID is empty, it defaults to `"$$$"`.

### RECORD=16: SchSheetEntry

Uses `ExportBasicEntryObject` as base.

```
# Field                        Key                              Tier  Notes
-- [ExportGraphicalObject]
-- [ExportBasicEntryObject]
1  side                         Side                             T1    (LeftRightSide enum)
2  distance_from_top            DistanceFromTop                  T1    coord
3  color                        Color                            COLOR
4  area_color                   AreaColor                        COLOR
5  text_color                   TextColor                        COLOR
6  text_font_id                 TextFontID                       T1
7  text_style                   TextStyle                        T1    DynamicString (via BusTextStyleToString)
8  name                         Name                             T1    DynamicString
9  harness_type                 HarnessType                      T1    DynamicString
10 unique_id                    UniqueID                         T1    DynamicString
-- [ExportSheetEntry-specific]
11 io_type                      IOType                           T1    (PortIO enum)
12 style                        Style                            T1    (PortArrowStyle enum)
13 arrow_kind                   ArrowKind                        T1    DynamicString (via ArrowKindToString)
14 default_cross_sheet_hidden   DefaultCrossSheetHidden          COND
```

### RECORD=43: SchParameterSet (NOT CompileMask)

**Naming clarification:** RECORD=43 is `eParameterSet`/`SchDataParameterSet` in the .NET code,
NOT CompileMask. CompileMask is RECORD=211.

```
# Field          Key            Tier
-- [ExportGraphicalObject]
1  location.x     Location.X     COORD
2  location.y     Location.Y     COORD
3  color          Color          COLOR
4  orientation    Orientation    T1    (RotationBy90)
5  name           Name           T1    DynamicString
6  style          Style          T1    (ParameterSetStyle enum)
7  unique_id      UniqueID       T1    DynamicString
```

### RECORD=211: SchCompileMask

```
# Field          Key            Tier  Notes
-- [ExportGraphicalObject]
1  unique_id      UniqueID       T1    DynamicString  NOTE: exported FIRST (before Location)
2  location.x     Location.X     COORD
3  location.y     Location.Y     COORD
4  corner.x       Corner.X       COORD
5  corner.y       Corner.Y       COORD
6  color          Color          COLOR
7  area_color     AreaColor      COLOR
8  collapsed      Collapsed      T1    bool
9  line_width     LineWidth      T1    (TSize enum)
```

**UniqueID ordering anomaly:** CompileMask and BusEntry export UniqueID FIRST, before
Location/Corner. This is opposite to most other records.

### RECORD=209: SchNote

```
# Field          Key            Tier
-- [ExportGraphicalObject]
1  location.x     Location.X     COORD
2  location.y     Location.Y     COORD
3  corner.x       Corner.X       COORD
4  corner.y       Corner.Y       COORD
5  line_width     LineWidth      T1    (TSize enum)
6  color          Color          COLOR
7  area_color     AreaColor      COLOR
8  text_color     TextColor      COLOR
9  font_id        FontID         T1
10 is_solid       IsSolid        T1    bool
11 show_border    ShowBorder     T1    bool
12 alignment      Alignment      T1    (HorizontalAlign enum)
13 word_wrap      WordWrap       T1    bool
14 clip_to_rect   ClipToRect     T1    bool
15 text           Text           TEXT  Export_Text (RTF-encoded)
16 text_margin    TextMargin     COORD
17 collapsed      Collapsed      T1    bool
18 author         Author         T1    DynamicString
19 unique_id      UniqueID       T1    DynamicString
```

---

### Shared Record Types (Same as SchLib)

These use identical export methods for both SchDoc and SchLib.

### RECORD=4: SchLabel

```
# Field          Key            Tier
-- [ExportGraphicalObject]
1  location.x     Location.X     COORD
2  location.y     Location.Y     COORD
3  orientation    Orientation    T1    (RotationBy90)
4  justification  Justification  T1    (TextJustification)
5  color          Color          COLOR
6  font_id        FontID         T1
7  text           Text           T1    DynamicString (included by default)
8  is_mirrored    IsMirrored     T1    bool
9  url            URL            T1    DynamicString
10 unique_id      UniqueID       T1    DynamicString
```

### RECORD=226: SchHyperlink

Same as RECORD=4 (SchLabel). Uses `ExportLabel()` directly.

### RECORD=5: SchBezier

```
# Field          Key            Tier
-- [ExportGraphicalObject]
1  line_width     LineWidth      T1    (TSize enum)
2  color          Color          COLOR
3  vertices       LOCATIONCOUNT + X{1..N} + Y{1..N}  INDEXED  (via ExportToFile)
4  unique_id      UniqueID       T1    DynamicString
```

### RECORD=6: SchPolyline

```
# Field          Key                    Tier
-- [ExportGraphicalObject]
1  line_width     LineWidth              T1    (TSize enum)
2  line_style     LineStyle              T1    Clamped to < DashDotted for backward compat
3  start_shape    StartLineShape          T1    (TLineShape enum)
4  end_shape      EndLineShape            T1    (TLineShape enum)
5  line_shape_size LineShapeSize          T1    (TSize enum)
6  color          Color                  COLOR
7  vertices       LOCATIONCOUNT + X{1..N} + Y{1..N}  INDEXED
8  line_style_ext LineStyleExt           T1    (ASCII-only byte, stores full enum)
9  unique_id      UniqueID               T1    DynamicString
```

### RECORD=7: SchPolygon

```
# Field          Key                    Tier
-- [ExportGraphicalObject]
1  line_width     LineWidth              T1    (TSize enum)
2  color          Color                  COLOR
3  area_color     AreaColor              COLOR
4  is_solid       IsSolid                T1    bool
5  transparent    Transparent            T1    bool
6  vertices       LOCATIONCOUNT + X{1..N} + Y{1..N}  INDEXED
7  unique_id      UniqueID               T1    DynamicString
```

### RECORD=8: SchEllipse

```
# Field            Key              Tier
-- [ExportGraphicalObject]
1  location.x       Location.X       COORD
2  location.y       Location.Y       COORD
3  radius           Radius           COORD
4  secondary_radius SecondaryRadius  COORD
5  line_width       LineWidth        T1    (TSize enum)
6  color            Color            COLOR
7  area_color       AreaColor        COLOR
8  is_solid         IsSolid          T1    bool
9  transparent      Transparent      T1    bool
10 unique_id        UniqueID         T1    DynamicString
```

### RECORD=9: SchPie

```
# Field          Key            Tier
-- [ExportGraphicalObject]
1  location.x     Location.X     COORD
2  location.y     Location.Y     COORD
3  radius         Radius         COORD
4  line_width     LineWidth      T1    (TSize enum)
5  start_angle    StartAngle     ANGLE (double)
6  end_angle      EndAngle       ANGLE (double)
7  color          Color          COLOR
8  area_color     AreaColor      COLOR
9  is_solid       IsSolid        T1    bool
```

**Note:** SchPie does NOT export UniqueID.

### RECORD=10: SchRoundRectangle

```
# Field          Key              Tier
-- [ExportGraphicalObject]
1  location.x     Location.X       COORD
2  location.y     Location.Y       COORD
3  corner.x       Corner.X         COORD
4  corner.y       Corner.Y         COORD
5  corner_x_rad   CornerXRadius    COORD
6  corner_y_rad   CornerYRadius    COORD
7  line_width     LineWidth        T1    (TSize enum)
8  color          Color            COLOR
9  area_color     AreaColor        COLOR
10 is_solid       IsSolid          T1    bool
11 unique_id      UniqueID         T1    DynamicString
```

### RECORD=11: SchEllipticalArc

```
# Field            Key              Tier
-- [ExportGraphicalObject]
1  location.x       Location.X       COORD
2  location.y       Location.Y       COORD
3  radius           Radius           COORD
4  secondary_radius SecondaryRadius  COORD
5  line_width       LineWidth        T1    (TSize enum)
6  start_angle      StartAngle       ANGLE
7  end_angle        EndAngle         ANGLE
8  color            Color            COLOR
9  unique_id        UniqueID         T1    DynamicString
```

### RECORD=12: SchArc

```
# Field          Key            Tier
-- [ExportGraphicalObject]
1  location.x     Location.X     COORD
2  location.y     Location.Y     COORD
3  radius         Radius         COORD
4  line_width     LineWidth      T1    (TSize enum)
5  start_angle    StartAngle     ANGLE
6  end_angle      EndAngle       ANGLE
7  color          Color          COLOR
8  unique_id      UniqueID       T1    DynamicString
```

### RECORD=13: SchLine

```
# Field          Key            Tier  Notes
-- [ExportGraphicalObject]
1  location.x     Location.X     COORD
2  location.y     Location.Y     COORD
3  corner.x       Corner.X       COORD
4  corner.y       Corner.Y       COORD
5  line_width     LineWidth      T1    (TSize enum)
6  line_style     LineStyle      T1    Clamped to < DashDotted for backward compat
7  color          Color          COLOR
8  line_style_ext LineStyleExt   T1    (ASCII-only byte, stores full enum)
9  unique_id      UniqueID       T1    DynamicString
```

### RECORD=14: SchRectangle

```
# Field          Key            Tier
-- [ExportGraphicalObject]
1  location.x     Location.X     COORD
2  location.y     Location.Y     COORD
3  corner.x       Corner.X       COORD
4  corner.y       Corner.Y       COORD
5  line_style_ext LineStyleExt   T1    (TLineStyle enum, exported as "LineStyleExt" key)
6  line_width     LineWidth      T1    (TSize enum)
7  color          Color          COLOR
8  area_color     AreaColor      COLOR
9  is_solid       IsSolid        T1    bool
10 transparent    Transparent    T1    bool
11 unique_id      UniqueID       T1    DynamicString
```

### RECORD=28: SchTextFrame

```
# Field          Key            Tier
-- [ExportGraphicalObject]
1  location.x     Location.X     COORD
2  location.y     Location.Y     COORD
3  corner.x       Corner.X       COORD
4  corner.y       Corner.Y       COORD
5  line_width     LineWidth      T1    (TSize enum)
6  color          Color          COLOR
7  area_color     AreaColor      COLOR
8  text_color     TextColor      COLOR
9  font_id        FontID         T1
10 is_solid       IsSolid        T1    bool
11 show_border    ShowBorder     T1    bool
12 alignment      Alignment      T1    (HorizontalAlign enum)
13 word_wrap      WordWrap       T1    bool
14 clip_to_rect   ClipToRect     T1    bool
15 text           Text           TEXT  Export_Text (RTF-encoded)
16 text_margin    TextMargin     COORD
17 unique_id      UniqueID       T1    DynamicString
```

### RECORD=30: SchImage

```
# Field          Key            Tier
-- [ExportGraphicalObject]
1  location.x     Location.X     COORD
2  location.y     Location.Y     COORD
3  corner.x       Corner.X       COORD
4  corner.y       Corner.Y       COORD
5  orientation    Orientation    T1    (RotationBy90)
6  line_width     LineWidth      T1    (TSize enum)
7  color          Color          COLOR
8  is_solid       IsSolid        T1    bool
9  keep_aspect    KeepAspect     T1    bool
10 embed_image    EmbedImage     T1    bool
11 file_name      FileName       T1    DynamicString
12 unique_id      UniqueID       T1    DynamicString
```

### RECORD=32: SchSheetName

```
# Field                Key                    Tier
-- [ExportGraphicalObject]
1  location.x           Location.X             COORD
2  location.y           Location.Y             COORD
3  orientation          Orientation            T1    (RotationBy90)
4  justification        Justification          T1    (TextJustification)
5  color                Color                  COLOR
6  font_id              FontID                 T1
7  is_hidden            IsHidden               T1    bool
8  text                 Text                   T1    DynamicString
9  is_mirrored          IsMirrored             T1    bool
10 not_auto_position    NotAutoPosition         T1    Inverted from AutoPosition
11 text_horz_anchor     TextHorzAnchor          T1    enum
12 text_vert_anchor     TextVertAnchor          T1    enum
13 unique_id            UniqueID               T1    DynamicString
```

### RECORD=33: SchSheetFileName

Identical parameter order to RECORD=32 (SchSheetName).

### RECORD=34: SchDesignator

Same as SchLib. See `docs/schlib/serialization.md` Section 4, RECORD=34.

Special behavior: If `!AutoPosition`, temporarily sets AutoPosition=true, exports
parameters (which writes `NotAutoPosition=F`), then appends `OverrideNotAutoPosition=T`.

### RECORD=41: SchParameter

Same as SchLib. See `docs/schlib/serialization.md` Section 4, RECORD=41.

```
# Field                      Key                            Tier  Notes
-- [ExportGraphicalObject]
1  location.x                 Location.X                     COORD
2  location.y                 Location.Y                     COORD
3  orientation                Orientation                    T1    (RotationBy90)
4  justification              Justification                  T1    (TextJustification)
5  color                      Color                          COLOR
6  font_id                    FontID                         T1
7  is_hidden                  IsHidden                       T1    bool
8  text                       Text                           T1    DynamicString (empty if name=="ProbeValueDisplay")
9  param_type                 ParamType                      T1    (ParameterKind)
10 name                       Name                           STR
11 show_name                  ShowName                       T1    bool
12 read_only_state            ReadOnlyState                  T1    (ParameterReadOnlyState)
13 unique_id                  UniqueID                       T1    DynamicString
14 description                Description                    T1    DynamicString
15 not_allow_lib_sync         NotAllowLibrarySynchronize      T1    Inverted
16 not_allow_db_sync          NotAllowDatabaseSynchronize     T1    Inverted
17 not_auto_position          NotAutoPosition                 T1    Inverted from AutoPosition
18 is_mirrored                IsMirrored                     T1    bool
19 text_horz_anchor           TextHorzAnchor                  T1    enum
20 text_vert_anchor           TextVertAnchor                  T1    enum
21 is_image_parameter         IsImageParameter                T1    bool
```

### RECORD=3: SchSymbol

```
# Field          Key            Tier
-- [ExportGraphicalObject]
1  symbol         Symbol         T1    (IeeeSymbol enum)
2  location.x     Location.X     COORD
3  location.y     Location.Y     COORD
4  scale_factor   ScaleFactor    COORD
5  orientation    Orientation    T1    (RotationBy90)
6  line_width     LineWidth      T1    (TSize enum)
7  color          Color          COLOR
8  is_mirrored    Mirror         T1    bool   Note: key is "Mirror" (not "IsMirrored")
```

### RECORD=37: SchBusEntry

```
# Field          Key            Tier  Notes
-- [ExportGraphicalObject]
1  unique_id      UniqueID       T1    DynamicString  NOTE: exported FIRST (before Location)
2  location.x     Location.X     COORD
3  location.y     Location.Y     COORD
4  corner.x       Corner.X       COORD
5  corner.y       Corner.Y       COORD
6  line_width     LineWidth      T1    (TSize enum)
7  color          Color          COLOR
```

**UniqueID ordering anomaly:** Same as CompileMask -- UniqueID is exported first.

### RECORD=210: SchProbe

```
# Field          Key            Tier
-- [ExportGraphicalObject]
1  location.x     Location.X     COORD
2  location.y     Location.Y     COORD
3  color          Color          COLOR
4  orientation    Orientation    T1    (RotationBy90)
5  name           Name           T1    DynamicString
6  unique_id      UniqueID       T1    DynamicString
```

### RECORD=44: SchImplementationList

```
-- [ExportGraphicalObject]   (only base fields, no additional)
```

### RECORD=45: SchImplementation

Same as SchLib. See `docs/schlib/serialization.md` Section 4, RECORD=45.

### RECORD=46: SchImplementationMap

Same as SchLib. See `docs/schlib/serialization.md` Section 4, RECORD=46.

### RECORD=47: SchMapDefiner

```
# Field                Key                Tier
-- [ExportDataObject]  (NOT GraphicalObject)
1  des_intf             DesIntf            STR
2  des_imp_count        DesImpCount        T1    (i32)
3  des_imp_{0..N}       DesImp{0..N}       STR   (indexed implementation designators)
```

**Note:** SchDoc's MapDefiner has additional fields vs SchLib. The base fields (`PinName`,
`PadName`) are still present but the export method also includes designator interface data.

### RECORD=48: SchImplementationParameters

```
-- [ExportDataObject]        (only base fields)
```

---

## 5. FileHeader Stream

### Block 0: Document Header

Single parameter text block (flags=0x00) containing the document identifier.

```
|HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0|Weight=<N>|MinorVersion=13|UniqueID=<id>|\0
```

| Key | Type | Value |
|-----|------|-------|
| `HEADER` | string | `Protel for Windows - Schematic Capture Binary File Version 5.0` |
| `Weight` | i32 | Total number of object records (blocks 1..N) |
| `MinorVersion` | i32 | **13** for Altium 26 (SchLib uses 9; legacy SchDoc uses 2) |
| `UniqueID` | string | Document-level unique identifier |

### Blocks 1..N: Content Records

All remaining blocks are parameter text blocks (flags=0x00), one per record in the
BaseWarehouse. For each record:

1. If `RECORD >= 256`: write `RECORD=254` + `RECORDEX=<actual_value>`
2. Otherwise: write `RECORD=<value>`
3. Call `FileFormat.ExportToFile(serializer, object)` for the record type

### Record Ordering in FileHeader

Records are written in BaseWarehouse order. The warehouse is built by:

1. Sheet (RECORD=31) at index 0
2. Template (RECORD=39) at index 1
3. Template-owned primitives
4. All other content records in depth-first ownership order

### OwnerIndex: Global Absolute Indexing

Unlike SchLib where OwnerIndex is component-relative, SchDoc uses **global absolute
indices** into the flat BaseWarehouse list:

- Block 1 (Sheet, RECORD=31) = index 0
- Block 2 (Template, RECORD=39) = index 1
- Sheet-level objects reference the sheet via OWNERINDEX=0
- Component children reference their parent component's absolute index

### Weight Calculation

Simply the count of all objects in the BaseWarehouse (excluding the header block itself):

```
Weight = BaseWarehouse.Count()
```

---

## 6. Additional Stream

### Header Block

```
|HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0|Weight=<N>|\0
```

Same HEADER string as the FileHeader stream. Weight = count of additional records.

### Content Blocks

RECORD=225 dashed rectangle overlay entries (and potentially other supplementary records).
Written the same way as FileHeader content blocks.

### When Weight = 0

The Additional stream is still written but contains only the header block with no Weight
parameter (or Weight absent). Total stream size: ~75 bytes.

---

## 7. Storage Stream (Embedded Images)

Identical to SchLib. See `docs/schlib/serialization.md` Section 10.

```
[header block, flags=0x00]
    |RECORD=0|HEADER=Icon storage|Weight=<count>|\0

[entry blocks, flags=0x01, one per image]
    0xD0 tag + id string + inner header + zlib-compressed data
```

---

## 8. Optional Streams

These streams are written only if they were present in the loaded document:

| Stream | Contents |
|--------|----------|
| `ObjectDefinitions` | Object definition records |
| `ReuseBlockInfos` | Reuse block metadata |
| `ReuseBlocks` / `ReuseBlocksV2` | Reuse block data |
| `HarnessConnectionPointConnector` | Harness connector data |
| `Files` | Embedded file data |

For round-trip, preserve these streams verbatim if they exist in the source file.

---

## 9. Block Stream Writing

Identical to SchLib. See `docs/schlib/serialization.md` Section 6.

### Block Header Format (4 bytes, little-endian i32)

```
bits [23:0]  = payload size in bytes  (mask 0x00FF_FFFF)
bits [31:24] = flags byte             (0x00 = text, 0x01 = binary)
```

### SchDoc-Specific: No Binary Blocks in FileHeader

Unlike SchLib where pins are binary blocks (flags=0x01), SchDoc FileHeader contains
ONLY parameter text blocks (flags=0x00). Binary blocks (flags=0x01) appear only in
the Storage stream for embedded images.

---

## 10. CFB Container Writing

### CFB Structure

```
Root Storage
 +-- FileHeader                  (document header + ALL records)
 +-- Additional                  (RECORD=225 entries)
 +-- Storage                     (embedded images)
 |
 +-- ObjectDefinitions           (optional)
 +-- ReuseBlockInfos             (optional)
 +-- ReuseBlocks                 (optional)
 +-- ReuseBlocksV2               (optional)
 +-- HarnessConnectionPointConnector  (optional)
 +-- Files                       (optional)
```

### Stream Write Order

```
1. /FileHeader                   (header + all content records)
2. /Storage                      (embedded images)
3. /Additional                   (RECORD=225 entries)
4. /ObjectDefinitions            (optional)
5. /ReuseBlockInfos              (optional)
6. /ReuseBlocks                  (optional)
7. /ReuseBlocksV2                (optional)
8. /HarnessConnectionPointConnector  (optional)
9. /Files                        (optional)
```

### CFB Metadata

- CFB Version: V3 (sector size 512 bytes)
- No sub-storages needed (flat structure)
- No SectionKeys needed (no per-component sub-storages)
- No alias redirections (SchDoc concept: components not organized like SchLib)

---

## 11. Key Differences from SchLib Serialization

| Aspect | SchDoc | SchLib |
|--------|--------|--------|
| Header string | `...Schematic Capture Binary...` | `...Schematic Library Editor Binary...` |
| MinorVersion | 13 | 9 |
| CFB layout | Flat (streams only, no storages) | Hierarchical (per-component storages) |
| Pin format | Text parameters (flags=0x00) always | Binary (flags=0x01) in Data streams |
| Pin sidecar streams | None | 9 streams per component |
| SectionKeys | Not needed | Required for long component names |
| Aliases | Not applicable | Alias storages with Redirection |
| Additional stream | RECORD=225 entries | Not present |
| OwnerIndex | Global absolute (flat list) | Relative per-component |
| Sheet record | RECORD=31 (font table here) | Font table in FileHeader header |
| FixBaseWarehouse | MoveSpecialObjectsToTop + AddAutoJunctions | Standard only |
| Weight | BaseWarehouse.Count() | Sum of (records + aliases) per component |
| Optional streams | Up to 6 (ObjectDefs, ReuseBlocks, etc.) | None |

---

## 12. Byte-Perfect Validation Strategy

### Round-Trip Test

```
1. Read original file -> SchDoc
2. Serialize SchDoc -> new CFB file
3. Compare original and new at stream content level
4. On mismatch: report stream name, block index, expected vs actual bytes
```

### Known Sources of Non-Determinism

- **CFB sector allocation**: Compare at stream content level, not raw file bytes
- **Parameter key casing**: Must use exact canonical casing from `FileFormatV5.cs`
- **Floating-point formatting**: Must match Delphi-compatible formatting
- **Font table pruning**: `ExportStyleAndFontTable` only exports fonts with save flags set;
  font indices may be remapped

---

## 13. Implementation Checklist

### Layer 1: Low-Level Writers (shared with SchLib)

- [ ] `write_text_block(params) -> Vec<u8>` -- shared
- [ ] `write_binary_block(data) -> Vec<u8>` -- shared (for Storage stream only)
- [ ] `write_embedded_object(id, inner_data) -> Vec<u8>` -- shared
- [ ] `ParameterCollection::to_bytes() -> Vec<u8>` -- shared
- [ ] `zlib_compress(data) -> Vec<u8>` -- shared

### Layer 2: Record Serializers (mostly shared with SchLib)

SchDoc-only records:
- [ ] `SchSheet::to_params()` -- RECORD=31 with font table
- [ ] `SchTemplate::to_params()` -- RECORD=39
- [ ] `SchWire::to_params()` -- RECORD=27
- [ ] `SchBus::to_params()` -- RECORD=26
- [ ] `SchNetLabel::to_params()` -- RECORD=25
- [ ] `SchPowerObject::to_params()` -- RECORD=17
- [ ] `SchPort::to_params()` -- RECORD=18
- [ ] `SchNoConnect::to_params()` -- RECORD=22
- [ ] `SchJunction::to_params()` -- RECORD=29
- [ ] `SchSheetSymbol::to_params()` -- RECORD=15
- [ ] `SchSheetEntry::to_params()` -- RECORD=16
- [ ] `SchParameterSet::to_params()` -- RECORD=43
- [ ] `SchCompileMask::to_params()` -- RECORD=211
- [ ] `SchNote::to_params()` -- RECORD=209
- [ ] `SchBusEntry::to_params()` -- RECORD=37
- [ ] `SchSheetName::to_params()` -- RECORD=32
- [ ] `SchSheetFileName::to_params()` -- RECORD=33
- [ ] `SchProbe::to_params()` -- RECORD=210

Shared with SchLib (same export methods):
- [ ] `SchComponent::to_params()` -- RECORD=1
- [ ] `SchPin::to_params()` -- RECORD=2 (text format)
- [ ] `SchLabel::to_params()` -- RECORD=4
- [ ] `SchBezier::to_params()` -- RECORD=5
- [ ] `SchPolyline::to_params()` -- RECORD=6
- [ ] `SchPolygon::to_params()` -- RECORD=7
- [ ] `SchEllipse::to_params()` -- RECORD=8
- [ ] `SchPie::to_params()` -- RECORD=9
- [ ] `SchRoundRectangle::to_params()` -- RECORD=10
- [ ] `SchEllipticalArc::to_params()` -- RECORD=11
- [ ] `SchArc::to_params()` -- RECORD=12
- [ ] `SchLine::to_params()` -- RECORD=13
- [ ] `SchRectangle::to_params()` -- RECORD=14
- [ ] `SchTextFrame::to_params()` -- RECORD=28
- [ ] `SchImage::to_params()` -- RECORD=30
- [ ] `SchDesignator::to_params()` -- RECORD=34
- [ ] `SchParameter::to_params()` -- RECORD=41
- [ ] `SchImplementationList::to_params()` -- RECORD=44
- [ ] `SchImplementation::to_params()` -- RECORD=45
- [ ] `SchImplementationMap::to_params()` -- RECORD=46
- [ ] `SchMapDefiner::to_params()` -- RECORD=47
- [ ] `SchImplementationParameters::to_params()` -- RECORD=48

### Layer 3: Document-Level Serialization

- [ ] `SchDoc::write_file_header() -> Vec<u8>` -- header + all content records
- [ ] `SchDoc::write_storage() -> Vec<u8>` -- embedded images
- [ ] `SchDoc::write_additional() -> Vec<u8>` -- RECORD=225 entries
- [ ] `SchDoc::write_optional_streams()` -- preserve verbatim if present

### Layer 4: CFB Assembly

- [ ] `SchDoc::save_to_file(path) -> Result<()>` -- create CFB, write all streams
- [ ] `SchDoc::save_as(path) -> Result<()>` -- public API

### Layer 5: Validation

- [ ] `validate_round_trip(original, output) -> Result<Vec<Mismatch>>` -- stream-level comparison
- [ ] CLI command: `altium schdoc validate --original file.SchDoc --output copy.SchDoc`

---

## 14. Source References

### C# Decompiled Sources

| File | Purpose |
|------|---------|
| `FileFormatV5.cs` (5575 lines) | Per-record parameter order, all Export_* call sites |
| `SchDataExporterBaseV5.cs` | Save pipeline orchestration |
| `SchDataExporterDocumentV5.cs` | Document-level export (shared by SchDoc/HarnessDoc) |
| `SchDataExporterSheetV5.cs` | SchDoc-specific (MinorVersion=13, ReuseBlocks) |
| `SchDataSerializerParam.cs` | Tier 1/2 sparse-save method implementations |
| `SchDataSerializer.cs` | Base serializer (Export_Coord, Export_Boolean, etc.) |
| `SchDataObjectComparator.cs` | Child record sort order |
| `FileFormatConsts.cs` | Stream name constants, header strings |
| `BinaryFileCode.cs` | Binary instruction codes |

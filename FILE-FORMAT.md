# SchLib Round-Trip Binary Compatibility Issues

Issues found by round-tripping `Synthiam.SchLib` through `altium-cli schlib json` export
and `altium-cli schlib add-json` import, then comparing with `scripts/diff-ole.py`.

**Test file:** Synthiam.SchLib (566,784 bytes, 172 components + 1 alias)
**Round-trip:** `schlib json` → `schlib create` + `schlib add-json` → `diff-ole.py`

## Summary

| Result | Count |
|--------|-------|
| Total OLE streams | 176 |
| Byte-identical | 2 (PIC10F220/Redirection, Storage) |
| Added | 1 (SectionKeys — harmless, supports both mechanisms) |
| Removed | 0 |
| Changed | 173 |
| Changed (property-order only) | 76 streams (263 records) |

**File sizes:** 566,784 bytes (original) → 559,104 bytes (new)

---

## Remaining Issues

### 1. INDEXINSHEET vs OWNERINDEX (design decision)

**Status: Won't fix — deliberate design choice**

The original uses `INDEXINSHEET` as a sequential record index within each component.
Our writer uses `OWNERINDEX` for parent-child ownership linking. Both achieve the same
semantics but use different field names.

- **2237 instances** of INDEXINSHEET in original (0 in new)
- **1725 instances** of OWNERINDEX added in new
- Affects all 172 component streams

### 2. LOCATION.X/Y emitted on vertex-based records

**Status: Open — fixable**

Polylines (RECORD=6), polygons (RECORD=7), and beziers (RECORD=5) use indexed vertex
coordinates (X1/Y1, X2/Y2, ...) for their geometry. The original Altium file never
emits LOCATION.X/Y on these record types — the location is implicit from the first
vertex. Our `SchGraphicalBase` flatten always emits LOCATION.X/Y when non-zero.

| Record | Type | Extra LOCATION |
|--------|------|----------------|
| RECORD=6 | Polyline | 275 records (0 in original have LOCATION.X) |
| RECORD=7 | Polygon | 101 records (0 in original) |
| RECORD=5 | Bezier | 2 records (0 in original) |

- **~341 extra LOCATION.X** and **~337 extra LOCATION.Y** emitted

### 3. PARTIDLOCKED value not preserved through JSON

**Status: Open — fixable**

Original components have `PARTIDLOCKED=T` (81 instances) or `PARTIDLOCKED=F` (90 instances).
The JSON schema doesn't include `part_id_locked`, so the import always uses the default
(`false`). This changes 81 components from `PARTIDLOCKED=T` to `PARTIDLOCKED=F`.

- **81 instances** changed from T to F

### 4. LIBRARYPATH=* and SHEETPARTFILENAME=* not emitted

**Status: Open — fixable**

The original emits `LIBRARYPATH=*` and `SHEETPARTFILENAME=*` on most components even
though `*` is a placeholder value. Our writer conditionally skips these when the value
is `*`, causing them to be absent.

- **90 instances** of LIBRARYPATH=* removed
- **80 instances** of SHEETPARTFILENAME=* removed

### 5. DESIGNITEMID not preserved through JSON

**Status: Open — fixable**

58 components in the original have `DESIGNITEMID=<component_name>` in addition to
`LIBREFERENCE`. DESIGNITEMID is an alternative/legacy name for LIBREFERENCE. It passes
through `unknown_params` during file-to-file round-trips, but is lost during JSON
export/import since the JSON schema doesn't include it.

- **58 instances** removed

### 6. LINEWIDTH minor mismatches

**Status: Open — minor**

Net difference of -27 (original has 27 more LINEWIDTH=1 records than new). Breakdown:

| Cause | Count | Direction |
|-------|-------|-----------|
| Lines (RECORD=14): original has 1, we emit 0 | 1 | Missing |
| Polygons: original has 87/101, we emit 101/101 | 14 | Extra |
| Arcs: original has 104/105, we emit 105/105 | 1 | Extra |
| Ellipses: original has 5/6, we emit 6/6 | 1 | Extra |

Most primitives use `LineWidth::Small` (1), but a few use `LineWidth::Smallest` (0).
The JSON format doesn't preserve line_width, so the import uses a fixed default.

### 7. OWNERPARTDISPLAYMODE not preserved

**Status: Open — fixable**

14 records in the original have `OWNERPARTDISPLAYMODE=1` (for components with multiple
display modes). This field is not preserved through the JSON round-trip.

### 8. TRANSPARENT field inconsistency

**Status: Open — minor**

Net -3 difference. Some ellipses/arcs have `TRANSPARENT=T` in the original that we don't
emit, and some we emit that the original doesn't have.

### 9. Rare component-level properties not preserved

**Status: Open — minor**

| Property | Count | Notes |
|----------|-------|-------|
| NOTUSEDBTABLENAME | 3 | Rare component-level flag |
| DATABASEMODEL | 2 | Implementation record property |
| INTEGRATEDMODEL | 2 | Implementation record property |

### 10. Coordinate FRAC normalization (cosmetic)

**Status: Won't fix — cosmetic**

Non-canonical fractional coordinates (e.g., `RADIUS=14, RADIUS_FRAC=85746`) are
normalized to canonical form (`RADIUS=22, RADIUS_FRAC=5746`). Both decode to the
same raw coordinate value (225746). Affects ~30 records.

### 11. FileHeader property order

**Status: Won't fix — cosmetic**

The FileHeader stream differs only in property ordering (1 record). All values are
identical; keys are sorted differently due to INDEXINSHEET→OWNERINDEX change.

---

## Previously Fixed Issues

| Issue | Fix | Impact |
|-------|-----|--------|
| ISNOTACCESIBLE=T over-applied | Only set on graphical primitives (RECORD 4-14), not params/designators | Eliminated ~1155 spurious additions |
| Duplicate ISHIDDEN=T on parameters | Added raw_suffix support to ParameterCollection | Fixed 469 records |
| UNIQUEID not preserved on parameters | Added unique_id field to SchParameter and JSON | Fixed 982 records |
| SECONDARYRADIUS emitted when 0 | Added skip_default to frac fields in derive macro | Fixed ~111 records |
| TEXT= emitted for empty text | Added skip_default to SchLabel text field | Fixed ~240 records |
| PARTIDLOCKED=F not emitted | Changed from add_bool to explicit add("PARTIDLOCKED","F") | Fixed 171 records |
| LINEWIDTH=1 on lines | Changed SchLine import default to Smallest | Fixed ~102 records |
| Visible parameter ordering | Moved visible params before Designator | Fixed 6 components |
| WEIGHT calculation | Added alias count to weight | Fixed +1 difference |
| Slash escaping in names | Fixed get_section_key() | Fixed ATtiny45/85, PIC10F220T-I/OT |
| Alias/Redirection streams | Added write_alias_redirections() | Fixed alias round-trip |
| RECORD=48 OWNERINDEX | Fixed to point to Implementation | Fixed implementation hierarchy |
| Implementation-owned parameters | Moved to per-implementation JSON | Fixed ownership semantics |
| COMPCOUNT alias overcounting | Excluded aliases from count | Fixed header count |

---

## File Structure Comparison

| Aspect | Original | New | Match? |
|--------|----------|-----|--------|
| Component count | 172 | 172 | Yes |
| COMPCOUNT in header | 172 | 172 | Yes |
| LIBREF entries | 172 | 172 | Yes |
| PARTCOUNT entries | 172 | 172 | Yes |
| COMPDESCR entries | 92 | 92 | Yes |
| Alias entries | 1 (PIC10F220) | 1 (PIC10F220) | Yes |
| Redirection stream | 33 bytes | 33 bytes | Yes (identical) |
| Storage stream | 25 bytes | 25 bytes | Yes (identical) |
| SectionKeys stream | absent | 112 bytes | Extra (harmless) |
| Record types 44-48 | present | present | Yes |
| Implementation-owned params | owned by RECORD=48 | owned by RECORD=48 | Yes |

## Reproduction

```bash
# Export to JSON
altium-cli schlib json Synthiam.SchLib --pretty > /tmp/synthiam-export.json

# Create new library and import
altium-cli schlib create /tmp/Synthiam-new.SchLib
altium-cli schlib add-json /tmp/Synthiam-new.SchLib -f /tmp/synthiam-export.json

# Compare
python3 scripts/diff-ole.py summary Synthiam.SchLib /tmp/Synthiam-new.SchLib
python3 scripts/diff-ole.py text Synthiam.SchLib /tmp/Synthiam-new.SchLib
```

# Handoff: ecad-r1-r7 (altium-format API requests R1–R7 from ecadabra E2)

Date: 2026-10-22. Branch `ecad-r1-r7`, based on `18062d6` (the commit ecadabra's importer pins;
master has moved on to `bb98860`). Stopped early by the coordinator because of quota limits.
Requests and evidence: ecadabra branch `e2-altium-import`, `docs/spikes/e2-altium-import.md`.
The six problems held by the user (`docs/spikes/e0-corpus-and-nm.md` §5) were not touched.

## State per request

| Request | State |
|---|---|
| R2 polygons | **Partly done, WIP commit.** Code written, `cargo check -p altium-format --tests` passes. No test has been run. |
| R4 ids, component index, flags | Not started |
| R3 polygon membership | Not started |
| R1 Board6 outline | Not started (the shared contour parser it needs is in, see below) |
| R6 layer identity | Not started. The parameter-layer part is in R2 (see below); `copper_index` is not |
| R5 pad expansion mode | Not started |
| R7 region arcs | Not started |

## Tests actually observed

- `cargo check -p altium-format --tests`: exit 0 (debug, `CARGO_TARGET_DIR` outside the checkout).
- `cargo fmt -p altium-format`: applied.
- **Nothing else was run.** The new unit tests in `param_value.rs` have been compiled but never
  run. clippy has not been run, and neither have `cargo check -p altium-format-spec -p altium-cli`
  or any corpus check.

## What the WIP commit changes (R2)

Bugs found and fixed in `api/pcbdoc_read.rs`:

- **Vertices.** `VXn`/`VYn` were read as `i32`, and every parse error was swallowed by
  `.unwrap_or(None)`, so every pour came out with no vertices. The new crate-private
  `remove_param_contour` reads `KINDn VXn VYn CXn CYn SAn EAn Rn` as an arc-preserving
  `PcbContour`. A partial vertex, a bad value, a non-finite angle or a gap in the indices is an
  error. Lengths use the new `param_value::ExactMilCoord`: `<decimal>mil` with at most 4 fractional
  digits, parsed with integer arithmetic. The suffix is required, and a value out of the i32 range
  is an error.
- **Pour order.** `pour_order` read `POURORDER`, a key Polygons6 never stores. It now reads
  `POURINDEX`.
- **Names.** `NAME` is stored as comma-separated UTF-16 code units, and is now decoded with
  `wide_strings::decode_encoded_text`, which is now `pub(crate)`.
- **Layers.** `LAYER=MID2`/`PLANE1` (and every other unknown name) silently fell back to the
  default layer, Top. The new `layer_from_param_value` reads `TOP`, `BOTTOM`, `MID1..30`,
  `PLANE1..16`, canonical V6 names and numeric ids. Anything else is now an error. This also
  changes `parse_layer_param`, so Components6 and Dimensions6 are affected too: an unknown layer
  is now an error, not the default.
- **Net index.** A `NET` index outside Nets6 is an error. Before, it gave `None`.
- **Unknown keys.** Each Polygons6 record must be fully consumed (`assert_exhausted`), so an
  unknown key is an error.

Public API, additions only (`api::Polygon`; struct literals of `Polygon` exist only inside
altium-format):

```rust
pub contour: PcbContour,
pub polygon_type: PolygonType,              // "Polygon" | "Split Plane"
pub hatch_style: Option<PolyHatchStyle>,    // None on split planes
pub grid_size: Option<Coord>, pub track_width: Option<Coord>,
pub min_prim_length: Option<Coord>, pub arc_approximation: Option<Coord>, // ARCRESOLUTION
pub remove_dead: Option<bool>, pub pour_over: Option<bool>, pub pour_over_style: Option<u8>,
pub use_octagons: Option<bool>, pub remove_islands_by_area: Option<bool>,
pub area_threshold: Option<f64>, pub remove_necks: Option<bool>,
pub neck_width_threshold: Option<Coord>, pub neck_width_from_rule: Option<bool>,
pub shelved: Option<bool>, pub restore_layer: Option<LayerRef>, pub restore_net: Option<String>,
pub ignore_violations: Option<bool>, pub auto_name: Option<bool>,
pub optimal_void_rotation: Option<bool>, pub obey_polygon_cutout: Option<bool>,
pub copper_invalidate: Option<bool>,
pub selected: bool, pub locked: bool, pub primitive_lock: bool, pub polygon_outline: bool,
pub user_routed: bool, pub keepout: bool, pub union_index: i32,
```

`vertices` is kept and is now `contour.to_points()`. The doc comment on `PcbContour` now spells
out the TPolySegment semantics: segment i holds vertex i and the edge from i to i+1, and an arc
runs counter-clockwise from SA to EA with either end at SA. The corpus check that backs this:
197 arcs have SA at vertex i, 326 have SA at vertex i+1, and none match neither.

Deviations from R2's requested signature:
- R2 asked for `remaining_params: Vec<(String, String)>`. It is not added: altium-cli's CLAUDE.md
  forbids unknown-field capture. Every key seen in the corpus is typed instead, and an unknown key
  fails.
- `hatch_style` and the pour settings are `Option` because split-plane records omit them.
- The type is the existing `PolyHatchStyle`, not `PolygonHatchStyle`.

Write path (`api/pcbdoc_write.rs`): `build_polygon_records` now writes the full typed record in
stored form, the inverse of the reader. It writes `insert_param_contour` vertices in mil, the name
encoded, short layer names and `POURINDEX`. It returns `Result`. Before, it wrote integer vertices,
a raw name and `POURORDER`.

Unconfirmed: the hatch strings `45Degree`, `Vertical`, `Horizontal` and `None` follow Altium's
names, but only `Solid` and `90Degree` occur in the corpus. A `POLYGONTYPE` other than `Polygon`
or `Split Plane` is an error.

## Corpus facts gathered (akiselev/altium-cli-test-pcbdoc `6bc5aa7`, 94 OLE PcbDocs with Polygons6)

- Polygons6: 1,618 records (1,513 `Polygon`, 105 `Split Plane`) and 25,070 vertices. Every
  length has the `mil` suffix with at most 4 fractional digits. Every contour repeats its first
  vertex as the last.
- 71 line edges carry stale arc fields, which Altium ignores for line edges.
- Board6 outline: 94 records, 1,037 vertices, the same key layout. One board's outline is open.
  Integer parts reach 214,748 mil, right at the i32 limit.
- UniqueIDPrimitiveInformation holds only `Pad` entries (58,719). PrimitiveGuids is present in
  only 18 files. So R4's `unique_id` can be filled for pads only, unless the PrimitiveGuids
  `IndexForSave` mapping is verified.
- Components6 stores `UNIQUEID`, `LOCKED` and `PRIMITIVELOCK`.
- ExtendedPrimitiveInformation in PcbDocs covers Track, Region, Arc and Fill only, never Pad. For
  R5, pad modes come from the pad cache's `paste_mask_expansion_valid` and
  `solder_mask_expansion_valid` (`TCacheState`: Invalid, Valid, Manual).
- V9 stack layer ids in rfsoc-amc: 0x0100_00ss for signal layers and 0x0101_00ss for planes. The
  species is not in physical order. For R6, the mapping V7 signal species s → V6 s (0xFFFF →
  Bottom) and plane species p → V6 38+p still has to be confirmed against primitives that carry
  both the V6 and the V7 layer.

Scripts used, outside the repository: `/home/dev/ecad-r1-r7-work/tools/{stream,survey,arcs,lines,decimals,guids}.py`
(python3 -I with olefile).

## Next steps

1. Run `cargo test -p altium-format --lib exact_mil_coord`. Add unit tests for
   `convert_polygon`, `remove_param_contour` (incomplete vertex, gap, bad unit, arc), layer
   names, and a Polygons6 write→read round trip.
2. Add a `test-fixtures` corpus test: with `data/pcbdoc` linked to the corpus clone, count
   polygons and vertices parsed by `board()`, and confirm that no board that read before now
   fails.
3. Run `cargo check -p altium-format-spec -p altium-cli` (the coordinator allowed this) and clippy.
4. Then R4, R3, R1, R6, R5, R7 in that order, one commit each. R1 can reuse
   `remove_param_contour` on a clone of the Board6 params. It cannot use `assert_exhausted`, so it
   should rely on the stray-key check.
5. Adding fields to Track, Arc, Via, Pad, Fill, Region, Text, PcbDocComponent and BoardSettings
   breaks struct literals in altium-format-spec (`executor.rs`, `reconciler.rs`, `dump.rs`) and
   altium-cli (`main.rs`). Fill those with defaults; the coordinator approved this. Report it as
   an API break.

# Crate Dependency Recommendations

Trade-off analysis for external crate selection. Each recommendation reflects
the **strict parsing philosophy**: every byte must be accounted for, unknown
data is an error, no round-trip preservation of opaque blobs.

License requirement: all dependencies must be MIT and/or Apache-2.0 compatible.

---

## 1. OLE/CFB Parsing: `cfb`

**Need:** Every Altium file (.SchDoc, .SchLib, .PcbDoc, .PcbLib, .IntLib) is a
Microsoft Compound Binary File (CFB v3) container. We need read/write access to
named streams and storages.

| Crate | Version | License | Status |
|-------|---------|---------|--------|
| `cfb` | 0.12+ | MIT | Active, sole maintainer (mdsteele) |
| `ole` | 0.2 | MIT | Unmaintained since 2019, read-only |

**Why `cfb`:**
- Only viable Rust crate for CFB read/write.
- Pure Rust, no C dependencies.
- Supports v3 and v4, stream/storage creation, removal, renaming.
- Lazy sector chain reads -- even multi-MB files open in milliseconds.

**Known limitations:**
- No streaming/zero-copy API; reads entire stream contents into `Vec<u8>`.
  This is fine for our use case (we need the full stream in memory anyway).
- Single-threaded access to the `CompoundFile` handle. Not `Send` or `Sync`
  across the inner `File`. We only hold it open during load/save, so this
  doesn't matter.
- The author is responsive to issues but the crate has a small bus factor of 1.
  If it becomes unmaintained, the format is well-specified enough to fork or
  reimplement the subset we use.

**Verdict:** Use `cfb`. No realistic alternative.

---

## 2. Binary Parsing: `byteorder` (or manual)

**Need:** PCB records use packed little-endian binary structures. Every byte
must be accounted for -- there are no "skip N unknown bytes" in the strict
model. The derive macros generate sequential read/write code over a cursor.

| Crate | Approach | License | Size |
|-------|----------|---------|------|
| `byteorder` 1.5 | Extension traits on Read/Write | MIT/Unlicense | ~2KB |
| `binrw` 0.14 | Derive macros for binary structs | MIT | ~200KB compiled, ~8s compile |
| `nom` 7.x | Parser combinator | MIT | ~50KB |
| `winnow` 0.6 | Parser combinator (nom successor) | MIT | ~40KB |
| Manual `from_le_bytes` | No dependency | N/A | 0KB |

**Why `byteorder` over `binrw`:**
- We already have our own derive macros (`AltiumRecord`) that generate binary
  parsing code. `binrw` would be a second, competing derive system.
- Our Altium-specific attributes (subrecord framing, per-layer pad stacks,
  sidecar merge) cannot be expressed in `binrw`'s attribute model.
- `byteorder` is a thin trait extension (`reader.read_i32::<LE>()`) that the
  generated code calls directly. Zero abstraction overhead.
- Adding `binrw` costs ~8 seconds of proc-macro compile time for no benefit.

**Why not `nom`/`winnow`:**
- Parser combinators shine for streaming/incremental parsing. Our records are
  size-prefixed blocks already loaded into `&[u8]` or `Cursor<&[u8]>`.
  Sequential reads via `byteorder` are clearer and faster than combinator
  chains for this use case.
- Combinators also make it harder to enforce "every byte consumed" since
  leftover input is a normal condition in combinator parsers.

**Why not manual `from_le_bytes`:**
- Requires manual offset tracking. `byteorder` over `Read` handles cursor
  advancement automatically. The dependency is ~2KB and eliminates a class of
  off-by-one bugs.
- Acceptable alternative if we want zero dependencies, but the ergonomic cost
  isn't worth it.

**Strict parsing integration:**
- After reading all known fields, the generated code checks that the cursor
  position equals the expected record length. Any remaining bytes are an
  error, not preserved as unknown trailing data.
- `byteorder` read calls that hit EOF produce `io::Error`, which we wrap in
  `AltiumFormatError::BinaryParse` with offset and field context.

**Verdict:** Use `byteorder`. Simplest, smallest, and works perfectly with our
codegen approach.

---

## 3. Error Handling: `thiserror`

**Need:** Per CLAUDE.md, everything fallible returns `Result<T, AltiumFormatError>`.
We need structured error types with variants for I/O, CFB, parameter parsing,
binary parsing, unknown records/fields, encoding, and compression errors.

| Crate | Approach | License | Use case |
|-------|----------|---------|----------|
| `thiserror` 2.x | Derive `Error` on enums/structs | MIT/Apache-2.0 | Library errors |
| `anyhow` 1.x | Boxed error with context | MIT/Apache-2.0 | Application errors |
| `eyre` 0.6 | `anyhow` alternative | MIT/Apache-2.0 | Application errors |
| Manual | `impl Display + Error` | N/A | Full control |

**Why `thiserror` for `altium-format` and `altium-format-ops`:**
- Library crates must expose typed, matchable errors. `anyhow`/`eyre` erase
  error types, making programmatic error handling impossible for callers.
- `#[derive(Error)]` with `#[error("...")]` and `#[from]` generates exactly
  the code you'd write by hand, minus the boilerplate.
- Zero runtime overhead (no allocation, no vtable indirection).

**Why `anyhow` for `altium-cli`:**
- The CLI binary doesn't need typed errors for its `main()` -- it prints them
  and exits. `anyhow::Result` simplifies error propagation at the application
  boundary.
- `anyhow::Context` provides `.context("while opening PcbDoc")` for
  user-facing error messages without polluting the library error types.

**Error type strategy:**
- `altium-format`: `AltiumFormatError` enum with `thiserror`
- `altium-format-ops`: `AltiumOpsError` enum with `thiserror`, wrapping
  `AltiumFormatError` via `#[from]`
- `altium-cli`: `anyhow::Result` at the binary level

**Verdict:** `thiserror` for library crates, `anyhow` for the CLI binary.

---

## 4. Derive Macro Infrastructure: `syn` + `quote` + `proc-macro2`

**Need:** The `altium-format-derive` crate implements proc macros that parse
Rust struct/enum definitions and generate serialization code.

| Crate | Version | Role |
|-------|---------|------|
| `syn` 2.x | Parse Rust syntax into AST | Required |
| `quote` 1.x | Generate Rust tokens from templates | Required |
| `proc-macro2` 1.x | Token stream abstraction | Required (by syn/quote) |

These are the universal standard for Rust proc macros. No alternatives worth
considering.

**Feature flags on `syn`:** Use `full`, `parsing`, `extra-traits`. We parse
full struct/enum definitions with complex nested attributes.

**Why not `darling`:**
- `darling` simplifies attribute parsing by deriving `FromDeriveInput` /
  `FromField`. It reduces boilerplate in the macro crate itself, but adds
  another compile-time dependency. Worth considering if the attribute model
  becomes complex, but not required initially.

**Why not `venial`:**
- `venial` is lighter than `syn` but cannot handle the nested attribute
  patterns our macros require (e.g., `#[altium(param = "X", frac = "X_FRAC")]`).

**Verdict:** `syn` 2.x + `quote` 1.x + `proc-macro2` 1.x. Standard setup.

---

## 5. Compression: `flate2`

**Need:** Altium files use zlib compression in:
1. Compressed storage blocks in the `Storage` stream (`0xD0` magic byte)
2. Sidecar streams in SchLib (PinFrac, PinPackageLength, etc.)
3. IntLib embedded sub-files

Both decompression (read) and compression (write) are needed.

| Crate | Backend | License | Notes |
|-------|---------|---------|-------|
| `flate2` 1.x | `miniz_oxide` (default, pure Rust) | MIT/Apache-2.0 | Standard |
| `flate2` 1.x | `zlib-ng` (optional C backend) | MIT/Apache-2.0 | Faster |
| `miniz_oxide` | Direct pure Rust | MIT/Apache-2.0 | Lower-level API |

**Why `flate2` with default backend:**
- Pure Rust by default -- no C compiler required.
- Standard `ZlibDecoder`/`ZlibEncoder` over `Read`/`Write`.
- Compressed blocks in Altium files are typically < 100KB. Performance
  differences between backends are irrelevant at this scale.
- Used by the Rust compiler itself. Battle-tested.

**Verdict:** Use `flate2` with default `miniz_oxide` backend.

---

## 6. String Encoding: `encoding_rs`

**Need:** Altium files use Windows-1252 encoding for parameter strings, with
a `%UTF8%` key prefix for UTF-8 values re-encoded through Windows-1252. PCB
WideStrings sidecars use UTF-16LE.

| Crate | License | Coverage |
|-------|---------|----------|
| `encoding_rs` 0.8 | MIT/Apache-2.0 | All WHATWG encodings including Windows-1252, UTF-16LE |

**Why `encoding_rs`:**
- Provides `WINDOWS_1252.decode()` / `.encode()` for text records.
- Handles UTF-16LE for WideStrings sidecar streams.
- Used by Firefox. Extremely well-tested.
- Zero-copy decode when input is ASCII-only (common case for parameter keys).
- Properly handles the edge cases where Windows-1252 differs from ISO-8859-1
  (code points 0x80-0x9F map to printable characters in Windows-1252 but are
  control characters in ISO-8859-1).

**No alternatives worth considering.** `encoding_rs` is the only maintained
Rust crate with Windows-1252 support.

**Verdict:** Use `encoding_rs` 0.8.x.

---

## 7. CLI Framework: `clap`

**Need:** The `altium-cli` binary needs subcommand dispatch, positional
arguments, flags/options, shell completion, and help text.

| Crate | License | Approach |
|-------|---------|----------|
| `clap` 4.x | MIT/Apache-2.0 | Derive + builder, industry standard |
| `clap_complete` 4.x | MIT/Apache-2.0 | Shell completion scripts |
| `argh` 0.1 | BSD-3-Clause | Google's lightweight parser |

**Why `clap`:**
- Most featureful CLI framework in Rust. Derive macros for declarative
  argument definitions.
- `clap_complete` generates Bash/Zsh/Fish/PowerShell completions.
- Well-documented, widely understood by Rust developers.
- ~300KB compiled. Acceptable for a CLI binary.

**Why not `argh`:**
- Lighter compile time but missing shell completions, custom value parsers,
  and env variable fallbacks.

**Verdict:** Use `clap` 4.x + `clap_complete` 4.x.

---

## 8. Logging: `tracing`

**Need:** Diagnostic output for debugging file parsing. Under the strict
philosophy, unknown fields/records are errors, not warnings -- but we still
need structured diagnostic output for understanding what the parser is doing
(which stream it's reading, record counts, sidecar merge steps).

| Crate | License | Structured | Spans |
|-------|---------|-----------|-------|
| `log` 0.4 | MIT/Apache-2.0 | No | No |
| `tracing` 0.1 | MIT | Yes | Yes |

**Why `tracing` over `log`:**
- `tracing` provides **spans** which are invaluable for structured parsing
  diagnostics: `parsing stream "Pads6/Data" > record 142 > field "hole_size"`.
- Spans have built-in timing, useful for performance profiling.
- `tracing` has a `log` compatibility layer, so it works with `log` subscribers
  too.
- The CLI can use `tracing-subscriber` with `EnvFilter` for `RUST_LOG`-style
  filtering.

**Why not `log`:**
- `log` only provides flat messages without structural context. For a parser
  that processes nested structures (file > stream > record > field), spans
  provide much better diagnostics.

**Migration cost:** Low. `tracing::info!()` is syntax-compatible with
`log::info!()`. The library should not expose `tracing` types in its public
API (use it internally only).

**Verdict:** Use `tracing` for all crates. Use `tracing-subscriber` in
`altium-cli`.

---

## 9. Testing

**Need:** Strict format testing where every byte must be accounted for.
Tests must verify that our model is complete -- any unrecognized data is a
failure, not something to snapshot and ignore.

### 9.1 Snapshot Testing: `insta`

| Crate | License | Use case |
|-------|---------|----------|
| `insta` 1.x | Apache-2.0 | Snapshot testing with `cargo insta review` |

**Why `insta`:**
- Golden file testing against real Altium files. Parse a file, serialize the
  parsed model to a readable format (debug repr or JSON), compare against a
  stored snapshot.
- `cargo insta review` provides an interactive TUI for reviewing changes.
- Supports inline snapshots (embedded in test code) and file snapshots.
- Under the strict philosophy, snapshots serve as regression tests: if a
  record type is extended or a new field is added, the snapshot diff shows
  exactly what changed.

### 9.2 Property-Based Testing: `proptest`

| Crate | License | Use case |
|-------|---------|----------|
| `proptest` 1.x | MIT/Apache-2.0 | Property-based testing with shrinking |

**Why `proptest`:**
- Generate arbitrary record structs and verify `serialize(record) -> parse ->
  record == original`. This is the **semantic equivalence test**, not byte
  equivalence (since we don't preserve opaque blobs).
- Strategies can be constrained to valid field ranges (e.g., layer IDs 0-82,
  shape enums 0-10).
- Shrinking finds minimal failing cases.

### 9.3 Fuzz Testing: `cargo-fuzz` / `arbitrary`

| Crate | License | Use case |
|-------|---------|----------|
| `arbitrary` 1.x | MIT/Apache-2.0 | Structured fuzzing support |
| `cargo-fuzz` | N/A (tool) | Fuzz testing harness |

**Why fuzz testing:**
- Feed random bytes to the parser and verify it either returns a valid
  document or a clean error -- never panics, never UB.
- Especially important for binary parsing where offset calculations can go
  wrong.
- Under the strict philosophy, the parser must reject malformed input
  cleanly, not crash or produce garbage.

### 9.4 Assertions: `static_assertions`

| Crate | License | Use case |
|-------|---------|----------|
| `static_assertions` 1.x | MIT/Apache-2.0 | Compile-time assertions |

**Why:**
- `assert_impl_all!(SchDoc: Send, Sync)` -- prevent accidental regression.
- `assert_eq_size!(Coord, i32)` -- ensure newtypes don't grow unexpectedly.

**Testing verdict:** Use `insta` + `proptest` as dev-dependencies. Add
`cargo-fuzz` harness for pre-release robustness testing. Use
`static_assertions` for compile-time invariants.

---

## 10. Coordinate Math: Custom `Coord(i32)` Newtype

**Need:** Fixed-point coordinate system: 10,000 internal units per mil. Both
schematic and PCB share this. Need arithmetic ops, conversion methods, DXP
fractional encoding.

| Option | License | Approach |
|--------|---------|----------|
| Custom `Coord(i32)` | N/A | Newtype with selected ops |
| `fixed` crate | MIT/Apache-2.0 | Generic fixed-point (power-of-2) |
| `fpdec` | MIT | Decimal fixed-point |

**Why custom over `fixed`:**
- The `fixed` crate uses power-of-2 fractional bits. Altium's 10,000 units/mil
  is a decimal system. `fixed` would require constant conversion with rounding
  errors.
- A newtype `Coord(i32)` with domain-specific methods is clearer:
  `Coord::from_mils()`, `Coord::to_mm()`, `Coord::from_dxp_frac(integer, frac)`.
- `Add<Coord>`, `Sub<Coord>`, `Neg`, `Mul<i32>`, `Div<i32>` -- but NOT
  `Mul<Coord>` or `Div<Coord>` (coordinates should not be multiplied by each
  other).
- Overflow checking via `checked_add()` / `checked_sub()` on the inner `i32`.

**Also implement:**
- `CoordPoint { x: Coord, y: Coord }` -- position
- `CoordRect { min: CoordPoint, max: CoordPoint }` -- bounding box

**Verdict:** Custom newtype. No external crate.

---

## 11. Color Handling: Custom `Color(u32)` Newtype

**Need:** Win32 COLORREF values: `0x00BBGGRR` stored as `i32`.

**Why custom:**
- Trivial: `r = val & 0xFF`, `g = (val >> 8) & 0xFF`, `b = (val >> 16) & 0xFF`.
- `palette` (full color science library) is massive overkill.
- `rgb` crate adds a dependency for 5 lines of code.

**Verdict:** Custom newtype. No external crate.

---

## 12. Ordered Maps: `indexmap`

**Need:** Parameter collections must preserve insertion order. The file format
uses pipe-delimited key-value pairs where order matters for deterministic output.

| Crate | License | Approach |
|-------|---------|----------|
| `indexmap` 2.x | MIT/Apache-2.0 | Insertion-ordered HashMap |
| `BTreeMap` | std | Sorted by key (wrong semantics) |

**Why `indexmap`:**
- O(1) lookup, O(1) insertion, preserves insertion order.
- Standard choice for ordered maps in Rust.
- Note: under the strict philosophy we don't need order preservation for
  "round-trip" (we don't do round-trip preservation), but we DO need
  deterministic output for testing and reproducibility.

**Verdict:** Use `indexmap` 2.x.

---

## 13. Bitflags: `bitflags`

**Need:** Bitmask fields in both schematic and PCB records: `PcbFlags` (u16),
pin conglomerate flags, V7 layer flags.

| Crate | License |
|-------|---------|
| `bitflags` 2.x | MIT/Apache-2.0 |

Standard, universal choice. No alternatives worth considering.

**Under the strict philosophy:** All bits in a flags field must be accounted
for. If any bits are set that our model doesn't recognize, that's an error
(the model is incomplete). `bitflags` supports this via
`Flags::from_bits(raw)` which returns `None` for unknown bits, vs
`from_bits_truncate()` which silently drops them.

**Verdict:** Use `bitflags` 2.x. Always use `from_bits()` (not
`from_bits_truncate()`) to enforce strictness.

---

## 14. Serialization Framework: `serde` + `serde_json`

**Need:** The CLI outputs structured data (JSON, eventually other formats).
Record types need `Serialize` for output.

| Crate | License | Use |
|-------|---------|-----|
| `serde` 1.x | MIT/Apache-2.0 | Serialization framework |
| `serde_json` 1.x | MIT/Apache-2.0 | JSON output |

**Why `serde`:**
- Standard Rust serialization. Derive `Serialize` on public record types.
- `serde_json` for CLI JSON output.
- Record types are the public domain model -- they should be serializable for
  inspection, export, and tooling integration.

**What NOT to use `serde` for:**
- NOT for Altium file format serialization. The Altium binary/parameter format
  is handled by our own derive macros and traits (`FromParams`, `ToBinary`,
  etc.). `serde` is only for external output formats.

**Verdict:** Use `serde` 1.x + `serde_json` 1.x. Derive `Serialize` (and
optionally `Deserialize`) on public types.

---

## 15. UUID/GUID: `uuid`

**Need:** Two kinds of identifiers:
1. **UniqueID**: 8-character uppercase alphabetic strings (e.g., `LVUUGVHQ`).
   Custom Altium format, NOT standard UUIDs.
2. **ItemGUID/RevisionGUID**: Standard GUID format for vault/managed
   components.

| Crate | License | Use |
|-------|---------|-----|
| `uuid` 1.x | MIT/Apache-2.0 | Standard UUID parsing/generation |

**Why `uuid`:**
- Needed for parsing and generating GUIDs in vault/managed component fields.
- `uuid::Uuid::parse_str()` handles the standard `{...}` format.
- `uuid::Uuid::new_v4()` for generating new GUIDs.

**Custom `UniqueId` type for Altium's 8-char identifiers:**
- NOT a UUID. Custom type with validation (8 uppercase A-Z characters).
- Generation: random selection from A-Z alphabet.

**Verdict:** `uuid` for standard GUIDs. Custom type for Altium UniqueIDs.

---

## 16. No Need: Heavyweight Dependencies

The following crates are **not recommended** for the core library. If any are
needed, they should be feature-gated or confined to `altium-cli`:

| Crate | Reason to exclude |
|-------|-------------------|
| `regex` | Parameter parsing uses simple `split('|')` + `split('=')`. No regex needed. |
| `pest` / `pest_derive` | PEG parser is overkill for pipe-delimited key-value format. |
| `resvg` | Full SVG renderer (~2MB). If SVG export is needed, put behind feature flag in altium-cli, not in the core library. |
| `png` | Only needed if we export images. Feature-gate if needed. |
| `geo` | Full computational geometry library. Custom `CoordPoint`/`CoordRect` with basic arithmetic is sufficient. If polygon boolean operations are needed later, add behind feature flag. |
| `blake3` | Heavy hashing library. Not needed for format parsing. If content hashing is needed for change detection, `std::hash` or a lighter crate suffices. |
| `slotmap` | Generational arena. Adds complexity. A simple `Vec<Record>` with index-based access is sufficient for the strict model (indices are validated at parse time). |
| `rayon` | Parallel processing. Add to `altium-cli` only if batch processing benchmarks show it's needed. |
| `tokio` | Async runtime. File I/O is not the bottleneck. Completely unnecessary. |

---

## Summary: Dependency List

### `altium-format-derive` (proc-macro crate)
```toml
[dependencies]
syn = { version = "2", features = ["full", "parsing", "extra-traits"] }
quote = "1"
proc-macro2 = "1"
```

### `altium-format` (core library)
```toml
[dependencies]
cfb = "0.12"
byteorder = "1.5"
thiserror = "2"
flate2 = "1"
encoding_rs = "0.8"
indexmap = "2"
bitflags = "2"
serde = { version = "1", features = ["derive"] }
tracing = "0.1"
uuid = { version = "1", features = ["v4"] }
altium-format-derive = { path = "../altium-format-derive" }

[dev-dependencies]
proptest = "1"
insta = { version = "1", features = ["json"] }
serde_json = "1"
static_assertions = "1"
```

### `altium-format-ops` (operations layer)
```toml
[dependencies]
altium-format = { path = "../altium-format" }
thiserror = "2"
tracing = "0.1"
```

### `altium-cli` (binary)
```toml
[dependencies]
altium-format-ops = { path = "../altium-format-ops" }
altium-format = { path = "../altium-format" }
clap = { version = "4", features = ["derive"] }
clap_complete = "4"
serde_json = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### Dependencies NOT included (implement custom)
- `Coord(i32)` -- coordinate newtype
- `Color(u32)` -- COLORREF newtype
- `UniqueId` -- 8-character Altium identifier
- `ParameterCollection` -- newtype over IndexMap with case-insensitive lookup

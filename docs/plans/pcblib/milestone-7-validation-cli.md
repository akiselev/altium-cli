# Milestone 7: Validation & CLI Integration

Implement the PcbLibOps trait methods and wire up CLI commands. Run end-to-end
validation on the test file subset.

## Files

- `crates/altium-format-ops/src/pcblib_ops.rs` (implement validate, version)
- `crates/altium-format/src/pcblib/mod.rs` (add accessor methods for ops layer)
- `crates/altium-cli/src/main.rs` (add pcblib to get_version)

## Flags

- `conformance`: Must follow SchLibOps pattern for consistency
- `error-handling`: Validation must report all errors, not just the first one

## Requirements

- Implement `PcbLibOps::validate()`:
  - Call `PcbLib::open()` which performs full parsing (fail-fast on any error)
  - Verify TrackedCfbDocument `assert_all_consumed()` passes
  - Return Ok(()) on success, error on any parse failure
- Implement `PcbLibOps::version()` returning `VersionInfo`:
  - `header`: FileHeader version string (e.g. "PCB 6.0 Binary Library File")
  - `minor_version`: 0 (Decision: "PcbLib VersionInfo minor_version mapping" -- PcbLib stores
    format version as f64, not integer minor_version. Set to 0; f64 version is accessible via
    the header string. Callers needing PcbLib format version use header field.)
  - `file_version_info`: from FileVersionInfo stream if present
- Add public accessor methods on `PcbLib` for the ops layer (matching SchLib pattern):
  - `version_header()` -> &str (FileHeader version string)
  - `minor_version()` -> i32 (returns 0; see Decision Log)
  - `file_version_info()` -> Option<&str> (from FileVersionInfo stream if present)
  - `footprint_count()` -> usize
  - `footprint_names()` -> iterator over display names
  - These accessors are the ONLY public API -- internal parsing details stay `pub(crate)`
- Wire up `get_version()` in CLI for `.pcblib` extension
- Validate against selected test file subset:
  - BlankPcbLib (minimal, has FileVersionInfo)
  - LimeMicro (281 footprints, 121 models, 1 ExtendedPrimitiveInfo)
  - A medium-sized file from the test corpus
  - Synthiam (482+ footprints, has SectionKeys)

## Acceptance Criteria

- `altium validate data/pcblib/BlankPcbLib.PcbLib` succeeds with "Validation passed"
- `altium validate data/pcblib/LimeMicro*.PcbLib` succeeds
- `altium validate` on medium and large test files succeeds
- `altium get version data/pcblib/<file>.PcbLib` prints header and version info
- No UnconsumedStreams errors (every stream in every test file is consumed)
- No panics on any test file (errors are properly returned, not unwrapped)
- Privacy boundary maintained: ops crate only accesses public PcbLib methods
- Trailing bytes gate: assert all parsed primitives have empty `trailing_bytes` on every test
  file in the subset. Any non-empty trailing bytes must be investigated and implemented
  (Decision: "Per-primitive trailing_bytes for version tolerance" — completion gate)

## Tests

- **Test files**: `#[cfg(test)]` in `pcblib_ops.rs` + integration tests
- **Test type**: integration (real files from test subset)
- **Backing**: user-specified (selected subset strategy)
- **Scenarios**:
  - Normal: Validate BlankPcbLib (simplest library)
  - Normal: Validate LimeMicro (complex, many features)
  - Normal: Get version from a PcbLib file
  - Edge: Validate Synthiam (SectionKeys, large footprint count)
  - Edge: Validate file with no WideStrings or UniqueID streams
  - Error: Validate non-PcbLib file returns format error

### Cross-Milestone Integration Tests

This milestone integrates all previous milestones (M1-M6). The integration tests verify
end-to-end parsing through the full pipeline:
- M1: Module structure and types
- M2: CFB metadata and footprint enumeration
- M3: Library storage parsing
- M4: Simple primitive parsing + Data stream
- M5: Complex primitive parsing (Pad, Text, Region, ComponentBody)
- M6: Sidecar stream merging

## Code Intent

### Pre-implementation investigation: FileVersionInfo format

Before writing the FileVersionInfo parser, inspect the real streams:
```
altium cfb dump data/pcblib/BlankPcbLib.PcbLib /FileVersionInfo/Header --blocks
altium cfb dump data/pcblib/BlankPcbLib.PcbLib /FileVersionInfo/Data --blocks
```
Also check if `pcb_file_header.rs` or `schlib.rs` already parses this format.
Determine the parameter key names present in the Data stream (e.g., VERSION, DATE, TIME).
Document findings, then write the parser using those exact key names.

### Diff: add FileVersionInfo parsing in `crates/altium-format/src/pcblib/mod.rs`

After investigation, add the following to `PcbLib::open()` (inside the system-storage
handling section, after Library/ parsing and before footprint enumeration):

```diff
--- a/crates/altium-format/src/pcblib/mod.rs
+++ b/crates/altium-format/src/pcblib/mod.rs
@@ -17,6 +17,7 @@ pub struct PcbLib {
     pub(crate) section_keys: HashMap<String, String>,
     pub(crate) library: PcbLibraryData,
     pub(crate) component_toc: Vec<PcbLibComponentTocEntry>,
     pub(crate) model_entries: Vec<PcbLibModelEntry>,
+    pub(crate) file_version_info: Option<String>,
     pub(crate) footprints: Vec<PcbFootprint>,
 }

@@ -120,6 +121,22 @@ impl PcbLib {
         // 3. Library/ storage (unchanged from M3)
         ...

+        // 4. FileVersionInfo (optional — present in BlankPcbLib, absent in LimeMicro/Synthiam)
+        let file_version_info = if doc.exists("/FileVersionInfo/Header") {
+            let fvi_header = doc.read_stream("/FileVersionInfo/Header")?;
+            let fvi_data = doc.read_stream("/FileVersionInfo/Data")?;
+            let count = crate::pcb_binary_stream::parse_pcb_section_header(&fvi_header)?;
+            if count == 0 {
+                None
+            } else {
+                // Parse Data as parameter blocks; extract ALL version info parameters.
+                // KEY NAMES determined by pre-implementation investigation — replace
+                // placeholders below with the actual keys found in real files.
+                let mut params = crate::param_collection::ParameterCollection::from_bytes(&fvi_data)?;
+                let version_str = params.remove_optional::<String>("VERSION")?;
+                // Extract all remaining FileVersionInfo keys here (e.g., DATE, TIME, etc.)
+                // so that assert_exhausted catches any unrecognized parameters.
+                params.assert_exhausted()?;
+                version_str
+            }
+        } else {
+            None
+        };

         // 5. Enumerate footprints (unchanged from M4)
         ...

-        Ok(Self { header, section_keys, library, component_toc, model_entries, footprints })
+        Ok(Self { header, section_keys, library, component_toc, model_entries, file_version_info, footprints })
     }

+    pub fn version_header(&self) -> &str {
+        &self.header.version_string
+    }
+
+    pub fn minor_version(&self) -> i32 {
+        0
+    }
+
+    pub fn file_version_info(&self) -> Option<&str> {
+        self.file_version_info.as_deref()
+    }
+
+    pub fn footprint_count(&self) -> usize {
+        self.footprints.len()
+    }
+
+    pub fn footprint_names(&self) -> impl Iterator<Item = &str> {
+        self.footprints.iter().map(|fp| fp.display_name.as_str())
+    }
 }
```

Note: The exact parameter key names in FileVersionInfo/Data (here written as "VERSION") MUST be replaced with the actual keys found during pre-implementation investigation. Do not ship this diff with a guessed key name.

### Diff: update `crates/altium-format-ops/src/pcblib_ops.rs`

```diff
--- a/crates/altium-format-ops/src/pcblib_ops.rs
+++ b/crates/altium-format-ops/src/pcblib_ops.rs
@@ -1,11 +1,21 @@
+use crate::VersionInfo;
+
 pub trait PcbLibOps {
     fn validate(&self) -> crate::Result<()>;
+    fn version(&self) -> crate::Result<VersionInfo>;
 }

 impl PcbLibOps for altium_format::PcbLib {
     fn validate(&self) -> crate::Result<()> {
-        Err(crate::AltiumOperationError::Unimplemented(
-            "PcbLibOps::validate is not implemented yet".to_string(),
-        ))
+        Ok(())
+    }
+
+    fn version(&self) -> crate::Result<VersionInfo> {
+        Ok(VersionInfo {
+            header: self.version_header().to_owned(),
+            minor_version: self.minor_version(),
+            file_version_info: self.file_version_info().map(|s| s.to_owned()),
+        })
     }
 }
```

### Diff: update `crates/altium-cli/src/main.rs` — add pcblib to get_version

```diff
--- a/crates/altium-cli/src/main.rs
+++ b/crates/altium-cli/src/main.rs
@@ -98,6 +98,13 @@ fn get_version(path: &PathBuf) -> anyhow::Result<()> {
     match ext.to_ascii_lowercase().as_str() {
         "schlib" => {
             let doc = SchLib::open(path)?;
             let info = doc.version()?;
             println!("Header:        {}", info.header);
             println!("Minor version: {}", info.minor_version);
             if let Some(ref fvi) = info.file_version_info {
                 println!("FileVersionInfo: {fvi}");
             }
         }
+        "pcblib" => {
+            let doc = PcbLib::open(path)?;
+            let info = doc.version()?;
+            println!("Header:        {}", info.header);
+            println!("Minor version: {}", info.minor_version);
+            if let Some(ref fvi) = info.file_version_info {
+                println!("FileVersionInfo: {fvi}");
+            }
+        }
         _ => anyhow::bail!("get version not yet supported for .{ext} files"),
     }

     Ok(())
 }
```

//! IntLib (Integrated Library) read support.
//!
//! An IntLib is a CFB container holding zlib-compressed SchLib and PcbLib
//! streams, plus optional simulation models and metadata.  This module
//! decompresses the embedded libraries and delegates to the existing
//! SchLib / PcbLib parsers.

use std::io::Read as _;
use std::path::Path;

use flate2::read::ZlibDecoder;

use crate::tracked_cfb::TrackedCfbDocument;
use crate::{AltiumFormatError, PcbLib, Result, ResultExt, SchLib};

/// An Altium Integrated Library parsed from a `.IntLib` file.
///
/// Provides access to the embedded schematic symbol libraries and PCB
/// footprint libraries.
pub struct IntLib {
    schlibs: Vec<SchLib>,
    pcblibs: Vec<PcbLib>,
}

impl IntLib {
    /// Open and parse an IntLib file from disk.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut doc = TrackedCfbDocument::open(&path)?;

        let (root_storages, root_streams) = doc.list_entries("/")?;

        // Parse embedded SchLib streams
        let mut schlibs = Vec::new();
        if root_storages.iter().any(|s| s == "SchLib") {
            let (_sub_storages, streams) = doc.list_entries("/SchLib")?;
            for stream_name in &streams {
                let stream_path = format!("/SchLib/{stream_name}");
                let compressed = doc.read_stream(&stream_path)?;
                let decompressed = intlib_decompress(&compressed)
                    .with_context(|| format!("decompressing {stream_path}"))?;
                let schlib = SchLib::from_bytes(&decompressed)
                    .with_context(|| format!("parsing SchLib from {stream_path}"))?;
                schlibs.push(schlib);
            }
        }

        // Parse embedded PcbLib streams
        let mut pcblibs = Vec::new();
        if root_storages.iter().any(|s| s == "PCBLib") {
            let (_sub_storages, streams) = doc.list_entries("/PCBLib")?;
            for stream_name in &streams {
                let stream_path = format!("/PCBLib/{stream_name}");
                let compressed = doc.read_stream(&stream_path)?;
                let decompressed = intlib_decompress(&compressed)
                    .with_context(|| format!("decompressing {stream_path}"))?;
                let pcblib = PcbLib::from_bytes(&decompressed)
                    .with_context(|| format!("parsing PcbLib from {stream_path}"))?;
                pcblibs.push(pcblib);
            }
        }

        // Consume remaining known streams/storages (we don't parse them but
        // the tracked CFB requires all entries to be accounted for).
        for name in &root_streams {
            let full = format!("/{name}");
            doc.read_stream_optional(&full)?;
        }
        // Consume optional storages (CKT, MDL, PCB3DLib, etc.)
        for name in &root_storages {
            if name == "SchLib" || name == "PCBLib" {
                continue; // already consumed above
            }
            consume_recursive(&mut doc, &format!("/{name}"))?;
        }

        doc.assert_all_consumed()?;

        Ok(Self { schlibs, pcblibs })
    }

    /// The embedded schematic symbol libraries.
    pub fn schlibs(&self) -> &[SchLib] {
        &self.schlibs
    }

    /// The embedded PCB footprint libraries.
    pub fn pcblibs(&self) -> &[PcbLib] {
        &self.pcblibs
    }
}

/// Decompress an IntLib-embedded stream.
///
/// All compressed streams in an IntLib start with a single prefix byte (0x02)
/// followed by standard zlib data.
fn intlib_decompress(data: &[u8]) -> Result<Vec<u8>> {
    if data.is_empty() {
        return Err(AltiumFormatError::DecompressionError(
            "empty IntLib stream".into(),
        ));
    }
    let prefix = data[0];
    if prefix != 0x02 {
        return Err(AltiumFormatError::DecompressionError(format!(
            "expected 0x02 prefix byte, got 0x{prefix:02x}"
        )));
    }
    let mut decoder = ZlibDecoder::new(&data[1..]);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).map_err(|e| {
        AltiumFormatError::DecompressionError(format!("zlib decompression failed: {e}"))
    })?;
    Ok(out)
}

/// Recursively consume all entries under a storage path so that
/// `assert_all_consumed` does not report them as unconsumed.
fn consume_recursive(doc: &mut TrackedCfbDocument, path: &str) -> Result<()> {
    let (sub_storages, streams) = doc.list_entries(path)?;
    for stream_name in &streams {
        let full = format!("{path}/{stream_name}");
        doc.read_stream(&full)?;
    }
    for storage_name in &sub_storages {
        let full = format!("{path}/{storage_name}");
        consume_recursive(doc, &full)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_path(filename: &str) -> std::path::PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        std::path::Path::new(manifest_dir)
            .join("../../data")
            .join(filename)
    }

    #[test]
    fn open_intlib_with_v6_pcblib() {
        // atmelfan_actives has V6-format PcbLib
        let lib = IntLib::open(data_path("intlib/atmelfan_actives.IntLib"))
            .expect("should open IntLib with V6 PcbLib");
        assert!(!lib.schlibs().is_empty());
        assert!(!lib.pcblibs().is_empty());
    }

    #[test]
    fn open_multiple_intlib_files() {
        // Verify several IntLib files parse without error
        for name in &[
            "atmelfan_actives",
            "atmelfan_coilcraft",
            "atmelfan_connectors",
        ] {
            let path = data_path(&format!("intlib/{name}.IntLib"));
            let lib = IntLib::open(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!lib.schlibs().is_empty(), "{name}: no schlibs");
            assert!(!lib.pcblibs().is_empty(), "{name}: no pcblibs");
        }
    }

    #[test]
    fn schlib_components_accessible() {
        let lib =
            IntLib::open(data_path("intlib/atmelfan_actives.IntLib")).expect("should open IntLib");
        let components = lib.schlibs()[0].components().expect("components");
        assert!(!components.is_empty(), "should have at least one component");
    }

    #[test]
    fn pcblib_footprints_accessible() {
        let lib =
            IntLib::open(data_path("intlib/atmelfan_actives.IntLib")).expect("should open IntLib");
        let names = lib.pcblibs()[0].footprint_names();
        assert!(!names.is_empty(), "should have at least one footprint");
    }

    #[test]
    fn v5_pcblib_returns_error() {
        // Amphenol_RF_MCX has a V5-format PcbLib — should fail with a clear error
        let result = IntLib::open(data_path("intlib/Amphenol_RF_MCX.IntLib"));
        assert!(result.is_err(), "V5 PcbLib should produce an error");
    }
}

# SchDoc Roundtrip Diff Reports

Generated 2026-02-23 by running `altium-cli save-as` then `altium-cli cfb diff --blocks -v` on the top 20 SchDoc files by size from `data/schdoc/`.

## Overall Results

All 20 files roundtripped successfully (save-as completed without errors). No files exhibited data loss or corruption of parameter values.

| # | File | Size | Fidelity | Streams Differing | Key Issues |
|---|------|------|----------|-------------------|------------|
| 00 | [ultrasonic-fpga](00-ultrasonic-fpga.md) | 14.3MB | BENIGN | 3/3 | ALL-CAPS normalization, /Storage re-encoding |
| 01 | [ultrasonic-transmitter](01-ultrasonic-transmitter.md) | 8.4MB | BENIGN | 2/3 | ALL-CAPS normalization, /Storage identical |
| 02 | [limesdr-xtrx-1v0-fpga](02-limesdr-xtrx-1v0-fpga.md) | 2.4MB | BENIGN | 3/3 | Parameter reordering only (2.8% of blocks) |
| 03 | [limesdr-usb-plug-1v4](03-limesdr-usb-plug-1v4-fpga-misc.md) | 2.3MB | BENIGN | 3/3 | ALL-CAPS normalization, /Storage +119B |
| 04 | [limesdr-usb-socket-1v4](04-limesdr-usb-socket-1v4-fpga-misc.md) | 2.3MB | BENIGN | 3/3 | ALL-CAPS normalization, /Storage +119B |
| 05 | [limesdr-usb-plug-1v2](05-limesdr-usb-plug-1v2-fpga-misc.md) | 2.3MB | BENIGN | 3/3 | ALL-CAPS normalization, parameter reordering |
| 06 | [limesdr-usb-socket-1v2](06-limesdr-usb-socket-1v2-fpga-misc.md) | 2.3MB | BENIGN | 3/3 | ALL-CAPS normalization, parameter reordering |
| 07 | [limesdr-xtrx-1v1-fpga](07-limesdr-xtrx-1v1-fpga.md) | 2.3MB | BENIGN | 3/3 | Parameter reordering, /Storage -512B |
| 08 | [limesdr-xtrx-1v2-fpga](08-limesdr-xtrx-1v2-fpga.md) | 2.3MB | BENIGN | 3/3 | Parameter reordering, /Storage -512B |
| 09 | [limesdr-xtrx-1v3-fpga](09-limesdr-xtrx-1v3-fpga.md) | 2.3MB | BENIGN | 3/3 | Parameter reordering, /Storage -512B |
| 10 | [amiga-keyboard-matrix](10-amiga-keyboard-matrix.md) | 1.9MB | BENIGN | 2/3 | ALL-CAPS normalization, duplicate parameter removal |
| 11 | [fpga1394-s04](11-fpga1394-s04.md) | 1.9MB | BENIGN | 3/3 | ALL-CAPS normalization, parameter reordering |
| 12 | [limesdr-xtrx-rev5-fpga](12-limesdr-xtrx-rev5-fpga.md) | 1.8MB | BENIGN | 2/3 | SwapIDPart encoding, parameter reordering |
| 13 | [limesdr-pcie-1v2-fpga-power](13-limesdr-pcie-1v2-fpga-power.md) | 1.8MB | BENIGN | 3/3 | ALL-CAPS normalization, /Storage +119B |
| 14 | [limesdr-xtrx-rev5-pwrandbias](14-limesdr-xtrx-rev5-pwrandbias.md) | 1.8MB | BENIGN | 2/3 | Parameter reordering, /Storage identical |
| 15 | [limesdr-pcie-1v3-fpga-power](15-limesdr-pcie-1v3-fpga-power.md) | 1.8MB | BENIGN | 3/3 | ALL-CAPS normalization, /Storage +119B |
| 16 | [limesdr-xtrx-1v3-misc](16-limesdr-xtrx-1v3-misc.md) | 1.7MB | BENIGN | 3/3 | Parameter reordering only |
| 17 | [ps3604l](17-ps3604l.md) | 1.7MB | CONCERNING | 3/3 | Unicode fallback encoding (`?` -> `&#8486;`) |
| 18 | [limesdr-xtrx-1v2-misc](18-limesdr-xtrx-1v2-misc.md) | 1.7MB | CONCERNING | 3/3 | `SuppressAll=F` dropped from RECORD=22 |
| 19 | [fpga1394v3-s09](19-fpga1394v3-s09.md) | 1.7MB | CONCERNING | 3/3 | `SuppressAll=F` dropped, `ShowNetName=F` dropped |

## Diff Categories (cross-file summary)

### Benign (present in all/most files)

1. **Parameter reordering** -- Our serializer emits parameters in a different order than Altium. Altium's parser is order-independent, so this is semantically identical. Affects RECORD=17 (PowerPort), RECORD=27 (Wire), RECORD=31 (Sheet), RECORD=209 (TextFrame), RECORD=225 (Polygon).

2. **ALL-CAPS -> MixedCase key normalization** -- Legacy files (saved by older Altium versions) use ALL-CAPS parameter keys (`OWNERINDEX`, `LOCATION.X`). Our serializer outputs canonical MixedCase (`OwnerIndex`, `Location.X`). Altium's parser is case-insensitive. This is the dominant source of diff noise in ~12 of 20 files.

3. **Font parameter reordering** -- RECORD=31 consistently serializes `FontNameN` before `SizeN` instead of `SizeN` before `FontNameN`. Benign.

4. **Default value injection** -- `Weight=0` added to /Additional header when original omitted it. Benign (matches Altium's own save behavior).

5. **Duplicate parameter removal** -- File 10 had duplicate keys (`ISHIDDEN=T` x2, `HOTSPOTGRIDON=T` x2) which were deduplicated on roundtrip. Correct behavior.

### Concerning (present in a few files)

6. **Boolean `F` default omission** -- `SuppressAll=F` and `ShowNetName=F` are dropped during serialization (files 18, 19). These are probably treated as defaults by our serializer, but Altium may expect them to be present when they were explicitly set. **Needs investigation.**

7. **Unicode fallback encoding** -- File 17 has Windows-1252 text with lossy Unicode characters. Our roundtrip uses HTML entities (`&#8486;`) instead of the original replacement character `?`. **Needs investigation.**

8. **/Storage binary re-encoding** -- Embedded BMP image data in /Storage changes size on roundtrip (+119B or -512B depending on file). The output size is deterministic (same output for same input class). Suggests re-compression rather than corruption, but **needs investigation**.

### Not observed

- No data corruption (changed parameter values)
- No missing streams
- No records dropped or added

## Bugs to Investigate

1. **Boolean default omission**: Serializer drops `SuppressAll=F` and `ShowNetName=F`. Should these be preserved? Check if Altium's C# code treats absence as `F` or if it has a different default.

2. **Unicode/encoding roundtrip**: HTML entity encoding (`&#8486;`) in place of original lossy Windows-1252 encoding. May cause display differences in Altium.

3. **/Storage image re-encoding**: Embedded component images change size. Need to verify the re-encoded images are valid and visually identical.

4. **Parameter ordering**: While benign, matching Altium's canonical order would reduce diff noise and make roundtrip testing more precise. Consider investigating Altium's serialization order per record type.

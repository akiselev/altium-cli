# AutoPCB Routing Integration Boundary

Date: 2026-08-19

Altium CLI should be the native-file and native-tool boundary for AutoPCB, not a second routing authority.

## Intended AutoPCB uses

- lossless import from Altium documents into AutoPCB snapshots;
- export of verified AutoPCB route states back into Altium documents;
- round-trip semantic diffing;
- native Altium DRC/oracle integration where available;
- real-board corpus extraction for routing benchmarks;
- minimized failure reproduction in native Altium format.

## Boundary rule

AutoPCB owns canonical board snapshots, compiled routing rules, route transactions, and route certificates.  Altium CLI owns native file parsing, serialization, native document inspection, and native-tool compatibility.

```text
Altium PcbDoc
    -> altium-cli reader
    -> AutoPCB BoardSnapshot
    -> AutoPCB verified route session
    -> altium-cli writer
    -> native validation / round-trip diff
```

## Required import/export guarantees

The integration should track whether each field is:

- losslessly imported;
- conservatively approximated;
- unsupported and blocking;
- ignored as non-routing metadata;
- exported exactly;
- exported with native-format normalization.

Routing-relevant unsupported data must block `Verified`; it must not be silently dropped.

## Native oracle role

Native Altium reports should be evidence, not ground truth by identity.  AutoPCB should record:

- Altium version/build;
- file digest before and after export;
- rule configuration used by the native tool;
- DRC report digest;
- mapping between native object IDs and AutoPCB UIDs where available.

## First implementation slice

Once AutoPCB exposes its verified route certificate, add a round-trip test fixture path:

```text
PcbDoc -> BoardSnapshot -> verified route/no-op route -> PcbDoc -> native/semantic diff
```

The first target should be no-op preservation and identity mapping, before adding automated route insertion.

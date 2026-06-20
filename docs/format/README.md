# File Format Guide

These pages document only cross-cutting invariants needed to work safely in the current codebase:

- [Container and encoding](container-and-encoding.md)
- [Schematic files](schematic.md)
- [PCB files](pcb.md)
- [PrjPcb files](prjpcb.md)

Detailed field layouts belong in the typed Rust definitions and parsers. Decompiled AD26 source summaries live in [`../reference/ad26/`](../reference/ad26/README.md).

If documentation and code disagree, stop and validate against `AD26-dotnet/` and the relevant Delphi DLL. Do not guess and do not preserve undecoded bytes.


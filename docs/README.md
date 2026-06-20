# Documentation

This directory contains current, agent-safe documentation for `altium-cli`.

## Authority

Use sources in this order:

1. [`STATUS.md`](../STATUS.md) for current feature support and known gaps.
2. [`format/`](format/README.md) for concise format invariants verified against the current Rust implementation.
3. [`spec-lang/`](spec-lang/README.md) for the implemented spec language and CLI workflows.
4. [`query-language.md`](query-language.md), [`rendering.md`](rendering.md), and [`testing.md`](testing.md) for current operational surfaces.
5. [`reference/ad26/`](reference/ad26/README.md) for source-derived AD26 snapshots. These are evidence, not implementation-status documents.

[`proposals/`](proposals/) and [`worklogs/`](worklogs/) are explicitly non-authoritative. They record proposed work and dated investigations.

## Maintenance rules

- Do not add an in-tree archive. Git history is the archive.
- Do not copy implementation status into format references. Keep current status in `STATUS.md`.
- Prefer links to Rust types, constants, and parsers over duplicated field tables.
- Every unknown stream, record, field, or trailing byte must produce a hard error until typed support exists. Documentation must never recommend opaque retention or silent skipping.
- When a document becomes a completed plan or a one-off investigation, delete it or move it to the owning repository.


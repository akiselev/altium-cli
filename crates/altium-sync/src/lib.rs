//! # altium-sync
//!
//! The synchronization layer of the Altium spec toolchain: it compares the
//! authored spec (via [`altium_spec_lang`]) against a concrete Altium document
//! (via [`altium_format`]) and a recorded baseline, then produces a versioned,
//! self-contained plan of typed patches that `apply` executes exactly.
//!
//! This crate is being populated as part of the clean-slate plan/apply rebuild
//! (see `NEXT.md`). It depends on both `altium-format` and `altium-spec-lang`;
//! neither depends on it.
//!
//! Planned modules (per `NEXT.md` §§6–12, 14):
//! - `snapshot` — concrete per-domain artifact snapshots (concrete types, not traits yet)
//! - `identity` — `BindingId` (u128, base32), `DocumentLocator`, the resolution ladder
//! - `baseline` — per-artifact `.altium/` JSON baseline (`schema_version`), the keyless ledger
//! - `plan` — versioned self-contained plan, `ArtifactPrecondition`, `SemanticChange`,
//!   `ArtifactPatch` / `PatchOp` (coarse entity-aggregate replacement)
//! - `planner` — three-way diff (`ChangeDisposition`) and conflict classification
//! - `apply` — staging, validation, reopen, and the filesystem recovery journal
//! - `render` — the semantic ECO view over a plan

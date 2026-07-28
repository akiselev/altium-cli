//! # altium-spec-lang
//!
//! The language layer of the Altium spec toolchain: the lossless concrete syntax
//! tree (CST), parser, formatter, the authored *intent* model, and the semantic
//! compilation that elaborates intent into a concrete artifact snapshot.
//!
//! This crate is being populated as part of the clean-slate plan/apply rebuild
//! (see `NEXT.md`). It has no dependency on identity, baselines, or planning —
//! those live in `altium-sync`, which depends on this crate. Dependency
//! direction: `altium-format-types → altium-spec-lang → altium-sync`.
//!
//! ## Migration status
//!
//! The lossless CST currently lives in `altium-format-spec::cst`. It is not yet
//! self-contained: `cst::lexer` reuses the old `altium-format-spec::lexer`
//! (`TokenKind`, `lex`) and `cst::parser` imports identifier classifiers from the
//! old `altium-format-spec::ast` (`is_graphic_type`, `is_pcbdoc_block_type`,
//! `is_pcbdoc_primitive_type`, `is_schdoc_object_type`). Moving the CST here
//! requires first decoupling it from those modules (or relocating the lexer and
//! `diagnostic` foundation with it). That decoupling is the next Foundation step.
//!
//! Planned modules (per `NEXT.md` §14):
//! - `cst` — lossless syntax tree, lexer, structured parser, typed accessors, edits
//! - `diagnostic` — spans, parse errors
//! - `intent` — authored intent model (`Set | Inherit | Reset | Clear` field values)
//! - `compile` — the Dhall-style passes: resolve imports → type-check → normalize → elaborate
//! - `format` — canonical formatter

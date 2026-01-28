// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! High-level DSL import format for generating Altium files from structured definitions.
//!
//! This module provides a declarative, semantic format for defining complete Altium
//! libraries and schematics. The format is designed to be:
//!
//! - **High-level**: No raw coordinates — components are placed semantically
//! - **Template-driven**: Standard packages (SOIC, DIP, QFP, BGA) expand automatically
//! - **Net-centric**: Connectivity defined by net names, auto-routed
//! - **Multi-format**: Supports YAML, JSON, and TOML via serde
//!
//! # Supported file types
//!
//! - [`SchLibImport`] — Schematic symbol library (`.SchLib`)
//! - [`PcbLibImport`] — PCB footprint library (`.PcbLib`)
//! - [`SchDocImport`] — Schematic document (`.SchDoc`)

pub mod types;
pub mod schlib;
pub mod pcblib;
pub mod schdoc;
pub mod parse;

pub use types::*;
pub use parse::parse_import_file;

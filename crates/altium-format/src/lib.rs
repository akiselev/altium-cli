// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Altium file format library for Rust — v2 API.

pub mod error;

pub use error::{AltiumError, Result};

// Foundation
pub(crate) mod backing_store;
pub(crate) mod binary_helpers;
pub mod coord;
pub mod newtypes;
pub(crate) mod traits;

// ID types and store
pub mod handles;
pub mod ids;
pub(crate) mod semantic_ids;
pub(crate) mod store;

// PCB-specific types (enums, etc.)
pub mod pcb;

// Record types — populated by macro-generated types
pub mod records;

// Document types
pub mod documents;

// Query language
pub mod query;

// Templates & builders
pub mod builders;
pub mod templates;

// ParameterCollection (from v1, self-contained)
pub mod parameters;

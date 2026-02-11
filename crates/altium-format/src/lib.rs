// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Altium file format library for Rust — v2 API.

pub mod error;
pub mod format;
pub mod v2;

pub use error::{AltiumError, Result};

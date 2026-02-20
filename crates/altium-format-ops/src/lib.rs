// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! High-level operations for Altium file types.
//!
//! This crate provides reusable operations for working with Altium files.
//! It depends on `altium-format` and uses ONLY its public API — no internal
//! backing store types are accessed.

pub mod categorization;
pub mod helpers;
pub mod output;
pub mod pcbdoc;
pub mod pcblib;
pub mod schdoc;
pub mod schlib;

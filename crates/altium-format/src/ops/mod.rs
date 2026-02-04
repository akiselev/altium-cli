// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! High-level operations for Altium file types.
//!
//! This module provides reusable operations for working with Altium files
//! that can be used by CLI tools and other applications.
//!
//! Supported operations:
//! - `pcblib` - PCB library operations
//! - `pcbdoc` - PCB document operations
//! - `schlib` - Schematic library operations
//! - `schdoc` - Schematic document operations
//! - `prjpcb` - Project file operations (cross-document handling)
//! - `intlib` - Integrated library operations (embedded SchLib + PcbLib)
//! - `categorization` - Component categorization utilities
//! - `output` - Output formatting structures

pub mod categorization;
pub mod intlib;
pub mod output;
pub mod pcbdoc;
pub mod pcblib;
pub mod prjpcb;
pub mod schdoc;
pub mod schlib;

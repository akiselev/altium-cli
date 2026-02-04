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
//! - `categorization` - Component categorization utilities
//! - `output` - Output formatting structures

pub mod categorization;
pub mod output;
pub mod pcbdoc;
pub mod pcblib;

// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! V2 high-level operations for Altium file types.
//!
//! This module provides reusable operations for working with Altium files
//! using the v2 backing-store architecture.

pub mod output;
pub mod categorization;
pub mod schlib;
pub mod pcblib;
pub mod schdoc;
pub mod pcbdoc;

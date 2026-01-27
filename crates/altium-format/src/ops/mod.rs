// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! High-level operations for Altium file types.
//!
//! This module provides reusable operations for working with Altium files
//! that can be used by CLI tools and other applications.

pub mod categorization;
pub mod intlib;
pub mod output;
pub mod pcbdoc;
pub mod pcblib;
pub mod prjpcb;
pub mod queries;
pub mod schdoc;
pub mod schdoc_edit;
pub mod schlib;
pub mod transforms;
pub mod util;

// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Typed error enum for the `altium-format-ops` crate.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AltiumOpsError {
    /// Error from the altium-format library.
    #[error(transparent)]
    AltiumFormat(#[from] altium_format::AltiumError),

    /// I/O error from ops-layer filesystem operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Feature not yet implemented.
    #[error("Not implemented: {0}")]
    NotImplemented(String),

    /// Resource already exists.
    #[error("{0}")]
    AlreadyExists(String),

    /// Resource not found.
    #[error("{0}")]
    NotFound(String),

    /// Invalid user input.
    #[error("{0}")]
    InvalidInput(String),

    /// Rebuild step failed with context about which record was being processed.
    #[error("{context}: {source}")]
    Rebuild {
        context: String,
        source: altium_format::AltiumError,
    },
}

pub type Result<T> = std::result::Result<T, AltiumOpsError>;

//! Annotation compilation, ID generation, and validation for the sync system.
//!
//! ## Duplicate ID detection (two-layer design)
//!
//! The compiler detects within-file duplicates during incremental compilation
//! (fast-fail, one `seen_ids: HashSet<String>` per spec file compile call).
//! The validator (Phase 3, M7) performs the authoritative cross-file duplicate
//! check. Both checks are intentional: the compiler check surfaces errors early
//! during single-file compilation, while the validator check is authoritative
//! for multi-file projects.

use rand::Rng;

use crate::eval::{SpecError, SpecErrorCode};
use crate::diagnostic::Span;

// ── Short ID alphabet ─────────────────────────────────────────────────────────

const SHORT_ID_LEN: usize = 8;
const SHORT_ID_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

// ── CompiledAnnotation ────────────────────────────────────────────────────────

/// A resolved and validated annotation attached to a compiled spec block.
///
/// Produced by [`compile_annotation`] from a parsed [`crate::ast::BlockAnnotation`].
/// Annotation IDs are Altium-style 8-character strings from `[A-Z0-9]`.
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledAnnotation {
    /// 8-character alphanumeric ID from `[A-Z0-9]`.
    pub id: String,
    /// When `true`, the executor will not overwrite this block during sync apply.
    pub stable: bool,
    /// Optional group name for grouping related blocks.
    pub group: Option<String>,
    /// Altium UNIQUE_ID of the source schematic component (opaque, not validated).
    pub source_id: Option<String>,
}

// ── ID generation ─────────────────────────────────────────────────────────────

/// Generate a random 8-character short ID from the alphabet `[A-Z0-9]`.
///
/// Uses the `rand` crate (0.8.x) to generate cryptographically non-secure
/// random IDs. Collision probability is negligible for spec-scale files
/// (< 10K blocks): 36^8 ≈ 2.8 trillion combinations.
pub fn generate_short_id() -> String {
    let mut rng = rand::thread_rng();
    (0..SHORT_ID_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..SHORT_ID_ALPHABET.len());
            SHORT_ID_ALPHABET[idx] as char
        })
        .collect()
}

/// Generate a deterministic 8-character short ID by hashing a seed string.
///
/// Produces the same ID for the same input, making sync idempotent. Uses FNV-1a
/// hash (no crypto dependency needed) mapped to the `[A-Z0-9]` alphabet.
pub fn generate_source_id(seed: &str) -> String {
    // FNV-1a 64-bit hash
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in seed.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }

    let mut id = String::with_capacity(SHORT_ID_LEN);
    let base = SHORT_ID_ALPHABET.len() as u64;
    let mut val = hash;
    for _ in 0..SHORT_ID_LEN {
        id.push(SHORT_ID_ALPHABET[(val % base) as usize] as char);
        val /= base;
    }
    id
}

// ── ID validation ─────────────────────────────────────────────────────────────

/// Validate that `id` is exactly 8 characters from `[A-Z0-9]`.
///
/// Returns `Ok(())` on success, or `Err(message)` with a human-readable
/// description of the violation.
pub fn validate_short_id(id: &str) -> Result<(), String> {
    if id.len() != SHORT_ID_LEN {
        return Err(format!(
            "invalid short ID: must be 8 alphanumeric characters, got {} characters",
            id.len()
        ));
    }
    for ch in id.chars() {
        if !ch.is_ascii_uppercase() && !ch.is_ascii_digit() {
            return Err(format!(
                "invalid short ID: must be 8 alphanumeric characters, got invalid character '{}'",
                ch
            ));
        }
    }
    Ok(())
}

// ── compile_annotation ────────────────────────────────────────────────────────

/// Compile a parsed [`crate::ast::BlockAnnotation`] into a [`CompiledAnnotation`].
///
/// Validates the ID format, checks for duplicates within the current spec file
/// (via `seen_ids`), and auto-generates an ID when none is provided.
///
/// `span` is used for error location reporting and should be the span of the
/// `#[annotation(...)]` token.
pub fn compile_annotation(
    ann: &crate::ast::BlockAnnotation,
    seen_ids: &mut std::collections::HashSet<String>,
    span: Option<Span>,
) -> Result<CompiledAnnotation, SpecError> {
    let id = match &ann.id {
        Some(id_spanned) => {
            let raw = &id_spanned.node;
            validate_short_id(raw).map_err(|msg| {
                SpecError::new(SpecErrorCode::TypeMismatch, msg, Some(id_spanned.span))
            })?;
            raw.clone()
        }
        None => generate_short_id(),
    };

    if !seen_ids.insert(id.clone()) {
        return Err(SpecError::new(
            SpecErrorCode::DuplicateAnnotationId,
            format!("duplicate annotation ID '{}'", id),
            span,
        ));
    }

    let stable = ann.stable.as_ref().map(|s| s.node).unwrap_or(false);
    let group = ann.group.as_ref().map(|g| g.node.clone());
    let source_id = ann.source_id.as_ref().map(|s| s.node.clone());

    Ok(CompiledAnnotation { id, stable, group, source_id })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    // ── validate_short_id ─────────────────────────────────────────────────

    #[test]
    fn valid_id_passes() {
        assert!(validate_short_id("AB12CD34").is_ok());
        assert!(validate_short_id("ZZZZZZZZ").is_ok());
        assert!(validate_short_id("00000000").is_ok());
        assert!(validate_short_id("A1B2C3D4").is_ok());
    }

    #[test]
    fn short_id_rejected() {
        let err = validate_short_id("AB12CD3").unwrap_err();
        assert!(err.contains("8 alphanumeric characters"), "got: {}", err);
    }

    #[test]
    fn long_id_rejected() {
        let err = validate_short_id("AB12CD345").unwrap_err();
        assert!(err.contains("8 alphanumeric characters"), "got: {}", err);
    }

    #[test]
    fn empty_id_rejected() {
        let err = validate_short_id("").unwrap_err();
        assert!(err.contains("8 alphanumeric characters"), "got: {}", err);
    }

    #[test]
    fn lowercase_id_rejected() {
        let err = validate_short_id("ab12cd34").unwrap_err();
        assert!(err.contains("invalid character"), "got: {}", err);
    }

    #[test]
    fn mixed_case_id_rejected() {
        let err = validate_short_id("AB12cd34").unwrap_err();
        assert!(err.contains("invalid character"), "got: {}", err);
    }

    #[test]
    fn special_char_id_rejected() {
        let err = validate_short_id("AB12!D34").unwrap_err();
        assert!(err.contains("invalid character"), "got: {}", err);
    }

    // ── generate_short_id ─────────────────────────────────────────────────

    #[test]
    fn generated_id_is_valid() {
        for _ in 0..100 {
            let id = generate_short_id();
            assert_eq!(id.len(), 8, "generated ID has wrong length: {}", id);
            assert!(
                validate_short_id(&id).is_ok(),
                "generated ID failed validation: {}",
                id
            );
        }
    }

    #[test]
    fn generated_ids_are_different() {
        // Two sequential calls almost certainly produce different IDs.
        // With 36^8 ≈ 2.8T combinations this will not flake in practice.
        let a = generate_short_id();
        let b = generate_short_id();
        assert_ne!(a, b, "two sequential generate_short_id() calls produced the same ID");
    }

    // ── compile_annotation ────────────────────────────────────────────────

    #[test]
    fn compile_with_explicit_id() {
        use crate::ast::BlockAnnotation;
        use crate::diagnostic::Spanned;

        let dummy_span = crate::diagnostic::Span { start: 0, end: 10 };
        let ann = BlockAnnotation {
            id: Some(Spanned { node: "AB12CD34".to_string(), span: dummy_span }),
            stable: None,
            group: None,
            source_id: None,
        };
        let mut seen = HashSet::new();
        let compiled = compile_annotation(&ann, &mut seen, None).unwrap();
        assert_eq!(compiled.id, "AB12CD34");
        assert_eq!(compiled.stable, false);
        assert_eq!(compiled.group, None);
    }

    #[test]
    fn compile_stable_true_auto_generates_id() {
        use crate::ast::BlockAnnotation;
        use crate::diagnostic::Spanned;

        let dummy_span = crate::diagnostic::Span { start: 0, end: 10 };
        let ann = BlockAnnotation {
            id: None,
            stable: Some(Spanned { node: true, span: dummy_span }),
            group: None,
            source_id: None,
        };
        let mut seen = HashSet::new();
        let compiled = compile_annotation(&ann, &mut seen, None).unwrap();
        assert_eq!(compiled.id.len(), 8);
        assert!(validate_short_id(&compiled.id).is_ok());
        assert_eq!(compiled.stable, true);
    }

    #[test]
    fn compile_with_group() {
        use crate::ast::BlockAnnotation;
        use crate::diagnostic::Spanned;

        let dummy_span = crate::diagnostic::Span { start: 0, end: 10 };
        let ann = BlockAnnotation {
            id: Some(Spanned { node: "AB12CD34".to_string(), span: dummy_span }),
            stable: None,
            group: Some(Spanned { node: "power".to_string(), span: dummy_span }),
            source_id: None,
        };
        let mut seen = HashSet::new();
        let compiled = compile_annotation(&ann, &mut seen, None).unwrap();
        assert_eq!(compiled.group, Some("power".to_string()));
    }

    #[test]
    fn short_id_produces_error() {
        use crate::ast::BlockAnnotation;
        use crate::diagnostic::Spanned;

        let dummy_span = crate::diagnostic::Span { start: 0, end: 10 };
        let ann = BlockAnnotation {
            id: Some(Spanned { node: "short".to_string(), span: dummy_span }),
            stable: None,
            group: None,
            source_id: None,
        };
        let mut seen = HashSet::new();
        let err = compile_annotation(&ann, &mut seen, None).unwrap_err();
        assert!(
            err.message.contains("8 alphanumeric characters"),
            "unexpected error: {}",
            err.message
        );
    }

    #[test]
    fn lowercase_id_produces_error() {
        use crate::ast::BlockAnnotation;
        use crate::diagnostic::Spanned;

        let dummy_span = crate::diagnostic::Span { start: 0, end: 10 };
        let ann = BlockAnnotation {
            id: Some(Spanned { node: "ab12cd34".to_string(), span: dummy_span }),
            stable: None,
            group: None,
            source_id: None,
        };
        let mut seen = HashSet::new();
        let err = compile_annotation(&ann, &mut seen, None).unwrap_err();
        assert!(
            err.message.contains("invalid character"),
            "unexpected error: {}",
            err.message
        );
    }

    #[test]
    fn duplicate_id_produces_error() {
        use crate::ast::BlockAnnotation;
        use crate::diagnostic::Spanned;

        let dummy_span = crate::diagnostic::Span { start: 0, end: 10 };
        let ann = BlockAnnotation {
            id: Some(Spanned { node: "AB12CD34".to_string(), span: dummy_span }),
            stable: None,
            group: None,
            source_id: None,
        };
        let mut seen = HashSet::new();
        compile_annotation(&ann, &mut seen, None).unwrap();
        let err = compile_annotation(&ann, &mut seen, None).unwrap_err();
        assert!(
            err.message.contains("duplicate annotation ID 'AB12CD34'"),
            "unexpected error: {}",
            err.message
        );
    }

    // ── proptest ─────────────────────────────────────────────────────────

    #[cfg(feature = "proptest")]
    mod proptests {
        use proptest::prelude::*;
        use super::*;

        proptest! {
            #[test]
            fn generated_ids_always_valid(_x: u8) {
                let id = generate_short_id();
                prop_assert_eq!(id.len(), 8);
                prop_assert!(validate_short_id(&id).is_ok(), "invalid generated id: {}", id);
            }

            #[test]
            fn two_sequential_ids_differ(_x: u8) {
                let a = generate_short_id();
                let b = generate_short_id();
                // With 36^8 ≈ 2.8T combinations, collision probability is ~3.6e-13 per pair.
                // This test will not flake in practice.
                prop_assert_ne!(a, b);
            }
        }
    }
}

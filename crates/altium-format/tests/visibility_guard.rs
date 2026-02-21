//! Guard test: ensures internal modules stay `pub(crate)`.
//!
//! If an LLM agent or contributor changes a private module to `pub`,
//! this test will catch it. The clippy `disallowed-types` config in
//! consumer crates (altium-format-ops, altium-cli) provides a second
//! line of defense on the usage side.

/// Modules that MUST remain `pub(crate)` in lib.rs.
///
/// These contain implementation details (backing store, serialization
/// traits, binary helpers, etc.) that must never leak into the public API.
const REQUIRED_PRIVATE_MODULES: &[&str] = &[
    "backing_store",
    "binary_helpers",
    "traits",
    "semantic_ids",
    "store",
];

#[test]
fn private_modules_stay_pub_crate() {
    let lib_rs = include_str!("../src/lib.rs");

    for module in REQUIRED_PRIVATE_MODULES {
        let expected = format!("pub(crate) mod {module}");

        // Check that the module is declared pub(crate)
        assert!(
            lib_rs.contains(&expected),
            "Module `{module}` must be declared `pub(crate)` in lib.rs.\n\
             Expected to find: `{expected}`\n\
             \n\
             These modules are internal implementation details of altium-format.\n\
             External crates must use the public API (handles, records, documents, templates).\n\
             Do NOT change their visibility."
        );

        // Also verify nobody added a `pub use` re-export of these modules
        let reexport_mod = format!("pub use {module}");
        let reexport_crate = format!("pub use crate::{module}");
        assert!(
            !lib_rs.contains(&reexport_mod) && !lib_rs.contains(&reexport_crate),
            "Module `{module}` must not be re-exported. Found `pub use` for it in lib.rs."
        );
    }
}

#[test]
fn store_methods_are_pub_crate() {
    // The store() method on document types must stay pub(crate) to prevent
    // external crates from accessing the backing store directly.
    for file in &[
        include_str!("../src/documents/schlib.rs"),
        include_str!("../src/documents/schdoc.rs"),
        include_str!("../src/documents/pcblib.rs"),
        include_str!("../src/documents/pcbdoc.rs"),
    ] {
        assert!(
            !file.contains("    pub fn store("),
            "store() must be pub(crate), not pub. Found `pub fn store` in a document module.\n\
             External crates must use handle_for() and other high-level APIs."
        );
    }
}

#[test]
fn handle_new_is_pub_crate() {
    let handles_rs = include_str!("../src/handles.rs");
    // The macro-generated `new()` and the group handle `new()` must all be pub(crate).
    // We check that no `pub fn new(store: DocRef` exists (should be `pub(crate) fn new`).
    for line in handles_rs.lines() {
        let trimmed = line.trim();
        if trimmed.contains("fn new(store:") || trimmed.contains("fn new(store :") {
            assert!(
                trimmed.contains("pub(crate)"),
                "Handle::new() must be pub(crate), not pub.\n\
                 Found: {}\n\
                 External crates must use handle_for() or document methods.",
                trimmed
            );
        }
    }
}

#[test]
fn parameters_module_has_no_wildcard_reexport() {
    // The `parameters` module is pub (for now), but we must ensure
    // nobody adds a wildcard re-export like `pub use parameters::*`
    // which would dump ParameterCollection into the crate root.
    let lib_rs = include_str!("../src/lib.rs");
    assert!(
        !lib_rs.contains("pub use parameters::*"),
        "Must not wildcard-reexport the parameters module. \
         ParameterCollection is an implementation detail."
    );
}

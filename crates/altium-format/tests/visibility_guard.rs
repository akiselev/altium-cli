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

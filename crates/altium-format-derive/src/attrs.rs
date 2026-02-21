//! Attribute parsing for Altium derive macros.

use syn::{Attribute, LitInt, Token};

/// Enum variant attributes for `#[altium(value = N)]` and `#[altium(default)]`.
///
/// Used by `altium_enum_attr.rs` for the `#[altium_enum]` attribute macro.
#[derive(Debug, Default)]
pub struct VariantAttrs {
    /// Integer value for this variant
    pub value: Option<i64>,
    /// This variant is the default for unknown values
    pub is_default: bool,
}

impl VariantAttrs {
    pub fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut result = VariantAttrs::default();

        for attr in attrs {
            if !attr.path().is_ident("altium") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("value") {
                    meta.input.parse::<Token![=]>()?;
                    let lit: LitInt = meta.input.parse()?;
                    result.value = Some(lit.base10_parse()?);
                } else if meta.path.is_ident("default") {
                    result.is_default = true;
                } else {
                    return Err(meta.error(format!(
                        "unknown variant attribute: {}",
                        meta.path
                            .get_ident()
                            .map(|i| i.to_string())
                            .unwrap_or_default()
                    )));
                }
                Ok(())
            })?;
        }

        Ok(result)
    }
}

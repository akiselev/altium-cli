//! `#[altium_enum]` attribute macro implementation.
//!
//! Generates `AltiumEnum` and `ParamCodec` trait implementations for enums
//! that map to integer values in Altium parameter files.
//!
//! Unlike the v1 `#[derive(AltiumEnum)]`, this attribute macro:
//! - Targets `crate::v2::traits::AltiumEnum` / `crate::v2::traits::ParamCodec`
//! - Does NOT generate a `Default` impl
//! - Directly generates the `ParamCodec` impl inline

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Fields, Ident, ItemEnum, LitStr, Token};

use crate::attrs::VariantAttrs;

// ---------------------------------------------------------------------------
// Macro-level attribute parsing
// ---------------------------------------------------------------------------

/// Parsed arguments from `#[altium_enum(...)]`.
struct AltiumEnumArgs {
    /// Name of the fallback variant for unknown values.
    /// If `None`, the first variant is used.
    fallback: Option<String>,
}

impl Parse for AltiumEnumArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut fallback = None;

        if !input.is_empty() {
            let ident: Ident = input.parse()?;
            if ident != "fallback" {
                return Err(syn::Error::new_spanned(ident, "expected `fallback`"));
            }
            input.parse::<Token![=]>()?;
            let lit: LitStr = input.parse()?;
            fallback = Some(lit.value());
        }

        Ok(AltiumEnumArgs { fallback })
    }
}

// ---------------------------------------------------------------------------
// Variant info
// ---------------------------------------------------------------------------

struct VariantInfo {
    ident: Ident,
    value: i64,
}

// ---------------------------------------------------------------------------
// Main expansion
// ---------------------------------------------------------------------------

pub fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args: AltiumEnumArgs = syn::parse2(attr)?;
    let input: ItemEnum = syn::parse2(item)?;

    let name = &input.ident;

    // Collect variant information
    let mut variants_info = Vec::new();
    let mut next_value: i64 = 0;
    let mut any_altium_value_attr = false;

    for variant in &input.variants {
        // Only unit variants are supported
        match &variant.fields {
            Fields::Unit => {}
            _ => {
                return Err(syn::Error::new_spanned(
                    &variant.ident,
                    "altium_enum only supports unit variants",
                ));
            }
        }

        let attrs = VariantAttrs::from_attrs(&variant.attrs)?;

        let (value, has_altium_value_attr) = if let Some(v) = attrs.value {
            next_value = v + 1;
            (v, true)
        } else if let Some((
            _,
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(int_lit),
                ..
            }),
        )) = &variant.discriminant
        {
            let v: i64 = int_lit.base10_parse()?;
            next_value = v + 1;
            (v, false)
        } else {
            let v = next_value;
            next_value += 1;
            (v, false)
        };

        if has_altium_value_attr {
            any_altium_value_attr = true;
        }

        variants_info.push(VariantInfo {
            ident: variant.ident.clone(),
            value,
        });
    }

    if variants_info.is_empty() {
        return Err(syn::Error::new_spanned(
            name,
            "altium_enum requires at least one variant",
        ));
    }

    // Determine the fallback variant
    let fallback_ident = if let Some(ref fb_name) = args.fallback {
        let found = variants_info
            .iter()
            .find(|v| v.ident == fb_name)
            .ok_or_else(|| {
                syn::Error::new_spanned(
                    name,
                    format!("fallback variant `{}` not found in enum", fb_name),
                )
            })?;
        found.ident.clone()
    } else {
        variants_info[0].ident.clone()
    };

    // Generate from_int match arms (cast i64 values to i32 for the match)
    let from_int_arms: Vec<_> = variants_info
        .iter()
        .map(|vi| {
            let ident = &vi.ident;
            let value = vi.value as i32;
            quote! { #value => Self::#ident, }
        })
        .collect();

    // Determine if we can use `*self as i32` for to_int.
    // We can only do this if no variant has an `#[altium(value = N)]` attribute
    // that overrides the discriminant. When all values come from discriminants
    // (or auto-increment matching discriminants), `*self as i32` is safe.
    let to_int_body = if any_altium_value_attr {
        // Use a match expression since altium(value) may differ from discriminant
        let to_int_arms: Vec<_> = variants_info
            .iter()
            .map(|vi| {
                let ident = &vi.ident;
                let value = vi.value as i32;
                quote! { Self::#ident => #value, }
            })
            .collect();
        quote! {
            match self {
                #(#to_int_arms)*
            }
        }
    } else {
        quote! { *self as i32 }
    };

    // Build the cleaned enum — strip `#[altium(...)]` attributes from variants
    let mut clean_enum = input.clone();
    for variant in &mut clean_enum.variants {
        variant.attrs.retain(|attr| !attr.path().is_ident("altium"));
    }

    Ok(quote! {
        #clean_enum

        impl crate::v2::traits::AltiumEnum for #name {
            fn from_int(value: i32) -> Self {
                match value {
                    #(#from_int_arms)*
                    _ => Self::#fallback_ident,
                }
            }

            fn to_int(&self) -> i32 {
                #to_int_body
            }
        }

        impl crate::v2::traits::ParamCodec for #name {
            fn read(
                params: &crate::v2::parameters::ParameterCollection,
                key: &str,
            ) -> Option<Self> {
                use crate::v2::traits::AltiumEnum;
                params.get(key).map(|v| Self::from_int(v.as_int_or(0)))
            }

            fn write(
                &self,
                params: &mut crate::v2::parameters::ParameterCollection,
                key: &str,
            ) {
                use crate::v2::traits::AltiumEnum;
                params.add_int(key, self.to_int());
            }
        }
    })
}

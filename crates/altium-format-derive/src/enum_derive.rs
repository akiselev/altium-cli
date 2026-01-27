//! AltiumEnum derive macro implementation.
//!
//! Generates integer-to-enum and enum-to-integer conversion methods.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Ident};

use crate::attrs::{ContainerAttrs, VariantAttrs};

pub fn derive_enum(input: DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let container_attrs = ContainerAttrs::from_attrs(&input.attrs)?;

    let variants = match &input.data {
        Data::Enum(data) => &data.variants,
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "AltiumEnum only supports enums",
            ))
        }
    };

    // Get the integer representation type
    let repr_ty = container_attrs
        .format
        .as_deref()
        .unwrap_or("i32");
    let repr_ident = format_ident!("{}", repr_ty);

    // Collect variant information
    let mut variant_info = Vec::new();
    let mut default_variant: Option<Ident> = None;
    let mut next_value: i64 = 0;

    for variant in variants {
        let attrs = VariantAttrs::from_attrs(&variant.attrs)?;

        // Get variant value
        let value = if let Some(v) = attrs.value {
            next_value = v + 1;
            v
        } else if let Some((_, syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(int_lit), .. }))) = &variant.discriminant {
            // Parse discriminant value
            let v: i64 = int_lit.base10_parse()?;
            next_value = v + 1;
            v
        } else if variant.discriminant.is_some() {
            // Non-integer literal discriminant - use auto-increment
            let v = next_value;
            next_value += 1;
            v
        } else {
            let v = next_value;
            next_value += 1;
            v
        };

        if attrs.is_default {
            default_variant = Some(variant.ident.clone());
        }

        // Check that variant has no fields
        match &variant.fields {
            Fields::Unit => {}
            _ => {
                return Err(syn::Error::new_spanned(
                    &variant.ident,
                    "AltiumEnum only supports unit variants",
                ))
            }
        }

        variant_info.push((variant.ident.clone(), value));
    }

    // Generate from_int match arms
    let from_int_arms: Vec<_> = variant_info
        .iter()
        .map(|(ident, value)| {
            quote! {
                #value => #name::#ident,
            }
        })
        .collect();

    // Generate to_int match arms
    let to_int_arms: Vec<_> = variant_info
        .iter()
        .map(|(ident, value)| {
            quote! {
                #name::#ident => #value,
            }
        })
        .collect();

    // Default handling for from_int
    let default_arm = if let Some(default_var) = &default_variant {
        quote! { _ => #name::#default_var }
    } else {
        let first_variant = &variant_info[0].0;
        quote! { _ => #name::#first_variant }
    };

    Ok(quote! {
        impl #name {
            /// Convert from integer value.
            pub fn from_int(value: #repr_ident) -> Self {
                match value as i64 {
                    #(#from_int_arms)*
                    #default_arm
                }
            }

            /// Convert to integer value.
            pub fn to_int(self) -> #repr_ident {
                (match self {
                    #(#to_int_arms)*
                }) as #repr_ident
            }
        }

        impl crate::traits::FromParamValue for #name {
            fn from_param_value(value: &crate::types::ParameterValue) -> crate::error::Result<Self> {
                Ok(Self::from_int(value.as_int_or(0) as #repr_ident))
            }
        }

        impl crate::traits::ToParamValue for #name {
            fn to_param_value(&self) -> String {
                self.to_int().to_string()
            }
        }

        impl Default for #name {
            fn default() -> Self {
                #default_arm
            }
        }
    })
}

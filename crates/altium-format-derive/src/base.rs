//! AltiumBase derive macro implementation.
//!
//! Generates `HasXxxBase` traits for composition-based inheritance.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields};

use crate::attrs::{ContainerAttrs, FieldAttrs};

pub fn derive_base(input: DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let container_attrs = ContainerAttrs::from_attrs(&input.attrs)?;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    "AltiumBase only supports structs with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "AltiumBase only supports structs",
            ))
        }
    };

    // Collect field information
    let mut field_info = Vec::new();
    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let attrs = FieldAttrs::from_attrs(&field.attrs)?;
        field_info.push((field_name.clone(), field_type.clone(), attrs));
    }

    // Generate trait name
    let base_name = container_attrs
        .base_name
        .as_ref()
        .cloned()
        .unwrap_or_else(|| name.to_string());
    let trait_name = format_ident!("Has{}", base_name);
    let base_getter = format_ident!("{}_base", to_snake_case(&base_name));
    let base_getter_mut = format_ident!("{}_base_mut", to_snake_case(&base_name));

    // Generate accessor methods
    let mut accessor_trait_methods = Vec::new();
    let mut accessor_self_impls = Vec::new();

    for (field_name, field_type, attrs) in &field_info {
        if attrs.skip || attrs.flatten || attrs.unknown {
            continue;
        }

        // Getter
        let getter_name = field_name.clone();
        accessor_trait_methods.push(quote! {
            fn #getter_name(&self) -> &#field_type {
                &self.#base_getter().#field_name
            }
        });

        // Setter
        let setter_name = format_ident!("set_{}", field_name);
        accessor_trait_methods.push(quote! {
            fn #setter_name(&mut self, value: #field_type) {
                self.#base_getter_mut().#field_name = value;
            }
        });

        // Self impls (for the base type itself)
        accessor_self_impls.push(quote! {
            fn #getter_name(&self) -> &#field_type {
                &self.#field_name
            }

            fn #setter_name(&mut self, value: #field_type) {
                self.#field_name = value;
            }
        });
    }

    // Generate FromParams/ToParams
    let mut field_reads = Vec::new();
    let mut field_writes = Vec::new();

    for (field_name, _field_type, attrs) in &field_info {
        if attrs.skip {
            field_reads.push(quote! {
                #field_name: Default::default(),
            });
            continue;
        }

        if attrs.flatten {
            field_reads.push(quote! {
                #field_name: crate::traits::FromParams::from_params(params)?,
            });
            field_writes.push(quote! {
                crate::traits::ToParams::append_to_params(&self.#field_name, params);
            });
            continue;
        }

        if let Some(param_key) = &attrs.param {
            let read_expr = if let Some(frac_key) = &attrs.frac {
                quote! {
                    {
                        let int_val = params.get(#param_key)
                            .map(|v| v.as_int_or(0))
                            .unwrap_or(0);
                        let frac_val = params.get(#frac_key)
                            .map(|v| v.as_int_or(0))
                            .unwrap_or(0);
                        crate::types::dxp_frac_to_coord(int_val, frac_val)
                    }
                }
            } else if attrs.optional {
                quote! {
                    params.get(#param_key)
                        .map(|v| crate::traits::FromParamValue::from_param_value(&v))
                        .transpose()?
                }
            } else {
                quote! {
                    params.get(#param_key)
                        .map(|v| crate::traits::FromParamValue::from_param_value(&v))
                        .transpose()?
                        .unwrap_or_default()
                }
            };

            field_reads.push(quote! {
                #field_name: #read_expr,
            });

            // Write expression
            let write_expr = if let Some(frac_key) = &attrs.frac {
                quote! {
                    {
                        let (int_val, frac_val) = crate::types::coord_to_dxp_frac(self.#field_name);
                        params.add_int(#param_key, int_val);
                        params.add_int(#frac_key, frac_val);
                    }
                }
            } else if attrs.optional {
                quote! {
                    if let Some(ref val) = self.#field_name {
                        params.add(#param_key, &crate::traits::ToParamValue::to_param_value(val));
                    }
                }
            } else {
                quote! {
                    params.add(#param_key, &crate::traits::ToParamValue::to_param_value(&self.#field_name));
                }
            };

            field_writes.push(write_expr);
        }
    }

    // Check for parent base (extends)
    let parent_trait_bound = if let Some(extends) = &container_attrs.extends {
        let parent_trait = format_ident!("Has{}", extends);
        quote! { + #parent_trait }
    } else {
        quote! {}
    };

    // Generate special methods for SchPrimitiveBase
    let special_methods = if base_name == "SchPrimitiveBase" {
        let has_owner_index = field_info
            .iter()
            .any(|(name, _, _)| *name == "owner_index");

        if has_owner_index {
            quote! {
                /// Get the owner index (parent reference).
                fn owner_index(&self) -> i32 {
                    self.#base_getter().owner_index
                }

                /// Set the owner index.
                fn set_owner_index(&mut self, index: i32) {
                    self.#base_getter_mut().owner_index = index;
                }
            }
        } else {
            quote! {}
        }
    } else {
        quote! {}
    };

    let special_self_impls = if base_name == "SchPrimitiveBase" {
        let has_owner_index = field_info
            .iter()
            .any(|(name, _, _)| *name == "owner_index");

        if has_owner_index {
            quote! {
                fn owner_index(&self) -> i32 {
                    self.owner_index
                }

                fn set_owner_index(&mut self, index: i32) {
                    self.owner_index = index;
                }
            }
        } else {
            quote! {}
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        /// Generated trait for accessing #name fields via composition.
        pub trait #trait_name #parent_trait_bound {
            /// Get a reference to the base structure.
            fn #base_getter(&self) -> &#name;
            /// Get a mutable reference to the base structure.
            fn #base_getter_mut(&mut self) -> &mut #name;

            #special_methods

            #(#accessor_trait_methods)*
        }

        /// Self-implementation for the base type itself.
        impl #trait_name for #name {
            fn #base_getter(&self) -> &#name {
                self
            }

            fn #base_getter_mut(&mut self) -> &mut #name {
                self
            }

            #special_self_impls

            #(#accessor_self_impls)*
        }

        impl crate::traits::FromParams for #name {
            fn from_params(params: &crate::types::ParameterCollection) -> crate::error::Result<Self> {
                Ok(Self {
                    #(#field_reads)*
                })
            }
        }

        impl crate::traits::ToParams for #name {
            fn to_params(&self) -> crate::types::ParameterCollection {
                let mut params = crate::types::ParameterCollection::new();
                self.append_to_params(&mut params);
                params
            }

            fn append_to_params(&self, params: &mut crate::types::ParameterCollection) {
                #(#field_writes)*
            }
        }
    })
}

/// Convert PascalCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

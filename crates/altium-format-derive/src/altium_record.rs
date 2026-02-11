//! Implementation of the `#[altium_record]` attribute macro.
//!
//! This attribute macro replaces the annotated struct with a thin wrapper
//! around `RecordOrigin` and generates getters, setters, update closures,
//! a builder, and a `RecordType` trait impl.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute, Error, Expr, ExprLit, Fields, Ident, ItemStruct, Lit, Meta, Result, Token, Type,
};

// ---------------------------------------------------------------------------
// Macro-level attribute parsing
// ---------------------------------------------------------------------------

/// Parsed representation of `#[altium_record(kind = "sch", record_id = 2, codec = "params")]`.
#[allow(dead_code)]
struct MacroAttrs {
    kind: RecordKind,
    record_id: Option<u8>,
    object_id: Option<Ident>,
    codec: Codec,
    parse_fn: Option<String>,
    serialize_fn: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum RecordKind {
    Sch,
    Pcb,
}

#[derive(Clone, Copy, PartialEq)]
enum Codec {
    Params,
    Binary,
}

impl Parse for MacroAttrs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut kind = None;
        let mut record_id = None;
        let mut object_id = None;
        let mut codec = None;
        let mut parse_fn = None;
        let mut serialize_fn = None;

        let pairs = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
        for meta in pairs {
            match &meta {
                Meta::NameValue(nv) => {
                    let key = nv
                        .path
                        .get_ident()
                        .ok_or_else(|| Error::new(nv.path.span(), "expected identifier"))?
                        .to_string();
                    match key.as_str() {
                        "kind" => {
                            let s = expr_to_string(&nv.value)?;
                            kind = Some(match s.as_str() {
                                "sch" => RecordKind::Sch,
                                "pcb" => RecordKind::Pcb,
                                other => {
                                    return Err(Error::new(
                                        nv.value.span(),
                                        format!("unknown kind: {other:?}, expected \"sch\" or \"pcb\""),
                                    ))
                                }
                            });
                        }
                        "record_id" => {
                            let n = expr_to_int(&nv.value)?;
                            record_id = Some(n as u8);
                        }
                        "object_id" => {
                            // object_id = Track  (an ident, not a string)
                            if let Expr::Path(ep) = &nv.value {
                                if let Some(ident) = ep.path.get_ident() {
                                    object_id = Some(ident.clone());
                                } else {
                                    return Err(Error::new(
                                        ep.span(),
                                        "expected a simple identifier for object_id",
                                    ));
                                }
                            } else {
                                return Err(Error::new(
                                    nv.value.span(),
                                    "expected an identifier for object_id",
                                ));
                            }
                        }
                        "codec" => {
                            let s = expr_to_string(&nv.value)?;
                            codec = Some(match s.as_str() {
                                "params" => Codec::Params,
                                "binary" => Codec::Binary,
                                other => {
                                    return Err(Error::new(
                                        nv.value.span(),
                                        format!(
                                            "unknown codec: {other:?}, expected \"params\" or \"binary\""
                                        ),
                                    ))
                                }
                            });
                        }
                        "parse_fn" => {
                            parse_fn = Some(expr_to_string(&nv.value)?);
                        }
                        "serialize_fn" => {
                            serialize_fn = Some(expr_to_string(&nv.value)?);
                        }
                        other => {
                            return Err(Error::new(
                                nv.path.span(),
                                format!("unknown attribute: {other}"),
                            ));
                        }
                    }
                }
                other => {
                    return Err(Error::new(other.span(), "expected name = value pair"));
                }
            }
        }

        let kind =
            kind.ok_or_else(|| Error::new(Span::call_site(), "missing required attribute `kind`"))?;
        let codec = codec
            .ok_or_else(|| Error::new(Span::call_site(), "missing required attribute `codec`"))?;

        // Validate combinations
        if kind == RecordKind::Sch && record_id.is_none() {
            return Err(Error::new(
                Span::call_site(),
                "sch records require `record_id`",
            ));
        }
        if kind == RecordKind::Pcb && object_id.is_none() {
            return Err(Error::new(
                Span::call_site(),
                "pcb records require `object_id`",
            ));
        }

        Ok(MacroAttrs {
            kind,
            record_id,
            object_id,
            codec,
            parse_fn,
            serialize_fn,
        })
    }
}

// ---------------------------------------------------------------------------
// Field-level attribute parsing
// ---------------------------------------------------------------------------

/// Parsed field-level `#[altium(...)]` attributes.
#[allow(dead_code)]
struct FieldAttrs {
    /// `#[altium(key = "DESIGNATOR")]`
    key: Option<String>,
    /// `#[altium(codec_fn = "name")]`
    codec_fn: Option<String>,
    /// `#[altium(header)]`
    header: bool,
    /// `#[altium(trailing)]`
    trailing: bool,
    /// `#[altium(skip)]`
    skip: bool,
}

impl FieldAttrs {
    fn parse_from(attrs: &[Attribute]) -> Result<Self> {
        let mut key = None;
        let mut codec_fn = None;
        let mut header = false;
        let mut trailing = false;
        let mut skip = false;

        for attr in attrs {
            if !attr.path().is_ident("altium") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("key") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(s) = lit {
                        key = Some(s.value());
                    } else {
                        return Err(Error::new(lit.span(), "expected string literal for `key`"));
                    }
                } else if meta.path.is_ident("codec_fn") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(s) = lit {
                        codec_fn = Some(s.value());
                    } else {
                        return Err(Error::new(
                            lit.span(),
                            "expected string literal for `codec_fn`",
                        ));
                    }
                } else if meta.path.is_ident("header") {
                    header = true;
                } else if meta.path.is_ident("trailing") {
                    trailing = true;
                } else if meta.path.is_ident("skip") {
                    skip = true;
                } else {
                    return Err(Error::new(
                        meta.path.span(),
                        format!(
                            "unknown field attribute: {}",
                            meta.path
                                .get_ident()
                                .map(|i| i.to_string())
                                .unwrap_or_else(|| "?".into())
                        ),
                    ));
                }
                Ok(())
            })?;
        }

        Ok(FieldAttrs {
            key,
            codec_fn,
            header,
            trailing,
            skip,
        })
    }
}

/// Processed field info used during code generation.
struct FieldInfo {
    /// The original field name.
    name: Ident,
    /// The original field type.
    ty: Type,
    /// Parsed altium attributes.
    attrs: FieldAttrs,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn expr_to_string(expr: &Expr) -> Result<String> {
    if let Expr::Lit(ExprLit {
        lit: Lit::Str(s), ..
    }) = expr
    {
        Ok(s.value())
    } else {
        Err(Error::new(expr.span(), "expected string literal"))
    }
}

fn expr_to_int(expr: &Expr) -> Result<i64> {
    if let Expr::Lit(ExprLit {
        lit: Lit::Int(n), ..
    }) = expr
    {
        n.base10_parse::<i64>()
    } else {
        Err(Error::new(expr.span(), "expected integer literal"))
    }
}

/// Returns the known byte size for a type used in binary sequential layout,
/// or `None` for unknown types.
fn binary_type_size(ty: &Type) -> Option<usize> {
    let path = match ty {
        Type::Path(tp) => &tp.path,
        _ => return None,
    };
    let ident = path.segments.last()?.ident.to_string();
    match ident.as_str() {
        "u8" | "i8" | "bool" => Some(1),
        "u16" | "i16" => Some(2),
        "u32" | "i32" | "PcbCoord" => Some(4),
        "f64" => Some(8),
        "PcbCommonHeader" => Some(13),
        _ => None,
    }
}

/// Returns the binary read expression for a given type at a given offset.
fn binary_read_expr(ty: &Type, offset: usize) -> Result<TokenStream> {
    let path = match ty {
        Type::Path(tp) => &tp.path,
        _ => {
            return Err(Error::new(
                ty.span(),
                "unsupported type for binary sequential layout",
            ))
        }
    };
    let ident_str = path
        .segments
        .last()
        .ok_or_else(|| Error::new(ty.span(), "empty type path"))?
        .ident
        .to_string();
    let offset_lit = proc_macro2::Literal::usize_unsuffixed(offset);
    let tokens = match ident_str.as_str() {
        "u8" => quote! { crate::v2::binary_helpers::read_u8(&self.origin.binary().raw_block, #offset_lit) },
        "i8" => quote! { crate::v2::binary_helpers::read_i8(&self.origin.binary().raw_block, #offset_lit) },
        "bool" => quote! { crate::v2::binary_helpers::read_bool(&self.origin.binary().raw_block, #offset_lit) },
        "u16" => quote! { crate::v2::binary_helpers::read_u16_le(&self.origin.binary().raw_block, #offset_lit) },
        "i16" => quote! { crate::v2::binary_helpers::read_i16_le(&self.origin.binary().raw_block, #offset_lit) },
        "u32" => quote! { crate::v2::binary_helpers::read_u32_le(&self.origin.binary().raw_block, #offset_lit) },
        "i32" => quote! { crate::v2::binary_helpers::read_i32_le(&self.origin.binary().raw_block, #offset_lit) },
        "PcbCoord" => quote! { crate::v2::binary_helpers::read_pcb_coord(&self.origin.binary().raw_block, #offset_lit) },
        "f64" => quote! { crate::v2::binary_helpers::read_f64_le(&self.origin.binary().raw_block, #offset_lit) },
        "PcbCommonHeader" => quote! { crate::v2::binary_helpers::PcbCommonHeader::read(&self.origin.binary().raw_block, #offset_lit) },
        other => {
            return Err(Error::new(
                ty.span(),
                format!("unsupported binary type: {other}"),
            ))
        }
    };
    Ok(tokens)
}

/// Returns the binary write expression for a given type at a given offset.
fn binary_write_expr(ty: &Type, offset: usize) -> Result<TokenStream> {
    let path = match ty {
        Type::Path(tp) => &tp.path,
        _ => {
            return Err(Error::new(
                ty.span(),
                "unsupported type for binary sequential layout",
            ))
        }
    };
    let ident_str = path
        .segments
        .last()
        .ok_or_else(|| Error::new(ty.span(), "empty type path"))?
        .ident
        .to_string();
    let offset_lit = proc_macro2::Literal::usize_unsuffixed(offset);
    let tokens = match ident_str.as_str() {
        "u8" => quote! { crate::v2::binary_helpers::write_u8(&mut self.origin.binary_mut().raw_block, #offset_lit, value) },
        "i8" => quote! { crate::v2::binary_helpers::write_i8(&mut self.origin.binary_mut().raw_block, #offset_lit, value) },
        "bool" => quote! { crate::v2::binary_helpers::write_bool(&mut self.origin.binary_mut().raw_block, #offset_lit, value) },
        "u16" => quote! { crate::v2::binary_helpers::write_u16_le(&mut self.origin.binary_mut().raw_block, #offset_lit, value) },
        "i16" => quote! { crate::v2::binary_helpers::write_i16_le(&mut self.origin.binary_mut().raw_block, #offset_lit, value) },
        "u32" => quote! { crate::v2::binary_helpers::write_u32_le(&mut self.origin.binary_mut().raw_block, #offset_lit, value) },
        "i32" => quote! { crate::v2::binary_helpers::write_i32_le(&mut self.origin.binary_mut().raw_block, #offset_lit, value) },
        "PcbCoord" => quote! { crate::v2::binary_helpers::write_pcb_coord(&mut self.origin.binary_mut().raw_block, #offset_lit, value) },
        "f64" => quote! { crate::v2::binary_helpers::write_f64_le(&mut self.origin.binary_mut().raw_block, #offset_lit, value) },
        "PcbCommonHeader" => quote! { value.write(&mut self.origin.binary_mut().raw_block, #offset_lit) },
        other => {
            return Err(Error::new(
                ty.span(),
                format!("unsupported binary type for write: {other}"),
            ))
        }
    };
    Ok(tokens)
}

/// Determines if a type is a "string newtype" that should use `impl Into<T>` in setters.
/// We check for String and known string newtypes from the newtypes module.
fn is_string_newtype(ty: &Type) -> bool {
    let path = match ty {
        Type::Path(tp) => &tp.path,
        _ => return false,
    };
    let ident_str = match path.segments.last() {
        Some(seg) => seg.ident.to_string(),
        None => return false,
    };
    matches!(
        ident_str.as_str(),
        "String" | "Designator" | "LibReference" | "NetName" | "UniqueId" | "Description" | "PinName"
    )
}

// ---------------------------------------------------------------------------
// Top-level expand function
// ---------------------------------------------------------------------------

pub fn expand(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let macro_attrs: MacroAttrs = syn::parse2(attr)?;
    let input: ItemStruct = syn::parse2(item)?;

    let struct_name = &input.ident;
    let vis = &input.vis;

    // Parse fields from the original struct
    let fields = match &input.fields {
        Fields::Named(named) => &named.named,
        _ => {
            return Err(Error::new(
                input.span(),
                "altium_record requires a struct with named fields",
            ))
        }
    };

    let field_infos: Vec<FieldInfo> = fields
        .iter()
        .map(|f| {
            let attrs = FieldAttrs::parse_from(&f.attrs)?;
            Ok(FieldInfo {
                name: f.ident.clone().unwrap(),
                ty: f.ty.clone(),
                attrs,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // Generate the replaced struct
    let struct_def = quote! {
        #vis struct #struct_name {
            origin: crate::v2::backing_store::RecordOrigin,
        }
    };

    // Generate constructors
    let constructors = quote! {
        impl #struct_name {
            pub fn from_origin(origin: crate::v2::backing_store::RecordOrigin) -> Self {
                Self { origin }
            }

            pub fn new(origin: crate::v2::backing_store::RecordOrigin) -> Self {
                Self { origin }
            }
        }
    };

    // Generate accessor methods based on codec
    let accessors = match macro_attrs.codec {
        Codec::Params => gen_param_accessors(struct_name, &field_infos)?,
        Codec::Binary => {
            if macro_attrs.parse_fn.is_some() {
                gen_binary_custom_accessors(struct_name, &field_infos)?
            } else {
                gen_binary_sequential_accessors(struct_name, &field_infos)?
            }
        }
    };

    // Generate RecordType trait impl
    let record_type_impl = gen_record_type_impl(struct_name, &macro_attrs)?;

    // Generate builder (only for param and binary-sequential records)
    let builder = gen_builder(struct_name, &macro_attrs, &field_infos)?;

    Ok(quote! {
        #struct_def
        #constructors
        #accessors
        #record_type_impl
        #builder
    })
}

// ---------------------------------------------------------------------------
// Param-based accessor generation
// ---------------------------------------------------------------------------

fn gen_param_accessors(struct_name: &Ident, fields: &[FieldInfo]) -> Result<TokenStream> {
    let mut methods = Vec::new();

    for field in fields {
        if field.attrs.skip {
            continue;
        }

        let field_name = &field.name;
        let field_ty = &field.ty;
        let getter_name = format_ident!("{}", field_name);
        let try_getter_name = format_ident!("try_{}", field_name);
        let setter_name = format_ident!("set_{}", field_name);
        let updater_name = format_ident!("update_{}", field_name);

        if let Some(ref codec_fn_name) = field.attrs.codec_fn {
            // codec_fn escape hatch
            let codec_fn_path: syn::Path = syn::parse_str(codec_fn_name)?;
            methods.push(quote! {
                pub fn #getter_name(&self) -> #field_ty {
                    #codec_fn_path::read(&self.origin.param().params)
                }

                pub fn #setter_name(&mut self, value: #field_ty) {
                    #codec_fn_path::write(&value, &mut self.origin.param_mut().params);
                }

                pub fn #updater_name<R>(&mut self, f: impl FnOnce(&mut #field_ty) -> R) -> R {
                    let mut value = self.#getter_name();
                    let result = f(&mut value);
                    self.#setter_name(value);
                    result
                }
            });
        } else if let Some(ref key) = field.attrs.key {
            // Standard param key-based accessor
            let use_into = is_string_newtype(field_ty);

            let setter_body = if use_into {
                quote! {
                    pub fn #setter_name(&mut self, value: impl Into<#field_ty>) {
                        <#field_ty as crate::v2::traits::ParamCodec>::write(
                            &value.into(),
                            &mut self.origin.param_mut().params,
                            #key,
                        );
                    }
                }
            } else {
                quote! {
                    pub fn #setter_name(&mut self, value: #field_ty) {
                        <#field_ty as crate::v2::traits::ParamCodec>::write(
                            &value,
                            &mut self.origin.param_mut().params,
                            #key,
                        );
                    }
                }
            };

            methods.push(quote! {
                pub fn #getter_name(&self) -> #field_ty {
                    <#field_ty as crate::v2::traits::ParamCodec>::read(
                        &self.origin.param().params,
                        #key,
                    )
                    .unwrap_or_default()
                }

                pub fn #try_getter_name(&self) -> Option<#field_ty> {
                    <#field_ty as crate::v2::traits::ParamCodec>::read(
                        &self.origin.param().params,
                        #key,
                    )
                }

                #setter_body

                pub fn #updater_name<R>(&mut self, f: impl FnOnce(&mut #field_ty) -> R) -> R {
                    let mut value = self.#getter_name();
                    let result = f(&mut value);
                    self.#setter_name(value);
                    result
                }
            });
        }
        // If neither key nor codec_fn, the field has no param accessor (e.g. header, trailing)
    }

    Ok(quote! {
        impl #struct_name {
            #(#methods)*
        }
    })
}

// ---------------------------------------------------------------------------
// Binary sequential accessor generation
// ---------------------------------------------------------------------------

fn gen_binary_sequential_accessors(
    struct_name: &Ident,
    fields: &[FieldInfo],
) -> Result<TokenStream> {
    let mut methods = Vec::new();
    let mut current_offset: usize = 0;

    for field in fields {
        if field.attrs.skip {
            continue;
        }

        let field_name = &field.name;
        let field_ty = &field.ty;
        let getter_name = format_ident!("{}", field_name);
        let setter_name = format_ident!("set_{}", field_name);

        let type_size = binary_type_size(field_ty).ok_or_else(|| {
            Error::new(
                field_ty.span(),
                format!(
                    "unknown binary size for type `{}` in sequential layout",
                    quote!(#field_ty)
                ),
            )
        })?;

        let offset = current_offset;
        let read_expr = binary_read_expr(field_ty, offset)?;
        let write_expr = binary_write_expr(field_ty, offset)?;

        methods.push(quote! {
            pub fn #getter_name(&self) -> #field_ty {
                #read_expr
            }

            pub fn #setter_name(&mut self, value: #field_ty) {
                #write_expr
            }
        });

        current_offset += type_size;
    }

    Ok(quote! {
        impl #struct_name {
            #(#methods)*
        }
    })
}

// ---------------------------------------------------------------------------
// Binary custom parser accessor generation (with field_spans)
// ---------------------------------------------------------------------------

fn gen_binary_custom_accessors(
    struct_name: &Ident,
    fields: &[FieldInfo],
) -> Result<TokenStream> {
    let mut constants = Vec::new();
    let mut methods = Vec::new();
    let mut field_index: usize = 0;

    for field in fields {
        if field.attrs.skip {
            continue;
        }

        let field_name = &field.name;
        let field_ty = &field.ty;
        let const_name = format_ident!(
            "FIELD_{}",
            field_name.to_string().to_uppercase()
        );
        let getter_name = format_ident!("{}", field_name);
        let setter_name = format_ident!("set_{}", field_name);

        let idx = field_index;
        let idx_lit = proc_macro2::Literal::usize_unsuffixed(idx);

        constants.push(quote! {
            pub const #const_name: usize = #idx_lit;
        });

        // For custom parser fields, we use field_spans to get the offset
        let read_expr = binary_custom_read_expr(field_ty, idx)?;
        let write_expr = binary_custom_write_expr(field_ty, idx)?;

        methods.push(quote! {
            pub fn #getter_name(&self) -> #field_ty {
                #read_expr
            }

            pub fn #setter_name(&mut self, value: #field_ty) {
                #write_expr
            }
        });

        field_index += 1;
    }

    Ok(quote! {
        impl #struct_name {
            #(#constants)*
            #(#methods)*
        }
    })
}

/// Returns the binary read expression for custom-parser fields using field_spans.
fn binary_custom_read_expr(ty: &Type, field_index: usize) -> Result<TokenStream> {
    let path = match ty {
        Type::Path(tp) => &tp.path,
        _ => {
            return Err(Error::new(
                ty.span(),
                "unsupported type for binary custom layout",
            ))
        }
    };
    let ident_str = path
        .segments
        .last()
        .ok_or_else(|| Error::new(ty.span(), "empty type path"))?
        .ident
        .to_string();
    let idx_lit = proc_macro2::Literal::usize_unsuffixed(field_index);

    let tokens = match ident_str.as_str() {
        "u8" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                crate::v2::binary_helpers::read_u8(&self.origin.binary().raw_block, span.offset)
            }
        },
        "i8" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                crate::v2::binary_helpers::read_i8(&self.origin.binary().raw_block, span.offset)
            }
        },
        "bool" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                crate::v2::binary_helpers::read_bool(&self.origin.binary().raw_block, span.offset)
            }
        },
        "u16" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                crate::v2::binary_helpers::read_u16_le(&self.origin.binary().raw_block, span.offset)
            }
        },
        "i16" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                crate::v2::binary_helpers::read_i16_le(&self.origin.binary().raw_block, span.offset)
            }
        },
        "u32" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                crate::v2::binary_helpers::read_u32_le(&self.origin.binary().raw_block, span.offset)
            }
        },
        "i32" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                crate::v2::binary_helpers::read_i32_le(&self.origin.binary().raw_block, span.offset)
            }
        },
        "PcbCoord" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                crate::v2::binary_helpers::read_pcb_coord(&self.origin.binary().raw_block, span.offset)
            }
        },
        "f64" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                crate::v2::binary_helpers::read_f64_le(&self.origin.binary().raw_block, span.offset)
            }
        },
        "PcbCommonHeader" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                crate::v2::binary_helpers::PcbCommonHeader::read(&self.origin.binary().raw_block, span.offset)
            }
        },
        other => {
            return Err(Error::new(
                ty.span(),
                format!("unsupported binary type for custom read: {other}"),
            ))
        }
    };
    Ok(tokens)
}

/// Returns the binary write expression for custom-parser fields using field_spans.
fn binary_custom_write_expr(ty: &Type, field_index: usize) -> Result<TokenStream> {
    let path = match ty {
        Type::Path(tp) => &tp.path,
        _ => {
            return Err(Error::new(
                ty.span(),
                "unsupported type for binary custom layout",
            ))
        }
    };
    let ident_str = path
        .segments
        .last()
        .ok_or_else(|| Error::new(ty.span(), "empty type path"))?
        .ident
        .to_string();
    let idx_lit = proc_macro2::Literal::usize_unsuffixed(field_index);

    let tokens = match ident_str.as_str() {
        "u8" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                let offset = span.offset;
                crate::v2::binary_helpers::write_u8(&mut self.origin.binary_mut().raw_block, offset, value);
            }
        },
        "i8" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                let offset = span.offset;
                crate::v2::binary_helpers::write_i8(&mut self.origin.binary_mut().raw_block, offset, value);
            }
        },
        "bool" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                let offset = span.offset;
                crate::v2::binary_helpers::write_bool(&mut self.origin.binary_mut().raw_block, offset, value);
            }
        },
        "u16" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                let offset = span.offset;
                crate::v2::binary_helpers::write_u16_le(&mut self.origin.binary_mut().raw_block, offset, value);
            }
        },
        "i16" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                let offset = span.offset;
                crate::v2::binary_helpers::write_i16_le(&mut self.origin.binary_mut().raw_block, offset, value);
            }
        },
        "u32" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                let offset = span.offset;
                crate::v2::binary_helpers::write_u32_le(&mut self.origin.binary_mut().raw_block, offset, value);
            }
        },
        "i32" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                let offset = span.offset;
                crate::v2::binary_helpers::write_i32_le(&mut self.origin.binary_mut().raw_block, offset, value);
            }
        },
        "PcbCoord" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                let offset = span.offset;
                crate::v2::binary_helpers::write_pcb_coord(&mut self.origin.binary_mut().raw_block, offset, value);
            }
        },
        "f64" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                let offset = span.offset;
                crate::v2::binary_helpers::write_f64_le(&mut self.origin.binary_mut().raw_block, offset, value);
            }
        },
        "PcbCommonHeader" => quote! {
            {
                let span = &self.origin.binary().field_spans[#idx_lit];
                let offset = span.offset;
                value.write(&mut self.origin.binary_mut().raw_block, offset);
            }
        },
        other => {
            return Err(Error::new(
                ty.span(),
                format!("unsupported binary type for custom write: {other}"),
            ))
        }
    };
    Ok(tokens)
}

// ---------------------------------------------------------------------------
// RecordType trait impl generation
// ---------------------------------------------------------------------------

fn gen_record_type_impl(struct_name: &Ident, attrs: &MacroAttrs) -> Result<TokenStream> {
    let record_id_expr = match attrs.kind {
        RecordKind::Sch => {
            let id = attrs.record_id.unwrap();
            let id_lit = proc_macro2::Literal::u8_unsuffixed(id);
            quote! { #id_lit }
        }
        RecordKind::Pcb => {
            let obj_id = attrs.object_id.as_ref().unwrap();
            // For PCB, use the object_id variant's discriminant.
            // We generate: crate::v2::pcb::enums::PcbObjectId::#obj_id as u8
            quote! { crate::v2::pcb::enums::PcbObjectId::#obj_id as u8 }
        }
    };

    Ok(quote! {
        impl crate::v2::traits::RecordType for #struct_name {
            const RECORD_ID: u8 = #record_id_expr;

            fn origin(&self) -> &crate::v2::backing_store::RecordOrigin {
                &self.origin
            }

            fn origin_mut(&mut self) -> &mut crate::v2::backing_store::RecordOrigin {
                &mut self.origin
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Builder generation
// ---------------------------------------------------------------------------

fn gen_builder(
    struct_name: &Ident,
    macro_attrs: &MacroAttrs,
    fields: &[FieldInfo],
) -> Result<TokenStream> {
    let builder_name = format_ident!("{}Builder", struct_name);

    let mut builder_methods = Vec::new();

    for field in fields {
        if field.attrs.skip {
            continue;
        }

        let field_name = &field.name;
        let field_ty = &field.ty;
        let setter_name = format_ident!("set_{}", field_name);
        let builder_method_name = format_ident!("{}", field_name);

        // Only generate builder methods for fields that have setters
        let has_setter = field.attrs.key.is_some()
            || field.attrs.codec_fn.is_some()
            || (macro_attrs.codec == Codec::Binary);

        if !has_setter {
            continue;
        }

        let use_into = macro_attrs.codec == Codec::Params && is_string_newtype(field_ty);

        let method = if use_into {
            quote! {
                pub fn #builder_method_name(mut self, value: impl Into<#field_ty>) -> Self {
                    self.record.#setter_name(value);
                    self
                }
            }
        } else {
            quote! {
                pub fn #builder_method_name(mut self, value: #field_ty) -> Self {
                    self.record.#setter_name(value);
                    self
                }
            }
        };

        builder_methods.push(method);
    }

    Ok(quote! {
        pub struct #builder_name {
            record: #struct_name,
        }

        impl #builder_name {
            pub fn new(template: fn() -> crate::v2::backing_store::RecordOrigin) -> Self {
                Self {
                    record: #struct_name::new(template()),
                }
            }

            #(#builder_methods)*

            pub fn build(self) -> #struct_name {
                self.record
            }
        }

        impl #struct_name {
            pub fn builder(template: fn() -> crate::v2::backing_store::RecordOrigin) -> #builder_name {
                #builder_name::new(template)
            }
        }
    })
}

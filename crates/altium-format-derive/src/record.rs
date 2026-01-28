//! AltiumRecord derive macro implementation.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, Type};

use crate::attrs::{ContainerAttrs, FieldAttrs};

pub fn derive_record(input: DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let container_attrs = ContainerAttrs::from_attrs(&input.attrs)?;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    "AltiumRecord only supports structs with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "AltiumRecord only supports structs",
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

    // Determine format from attributes or infer from fields
    let format = container_attrs.format.as_deref().unwrap_or_else(|| {
        if container_attrs.record_id.is_some() {
            "params"
        } else if container_attrs.object_id.is_some() {
            "binary"
        } else {
            "params" // Default
        }
    });

    let mut impls = TokenStream::new();

    // Generate AltiumRecord trait impl
    let record_type_name = name.to_string();
    impls.extend(quote! {
        impl crate::traits::AltiumRecord for #name {
            fn record_type_name() -> &'static str {
                #record_type_name
            }
        }
    });

    // Generate format-specific impls
    match format {
        "params" | "both" => {
            impls.extend(generate_from_params(name, &field_info, &container_attrs)?);
            impls.extend(generate_to_params(name, &field_info, &container_attrs)?);
        }
        _ => {}
    }

    match format {
        "binary" | "both" => {
            impls.extend(generate_from_binary(name, &field_info, &container_attrs)?);
            impls.extend(generate_to_binary(name, &field_info, &container_attrs)?);
        }
        _ => {}
    }

    // Note: We don't generate SchPrimitive or PcbPrimitive impls here.
    // These traits have specific requirements (calculate_bounds, etc.) that
    // need manual implementation per record type. The derive macro only handles
    // FromParams/ToParams and FromBinary/ToBinary.

    Ok(impls)
}

fn generate_from_params(
    name: &Ident,
    fields: &[(Ident, Type, FieldAttrs)],
    _container_attrs: &ContainerAttrs,
) -> syn::Result<TokenStream> {
    let mut known_keys = Vec::new();
    let mut indexed_prefixes = Vec::new();
    let mut field_reads = Vec::new();

    for (field_name, _field_type, attrs) in fields {
        if attrs.skip {
            field_reads.push(quote! {
                #field_name: Default::default(),
            });
            continue;
        }

        if attrs.flatten {
            // Flatten base type
            field_reads.push(quote! {
                #field_name: crate::traits::FromParams::from_params(params)?,
            });
            continue;
        }

        if attrs.unknown {
            // This will be handled at the end
            continue;
        }

        // Handle indexed coordinates (e.g., X1, Y1, X2, Y2, ...)
        if attrs.indexed_coords {
            let prefix_x = attrs.prefix_x.as_deref().unwrap_or("X");
            let prefix_y = attrs.prefix_y.as_deref().unwrap_or("Y");
            let count_param = attrs.count_param.as_deref().unwrap_or("LOCATIONCOUNT");

            // Add count param to known keys
            known_keys.push(count_param.to_uppercase());
            // Add prefixes for indexed parameter exclusion
            indexed_prefixes.push(prefix_x.to_uppercase());
            indexed_prefixes.push(prefix_y.to_uppercase());

            field_reads.push(quote! {
                #field_name: {
                    let count = params.get(#count_param)
                        .map(|v| v.as_int_or(0))
                        .unwrap_or(0);
                    let mut vertices = Vec::with_capacity(count as usize);
                    for i in 1..=count {
                        let x_key = format!("{}{}", #prefix_x, i);
                        let x_frac_key = format!("{}{}_FRAC", #prefix_x, i);
                        let y_key = format!("{}{}", #prefix_y, i);
                        let y_frac_key = format!("{}{}_FRAC", #prefix_y, i);

                        let x = params.get(&x_key)
                            .map(|v| v.as_int_or(0))
                            .unwrap_or(0);
                        let x_frac = params.get(&x_frac_key)
                            .map(|v| v.as_int_or(0))
                            .unwrap_or(0);
                        let y = params.get(&y_key)
                            .map(|v| v.as_int_or(0))
                            .unwrap_or(0);
                        let y_frac = params.get(&y_frac_key)
                            .map(|v| v.as_int_or(0))
                            .unwrap_or(0);

                        vertices.push((
                            crate::types::dxp_frac_to_coord(x, x_frac),
                            crate::types::dxp_frac_to_coord(y, y_frac),
                        ));
                    }
                    vertices
                },
            });
            continue;
        }

        if let Some(param_key) = &attrs.param {
            known_keys.push(param_key.to_uppercase());

            let read_expr = if let Some(frac_key) = &attrs.frac {
                // Integer with fractional part
                known_keys.push(frac_key.to_uppercase());
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
            } else if attrs.has_default {
                if let Some(default_val) = &attrs.default_value {
                    quote! {
                        params.get(#param_key)
                            .map(|v| crate::traits::FromParamValue::from_param_value(&v))
                            .transpose()?
                            .unwrap_or(#default_val)
                    }
                } else {
                    quote! {
                        params.get(#param_key)
                            .map(|v| crate::traits::FromParamValue::from_param_value(&v))
                            .transpose()?
                            .unwrap_or_default()
                    }
                }
            } else if attrs.color {
                quote! {
                    params.get(#param_key)
                        .map(|v| crate::types::Color::from_win32(v.as_int_or(0)))
                        .unwrap_or_default()
                }
            } else if attrs.list {
                quote! {
                    params.get(#param_key)
                        .map(|v| crate::traits::FromParamList::from_param_list(&v))
                        .transpose()?
                        .unwrap_or_default()
                }
            } else {
                // Required field
                quote! {
                    params.get(#param_key)
                        .ok_or_else(|| crate::error::AltiumError::MissingParameter(#param_key.to_string()))?
                        .pipe(|v| crate::traits::FromParamValue::from_param_value(&v))?
                }
            };

            field_reads.push(quote! {
                #field_name: #read_expr,
            });
        }
    }

    // Handle unknown fields
    let unknown_field = fields.iter().find(|(_, _, attrs)| attrs.unknown);
    let unknown_handling = if let Some((field_name, _, _)) = unknown_field {
        let known_keys_array = &known_keys;
        let indexed_prefixes_array = &indexed_prefixes;
        if indexed_prefixes.is_empty() {
            quote! {
                #field_name: crate::types::UnknownFields::from_remaining_params(
                    params,
                    &[#(#known_keys_array),*],
                ),
            }
        } else {
            quote! {
                #field_name: crate::types::UnknownFields::from_remaining_params_with_prefixes(
                    params,
                    &[#(#known_keys_array),*],
                    &[#(#indexed_prefixes_array),*],
                ),
            }
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        impl crate::traits::FromParams for #name {
            fn from_params(params: &crate::types::ParameterCollection) -> crate::error::Result<Self> {
                Ok(Self {
                    #(#field_reads)*
                    #unknown_handling
                })
            }
        }
    })
}

fn generate_to_params(
    name: &Ident,
    fields: &[(Ident, Type, FieldAttrs)],
    container_attrs: &ContainerAttrs,
) -> syn::Result<TokenStream> {
    let mut field_writes = Vec::new();

    for (field_name, _field_type, attrs) in fields {
        if attrs.skip || attrs.unknown {
            continue;
        }

        if attrs.flatten {
            field_writes.push(quote! {
                crate::traits::ToParams::append_to_params(&self.#field_name, params);
            });
            continue;
        }

        // Handle indexed coordinates export
        if attrs.indexed_coords {
            let prefix_x = attrs.prefix_x.as_deref().unwrap_or("X");
            let prefix_y = attrs.prefix_y.as_deref().unwrap_or("Y");
            let count_param = attrs.count_param.as_deref().unwrap_or("LOCATIONCOUNT");

            field_writes.push(quote! {
                params.add_int(#count_param, self.#field_name.len() as i32);
                for (i, (x, y)) in self.#field_name.iter().enumerate() {
                    let idx = i + 1;
                    let (x_int, x_frac) = crate::types::coord_to_dxp_frac(*x);
                    let (y_int, y_frac) = crate::types::coord_to_dxp_frac(*y);
                    params.add_int(&format!("{}{}", #prefix_x, idx), x_int);
                    params.add_int(&format!("{}{}_FRAC", #prefix_x, idx), x_frac);
                    params.add_int(&format!("{}{}", #prefix_y, idx), y_int);
                    params.add_int(&format!("{}{}_FRAC", #prefix_y, idx), y_frac);
                }
            });
            continue;
        }

        if let Some(param_key) = &attrs.param {
            let write_expr = if let Some(frac_key) = &attrs.frac {
                // Integer with fractional part
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
            } else if attrs.color {
                quote! {
                    params.add_int(#param_key, self.#field_name.to_win32());
                }
            } else if attrs.list {
                quote! {
                    if !self.#field_name.is_empty() {
                        params.add(#param_key, &crate::traits::ToParamList::to_param_list(&self.#field_name));
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

    // Handle unknown fields
    let unknown_field = fields.iter().find(|(_, _, attrs)| attrs.unknown);
    let unknown_handling = if let Some((field_name, _, _)) = unknown_field {
        quote! {
            self.#field_name.merge_into_params(params);
        }
    } else {
        quote! {}
    };

    // Write RECORD AFTER all fields (including flattened children) so the
    // parent's record_id always wins over any flattened child's record_id.
    let record_write = if let Some(record_id) = container_attrs.record_id {
        quote! {
            params.add_int("RECORD", #record_id);
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        impl crate::traits::ToParams for #name {
            fn to_params(&self) -> crate::types::ParameterCollection {
                let mut params = crate::types::ParameterCollection::new();
                self.append_to_params(&mut params);
                params
            }

            fn append_to_params(&self, params: &mut crate::types::ParameterCollection) {
                #(#field_writes)*
                #unknown_handling
                #record_write
            }
        }
    })
}

fn generate_from_binary(
    name: &Ident,
    fields: &[(Ident, Type, FieldAttrs)],
    _container_attrs: &ContainerAttrs,
) -> syn::Result<TokenStream> {
    let mut field_reads = Vec::new();

    for (field_name, _field_type, attrs) in fields {
        if attrs.skip {
            field_reads.push(quote! {
                #field_name: Default::default(),
            });
            continue;
        }

        if attrs.flatten {
            field_reads.push(quote! {
                #field_name: crate::traits::FromBinary::read_from(reader)?,
            });
            continue;
        }

        if attrs.unknown_binary {
            // Read remaining bytes
            field_reads.push(quote! {
                #field_name: {
                    let mut remaining = Vec::new();
                    reader.read_to_end(&mut remaining)?;
                    remaining
                },
            });
            continue;
        }

        if let Some(skip_bytes) = attrs.skip_bytes {
            field_reads.push(quote! {
                #field_name: {
                    let mut skip_buf = [0u8; #skip_bytes];
                    reader.read_exact(&mut skip_buf)?;
                    Default::default()
                },
            });
            continue;
        }

        if attrs.coord_point {
            field_reads.push(quote! {
                #field_name: crate::io::reader::read_coord_point(reader)?,
            });
        } else if attrs.coord {
            field_reads.push(quote! {
                #field_name: crate::types::Coord::from_raw(reader.read_i32::<byteorder::LittleEndian>()?),
            });
        } else if attrs.string_block {
            field_reads.push(quote! {
                #field_name: crate::io::reader::read_string_block(reader)?,
            });
        } else if attrs.pascal_string {
            field_reads.push(quote! {
                #field_name: crate::io::reader::read_pascal_short_string(reader)?,
            });
        } else if let Some(array_size) = attrs.array_size {
            field_reads.push(quote! {
                #field_name: {
                    let mut arr = [Default::default(); #array_size];
                    for item in arr.iter_mut() {
                        *item = crate::traits::FromBinary::read_from(reader)?;
                    }
                    arr
                },
            });
        } else if let Some(binary_ty) = &attrs.binary_ty {
            let read_expr = match binary_ty.as_str() {
                "i8" => quote! { reader.read_i8()? },
                "u8" => quote! { reader.read_u8()? },
                "i16le" => quote! { reader.read_i16::<byteorder::LittleEndian>()? },
                "u16le" => quote! { reader.read_u16::<byteorder::LittleEndian>()? },
                "i32le" => quote! { reader.read_i32::<byteorder::LittleEndian>()? },
                "u32le" => quote! { reader.read_u32::<byteorder::LittleEndian>()? },
                "i64le" => quote! { reader.read_i64::<byteorder::LittleEndian>()? },
                "u64le" => quote! { reader.read_u64::<byteorder::LittleEndian>()? },
                "f32le" => quote! { reader.read_f32::<byteorder::LittleEndian>()? },
                "f64le" => quote! { reader.read_f64::<byteorder::LittleEndian>()? },
                "bool" => quote! { reader.read_u8()? != 0 },
                _ => quote! { crate::traits::FromBinary::read_from(reader)? },
            };
            field_reads.push(quote! {
                #field_name: #read_expr,
            });
        } else {
            field_reads.push(quote! {
                #field_name: crate::traits::FromBinary::read_from(reader)?,
            });
        }
    }

    Ok(quote! {
        impl crate::traits::FromBinary for #name {
            fn read_from<R: std::io::Read>(reader: &mut R) -> crate::error::Result<Self> {
                use byteorder::ReadBytesExt;
                Ok(Self {
                    #(#field_reads)*
                })
            }
        }
    })
}

fn generate_to_binary(
    name: &Ident,
    fields: &[(Ident, Type, FieldAttrs)],
    _container_attrs: &ContainerAttrs,
) -> syn::Result<TokenStream> {
    let mut field_writes = Vec::new();
    let mut size_calcs = Vec::new();

    for (field_name, _field_type, attrs) in fields {
        if attrs.skip {
            continue;
        }

        if attrs.flatten {
            field_writes.push(quote! {
                crate::traits::ToBinary::write_to(&self.#field_name, writer)?;
            });
            size_calcs.push(quote! {
                + crate::traits::ToBinary::binary_size(&self.#field_name)
            });
            continue;
        }

        if attrs.unknown_binary {
            field_writes.push(quote! {
                writer.write_all(&self.#field_name)?;
            });
            size_calcs.push(quote! {
                + self.#field_name.len()
            });
            continue;
        }

        if let Some(skip_bytes) = attrs.skip_bytes {
            field_writes.push(quote! {
                writer.write_all(&[0u8; #skip_bytes])?;
            });
            size_calcs.push(quote! {
                + #skip_bytes
            });
            continue;
        }

        if attrs.coord_point {
            field_writes.push(quote! {
                crate::io::writer::write_coord_point(writer, self.#field_name)?;
            });
            size_calcs.push(quote! { + 8 }); // Two i32
        } else if attrs.coord {
            field_writes.push(quote! {
                writer.write_i32::<byteorder::LittleEndian>(self.#field_name.to_raw())?;
            });
            size_calcs.push(quote! { + 4 });
        } else if attrs.string_block {
            field_writes.push(quote! {
                crate::io::writer::write_string_block(writer, &self.#field_name)?;
            });
            size_calcs.push(quote! {
                + 4 + self.#field_name.len()
            });
        } else if attrs.pascal_string {
            field_writes.push(quote! {
                crate::io::writer::write_pascal_short_string(writer, &self.#field_name)?;
            });
            size_calcs.push(quote! {
                + 1 + self.#field_name.len()
            });
        } else if let Some(array_size) = attrs.array_size {
            field_writes.push(quote! {
                for item in &self.#field_name {
                    crate::traits::ToBinary::write_to(item, writer)?;
                }
            });
            size_calcs.push(quote! {
                + #array_size * std::mem::size_of_val(&self.#field_name[0])
            });
        } else if let Some(binary_ty) = &attrs.binary_ty {
            let (write_expr, size) = match binary_ty.as_str() {
                "i8" => (quote! { writer.write_i8(self.#field_name)? }, 1),
                "u8" => (quote! { writer.write_u8(self.#field_name)? }, 1),
                "i16le" => (quote! { writer.write_i16::<byteorder::LittleEndian>(self.#field_name)? }, 2),
                "u16le" => (quote! { writer.write_u16::<byteorder::LittleEndian>(self.#field_name)? }, 2),
                "i32le" => (quote! { writer.write_i32::<byteorder::LittleEndian>(self.#field_name)? }, 4),
                "u32le" => (quote! { writer.write_u32::<byteorder::LittleEndian>(self.#field_name)? }, 4),
                "i64le" => (quote! { writer.write_i64::<byteorder::LittleEndian>(self.#field_name)? }, 8),
                "u64le" => (quote! { writer.write_u64::<byteorder::LittleEndian>(self.#field_name)? }, 8),
                "f32le" => (quote! { writer.write_f32::<byteorder::LittleEndian>(self.#field_name)? }, 4),
                "f64le" => (quote! { writer.write_f64::<byteorder::LittleEndian>(self.#field_name)? }, 8),
                "bool" => (quote! { writer.write_u8(if self.#field_name { 1 } else { 0 })? }, 1),
                _ => (quote! { crate::traits::ToBinary::write_to(&self.#field_name, writer)? }, 0),
            };
            field_writes.push(write_expr);
            if size > 0 {
                size_calcs.push(quote! { + #size });
            } else {
                size_calcs.push(quote! {
                    + crate::traits::ToBinary::binary_size(&self.#field_name)
                });
            }
        } else {
            field_writes.push(quote! {
                crate::traits::ToBinary::write_to(&self.#field_name, writer)?;
            });
            size_calcs.push(quote! {
                + crate::traits::ToBinary::binary_size(&self.#field_name)
            });
        }
    }

    Ok(quote! {
        impl crate::traits::ToBinary for #name {
            fn write_to<W: std::io::Write>(&self, writer: &mut W) -> crate::error::Result<()> {
                use byteorder::WriteBytesExt;
                #(#field_writes)*
                Ok(())
            }

            fn binary_size(&self) -> usize {
                0 #(#size_calcs)*
            }
        }
    })
}

// NOTE: These functions are preserved for potential future use when migrating PCB records
// or if we decide to auto-generate SchPrimitive impls again.

#[allow(dead_code)]
fn generate_sch_primitive(
    name: &Ident,
    fields: &[(Ident, Type, FieldAttrs)],
    record_id: i32,
) -> syn::Result<TokenStream> {
    // Find the base field for owner_index
    let owner_index_field = fields
        .iter()
        .find(|(_, _, attrs)| attrs.flatten)
        .map(|(name, _, _)| name);

    let owner_index_impl = if let Some(base_field) = owner_index_field {
        quote! {
            fn owner_index(&self) -> i32 {
                self.#base_field.owner_index()
            }

            fn set_owner_index(&mut self, index: i32) {
                self.#base_field.set_owner_index(index);
            }
        }
    } else {
        // Look for direct owner_index field
        let direct_field = fields
            .iter()
            .find(|(name, _, attrs)| {
                *name == "owner_index"
                    || attrs.param.as_deref() == Some("OWNERINDEX")
            });

        if let Some((field_name, _, _)) = direct_field {
            quote! {
                fn owner_index(&self) -> i32 {
                    self.#field_name
                }

                fn set_owner_index(&mut self, index: i32) {
                    self.#field_name = index;
                }
            }
        } else {
            quote! {
                fn owner_index(&self) -> i32 { 0 }
                fn set_owner_index(&mut self, _index: i32) {}
            }
        }
    };

    Ok(quote! {
        impl crate::traits::SchPrimitive for #name {
            const RECORD_ID: i32 = #record_id;

            #owner_index_impl

            fn calculate_bounds(&self) -> crate::types::CoordRect {
                // Default implementation - override manually if needed
                crate::types::CoordRect::default()
            }
        }
    })
}

#[allow(dead_code)]
fn generate_pcb_primitive(
    name: &Ident,
    fields: &[(Ident, Type, FieldAttrs)],
    object_id: &Ident,
) -> syn::Result<TokenStream> {
    // Find the base field for layer
    let layer_field = fields
        .iter()
        .find(|(_, _, attrs)| attrs.flatten)
        .map(|(name, _, _)| name);

    let layer_impl = if let Some(base_field) = layer_field {
        quote! {
            fn layer(&self) -> crate::types::Layer {
                self.#base_field.layer()
            }
        }
    } else {
        quote! {
            fn layer(&self) -> crate::types::Layer {
                crate::types::Layer::default()
            }
        }
    };

    Ok(quote! {
        impl crate::traits::PcbPrimitive for #name {
            const OBJECT_ID: crate::records::pcb::PcbObjectId = crate::records::pcb::PcbObjectId::#object_id;

            #layer_impl

            fn calculate_bounds(&self) -> crate::types::CoordRect {
                // Default implementation - override manually if needed
                crate::types::CoordRect::default()
            }
        }
    })
}

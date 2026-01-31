//! Attribute parsing for Altium derive macros.

use syn::{Attribute, Expr, Ident, LitInt, LitStr, Token};

/// Container-level attributes for `#[altium(...)]`
#[derive(Debug, Default)]
pub struct ContainerAttrs {
    /// Record ID for schematic records (e.g., 2 for Pin)
    pub record_id: Option<i32>,
    /// Object ID for PCB records (e.g., "Pad")
    pub object_id: Option<Ident>,
    /// Serialization format: "params", "binary", or "both"
    pub format: Option<String>,
    /// Base name for AltiumBase derive
    pub base_name: Option<String>,
    /// Parent base type for inheritance
    pub extends: Option<String>,
}

impl ContainerAttrs {
    pub fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut result = ContainerAttrs::default();

        for attr in attrs {
            if !attr.path().is_ident("altium") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("record_id") {
                    meta.input.parse::<Token![=]>()?;
                    let lit: LitInt = meta.input.parse()?;
                    result.record_id = Some(lit.base10_parse()?);
                } else if meta.path.is_ident("object_id") {
                    meta.input.parse::<Token![=]>()?;
                    result.object_id = Some(meta.input.parse()?);
                } else if meta.path.is_ident("format") {
                    meta.input.parse::<Token![=]>()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.format = Some(lit.value());
                } else if meta.path.is_ident("base_name") {
                    meta.input.parse::<Token![=]>()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.base_name = Some(lit.value());
                } else if meta.path.is_ident("extends") {
                    meta.input.parse::<Token![=]>()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.extends = Some(lit.value());
                } else {
                    return Err(meta.error(format!(
                        "unknown container attribute: {}",
                        meta.path.get_ident().map(|i| i.to_string()).unwrap_or_default()
                    )));
                }
                Ok(())
            })?;
        }

        Ok(result)
    }
}

/// Field-level attributes for `#[altium(...)]`
#[derive(Debug, Default, Clone)]
pub struct FieldAttrs {
    /// Flatten a base type's fields
    pub flatten: bool,
    /// Parameter key name (e.g., "ELECTRICAL")
    pub param: Option<String>,
    /// Fractional part parameter key (e.g., "PINLENGTH_FRAC")
    pub frac: Option<String>,
    /// Use default value if missing
    pub has_default: bool,
    /// Specific default value (if not Default::default())
    pub default_value: Option<Expr>,
    /// Wrap in Option<T>
    pub optional: bool,
    /// Binary field type (e.g., "i32le")
    pub binary_ty: Option<String>,
    /// Binary coordinate point
    pub coord_point: bool,
    /// Binary coordinate value
    pub coord: bool,
    /// Binary string block (length-prefixed)
    pub string_block: bool,
    /// Binary pascal string (byte length)
    pub pascal_string: bool,
    /// Binary array size
    pub array_size: Option<usize>,
    /// Skip bytes during read
    pub skip_bytes: Option<usize>,
    /// Store unknown parameters
    pub unknown: bool,
    /// Store unknown binary bytes
    pub unknown_binary: bool,
    /// Skip field entirely
    pub skip: bool,
    /// List parameter (comma-separated)
    pub list: bool,
    /// Nested parameters
    pub nested: bool,
    /// Color parameter (Win32 COLORREF)
    pub color: bool,
    /// Skip emitting field when value equals Default::default()
    pub skip_default: bool,
    /// Indexed coordinate points (Vec<(i32, i32)>)
    pub indexed_coords: bool,
    /// X prefix for indexed coords (e.g., "X" for X1, X2, ...)
    pub prefix_x: Option<String>,
    /// Y prefix for indexed coords (e.g., "Y" for Y1, Y2, ...)
    pub prefix_y: Option<String>,
    /// Count parameter name (e.g., "LOCATIONCOUNT")
    pub count_param: Option<String>,
}

impl FieldAttrs {
    pub fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut result = FieldAttrs::default();

        for attr in attrs {
            if !attr.path().is_ident("altium") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("flatten") {
                    result.flatten = true;
                } else if meta.path.is_ident("param") {
                    meta.input.parse::<Token![=]>()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.param = Some(lit.value());
                } else if meta.path.is_ident("frac") {
                    meta.input.parse::<Token![=]>()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.frac = Some(lit.value());
                } else if meta.path.is_ident("default") {
                    result.has_default = true;
                    // Check for optional value
                    if meta.input.peek(Token![=]) {
                        meta.input.parse::<Token![=]>()?;
                        result.default_value = Some(meta.input.parse()?);
                    }
                } else if meta.path.is_ident("optional") {
                    result.optional = true;
                } else if meta.path.is_ident("binary") {
                    // Binary marker, look for ty= or specific type
                    result.binary_ty = Some("raw".to_string());
                } else if meta.path.is_ident("ty") {
                    meta.input.parse::<Token![=]>()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.binary_ty = Some(lit.value());
                } else if meta.path.is_ident("coord_point") {
                    result.coord_point = true;
                } else if meta.path.is_ident("coord") {
                    result.coord = true;
                } else if meta.path.is_ident("string_block") {
                    result.string_block = true;
                } else if meta.path.is_ident("pascal_string") {
                    result.pascal_string = true;
                } else if meta.path.is_ident("array") {
                    meta.input.parse::<Token![=]>()?;
                    let lit: LitInt = meta.input.parse()?;
                    result.array_size = Some(lit.base10_parse()?);
                } else if meta.path.is_ident("skip_bytes") {
                    meta.input.parse::<Token![=]>()?;
                    let lit: LitInt = meta.input.parse()?;
                    result.skip_bytes = Some(lit.base10_parse()?);
                } else if meta.path.is_ident("unknown") {
                    result.unknown = true;
                } else if meta.path.is_ident("unknown_binary") {
                    result.unknown_binary = true;
                } else if meta.path.is_ident("skip") {
                    result.skip = true;
                } else if meta.path.is_ident("list") {
                    result.list = true;
                } else if meta.path.is_ident("nested") {
                    result.nested = true;
                } else if meta.path.is_ident("color") {
                    result.color = true;
                } else if meta.path.is_ident("skip_default") {
                    result.skip_default = true;
                } else if meta.path.is_ident("indexed_coords") {
                    result.indexed_coords = true;
                } else if meta.path.is_ident("prefix_x") {
                    meta.input.parse::<Token![=]>()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.prefix_x = Some(lit.value());
                } else if meta.path.is_ident("prefix_y") {
                    meta.input.parse::<Token![=]>()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.prefix_y = Some(lit.value());
                } else if meta.path.is_ident("count") {
                    meta.input.parse::<Token![=]>()?;
                    let lit: LitStr = meta.input.parse()?;
                    result.count_param = Some(lit.value());
                } else {
                    return Err(meta.error(format!(
                        "unknown field attribute: {}",
                        meta.path.get_ident().map(|i| i.to_string()).unwrap_or_default()
                    )));
                }
                Ok(())
            })?;
        }

        Ok(result)
    }

    /// Check if this field is a parameter-based field
    #[allow(dead_code)]
    pub fn is_param_field(&self) -> bool {
        self.param.is_some()
    }

    /// Check if this field is a binary-based field
    #[allow(dead_code)]
    pub fn is_binary_field(&self) -> bool {
        self.binary_ty.is_some() || self.coord_point || self.coord ||
        self.string_block || self.pascal_string || self.array_size.is_some()
    }
}

/// Enum variant attributes
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
                        meta.path.get_ident().map(|i| i.to_string()).unwrap_or_default()
                    )));
                }
                Ok(())
            })?;
        }

        Ok(result)
    }
}

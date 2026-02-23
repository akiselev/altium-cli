use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Data, DeriveInput, Expr, Fields, Ident, Token, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

// ── FromParams derive ────────────────────────────────────────────────────────

/// Derive macro that generates `pub(crate) fn from_params(params: &mut ParameterCollection) -> Result<Self>`.
///
/// Each field must have a `#[param(...)]` attribute specifying how to extract its value.
#[proc_macro_derive(FromParams, attributes(param))]
pub fn derive_from_params(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_from_params(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_from_params(input: DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let vis = &input.vis;
    let fields = get_named_fields(&input)?;

    let mut field_inits = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_ty = &field.ty;
        let param_attr = get_param_attr(field)?;
        let strategy = param_attr.parse_args::<ParamStrategy>()?;
        let init_expr = strategy.to_deserialize_tokens(field_name, field_ty)?;
        field_inits.push(quote! { #field_name: #init_expr });
    }

    // Use the same visibility as the struct for the generated method.
    Ok(quote! {
        impl #name {
            #vis fn from_params(params: &mut ParameterCollection) -> Result<Self> {
                Ok(Self {
                    #(#field_inits,)*
                })
            }
        }
    })
}

// ── ToParams derive ──────────────────────────────────────────────────────────

/// Derive macro that generates `pub(crate) fn to_params(&self, params: &mut ParameterCollection)`.
///
/// Uses the same `#[param(...)]` attributes as `FromParams`. Serialization tier:
/// - T1 (default): skip parameter when value equals type default (0, false, "")
/// - T2 (`tier2` flag): always write parameter regardless of value
#[proc_macro_derive(ToParams, attributes(param))]
pub fn derive_to_params(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_to_params(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_to_params(input: DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let vis = &input.vis;
    let fields = get_named_fields(&input)?;

    let mut stmts = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let param_attr = get_param_attr(field)?;

        let (strategy, tier2, skip_default) =
            param_attr.parse_args_with(|input: ParseStream| {
                let (flags, named) = collect_tokens(input)?;
                let flag_strs: Vec<String> = flags.iter().map(|f| f.to_string()).collect();
                let is_tier2 = flag_strs.iter().any(|f| f == "tier2");
                let is_skip_default = flag_strs.iter().any(|f| f == "skip_default");
                let filtered_flags: Vec<Ident> = flags
                    .into_iter()
                    .filter(|f| f != "tier2" && f != "skip_default")
                    .collect();
                let strategy = build_strategy(filtered_flags, named)?;
                Ok((strategy, is_tier2, is_skip_default))
            })?;

        stmts.push(strategy.to_serialize_tokens(field_name, &field.ty, tier2, skip_default));
    }

    Ok(quote! {
        impl #name {
            #vis fn to_params(&self, params: &mut ParameterCollection) {
                #(#stmts)*
            }
        }
    })
}

// ── Shared helpers ───────────────────────────────────────────────────────────

fn get_named_fields(
    input: &DeriveInput,
) -> syn::Result<&syn::punctuated::Punctuated<syn::Field, Token![,]>> {
    match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(named) => Ok(&named.named),
            _ => Err(syn::Error::new_spanned(
                &input.ident,
                "requires a struct with named fields",
            )),
        },
        _ => Err(syn::Error::new_spanned(
            &input.ident,
            "can only be derived for structs",
        )),
    }
}

fn get_param_attr(field: &syn::Field) -> syn::Result<&syn::Attribute> {
    let field_name = field.ident.as_ref().unwrap();
    field
        .attrs
        .iter()
        .find(|a| a.path().is_ident("param"))
        .ok_or_else(|| syn::Error::new_spanned(field_name, "field missing #[param(...)] attribute"))
}

// ── Strategy ─────────────────────────────────────────────────────────────────

/// Represents the parsed content of a `#[param(...)]` attribute.
enum ParamStrategy {
    /// `#[param(key = PATH)]`
    Required { key: Expr },
    /// `#[param(key = PATH, default = EXPR)]`
    WithDefault { key: Expr, default: Expr },
    /// `#[param(key = PATH, optional)]`
    Optional { key: Expr },
    /// `#[param(coord, key = K, frac_key = FK)]`
    Coord { key: Expr, frac_key: Expr },
    /// `#[param(coord_point, x_key = XK, x_frac = XF, y_key = YK, y_frac = YF)]`
    CoordPoint {
        x_key: Expr,
        x_frac: Expr,
        y_key: Expr,
        y_frac: Expr,
    },
    /// `#[param(indexed_coords, count_key = CK, x_prefix = XP, y_prefix = YP)]`
    IndexedCoords {
        count_key: Expr,
        x_prefix: Expr,
        y_prefix: Expr,
    },
    /// `#[param(flatten)]`
    Flatten,
    /// `#[param(list, key = PATH)]`
    List { key: Expr },
    /// `#[param(list_or_empty, key = PATH)]`
    ListOrEmpty { key: Expr },
}

impl ParamStrategy {
    // Generates the expression that deserializes a field from a ParameterCollection.
    fn to_deserialize_tokens(
        &self,
        _field_name: &Ident,
        field_ty: &Type,
    ) -> syn::Result<TokenStream2> {
        Ok(match self {
            ParamStrategy::Required { key } => {
                quote! { params.remove_required::<#field_ty>(#key)? }
            }
            ParamStrategy::WithDefault { key, default } => {
                quote! { params.remove_with_default::<#field_ty>(#key, #default)? }
            }
            ParamStrategy::Optional { key } => {
                // field_ty is Option<T>, but remove_optional infers T from the Option wrapper
                quote! { params.remove_optional(#key)? }
            }
            ParamStrategy::Coord { key, frac_key } => {
                quote! { params.remove_coord(#key, #frac_key)? }
            }
            ParamStrategy::CoordPoint {
                x_key,
                x_frac,
                y_key,
                y_frac,
            } => {
                quote! {
                    {
                        let x = params.remove_coord(#x_key, #x_frac)?;
                        let y = params.remove_coord(#y_key, #y_frac)?;
                        CoordPoint::new(x, y)
                    }
                }
            }
            ParamStrategy::IndexedCoords {
                count_key,
                x_prefix,
                y_prefix,
            } => {
                quote! { params.remove_indexed_coords(#count_key, #x_prefix, #y_prefix)? }
            }
            ParamStrategy::Flatten => {
                // Extract the inner type T from the field type for the flatten call.
                quote! { <#field_ty>::from_params(params)? }
            }
            ParamStrategy::List { key } => {
                quote! { params.remove_list(#key)? }
            }
            ParamStrategy::ListOrEmpty { key } => {
                quote! { params.remove_list_or_empty(#key)? }
            }
        })
    }

    // Generates the statement that serializes a field into a ParameterCollection.
    // `tier2`: if true, always write (T2); if false, skip when value equals type zero (T1).
    // `skip_default`: if true, skip when value equals the `default` expression (not type zero).
    // `field_type`: the concrete type, used for unambiguous Default::default() calls.
    fn to_serialize_tokens(
        &self,
        field_name: &Ident,
        field_type: &syn::Type,
        tier2: bool,
        skip_default: bool,
    ) -> TokenStream2 {
        match self {
            ParamStrategy::Required { key } => {
                if tier2 {
                    quote! {
                        params.insert(#key, self.#field_name.to_param_value());
                    }
                } else {
                    quote! {
                        if self.#field_name != <#field_type as Default>::default() {
                            params.insert(#key, self.#field_name.to_param_value());
                        }
                    }
                }
            }
            ParamStrategy::WithDefault { key, default } => {
                if tier2 {
                    quote! {
                        params.insert(#key, self.#field_name.to_param_value());
                    }
                } else if skip_default {
                    // skip_default: skip when value equals the parse default expression.
                    // Used when Altium omits the field when it matches the non-zero default
                    // (e.g., Text="*" is omitted, not Text="").
                    quote! {
                        if self.#field_name != #default {
                            params.insert(#key, self.#field_name.to_param_value());
                        }
                    }
                } else {
                    // T1: skip when value == type zero. The `default` attribute only
                    // affects parsing (value used when the param is absent). Altium's
                    // T1 always compares against the type's zero value (0, false, "").
                    quote! {
                        if self.#field_name != <#field_type as Default>::default() {
                            params.insert(#key, self.#field_name.to_param_value());
                        }
                    }
                }
            }
            ParamStrategy::Optional { key } => {
                quote! {
                    if let Some(ref v) = self.#field_name {
                        params.insert(#key, v.to_param_value());
                    }
                }
            }
            ParamStrategy::Coord { key, frac_key } => {
                if tier2 {
                    quote! {
                        params.insert_coord(#key, #frac_key, self.#field_name);
                    }
                } else {
                    quote! {
                        if self.#field_name.to_internal() != 0 {
                            params.insert_coord(#key, #frac_key, self.#field_name);
                        }
                    }
                }
            }
            ParamStrategy::CoordPoint {
                x_key,
                x_frac,
                y_key,
                y_frac,
            } => {
                if tier2 {
                    quote! {
                        params.insert_coord_point(
                            #x_key, #x_frac, #y_key, #y_frac, self.#field_name
                        );
                    }
                } else {
                    // T1: each coord independently skipped if zero
                    quote! {
                        if self.#field_name.x.to_internal() != 0 {
                            params.insert_coord(#x_key, #x_frac, self.#field_name.x);
                        }
                        if self.#field_name.y.to_internal() != 0 {
                            params.insert_coord(#y_key, #y_frac, self.#field_name.y);
                        }
                    }
                }
            }
            ParamStrategy::IndexedCoords {
                count_key,
                x_prefix,
                y_prefix,
            } => {
                // Always write: parsing uses remove_required for count key
                quote! {
                    params.insert_indexed_coords(
                        #count_key, #x_prefix, #y_prefix, &self.#field_name
                    );
                }
            }
            ParamStrategy::Flatten => {
                quote! {
                    self.#field_name.to_params(params);
                }
            }
            ParamStrategy::List { key } => {
                quote! {
                    {
                        let s: String = self.#field_name.iter()
                            .map(|v| v.to_param_value())
                            .collect::<Vec<_>>()
                            .join(",");
                        params.insert(#key, s);
                    }
                }
            }
            ParamStrategy::ListOrEmpty { key } => {
                quote! {
                    if !self.#field_name.is_empty() {
                        let s: String = self.#field_name.iter()
                            .map(|v| v.to_param_value())
                            .collect::<Vec<_>>()
                            .join(",");
                        params.insert(#key, s);
                    }
                }
            }
        }
    }
}

// ── Attribute parsing ────────────────────────────────────────────────────────

/// A `name = expr` pair as it appears inside `#[param(...)]`.
struct NameValue {
    name: Ident,
    value: Expr,
}

impl Parse for NameValue {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: Expr = input.parse()?;
        Ok(NameValue { name, value })
    }
}

/// The tokens that can appear inside `#[param(...)]`.
enum ParamToken {
    /// A bare keyword like `coord`, `flatten`, `optional`, `list`, `list_or_empty`,
    /// `coord_point`, `indexed_coords`, `tier2`.
    Flag(Ident),
    /// A `name = expr` pair.
    NameValue(NameValue),
}

impl Parse for ParamToken {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            let value: Expr = input.parse()?;
            Ok(ParamToken::NameValue(NameValue { name: ident, value }))
        } else {
            Ok(ParamToken::Flag(ident))
        }
    }
}

// Collects all tokens from a `#[param(...)]` attribute into flags and named pairs.
fn collect_tokens(
    input: ParseStream<'_>,
) -> syn::Result<(Vec<Ident>, std::collections::HashMap<String, Expr>)> {
    let tokens: Punctuated<ParamToken, Token![,]> = Punctuated::parse_terminated(input)?;
    let mut flags = Vec::new();
    let mut named = std::collections::HashMap::new();
    for token in tokens {
        match token {
            ParamToken::Flag(f) => flags.push(f),
            ParamToken::NameValue(nv) => {
                named.insert(nv.name.to_string(), nv.value);
            }
        }
    }
    Ok((flags, named))
}

// Determines the ParamStrategy variant from pre-parsed flags and named pairs.
fn build_strategy(
    flags: Vec<Ident>,
    mut named: std::collections::HashMap<String, Expr>,
) -> syn::Result<ParamStrategy> {
    let flag_names: Vec<String> = flags.iter().map(|f| f.to_string()).collect();
    let flag_names: Vec<&str> = flag_names.iter().map(String::as_str).collect();

    if flag_names.contains(&"flatten") {
        if !named.is_empty() || flags.len() > 1 {
            return Err(syn::Error::new(
                flags[0].span(),
                "#[param(flatten)] takes no other arguments",
            ));
        }
        return Ok(ParamStrategy::Flatten);
    }

    if flag_names.contains(&"coord") {
        let key = named.remove("key").ok_or_else(|| {
            syn::Error::new(flags[0].span(), "#[param(coord)] requires `key = ...`")
        })?;
        let frac_key = named.remove("frac_key").ok_or_else(|| {
            syn::Error::new(flags[0].span(), "#[param(coord)] requires `frac_key = ...`")
        })?;
        if !named.is_empty() {
            return Err(syn::Error::new(
                flags[0].span(),
                "#[param(coord)] has unrecognized arguments",
            ));
        }
        return Ok(ParamStrategy::Coord { key, frac_key });
    }

    if flag_names.contains(&"coord_point") {
        let x_key = named.remove("x_key").ok_or_else(|| {
            syn::Error::new(
                flags[0].span(),
                "#[param(coord_point)] requires `x_key = ...`",
            )
        })?;
        let x_frac = named.remove("x_frac").ok_or_else(|| {
            syn::Error::new(
                flags[0].span(),
                "#[param(coord_point)] requires `x_frac = ...`",
            )
        })?;
        let y_key = named.remove("y_key").ok_or_else(|| {
            syn::Error::new(
                flags[0].span(),
                "#[param(coord_point)] requires `y_key = ...`",
            )
        })?;
        let y_frac = named.remove("y_frac").ok_or_else(|| {
            syn::Error::new(
                flags[0].span(),
                "#[param(coord_point)] requires `y_frac = ...`",
            )
        })?;
        return Ok(ParamStrategy::CoordPoint {
            x_key,
            x_frac,
            y_key,
            y_frac,
        });
    }

    if flag_names.contains(&"indexed_coords") {
        let count_key = named.remove("count_key").ok_or_else(|| {
            syn::Error::new(
                flags[0].span(),
                "#[param(indexed_coords)] requires `count_key = ...`",
            )
        })?;
        let x_prefix = named.remove("x_prefix").ok_or_else(|| {
            syn::Error::new(
                flags[0].span(),
                "#[param(indexed_coords)] requires `x_prefix = ...`",
            )
        })?;
        let y_prefix = named.remove("y_prefix").ok_or_else(|| {
            syn::Error::new(
                flags[0].span(),
                "#[param(indexed_coords)] requires `y_prefix = ...`",
            )
        })?;
        return Ok(ParamStrategy::IndexedCoords {
            count_key,
            x_prefix,
            y_prefix,
        });
    }

    if flag_names.contains(&"list") {
        let key = named.remove("key").ok_or_else(|| {
            syn::Error::new(flags[0].span(), "#[param(list)] requires `key = ...`")
        })?;
        return Ok(ParamStrategy::List { key });
    }

    if flag_names.contains(&"list_or_empty") {
        let key = named.remove("key").ok_or_else(|| {
            syn::Error::new(
                flags[0].span(),
                "#[param(list_or_empty)] requires `key = ...`",
            )
        })?;
        return Ok(ParamStrategy::ListOrEmpty { key });
    }

    // No mode flag: must have `key = ...`; optionally `default = ...` or `optional`.
    let key = named.remove("key").ok_or_else(|| {
        // Point at the first token if available, otherwise use a generic span.
        let span = flags
            .first()
            .map(|f| f.span())
            .unwrap_or_else(proc_macro2::Span::call_site);
        syn::Error::new(span, "#[param] requires a mode flag or `key = ...`")
    })?;

    let is_optional = flag_names.contains(&"optional");
    let default_expr = named.remove("default");

    match (is_optional, default_expr) {
        (true, Some(_)) => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[param] cannot have both `optional` and `default`",
        )),
        (true, None) => Ok(ParamStrategy::Optional { key }),
        (false, Some(default)) => Ok(ParamStrategy::WithDefault { key, default }),
        (false, None) => Ok(ParamStrategy::Required { key }),
    }
}

impl Parse for ParamStrategy {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let (flags, named) = collect_tokens(input)?;
        build_strategy(flags, named)
    }
}

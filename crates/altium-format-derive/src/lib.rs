use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Data, DeriveInput, Expr, Fields, Ident, Token, Type,
    parse_macro_input,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

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

    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(named) => &named.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "FromParams requires a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "FromParams can only be derived for structs",
            ));
        }
    };

    let mut field_inits = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_ty = &field.ty;

        let param_attr = field
            .attrs
            .iter()
            .find(|a| a.path().is_ident("param"))
            .ok_or_else(|| {
                syn::Error::new_spanned(
                    field_name,
                    "field missing #[param(...)] attribute",
                )
            })?;

        let strategy = param_attr.parse_args::<ParamStrategy>()?;
        let init_expr = strategy.to_tokens(field_name, field_ty)?;
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
    CoordPoint { x_key: Expr, x_frac: Expr, y_key: Expr, y_frac: Expr },
    /// `#[param(indexed_coords, count_key = CK, x_prefix = XP, y_prefix = YP)]`
    IndexedCoords { count_key: Expr, x_prefix: Expr, y_prefix: Expr },
    /// `#[param(flatten)]`
    Flatten,
    /// `#[param(list, key = PATH)]`
    List { key: Expr },
    /// `#[param(list_or_empty, key = PATH)]`
    ListOrEmpty { key: Expr },
}

impl ParamStrategy {
    fn to_tokens(&self, _field_name: &Ident, field_ty: &Type) -> syn::Result<TokenStream2> {
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
            ParamStrategy::CoordPoint { x_key, x_frac, y_key, y_frac } => {
                quote! {
                    {
                        let x = params.remove_coord(#x_key, #x_frac)?;
                        let y = params.remove_coord(#y_key, #y_frac)?;
                        CoordPoint::new(x, y)
                    }
                }
            }
            ParamStrategy::IndexedCoords { count_key, x_prefix, y_prefix } => {
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
    /// A bare keyword like `coord`, `flatten`, `optional`, `list`, `list_or_empty`, `coord_point`, `indexed_coords`.
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

impl Parse for ParamStrategy {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let tokens: Punctuated<ParamToken, Token![,]> =
            Punctuated::parse_terminated(input)?;

        let mut flags: Vec<Ident> = Vec::new();
        let mut named: std::collections::HashMap<String, Expr> =
            std::collections::HashMap::new();

        for token in tokens {
            match token {
                ParamToken::Flag(f) => flags.push(f),
                ParamToken::NameValue(nv) => {
                    named.insert(nv.name.to_string(), nv.value);
                }
            }
        }

        // Determine which variant we're in based on the flags/keys present.
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
                syn::Error::new(flags[0].span(), "#[param(coord_point)] requires `x_key = ...`")
            })?;
            let x_frac = named.remove("x_frac").ok_or_else(|| {
                syn::Error::new(flags[0].span(), "#[param(coord_point)] requires `x_frac = ...`")
            })?;
            let y_key = named.remove("y_key").ok_or_else(|| {
                syn::Error::new(flags[0].span(), "#[param(coord_point)] requires `y_key = ...`")
            })?;
            let y_frac = named.remove("y_frac").ok_or_else(|| {
                syn::Error::new(flags[0].span(), "#[param(coord_point)] requires `y_frac = ...`")
            })?;
            return Ok(ParamStrategy::CoordPoint { x_key, x_frac, y_key, y_frac });
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
            return Ok(ParamStrategy::IndexedCoords { count_key, x_prefix, y_prefix });
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
}

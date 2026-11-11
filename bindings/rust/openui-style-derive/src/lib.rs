//! `#[derive(ConstructorTokens)]` for `openui-style` value types.
//!
//! The derive is internal to `openui-style`: the generated impl names
//! `crate::constructor_tokens`. It lets `openui-macros` parse a style literal
//! at compile time and emit an expression that builds the same typed value,
//! so nothing is parsed at run time (RFC 0001).
//!
//! A variant marked `#[constructor_tokens(unsupported)]` is one that no style
//! literal produces, such as a value sampled from an animation. Asking for its
//! tokens returns `Unsupported` instead of naming its fields.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as Tokens};
use quote::{format_ident, quote};
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields, Ident, LitStr};

#[proc_macro_derive(ConstructorTokens, attributes(constructor_tokens))]
pub fn derive_constructor_tokens(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(input: &DeriveInput) -> syn::Result<Tokens> {
    let name = &input.ident;
    let module = quote!(crate::constructor_tokens);
    let mut generics = input.generics.clone();
    for parameter in generics.type_params_mut() {
        parameter.bounds.push(syn::parse_quote!(
            crate::constructor_tokens::ConstructorTokens
        ));
    }
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let body = match &input.data {
        Data::Struct(data) => {
            if is_unsupported(&input.attrs)? {
                return Err(syn::Error::new_spanned(
                    name,
                    "`unsupported` applies to enum variants only",
                ));
            }
            let (pattern, build) = construct(quote!(Self), &[name.clone()], &data.fields);
            quote! {
                let #pattern = self;
                #build
            }
        }
        Data::Enum(data) => {
            let mut arms = Vec::with_capacity(data.variants.len());
            for variant in &data.variants {
                let ident = &variant.ident;
                if is_unsupported(&variant.attrs)? {
                    let rest = match &variant.fields {
                        Fields::Named(_) => quote!({ .. }),
                        Fields::Unnamed(_) => quote!((..)),
                        Fields::Unit => quote!(),
                    };
                    let what = LitStr::new(&format!("{name}::{ident}"), Span::call_site());
                    arms.push(quote! {
                        Self::#ident #rest => ::core::result::Result::Err(
                            #module::Unsupported::new(#what)
                        ),
                    });
                } else {
                    let (pattern, build) = construct(
                        quote!(Self::#ident),
                        &[name.clone(), ident.clone()],
                        &variant.fields,
                    );
                    arms.push(quote!(#pattern => { #build }));
                }
            }
            quote! {
                match self {
                    #(#arms)*
                }
            }
        }
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                name,
                "ConstructorTokens cannot be derived for unions",
            ))
        }
    };
    Ok(quote! {
        impl #impl_generics #module::ConstructorTokens for #name #type_generics #where_clause {
            fn constructor_tokens(
                &self,
                root: &#module::TokenStream,
            ) -> ::core::result::Result<#module::TokenStream, #module::Unsupported> {
                #body
            }
        }
    })
}

/// A destructuring pattern for `fields` and the statements that build the
/// constructor expression `root::<segments>` from the bound fields.
fn construct(pattern_path: Tokens, segments: &[Ident], fields: &Fields) -> (Tokens, Tokens) {
    let module = quote!(crate::constructor_tokens);
    let names: Vec<String> = segments.iter().map(Ident::to_string).collect();
    let path = quote!(#module::path(root, &[#(#names),*]));
    match fields {
        Fields::Unit => (pattern_path, quote!(::core::result::Result::Ok(#path))),
        Fields::Unnamed(unnamed) => {
            let bindings: Vec<Ident> = (0..unnamed.unnamed.len())
                .map(|index| format_ident!("__field_{index}"))
                .collect();
            let build = quote! {
                ::core::result::Result::Ok(#module::call(
                    #path,
                    ::std::vec![#(
                        #module::ConstructorTokens::constructor_tokens(#bindings, root)?
                    ),*],
                ))
            };
            (quote!(#pattern_path(#(#bindings),*)), build)
        }
        Fields::Named(named) => {
            let idents: Vec<&Ident> = named
                .named
                .iter()
                .map(|field| field.ident.as_ref().expect("named field"))
                .collect();
            let bindings: Vec<Ident> = idents
                .iter()
                .map(|ident| {
                    format_ident!("__field_{}", ident.to_string().trim_start_matches("r#"))
                })
                .collect();
            let labels: Vec<String> = idents.iter().map(|ident| ident.to_string()).collect();
            let build = quote! {
                ::core::result::Result::Ok(#module::braced(
                    #path,
                    ::std::vec![#((
                        #labels,
                        #module::ConstructorTokens::constructor_tokens(#bindings, root)?,
                    )),*],
                ))
            };
            (quote!(#pattern_path { #(#idents: #bindings),* }), build)
        }
    }
}

fn is_unsupported(attributes: &[Attribute]) -> syn::Result<bool> {
    let mut unsupported = false;
    for attribute in attributes {
        if !attribute.path().is_ident("constructor_tokens") {
            continue;
        }
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("unsupported") {
                unsupported = true;
                Ok(())
            } else {
                Err(meta.error("expected `unsupported`"))
            }
        })?;
    }
    Ok(unsupported)
}

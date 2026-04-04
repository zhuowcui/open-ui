//! `#[component]` attribute macro implementation.
//!
//! Transforms a function with named parameters into a component with a
//! generated `Props` struct.

use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{Ident, ItemFn};

/// Entry point for the `#[component]` attribute macro.
pub(crate) fn component_macro(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let func: ItemFn = match syn::parse2(input) {
        Ok(f) => f,
        Err(err) => return err.to_compile_error(),
    };

    match transform_component(func) {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error(),
    }
}

/// Perform the component transformation.
fn transform_component(func: ItemFn) -> syn::Result<TokenStream> {
    let vis = &func.vis;
    let fn_name = &func.sig.ident;
    let fn_attrs = &func.attrs;
    let return_type = &func.sig.output;
    let stmts = &func.block.stmts;
    let generics = &func.sig.generics;

    let props_name = format_ident!("{}Props", fn_name);

    let mut struct_fields = Vec::new();
    let mut destructure_stmts = Vec::new();

    for param in &func.sig.inputs {
        let typed = match param {
            syn::FnArg::Typed(t) => t,
            syn::FnArg::Receiver(r) => {
                return Err(syn::Error::new_spanned(
                    r,
                    "components cannot have a `self` parameter",
                ));
            }
        };

        let ident = match &*typed.pat {
            syn::Pat::Ident(pat_ident) => &pat_ident.ident,
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "component parameters must be simple identifiers",
                ));
            }
        };

        let ty = &typed.ty;
        let is_optional = has_prop_attr(&typed.attrs, "optional");
        let is_children = has_prop_attr(&typed.attrs, "children");

        if is_optional {
            struct_fields.push(quote! {
                /// Auto-generated optional prop.
                pub #ident: ::std::option::Option<#ty>
            });
            destructure_stmts.push(quote! {
                #[allow(unused_variables)]
                let #ident: #ty = props.#ident.unwrap_or_default();
            });
        } else if is_children {
            struct_fields.push(quote! {
                /// Auto-generated children prop.
                pub #ident: #ty
            });
            destructure_stmts.push(quote! {
                #[allow(unused_variables)]
                let #ident: #ty = props.#ident;
            });
        } else {
            struct_fields.push(quote! {
                /// Auto-generated required prop.
                pub #ident: #ty
            });
            destructure_stmts.push(quote! {
                #[allow(unused_variables)]
                let #ident: #ty = props.#ident;
            });
        }
    }

    let span = fn_name.span();

    Ok(quote_spanned! {span=>
        /// Auto-generated props struct for the [`#fn_name`] component.
        #[allow(non_camel_case_types)]
        #vis struct #props_name {
            #(#struct_fields),*
        }

        #(#fn_attrs)*
        #vis fn #fn_name #generics (props: #props_name) #return_type {
            #(#destructure_stmts)*
            #(#stmts)*
        }
    })
}

/// Check if any attribute on a parameter is `#[prop(keyword)]`.
fn has_prop_attr(attrs: &[syn::Attribute], keyword: &str) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("prop") {
            return false;
        }
        attr.parse_args::<Ident>()
            .map(|i| i == keyword)
            .unwrap_or(false)
    })
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn basic_component() {
        let input = quote! {
            fn Counter(initial: i32) -> ViewNode {
                let _ = initial;
                ViewNode::Empty
            }
        };
        let output = component_macro(TokenStream::new(), input);
        let output_str = output.to_string();
        assert!(output_str.contains("CounterProps"));
        assert!(output_str.contains("pub initial : i32"));
        assert!(output_str.contains("props : CounterProps"));
    }

    #[test]
    fn optional_prop() {
        let input = quote! {
            fn Card(title: String, #[prop(optional)] subtitle: String) -> ViewNode {
                let _ = (title, subtitle);
                ViewNode::Empty
            }
        };
        let output = component_macro(TokenStream::new(), input);
        let output_str = output.to_string();
        assert!(output_str.contains("pub title : String"));
        assert!(output_str.contains("Option < String >"));
        assert!(output_str.contains("unwrap_or_default"));
    }

    #[test]
    fn no_params() {
        let input = quote! {
            fn EmptyComponent() -> ViewNode {
                ViewNode::Empty
            }
        };
        let output = component_macro(TokenStream::new(), input);
        let output_str = output.to_string();
        assert!(output_str.contains("EmptyComponentProps"));
        assert!(output_str.contains("props : EmptyComponentProps"));
    }

    #[test]
    fn self_param_error() {
        let input = quote! {
            fn Bad(self) -> ViewNode {
                ViewNode::Empty
            }
        };
        let output = component_macro(TokenStream::new(), input);
        let output_str = output.to_string();
        assert!(output_str.contains("compile_error"));
    }
}

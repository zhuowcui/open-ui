//! Compile-time cases for the typed style constructor tests.

use proc_macro2::{Span, TokenStream};
use quote::quote;

use crate::view::typed_value_tokens;

/// One `css-name: literal` per line; `#` starts a comment line. Each line
/// must parse, so the corpus cannot silently shrink.
const CORPUS: &str = include_str!("typed_style_corpus.txt");

pub(crate) fn expand(input: TokenStream) -> TokenStream {
    if !input.is_empty() {
        return syn::Error::new(Span::call_site(), "expected no input").to_compile_error();
    }
    let mut cases = Vec::new();
    for metadata in openui_style::PROPERTY_METADATA {
        if let Ok(value) = openui_style::parse_literal(metadata.property, metadata.initial) {
            cases.push((metadata.property, metadata.initial.to_owned(), value));
        }
    }
    let mut errors = Vec::new();
    for (number, line) in CORPUS.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, literal)) = line.split_once(':') else {
            errors.push(error(number, line, "expected `css-name: literal`"));
            continue;
        };
        let Some(property) = openui_style::StyleProperty::from_css_name(name.trim()) else {
            errors.push(error(number, line, "unknown property"));
            continue;
        };
        match openui_style::parse_literal(property, literal.trim()) {
            Ok(value) => cases.push((property, literal.trim().to_owned(), value)),
            Err(parse) => errors.push(error(number, line, &parse.to_string())),
        }
    }
    if !errors.is_empty() {
        // Brace-delimited `compile_error!` items, then a value of the
        // expected type, so every corpus error is reported in one build.
        return quote!({ #(#errors)* ::std::vec::Vec::new() });
    }
    let mut rows = Vec::with_capacity(cases.len());
    for (property, literal, value) in cases {
        let constructed = match typed_value_tokens(&value) {
            Ok(tokens) => tokens,
            Err(message) => return error(0, &literal, &message),
        };
        let variant = syn::Ident::new(&format!("{property:?}"), Span::call_site());
        rows.push(quote! {
            (::openui::typed_style::StyleProperty::#variant, #literal, #constructed)
        });
    }
    quote! {{
        let cases: ::std::vec::Vec<(
            ::openui::typed_style::StyleProperty,
            &'static str,
            ::openui::typed_style::StyleValue,
        )> = ::std::vec![#(#rows),*];
        cases
    }}
}

fn error(number: usize, line: &str, message: &str) -> TokenStream {
    let message = format!("typed_style_corpus.txt:{}: `{line}`: {message}", number + 1);
    syn::Error::new(Span::call_site(), message).to_compile_error()
}

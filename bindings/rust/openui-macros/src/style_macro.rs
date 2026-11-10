//! `style!`: a CSS declaration block compiled to a typed `Style` (RFC 0001).
//!
//! The block is read as CSS text and parsed by `openui-style`'s macro-time
//! declaration parser, which uses the runtime `parse_literal` for each value.
//! Each value is then emitted as its constructor expression, so nothing is
//! parsed at run time.
//!
//! Two forms are accepted:
//!
//! - `style! { padding: 8px 16px; color: red }`. The CSS text is the macro
//!   call's own source text. Rust has already split it into tokens, which
//!   loses the spacing that CSS depends on (`space-between` against
//!   `a - b`), so the macro reads the source and checks that it lexes to the
//!   same tokens it received. Inside another macro the source may not be the
//!   CSS that was passed; then the check fails and asks for the string form.
//! - `style!("padding: 8px 16px; color: red")`, for any block, including CSS
//!   that Rust cannot tokenize (`#3e3`, single-quoted strings).

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::quote;

use crate::view::{property_tokens, typed_value_tokens};

/// `source` is the macro call's source text, when the compiler has it.
pub(crate) fn expand(input: TokenStream, source: Option<String>) -> TokenStream {
    let (text, span) = match css_text(&input, source) {
        Ok(found) => found,
        Err(error) => return error.to_compile_error(),
    };
    let declarations = match openui_style::declaration_block::parse_declaration_block(&text) {
        Ok(declarations) => declarations,
        Err(errors) => {
            let errors = errors
                .into_iter()
                .map(|error| syn::Error::new(span, error.message).to_compile_error());
            return quote!({ #(#errors)* ::openui::typed_style::Style::default() });
        }
    };
    let mut items = Vec::with_capacity(declarations.len());
    for declaration in &declarations {
        let css = &text[declaration.source.clone()];
        if declaration.important {
            return syn::Error::new(
                span,
                format!(
                    "`{css}`: `!important` has no effect on element-attached styles until \
                     style rules land (RFC 0001); remove it"
                ),
            )
            .to_compile_error();
        }
        let property = property_tokens(declaration.property);
        let value = match typed_value_tokens(&declaration.value) {
            Ok(value) => value,
            Err(error) => {
                return syn::Error::new(span, format!("`{css}`: {error}")).to_compile_error()
            }
        };
        items.push(quote! {
            ::openui::typed_style::Declaration { property: #property, value: #value }
        });
    }
    quote! {
        <::openui::typed_style::Style as ::core::iter::FromIterator<
            ::openui::typed_style::Declaration,
        >>::from_iter([#(#items),*])
    }
}

/// The CSS text of the block and the span its errors point at.
fn css_text(input: &TokenStream, source: Option<String>) -> syn::Result<(String, Span)> {
    let mut tokens: Vec<TokenTree> = input.clone().into_iter().collect();
    // A `$css:literal` from `macro_rules!` arrives in an invisible group.
    while let [TokenTree::Group(group)] = tokens.as_slice() {
        if group.delimiter() != Delimiter::None {
            break;
        }
        tokens = group.stream().into_iter().collect();
    }
    if let [TokenTree::Literal(literal)] = tokens.as_slice() {
        if let Ok(string) = syn::parse2::<syn::LitStr>(TokenTree::Literal(literal.clone()).into()) {
            return Ok((string.value(), string.span()));
        }
        return Err(syn::Error::new(
            literal.span(),
            "style! takes a CSS declaration block or one string literal",
        ));
    }
    if tokens.is_empty() {
        return Ok((String::new(), Span::call_site()));
    }
    let unreadable = || {
        syn::Error::new(
            Span::call_site(),
            "style! cannot read this block's CSS text from the source, which happens \
             inside another macro; write it as a string: style!(\"...\")",
        )
    };
    let body = source
        .as_deref()
        .and_then(macro_body)
        .ok_or_else(unreadable)?;
    let body = strip_line_comments(body);
    let relexed: TokenStream = body.parse().map_err(|_| unreadable())?;
    if relexed.to_string() != input.to_string() {
        return Err(unreadable());
    }
    Ok((body, Span::call_site()))
}

/// The text between the outer delimiters of `name! { ... }`, `name!( ... )`
/// or `name![ ... ]`.
fn macro_body(source: &str) -> Option<&str> {
    let bang = source.find('!')?;
    let rest = source[bang + 1..].trim_start();
    let close = match rest.chars().next()? {
        '{' => '}',
        '(' => ')',
        '[' => ']',
        _ => return None,
    };
    let inner = rest[1..].trim_end().strip_suffix(close)?;
    // Only a single delimited group is a macro body.
    let group: TokenStream = rest.trim_end().parse().ok()?;
    let mut trees = group.into_iter();
    match (trees.next(), trees.next()) {
        (Some(TokenTree::Group(group)), None) if group.delimiter() != Delimiter::None => {
            Some(inner)
        }
        _ => None,
    }
}

/// Rust `//` comments are not CSS. They are dropped outside strings and
/// brackets (so `url(http://...)` keeps its slashes); the token check still
/// sees the same tokens because Rust drops them too.
fn strip_line_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut quote = None;
    let mut depth = 0usize;
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if let Some(open) = quote {
            out.push(character);
            if character == '\\' {
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            } else if character == open {
                quote = None;
            }
            continue;
        }
        match character {
            '"' | '\'' => quote = Some(character),
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            '/' if depth == 0 && chars.peek() == Some(&'/') => {
                for skipped in chars.by_ref() {
                    if skipped == '\n' {
                        out.push('\n');
                        break;
                    }
                }
                continue;
            }
            _ => {}
        }
        out.push(character);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expand_source(source: &str) -> String {
        let body = macro_body(source).expect("macro body");
        let input: TokenStream = body.parse().expect("tokens");
        expand(input, Some(source.to_owned())).to_string()
    }

    #[test]
    fn braced_blocks_read_their_source_text() {
        let tokens = expand_source("style! { justify-content: space-between; padding: 8px 16px }");
        assert!(
            tokens.contains("StyleProperty :: JustifyContent"),
            "{tokens}"
        );
        assert!(tokens.contains("StyleProperty :: Padding"), "{tokens}");
        assert!(!tokens.contains("compile_error"), "{tokens}");
        assert!(!tokens.contains("parse_literal"), "{tokens}");
    }

    #[test]
    fn string_blocks_need_no_source_text() {
        let input = quote!("color: #3e3; font-family: 'Inter'");
        let tokens = expand(input, None).to_string();
        assert!(tokens.contains("StyleProperty :: Color"), "{tokens}");
        assert!(tokens.contains("StyleProperty :: FontFamily"), "{tokens}");
        assert!(!tokens.contains("compile_error"), "{tokens}");
    }

    #[test]
    fn source_that_does_not_match_the_tokens_is_rejected() {
        // A macro_rules wrapper: the call site's text is not the CSS passed in.
        let input = quote!(display: flex);
        let tokens = expand(input, Some("style! { $($css)* }".to_owned())).to_string();
        assert!(tokens.contains("write it as a string"), "{tokens}");
        let tokens = expand(quote!(display: flex), None).to_string();
        assert!(tokens.contains("write it as a string"), "{tokens}");
    }

    #[test]
    fn rust_line_comments_are_not_css() {
        let tokens = expand_source("style! {\n  // spacing\n  padding: 8px; // trailing\n}");
        assert!(tokens.contains("StyleProperty :: Padding"), "{tokens}");
        assert!(!tokens.contains("compile_error"), "{tokens}");
    }

    #[test]
    fn invalid_declarations_are_compile_errors() {
        let tokens = expand(quote!("colr: red; display: sideways"), None).to_string();
        assert!(
            tokens.contains("unknown Open UI style property `colr`"),
            "{tokens}"
        );
        assert!(tokens.contains("sideways"), "{tokens}");
    }

    #[test]
    fn important_is_rejected_until_rules_land() {
        let tokens = expand(quote!("display: flex !important"), None).to_string();
        assert!(tokens.contains("!important"), "{tokens}");
        assert!(tokens.contains("compile_error"), "{tokens}");
    }

    #[test]
    fn an_empty_block_is_an_empty_style() {
        let tokens = expand(TokenStream::new(), None).to_string();
        assert!(tokens.contains("from_iter ([])"), "{tokens}");
    }

    #[test]
    fn macro_bodies_take_any_delimiter() {
        assert_eq!(macro_body("style! { a: b }"), Some(" a: b "));
        assert_eq!(macro_body("style!(a: b)"), Some("a: b"));
        assert_eq!(macro_body("style![a: b]"), Some("a: b"));
        assert_eq!(macro_body("style! { a } { b }"), None);
        assert_eq!(macro_body("style"), None);
    }
}

//! Typed style values as constructor expressions, for macros only.
//!
//! `openui-macros` parses a style literal at compile time with the same
//! `parse_literal` the runtime uses, then asks the value for an expression
//! that rebuilds it. The emitted code calls constructors directly, so nothing
//! is parsed at run time (RFC 0001). This module exists only with the
//! internal `constructor-tokens` feature and is not part of the public API.
//!
//! Floats are emitted as their bit patterns, so every value (including
//! negative zero) rebuilds bit for bit.

use openui_geometry::{Length, LengthType};
pub use openui_style_derive::ConstructorTokens;
pub use proc_macro2::TokenStream;
use proc_macro2::{Ident, Literal, Span};
use quote::quote;

/// A value that no style literal produces, such as an animation sample or a
/// per-document image handle. It has no constructor expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unsupported {
    what: &'static str,
}

impl Unsupported {
    pub const fn new(what: &'static str) -> Self {
        Self { what }
    }

    pub const fn what(&self) -> &'static str {
        self.what
    }
}

impl std::fmt::Display for Unsupported {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} has no constructor expression: no style literal produces it",
            self.what
        )
    }
}

impl std::error::Error for Unsupported {}

/// An expression that rebuilds `self`, with every type named under `root`
/// (for example `::openui::typed_style`).
pub trait ConstructorTokens {
    fn constructor_tokens(&self, root: &TokenStream) -> Result<TokenStream, Unsupported>;
}

/// `root::A::B`.
pub fn path(root: &TokenStream, segments: &[&str]) -> TokenStream {
    let segments = segments
        .iter()
        .map(|segment| Ident::new(segment, Span::call_site()));
    quote!(#root #(:: #segments)*)
}

/// `callee(arguments, ...)`.
pub fn call(callee: TokenStream, arguments: Vec<TokenStream>) -> TokenStream {
    quote!(#callee(#(#arguments),*))
}

/// `path { label: value, ... }`.
pub fn braced(path: TokenStream, fields: Vec<(&str, TokenStream)>) -> TokenStream {
    let fields = fields.into_iter().map(|(label, value)| {
        let label = Ident::new(label, Span::call_site());
        quote!(#label: #value)
    });
    quote!(#path { #(#fields),* })
}

impl ConstructorTokens for f32 {
    fn constructor_tokens(&self, _root: &TokenStream) -> Result<TokenStream, Unsupported> {
        let bits = Literal::u32_suffixed(self.to_bits());
        Ok(quote!(::core::primitive::f32::from_bits(#bits)))
    }
}

impl ConstructorTokens for f64 {
    fn constructor_tokens(&self, _root: &TokenStream) -> Result<TokenStream, Unsupported> {
        let bits = Literal::u64_suffixed(self.to_bits());
        Ok(quote!(::core::primitive::f64::from_bits(#bits)))
    }
}

macro_rules! integer_tokens {
    ($($ty:ty => $literal:ident),* $(,)?) => {$(
        impl ConstructorTokens for $ty {
            fn constructor_tokens(&self, _root: &TokenStream) -> Result<TokenStream, Unsupported> {
                let literal = Literal::$literal(*self);
                // Parenthesized so a negative literal is one operand anywhere.
                Ok(quote!((#literal)))
            }
        }
    )*};
}

integer_tokens! {
    u8 => u8_suffixed,
    u16 => u16_suffixed,
    u32 => u32_suffixed,
    u64 => u64_suffixed,
    usize => usize_suffixed,
    i8 => i8_suffixed,
    i16 => i16_suffixed,
    i32 => i32_suffixed,
    i64 => i64_suffixed,
    isize => isize_suffixed,
}

impl ConstructorTokens for bool {
    fn constructor_tokens(&self, _root: &TokenStream) -> Result<TokenStream, Unsupported> {
        Ok(if *self { quote!(true) } else { quote!(false) })
    }
}

impl ConstructorTokens for char {
    fn constructor_tokens(&self, _root: &TokenStream) -> Result<TokenStream, Unsupported> {
        let literal = Literal::character(*self);
        Ok(quote!(#literal))
    }
}

impl ConstructorTokens for String {
    fn constructor_tokens(&self, _root: &TokenStream) -> Result<TokenStream, Unsupported> {
        let literal = Literal::string(self);
        Ok(quote!(::std::string::String::from(#literal)))
    }
}

impl<T: ConstructorTokens> ConstructorTokens for Option<T> {
    fn constructor_tokens(&self, root: &TokenStream) -> Result<TokenStream, Unsupported> {
        Ok(match self {
            Some(value) => {
                let value = value.constructor_tokens(root)?;
                quote!(::core::option::Option::Some(#value))
            }
            None => quote!(::core::option::Option::None),
        })
    }
}

impl<T: ConstructorTokens> ConstructorTokens for Box<T> {
    fn constructor_tokens(&self, root: &TokenStream) -> Result<TokenStream, Unsupported> {
        let value = (**self).constructor_tokens(root)?;
        Ok(quote!(::std::boxed::Box::new(#value)))
    }
}

impl<T: ConstructorTokens> ConstructorTokens for Vec<T> {
    fn constructor_tokens(&self, root: &TokenStream) -> Result<TokenStream, Unsupported> {
        let items = self
            .iter()
            .map(|item| item.constructor_tokens(root))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(quote!(::std::vec![#(#items),*]))
    }
}

impl<T: ConstructorTokens, const N: usize> ConstructorTokens for [T; N] {
    fn constructor_tokens(&self, root: &TokenStream) -> Result<TokenStream, Unsupported> {
        let items = self
            .iter()
            .map(|item| item.constructor_tokens(root))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(quote!([#(#items),*]))
    }
}

macro_rules! tuple_tokens {
    ($(($($name:ident),+)),* $(,)?) => {$(
        impl<$($name: ConstructorTokens),+> ConstructorTokens for ($($name,)+) {
            #[allow(non_snake_case)]
            fn constructor_tokens(&self, root: &TokenStream) -> Result<TokenStream, Unsupported> {
                let ($($name,)+) = self;
                $(let $name = $name.constructor_tokens(root)?;)+
                Ok(quote!(($(#$name,)+)))
            }
        }
    )*};
}

tuple_tokens! {
    (A, B),
    (A, B, C),
}

/// `Length` keeps its fields private, so it is rebuilt through its `const fn`
/// constructors. The rebuilt value is checked bit for bit: a payload that no
/// constructor produces is `Unsupported` rather than silently changed.
impl ConstructorTokens for Length {
    fn constructor_tokens(&self, root: &TokenStream) -> Result<TokenStream, Unsupported> {
        let value = self.value();
        let offset = self.calc_offset();
        let constructor = |name: &str, arguments: &[f32]| {
            let arguments = arguments
                .iter()
                .map(|argument| argument.constructor_tokens(root))
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, Unsupported>(call(path(root, &["Length", name]), arguments))
        };
        let (rebuilt, tokens) = match self.length_type() {
            LengthType::Auto => (Length::auto(), constructor("auto", &[])?),
            LengthType::Percent => (Length::percent(value), constructor("percent", &[value])?),
            LengthType::Fixed => (Length::px(value), constructor("px", &[value])?),
            LengthType::MinContent => (Length::min_content(), constructor("min_content", &[])?),
            LengthType::MaxContent => (Length::max_content(), constructor("max_content", &[])?),
            LengthType::Stretch => (Length::stretch(), constructor("stretch", &[])?),
            LengthType::FitContent if offset.to_bits() != 0 => (
                Length::fit_content_px(offset),
                constructor("fit_content_px", &[offset])?,
            ),
            LengthType::FitContent if value.to_bits() != 0 => (
                Length::fit_content_percent(value),
                constructor("fit_content_percent", &[value])?,
            ),
            LengthType::FitContent => (Length::fit_content(), constructor("fit_content", &[])?),
            LengthType::Calculated => (
                Length::calc_percent_px(value, offset),
                constructor("calc_percent_px", &[value, offset])?,
            ),
            LengthType::Flex => (Length::flex(value), constructor("flex", &[value])?),
            LengthType::None => (Length::none(), constructor("none", &[])?),
            LengthType::Content => (Length::content(), constructor("content", &[])?),
            LengthType::MinIntrinsic => return Err(Unsupported::new("Length::MinIntrinsic")),
        };
        let same = rebuilt.length_type() == self.length_type()
            && rebuilt.value().to_bits() == value.to_bits()
            && rebuilt.calc_offset().to_bits() == offset.to_bits();
        if same {
            Ok(tokens)
        } else {
            Err(Unsupported::new("a Length payload no constructor builds"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> TokenStream {
        quote!(::openui_style)
    }

    #[test]
    fn floats_keep_their_bits() {
        let tokens = (-0.0_f32).constructor_tokens(&root()).unwrap().to_string();
        assert!(tokens.contains("2147483648u32"), "{tokens}");
        let tokens = f64::NAN.constructor_tokens(&root()).unwrap().to_string();
        assert!(tokens.contains("f64 :: from_bits"), "{tokens}");
    }

    #[test]
    fn lengths_rebuild_through_their_constructors() {
        let root = root();
        let cases = [
            (Length::px(12.5), "Length :: px"),
            (Length::percent(50.0), "Length :: percent"),
            (
                Length::calc_percent_px(50.0, -10.0),
                "Length :: calc_percent_px",
            ),
            (Length::fit_content_px(40.0), "Length :: fit_content_px"),
            (
                Length::fit_content_percent(25.0),
                "Length :: fit_content_percent",
            ),
            (Length::fit_content(), "Length :: fit_content"),
            (Length::auto(), "Length :: auto"),
            (Length::none(), "Length :: none"),
        ];
        for (length, expected) in cases {
            let tokens = length.constructor_tokens(&root).unwrap().to_string();
            assert!(tokens.contains(expected), "{tokens}");
        }
    }

    #[test]
    fn animation_only_values_have_no_constructor() {
        let value = crate::StyleValue::ColorExpression(
            crate::AnimationColorExpression::from_color(crate::Color::RED),
        );
        assert_eq!(
            value.constructor_tokens(&root()).unwrap_err().what(),
            "StyleValue::ColorExpression"
        );
    }
}

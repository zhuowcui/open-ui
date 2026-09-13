//! Parser and code generator for the `view!` macro.
//!
//! Parses a JSX-like syntax tree and emits Rust code that constructs
//! DOM elements via the `openui` crate's API.

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{braced, Expr, Ident, LitStr, Token};

fn property(prop: &str, span: Span) -> Result<openui_style::StyleProperty, TokenStream> {
    openui_style::StyleProperty::from_css_name(prop).ok_or_else(|| {
        syn::Error::new(span, format!("unknown Open UI style property `{prop}`")).to_compile_error()
    })
}

fn enum_tokens<T: std::fmt::Debug>(prefix: TokenStream, value: T) -> TokenStream {
    let variant = format_ident!("{}", format!("{value:?}"));
    quote! { #prefix::#variant }
}

fn length_tokens(value: openui_style::LengthValue) -> TokenStream {
    use openui_style::LengthValue;
    match value {
        LengthValue::Computed(value) if value.is_auto() => {
            quote! { ::openui::typed_style::LengthValue::auto() }
        }
        LengthValue::Computed(value) if value.is_none() => {
            quote! { ::openui::typed_style::LengthValue::none() }
        }
        LengthValue::Computed(value) if value.is_percent() => {
            let v = value.value();
            quote! { ::openui::typed_style::LengthValue::percent(#v) }
        }
        LengthValue::Computed(value) => {
            let v = value.value();
            quote! { ::openui::typed_style::LengthValue::px(#v) }
        }
        LengthValue::Em(v) => quote! { ::openui::typed_style::LengthValue::Em(#v) },
        LengthValue::Rem(v) => quote! { ::openui::typed_style::LengthValue::Rem(#v) },
        LengthValue::ViewportWidth(v) => {
            quote! { ::openui::typed_style::LengthValue::ViewportWidth(#v) }
        }
        LengthValue::ViewportHeight(v) => {
            quote! { ::openui::typed_style::LengthValue::ViewportHeight(#v) }
        }
    }
}

fn color_tokens(value: openui_style::Color) -> TokenStream {
    let (r, g, b, a) = (value.r, value.g, value.b, value.a);
    quote! { ::openui::typed_style::Color::from_rgba_f32(#r, #g, #b, #a) }
}

fn edges_tokens(value: openui_style::Edges<openui_style::LengthValue>) -> TokenStream {
    let top = length_tokens(value.top);
    let right = length_tokens(value.right);
    let bottom = length_tokens(value.bottom);
    let left = length_tokens(value.left);
    quote! { ::openui::typed_style::Edges { top: #top, right: #right, bottom: #bottom, left: #left } }
}

fn style_value_tokens(value: openui_style::StyleValue) -> TokenStream {
    use openui_style::StyleValue;
    match value {
        StyleValue::Display(v) => {
            let v = enum_tokens(quote!(::openui::typed_style::Display), v);
            quote!(::openui::typed_style::StyleValue::Display(#v))
        }
        StyleValue::Position(v) => {
            let v = enum_tokens(quote!(::openui::typed_style::Position), v);
            quote!(::openui::typed_style::StyleValue::Position(#v))
        }
        StyleValue::Overflow(v) => {
            let v = enum_tokens(quote!(::openui::typed_style::Overflow), v);
            quote!(::openui::typed_style::StyleValue::Overflow(#v))
        }
        StyleValue::Length(v) => {
            let v = length_tokens(v);
            quote!(::openui::typed_style::StyleValue::Length(#v))
        }
        StyleValue::Edges(v) => {
            let v = edges_tokens(v);
            quote!(::openui::typed_style::StyleValue::Edges(#v))
        }
        StyleValue::Color(v) => {
            let v = color_tokens(v);
            quote!(::openui::typed_style::StyleValue::Color(#v))
        }
        StyleValue::Number(v) => quote!(::openui::typed_style::StyleValue::Number(#v)),
        StyleValue::Integer(v) => quote!(::openui::typed_style::StyleValue::Integer(#v)),
        StyleValue::FlexDirection(v) => {
            let v = enum_tokens(quote!(::openui::typed_style::FlexDirection), v);
            quote!(::openui::typed_style::StyleValue::FlexDirection(#v))
        }
        StyleValue::FlexWrap(v) => {
            let v = enum_tokens(quote!(::openui::typed_style::FlexWrap), v);
            quote!(::openui::typed_style::StyleValue::FlexWrap(#v))
        }
        StyleValue::ItemAlignment(v) => {
            let position = enum_tokens(quote!(::openui::typed_style::ItemPosition), v.position);
            let overflow =
                enum_tokens(quote!(::openui::typed_style::OverflowAlignment), v.overflow);
            quote!(::openui::typed_style::StyleValue::ItemAlignment(::openui::typed_style::ItemAlignment { position: #position, overflow: #overflow }))
        }
        StyleValue::ContentAlignment(v) => {
            let position = enum_tokens(quote!(::openui::typed_style::ContentPosition), v.position);
            let distribution = enum_tokens(
                quote!(::openui::typed_style::ContentDistribution),
                v.distribution,
            );
            let overflow =
                enum_tokens(quote!(::openui::typed_style::OverflowAlignment), v.overflow);
            quote!(::openui::typed_style::StyleValue::ContentAlignment(::openui::typed_style::ContentAlignment { position: #position, distribution: #distribution, overflow: #overflow }))
        }
        StyleValue::Gap(v) => {
            let row = length_tokens(v.row);
            let column = length_tokens(v.column);
            quote!(::openui::typed_style::StyleValue::Gap(::openui::typed_style::Gap { row: #row, column: #column }))
        }
        StyleValue::FontFamily(v) => {
            use openui_style::FontFamily;
            let families: Vec<_> = v.families.into_iter().map(|family| match family {
                FontFamily::Named(name) => quote!(::openui::typed_style::FontFamily::Named(::std::string::String::from(#name))),
                FontFamily::Generic(v) => { let v=enum_tokens(quote!(::openui::typed_style::GenericFontFamily),v); quote!(::openui::typed_style::FontFamily::Generic(#v)) }
            }).collect();
            quote!(::openui::typed_style::StyleValue::FontFamily(
                ::openui::typed_style::FontFamilyList {
                    families: ::std::vec![#(#families),*]
                }
            ))
        }
        StyleValue::FontWeight(v) => {
            let v = v.0;
            quote!(::openui::typed_style::StyleValue::FontWeight(::openui::typed_style::FontWeight(#v)))
        }
        StyleValue::Border(v) => {
            let width = v.width;
            let style = enum_tokens(quote!(::openui::typed_style::BorderStyle), v.style);
            let color = color_tokens(v.color);
            quote!(::openui::typed_style::StyleValue::Border(::openui::typed_style::Border { width: #width, style: #style, color: #color }))
        }
        StyleValue::CornerRadii(v) => {
            let edges = edges_tokens(v.0);
            quote!(::openui::typed_style::StyleValue::CornerRadii(::openui::typed_style::CornerRadii(#edges)))
        }
        StyleValue::Cursor(v) => {
            let v = enum_tokens(quote!(::openui::typed_style::Cursor), v);
            quote!(::openui::typed_style::StyleValue::Cursor(#v))
        }
        StyleValue::ListStyle(v) => {
            let v = enum_tokens(quote!(::openui::typed_style::ListStyleType), v);
            quote!(::openui::typed_style::StyleValue::ListStyle(#v))
        }
        StyleValue::Transform(_) => quote!(::openui::typed_style::StyleValue::Transform(
            ::openui::typed_style::TransformList::default()
        )),
    }
}

fn property_tokens(value: openui_style::StyleProperty) -> TokenStream {
    let variant = format_ident!("{}", format!("{value:?}"));
    quote! { ::openui::typed_style::StyleProperty::#variant }
}

fn dynamic_type_tokens(value: openui_style::StyleProperty) -> TokenStream {
    match value.metadata().rust_type {
        "f32" => quote!(f32),
        "i32" => quote!(i32),
        "Edges<LengthValue>" => {
            quote!(::openui::typed_style::Edges<::openui::typed_style::LengthValue>)
        }
        name => {
            let ty = format_ident!("{name}");
            quote!(::openui::typed_style::#ty)
        }
    }
}

// ─── AST ────────────────────────────────────────────────────────────

/// The full body of a `view! { ... }` invocation.
#[cfg_attr(test, derive(Debug))]
struct ViewBody {
    nodes: Vec<ViewNode>,
}

/// A single node in the view tree.
#[cfg_attr(test, derive(Debug))]
enum ViewNode {
    /// `<tag attrs...>children</tag>` or `<tag attrs... />`
    Element {
        tag: Ident,
        attrs: Vec<Attr>,
        children: Vec<ViewNode>,
        is_component: bool,
    },
    /// `"literal text"`
    Text(LitStr),
    /// `{expression}`
    Dynamic { expr: Expr, span: Span },
}

/// An attribute on an element.
#[cfg_attr(test, derive(Debug))]
enum Attr {
    /// `name="value"` — static string attribute.
    Static {
        name: String,
        value: LitStr,
        span: Span,
    },
    /// `on:eventname=handler_expr` — event handler.
    Event {
        name: String,
        handler: Expr,
        span: Span,
    },
    /// `style:prop="value"` — static inline style.
    StyleStatic {
        prop: String,
        value: LitStr,
        span: Span,
    },
    /// `style:prop=expr` — dynamic inline style (expression is a closure).
    StyleDynamic {
        prop: String,
        expr: Expr,
        span: Span,
    },
    /// `name` (no value) — boolean attribute.
    Bool { name: String, span: Span },
    /// `name=expr` — expression attribute or component prop.
    Dynamic {
        name: String,
        value: Expr,
        span: Span,
    },
}

// ─── Parsing ────────────────────────────────────────────────────────

impl Parse for ViewBody {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut nodes = Vec::new();
        while !input.is_empty() {
            nodes.push(parse_node(input)?);
        }
        Ok(ViewBody { nodes })
    }
}

/// Parse a single view node: element, text literal, or dynamic expression.
fn parse_node(input: ParseStream) -> syn::Result<ViewNode> {
    if input.peek(Token![<]) {
        parse_element(input)
    } else if input.peek(LitStr) {
        let lit: LitStr = input.parse()?;
        Ok(ViewNode::Text(lit))
    } else if input.peek(syn::token::Brace) {
        let content;
        let brace = braced!(content in input);
        let expr: Expr = content.parse()?;
        Ok(ViewNode::Dynamic {
            expr,
            span: brace.span.join(),
        })
    } else {
        Err(input.error("expected `<`, a string literal, or `{expression}`"))
    }
}

/// Parse an element: `<tag attrs...>children</tag>` or `<tag attrs... />`.
fn parse_element(input: ParseStream) -> syn::Result<ViewNode> {
    let _lt: Token![<] = input.parse()?;

    // Tag name (use parse_any to support keywords like `type` used as custom tags)
    let tag: Ident = input.call(Ident::parse_any)?;
    let tag_str = tag.to_string();
    let is_component = tag_str.chars().next().map_or(false, |c| c.is_uppercase());

    // Attributes
    let mut attrs = Vec::new();
    loop {
        if input.peek(Token![>]) {
            break;
        }
        if input.peek(Token![/]) && input.peek2(Token![>]) {
            break;
        }
        if input.is_empty() {
            return Err(syn::Error::new(tag.span(), "unclosed element tag"));
        }
        attrs.push(parse_attr(input)?);
    }

    // Self-closing `/>` or opening `>`
    if input.peek(Token![/]) {
        input.parse::<Token![/]>()?;
        input.parse::<Token![>]>()?;
        return Ok(ViewNode::Element {
            tag,
            attrs,
            children: Vec::new(),
            is_component,
        });
    }

    // `>` — start children
    input.parse::<Token![>]>()?;

    // Parse children until `</tag>`
    let mut children = Vec::new();
    loop {
        if input.peek(Token![<]) && input.peek2(Token![/]) {
            break;
        }
        if input.is_empty() {
            return Err(syn::Error::new(
                tag.span(),
                format!("unclosed element `<{}>` — expected `</{}>`", tag, tag),
            ));
        }
        children.push(parse_node(input)?);
    }

    // Closing tag `</tag>`
    input.parse::<Token![<]>()?;
    input.parse::<Token![/]>()?;
    let close_tag: Ident = input.call(Ident::parse_any)?;
    if close_tag != tag {
        return Err(syn::Error::new(
            close_tag.span(),
            format!(
                "mismatched closing tag: expected `</{}>`, found `</{}>`",
                tag, close_tag
            ),
        ));
    }
    input.parse::<Token![>]>()?;

    Ok(ViewNode::Element {
        tag,
        attrs,
        children,
        is_component,
    })
}

/// Parse a single attribute.
fn parse_attr(input: ParseStream) -> syn::Result<Attr> {
    let span = input.span();

    // Attribute name — may be a keyword (e.g. `type`, `for`)
    let first: Ident = input.call(Ident::parse_any)?;
    let first_str = first.to_string();

    // ── on:eventname ────────────────────────────────────────
    if first_str == "on" && input.peek(Token![:]) {
        input.parse::<Token![:]>()?;
        let event_name: Ident = input.call(Ident::parse_any)?;
        input.parse::<Token![=]>()?;
        let handler = parse_attr_value_expr(input)?;
        return Ok(Attr::Event {
            name: event_name.to_string(),
            handler,
            span,
        });
    }

    // ── style:prop-name ─────────────────────────────────────
    if first_str == "style" && input.peek(Token![:]) {
        input.parse::<Token![:]>()?;
        let prop_str = parse_dash_name(input)?;
        input.parse::<Token![=]>()?;

        if input.peek(LitStr) {
            let value: LitStr = input.parse()?;
            return Ok(Attr::StyleStatic {
                prop: prop_str,
                value,
                span,
            });
        }
        let expr = parse_attr_value_expr(input)?;
        return Ok(Attr::StyleDynamic {
            prop: prop_str,
            expr,
            span,
        });
    }

    // ── Regular attribute (possibly dash-separated) ─────────
    let full_name = if input.peek(Token![-]) && !input.peek(Token![>]) {
        let mut name = first_str;
        while input.peek(Token![-]) && !input.peek2(Token![>]) {
            input.parse::<Token![-]>()?;
            let part: Ident = input.call(Ident::parse_any)?;
            name.push('-');
            name.push_str(&part.to_string());
        }
        name
    } else {
        first_str
    };

    // Has a value?
    if input.peek(Token![=]) {
        input.parse::<Token![=]>()?;
        if input.peek(LitStr) {
            let value: LitStr = input.parse()?;
            return Ok(Attr::Static {
                name: full_name,
                value,
                span,
            });
        }
        let expr = parse_attr_value_expr(input)?;
        return Ok(Attr::Dynamic {
            name: full_name,
            value: expr,
            span,
        });
    }

    // Boolean attribute (no value)
    Ok(Attr::Bool {
        name: full_name,
        span,
    })
}

/// Parse a dash-separated name like `background-color`.
fn parse_dash_name(input: ParseStream) -> syn::Result<String> {
    let first: Ident = input.call(Ident::parse_any)?;
    let mut name = first.to_string();
    while input.peek(Token![-]) && !input.peek2(Token![>]) {
        input.parse::<Token![-]>()?;
        let part: Ident = input.call(Ident::parse_any)?;
        name.push('-');
        name.push_str(&part.to_string());
    }
    Ok(name)
}

/// Parse an attribute value expression.
///
/// Handles three forms:
/// 1. `{expr}` — braced expression (unambiguous)
/// 2. `"string"` — string literal (caller usually handles this first)
/// 3. Bare tokens — collected until a boundary (`>`, `/>`, or next attribute)
///
/// Bare `>` in expressions must be wrapped in braces: `attr={a > b}`.
fn parse_attr_value_expr(input: ParseStream) -> syn::Result<Expr> {
    // Braced expression — parse contents directly
    if input.peek(syn::token::Brace) {
        let content;
        braced!(content in input);
        return content.parse();
    }

    // Collect tokens until boundary
    let mut tokens = TokenStream::new();
    let start_span = input.span();
    let mut last_joint = false;

    while !input.is_empty() {
        // Stop at `>` (unless part of a compound like `->`)
        if input.peek(Token![>]) && !last_joint {
            break;
        }

        // Stop at `/>` (unless part of compound)
        if input.peek(Token![/]) && !last_joint {
            let fork = input.fork();
            let _ = fork.parse::<Token![/]>();
            if fork.peek(Token![>]) {
                break;
            }
        }

        // Stop at the start of a new attribute
        if !tokens.is_empty() && !last_joint && is_attr_boundary(input) {
            break;
        }

        // Consume one token tree
        let tt: TokenTree = input.parse()?;
        last_joint = matches!(
            &tt,
            TokenTree::Punct(p) if p.spacing() == proc_macro2::Spacing::Joint
        );
        tokens.extend(std::iter::once(tt));
    }

    if tokens.is_empty() {
        return Err(syn::Error::new(start_span, "expected attribute value"));
    }

    syn::parse2(tokens)
}

/// Check whether the next tokens look like the start of a new attribute
/// (an identifier followed by `=` or `:`).
fn is_attr_boundary(input: ParseStream) -> bool {
    if !input.peek(Ident::peek_any) {
        return false;
    }
    let fork = input.fork();
    if fork.call(Ident::parse_any).is_err() {
        return false;
    }

    // ident followed by `=` or `:`
    if fork.peek(Token![=]) || fork.peek(Token![:]) {
        return true;
    }

    // Dashed name followed by `=` (e.g. `data-id=...`)
    if fork.peek(Token![-]) {
        while fork.peek(Token![-]) {
            let _ = fork.parse::<Token![-]>();
            if fork.call(Ident::parse_any).is_err() {
                return false;
            }
        }
        if fork.peek(Token![=]) {
            return true;
        }
    }

    false
}

// ─── Code generation ────────────────────────────────────────────────

/// Entry point: parse and generate code for `view! { ... }`.
pub(crate) fn view_macro(input: TokenStream) -> TokenStream {
    match syn::parse2::<ViewBody>(input) {
        Ok(body) => generate(&body),
        Err(err) => err.to_compile_error(),
    }
}

/// Generate the output token stream for a parsed view body.
fn generate(body: &ViewBody) -> TokenStream {
    match body.nodes.len() {
        0 => quote! { ::openui::ViewNode::Empty },
        1 => gen_view_node(&body.nodes[0]),
        _ => {
            let items: Vec<_> = body.nodes.iter().map(gen_view_node).collect();
            quote! {
                ::openui::ViewNode::Fragment(::std::vec![#(#items),*])
            }
        }
    }
}

/// Generate code for a single top-level view node.
fn gen_view_node(node: &ViewNode) -> TokenStream {
    match node {
        ViewNode::Element {
            is_component: false,
            tag,
            attrs,
            children,
        } => gen_html_element(tag, attrs, children),
        ViewNode::Element {
            is_component: true,
            tag,
            attrs,
            children,
        } => gen_component_node(tag, attrs, children),
        ViewNode::Text(lit) => {
            let val = lit.value();
            quote_spanned! {lit.span()=>
                ::openui::ViewNode::Text(::std::string::String::from(#val))
            }
        }
        ViewNode::Dynamic { expr, span } => gen_dynamic_view_node(expr, *span),
    }
}

/// Generate code for an HTML element that returns `ViewNode::Element(el)`.
fn gen_html_element(tag: &Ident, attrs: &[Attr], children: &[ViewNode]) -> TokenStream {
    let tag_str = tag.to_string();
    let span = tag.span();

    let attr_stmts: Vec<_> = attrs.iter().map(gen_attr_stmt).collect();
    let child_stmts: Vec<_> = children.iter().map(gen_child_stmt).collect();

    quote_spanned! {span=> {
        let __doc = ::openui::current_document();
        #[allow(unused_variables)]
        let __el = ::openui::Element::create(__doc, #tag_str)
            .expect("failed to create element");
        #(#attr_stmts)*
        #(#child_stmts)*
        ::openui::ViewNode::Element(__el)
    }}
}

/// Generate code for a dynamic view node (reactive text).
///
/// Returns a `ViewNode::MountFn` that creates a DOM Text node on the parent
/// during mount. This avoids wrapper elements like `<span>`.
fn gen_dynamic_view_node(expr: &Expr, span: Span) -> TokenStream {
    quote_spanned! {span=> {
        ::openui::ViewNode::MountFn(Box::new(move |__el: &::openui::Element| {
            let __text_node = __el.create_text_child(
                &::std::string::ToString::to_string(&(#expr))
            );
            let __text_raw = __text_node.as_raw();
            ::openui::on_cleanup(move || { drop(__text_node); });
            ::openui::create_effect({
                let __text_raw = __text_raw;
                move || {
                    let __node_ref = unsafe {
                        ::openui::TextNode::from_raw_borrowed(__text_raw)
                    };
                    let __val = ::std::string::ToString::to_string(&(#expr));
                    __node_ref.set_data(&__val);
                }
            });
        }))
    }}
}

/// Generate an attribute-setting statement (runs in scope where `__el` exists).
fn gen_attr_stmt(attr: &Attr) -> TokenStream {
    match attr {
        Attr::Static { name, value, span } => {
            quote_spanned! {*span=>
                __el.set_attribute(#name, #value).expect("set attribute");
            }
        }
        Attr::Event {
            name,
            handler,
            span,
        } => {
            quote_spanned! {*span=>
                __el.on(#name, #handler).expect("set event handler");
            }
        }
        Attr::StyleStatic { prop, value, span } => {
            let property = match property(prop, *span) {
                Ok(value) => value,
                Err(error) => return error,
            };
            let parsed = match openui_style::parse_literal(property, &value.value()) {
                Ok(value) => value,
                Err(error) => {
                    return syn::Error::new(value.span(), error.to_string()).to_compile_error()
                }
            };
            let property = property_tokens(property);
            let value = style_value_tokens(parsed);
            quote_spanned! {*span=> __el.set_property(#property, #value).expect("set typed style"); }
        }
        Attr::StyleDynamic { prop, expr, span } => {
            let property = match property(prop, *span) {
                Ok(value) => value,
                Err(error) => return error,
            };
            let property = property_tokens(property);
            let expected =
                dynamic_type_tokens(match openui_style::StyleProperty::from_css_name(prop) {
                    Some(value) => value,
                    None => unreachable!(),
                });
            quote_spanned! {*span=> {
                let __style_raw = __el.as_raw();
                ::openui::create_effect({
                    let __style_raw = __style_raw;
                    move || {
                        let __el_ref = unsafe {
                            ::openui::Element::from_raw_borrowed(__style_raw)
                        };
                        let __val: #expected = (#expr)();
                        __el_ref.set_property(#property, __val.into()).expect("set typed style");
                    }
                });
            }}
        }
        Attr::Bool { name, span } => {
            quote_spanned! {*span=>
                __el.set_attribute(#name, "").expect("set attribute");
            }
        }
        Attr::Dynamic {
            name, value, span, ..
        } => {
            quote_spanned! {*span=>
                __el.set_attribute(#name, &::std::string::ToString::to_string(&(#value)))
                    .expect("set attribute");
            }
        }
    }
}

/// Generate a child-appending statement (runs in scope where `__el` is parent).
fn gen_child_stmt(child: &ViewNode) -> TokenStream {
    match child {
        ViewNode::Element {
            is_component: false,
            tag,
            attrs,
            children,
        } => gen_child_html_element(tag, attrs, children),
        ViewNode::Element {
            is_component: true,
            tag,
            attrs,
            children,
        } => gen_child_component(tag, attrs, children),
        ViewNode::Text(lit) => {
            let text = lit.value();
            quote_spanned! {lit.span()=>
                __el.append_text_node(#text);
            }
        }
        ViewNode::Dynamic { expr, span } => {
            quote_spanned! {*span=> {
                let __text_node = __el.create_text_child(
                    &::std::string::ToString::to_string(&(#expr))
                );
                let __text_raw = __text_node.as_raw();
                // Transfer ownership to scope cleanup so the text node is
                // removed from the DOM when the enclosing scope is disposed.
                ::openui::on_cleanup(move || { drop(__text_node); });
                ::openui::create_effect({
                    let __text_raw = __text_raw;
                    move || {
                        let __node_ref = unsafe {
                            ::openui::TextNode::from_raw_borrowed(__text_raw)
                        };
                        let __val = ::std::string::ToString::to_string(&(#expr));
                        __node_ref.set_data(&__val);
                    }
                });
            }}
        }
    }
}

/// Generate a child HTML element: create it in its own scope, then append.
fn gen_child_html_element(tag: &Ident, attrs: &[Attr], children: &[ViewNode]) -> TokenStream {
    let tag_str = tag.to_string();
    let span = tag.span();

    let attr_stmts: Vec<_> = attrs.iter().map(gen_attr_stmt).collect();
    let child_stmts: Vec<_> = children.iter().map(gen_child_stmt).collect();

    // Inner block defines its own `__el`; outer block appends to parent `__el`.
    quote_spanned! {span=> {
        let __child = {
            let __doc = ::openui::current_document();
            #[allow(unused_variables)]
            let __el = ::openui::Element::create(__doc, #tag_str)
                .expect("failed to create element");
            #(#attr_stmts)*
            #(#child_stmts)*
            __el
        };
        __el.append_child(&__child);
        ::std::mem::forget(__child);
    }}
}

/// Generate a component call for a child position.
fn gen_child_component(tag: &Ident, attrs: &[Attr], children: &[ViewNode]) -> TokenStream {
    let view_expr = gen_component_call_expr(tag, attrs, children);
    quote! {{
        let __child_view = ::openui::IntoView::into_view(#view_expr);
        ::openui::mount_view(&__el, __child_view);
    }}
}

/// Generate a component invocation that returns `ViewNode`.
fn gen_component_node(tag: &Ident, attrs: &[Attr], children: &[ViewNode]) -> TokenStream {
    let call = gen_component_call_expr(tag, attrs, children);
    quote! {
        ::openui::IntoView::into_view(#call)
    }
}

/// Generate the actual component function call expression.
fn gen_component_call_expr(tag: &Ident, attrs: &[Attr], children: &[ViewNode]) -> TokenStream {
    let props_type = format_ident!("{}Props", tag);

    let fields: Vec<_> = attrs
        .iter()
        .filter_map(|attr| match attr {
            Attr::Dynamic { name, value, .. } => {
                let field = format_ident!("{}", name);
                Some(quote! { #field: #value })
            }
            Attr::Static { name, value, .. } => {
                let field = format_ident!("{}", name);
                Some(quote! { #field: ::std::string::ToString::to_string(&#value) })
            }
            _ => None,
        })
        .collect();

    // If there are children, collect them into a `children` field
    let children_field = if children.is_empty() {
        None
    } else {
        let child_exprs: Vec<_> = children.iter().map(gen_view_node).collect();
        Some(quote! { children: ::std::vec![#(#child_exprs),*] })
    };

    let all_fields: Vec<_> = fields.into_iter().chain(children_field).collect();

    quote! {
        #tag(#props_type { #(#all_fields),* })
    }
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    /// Helper: parse a token stream as a ViewBody and assert success.
    fn parse_ok(input: TokenStream) -> ViewBody {
        syn::parse2::<ViewBody>(input).expect("failed to parse view body")
    }

    #[test]
    fn parse_self_closing_element() {
        let body = parse_ok(quote! { <br /> });
        assert_eq!(body.nodes.len(), 1);
        match &body.nodes[0] {
            ViewNode::Element {
                tag,
                children,
                is_component,
                ..
            } => {
                assert_eq!(tag.to_string(), "br");
                assert!(children.is_empty());
                assert!(!is_component);
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn parse_element_with_children() {
        let body = parse_ok(quote! { <div><span>"hello"</span></div> });
        assert_eq!(body.nodes.len(), 1);
        match &body.nodes[0] {
            ViewNode::Element { tag, children, .. } => {
                assert_eq!(tag.to_string(), "div");
                assert_eq!(children.len(), 1);
                match &children[0] {
                    ViewNode::Element { tag, children, .. } => {
                        assert_eq!(tag.to_string(), "span");
                        assert_eq!(children.len(), 1);
                        assert!(matches!(&children[0], ViewNode::Text(_)));
                    }
                    _ => panic!("expected child Element"),
                }
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn parse_static_attributes() {
        let body = parse_ok(quote! { <div class="foo" id="bar" /> });
        match &body.nodes[0] {
            ViewNode::Element { attrs, .. } => {
                assert_eq!(attrs.len(), 2);
                assert!(matches!(&attrs[0], Attr::Static { name, .. } if name == "class"));
                assert!(matches!(&attrs[1], Attr::Static { name, .. } if name == "id"));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn parse_boolean_attribute() {
        let body = parse_ok(quote! { <input disabled /> });
        match &body.nodes[0] {
            ViewNode::Element { attrs, .. } => {
                assert_eq!(attrs.len(), 1);
                assert!(matches!(&attrs[0], Attr::Bool { name, .. } if name == "disabled"));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn parse_event_handler() {
        let body = parse_ok(quote! { <button on:click=|_| {} /> });
        match &body.nodes[0] {
            ViewNode::Element { attrs, .. } => {
                assert_eq!(attrs.len(), 1);
                assert!(matches!(&attrs[0], Attr::Event { name, .. } if name == "click"));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn parse_style_static() {
        let body = parse_ok(quote! { <div style:width="100px" /> });
        match &body.nodes[0] {
            ViewNode::Element { attrs, .. } => {
                assert_eq!(attrs.len(), 1);
                assert!(matches!(&attrs[0], Attr::StyleStatic { prop, .. } if prop == "width"));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn parse_style_dynamic() {
        let body = parse_ok(quote! { <div style:opacity=move || { "1" } /> });
        match &body.nodes[0] {
            ViewNode::Element { attrs, .. } => {
                assert_eq!(attrs.len(), 1);
                assert!(matches!(&attrs[0], Attr::StyleDynamic { prop, .. } if prop == "opacity"));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn parse_dashed_attribute() {
        let body = parse_ok(quote! { <div data-id="x" /> });
        match &body.nodes[0] {
            ViewNode::Element { attrs, .. } => {
                assert_eq!(attrs.len(), 1);
                assert!(matches!(&attrs[0], Attr::Static { name, .. } if name == "data-id"));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn parse_dashed_style_prop() {
        let body = parse_ok(quote! { <div style:background-color="red" /> });
        match &body.nodes[0] {
            ViewNode::Element { attrs, .. } => {
                assert_eq!(attrs.len(), 1);
                assert!(matches!(
                    &attrs[0],
                    Attr::StyleStatic { prop, .. } if prop == "background-color"
                ));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn parse_dynamic_child() {
        let body = parse_ok(quote! { <p>{some_expr}</p> });
        match &body.nodes[0] {
            ViewNode::Element { children, .. } => {
                assert_eq!(children.len(), 1);
                assert!(matches!(&children[0], ViewNode::Dynamic { .. }));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn parse_mixed_text_and_dynamic() {
        let body = parse_ok(quote! { <p>"Count: " {count}</p> });
        match &body.nodes[0] {
            ViewNode::Element { children, .. } => {
                assert_eq!(children.len(), 2);
                assert!(matches!(&children[0], ViewNode::Text(_)));
                assert!(matches!(&children[1], ViewNode::Dynamic { .. }));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn parse_component() {
        let body = parse_ok(quote! { <Counter initial=42 /> });
        match &body.nodes[0] {
            ViewNode::Element {
                tag,
                is_component,
                attrs,
                ..
            } => {
                assert_eq!(tag.to_string(), "Counter");
                assert!(is_component);
                assert_eq!(attrs.len(), 1);
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn parse_multiple_root_nodes() {
        let body = parse_ok(quote! { <div /><span /> });
        assert_eq!(body.nodes.len(), 2);
    }

    #[test]
    fn parse_empty_element() {
        let body = parse_ok(quote! { <div></div> });
        match &body.nodes[0] {
            ViewNode::Element { children, .. } => {
                assert!(children.is_empty());
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn parse_deeply_nested() {
        let body = parse_ok(quote! {
            <div>
                <span>
                    <em>"deep"</em>
                </span>
            </div>
        });
        match &body.nodes[0] {
            ViewNode::Element { children, .. } => {
                assert_eq!(children.len(), 1);
                match &children[0] {
                    ViewNode::Element { children, .. } => {
                        assert_eq!(children.len(), 1);
                        match &children[0] {
                            ViewNode::Element { tag, children, .. } => {
                                assert_eq!(tag.to_string(), "em");
                                assert_eq!(children.len(), 1);
                            }
                            _ => panic!("expected em element"),
                        }
                    }
                    _ => panic!("expected span element"),
                }
            }
            _ => panic!("expected div element"),
        }
    }

    #[test]
    fn parse_multiple_attrs() {
        let body = parse_ok(quote! {
            <button on:click=|_| {} class="btn" disabled style:color="red" />
        });
        match &body.nodes[0] {
            ViewNode::Element { attrs, .. } => {
                assert_eq!(attrs.len(), 4);
                assert!(matches!(&attrs[0], Attr::Event { name, .. } if name == "click"));
                assert!(matches!(&attrs[1], Attr::Static { name, .. } if name == "class"));
                assert!(matches!(&attrs[2], Attr::Bool { name, .. } if name == "disabled"));
                assert!(matches!(&attrs[3], Attr::StyleStatic { prop, .. } if prop == "color"));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn parse_component_with_children() {
        let body = parse_ok(quote! {
            <Card>
                <h2>"Title"</h2>
            </Card>
        });
        match &body.nodes[0] {
            ViewNode::Element {
                tag,
                is_component,
                children,
                ..
            } => {
                assert_eq!(tag.to_string(), "Card");
                assert!(is_component);
                assert_eq!(children.len(), 1);
            }
            _ => panic!("expected component element"),
        }
    }

    #[test]
    fn parse_event_handler_complex() {
        // Closure with a block body — should parse correctly
        let body = parse_ok(quote! {
            <button on:click=move |_| { some_action() } />
        });
        match &body.nodes[0] {
            ViewNode::Element { attrs, .. } => {
                assert_eq!(attrs.len(), 1);
                assert!(matches!(&attrs[0], Attr::Event { .. }));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn parse_braced_attr_value() {
        let body = parse_ok(quote! { <div handler={a > b} /> });
        match &body.nodes[0] {
            ViewNode::Element { attrs, .. } => {
                assert_eq!(attrs.len(), 1);
                assert!(matches!(&attrs[0], Attr::Dynamic { name, .. } if name == "handler"));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn codegen_self_closing() {
        let body = parse_ok(quote! { <div /> });
        let output = generate(&body);
        let output_str = output.to_string();
        assert!(
            output_str.contains("create"),
            "expected 'create' in: {}",
            output_str
        );
        assert!(output_str.contains("\"div\""));
    }

    #[test]
    fn codegen_static_attr() {
        let body = parse_ok(quote! { <div class="foo" /> });
        let output = generate(&body);
        let output_str = output.to_string();
        assert!(output_str.contains("set_attribute"));
        assert!(output_str.contains("\"class\""));
        assert!(output_str.contains("\"foo\""));
    }

    #[test]
    fn codegen_event() {
        let body = parse_ok(quote! { <button on:click=|_| {} /> });
        let output = generate(&body);
        let output_str = output.to_string();
        assert!(
            output_str.contains(". on (") || output_str.contains(".on("),
            "expected '.on(' in: {}",
            output_str
        );
        assert!(output_str.contains("\"click\""));
    }

    #[test]
    fn codegen_static_style_is_typed_and_preparsed() {
        let body = parse_ok(quote! { <div style:padding="8px 16px" /> });
        let output = generate(&body).to_string();
        assert!(output.contains("set_property"));
        assert!(output.contains("StyleProperty :: Padding"));
        assert!(output.contains("LengthValue :: px"));
        assert!(!output.contains("set_style"));
    }

    #[test]
    fn codegen_unknown_property_is_compile_error() {
        let body = parse_ok(quote! { <div style:definitely-unknown="10px" /> });
        assert!(generate(&body).to_string().contains("compile_error"));
    }

    #[test]
    fn codegen_invalid_literal_is_compile_error() {
        let body = parse_ok(quote! { <div style:opacity="opaque-ish" /> });
        assert!(generate(&body).to_string().contains("compile_error"));
    }

    #[test]
    fn codegen_text_child() {
        let body = parse_ok(quote! { <p>"hello"</p> });
        let output = generate(&body);
        let output_str = output.to_string();
        assert!(output_str.contains("append_text_node"));
        assert!(output_str.contains("\"hello\""));
    }

    #[test]
    fn codegen_fragment() {
        let body = parse_ok(quote! { <div /><span /> });
        let output = generate(&body);
        let output_str = output.to_string();
        assert!(output_str.contains("Fragment"));
    }

    #[test]
    fn codegen_dynamic_child_uses_text_node() {
        let body = parse_ok(quote! { <p>{val}</p> });
        let output = generate(&body);
        let output_str = output.to_string();
        assert!(
            output_str.contains("create_text_child"),
            "expected 'create_text_child' in: {}",
            output_str
        );
        assert!(
            output_str.contains("set_data"),
            "expected 'set_data' (not set_text) in: {}",
            output_str
        );
        assert!(
            !output_str.contains("create(__doc, \"span\")"),
            "must not create a <span> wrapper for dynamic text: {}",
            output_str
        );
    }

    #[test]
    fn codegen_standalone_dynamic_uses_mount_fn() {
        let body = parse_ok(quote! { {val} });
        let output = generate(&body);
        let output_str = output.to_string();
        assert!(
            output_str.contains("MountFn"),
            "expected 'MountFn' in standalone dynamic: {}",
            output_str
        );
        assert!(
            output_str.contains("create_text_child"),
            "expected 'create_text_child' in: {}",
            output_str
        );
    }

    #[test]
    fn mismatched_close_tag_error() {
        let result = syn::parse2::<ViewBody>(quote! { <div></span> });
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("mismatched"));
    }
}

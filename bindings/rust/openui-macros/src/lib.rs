//! Proc macros for Open UI — JSX-like `view!` and `#[component]` attribute.

use proc_macro::TokenStream;

mod component;
mod style_macro;
mod typed_style_cases;
mod view;

/// JSX-like declarative UI macro.
///
/// Generates Rust code that creates DOM elements, sets attributes and styles,
/// wires event handlers, and builds a reactive UI tree.
///
/// # Syntax
///
/// ```ignore
/// view! {
///     <div class="container">
///         <h1>"Hello!"</h1>
///         <button on:click=move |_| count.set(count.get() + 1)>
///             "Count: " {count}
///         </button>
///     </div>
/// }
/// ```
///
/// ## Supported features
///
/// - Static HTML elements with children and self-closing tags
/// - String literal text nodes: `"text"`
/// - Dynamic text interpolation: `{expression}`
/// - Event handlers: `on:click=expr`
/// - Static styles: `style:width="100px"`
/// - Dynamic typed styles: `style:opacity={signal.get()}`
/// - Standard and boolean HTML attributes
/// - Component instantiation via PascalCase tags
/// - Multiple root nodes (returns a `Fragment`)
///
/// ## Note
///
/// Expressions containing bare `>` (comparison) in attribute values must
/// be wrapped in braces: `attr={a > b}`.
#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    view::view_macro(input.into()).into()
}

/// A CSS declaration block compiled to a typed `openui::Style`.
///
/// Each value is parsed at compile time with the same grammar as `view!`'s
/// `style:` attributes and emitted as the constructor of its typed value, so
/// nothing is parsed at run time. An unknown property or an invalid value is
/// a compile error.
///
/// ```ignore
/// let card = style! { display: flex; justify-content: space-between; padding: 8px 16px };
/// element.apply_style(&card)?;
/// ```
///
/// CSS that Rust cannot split into tokens, such as `#3e3` or a single-quoted
/// string, goes in a string literal: `style!("color: #3e3")`. The braced form
/// reads the macro call's source text, so inside another macro use the
/// string form as well.
#[proc_macro]
pub fn style(input: TokenStream) -> TokenStream {
    let source = proc_macro::Span::call_site().source_text();
    style_macro::expand(input.into(), source).into()
}

/// Component attribute macro.
///
/// Transforms a function into a component by generating a props struct
/// and rewriting the function to accept it.
///
/// # Examples
///
/// ```ignore
/// #[component]
/// fn Counter(initial: i32) -> impl IntoView {
///     let count = create_signal(initial);
///     view! { <p>{count}</p> }
/// }
///
/// // Generates:
/// // pub struct CounterProps { pub initial: i32 }
/// // pub fn Counter(props: CounterProps) -> impl IntoView { ... }
/// ```
///
/// ## Optional props
///
/// ```ignore
/// #[component]
/// fn Card(title: String, #[prop(optional)] subtitle: String) -> impl IntoView {
///     // subtitle is Option<String>, unwrapped with Default
///     // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn component(attr: TokenStream, input: TokenStream) -> TokenStream {
    component::component_macro(attr.into(), input.into()).into()
}

/// Test support, not public API: every property's initial value that
/// `parse_literal` accepts, plus every line of `typed_style_corpus.txt`,
/// parsed at compile time and emitted as
/// `(StyleProperty, literal, constructed StyleValue)` tuples. The tests
/// compare each constructed value with the runtime parse of its literal.
#[doc(hidden)]
#[proc_macro]
pub fn __typed_style_literal_cases(input: TokenStream) -> TokenStream {
    typed_style_cases::expand(input.into()).into()
}

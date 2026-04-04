//! Proc macros for Open UI — JSX-like `view!` and `#[component]` attribute.

use proc_macro::TokenStream;

mod component;
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
/// - Dynamic styles: `style:opacity=move || expr`
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

//! Todo List — demonstrates the `For` component for reactive list rendering.
//!
//! Creates a signal holding a `Vec<String>` of todo items and renders them
//! using the `For` component for keyed list reconciliation. Since we don't
//! have real input binding, the list starts with pre-populated items.

use openui::prelude::*;

fn main() {
    let mut app = App::new(800, 600);

    app.render(|| {
        // Reactive list of todo items.
        let items = create_signal(vec![
            "Learn Rust".to_string(),
            "Build with Open UI".to_string(),
            "Write reactive components".to_string(),
            "Ship to production".to_string(),
            "Celebrate! 🎉".to_string(),
        ]);

        // Build the static page shell with view!, then mount the dynamic
        // list separately via For().
        let page = view! {
            <div
                style:display="flex"
                style:flex-direction="column"
                style:align-items="center"
                style:min-height="100vh"
                style:background-color="#f7fafc"
                style:font-family="sans-serif"
                style:padding="40px"
            >
                <h1 style:font-size="32px" style:color="#2d3748" style:margin-bottom="8px">
                    "Todo List"
                </h1>

                <p style:font-size="16px" style:color="#718096" style:margin-bottom="24px">
                    {format!("{} items", items.get().len())}
                </p>

                // Container for the dynamic list rendered by For().
                <ul style:list-style="none" style:padding="0" style:width="400px"
                    style:margin="0" id="todo-list">
                </ul>
            </div>
        };

        // Render the keyed list into the <ul> container.
        // For() takes: data closure, key extractor, and child renderer.
        let list_view = For(
            move || items.get(),
            |item| item.clone(),
            |item| {
                view! {
                    <li
                        style:padding="12px 16px"
                        style:margin-bottom="8px"
                        style:background-color="white"
                        style:border-radius="8px"
                        style:border="1px solid #e2e8f0"
                        style:font-size="16px"
                        style:color="#2d3748"
                        style:display="flex"
                        style:align-items="center"
                    >
                        <span style:color="#48bb78" style:margin-right="12px"
                              style:font-size="18px">
                            "✓"
                        </span>
                        {item}
                    </li>
                }
            },
        );

        // Mount list into the page after the page is constructed.
        // We need to get the <ul> and mount into it. Since the view is
        // already a ViewNode, we return a Fragment containing both.
        ViewNode::Fragment(vec![page, list_view])
    });

    app.run_frames(1).render_to_png("todo.png");

    println!("Rendered todo.png");
}

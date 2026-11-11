use openui::prelude::*;

// No style at all: every element starts from CSS's initial values.
pub fn example() -> ViewNode {
    view! {
        <div>
            <h1>"A heading"</h1>
            <p>"A paragraph with " <strong>"strong"</strong> " text."</p>
            <ul>
                <li>"A list item"</li>
            </ul>
            <button>"A button"</button>
        </div>
    }
}

use openui::prelude::*;

pub fn example() -> ViewNode {
    let opened = create_signal(0);
    view! {
        <p style:margin="0" style:font-size="15px" style:line-height="1.6" style:width="400px">
            "Div draws " <strong style:font-weight="600">"native windows"</strong>
            ", not web pages. Inline elements such as " <em style:font-style="italic">"em"</em>
            " and " <small style:font-size="12px">"small"</small> " have no style of their own. "
            <a
                style:color="#2563eb"
                style:cursor="pointer"
                on:click={move |_| opened.set(opened.get() + 1)}
            >
                "This link"
            </a>
            " runs a closure; it does not navigate."
        </p>
    }
}

use openui::prelude::*;

pub fn example() -> ViewNode {
    view! {
        <div
            style:display="flex"
            style:align-items="center"
            style:gap="8px"
            style:padding="8px"
            style:border="1px solid #e4e4e7"
            style:border-radius="10px"
            style:font-size="14px"
        >
            <span style:font-weight="600" style:padding="0 8px">"Inbox"</span>
            // There is no `flex` shorthand: use flex-grow and flex-basis.
            <span style:flex-grow="1" style:flex-basis="0px" style:color="#71717a">
                "3 unread"
            </span>
            <span style:padding="4px 10px" style:border-radius="6px" style:background-color="#f4f4f5">
                "Filter"
            </span>
            <span style:padding="4px 10px" style:border-radius="6px" style:background-color="#18181b" style:color="#fafafa">
                "Compose"
            </span>
        </div>
    }
}

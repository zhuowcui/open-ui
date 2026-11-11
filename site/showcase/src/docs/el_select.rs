use openui::prelude::*;

pub fn example() -> ViewNode {
    view! {
        <select
            style:width="200px"
            style:height="34px"
            style:padding="0 10px"
            style:border="1px solid #d4d4d8"
            style:border-radius="8px"
            style:font-size="14px"
            style:overflow="hidden"
        >
            <option value="light">"Light"</option>
            <option value="dark">"Dark"</option>
            <option value="system">"System"</option>
        </select>
    }
}

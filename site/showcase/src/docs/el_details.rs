use openui::prelude::*;

pub fn example() -> ViewNode {
    view! {
        <div style:width="380px" style:font-size="14px">
            <details open style:border-bottom="1px solid #e4e4e7" style:padding="10px 0">
                <summary style:font-weight="500" style:cursor="pointer">
                    "Is Div a browser?"
                </summary>
                <p style:margin="8px 0 0" style:color="#52525b">
                    "No. It lays out and paints your views itself, with no HTML or JavaScript."
                </p>
            </details>
            <details style:border-bottom="1px solid #e4e4e7" style:padding="10px 0">
                <summary style:font-weight="500" style:cursor="pointer">
                    "Which platforms does it support?"
                </summary>
                <p style:margin="8px 0 0">"Hidden until the summary is clicked."</p>
            </details>
        </div>
    }
}

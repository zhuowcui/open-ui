use openui::prelude::*;

pub fn example() -> ViewNode {
    view! {
        <div style:width="340px" style:font-size="14px">
            <label style:display="block" style:font-weight="500" style:margin-bottom="6px">
                "Release notes"
            </label>
            <textarea
                rows="4"
                value="Faster startup. Fixed text selection in tables."
                style:box-sizing="border-box"
                style:width="100%"
                style:border="1px solid #d4d4d8"
                style:border-radius="8px"
                style:font-size="14px"
                style:line-height="1.5"
            ></textarea>
        </div>
    }
}

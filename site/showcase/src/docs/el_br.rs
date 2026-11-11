use openui::prelude::*;

pub fn example() -> ViewNode {
    view! {
        <div style:display="flex" style:gap="32px" style:font-size="14px">
            <p style:margin="0" style:line-height="1.5">
                "Acme Inc." <br /> "12 Market Street" <br /> "Springfield"
            </p>
            // wbr marks where a long word may wrap.
            <p
                style:margin="0"
                style:width="150px"
                style:padding="8px"
                style:border="1px dashed #a1a1aa"
                style:line-height="1.5"
            >
                "releases/" <wbr /> "openui-" <wbr /> "0.2.0-" <wbr /> "windows-" <wbr /> "x86_64.zip"
            </p>
        </div>
    }
}

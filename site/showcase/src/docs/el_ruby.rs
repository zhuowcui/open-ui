use openui::prelude::*;

pub fn example() -> ViewNode {
    // The annotation sits above the base text: leave line height for it.
    view! {
        <p style:margin="0" style:font-size="28px" style:line-height="2">
            <ruby>"東" <rt style:font-size="12px">"とう"</rt></ruby>
            <ruby>"京" <rt style:font-size="12px">"きょう"</rt></ruby>
            " Tokyo"
        </p>
    }
}

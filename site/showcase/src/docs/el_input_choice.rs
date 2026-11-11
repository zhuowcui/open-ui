use openui::prelude::*;

// Checkboxes, radios and ranges have no default size: give them one.
pub fn example() -> ViewNode {
    view! {
        <div style:font-size="14px" style:line-height="1.5">
            <div style:display="flex" style:align-items="center" style:gap="8px">
                <input type="checkbox" checked style:width="16px" style:height="16px" />
                "Email me about new releases"
            </div>
            <div style:display="flex" style:align-items="center" style:gap="8px" style:margin-top="12px">
                <input type="radio" name="plan" checked style:width="16px" style:height="16px" />
                "Monthly"
                <input type="radio" name="plan" style:width="16px" style:height="16px" />
                "Yearly"
            </div>
            <div style:display="flex" style:align-items="center" style:gap="8px" style:margin-top="12px">
                "Volume"
                <input type="range" min="0" max="10" value="7" style:width="160px" style:height="16px" />
            </div>
        </div>
    }
}

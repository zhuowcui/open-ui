use openui::prelude::*;

pub fn example() -> ViewNode {
    view! {
        <div style:width="320px" style:font-size="14px">
            <div style:display="flex" style:justify-content="space-between" style:margin-bottom="6px">
                <span>"Disk"</span>
                <span style:color="#71717a">"62 of 100 GB"</span>
            </div>
            // meter reads value, min and max.
            <meter value="62" min="0" max="100" style:width="320px" style:height="12px"></meter>
            <div style:display="flex" style:justify-content="space-between" style:margin="16px 0 6px">
                <span>"Upload"</span>
                <span style:color="#71717a">"30%"</span>
            </div>
            // progress ignores max: its value is a fraction from 0 to 1.
            <progress value="0.3" style:width="320px" style:height="12px"></progress>
        </div>
    }
}

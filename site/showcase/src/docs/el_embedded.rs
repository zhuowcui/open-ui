use openui::prelude::*;

// These elements lay out as boxes, but nothing plays or loads in them.
pub fn example() -> ViewNode {
    view! {
        <div
            style:display="flex"
            style:gap="16px"
            style:align-items="flex-start"
            style:font-size="13px"
            style:color="#71717a"
        >
            <div>
                <video controls style:width="200px" style:height="112px"></video>
                <div>"video with controls"</div>
            </div>
            <div>
                <canvas
                    style:width="120px"
                    style:height="112px"
                    style:border="1px dashed #a1a1aa"
                ></canvas>
                <div>"canvas (border added)"</div>
            </div>
        </div>
    }
}

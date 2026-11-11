use openui::prelude::*;

// Each form is its own scope for radio groups, even with the same name.
pub fn example() -> ViewNode {
    view! {
        <div style:display="flex" style:gap="32px" style:font-size="14px">
            <form>
                <p style:margin="0 0 6px" style:font-weight="500">"Shipping"</p>
                <div style:display="flex" style:align-items="center" style:gap="8px">
                    <input type="radio" name="speed" checked style:width="14px" style:height="14px" />
                    "Standard"
                    <input type="radio" name="speed" style:width="14px" style:height="14px" />
                    "Express"
                </div>
            </form>
            <form>
                <p style:margin="0 0 6px" style:font-weight="500">"Returns"</p>
                <div style:display="flex" style:align-items="center" style:gap="8px">
                    <input type="radio" name="speed" style:width="14px" style:height="14px" />
                    "Standard"
                    <input type="radio" name="speed" checked style:width="14px" style:height="14px" />
                    "Express"
                </div>
            </form>
        </div>
    }
}

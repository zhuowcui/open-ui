use openui::prelude::*;

pub fn example() -> ViewNode {
    view! {
        <div style:display="flex" style:gap="16px" style:font-size="14px">
            <fieldset
                style:border="1px solid #d4d4d8"
                style:border-radius="8px"
                style:padding="8px 14px 12px"
            >
                <legend style:padding="0 4px" style:font-weight="500">"Theme"</legend>
                <div style:display="flex" style:align-items="center" style:gap="8px">
                    <input type="radio" name="theme" checked style:width="14px" style:height="14px" />
                    "Light"
                    <input type="radio" name="theme" style:width="14px" style:height="14px" />
                    "Dark"
                </div>
            </fieldset>
            // A disabled fieldset disables every control inside it.
            <fieldset
                disabled
                style:border="1px solid #d4d4d8"
                style:border-radius="8px"
                style:padding="8px 14px 12px"
                style:color="#a1a1aa"
            >
                <legend style:padding="0 4px" style:font-weight="500">"Sync (off)"</legend>
                <div style:display="flex" style:align-items="center" style:gap="8px">
                    <input type="checkbox" checked style:width="14px" style:height="14px" />
                    "Sync settings"
                </div>
            </fieldset>
        </div>
    }
}

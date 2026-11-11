use openui::prelude::*;

pub fn example() -> ViewNode {
    let failed = create_signal(true);
    // A dynamic value has the property's Rust type, and updates with its signals.
    let tone = move || {
        if failed.get() {
            Color::from_hex(0xb91c1c, false)
        } else {
            Color::from_hex(0x15803d, false)
        }
    };
    view! {
        <div
            style:display="flex"
            style:align-items="center"
            style:gap="10px"
            style:padding="12px 16px"
            style:border="1px solid #e4e4e7"
            style:border-radius="10px"
            style:font-size="14px"
        >
            <span style:font-weight="600">"Build #418"</span>
            <span style:color={tone()} style:font-weight="500">
                {if failed.get() { "Failed" } else { "Passed" }}
            </span>
            <button
                style:margin-left="auto"
                style:padding="4px 10px"
                style:border="1px solid #d4d4d8"
                style:border-radius="6px"
                on:click={move |_| failed.set(!failed.get())}
            >
                "Retry"
            </button>
        </div>
    }
}

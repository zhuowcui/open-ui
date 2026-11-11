use openui::prelude::*;

pub fn example() -> ViewNode {
    let count = create_signal(0);
    view! {
        <div style:display="flex" style:gap="12px" style:align-items="center">
            <button
                style:padding="8px 16px"
                style:border="1px solid #18181b"
                style:border-radius="8px"
                style:background-color="#18181b"
                style:color="#fafafa"
                style:font-size="14px"
                style:font-weight="500"
                on:click={move |_| count.set(count.get() + 1)}
            >
                "Add one"
            </button>
            <button
                style:padding="8px 16px"
                style:border="1px solid #e4e4e7"
                style:border-radius="8px"
                style:background-color="#ffffff"
                style:font-size="14px"
                on:click={move |_| count.set(0)}
            >
                "Reset"
            </button>
            <span style:font-size="14px" style:color="#71717a">
                {format!("Count: {}", count.get())}
            </span>
        </div>
    }
}

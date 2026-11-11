use openui::prelude::*;

#[component]
fn Stat(label: String, value: String) -> ViewNode {
    view! {
        <div style:padding="12px 16px" style:border="1px solid #e4e4e7" style:border-radius="10px">
            <div style:color="#71717a" style:font-size="13px">{label}</div>
            <div style:font-size="24px" style:font-weight="600">{value}</div>
        </div>
    }
}

pub fn example() -> ViewNode {
    let likes = create_signal(0);
    view! {
        <div style:display="flex" style:gap="12px">
            <Stat label={"Uptime".to_string()} value={"99.98%".to_string()} />
            <Stat label={"Visits".to_string()} value={"1,284".to_string()} />
            <button
                style:padding="0 14px"
                style:border="1px solid #e4e4e7"
                style:border-radius="10px"
                on:click={move |_| likes.update(|n| *n += 1)}
            >
                {format!("Like ({})", likes.get())}
            </button>
        </div>
    }
}

use openui::prelude::*;

pub fn deploy_card() -> ViewNode {
    let deploys = create_signal(12);
    view! {
        <div
            style:box-sizing="border-box"
            style:width="360px"
            style:padding="24px"
            style:border="1px solid #e5e5e5"
            style:border-radius="14px"
        >
            <div style:display="flex" style:justify-content="space-between">
                <span style:font-size="16px" style:font-weight="600">"Production"</span>
                <span
                    style:padding="2px 8px"
                    style:border-radius="9999px"
                    style:background-color="#dcfce7"
                    style:color="#166534"
                    style:font-size="12px"
                    style:font-weight="500"
                >
                    "Live"
                </span>
            </div>
            <p style:margin="4px 0 20px" style:color="#737373">
                {format!("Deploy #{} went out 4 minutes ago", deploys.get())}
            </p>
            <div style:display="flex" style:gap="8px">
                <button
                    style:padding="6px 12px"
                    style:border-radius="6px"
                    style:background-color="#171717"
                    style:color="#fafafa"
                    style:font-weight="500"
                    on:click={move |_| deploys.set(deploys.get() + 1)}
                >
                    "Redeploy"
                </button>
                <button
                    style:padding="5px 12px"
                    style:border="1px solid #e5e5e5"
                    style:border-radius="6px"
                    style:font-weight="500"
                >
                    "View logs"
                </button>
            </div>
        </div>
    }
}

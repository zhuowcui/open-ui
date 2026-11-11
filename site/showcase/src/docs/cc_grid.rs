use openui::prelude::*;

pub fn example() -> ViewNode {
    view! {
        <div
            style:display="grid"
            style:grid-template-columns="1fr 1fr 1fr"
            style:gap="12px"
            style:font-size="14px"
        >
            <div style:padding="12px" style:border="1px solid #e4e4e7" style:border-radius="10px">
                <div style:color="#71717a">"Revenue"</div>
                <div style:font-size="20px" style:font-weight="600">"$45,231"</div>
            </div>
            <div style:padding="12px" style:border="1px solid #e4e4e7" style:border-radius="10px">
                <div style:color="#71717a">"Customers"</div>
                <div style:font-size="20px" style:font-weight="600">"2,350"</div>
            </div>
            <div style:padding="12px" style:border="1px solid #e4e4e7" style:border-radius="10px">
                <div style:color="#71717a">"Uptime"</div>
                <div style:font-size="20px" style:font-weight="600">"99.9%"</div>
            </div>
        </div>
    }
}

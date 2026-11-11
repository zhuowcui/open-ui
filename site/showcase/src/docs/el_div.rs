use openui::prelude::*;

pub fn example() -> ViewNode {
    let title = view! {
        <h2 style:font-size="18px" style:font-weight="600" style:margin="0">
            "Notifications"
        </h2>
    };
    // Tags carry no meaning in Open UI, so give the heading its role.
    if let ViewNode::Element(h2) = &title {
        h2.set_accessibility_role(AccessibilityRole::Heading).unwrap();
    }
    view! {
        <section
            style:width="380px"
            style:border="1px solid #e4e4e7"
            style:border-radius="12px"
        >
            <header style:padding="16px 20px" style:border-bottom="1px solid #e4e4e7">
                <Slot child={title} />
                <p style:margin="4px 0 0" style:color="#71717a" style:font-size="14px">
                    "Choose what we email you about."
                </p>
            </header>
            <div style:padding="16px 20px" style:font-size="14px">
                <p style:margin="0 0 8px">"Product updates"</p>
                <p style:margin="0">"Security alerts"</p>
            </div>
            <footer
                style:padding="12px 20px"
                style:background-color="#fafafa"
                style:border-radius="0 0 12px 12px"
                style:color="#71717a"
                style:font-size="13px"
            >
                "Saved automatically"
            </footer>
        </section>
    }
}

#[component]
fn Slot(child: ViewNode) -> ViewNode {
    child
}

use openui::prelude::*;

const LANDSCAPE: &[u8] = include_bytes!("../../assets/landscape.png");
const LANDSCAPE_SHA256: &str = "14fc4fc73d051e56a39d588c0beaf439dc767b0d36c322021870fa980a55b678";

pub fn example() -> ViewNode {
    // src loads nothing: register the bytes, then point the img at them.
    let image = current_document()
        .register_image_resource("landscape.png", "image/png", LANDSCAPE_SHA256, LANDSCAPE)
        .expect("register image");
    let img = view! { <img alt="Green hills under a blue sky" style:border-radius="10px" /> };
    if let ViewNode::Element(element) = &img {
        element.set_image_resource(image, Some((240.0, 150.0))).unwrap();
    }
    view! {
        <div style:width="240px" style:font-size="13px" style:color="#71717a">
            <Slot child={img} />
            <div style:margin-top="8px">"landscape.png, 240 × 150"</div>
        </div>
    }
}

#[component]
fn Slot(child: ViewNode) -> ViewNode {
    child
}

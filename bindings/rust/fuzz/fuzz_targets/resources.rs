#![no_main]

use libfuzzer_sys::fuzz_target;
use openui_dom::ElementTag;
use openui_engine::{Engine, ViewportMetrics};

fuzz_target!(|data: &[u8]| {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(32.0, 32.0, 1.0).unwrap()).unwrap();
    let image = engine.create_element(ElementTag::Image).unwrap();
    engine.append_child(engine.root(), image).unwrap();
    for (index, bytes) in data.chunks(64).take(64).enumerate() {
        let source = format!("fuzz:{index}");
        let hash = format!("input:{:02x}", bytes.first().copied().unwrap_or(0));
        let resource =
            engine.register_image_resource(source, "application/octet-stream", hash, bytes);
        let width = f32::from(bytes.first().copied().unwrap_or(0));
        let height = f32::from(bytes.get(1).copied().unwrap_or(0));
        let _ = engine.set_image_resource(image, resource, Some((width, height)));
        let _ = engine.scene();
    }
});

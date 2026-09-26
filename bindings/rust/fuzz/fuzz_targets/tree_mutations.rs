#![no_main]

use libfuzzer_sys::fuzz_target;
use openui_dom::ElementTag;
use openui_engine::{Engine, NodeHandle, ViewportMetrics};
use openui_style::{Color, StyleProperty};

fuzz_target!(|data: &[u8]| {
    let Ok(mut engine) = Engine::new(ViewportMetrics::from_logical_size(96.0, 64.0, 1.0).unwrap())
    else {
        return;
    };
    let root = engine.root();
    let mut live: Vec<NodeHandle> = vec![root];
    for chunk in data.chunks(3).take(512) {
        let op = chunk[0] % 5;
        let index = usize::from(*chunk.get(1).unwrap_or(&0)) % live.len();
        match op {
            0 if live.len() < 128 => {
                if let Ok(node) = engine.create_element(ElementTag::Div) {
                    let parent = live[index];
                    if engine.append_child(parent, node).is_ok() {
                        live.push(node);
                    }
                }
            }
            1 if index > 0 => {
                let node = live.swap_remove(index);
                let _ = engine.remove(node);
            }
            2 => {
                let value = f32::from(*chunk.get(2).unwrap_or(&0)) / 255.0;
                let _ = engine.set_property(live[index], StyleProperty::Opacity, value.into());
            }
            3 => {
                let shade = *chunk.get(2).unwrap_or(&0);
                let _ = engine.set_property(
                    live[index],
                    StyleProperty::BackgroundColor,
                    Color::from_rgba8(shade, shade, shade, 255).into(),
                );
            }
            _ => {
                let _ = engine.scene();
            }
        }
    }
});

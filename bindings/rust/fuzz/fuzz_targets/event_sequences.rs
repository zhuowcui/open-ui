#![no_main]

use libfuzzer_sys::fuzz_target;
use openui_dom::ElementTag;
use openui_engine::{Engine, PointerEventKind, ViewportMetrics};
use openui_style::{Display, LengthValue, StyleProperty};

fuzz_target!(|data: &[u8]| {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(128.0, 128.0, 1.0).unwrap()).unwrap();
    let target = engine.create_element(ElementTag::Button).unwrap();
    engine.append_child(engine.root(), target).unwrap();
    for (property, value) in [
        (StyleProperty::Display, Display::Block.into()),
        (StyleProperty::Width, LengthValue::px(64.0).into()),
        (StyleProperty::Height, LengthValue::px(32.0).into()),
    ] {
        engine.set_property(target, property, value).unwrap();
    }
    let _ = engine.scene();
    for chunk in data.chunks(4).take(1_024) {
        let kind = match chunk[0] & 3 {
            0 => PointerEventKind::Move,
            1 => PointerEventKind::Down,
            2 => PointerEventKind::Up,
            _ => PointerEventKind::Cancel,
        };
        let x = f32::from(*chunk.get(1).unwrap_or(&0)) - 64.0;
        let y = f32::from(*chunk.get(2).unwrap_or(&0)) - 64.0;
        let pointer = u64::from(*chunk.get(3).unwrap_or(&0));
        let _ = engine.pointer_event(pointer, kind, x, y);
    }
});

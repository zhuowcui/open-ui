//! Native event delegation, retained mutation, and weak event ownership.
use openui::prelude::*;
use std::{cell::RefCell, path::PathBuf, rc::Rc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let output = PathBuf::from(arguments.next().ok_or("output directory required")?);
    let scale: f64 = arguments.next().ok_or("device scale required")?.parse()?;
    if arguments.next().is_some() {
        return Err("unexpected native event arguments".into());
    }
    let document = Document::with_font_collection(
        ViewportMetrics::from_logical_size(160.0, 100.0, scale)?,
        FontCollection::deterministic_test(),
    )?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_attribute("data-name", "root")?;
    let target = Element::create(&document, "div")?;
    target.set_position(Position::Absolute)?;
    target.set_left(Length::px(20.0))?;
    target.set_top(Length::px(20.0))?;
    target.set_width(LengthValue::px(40.0))?;
    target.set_height(LengthValue::px(40.0))?;
    target.set_background_color(Color::BLUE)?;
    target.set_attribute("data-name", "target")?;
    root.append_child(&target)?;

    let events = Rc::new(RefCell::new(Vec::<Event>::new()));
    let route = Rc::new(RefCell::new(Vec::new()));
    for (receiver, capture, name, phase) in [
        (&root, true, "root", EventPhase::Capture),
        (&target, false, "target", EventPhase::Target),
        (&root, false, "root", EventPhase::Bubble),
    ] {
        let saved = events.clone();
        let observed = route.clone();
        let callback = move |event: &Event| {
            let element = event.target().expect("live native event target");
            let current = event.current_target().expect("live native listener");
            assert_eq!(
                element.get_attribute("data-name").unwrap().as_deref(),
                Some("target")
            );
            assert_eq!(
                current.get_attribute("data-name").unwrap().as_deref(),
                Some(name)
            );
            assert_eq!(event.phase(), Some(phase));
            if phase == EventPhase::Bubble {
                // The parent changes the unnamed child directly through the event.
                element.set_background_color(Color::RED).unwrap();
            }
            observed.borrow_mut().push((name, phase));
            saved.borrow_mut().push(event.clone());
        };
        if capture {
            receiver.on_capture("click", callback)?;
        } else {
            receiver.on("click", callback)?;
        }
    }
    std::fs::create_dir_all(&output)?;
    let before = target
        .bounding_rect()?
        .ok_or("owned target bounds required")?;
    std::fs::write(output.join("before.png"), document.render_to_png_buffer()?)?;
    target.click()?;
    assert_eq!(
        route.borrow().as_slice(),
        [
            ("root", EventPhase::Capture),
            ("target", EventPhase::Target),
            ("root", EventPhase::Bubble),
        ]
    );
    assert!(events
        .borrow()
        .iter()
        .all(|event| event.phase().is_none() && event.current_target().is_none()));
    assert!(events.borrow().iter().all(|event| event.target().is_some()));
    let after = target
        .bounding_rect()?
        .ok_or("owned target bounds required")?;
    assert_eq!(
        (before.x, before.y, before.width, before.height),
        (20.0, 20.0, 40.0, 40.0)
    );
    assert_eq!(before, after);
    std::fs::write(output.join("after.png"), document.render_to_png_buffer()?)?;
    let weak = target.downgrade();
    drop(target);
    drop(root);
    drop(document);
    assert!(
        weak.upgrade().is_none(),
        "saved events must not retain the document"
    );
    assert!(events
        .borrow()
        .iter()
        .all(|event| event.target().is_none() && event.current_target().is_none()));
    std::fs::write(output.join("events.json"),
        format!("{{\"callbacks\":3,\"owned_bounds_unchanged\":true,\"phase_cleared\":true,\"targets_expired_after_teardown\":true,\"scale\":{scale}}}\n"))?;
    println!("native event targets: delegated mutation, phases, geometry and teardown passed");
    Ok(())
}

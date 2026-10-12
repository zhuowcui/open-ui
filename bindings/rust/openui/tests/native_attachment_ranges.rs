use openui::prelude::*;
use openui::{AnimationAttachmentSnapshot, AnimationId};
use std::{cell::RefCell, rc::Rc};

fn box_(document: &Document, parent: &Element, width: f32, height: f32) -> Element {
    let element = Element::create(document, "div").unwrap();
    element.set_display(Display::Block).unwrap();
    element.set_width(LengthValue::px(width)).unwrap();
    element.set_height(LengthValue::px(height)).unwrap();
    parent.append_child(&element).unwrap();
    element
}
fn fixture(scale: f64) -> (Document, Element, Element, Element) {
    let document = Document::with_font_collection(
        ViewportMetrics::from_logical_size(200.0, 160.0, scale).unwrap(),
        FontCollection::deterministic_test(),
    )
    .unwrap();
    let body = document.body();
    body.set_margin(Edges::all(LengthValue::px(0.0))).unwrap();
    body.set_background_color(Color::WHITE).unwrap();
    let source = box_(&document, &body, 100.0, 80.0);
    source.set_overflow(Overflow::Hidden).unwrap();
    let content = box_(&document, &source, 100.0, 400.0);
    box_(&document, &content, 100.0, 120.0);
    let subject = box_(&document, &content, 100.0, 20.0);
    let target = box_(&document, &body, 40.0, 30.0);
    target.set_background_color(Color::BLUE).unwrap();
    (document, source, subject, target)
}
fn options() -> AnimationOptions {
    AnimationOptions {
        duration_ms: 100.0,
        fill: FillMode::Both,
        ..AnimationOptions::default()
    }
}
fn snapshot(document: &Document, id: AnimationId) -> AnimationAttachmentSnapshot {
    document.animation_attachment_snapshot(id).unwrap().unwrap()
}

#[test]
fn native_named_range_mutation_updates_pixels_and_reentrant_callbacks_on_the_same_animation() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (document, source, subject, target) = fixture(scale);
        let id = target
            .animate_on_view_range(
                StyleProperty::Opacity,
                Keyframes::from_values(0.0_f32, 1.0_f32),
                options(),
                &subject,
                TimelineAxis::Block,
                AnimationAttachmentRange::named(TimelineNamedRange::Entry),
            )
            .unwrap();
        source.scroll_to(0.0, 50.0).unwrap();
        assert_eq!(target.computed_style().unwrap().opacity, 0.5);
        let before = document.render_to_png_buffer().unwrap();
        let owned = snapshot(&document, id);
        assert_eq!(
            owned.resolved.unwrap().attachment,
            TimelineRangeBounds {
                start: 40.0,
                end: 60.0
            }
        );
        let observed = Rc::new(RefCell::new(Vec::new()));
        let native = document.clone();
        let element = target.clone();
        let events = observed.clone();
        target
            .on("animationend", move |_| {
                let state = native.animation_state(id).unwrap();
                let ranges = snapshot(&native, id).resolved.unwrap();
                events.borrow_mut().push((
                    state.phase,
                    ranges.attachment.start,
                    element.computed_style().unwrap().opacity,
                ));
            })
            .unwrap();
        document
            .set_animation_attachment_range(
                id,
                AnimationAttachmentRange::named(TimelineNamedRange::Exit),
            )
            .unwrap();
        assert_eq!(&*observed.borrow(), &[(AnimationPhase::Before, 120.0, 0.0)]);
        assert_eq!(document.animation_state(id).unwrap().id, id);
        assert_ne!(document.render_to_png_buffer().unwrap(), before);
        assert_eq!(owned.resolved.unwrap().attachment.start, 40.0);
        source.scroll_to(0.0, 130.0).unwrap();
        assert_eq!(target.computed_style().unwrap().opacity, 0.5);
        assert_eq!(document.get_time().unwrap(), 0.0);
        document.pause_animation(id).unwrap();
        let paused = document.render_to_png_buffer().unwrap();
        source.scroll_to(0.0, 170.0).unwrap();
        assert_eq!(document.render_to_png_buffer().unwrap(), paused);
        document.play_animation(id).unwrap();
        assert_eq!(target.computed_style().unwrap().opacity, 1.0);
        target.remove_event("animationend").unwrap();
    }
}

#[test]
fn native_browser_descriptors_reject_wrong_document_and_invalid_offsets_transactionally() {
    let (document, source, subject, target) = fixture(1.0);
    let id = target
        .animate_on_view_range(
            StyleProperty::Opacity,
            Keyframes::from_values(0.0_f32, 1.0_f32),
            options(),
            &subject,
            TimelineAxis::Y,
            AnimationAttachmentRange::default(),
        )
        .unwrap();
    source.scroll_to(0.0, 90.0).unwrap();
    let previous = snapshot(&document, id);
    let picture = document.render_to_png_buffer().unwrap();
    let (foreign, foreign_source, foreign_subject, _) = fixture(1.0);
    assert!(target
        .animate_on_scroll_range(
            StyleProperty::Opacity,
            Keyframes::from_values(0.0_f32, 1.0_f32),
            options(),
            &foreign_source,
            TimelineAxis::Y,
            AnimationAttachmentRange::default()
        )
        .is_err());
    assert!(target
        .animate_on_view_range(
            StyleProperty::Opacity,
            Keyframes::from_values(0.0_f32, 1.0_f32),
            options(),
            &foreign_subject,
            TimelineAxis::Y,
            AnimationAttachmentRange::default()
        )
        .is_err());
    let invalid = AnimationAttachmentRange {
        start: Some(TimelineRangeOffset {
            name: TimelineNamedRange::Cover,
            offset: LengthValue::calc_percent_px(f32::NAN, 0.0),
        }),
        end: None,
    };
    assert!(document
        .set_animation_attachment_range(id, invalid)
        .is_err());
    assert_eq!(snapshot(&document, id), previous);
    assert_eq!(document.render_to_png_buffer().unwrap(), picture);
    let weak = target.downgrade();
    drop(target);
    drop(source);
    drop(subject);
    drop(document);
    assert!(weak.upgrade().is_none());
    drop(foreign);
}

#[test]
fn native_relative_offsets_and_inactive_owned_snapshots_resolve_through_public_style_mutations() {
    let (document, source, subject, target) = fixture(1.0);
    target.set_font_size(LengthValue::px(12.0)).unwrap();
    let range = AnimationAttachmentRange {
        start: Some(
            TimelineRangeOffset::new(TimelineNamedRange::Cover, LengthValue::Em(1.0)).unwrap(),
        ),
        end: None,
    };
    let id = target
        .animate_on_view_range(
            StyleProperty::Opacity,
            Keyframes::from_values(0.0_f32, 1.0_f32),
            options(),
            &subject,
            TimelineAxis::Y,
            range,
        )
        .unwrap();
    let owned = snapshot(&document, id);
    assert_eq!(owned.resolved.unwrap().attachment.start, 52.0);
    target.set_font_size(LengthValue::px(20.0)).unwrap();
    assert_eq!(
        snapshot(&document, id).resolved.unwrap().attachment.start,
        60.0
    );
    assert_eq!(owned.resolved.unwrap().attachment.start, 52.0);
    source.set_display(Display::None).unwrap();
    let inactive = snapshot(&document, id);
    assert_eq!(inactive.range, range);
    assert_eq!(inactive.resolved, None);
    document.seek_animation_progress(id, 40.0).unwrap();
    source.set_display(Display::Block).unwrap();
    assert!(snapshot(&document, id).resolved.is_some());
    assert_eq!(
        document
            .animation_timeline_snapshot(id)
            .unwrap()
            .unwrap()
            .current_time_percentage,
        Some(40.0)
    );
    assert_eq!(target.computed_style().unwrap().opacity, 0.5);
    assert_eq!(inactive.resolved, None);
}

#[test]
fn native_explicit_view_insets_follow_subject_font_and_auto_scroll_padding() {
    let (document, source, subject, target) = fixture(1.0);
    subject.set_font_size(LengthValue::px(12.0)).unwrap();
    target.set_font_size(LengthValue::px(20.0)).unwrap();
    source
        .set_scroll_padding_bottom(LengthValue::percent(12.3))
        .unwrap();
    let id = target
        .animate_on_view_range_with_inset(
            StyleProperty::Opacity,
            Keyframes::from_values(0.0_f32, 1.0_f32),
            options(),
            &subject,
            TimelineAxis::Block,
            ViewTimelineInset::new(Some(LengthValue::Em(1.0)), None).unwrap(),
            AnimationAttachmentRange::default(),
        )
        .unwrap();
    assert_eq!(
        snapshot(&document, id).resolved.unwrap().named.cover,
        TimelineRangeBounds {
            start: 49.828125,
            end: 128.0
        }
    );
    subject.set_font_size(LengthValue::px(16.0)).unwrap();
    assert_eq!(
        snapshot(&document, id).resolved.unwrap().named.cover.end,
        124.0
    );
}

#[test]
fn native_newest_time_control_wins_over_deferred_progress_when_source_becomes_visible() {
    for operation in ["seek", "play", "finish", "range"] {
        let (document, source, _, target) = fixture(1.25);
        let id = target
            .animate_on_scroll_range(
                StyleProperty::Opacity,
                Keyframes::from_values(0.0_f32, 1.0_f32),
                options(),
                &source,
                TimelineAxis::Y,
                AnimationAttachmentRange::default(),
            )
            .unwrap();
        document.animation_timeline_snapshot(id).unwrap();
        source.set_display(Display::None).unwrap();
        assert!(
            !document
                .animation_timeline_snapshot(id)
                .unwrap()
                .unwrap()
                .active
        );
        document.seek_animation_progress(id, 50.0).unwrap();
        match operation {
            "seek" => document.seek_animation(id, 1.28).unwrap(),
            "play" => document.play_animation(id).unwrap(),
            "finish" => document.finish_animation(id).unwrap(),
            "range" => document
                .set_animation_attachment_range(id, AnimationAttachmentRange::default())
                .unwrap(),
            _ => unreachable!(),
        }
        source.set_display(Display::Block).unwrap();
        let resolved = document.animation_timeline_snapshot(id).unwrap().unwrap();
        let (time, value) = match operation {
            "seek" => (25.0, 0.25),
            "finish" => (100.0, 1.0),
            _ => (0.0, 0.0),
        };
        assert_eq!(resolved.current_time_percentage, Some(time), "{operation}");
        assert_eq!(
            target.computed_style().unwrap().opacity,
            value,
            "{operation}"
        );
        let picture = document.render_to_png_buffer().unwrap();
        assert_eq!(document.render_to_png_buffer().unwrap(), picture);
    }
}

#[test]
fn native_deferred_progress_retains_paused_percentage_and_rejected_control_is_transactional() {
    let (document, source, _, target) = fixture(1.0);
    let id = target
        .animate_on_scroll_range(
            StyleProperty::Opacity,
            Keyframes::from_values(0.0_f32, 1.0_f32),
            options(),
            &source,
            TimelineAxis::Y,
            AnimationAttachmentRange::default(),
        )
        .unwrap();
    source.set_display(Display::None).unwrap();
    document.seek_animation_progress(id, 50.0).unwrap();
    assert!(document.seek_animation(id, f64::NAN).is_err());
    document.pause_animation(id).unwrap();
    document.set_animation_playback_rate(id, -2.0).unwrap();
    source.set_display(Display::Block).unwrap();
    assert_eq!(
        document
            .animation_timeline_snapshot(id)
            .unwrap()
            .unwrap()
            .current_time_percentage,
        Some(50.0)
    );
    assert_eq!(target.computed_style().unwrap().opacity, 0.5);
}

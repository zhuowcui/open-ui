use openui::prelude::*;
use std::{cell::Cell, rc::Rc};

fn append(document: &Document, parent: &Element, tag: &str) -> Element {
    let element = Element::create(document, tag).unwrap();
    parent.append_child(&element).unwrap();
    element
}

fn table_fixture(document: &Document) -> (Element, Element, Element) {
    let table = append(document, &document.body(), "table");
    table.set_border_collapse(BorderCollapse::Collapse).unwrap();
    table
        .set_border_spacing((Length::px(0.0), Length::px(0.0)))
        .unwrap();
    table.set_width(LengthValue::px(30.0)).unwrap();
    let group = append(document, &table, "tbody");
    let mut first = None;
    let mut next = None;
    for count in [2, 3] {
        let row = append(document, &group, "tr");
        for _ in 0..count {
            let cell = append(document, &row, "td");
            cell.set_width(LengthValue::px(10.0)).unwrap();
            cell.set_height(LengthValue::px(10.0)).unwrap();
            cell.set_padding(Edges::all(LengthValue::px(0.0))).unwrap();
            cell.set_border(Border::NONE).unwrap();
            cell.set_background_color(Color::RED).unwrap();
            if first.is_none() {
                first = Some(cell);
            } else if next.is_none() {
                next = Some(cell);
            }
        }
    }
    (table, first.unwrap(), next.unwrap())
}

#[test]
fn native_table_span_reflection_matches_attributes_limits_and_element_kinds() {
    let document = Document::new(320, 240).unwrap();
    for tag in ["td", "th", "col", "colgroup"] {
        let element = Element::create(&document, tag).unwrap();
        let attribute = if matches!(tag, "td" | "th") {
            "colspan"
        } else {
            "span"
        };
        for (value, expected) in [(0, 1), (2, 2), (1000, 1000), (1001, 1000), (u32::MAX, 1)] {
            element.set_table_column_span(value).unwrap();
            assert_eq!(element.column_span().unwrap(), expected);
            assert_eq!(
                element.get_attribute(attribute).unwrap().unwrap(),
                if value > i32::MAX as u32 {
                    "1".into()
                } else {
                    value.to_string()
                }
            );
        }
        for (text, expected) in [
            (" \t+3suffix", 3),
            ("-0", 1),
            ("-2", 1),
            ("4294967296", 1000),
            ("invalid", 1),
        ] {
            element.set_attribute(attribute, text).unwrap();
            assert_eq!(element.column_span().unwrap(), expected);
        }
        element.remove_attribute(attribute).unwrap();
        assert_eq!(element.column_span().unwrap(), 1);
        if matches!(tag, "td" | "th") {
            for (value, expected) in [(0, 0), (2, 2), (65535, 65534), (u32::MAX, 1)] {
                element.set_row_span(value).unwrap();
                assert_eq!(element.row_span().unwrap(), expected);
            }
            element.set_attribute("ROWSPAN", "-0trailing").unwrap();
            assert_eq!(element.row_span().unwrap(), 0);
            element.remove_attribute("ROWSPAN").unwrap();
            assert_eq!(element.row_span().unwrap(), 1);
        } else {
            assert!(element.set_row_span(2).is_err());
        }
    }
    let plain = Element::create(&document, "div").unwrap();
    assert!(plain.row_span().is_err());
    assert!(plain.column_span().is_err());
    assert!(plain.set_row_span(2).is_err());
    assert!(plain.set_table_column_span(2).is_err());
    assert!(plain.get_attribute("rowspan").unwrap().is_none());
    assert!(plain.get_attribute("colspan").unwrap().is_none());
}

#[test]
fn native_cell_span_callback_reflows_owned_boxes_and_removal_restores_pixels() {
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        let document = Document::with_viewport_metrics(
            ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap(),
        )
        .unwrap();
        let (_table, cell, following) = table_fixture(&document);
        let before = cell.bounding_rect().unwrap().unwrap();
        let next_before = following.bounding_rect().unwrap().unwrap();
        let frame = document.render_to_png_buffer().unwrap();
        let callbacks = Rc::new(Cell::new(0));
        let count = callbacks.clone();
        let weak = cell.downgrade();
        cell.on("click", move |_| {
            weak.upgrade().unwrap().set_table_column_span(2).unwrap();
            count.set(count.get() + 1);
        })
        .unwrap();
        cell.click().unwrap();
        assert_eq!(callbacks.get(), 1);
        assert_eq!(cell.column_span().unwrap(), 2);
        let after = cell.bounding_rect().unwrap().unwrap();
        let next_after = following.bounding_rect().unwrap().unwrap();
        assert_eq!(after.width, before.width * 2.0);
        assert_eq!(next_after.x - next_before.x, before.width);
        assert_eq!(cell.client_rects().unwrap(), [after]);
        cell.remove_attribute("colspan").unwrap();
        assert_eq!(cell.bounding_rect().unwrap().unwrap(), before);
        assert_eq!(following.bounding_rect().unwrap().unwrap(), next_before);
        assert_eq!(document.render_to_png_buffer().unwrap(), frame);
    }
}

#[test]
fn native_zero_and_positive_rowspans_stop_at_the_containing_section() {
    for authored_row_height in [false, true] {
        for span in [0, 9] {
            for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
                let document = Document::with_viewport_metrics(
                    ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap(),
                )
                .unwrap();
                let table = append(&document, &document.body(), "table");
                table.set_border_collapse(BorderCollapse::Collapse).unwrap();
                table
                    .set_border_spacing((Length::px(0.0), Length::px(0.0)))
                    .unwrap();
                let mut first = None;
                let mut last = None;
                for count in [2, 1] {
                    let group = append(&document, &table, "tbody");
                    for _ in 0..count {
                        let row = append(&document, &group, "tr");
                        if authored_row_height {
                            row.set_height(LengthValue::px(10.0)).unwrap();
                        }
                        let cell = append(&document, &row, "td");
                        cell.set_width(LengthValue::px(10.0)).unwrap();
                        cell.set_height(LengthValue::px(10.0)).unwrap();
                        if first.is_none() {
                            first = Some(cell.clone());
                        }
                        last = Some(cell);
                    }
                }
                let first = first.unwrap();
                let last = last.unwrap();
                let before = first.bounding_rect().unwrap().unwrap();
                let last_before = last.bounding_rect().unwrap().unwrap();
                let pixels_before = document.render_to_png_buffer().unwrap();
                assert_eq!(before.height, 10.0);
                first.set_row_span(span).unwrap();
                let after = first.bounding_rect().unwrap().unwrap();
                // A cell's authored height is a minimum for its whole span.
                // Independent row minima require two10px rows; cell-only
                // minima need only10px in total after the first cell spans.
                let expected = if authored_row_height { 20.0 } else { 10.0 };
                assert_eq!(
                    after.height, expected,
                    "span={span}, rows={authored_row_height}, scale={scale}"
                );
                let last_after = last.bounding_rect().unwrap().unwrap();
                assert_eq!(last_after.x, after.x);
                assert_eq!(last_after.y, after.y + after.height);
                first.remove_attribute("rowspan").unwrap();
                assert_eq!(first.bounding_rect().unwrap().unwrap(), before);
                assert_eq!(last.bounding_rect().unwrap().unwrap(), last_before);
                assert_eq!(document.render_to_png_buffer().unwrap(), pixels_before);
            }
        }
    }
}

#[test]
fn native_table_spans_keep_real_continuations_and_refresh_after_attribute_changes() {
    let document = Document::new(400, 240).unwrap();
    let columns = append(&document, &document.body(), "section");
    columns.set_width(LengthValue::px(210.0)).unwrap();
    columns.set_height(LengthValue::px(53.0)).unwrap();
    columns.set_column_count(Some(2)).unwrap();
    columns.set_column_gap(LengthValue::px(13.0)).unwrap();
    let table = append(&document, &columns, "table");
    table.set_border_collapse(BorderCollapse::Collapse).unwrap();
    let group = append(&document, &table, "tbody");
    let mut first = None;
    for _ in 0..2 {
        let row = append(&document, &group, "tr");
        for _ in 0..2 {
            let cell = append(&document, &row, "td");
            cell.set_width(LengthValue::px(29.0)).unwrap();
            cell.set_height(LengthValue::px(95.0)).unwrap();
            cell.set_border(Border {
                width: 7.0,
                style: BorderStyle::Double,
                color: Color::RED,
            })
            .unwrap();
            if first.is_none() {
                first = Some(cell);
            }
        }
    }
    let cell = first.unwrap();
    let before = cell.client_rects().unwrap();
    assert!(before.len() >= 2, "the table must actually fragment");
    let frame = document.render_to_png_buffer().unwrap();
    cell.set_attribute("colspan", "2").unwrap();
    let changed = cell.client_rects().unwrap();
    assert!(
        changed.len() >= 2,
        "span mutation must retain real continuation boxes"
    );
    assert_ne!(changed, before);
    cell.remove_attribute("colspan").unwrap();
    assert_eq!(cell.client_rects().unwrap(), before);
    assert_eq!(document.render_to_png_buffer().unwrap(), frame);
}

#[test]
fn native_table_span_clone_keeps_reflection_and_detached_edits_use_the_same_state() {
    let document = Document::new(320, 240).unwrap();
    let cell = append(&document, &document.body(), "td");
    cell.set_table_column_span(3).unwrap();
    cell.set_row_span(0).unwrap();
    let cloned = cell.clone_subtree().unwrap();
    assert_eq!(cloned.column_span().unwrap(), 3);
    assert_eq!(cloned.row_span().unwrap(), 0);
    assert_eq!(
        cloned.get_attribute("colspan").unwrap().as_deref(),
        Some("3")
    );
    cloned.set_attribute("colspan", "2").unwrap();
    document.body().append_child(&cloned).unwrap();
    assert_eq!(cloned.column_span().unwrap(), 2);
    assert_eq!(cell.column_span().unwrap(), 3);
    let weak = cloned.downgrade();
    cloned.remove().unwrap();
    assert!(weak.upgrade().is_none());
    assert!(cloned.column_span().is_err());
    assert!(cloned.set_row_span(1).is_err());
}

//! Native table span reflection and layout state share the attribute path.
//!
//! Pinned Chromium 147.0.7727.50: html_table_cell_element.cc:45-63,152-162;
//! html_table_col_element.cc:64-75,99-101; table_constants.h:12-20;
//! html_parser_idioms.cc:217-283; element.cc:11042-11052.

use crate::{Engine, EngineError, NodeHandle};
use openui_dom::{ElementTag, NodeId};

const MAX_COLUMN_SPAN: u32 = 1000;
const MAX_ROW_SPAN: u32 = 65534;

fn is_cell(tag: ElementTag) -> bool {
    matches!(tag, ElementTag::TableCell | ElementTag::TableHeaderCell)
}

fn column_attribute(tag: ElementTag) -> Option<&'static str> {
    match tag {
        ElementTag::TableCell | ElementTag::TableHeaderCell => Some("colspan"),
        ElementTag::TableColumn | ElementTag::TableColumnGroup => Some("span"),
        _ => None,
    }
}

// HTML spaces only, an optional sign, and the initial ASCII digit sequence.
// Trailing garbage is accepted. Minus zero is accepted, other negatives fail.
// Positive overflow clamps to the span maximum, including integers > u32::MAX.
fn reflected_span(value: Option<&str>, minimum: u32, maximum: u32) -> u32 {
    let Some(value) = value else { return 1 };
    let bytes = value.as_bytes();
    let mut index = 0;
    while bytes
        .get(index)
        .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c'))
    {
        index += 1;
    }
    let negative = bytes.get(index) == Some(&b'-');
    if negative || bytes.get(index) == Some(&b'+') {
        index += 1;
    }
    let first_digit = index;
    let mut number = 0_u64;
    while let Some(byte) = bytes.get(index).filter(|byte| byte.is_ascii_digit()) {
        number = number
            .saturating_mul(10)
            .saturating_add(u64::from(*byte - b'0'));
        index += 1;
    }
    if first_digit == index || (negative && number != 0) {
        return 1;
    }
    number.clamp(u64::from(minimum), u64::from(maximum)) as u32
}

// Chromium's unsigned IDL setter stores values through signed-i32 range;
// values outside that range become the default attribute string "1".
fn unsigned_reflection_value(value: u32) -> u32 {
    if value > i32::MAX as u32 {
        1
    } else {
        value
    }
}

impl Engine {
    /// Reflected used column span for a td/th cell or col/colgroup element.
    pub fn column_span(&self, handle: NodeHandle) -> Result<u32, EngineError> {
        let node = self.resolve(handle)?;
        let attribute = column_attribute(self.document.node(node).tag).ok_or(
            EngineError::InvalidInput("column span requires td, th, col or colgroup"),
        )?;
        Ok(reflected_span(
            self.document.attribute(node, attribute),
            1,
            MAX_COLUMN_SPAN,
        ))
    }

    /// Set the native reflected colspan (td/th) or span (col/colgroup).
    pub fn set_column_span(&mut self, handle: NodeHandle, value: u32) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        let attribute = column_attribute(self.document.node(node).tag).ok_or(
            EngineError::InvalidInput("column span requires td, th, col or colgroup"),
        )?;
        self.set_attribute(
            handle,
            attribute,
            unsigned_reflection_value(value).to_string(),
        )
    }

    /// Reflected row span for a td/th cell. Zero extends to its section's end.
    pub fn row_span(&self, handle: NodeHandle) -> Result<u32, EngineError> {
        let node = self.resolve(handle)?;
        if !is_cell(self.document.node(node).tag) {
            return Err(EngineError::InvalidInput("row span requires td or th"));
        }
        Ok(reflected_span(
            self.document.attribute(node, "rowspan"),
            0,
            MAX_ROW_SPAN,
        ))
    }

    /// Set the native reflected rowspan, preserving zero's section semantics.
    pub fn set_row_span(&mut self, handle: NodeHandle, value: u32) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        if !is_cell(self.document.node(node).tag) {
            return Err(EngineError::InvalidInput("row span requires td or th"));
        }
        self.set_attribute(
            handle,
            "rowspan",
            unsigned_reflection_value(value).to_string(),
        )
    }

    /// True means the changed attribute participates in intrinsic table layout.
    pub(crate) fn sync_table_span_attribute(
        &mut self,
        node: NodeId,
        name: &str,
        value: Option<&str>,
    ) -> bool {
        let tag = self.document.node(node).tag;
        if column_attribute(tag) == Some(name) {
            self.document.node_mut(node).table_col_span = reflected_span(value, 1, MAX_COLUMN_SPAN);
            true
        } else if is_cell(tag) && name == "rowspan" {
            self.document.node_mut(node).table_row_span = reflected_span(value, 0, MAX_ROW_SPAN);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_table_span_parser_retains_minus_zero_and_clamps_positive_overflow() {
        for (text, column, row) in [
            ("", 1, 1),
            (" \t+003tail", 3, 3),
            ("-0garbage", 1, 0),
            ("-000", 1, 0),
            ("-1", 1, 1),
            ("+", 1, 1),
            ("  - 0", 1, 1),
            ("0x10", 1, 0),
            ("1.5", 1, 1),
            ("1e8", 1, 1),
            ("\u{a0}3", 1, 1),
            ("1001", 1000, 1001),
            ("65535", 1000, 65534),
            ("4294967296", 1000, 65534),
            ("999999999999999999999999999999", 1000, 65534),
            ("-999999999999999999999999999999", 1, 1),
        ] {
            assert_eq!(
                reflected_span(Some(text), 1, MAX_COLUMN_SPAN),
                column,
                "{text:?}"
            );
            assert_eq!(reflected_span(Some(text), 0, MAX_ROW_SPAN), row, "{text:?}");
        }
    }

    #[test]
    fn typed_and_generic_spans_update_one_layout_state_and_skip_identical_attributes() {
        let mut engine =
            Engine::new(crate::ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap())
                .unwrap();
        for tag in [
            ElementTag::TableCell,
            ElementTag::TableHeaderCell,
            ElementTag::TableColumn,
            ElementTag::TableColumnGroup,
        ] {
            let element = engine.create_native_element(tag).unwrap();
            let node = engine.resolve(element).unwrap();
            let attribute = column_attribute(tag).unwrap();
            engine.set_column_span(element, 1001).unwrap();
            assert_eq!(engine.column_span(element).unwrap(), 1000);
            assert_eq!(engine.attribute(element, attribute).unwrap(), Some("1001"));
            assert_eq!(engine.document.node(node).table_col_span, 1000);
            let before = engine.dirty_generations();
            engine
                .set_attribute(element, attribute.to_uppercase(), "1001")
                .unwrap();
            assert_eq!(engine.dirty_generations(), before);
            engine
                .set_attribute(element, attribute, " \t+2suffix")
                .unwrap();
            assert_eq!(engine.column_span(element).unwrap(), 2);
            assert_eq!(engine.document.node(node).table_col_span, 2);
            assert_eq!(engine.dirty_generations().intrinsic, before.intrinsic + 1);
            engine
                .remove_attribute(element, &attribute.to_uppercase())
                .unwrap();
            assert_eq!(engine.column_span(element).unwrap(), 1);
            assert_eq!(engine.document.node(node).table_col_span, 1);
            engine.set_column_span(element, u32::MAX).unwrap();
            assert_eq!(engine.attribute(element, attribute).unwrap(), Some("1"));
            assert_eq!(engine.column_span(element).unwrap(), 1);
        }
        let cell = engine.create_native_element(ElementTag::TableCell).unwrap();
        engine.set_row_span(cell, 0).unwrap();
        assert_eq!(engine.row_span(cell).unwrap(), 0);
        assert_eq!(
            engine
                .document
                .node(engine.resolve(cell).unwrap())
                .table_row_span,
            0
        );
        engine.remove_attribute(cell, "rowspan").unwrap();
        assert_eq!(engine.row_span(cell).unwrap(), 1);
        assert_eq!(
            engine
                .document
                .node(engine.resolve(cell).unwrap())
                .table_row_span,
            1
        );
    }

    #[test]
    fn span_type_checks_precede_attribute_mutation_and_keep_documents_isolated() {
        let mut engine =
            Engine::new(crate::ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap())
                .unwrap();
        let plain = engine.create_native_element(ElementTag::Div).unwrap();
        assert!(engine.set_row_span(plain, 2).is_err());
        assert!(engine.set_column_span(plain, 2).is_err());
        assert!(engine.attribute(plain, "rowspan").unwrap().is_none());
        assert!(engine.attribute(plain, "colspan").unwrap().is_none());
        engine.set_attribute(plain, "colspan", "2").unwrap();
        assert_eq!(
            engine
                .document
                .node(engine.resolve(plain).unwrap())
                .table_col_span,
            1
        );
        let mut other =
            Engine::new(crate::ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap())
                .unwrap();
        let foreign = other.create_native_element(ElementTag::TableCell).unwrap();
        assert_eq!(
            engine.set_row_span(foreign, 2),
            Err(EngineError::WrongDocument)
        );
        assert_eq!(engine.column_span(foreign), Err(EngineError::WrongDocument));
        let removed = engine.create_native_element(ElementTag::TableCell).unwrap();
        engine.append_child(engine.root(), removed).unwrap();
        engine.remove(removed).unwrap();
        assert_eq!(engine.row_span(removed), Err(EngineError::StaleHandle));
        assert_eq!(
            engine.set_column_span(removed, 2),
            Err(EngineError::StaleHandle)
        );
    }
}

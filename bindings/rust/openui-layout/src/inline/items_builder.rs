//! InlineItemsBuilder — collects inline items from the DOM tree.
//!
//! Mirrors Blink's `InlineItemsBuilder` from
//! `third_party/blink/renderer/core/layout/inline/inline_items_builder.cc`.
//!
//! Walks children of a block-level node in document order, flattening
//! inline content into a linear sequence of `InlineItem`s. Handles:
//! - Text nodes (with white-space processing)
//! - Inline elements (open/close tag items)
//! - Forced breaks (`<br>` via ElementTag convention)
//! - Text shaping via openui-text

use openui_dom::{Document, ElementTag, NodeId};
use openui_style::{ComputedStyle, Direction, TextTransform, WhiteSpace};
use openui_text::{
    apply_text_transform, BidiParagraph, Font, FontDescription, TextDirection, TextShaper,
};
use std::sync::Arc;

use super::items::{CollapseType, InlineItem, InlineItemType};

/// The collected inline items data — output of the builder.
#[derive(Clone, Debug)]
pub struct InlineItemsData {
    /// Concatenated text content from all text nodes.
    pub text: String,
    /// Flat list of inline items in document order.
    pub items: Vec<InlineItem>,
    /// Styles referenced by items (index into this vec).
    pub styles: Vec<ComputedStyle>,
}

impl InlineItemsData {
    /// Shape all text items using HarfBuzz via the text shaper.
    ///
    /// Each text item gets its own `ShapeResult` based on the item's
    /// text range and associated style.
    pub fn shape_text(&mut self) {
        let shaper = TextShaper::new();
        for item in &mut self.items {
            if item.item_type == InlineItemType::Text && !item.text_range.is_empty() {
                let text = &self.text[item.text_range.clone()];
                let style = &self.styles[item.style_index];
                let font_desc = style_to_font_description(style);
                let font = Font::new(font_desc);
                let direction = if style.direction == Direction::Rtl {
                    TextDirection::Rtl
                } else {
                    TextDirection::Ltr
                };
                let result = shaper.shape(text, &font, direction);
                item.shape_result = Some(Arc::new(result));
            }
        }
    }

    /// Run UAX#9 bidirectional analysis and set bidi_level on each item.
    ///
    /// Blink: `InlineItemsBuilder::SetBidiLevel` / `BidiParagraph::SetParagraph`.
    pub fn apply_bidi(&mut self, base_direction: TextDirection) {
        if self.text.is_empty() {
            return;
        }

        let bidi = BidiParagraph::new(&self.text, Some(base_direction));
        let runs = bidi.runs();

        for item in &mut self.items {
            if item.item_type == InlineItemType::Text && !item.text_range.is_empty() {
                // Find the bidi run covering this item's start byte offset
                for run in &runs {
                    if item.text_range.start >= run.start && item.text_range.start < run.end {
                        item.bidi_level = run.level;
                        break;
                    }
                }
            }
        }
    }
}

/// Convert a `ComputedStyle` to a `FontDescription` for text shaping.
pub fn style_to_font_description(style: &ComputedStyle) -> FontDescription {
    FontDescription {
        family: style.font_family.clone(),
        size: style.font_size,
        specified_size: style.font_size,
        weight: style.font_weight,
        stretch: style.font_stretch,
        style: style.font_style,
        variant_caps: style.font_variant_caps,
        letter_spacing: style.letter_spacing,
        word_spacing: style.word_spacing,
        locale: None,
        font_smoothing: style.font_smoothing,
        text_rendering: style.text_rendering,
        feature_settings: style.font_feature_settings.clone(),
        variation_settings: style.font_variation_settings.clone(),
        font_synthesis_weight: style.font_synthesis_weight,
        font_synthesis_style: style.font_synthesis_style,
        font_optical_sizing: style.font_optical_sizing,
    }
}

/// Builder that walks the DOM and collects inline items.
pub struct InlineItemsBuilder<'a> {
    doc: &'a Document,
    text: String,
    items: Vec<InlineItem>,
    styles: Vec<ComputedStyle>,
}

impl<'a> InlineItemsBuilder<'a> {
    pub fn new(doc: &'a Document) -> Self {
        Self {
            doc,
            text: String::new(),
            items: Vec::new(),
            styles: Vec::new(),
        }
    }

    /// Collect all inline items from children of a block-level node.
    ///
    /// This is the main entry point. It walks all children of `block_node_id`
    /// and produces a flat `InlineItemsData`.
    pub fn collect(doc: &Document, block_node_id: NodeId) -> InlineItemsData {
        let mut builder = InlineItemsBuilder::new(doc);
        builder.collect_children(block_node_id);
        InlineItemsData {
            text: builder.text,
            items: builder.items,
            styles: builder.styles,
        }
    }

    /// Get or insert a style, returning its index.
    fn intern_style(&mut self, style: &ComputedStyle) -> usize {
        // Simple linear scan — inline item counts are small.
        // We don't dedup aggressively; each unique style object gets an entry.
        let idx = self.styles.len();
        self.styles.push(style.clone());
        idx
    }

    /// Walk children of a node and collect inline items.
    fn collect_children(&mut self, parent_id: NodeId) {
        for child_id in self.doc.children(parent_id).collect::<Vec<_>>() {
            let node = self.doc.node(child_id);
            match node.tag {
                ElementTag::Text => {
                    if let Some(ref text) = node.text {
                        let style = node.style.clone();
                        self.append_text(child_id, text, &style);
                    }
                }
                ElementTag::Span => {
                    let style = node.style.clone();
                    self.enter_inline(child_id, &style);
                    self.collect_children(child_id);
                    self.exit_inline(child_id, &style);
                }
                ElementTag::Div => {
                    // Check if this is inline-level
                    let display = node.style.display;
                    if display.is_inline_level() {
                        let style = node.style.clone();
                        self.append_atomic_inline(child_id, &style);
                    }
                    // Block-in-inline: not handled in this wave
                }
                ElementTag::Viewport => {
                    // Should not appear as a child in inline context
                }
            }
        }
    }

    /// Handle a text node — apply text-transform, process white-space, and append a Text item.
    fn append_text(&mut self, node_id: NodeId, text: &str, style: &ComputedStyle) {
        if text.is_empty() {
            return;
        }

        // Apply text-transform before white-space processing (matches Blink order).
        let transformed = if style.text_transform != TextTransform::None {
            apply_text_transform(text, style.text_transform)
        } else {
            text.to_string()
        };

        let processed = process_white_space(&transformed, style.white_space);
        if processed.is_empty() {
            return;
        }

        let style_index = self.intern_style(style);
        let start = self.text.len();
        self.text.push_str(&processed);
        let end = self.text.len();

        // Determine collapse type at the end
        let last_char = processed.as_bytes()[processed.len() - 1];
        let (end_collapse, is_newline) = match style.white_space {
            WhiteSpace::Normal | WhiteSpace::Nowrap => {
                if last_char == b' ' {
                    (CollapseType::Collapsible, false)
                } else {
                    (CollapseType::NotCollapsible, false)
                }
            }
            WhiteSpace::Pre => (CollapseType::NotCollapsible, false),
            WhiteSpace::PreWrap | WhiteSpace::BreakSpaces => {
                if last_char == b' ' || last_char == b'\t' {
                    (CollapseType::Collapsible, false)
                } else if last_char == b'\n' {
                    (CollapseType::Collapsible, true)
                } else {
                    (CollapseType::NotCollapsible, false)
                }
            }
            WhiteSpace::PreLine => {
                if last_char == b' ' {
                    (CollapseType::Collapsible, false)
                } else if last_char == b'\n' {
                    (CollapseType::NotCollapsible, true)
                } else {
                    (CollapseType::NotCollapsible, false)
                }
            }
        };

        self.items.push(InlineItem {
            item_type: InlineItemType::Text,
            text_range: start..end,
            node_id,
            shape_result: None,
            style_index,
            end_collapse_type: end_collapse,
            is_end_collapsible_newline: is_newline,
            bidi_level: if style.direction == Direction::Rtl { 1 } else { 0 },
        });
    }

    /// Handle inline element open (`<span>`).
    fn enter_inline(&mut self, node_id: NodeId, style: &ComputedStyle) {
        let style_index = self.intern_style(style);
        let offset = self.text.len();
        self.items.push(InlineItem {
            item_type: InlineItemType::OpenTag,
            text_range: offset..offset,
            node_id,
            shape_result: None,
            style_index,
            end_collapse_type: CollapseType::NotCollapsible,
            is_end_collapsible_newline: false,
            bidi_level: 0,
        });
    }

    /// Handle inline element close (`</span>`).
    fn exit_inline(&mut self, node_id: NodeId, style: &ComputedStyle) {
        let style_index = self.intern_style(style);
        let offset = self.text.len();
        self.items.push(InlineItem {
            item_type: InlineItemType::CloseTag,
            text_range: offset..offset,
            node_id,
            shape_result: None,
            style_index,
            end_collapse_type: CollapseType::NotCollapsible,
            is_end_collapsible_newline: false,
            bidi_level: 0,
        });
    }

    /// Handle an atomic inline element (inline-block, etc.).
    fn append_atomic_inline(&mut self, node_id: NodeId, style: &ComputedStyle) {
        let style_index = self.intern_style(style);
        let offset = self.text.len();
        // Insert object replacement character U+FFFC as placeholder
        self.text.push('\u{FFFC}');
        let end = self.text.len();
        self.items.push(InlineItem {
            item_type: InlineItemType::AtomicInline,
            text_range: offset..end,
            node_id,
            shape_result: None,
            style_index,
            end_collapse_type: CollapseType::NotCollapsible,
            is_end_collapsible_newline: false,
            bidi_level: 0,
        });
    }

    /// Handle a forced line break.
    pub fn append_break(&mut self, node_id: NodeId, style: &ComputedStyle) {
        let style_index = self.intern_style(style);
        let offset = self.text.len();
        self.text.push('\n');
        let end = self.text.len();
        self.items.push(InlineItem {
            item_type: InlineItemType::Control,
            text_range: offset..end,
            node_id,
            shape_result: None,
            style_index,
            end_collapse_type: CollapseType::NotCollapsible,
            is_end_collapsible_newline: true,
            bidi_level: 0,
        });
    }
}

// ── White-space processing (CSS Text Module Level 3 §4) ─────────────────

/// Process text according to the CSS `white-space` property.
///
/// Implements CSS Text Level 3 §4.1 (The White Space Processing Rules):
/// - normal/nowrap: collapse whitespace runs to single space
/// - pre/pre-wrap/break-spaces: preserve all whitespace
/// - pre-line: collapse spaces but preserve newlines
pub fn process_white_space(text: &str, white_space: WhiteSpace) -> String {
    match white_space {
        WhiteSpace::Normal | WhiteSpace::Nowrap => collapse_whitespace(text),
        WhiteSpace::Pre | WhiteSpace::PreWrap | WhiteSpace::BreakSpaces => text.to_string(),
        WhiteSpace::PreLine => collapse_spaces_preserve_newlines(text),
    }
}

/// Collapse runs of ASCII whitespace (space, tab, newline, CR, FF) to a single space.
///
/// CSS Text Level 3 §4.1.1: "Any sequence of collapsible spaces and tabs
/// immediately preceding or following a segment break is removed."
/// Then: "Every collapsible tab is converted to a collapsible space."
/// Then: "Any collapsible space immediately following another collapsible space
/// is collapsed to have zero advance width."
pub fn collapse_whitespace(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_whitespace = false;
    for ch in text.chars() {
        if ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' || ch == '\x0C' {
            if !in_whitespace {
                result.push(' ');
                in_whitespace = true;
            }
        } else {
            result.push(ch);
            in_whitespace = false;
        }
    }
    result
}

/// Collapse spaces but preserve newlines (for `white-space: pre-line`).
///
/// CSS Text Level 3 §4.1.1 for pre-line:
/// "Collapsible spaces before and after a forced line break are removed."
/// Newlines are treated as forced breaks. Sequences of spaces collapse to one.
pub fn collapse_spaces_preserve_newlines(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_space_run = false;
    for ch in text.chars() {
        if ch == '\n' {
            // Preserve newlines; reset space tracking
            // Remove any trailing space we just added
            if in_space_run {
                // Pop the collapsed space that preceded the newline
                if result.ends_with(' ') {
                    result.pop();
                }
            }
            result.push('\n');
            in_space_run = false;
        } else if ch == ' ' || ch == '\t' || ch == '\r' || ch == '\x0C' {
            if !in_space_run {
                result.push(' ');
                in_space_run = true;
            }
        } else {
            result.push(ch);
            in_space_run = false;
        }
    }
    result
}

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
use openui_style::{ComputedStyle, Direction, Display, Float, TabSize, TextTransform, UnicodeBidi, WhiteSpace};
use openui_text::{
    apply_text_transform, BidiParagraph, Font, FontDescription, TextDirection, TextShaper,
};
use std::sync::Arc;

use super::items::{CollapseType, InlineItem, InlineItemType};
use crate::length_resolver::resolve_margin_or_padding;

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
                // Use bidi level for direction: odd = RTL, even = LTR.
                // This ensures RTL sub-items (after bidi splitting) are shaped
                // with RTL direction, not the CSS direction property.
                let direction = if item.bidi_level % 2 == 1 {
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
    /// If a text item spans multiple bidi levels, it is split into multiple
    /// items at bidi level boundaries so that each sub-item can be shaped
    /// and reordered independently.
    ///
    /// CSS `unicode-bidi` control characters (LRE, RLE, LRO, RLO, LRI, RLI,
    /// FSI, PDF, PDI) are injected around inline element boundaries before
    /// bidi analysis, then mapped back to the original text positions.
    ///
    /// Blink: `InlineItemsBuilder::SetBidiLevel` / `BidiParagraph::SetParagraph`.
    pub fn apply_bidi(&mut self, base_direction: TextDirection) {
        if self.text.is_empty() {
            return;
        }

        // Build a bidi text buffer that includes unicode-bidi control characters
        // injected at inline element boundaries (OpenTag/CloseTag).
        // We also build a mapping from bidi buffer positions back to original
        // text positions so we can assign levels correctly.
        let mut bidi_text = String::with_capacity(self.text.len() + self.items.len() * 2);
        // Maps each byte in bidi_text back to the corresponding byte in self.text.
        // Control characters map to the tag item's text_range position.
        let mut bidi_to_orig: Vec<usize> = Vec::with_capacity(self.text.len() + self.items.len() * 2);

        for item in &self.items {
            match item.item_type {
                InlineItemType::OpenTag => {
                    let style = &self.styles[item.style_index];
                    let tag_pos = item.text_range.start;
                    for ch in bidi_open_chars(style.unicode_bidi, style.direction) {
                        let ch_str = String::from(ch);
                        for _ in 0..ch_str.len() {
                            bidi_to_orig.push(tag_pos);
                        }
                        bidi_text.push(ch);
                    }
                }
                InlineItemType::CloseTag => {
                    let style = &self.styles[item.style_index];
                    let tag_pos = item.text_range.start;
                    for ch in bidi_close_chars(style.unicode_bidi) {
                        let ch_str = String::from(ch);
                        for _ in 0..ch_str.len() {
                            bidi_to_orig.push(tag_pos);
                        }
                        bidi_text.push(ch);
                    }
                }
                InlineItemType::Text | InlineItemType::AtomicInline => {
                    // Copy the actual text content.
                    let range = item.text_range.clone();
                    let segment = &self.text[range.clone()];
                    for (i, _) in segment.as_bytes().iter().enumerate() {
                        bidi_to_orig.push(range.start + i);
                    }
                    bidi_text.push_str(segment);
                }
                InlineItemType::Control => {
                    let range = item.text_range.clone();
                    let segment = &self.text[range.clone()];
                    for (i, _) in segment.as_bytes().iter().enumerate() {
                        bidi_to_orig.push(range.start + i);
                    }
                    bidi_text.push_str(segment);
                }
                _ => {}
            }
        }

        let bidi = BidiParagraph::new(&bidi_text, Some(base_direction));
        let runs = bidi.runs();

        // Build a mapping from original text byte positions to bidi levels.
        // We use the bidi runs (which are in bidi_text coordinates) and map
        // them back to original text coordinates via bidi_to_orig.
        // Create a per-byte level map for the original text.
        let mut orig_levels = vec![0u8; self.text.len()];
        for run in &runs {
            // Walk each byte in the bidi run and map back to original position.
            for bidi_byte_pos in run.start..run.end {
                if bidi_byte_pos < bidi_to_orig.len() {
                    let orig_byte_pos = bidi_to_orig[bidi_byte_pos];
                    if orig_byte_pos < orig_levels.len() {
                        orig_levels[orig_byte_pos] = run.level;
                    }
                }
            }
        }

        // First pass: assign levels to each item from the level at its start byte.
        for item in &mut self.items {
            if (item.item_type == InlineItemType::Text
                || item.item_type == InlineItemType::AtomicInline)
                && !item.text_range.is_empty()
            {
                let start = item.text_range.start;
                if start < orig_levels.len() {
                    item.bidi_level = orig_levels[start];
                }
            }
        }

        // Second pass: assign bidi levels to OpenTag/CloseTag items from
        // their neighboring content items. OpenTag inherits the level of the
        // first subsequent Text/AtomicInline; CloseTag inherits the level of
        // the last preceding Text/AtomicInline. This ensures tag items don't
        // break contiguous bidi runs during UAX#9 L2 reordering.
        let base_level = if base_direction == TextDirection::Rtl { 1 } else { 0 };
        for i in 0..self.items.len() {
            match self.items[i].item_type {
                InlineItemType::OpenTag => {
                    let level = self.items[i + 1..]
                        .iter()
                        .find(|it| {
                            it.item_type == InlineItemType::Text
                                || it.item_type == InlineItemType::AtomicInline
                        })
                        .map(|it| it.bidi_level)
                        .unwrap_or(base_level);
                    self.items[i].bidi_level = level;
                }
                InlineItemType::CloseTag => {
                    let level = self.items[..i]
                        .iter()
                        .rev()
                        .find(|it| {
                            it.item_type == InlineItemType::Text
                                || it.item_type == InlineItemType::AtomicInline
                        })
                        .map(|it| it.bidi_level)
                        .unwrap_or(base_level);
                    self.items[i].bidi_level = level;
                }
                _ => {}
            }
        }

        // Third pass: split text items that span multiple bidi levels.
        // Re-derive runs in original text coordinates from orig_levels.
        let orig_runs = derive_runs_from_levels(&self.text, &orig_levels);

        let mut new_items = Vec::with_capacity(self.items.len());
        for item in self.items.drain(..) {
            if item.item_type != InlineItemType::Text || item.text_range.is_empty() {
                new_items.push(item);
                continue;
            }

            // Collect bidi runs that overlap this item's text range.
            let item_start = item.text_range.start;
            let item_end = item.text_range.end;
            let mut sub_items: Vec<(usize, usize, u8)> = Vec::new();

            for run in &orig_runs {
                if run.end <= item_start || run.start >= item_end {
                    continue;
                }
                let overlap_start = run.start.max(item_start);
                let overlap_end = run.end.min(item_end);
                if overlap_start < overlap_end {
                    sub_items.push((overlap_start, overlap_end, run.level));
                }
            }

            if sub_items.len() <= 1 {
                // Item is entirely within one bidi run — keep as-is.
                new_items.push(item);
            } else {
                // Split the item at bidi level boundaries.
                for (sub_start, sub_end, level) in sub_items {
                    new_items.push(InlineItem {
                        item_type: InlineItemType::Text,
                        text_range: sub_start..sub_end,
                        node_id: item.node_id,
                        shape_result: None, // Will be re-shaped below
                        style_index: item.style_index,
                        end_collapse_type: if sub_end == item_end {
                            item.end_collapse_type
                        } else {
                            CollapseType::NotCollapsible
                        },
                        is_end_collapsible_newline: if sub_end == item_end {
                            item.is_end_collapsible_newline
                        } else {
                            false
                        },
                        bidi_level: level,
                        intrinsic_inline_size: None,
                    });
                }
            }
        }

        self.items = new_items;
    }
}

/// Return bidi control characters to insert BEFORE an inline element's content
/// based on its `unicode-bidi` and `direction` properties.
///
/// CSS Writing Modes §2.2: unicode-bidi controls how the element interacts
/// with the bidirectional algorithm.
fn bidi_open_chars(unicode_bidi: UnicodeBidi, direction: Direction) -> Vec<char> {
    match unicode_bidi {
        UnicodeBidi::Normal => vec![],
        UnicodeBidi::Embed => {
            if direction == Direction::Ltr {
                vec!['\u{202A}'] // LRE
            } else {
                vec!['\u{202B}'] // RLE
            }
        }
        UnicodeBidi::Override => {
            if direction == Direction::Ltr {
                vec!['\u{202D}'] // LRO
            } else {
                vec!['\u{202E}'] // RLO
            }
        }
        UnicodeBidi::Isolate => {
            if direction == Direction::Ltr {
                vec!['\u{2066}'] // LRI
            } else {
                vec!['\u{2067}'] // RLI
            }
        }
        UnicodeBidi::IsolateOverride => {
            if direction == Direction::Ltr {
                vec!['\u{2066}', '\u{202D}'] // LRI + LRO
            } else {
                vec!['\u{2067}', '\u{202E}'] // RLI + RLO
            }
        }
        UnicodeBidi::Plaintext => {
            vec!['\u{2068}'] // FSI
        }
    }
}

/// Return bidi control characters to insert AFTER an inline element's content
/// based on its `unicode-bidi` property.
fn bidi_close_chars(unicode_bidi: UnicodeBidi) -> Vec<char> {
    match unicode_bidi {
        UnicodeBidi::Normal => vec![],
        UnicodeBidi::Embed | UnicodeBidi::Override => {
            vec!['\u{202C}'] // PDF
        }
        UnicodeBidi::Isolate => {
            vec!['\u{2069}'] // PDI
        }
        UnicodeBidi::IsolateOverride => {
            vec!['\u{202C}', '\u{2069}'] // PDF + PDI
        }
        UnicodeBidi::Plaintext => {
            vec!['\u{2069}'] // PDI
        }
    }
}

/// A simple bidi run in original text coordinates.
struct OrigBidiRun {
    start: usize,
    end: usize,
    level: u8,
}

/// Derive contiguous same-level runs from per-byte levels.
fn derive_runs_from_levels(text: &str, levels: &[u8]) -> Vec<OrigBidiRun> {
    if levels.is_empty() {
        return Vec::new();
    }
    let char_byte_offsets: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    if char_byte_offsets.is_empty() {
        return Vec::new();
    }

    let mut runs = Vec::new();
    let mut run_start = 0usize;
    let mut current_level = levels[0];

    for i in 1..levels.len() {
        // Only consider char boundaries to avoid splitting mid-char.
        if !text.is_char_boundary(i) {
            continue;
        }
        if levels[i] != current_level {
            runs.push(OrigBidiRun {
                start: run_start,
                end: i,
                level: current_level,
            });
            run_start = i;
            current_level = levels[i];
        }
    }
    runs.push(OrigBidiRun {
        start: run_start,
        end: text.len(),
        level: current_level,
    });
    runs
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
        locale: style.locale.clone(),
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
    /// Whether the last space appended to `text` came from a collapsible
    /// white-space mode (normal/nowrap/pre-line). A preserved space from
    /// `pre` or `pre-wrap` should not cause collapsing of the next node's
    /// leading space.
    last_space_collapsible: bool,
}

impl<'a> InlineItemsBuilder<'a> {
    pub fn new(doc: &'a Document) -> Self {
        Self {
            doc,
            text: String::new(),
            items: Vec::new(),
            styles: Vec::new(),
            last_space_collapsible: false,
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

    /// Collect inline items from a specific set of child node IDs.
    ///
    /// Used by anonymous block box wrapping (CSS 2.2 §9.2.1.1) when only
    /// a subset of children should participate in the inline formatting context.
    pub fn collect_for_children(
        doc: &Document,
        _block_node_id: NodeId,
        children: &[NodeId],
    ) -> InlineItemsData {
        let mut builder = InlineItemsBuilder::new(doc);
        for &child_id in children {
            builder.collect_single_child(child_id);
        }
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
            self.collect_single_child(child_id);
        }
    }

    /// Process a single child node into inline items.
    fn collect_single_child(&mut self, child_id: NodeId) {
        let node = self.doc.node(child_id);

        // display:none generates no boxes at all.
        if node.style.display == Display::None {
            return;
        }
        // Out-of-flow children (absolute, fixed, floated) don't participate
        // in inline layout.
        if node.style.is_out_of_flow() {
            return;
        }

        match node.tag {
            ElementTag::Text => {
                if let Some(ref text) = node.text {
                    let style = node.style.clone();
                    self.append_text(child_id, text, &style);
                }
            }
            ElementTag::Span => {
                let display = node.style.display;
                let style = node.style.clone();
                if display == Display::InlineBlock
                    || display == Display::InlineFlex
                    || display == Display::InlineGrid
                {
                    self.append_atomic_inline(child_id, &style);
                } else {
                    self.enter_inline(child_id, &style);
                    self.collect_children(child_id);
                    self.exit_inline(child_id, &style);
                }
            }
            ElementTag::Div => {
                let display = node.style.display;
                if display == Display::Inline {
                    // display:inline on a div creates a normal inline box, not atomic.
                    let style = node.style.clone();
                    self.enter_inline(child_id, &style);
                    self.collect_children(child_id);
                    self.exit_inline(child_id, &style);
                } else if display == Display::InlineBlock
                    || display == Display::InlineFlex
                    || display == Display::InlineGrid
                {
                    let style = node.style.clone();
                    self.append_atomic_inline(child_id, &style);
                }
            }
            ElementTag::Viewport => {
                // Should not appear as a child in inline context
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
        // Expand tab characters to spaces using CSS tab-size property.
        // In pre/pre-wrap/break-spaces modes, tabs are preserved by
        // process_white_space but need expansion to tab stops.
        let processed = if matches!(
            style.white_space,
            WhiteSpace::Pre | WhiteSpace::PreWrap | WhiteSpace::BreakSpaces
        ) {
            let font_desc = style_to_font_description(style);
            let font = Font::new(font_desc);
            let space_advance = font.width(" ");
            let font_clone = font;
            expand_tabs(&processed, &style.tab_size, space_advance, |ch| {
                // Use the actual shaped glyph advance for each character.
                let mut buf = [0u8; 4];
                font_clone.width(ch.encode_utf8(&mut buf))
            })
        } else {
            processed
        };
        if processed.is_empty() {
            return;
        }

        // CSS Text Level 3 §4.1.1 Phase I Rule 4: collapse cross-node
        // adjacent collapsible spaces. Only strip the leading space when
        // BOTH the previous space was collapsible AND the current mode is
        // collapsible — a preserved space from `pre` must not trigger
        // collapsing.
        let processed = if is_collapsible_ws_mode(style.white_space)
            && self.last_space_collapsible
            && self.text.ends_with(' ')
            && processed.starts_with(' ')
        {
            processed[1..].to_string()
        } else {
            processed
        };
        if processed.is_empty() {
            return;
        }

        let style_index = self.intern_style(style);
        let start = self.text.len();
        self.text.push_str(&processed);
        let end = self.text.len();

        // Determine collapse type at the end and update last_space_collapsible.
        let last_char = processed.as_bytes()[processed.len() - 1];
        let (end_collapse, is_newline) = match style.white_space {
            WhiteSpace::Normal | WhiteSpace::Nowrap => {
                if last_char == b' ' {
                    self.last_space_collapsible = true;
                    (CollapseType::Collapsible, false)
                } else {
                    self.last_space_collapsible = false;
                    (CollapseType::NotCollapsible, false)
                }
            }
            WhiteSpace::Pre => {
                self.last_space_collapsible = false;
                (CollapseType::NotCollapsible, false)
            }
            WhiteSpace::PreWrap => {
                if last_char == b' ' || last_char == b'\t' {
                    self.last_space_collapsible = false;
                    (CollapseType::Collapsible, false)
                } else if last_char == b'\n' {
                    self.last_space_collapsible = false;
                    (CollapseType::Collapsible, true)
                } else {
                    self.last_space_collapsible = false;
                    (CollapseType::NotCollapsible, false)
                }
            }
            WhiteSpace::BreakSpaces => {
                self.last_space_collapsible = false;
                // CSS Text §3: break-spaces preserves all spaces (including trailing).
                if last_char == b'\n' {
                    (CollapseType::NotCollapsible, true)
                } else {
                    (CollapseType::NotCollapsible, false)
                }
            }
            WhiteSpace::PreLine => {
                if last_char == b' ' {
                    self.last_space_collapsible = true;
                    (CollapseType::Collapsible, false)
                } else if last_char == b'\n' {
                    self.last_space_collapsible = false;
                    (CollapseType::NotCollapsible, true)
                } else {
                    self.last_space_collapsible = false;
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
            intrinsic_inline_size: None,
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
            intrinsic_inline_size: None,
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
            intrinsic_inline_size: None,
        });
    }

    /// Handle an atomic inline element (inline-block, etc.).
    ///
    /// Computes intrinsic inline size from the element's children for
    /// use by the line breaker when CSS `width` is `auto`.
    fn append_atomic_inline(&mut self, node_id: NodeId, style: &ComputedStyle) {
        let style_index = self.intern_style(style);
        let offset = self.text.len();
        // Insert object replacement character U+FFFC as placeholder
        self.text.push('\u{FFFC}');
        let end = self.text.len();

        // Compute intrinsic inline size by examining children.
        let intrinsic = self.compute_intrinsic_inline_size(node_id);

        self.items.push(InlineItem {
            item_type: InlineItemType::AtomicInline,
            text_range: offset..end,
            node_id,
            shape_result: None,
            style_index,
            end_collapse_type: CollapseType::NotCollapsible,
            is_end_collapsible_newline: false,
            bidi_level: 0,
            intrinsic_inline_size: intrinsic,
        });
    }

    /// Compute the intrinsic inline size of an atomic inline's content.
    ///
    /// Recursively walks all descendants, shapes text content, and accumulates
    /// widths to approximate the shrink-to-fit width for `width: auto` elements.
    /// Inline children sum widths; block children take the max (they stack vertically).
    fn compute_intrinsic_inline_size(&self, node_id: NodeId) -> Option<f32> {
        let (width, has_content) = self.compute_intrinsic_inline_size_recursive(node_id);
        if has_content { Some(width) } else { None }
    }

    /// Recursive helper: returns (accumulated_width, has_content).
    fn compute_intrinsic_inline_size_recursive(&self, node_id: NodeId) -> (f32, bool) {
        let mut max_width = 0.0f32;
        let mut current_inline_row = 0.0f32;
        let mut has_content = false;

        let shaper = TextShaper::new();

        for child_id in self.doc.children(node_id) {
            let child = self.doc.node(child_id);

            // Issue 4: Skip display:none — generates no boxes.
            if child.style.display == Display::None {
                continue;
            }
            // Issue 4: Skip absolutely positioned elements — out of flow.
            if child.style.position.is_absolutely_positioned() {
                continue;
            }
            // Issue 4: Skip floated elements — out of flow.
            if child.style.float != Float::None {
                continue;
            }

            match child.tag {
                ElementTag::Text => {
                    if let Some(ref text) = child.text {
                        if !text.is_empty() {
                            let processed = preprocess_text_for_shaping(text, &child.style);
                            if !processed.is_empty() {
                                has_content = true;
                                let font_desc = style_to_font_description(&child.style);
                                let font = Font::new(font_desc);

                                // For white-space modes that preserve newlines,
                                // split on \n so each line is measured separately.
                                let preserves_nl = matches!(
                                    child.style.white_space,
                                    WhiteSpace::Pre
                                        | WhiteSpace::PreWrap
                                        | WhiteSpace::BreakSpaces
                                        | WhiteSpace::PreLine
                                );
                                if preserves_nl && processed.contains('\n') {
                                    let lines: Vec<&str> = processed.split('\n').collect();
                                    for (i, line) in lines.iter().enumerate() {
                                        if !line.is_empty() {
                                            let sr = shaper.shape(line, &font, TextDirection::Ltr);
                                            current_inline_row += sr.width();
                                        }
                                        // Flush at newline boundary (except after the last segment)
                                        if i < lines.len() - 1 {
                                            max_width = max_width.max(current_inline_row);
                                            current_inline_row = 0.0;
                                        }
                                    }
                                } else {
                                    let sr = shaper.shape(&processed, &font, TextDirection::Ltr);
                                    current_inline_row += sr.width();
                                }
                            }
                        }
                    }
                }
                _ => {
                    let child_style = &child.style;

                    // Compute the child's own border+padding contribution.
                    let bp_left = child_style.effective_border_left() as f32
                        + resolve_margin_or_padding(
                            &child_style.padding_left,
                            openui_geometry::LayoutUnit::zero(),
                        )
                        .to_f32();
                    let bp_right = child_style.effective_border_right() as f32
                        + resolve_margin_or_padding(
                            &child_style.padding_right,
                            openui_geometry::LayoutUnit::zero(),
                        )
                        .to_f32();
                    let child_bp = bp_left + bp_right;

                    // Non-zero border+padding is itself content even if the
                    // descendant has no text or children.
                    if child_bp > 0.0 {
                        has_content = true;
                    }

                    if child_style.display.is_block_level() {
                        // Flush current inline row before block child.
                        max_width = max_width.max(current_inline_row);
                        current_inline_row = 0.0;

                        // Block-level children stack vertically → take max width.
                        let child_total = if child_style.width.length_type()
                            == openui_geometry::LengthType::Fixed
                        {
                            has_content = true;
                            child_style.width.value() + child_bp
                        } else {
                            let (child_width, child_has_content) =
                                self.compute_intrinsic_inline_size_recursive(child_id);
                            if child_has_content {
                                has_content = true;
                            }
                            child_width + child_bp
                        };
                        max_width = max_width.max(child_total);
                    } else {
                        // Inline-level children flow horizontally → sum widths.
                        if child_style.width.length_type()
                            == openui_geometry::LengthType::Fixed
                        {
                            has_content = true;
                            current_inline_row += child_style.width.value() + child_bp;
                        } else {
                            let (child_width, child_has_content) =
                                self.compute_intrinsic_inline_size_recursive(child_id);
                            if child_has_content {
                                has_content = true;
                            }
                            // Always add border+padding; add child_width only if child has content.
                            let contrib = if child_has_content { child_width + child_bp } else { child_bp };
                            current_inline_row += contrib;
                        }
                    }
                }
            }
        }

        // Flush final inline row.
        max_width = max_width.max(current_inline_row);

        (max_width, has_content)
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
            intrinsic_inline_size: None,
        });
    }
}

// ── White-space processing (CSS Text Module Level 3 §4) ─────────────────

/// Process text according to the CSS `white-space` property.
/// Returns true if the white-space mode collapses adjacent spaces.
fn is_collapsible_ws_mode(ws: WhiteSpace) -> bool {
    matches!(ws, WhiteSpace::Normal | WhiteSpace::Nowrap | WhiteSpace::PreLine)
}

/// Apply the same text preprocessing as real inline layout:
/// 1. text-transform (capitalize, uppercase, lowercase)
/// 2. white-space processing (collapse, preserve, etc.)
/// 3. Tab expansion for pre-like modes
///
/// Shared by `append_text` (real layout) and `compute_intrinsic_inline_size_recursive`
/// (intrinsic sizing) so that measured widths match rendered output.
pub fn preprocess_text_for_shaping(text: &str, style: &ComputedStyle) -> String {
    if text.is_empty() {
        return String::new();
    }

    // Step 1: text-transform
    let transformed = if style.text_transform != TextTransform::None {
        apply_text_transform(text, style.text_transform)
    } else {
        text.to_string()
    };

    // Step 2: white-space processing
    let processed = process_white_space(&transformed, style.white_space);

    // Step 3: tab expansion for pre-like modes
    if matches!(
        style.white_space,
        WhiteSpace::Pre | WhiteSpace::PreWrap | WhiteSpace::BreakSpaces
    ) {
        let font_desc = style_to_font_description(style);
        let font = Font::new(font_desc);
        let space_advance = font.width(" ");
        let font_clone = font;
        expand_tabs(&processed, &style.tab_size, space_advance, |ch| {
            let mut buf = [0u8; 4];
            font_clone.width(ch.encode_utf8(&mut buf))
        })
    } else {
        processed
    }
}

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

/// Expand tab characters to spaces according to the CSS `tab-size` property.
///
/// Tab stops are computed from a running advance width. For non-tab characters,
/// the `char_width` callback returns the actual shaped advance (or falls back
/// to `space_advance`), so that proportional fonts produce correct tab stops.
pub fn expand_tabs<F>(
    text: &str,
    tab_size: &TabSize,
    space_advance: f32,
    char_width: F,
) -> String
where
    F: Fn(char) -> f32,
{
    if !text.contains('\t') {
        return text.to_string();
    }
    let space_adv = if space_advance > 0.0 { space_advance } else { 1.0 };
    let tab_interval = match *tab_size {
        TabSize::Spaces(n) => (n.max(1) as f32) * space_adv,
        TabSize::Length(len) => if len > 0.0 { len } else { 8.0 * space_adv },
    };
    let mut result = String::with_capacity(text.len());
    let mut current_advance = 0.0f32;
    for ch in text.chars() {
        if ch == '\t' {
            // Compute next tab stop position.
            let next_stop = ((current_advance / tab_interval).floor() + 1.0) * tab_interval;
            let mut tab_width = next_stop - current_advance;
            // If the tab would be narrower than one space, jump to the next stop.
            if tab_width < space_adv {
                tab_width += tab_interval;
            }
            let num_spaces = (tab_width / space_adv).round().max(1.0) as usize;
            for _ in 0..num_spaces {
                result.push(' ');
            }
            current_advance += num_spaces as f32 * space_adv;
        } else if ch == '\n' {
            result.push(ch);
            current_advance = 0.0;
        } else {
            result.push(ch);
            current_advance += char_width(ch);
        }
    }
    result
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
/// Spaces immediately after a newline are also stripped (they are leading
/// spaces on the new line and are collapsible per §4.1.1).
pub fn collapse_spaces_preserve_newlines(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_space_run = false;
    let mut after_newline = false;
    for ch in text.chars() {
        if ch == '\n' {
            // Preserve newlines; reset space tracking.
            // Remove any trailing space we just added before the newline.
            if result.ends_with(' ') {
                result.pop();
            }
            result.push('\n');
            in_space_run = false;
            after_newline = true;
        } else if ch == ' ' || ch == '\t' || ch == '\r' || ch == '\x0C' {
            if after_newline {
                // Skip spaces immediately after a newline (CSS Text §4.1.1).
                continue;
            }
            if !in_space_run {
                result.push(' ');
                in_space_run = true;
            }
        } else {
            result.push(ch);
            in_space_run = false;
            after_newline = false;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_dom::{Document, ElementTag};
    use openui_text::TextDirection;

    // ── SP11 Round 11 Issue 1B: display:none / out-of-flow skipped in IFC ──

    #[test]
    fn display_none_child_not_collected_in_ifc() {
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("Hello".to_string());
        doc.append_child(container, text);

        let hidden = doc.create_node(ElementTag::Span);
        doc.node_mut(hidden).style.display = Display::None;
        doc.append_child(container, hidden);

        let data = InlineItemsBuilder::collect(&doc, container);
        let has_hidden_items = data.items.iter().any(|item| item.node_id == hidden);
        assert!(
            !has_hidden_items,
            "display:none child should not produce inline items"
        );
    }

    #[test]
    fn out_of_flow_child_not_collected_in_ifc() {
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("Hello".to_string());
        doc.append_child(container, text);

        let abs_span = doc.create_node(ElementTag::Span);
        doc.node_mut(abs_span).style.position = openui_style::Position::Absolute;
        doc.append_child(container, abs_span);

        let data = InlineItemsBuilder::collect(&doc, container);
        let has_abs_items = data.items.iter().any(|item| item.node_id == abs_span);
        assert!(
            !has_abs_items,
            "Out-of-flow child should not produce inline items"
        );
    }

    // ── SP11 Round 11 Issue 2: Atomic inline bidi level ──

    #[test]
    fn atomic_inline_gets_bidi_level_from_rtl_context() {
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("\u{0627}\u{0644}\u{0639}\u{0631}\u{0628}".to_string());
        doc.append_child(container, text);

        let inline_block = doc.create_node(ElementTag::Span);
        doc.node_mut(inline_block).style.display = Display::InlineBlock;
        doc.node_mut(inline_block).style.width = openui_geometry::Length::px(50.0);
        doc.append_child(container, inline_block);

        let mut data = InlineItemsBuilder::collect(&doc, container);
        data.apply_bidi(TextDirection::Rtl);

        let atomic = data.items.iter().find(|i| i.item_type == InlineItemType::AtomicInline);
        assert!(
            atomic.is_some(),
            "Should have an AtomicInline item"
        );
        let atomic = atomic.unwrap();
        assert!(
            atomic.bidi_level % 2 == 1,
            "Atomic inline in RTL context should have odd bidi level, got {}",
            atomic.bidi_level,
        );
    }

    // ── SP11 Round 15 Issue 2: unicode-bidi control character injection ──

    #[test]
    fn unicode_bidi_embed_ltr_affects_bidi_level() {
        // <div dir=ltr> <span unicode-bidi=embed dir=rtl> neutral text </span> </div>
        // With embed + RTL, neutral/weak characters inside should get an RTL
        // bidi level. Strong LTR chars may remain LTR per UAX#9 rules.
        // We use digits (weak) which are affected by embedding direction.
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.node_mut(container).style.direction = Direction::Ltr;
        doc.append_child(vp, container);

        let span = doc.create_node(ElementTag::Span);
        doc.node_mut(span).style.display = Display::Inline;
        doc.node_mut(span).style.unicode_bidi = UnicodeBidi::Embed;
        doc.node_mut(span).style.direction = Direction::Rtl;
        doc.append_child(container, span);

        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("123".to_string());
        doc.node_mut(text).style.direction = Direction::Rtl;
        doc.append_child(span, text);

        let mut data = InlineItemsBuilder::collect(&doc, container);
        data.apply_bidi(TextDirection::Ltr);

        // The text inside the embed+RTL span should have a non-zero bidi level,
        // indicating the embedding was applied. Exact level depends on UAX#9
        // resolution but should be > 0.
        let text_item = data.items.iter().find(|i| i.item_type == InlineItemType::Text);
        assert!(
            text_item.is_some(),
            "Should have a Text item"
        );
        let text_item = text_item.unwrap();
        assert!(
            text_item.bidi_level > 0,
            "Text inside unicode-bidi:embed+RTL should have non-zero bidi level, got {}",
            text_item.bidi_level,
        );
    }

    #[test]
    fn unicode_bidi_override_forces_direction() {
        // <div dir=ltr> <span unicode-bidi=bidi-override dir=rtl> abc </span> </div>
        // With override + RTL, ALL text should be forced RTL (odd bidi level).
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.node_mut(container).style.direction = Direction::Ltr;
        doc.append_child(vp, container);

        let span = doc.create_node(ElementTag::Span);
        doc.node_mut(span).style.display = Display::Inline;
        doc.node_mut(span).style.unicode_bidi = UnicodeBidi::Override;
        doc.node_mut(span).style.direction = Direction::Rtl;
        doc.append_child(container, span);

        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("abc".to_string());
        doc.node_mut(text).style.direction = Direction::Rtl;
        doc.append_child(span, text);

        let mut data = InlineItemsBuilder::collect(&doc, container);
        data.apply_bidi(TextDirection::Ltr);

        // All text items should have RTL level (override forces it).
        for item in &data.items {
            if item.item_type == InlineItemType::Text {
                assert!(
                    item.bidi_level % 2 == 1,
                    "Text inside unicode-bidi:override+RTL should have odd bidi level, got {}",
                    item.bidi_level,
                );
            }
        }
    }

    #[test]
    fn unicode_bidi_normal_does_not_inject_control_chars() {
        // With unicode-bidi: normal, no control chars should be injected.
        let chars = bidi_open_chars(UnicodeBidi::Normal, Direction::Ltr);
        assert!(chars.is_empty(), "Normal should produce no open chars");
        let chars = bidi_close_chars(UnicodeBidi::Normal);
        assert!(chars.is_empty(), "Normal should produce no close chars");
    }

    #[test]
    fn unicode_bidi_isolate_override_injects_correct_chars() {
        // IsolateOverride + LTR should inject LRI + LRO on open, PDF + PDI on close.
        let open = bidi_open_chars(UnicodeBidi::IsolateOverride, Direction::Ltr);
        assert_eq!(open, vec!['\u{2066}', '\u{202D}'], "LTR isolate-override: LRI + LRO");

        let close = bidi_close_chars(UnicodeBidi::IsolateOverride);
        assert_eq!(close, vec!['\u{202C}', '\u{2069}'], "isolate-override close: PDF + PDI");

        // IsolateOverride + RTL should inject RLI + RLO on open.
        let open_rtl = bidi_open_chars(UnicodeBidi::IsolateOverride, Direction::Rtl);
        assert_eq!(open_rtl, vec!['\u{2067}', '\u{202E}'], "RTL isolate-override: RLI + RLO");
    }

    // ── Issue 4 (R26): intrinsic sizing skips out-of-flow & display:none ──

    #[test]
    fn intrinsic_size_skips_display_none() {
        // A child with display:none should not contribute to intrinsic size.
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let visible_text = doc.create_node(ElementTag::Text);
        doc.node_mut(visible_text).text = Some("Hello".to_string());
        doc.append_child(container, visible_text);

        let hidden = doc.create_node(ElementTag::Span);
        doc.node_mut(hidden).style.display = Display::None;
        doc.append_child(container, hidden);

        let hidden_text = doc.create_node(ElementTag::Text);
        doc.node_mut(hidden_text).text = Some("This is hidden and very long text that should not count".to_string());
        doc.append_child(hidden, hidden_text);

        let builder = InlineItemsBuilder::new(&doc);
        let size_with_hidden = builder.compute_intrinsic_inline_size(container);

        // Now test without hidden: the size should be the same since display:none
        // children are skipped.
        let mut doc2 = Document::new();
        let vp2 = doc2.root();
        let container2 = doc2.create_node(ElementTag::Div);
        doc2.node_mut(container2).style.display = Display::Block;
        doc2.append_child(vp2, container2);

        let text2 = doc2.create_node(ElementTag::Text);
        doc2.node_mut(text2).text = Some("Hello".to_string());
        doc2.append_child(container2, text2);

        let builder2 = InlineItemsBuilder::new(&doc2);
        let size_without = builder2.compute_intrinsic_inline_size(container2);

        assert_eq!(
            size_with_hidden, size_without,
            "display:none child should not affect intrinsic size"
        );
    }

    #[test]
    fn intrinsic_size_skips_absolutely_positioned() {
        // An absolutely positioned child should not contribute to intrinsic size.
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let text_node = doc.create_node(ElementTag::Text);
        doc.node_mut(text_node).text = Some("A".to_string());
        doc.append_child(container, text_node);

        let abs_child = doc.create_node(ElementTag::Div);
        doc.node_mut(abs_child).style.position = openui_style::Position::Absolute;
        doc.node_mut(abs_child).style.width = openui_geometry::Length::px(999.0);
        doc.append_child(container, abs_child);

        let builder = InlineItemsBuilder::new(&doc);
        let size = builder.compute_intrinsic_inline_size(container);

        // Without the absolute child, we should get the same size.
        let mut doc2 = Document::new();
        let vp2 = doc2.root();
        let container2 = doc2.create_node(ElementTag::Div);
        doc2.node_mut(container2).style.display = Display::Block;
        doc2.append_child(vp2, container2);

        let text2 = doc2.create_node(ElementTag::Text);
        doc2.node_mut(text2).text = Some("A".to_string());
        doc2.append_child(container2, text2);

        let builder2 = InlineItemsBuilder::new(&doc2);
        let size_without = builder2.compute_intrinsic_inline_size(container2);

        assert_eq!(
            size, size_without,
            "absolutely positioned child should not affect intrinsic size"
        );
    }

    #[test]
    fn intrinsic_size_skips_floated_child() {
        // A floated child should not contribute to intrinsic size.
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let text_node = doc.create_node(ElementTag::Text);
        doc.node_mut(text_node).text = Some("B".to_string());
        doc.append_child(container, text_node);

        let floated = doc.create_node(ElementTag::Div);
        doc.node_mut(floated).style.float = openui_style::Float::Left;
        doc.node_mut(floated).style.width = openui_geometry::Length::px(500.0);
        doc.append_child(container, floated);

        let builder = InlineItemsBuilder::new(&doc);
        let size = builder.compute_intrinsic_inline_size(container);

        let mut doc2 = Document::new();
        let vp2 = doc2.root();
        let container2 = doc2.create_node(ElementTag::Div);
        doc2.node_mut(container2).style.display = Display::Block;
        doc2.append_child(vp2, container2);

        let text2 = doc2.create_node(ElementTag::Text);
        doc2.node_mut(text2).text = Some("B".to_string());
        doc2.append_child(container2, text2);

        let builder2 = InlineItemsBuilder::new(&doc2);
        let size_without = builder2.compute_intrinsic_inline_size(container2);

        assert_eq!(
            size, size_without,
            "floated child should not affect intrinsic size"
        );
    }

    // ── Issue 5 (R26): intrinsic sizing applies text preprocessing ──────

    #[test]
    fn intrinsic_size_applies_text_transform() {
        // "abc" with text-transform:uppercase → "ABC".
        // The measured width should differ since uppercase chars are typically wider.
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("iii".to_string()); // 'i' is narrow
        doc.node_mut(text).style.text_transform = openui_style::TextTransform::Uppercase;
        doc.append_child(container, text);

        let builder = InlineItemsBuilder::new(&doc);
        let size_upper = builder.compute_intrinsic_inline_size(container);

        // Compare with no transform
        let mut doc2 = Document::new();
        let vp2 = doc2.root();
        let container2 = doc2.create_node(ElementTag::Div);
        doc2.node_mut(container2).style.display = Display::Block;
        doc2.append_child(vp2, container2);

        let text2 = doc2.create_node(ElementTag::Text);
        doc2.node_mut(text2).text = Some("iii".to_string());
        doc2.node_mut(text2).style.text_transform = openui_style::TextTransform::None;
        doc2.append_child(container2, text2);

        let builder2 = InlineItemsBuilder::new(&doc2);
        let size_normal = builder2.compute_intrinsic_inline_size(container2);

        // The uppercase version ("III") should be wider than lowercase ("iii").
        assert!(
            size_upper.unwrap_or(0.0) >= size_normal.unwrap_or(0.0),
            "uppercase text should be at least as wide: upper={:?}, normal={:?}",
            size_upper, size_normal,
        );
    }

    #[test]
    fn intrinsic_size_collapses_whitespace_in_normal_mode() {
        // "a   b" with white-space:normal collapses to "a b".
        // Should match the width of pre-collapsed "a b".
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("a     b".to_string());
        doc.node_mut(text).style.white_space = WhiteSpace::Normal;
        doc.append_child(container, text);

        let builder = InlineItemsBuilder::new(&doc);
        let size_collapsed = builder.compute_intrinsic_inline_size(container);

        let mut doc2 = Document::new();
        let vp2 = doc2.root();
        let container2 = doc2.create_node(ElementTag::Div);
        doc2.node_mut(container2).style.display = Display::Block;
        doc2.append_child(vp2, container2);

        let text2 = doc2.create_node(ElementTag::Text);
        doc2.node_mut(text2).text = Some("a b".to_string());
        doc2.node_mut(text2).style.white_space = WhiteSpace::Normal;
        doc2.append_child(container2, text2);

        let builder2 = InlineItemsBuilder::new(&doc2);
        let size_single_space = builder2.compute_intrinsic_inline_size(container2);

        assert_eq!(
            size_collapsed, size_single_space,
            "collapsed 'a     b' should have same width as 'a b'"
        );
    }

    // ── SP11 Round 27 Issue 5: block-level descendants use max (not sum) ──

    #[test]
    fn intrinsic_size_flex_children_use_max_not_sum() {
        // Two Flex children (100px, 200px) should take max=200, not sum=300.
        let mut doc = Document::new();
        let vp = doc.root();
        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let child1 = doc.create_node(ElementTag::Div);
        doc.node_mut(child1).style.display = Display::Flex;
        doc.node_mut(child1).style.width = openui_geometry::Length::px(100.0);
        doc.append_child(container, child1);

        let child2 = doc.create_node(ElementTag::Div);
        doc.node_mut(child2).style.display = Display::Flex;
        doc.node_mut(child2).style.width = openui_geometry::Length::px(200.0);
        doc.append_child(container, child2);

        let builder = InlineItemsBuilder::new(&doc);
        let size = builder.compute_intrinsic_inline_size(container);
        assert!(size.is_some(), "Should have intrinsic size");
        let width = size.unwrap();
        assert!(
            (width - 200.0).abs() < 0.01,
            "Flex block-level children should use max width: expected ~200.0, got {}",
            width
        );
    }

    #[test]
    fn intrinsic_size_grid_children_use_max_not_sum() {
        let mut doc = Document::new();
        let vp = doc.root();
        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let child1 = doc.create_node(ElementTag::Div);
        doc.node_mut(child1).style.display = Display::Grid;
        doc.node_mut(child1).style.width = openui_geometry::Length::px(150.0);
        doc.append_child(container, child1);

        let child2 = doc.create_node(ElementTag::Div);
        doc.node_mut(child2).style.display = Display::Grid;
        doc.node_mut(child2).style.width = openui_geometry::Length::px(250.0);
        doc.append_child(container, child2);

        let builder = InlineItemsBuilder::new(&doc);
        let size = builder.compute_intrinsic_inline_size(container);
        assert!(size.is_some());
        let width = size.unwrap();
        assert!(
            (width - 250.0).abs() < 0.01,
            "Grid block-level children should use max width: expected ~250.0, got {}",
            width
        );
    }

    #[test]
    fn intrinsic_size_flow_root_children_use_max() {
        let mut doc = Document::new();
        let vp = doc.root();
        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let child1 = doc.create_node(ElementTag::Div);
        doc.node_mut(child1).style.display = Display::FlowRoot;
        doc.node_mut(child1).style.width = openui_geometry::Length::px(80.0);
        doc.append_child(container, child1);

        let child2 = doc.create_node(ElementTag::Div);
        doc.node_mut(child2).style.display = Display::FlowRoot;
        doc.node_mut(child2).style.width = openui_geometry::Length::px(120.0);
        doc.append_child(container, child2);

        let builder = InlineItemsBuilder::new(&doc);
        let size = builder.compute_intrinsic_inline_size(container);
        assert!(size.is_some());
        let width = size.unwrap();
        assert!(
            (width - 120.0).abs() < 0.01,
            "FlowRoot block-level children should use max: expected ~120.0, got {}",
            width
        );
    }

    #[test]
    fn intrinsic_size_inline_block_children_sum() {
        // Inline-level (InlineBlock) children flow horizontally → sum widths.
        let mut doc = Document::new();
        let vp = doc.root();
        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let child1 = doc.create_node(ElementTag::Div);
        doc.node_mut(child1).style.display = Display::InlineBlock;
        doc.node_mut(child1).style.width = openui_geometry::Length::px(100.0);
        doc.append_child(container, child1);

        let child2 = doc.create_node(ElementTag::Div);
        doc.node_mut(child2).style.display = Display::InlineBlock;
        doc.node_mut(child2).style.width = openui_geometry::Length::px(200.0);
        doc.append_child(container, child2);

        let builder = InlineItemsBuilder::new(&doc);
        let size = builder.compute_intrinsic_inline_size(container);
        assert!(size.is_some());
        let width = size.unwrap();
        assert!(
            (width - 300.0).abs() < 0.01,
            "InlineBlock children should sum widths: expected ~300.0, got {}",
            width
        );
    }

    // ── Issue 4 (R28): Intrinsic sizing overcounts inline content around block ──

    #[test]
    fn intrinsic_size_inline_around_block_uses_max_not_sum() {
        // inline(50px) + block(100px) + inline(50px)
        // Should be max(50, 100, 50) = 100, NOT 50+100+50 = 200.
        let mut doc = Document::new();
        let vp = doc.root();
        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let inline1 = doc.create_node(ElementTag::Div);
        doc.node_mut(inline1).style.display = Display::InlineBlock;
        doc.node_mut(inline1).style.width = openui_geometry::Length::px(50.0);
        doc.append_child(container, inline1);

        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.node_mut(block).style.width = openui_geometry::Length::px(100.0);
        doc.append_child(container, block);

        let inline2 = doc.create_node(ElementTag::Div);
        doc.node_mut(inline2).style.display = Display::InlineBlock;
        doc.node_mut(inline2).style.width = openui_geometry::Length::px(50.0);
        doc.append_child(container, inline2);

        let builder = InlineItemsBuilder::new(&doc);
        let size = builder.compute_intrinsic_inline_size(container);
        assert!(size.is_some());
        let width = size.unwrap();
        assert!(
            (width - 100.0).abs() < 0.01,
            "inline(50)+block(100)+inline(50) should be max(50,100,50)=100, got {}",
            width
        );
    }

    #[test]
    fn intrinsic_size_inline_row_flushed_before_block() {
        // inline(80px) + inline(70px) + block(60px) + inline(30px)
        // Inline row 1: 80+70=150, block: 60, inline row 2: 30
        // max(150, 60, 30) = 150
        let mut doc = Document::new();
        let vp = doc.root();
        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let inline1 = doc.create_node(ElementTag::Div);
        doc.node_mut(inline1).style.display = Display::InlineBlock;
        doc.node_mut(inline1).style.width = openui_geometry::Length::px(80.0);
        doc.append_child(container, inline1);

        let inline2 = doc.create_node(ElementTag::Div);
        doc.node_mut(inline2).style.display = Display::InlineBlock;
        doc.node_mut(inline2).style.width = openui_geometry::Length::px(70.0);
        doc.append_child(container, inline2);

        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.node_mut(block).style.width = openui_geometry::Length::px(60.0);
        doc.append_child(container, block);

        let inline3 = doc.create_node(ElementTag::Div);
        doc.node_mut(inline3).style.display = Display::InlineBlock;
        doc.node_mut(inline3).style.width = openui_geometry::Length::px(30.0);
        doc.append_child(container, inline3);

        let builder = InlineItemsBuilder::new(&doc);
        let size = builder.compute_intrinsic_inline_size(container);
        assert!(size.is_some());
        let width = size.unwrap();
        assert!(
            (width - 150.0).abs() < 0.01,
            "inline(80)+inline(70)+block(60)+inline(30) should be max(150,60,30)=150, got {}",
            width
        );
    }

    #[test]
    fn intrinsic_size_block_wider_than_inline_rows() {
        // inline(30px) + block(200px) + inline(40px)
        // max(30, 200, 40) = 200
        let mut doc = Document::new();
        let vp = doc.root();
        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let inline1 = doc.create_node(ElementTag::Div);
        doc.node_mut(inline1).style.display = Display::InlineBlock;
        doc.node_mut(inline1).style.width = openui_geometry::Length::px(30.0);
        doc.append_child(container, inline1);

        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.node_mut(block).style.width = openui_geometry::Length::px(200.0);
        doc.append_child(container, block);

        let inline2 = doc.create_node(ElementTag::Div);
        doc.node_mut(inline2).style.display = Display::InlineBlock;
        doc.node_mut(inline2).style.width = openui_geometry::Length::px(40.0);
        doc.append_child(container, inline2);

        let builder = InlineItemsBuilder::new(&doc);
        let size = builder.compute_intrinsic_inline_size(container);
        assert!(size.is_some());
        let width = size.unwrap();
        assert!(
            (width - 200.0).abs() < 0.01,
            "inline(30)+block(200)+inline(40) should be max(30,200,40)=200, got {}",
            width
        );
    }

    // ── SP11 Round 29 Issue 4: intrinsic sizing splits on preserved newlines ──

    #[test]
    fn intrinsic_size_pre_splits_on_newlines() {
        // With white-space: pre, "AAA\nB" should split into two lines.
        // The intrinsic width should be max(width("AAA"), width("B")), not width("AAA\nB").
        let mut doc = Document::new();
        let vp = doc.root();
        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("AAA\nB".to_string());
        doc.node_mut(text).style.white_space = WhiteSpace::Pre;
        doc.append_child(container, text);

        let builder = InlineItemsBuilder::new(&doc);
        let size = builder.compute_intrinsic_inline_size(container);
        assert!(size.is_some(), "Pre text with newlines should have content");

        // Also measure "AAA" as a single line.
        let mut doc2 = Document::new();
        let vp2 = doc2.root();
        let container2 = doc2.create_node(ElementTag::Div);
        doc2.node_mut(container2).style.display = Display::Block;
        doc2.append_child(vp2, container2);

        let text2 = doc2.create_node(ElementTag::Text);
        doc2.node_mut(text2).text = Some("AAA".to_string());
        doc2.node_mut(text2).style.white_space = WhiteSpace::Pre;
        doc2.append_child(container2, text2);

        let builder2 = InlineItemsBuilder::new(&doc2);
        let size_aaa = builder2.compute_intrinsic_inline_size(container2);

        assert_eq!(
            size.unwrap(),
            size_aaa.unwrap(),
            "Pre text 'AAA\\nB' intrinsic width should equal width of 'AAA' (widest line)"
        );
    }

    #[test]
    fn intrinsic_size_preline_splits_on_newlines() {
        // pre-line preserves newlines but collapses spaces.
        let mut doc = Document::new();
        let vp = doc.root();
        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("AAAA\nBB".to_string());
        doc.node_mut(text).style.white_space = WhiteSpace::PreLine;
        doc.append_child(container, text);

        let builder = InlineItemsBuilder::new(&doc);
        let size = builder.compute_intrinsic_inline_size(container);

        // Measure just "AAAA" for comparison.
        let mut doc2 = Document::new();
        let vp2 = doc2.root();
        let container2 = doc2.create_node(ElementTag::Div);
        doc2.node_mut(container2).style.display = Display::Block;
        doc2.append_child(vp2, container2);

        let text2 = doc2.create_node(ElementTag::Text);
        doc2.node_mut(text2).text = Some("AAAA".to_string());
        doc2.node_mut(text2).style.white_space = WhiteSpace::PreLine;
        doc2.append_child(container2, text2);

        let builder2 = InlineItemsBuilder::new(&doc2);
        let size_aaaa = builder2.compute_intrinsic_inline_size(container2);

        assert_eq!(
            size.unwrap(),
            size_aaaa.unwrap(),
            "pre-line 'AAAA\\nBB' should split → max(width('AAAA'), width('BB')) = width('AAAA')"
        );
    }

    #[test]
    fn intrinsic_size_normal_does_not_split_on_newlines() {
        // white-space: normal collapses newlines to spaces — no split.
        let mut doc = Document::new();
        let vp = doc.root();
        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("A\nB".to_string());
        doc.node_mut(text).style.white_space = WhiteSpace::Normal;
        doc.append_child(container, text);

        let builder = InlineItemsBuilder::new(&doc);
        let size = builder.compute_intrinsic_inline_size(container);

        // Also measure "A B" (collapsed form).
        let mut doc2 = Document::new();
        let vp2 = doc2.root();
        let container2 = doc2.create_node(ElementTag::Div);
        doc2.node_mut(container2).style.display = Display::Block;
        doc2.append_child(vp2, container2);

        let text2 = doc2.create_node(ElementTag::Text);
        doc2.node_mut(text2).text = Some("A B".to_string());
        doc2.node_mut(text2).style.white_space = WhiteSpace::Normal;
        doc2.append_child(container2, text2);

        let builder2 = InlineItemsBuilder::new(&doc2);
        let size_collapsed = builder2.compute_intrinsic_inline_size(container2);

        assert_eq!(
            size.unwrap(),
            size_collapsed.unwrap(),
            "Normal mode 'A\\nB' should collapse to 'A B', not split lines"
        );
    }

    // ── SP11 Round 29 Issue 5: empty descendants with border/padding ──────

    #[test]
    fn intrinsic_size_empty_span_with_border_counted() {
        // An empty inline span with border should contribute to intrinsic width.
        let mut doc = Document::new();
        let vp = doc.root();
        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let empty_span = doc.create_node(ElementTag::Span);
        doc.node_mut(empty_span).style.display = Display::Inline;
        doc.node_mut(empty_span).style.border_left_width = 5;
        doc.node_mut(empty_span).style.border_left_style = openui_style::BorderStyle::Solid;
        doc.node_mut(empty_span).style.border_right_width = 5;
        doc.node_mut(empty_span).style.border_right_style = openui_style::BorderStyle::Solid;
        doc.append_child(container, empty_span);

        let builder = InlineItemsBuilder::new(&doc);
        let size = builder.compute_intrinsic_inline_size(container);

        assert!(
            size.is_some(),
            "Empty span with border should be treated as having content"
        );
        assert!(
            size.unwrap() >= 10.0,
            "Width should include border: expected >= 10.0, got {}",
            size.unwrap()
        );
    }

    #[test]
    fn intrinsic_size_empty_span_with_padding_counted() {
        // An empty inline span with padding should contribute to intrinsic width.
        let mut doc = Document::new();
        let vp = doc.root();
        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let empty_span = doc.create_node(ElementTag::Span);
        doc.node_mut(empty_span).style.display = Display::Inline;
        doc.node_mut(empty_span).style.padding_left = openui_geometry::Length::px(8.0);
        doc.node_mut(empty_span).style.padding_right = openui_geometry::Length::px(8.0);
        doc.append_child(container, empty_span);

        let builder = InlineItemsBuilder::new(&doc);
        let size = builder.compute_intrinsic_inline_size(container);

        assert!(
            size.is_some(),
            "Empty span with padding should be treated as having content"
        );
        assert!(
            size.unwrap() >= 16.0,
            "Width should include padding: expected >= 16.0, got {}",
            size.unwrap()
        );
    }

    #[test]
    fn intrinsic_size_empty_block_with_border_counted() {
        // An empty block-level child with border should contribute to intrinsic width.
        let mut doc = Document::new();
        let vp = doc.root();
        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let empty_block = doc.create_node(ElementTag::Div);
        doc.node_mut(empty_block).style.display = Display::Block;
        doc.node_mut(empty_block).style.border_left_width = 10;
        doc.node_mut(empty_block).style.border_left_style = openui_style::BorderStyle::Solid;
        doc.node_mut(empty_block).style.border_right_width = 10;
        doc.node_mut(empty_block).style.border_right_style = openui_style::BorderStyle::Solid;
        doc.append_child(container, empty_block);

        let builder = InlineItemsBuilder::new(&doc);
        let size = builder.compute_intrinsic_inline_size(container);

        assert!(
            size.is_some(),
            "Empty block with border should be treated as having content"
        );
        assert!(
            size.unwrap() >= 20.0,
            "Width should include block borders: expected >= 20.0, got {}",
            size.unwrap()
        );
    }
}

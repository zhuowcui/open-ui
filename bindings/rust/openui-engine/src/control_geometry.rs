//! Intrinsic geometry owned by native text controls.
//!
//! Font metrics and attributes supply natural dimensions before flex, grid,
//! positioning, hit testing and painting share the retained layout. Explicit
//! resource/renderer metadata keeps its own dimensions.

use crate::Engine;
use openui_dom::{ElementTag, FormControlRole, NodeId, ReplacedContent, ReplacedResourceKind};
use openui_geometry::LayoutUnit;
use openui_style::{ComputedStyle, Display, FontFamily, LineHeight, Overflow, ScrollbarWidth};
use openui_text::{Font, FontDescription};

impl Engine {
    pub(crate) fn control_attribute_changes_geometry(&self, node: NodeId, name: &str) -> bool {
        match self.document.node(node).tag {
            ElementTag::Input => matches!(name, "size" | "type"),
            ElementTag::TextArea => matches!(name, "rows" | "cols" | "wrap"),
            _ => false,
        }
    }

    pub(crate) fn adjust_native_control_style(
        &self,
        index: u32,
        node: NodeId,
        style: &mut ComputedStyle,
    ) {
        if self.document.node(node).tag != ElementTag::TextArea
            || !self
                .controls
                .get(&index)
                .is_some_and(|state| state.native_intrinsic_sizing)
        {
            return;
        }
        style.update_derived(|style| {
            // Blink's text-area style adjustment treats visible overflow as
            // auto, including after an author reset. Contents has no box.
            if style.overflow_x == Overflow::Visible {
                style.overflow_x = Overflow::Auto;
            }
            if style.overflow_y == Overflow::Visible {
                style.overflow_y = Overflow::Auto;
            }
            if style.display == Display::Contents {
                style.display = Display::None;
            }
        });
    }

    pub(crate) fn refresh_native_control_geometry(&mut self) {
        let nodes: Vec<_> = self
            .controls
            .iter()
            .filter(|(_, state)| state.native_intrinsic_sizing)
            .filter_map(|(index, _)| self.slots[*index as usize].node.map(|node| (*index, node)))
            .collect();
        for (index, node) in nodes {
            let mut style = self.document.node(node).style.clone();
            self.adjust_native_control_style(index, node, &mut style);
            self.document.install_resolved_style(node, style);
            let metadata = self.native_control_metadata(node);
            self.document.node_mut(node).replaced = metadata;
        }
    }

    fn native_control_metadata(&self, node: NodeId) -> Option<ReplacedContent> {
        let data = self.document.node(node);
        let role = data.form_control?;
        if !matches!(role, FormControlRole::TextInput | FormControlRole::TextArea) {
            return None;
        }
        let style = &data.style;
        let font = self
            .document
            .resolve_font(FontDescription::from_computed_style(style));
        let (average, maximum) = character_widths(&font);
        let metrics = font.font_metrics().copied().unwrap_or_default();
        let normal = openui_text::used_line_height(&metrics, &LineHeight::Normal, style.font_size);
        let mut line = openui_text::used_line_height(&metrics, &style.line_height, style.font_size);
        let thickness = match style.scrollbar_width {
            ScrollbarWidth::Auto => 15.0,
            ScrollbarWidth::Thin => 10.0,
            ScrollbarWidth::None => 0.0,
        };
        let horizontal = style.writing_mode.is_horizontal();
        let (inline, block) = if role == FormControlRole::TextInput {
            // The anonymous single-line editor cannot be shorter than normal.
            // A tall fixed or percentage block size also uses normal leading.
            let height = if horizontal {
                &style.height
            } else {
                &style.width
            };
            if style.font_size >= line.trunc()
                || (height.is_percent() || (height.is_calculated() && height.value() != 0.0))
                || (height.is_fixed() && height.value() > line.trunc())
            {
                line = normal;
            }
            let count = positive_attribute(self.document.attribute(node, "size"), 20);
            (
                ((average * count as f32) + (maximum - average).max(0.0)).ceil(),
                line,
            )
        } else {
            let cols = positive_attribute(self.document.attribute(node, "cols"), 20);
            let rows = positive_attribute(self.document.attribute(node, "rows"), 2);
            let (overflow_inline, overflow_block) = if horizontal {
                (style.overflow_x, style.overflow_y)
            } else {
                (style.overflow_y, style.overflow_x)
            };
            let inline_bar = if matches!(overflow_block, Overflow::Auto | Overflow::Scroll) {
                thickness
            } else {
                0.0
            };
            let block_bar = if overflow_inline == Overflow::Scroll {
                thickness
            } else {
                0.0
            };
            let line = LayoutUnit::from_f32(line).to_f32();
            (
                (average * cols as f32).ceil() + inline_bar,
                line * rows as f32 + block_bar,
            )
        };
        let (width, height) = if horizontal {
            (inline, block)
        } else {
            (block, inline)
        };
        Some(ReplacedContent {
            resource: ReplacedResourceKind::TransparentCanvas,
            intrinsic_width: Some(width),
            intrinsic_height: Some(height),
            intrinsic_ratio: None,
        })
    }
}

// HTML positive integer attributes accept an optional leading plus and stop at
// the first non-digit. Zero, negative and overflowing values use the default.
fn positive_attribute(value: Option<&str>, default: u32) -> u32 {
    let Some(value) = value else {
        return default;
    };
    let value = value.trim_start_matches([' ', '\t', '\n', '\r', '\x0c']);
    let value = value.strip_prefix('+').unwrap_or(value);
    let mut parsed = 0_u32;
    let mut digits = false;
    for byte in value.bytes().take_while(u8::is_ascii_digit) {
        digits = true;
        let Some(next) = parsed
            .checked_mul(10)
            .and_then(|n| n.checked_add((byte - b'0') as u32))
        else {
            return default;
        };
        parsed = next;
    }
    if !digits || parsed == 0 || parsed > i32::MAX as u32 {
        default
    } else {
        parsed
    }
}

// Linux Blink uses the primary face's OS/2 average and rounded outline extent,
// falling back to x/zero advances for missing or historically invalid metrics.
fn character_widths(font: &Font) -> (f32, f32) {
    let Some(primary) = font.primary_font() else {
        return (font.size() * 0.5, 0.0);
    };
    let (_, metrics) = primary.sk_font().metrics();
    let average = if metrics.avg_char_width != 0.0 {
        metrics.avg_char_width
    } else if primary.sk_font().unichar_to_glyph('x' as i32) != 0 {
        font.width("x")
    } else {
        primary.metrics().x_height
    };
    let zero = primary.metrics().zero_width;
    let family = font
        .description()
        .family
        .families
        .first()
        .map(|family| match family {
            FontFamily::Named(name) => name.as_str(),
            FontFamily::Generic(_) => "generic",
        })
        .unwrap_or("");
    let valid = !family.is_empty()
        && !(zero > 0.0 && average > zero * 1.7)
        && !INVALID_AVERAGE_FAMILIES.contains(&family);
    if valid {
        (
            average.max(average.round()),
            (metrics.x_max - metrics.x_min).round(),
        )
    } else {
        (font.width("0"), 0.0)
    }
}

const INVALID_AVERAGE_FAMILIES: &[&str] = &[
    "American Typewriter",
    "Arial Hebrew",
    "Chalkboard",
    "Cochin",
    "Corsiva Hebrew",
    "Courier",
    "Euphemia UCAS",
    "Geneva",
    "Gill Sans",
    "Hei",
    "Helvetica",
    "Hoefler Text",
    "InaiMathi",
    "Kai",
    "Lucida Grande",
    "Marker Felt",
    "Monaco",
    "Mshtakan",
    "New Peninim MT",
    "Osaka",
    "Raanana",
    "STHeiti",
    "Symbol",
    "Times",
    "Apple Braille",
    "Apple LiGothic",
    "Apple LiSung",
    "Apple Symbols",
    "AppleGothic",
    "AppleMyungjo",
    "#GungSeo",
    "#HeadLineA",
    "#PCMyungjo",
    "#PilGi",
];

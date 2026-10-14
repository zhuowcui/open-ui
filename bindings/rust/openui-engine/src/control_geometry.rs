//! Intrinsic geometry owned by native text controls.
//!
//! Font metrics and attributes supply natural dimensions before flex, grid,
//! positioning, hit testing and painting share the retained layout. Explicit
//! resource/renderer metadata keeps its own dimensions.

use crate::{Engine, EngineError, NodeHandle};
use openui_dom::{ElementTag, FormControlRole, NodeId, ReplacedContent, ReplacedResourceKind};
use openui_geometry::LayoutUnit;
use openui_style::{
    CanonicalStyleField, ComputedStyle, Display, FontFamily, LineHeight, Overflow,
    RendererStyleValue, ScrollbarWidth, StyleProperty, StyleValue,
};
use openui_text::{Font, FontDescription};

impl Engine {
    /// Request the existing native intrinsic kernel with an owned standard
    /// scrollbar-width operand from a static capture adapter. This never
    /// replaces explicit renderer/resource geometry or reenables its ownership.
    #[doc(hidden)]
    pub fn request_native_control_geometry(
        &mut self,
        handle: NodeHandle,
        expected_role: FormControlRole,
        scrollbar_width: ScrollbarWidth,
    ) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        let control = self
            .controls
            .get(&handle.index)
            .ok_or(EngineError::NotAControl)?;
        if !matches!(
            expected_role,
            FormControlRole::TextInput | FormControlRole::TextArea
        ) || self.document.node(node).form_control != Some(expected_role)
            || control.role != expected_role
            || !control.native_intrinsic_sizing
        {
            return Err(EngineError::InvalidInput(
                "native geometry requires an owned text-control role",
            ));
        }
        if control.intrinsic_capture_scrollbar_width == Some(scrollbar_width) {
            return Ok(());
        }
        self.controls
            .get_mut(&handle.index)
            .unwrap()
            .intrinsic_capture_scrollbar_width = Some(scrollbar_width);
        self.mark_dirty(openui_style::InvalidationClass::Intrinsic);
        Ok(())
    }

    /// Metadata reads the standard source independently of pseudo-hidden
    /// painting. Canonical mutation/removal and effects retain their shared
    /// publication order; no animated computed snapshot becomes the baseline.
    fn intrinsic_scrollbar_width(&self, node: NodeId) -> ScrollbarWidth {
        let data = self.document.node(node);
        let Some(index) = self.node_slots.get(&node).copied() else {
            return data.style.scrollbar_width;
        };
        let Some(source) = self
            .controls
            .get(&index)
            .and_then(|state| state.intrinsic_capture_scrollbar_width)
        else {
            return data.style.scrollbar_width;
        };
        let slot = &self.slots[index as usize];
        let mut style = data.style.clone();
        style.update_derived(|fields| fields.scrollbar_width = source);
        let viewport = (
            self.viewport.logical_width() as f32,
            self.viewport.logical_height() as f32,
        );
        for contribution in slot.authored.contributions() {
            if contribution
                .fields()
                .contains(CanonicalStyleField::ScrollbarWidth)
            {
                contribution
                    .project_resolved(&mut style, contribution.value(), viewport)
                    .expect("validated scrollbar-width contribution");
            }
        }
        let parent = if data.parent.is_none() {
            ComputedStyle::initial()
        } else {
            self.document.node(data.parent).style.clone()
        };
        let root_font = self.document.node(self.document.root()).style.font_size;
        for (property, value) in
            self.animation_values_for_node(index, &style, &parent, root_font, viewport, false)
        {
            if property == StyleProperty::ScrollbarWidth {
                openui_style::apply_to_computed(&mut style, property, &value, viewport)
                    .expect("validated scrollbar-width effect");
            }
        }
        style.scrollbar_width
    }

    pub(crate) fn commit_intrinsic_capture_width(&mut self, index: u32, value: &StyleValue) {
        if let StyleValue::Renderer(RendererStyleValue::ScrollbarWidth(width)) = value {
            if let Some(control) = self.controls.get_mut(&index) {
                if control.intrinsic_capture_scrollbar_width.is_some() {
                    control.intrinsic_capture_scrollbar_width = Some(*width);
                }
            }
        }
    }

    pub(crate) fn intrinsic_capture_width_matches(&self, index: u32, value: &StyleValue) -> bool {
        match value {
            StyleValue::Renderer(RendererStyleValue::ScrollbarWidth(width)) => self
                .controls
                .get(&index)
                .and_then(|control| control.intrinsic_capture_scrollbar_width)
                .is_none_or(|source| source == *width),
            _ => true,
        }
    }

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
        let thickness = match self.intrinsic_scrollbar_width(node) {
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

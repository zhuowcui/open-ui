//! ::first-line pseudo-element support.
//!
//! CSS 2.1 §5.12.1: The `::first-line` pseudo-element applies special styling
//! to the first formatted line of a block container. Only a restricted subset
//! of properties apply (font, color, text-decoration, letter/word-spacing,
//! line-height, vertical-align, text-transform, background, and clear).
//!
//! Blink: `style_adjuster.cc` — `AdjustStyleForFirstLetter()` and
//! `HighlightPseudoStyle(kPseudoIdFirstLine)`.
//!
//! The first-line style is an overlay: we clone the block container's
//! style and override only the text-related properties that the author
//! specified on `::first-line`.  During inline layout, the first line uses
//! this overlay style for shaping and metrics; subsequent lines use the
//! normal style.

use openui_style::ComputedStyle;

/// The set of CSS properties that `::first-line` may override.
///
/// CSS 2.1 §5.12.1 restricts `::first-line` to font, color,
/// text-decoration, letter-spacing, word-spacing, line-height,
/// vertical-align, text-transform, and background properties.
///
/// This struct captures the overrides from the author style rule so
/// that `apply_first_line_overrides` can merge them into the base style.
#[derive(Debug, Clone)]
pub struct FirstLineOverrides {
    pub font_size: Option<f32>,
    pub font_weight: Option<openui_style::FontWeight>,
    pub font_style: Option<openui_style::FontStyleEnum>,
    pub color: Option<openui_style::Color>,
    pub text_decoration_line: Option<openui_style::TextDecorationLine>,
    pub letter_spacing: Option<f32>,
    pub word_spacing: Option<f32>,
    pub line_height: Option<openui_style::LineHeight>,
    pub text_transform: Option<openui_style::TextTransform>,
    pub vertical_align: Option<openui_style::VerticalAlign>,
}

impl Default for FirstLineOverrides {
    fn default() -> Self {
        Self {
            font_size: None,
            font_weight: None,
            font_style: None,
            color: None,
            text_decoration_line: None,
            letter_spacing: None,
            word_spacing: None,
            line_height: None,
            text_transform: None,
            vertical_align: None,
        }
    }
}

/// Apply `::first-line` overrides to a base style, producing a merged
/// style for the first formatted line.
///
/// CSS 2.1 §5.12.1: only the allowed properties are overridden; all
/// other properties remain as in the base style. The returned style is
/// used for text shaping and metrics on the first line only.
pub fn apply_first_line_overrides(
    base: &ComputedStyle,
    overrides: &FirstLineOverrides,
) -> ComputedStyle {
    let mut style = base.clone();
    if let Some(v) = overrides.font_size {
        style.font_size = v;
    }
    if let Some(v) = overrides.font_weight {
        style.font_weight = v;
    }
    if let Some(v) = overrides.font_style {
        style.font_style = v;
    }
    if let Some(v) = overrides.color {
        style.color = v;
    }
    if let Some(v) = overrides.text_decoration_line {
        style.text_decoration_line = v;
    }
    if let Some(v) = overrides.letter_spacing {
        style.letter_spacing = v;
    }
    if let Some(v) = overrides.word_spacing {
        style.word_spacing = v;
    }
    if let Some(v) = overrides.line_height {
        style.line_height = v;
    }
    if let Some(v) = overrides.text_transform {
        style.text_transform = v;
    }
    if let Some(v) = overrides.vertical_align {
        style.vertical_align = v;
    }
    style
}

/// Resolve first-line style from a block container's ComputedStyle.
///
/// If the block container has a `first_line_style` set, we return that
/// boxed style. Otherwise `None` — meaning the first line uses the
/// same style as all other lines.
#[inline]
pub fn resolve_first_line_style(block_style: &ComputedStyle) -> Option<&ComputedStyle> {
    block_style.first_line_style.as_deref()
}

/// Check whether a property is allowed on `::first-line`.
///
/// CSS 2.1 §5.12.1 restricts first-line to: font properties, color,
/// text-decoration, letter-spacing, word-spacing, line-height,
/// vertical-align, text-transform, and background properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstLineProperty {
    FontSize,
    FontWeight,
    FontStyle,
    Color,
    TextDecorationLine,
    LetterSpacing,
    WordSpacing,
    LineHeight,
    TextTransform,
    VerticalAlign,
}

impl FirstLineProperty {
    /// All properties that `::first-line` is allowed to override.
    pub const ALL: &'static [Self] = &[
        Self::FontSize,
        Self::FontWeight,
        Self::FontStyle,
        Self::Color,
        Self::TextDecorationLine,
        Self::LetterSpacing,
        Self::WordSpacing,
        Self::LineHeight,
        Self::TextTransform,
        Self::VerticalAlign,
    ];
}

/// Given a block style with a `::first-line` override, extract the
/// effective style for the first line.
///
/// This is the primary API used by the inline layout algorithm. On the
/// first line it calls this function; for subsequent lines it uses the
/// base style directly.
///
/// When `block_style.first_line_style` is `None`, this returns a clone
/// of the block style unchanged (the compiler should elide the clone
/// when the caller immediately borrows).
pub fn effective_first_line_style(block_style: &ComputedStyle) -> ComputedStyle {
    match &block_style.first_line_style {
        Some(fls) => *fls.clone(),
        None => block_style.clone(),
    }
}

/// Compute an overlay style from the first-line pseudo and the parent.
///
/// This creates a `ComputedStyle` that inherits everything from `parent`
/// except the properties explicitly set in `first_line_overrides`.
pub fn build_first_line_style(
    parent: &ComputedStyle,
    overrides: &FirstLineOverrides,
) -> ComputedStyle {
    apply_first_line_overrides(parent, overrides)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_style::ComputedStyle;

    #[test]
    fn default_overrides_produce_identical_style() {
        let base = ComputedStyle::initial();
        let overrides = FirstLineOverrides::default();
        let merged = apply_first_line_overrides(&base, &overrides);
        assert_eq!(merged.font_size, base.font_size);
        assert_eq!(merged.color, base.color);
        assert_eq!(merged.letter_spacing, base.letter_spacing);
    }

    #[test]
    fn font_size_override_applies() {
        let base = ComputedStyle::initial();
        let overrides = FirstLineOverrides {
            font_size: Some(24.0),
            ..Default::default()
        };
        let merged = apply_first_line_overrides(&base, &overrides);
        assert_eq!(merged.font_size, 24.0);
        assert_eq!(merged.color, base.color);
    }

    #[test]
    fn multiple_overrides_apply() {
        let base = ComputedStyle::initial();
        let overrides = FirstLineOverrides {
            font_size: Some(32.0),
            letter_spacing: Some(2.0),
            word_spacing: Some(4.0),
            ..Default::default()
        };
        let merged = apply_first_line_overrides(&base, &overrides);
        assert_eq!(merged.font_size, 32.0);
        assert_eq!(merged.letter_spacing, 2.0);
        assert_eq!(merged.word_spacing, 4.0);
    }

    #[test]
    fn resolve_first_line_returns_none_when_absent() {
        let style = ComputedStyle::initial();
        assert!(resolve_first_line_style(&style).is_none());
    }

    #[test]
    fn resolve_first_line_returns_some_when_present() {
        let mut style = ComputedStyle::initial();
        let mut fls = ComputedStyle::initial();
        fls.font_size = 24.0;
        style.first_line_style = Some(Box::new(fls));
        let resolved = resolve_first_line_style(&style).unwrap();
        assert_eq!(resolved.font_size, 24.0);
    }

    #[test]
    fn effective_first_line_with_override() {
        let mut block = ComputedStyle::initial();
        let mut fls = ComputedStyle::initial();
        fls.font_size = 48.0;
        fls.letter_spacing = 3.0;
        block.first_line_style = Some(Box::new(fls));
        let eff = effective_first_line_style(&block);
        assert_eq!(eff.font_size, 48.0);
        assert_eq!(eff.letter_spacing, 3.0);
    }

    #[test]
    fn effective_first_line_without_override() {
        let block = ComputedStyle::initial();
        let eff = effective_first_line_style(&block);
        assert_eq!(eff.font_size, 16.0);
    }

    #[test]
    fn first_line_property_list_completeness() {
        assert_eq!(FirstLineProperty::ALL.len(), 10);
    }
}

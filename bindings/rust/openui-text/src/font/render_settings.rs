//! Actual owned SkFont settings and a separate pure optional-style model.
//!
//! Neither observation nor optional-style merging changes font selection or
//! executes Fontconfig. Snapshots precede final device/scaler LCD eligibility.

use openui_geometry::{RasterConfiguration, TextRasterConfiguration};
use skia_safe::{font::Edging, Font as SkFont, FontHinting};

/// Exact optional-use encoding in pinned Chromium WebFontRenderStyle.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontRenderPreference {
    Off,
    On,
    #[default]
    NoPreference,
}

impl FontRenderPreference {
    pub const fn code(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::On => 1,
            Self::NoPreference => 2,
        }
    }
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Off),
            1 => Some(Self::On),
            2 => Some(Self::NoPreference),
            _ => None,
        }
    }
    fn override_with(self, other: Self) -> Self {
        if other == Self::NoPreference {
            self
        } else {
            other
        }
    }
}

/// Pure hint-style encoding, separate from an applied SkFont hinting value.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontRenderHintStyle {
    #[default]
    None,
    Slight,
    Normal,
    Full,
}

impl FontRenderHintStyle {
    pub const fn code(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Slight => 1,
            Self::Normal => 2,
            Self::Full => 3,
        }
    }
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::None),
            1 => Some(Self::Slight),
            2 => Some(Self::Normal),
            3 => Some(Self::Full),
            _ => None,
        }
    }
}

/// Pure optional preferences, not browser globals or a queried runtime tuple.
///
/// Default represents the source's initial all-NoPreference style. Hint style
/// is independent: it is overridden only when use_hinting is specified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontRenderStyle {
    use_bitmaps: FontRenderPreference,
    use_auto_hint: FontRenderPreference,
    use_hinting: FontRenderPreference,
    hint_style: FontRenderHintStyle,
    use_anti_alias: FontRenderPreference,
    use_subpixel_rendering: FontRenderPreference,
    use_subpixel_positioning: FontRenderPreference,
}

impl Default for FontRenderStyle {
    fn default() -> Self {
        Self {
            use_bitmaps: FontRenderPreference::NoPreference,
            use_auto_hint: FontRenderPreference::NoPreference,
            use_hinting: FontRenderPreference::NoPreference,
            hint_style: FontRenderHintStyle::None,
            use_anti_alias: FontRenderPreference::NoPreference,
            use_subpixel_rendering: FontRenderPreference::NoPreference,
            use_subpixel_positioning: FontRenderPreference::NoPreference,
        }
    }
}

impl FontRenderStyle {
    pub fn with_bitmaps(mut self, value: FontRenderPreference) -> Self {
        self.use_bitmaps = value;
        self
    }
    pub fn with_auto_hint(mut self, value: FontRenderPreference) -> Self {
        self.use_auto_hint = value;
        self
    }
    pub fn with_hinting(
        mut self,
        value: FontRenderPreference,
        hint_style: FontRenderHintStyle,
    ) -> Self {
        self.use_hinting = value;
        self.hint_style = hint_style;
        self
    }
    pub fn with_anti_alias(mut self, value: FontRenderPreference) -> Self {
        self.use_anti_alias = value;
        self
    }
    pub fn with_subpixel_rendering(mut self, value: FontRenderPreference) -> Self {
        self.use_subpixel_rendering = value;
        self
    }
    pub fn with_subpixel_positioning(mut self, value: FontRenderPreference) -> Self {
        self.use_subpixel_positioning = value;
        self
    }
    /// Pure counterpart of OverrideWith; no font, global, or query is touched.
    pub fn override_with(self, other: Self) -> Self {
        Self {
            use_bitmaps: self.use_bitmaps.override_with(other.use_bitmaps),
            use_auto_hint: self.use_auto_hint.override_with(other.use_auto_hint),
            use_hinting: self.use_hinting.override_with(other.use_hinting),
            hint_style: if other.use_hinting == FontRenderPreference::NoPreference {
                self.hint_style
            } else {
                other.hint_style
            },
            use_anti_alias: self.use_anti_alias.override_with(other.use_anti_alias),
            use_subpixel_rendering: self
                .use_subpixel_rendering
                .override_with(other.use_subpixel_rendering),
            use_subpixel_positioning: self
                .use_subpixel_positioning
                .override_with(other.use_subpixel_positioning),
        }
    }
    pub fn use_bitmaps(&self) -> FontRenderPreference {
        self.use_bitmaps
    }
    pub fn use_auto_hint(&self) -> FontRenderPreference {
        self.use_auto_hint
    }
    pub fn use_hinting(&self) -> FontRenderPreference {
        self.use_hinting
    }
    pub fn hint_style(&self) -> FontRenderHintStyle {
        self.hint_style
    }
    pub fn use_anti_alias(&self) -> FontRenderPreference {
        self.use_anti_alias
    }
    pub fn use_subpixel_rendering(&self) -> FontRenderPreference {
        self.use_subpixel_rendering
    }
    pub fn use_subpixel_positioning(&self) -> FontRenderPreference {
        self.use_subpixel_positioning
    }
}

/// Getter snapshot of one actual owned source or shaping SkFont.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppliedSkFontSettings {
    size: f32,
    scale_x: f32,
    skew_x: f32,
    edging: Edging,
    hinting: FontHinting,
    subpixel: bool,
    linear_metrics: bool,
    force_auto_hinting: bool,
    embedded_bitmaps: bool,
    embolden: bool,
    baseline_snap: bool,
}

impl AppliedSkFontSettings {
    pub(crate) fn capture(font: &SkFont) -> Self {
        Self {
            size: font.size(),
            scale_x: font.scale_x(),
            skew_x: font.skew_x(),
            edging: font.edging(),
            hinting: font.hinting(),
            subpixel: font.is_subpixel(),
            linear_metrics: font.is_linear_metrics(),
            force_auto_hinting: font.is_force_auto_hinting(),
            embedded_bitmaps: font.is_embedded_bitmaps(),
            embolden: font.is_embolden(),
            baseline_snap: font.is_baseline_snap(),
        }
    }
    pub fn size(&self) -> f32 {
        self.size
    }
    pub fn scale_x(&self) -> f32 {
        self.scale_x
    }
    pub fn skew_x(&self) -> f32 {
        self.skew_x
    }
    pub fn edging(&self) -> Edging {
        self.edging
    }
    pub fn hinting(&self) -> FontHinting {
        self.hinting
    }
    pub fn subpixel(&self) -> bool {
        self.subpixel
    }
    pub fn linear_metrics(&self) -> bool {
        self.linear_metrics
    }
    pub fn force_auto_hinting(&self) -> bool {
        self.force_auto_hinting
    }
    pub fn embedded_bitmaps(&self) -> bool {
        self.embedded_bitmaps
    }
    pub fn embolden(&self) -> bool {
        self.embolden
    }
    pub fn baseline_snap(&self) -> bool {
        self.baseline_snap
    }
}

/// Applied source/shaping settings, separate from the requested raster profile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppliedPlatformFontSettings {
    source_font: AppliedSkFontSettings,
    shaping_font: AppliedSkFontSettings,
    requested_raster_configuration: RasterConfiguration,
    selected_raster_configuration: TextRasterConfiguration,
    device_scale_factor: f64,
    native_control_text: bool,
    embedded_document_text: bool,
    native_button_text_metrics: bool,
    query_result: Option<FontRenderStyle>,
}

impl AppliedPlatformFontSettings {
    pub(crate) fn capture(
        source_font: &SkFont,
        shaping_font: &SkFont,
        requested_raster_configuration: RasterConfiguration,
        selected_raster_configuration: TextRasterConfiguration,
        device_scale_factor: f64,
        native_control_text: bool,
        embedded_document_text: bool,
        native_button_text_metrics: bool,
    ) -> Self {
        Self {
            source_font: AppliedSkFontSettings::capture(source_font),
            shaping_font: AppliedSkFontSettings::capture(shaping_font),
            requested_raster_configuration,
            selected_raster_configuration,
            device_scale_factor,
            native_control_text,
            embedded_document_text,
            native_button_text_metrics,
            query_result: None,
        }
    }
    pub fn source_font(&self) -> &AppliedSkFontSettings {
        &self.source_font
    }
    pub fn shaping_font(&self) -> &AppliedSkFontSettings {
        &self.shaping_font
    }
    pub fn requested_raster_configuration(&self) -> RasterConfiguration {
        self.requested_raster_configuration
    }
    pub fn selected_raster_configuration(&self) -> TextRasterConfiguration {
        self.selected_raster_configuration
    }
    pub fn device_scale_factor(&self) -> f64 {
        self.device_scale_factor
    }
    pub fn native_control_text(&self) -> bool {
        self.native_control_text
    }
    pub fn embedded_document_text(&self) -> bool {
        self.embedded_document_text
    }
    pub fn native_button_text_metrics(&self) -> bool {
        self.native_button_text_metrics
    }
    /// None on the legacy path: no platform render query was executed.
    pub fn query_result(&self) -> Option<&FontRenderStyle> {
        self.query_result.as_ref()
    }
}

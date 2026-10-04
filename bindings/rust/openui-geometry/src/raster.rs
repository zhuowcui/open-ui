//! Immutable raster policy and logical/physical snapping primitives.

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RasterBackend {
    Skia,
    ChromiumLinux,
    GaneshGl,
    /// CPU replay with Chromium's Fontations outline path for authored text.
    ChromiumLinuxFontations,
}

impl RasterBackend {
    /// CPU policies that select glyph strikes in physical pixels.
    pub const fn is_chromium_cpu(self) -> bool {
        matches!(self, Self::ChromiumLinux | Self::ChromiumLinuxFontations)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextEdging {
    Alias,
    AntiAlias,
    SubpixelAntiAlias,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextHinting {
    None,
    Slight,
    Normal,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RasterPixelGeometry {
    Unknown,
    RgbHorizontal,
    BgrHorizontal,
    RgbVertical,
    BgrVertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextRasterConfiguration {
    pub edging: TextEdging,
    pub hinting: TextHinting,
    pub subpixel_positioning: bool,
    pub force_autohint: bool,
    /// Horizontal LCD phase in 1/64 physical pixel units.
    pub lcd_phase_64ths: i16,
}

impl TextRasterConfiguration {
    pub const fn aliased(subpixel_positioning: bool) -> Self {
        Self {
            edging: TextEdging::Alias,
            hinting: TextHinting::None,
            subpixel_positioning,
            force_autohint: false,
            lcd_phase_64ths: 0,
        }
    }

    pub const fn chromium_lcd() -> Self {
        Self {
            edging: TextEdging::SubpixelAntiAlias,
            hinting: TextHinting::Slight,
            subpixel_positioning: true,
            force_autohint: false,
            lcd_phase_64ths: 0,
        }
    }
}

/// Complete immutable raster selection retained by engines, scenes and frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RasterConfiguration {
    pub backend: RasterBackend,
    pub author_text: TextRasterConfiguration,
    pub native_text: TextRasterConfiguration,
    pub embedded_text: TextRasterConfiguration,
    pub pixel_geometry: RasterPixelGeometry,
    /// Surface gamma encoded in thousandths (1200 means 1.2).
    pub gamma_milli: u16,
    /// Surface contrast encoded in thousandths (200 means 0.2).
    pub contrast_milli: u16,
}

impl RasterConfiguration {
    pub const fn deterministic_aliased(subpixel_positioning: bool) -> Self {
        Self {
            backend: RasterBackend::ChromiumLinux,
            author_text: TextRasterConfiguration::aliased(subpixel_positioning),
            native_text: TextRasterConfiguration::chromium_lcd(),
            embedded_text: TextRasterConfiguration::chromium_lcd(),
            pixel_geometry: RasterPixelGeometry::RgbHorizontal,
            gamma_milli: 1200,
            contrast_milli: 200,
        }
    }

    pub const fn chromium_linux_lcd() -> Self {
        let lcd = TextRasterConfiguration::chromium_lcd();
        Self {
            backend: RasterBackend::ChromiumLinux,
            // Chromium's Linux author text uses FreeType's light fitting at
            // every replay scale. The strike size changes with device scale;
            // the hinting mode itself does not.
            author_text: lcd,
            native_text: lcd,
            embedded_text: lcd,
            pixel_geometry: RasterPixelGeometry::RgbHorizontal,
            gamma_milli: 1200,
            contrast_milli: 200,
        }
    }

    /// Explicit Fontations counterpart of the Linux FreeType LCD policy.
    ///
    /// Chromium supports both typeface engines. Captures that explicitly
    /// select FreeType retain `chromium_linux_lcd`; applications comparing
    /// Fontations authored text select this constructor. Surface and layout
    /// settings are otherwise identical, and the engine never changes this
    /// selection based on a font family, fixture ID or machine environment.
    pub const fn chromium_linux_fontations_lcd() -> Self {
        let mut configuration = Self::chromium_linux_lcd();
        configuration.backend = RasterBackend::ChromiumLinuxFontations;
        configuration
    }

    /// Chromium-aligned Linux text policy replayed through Ganesh OpenGL.
    ///
    /// Selecting this configuration never probes the environment. Callers
    /// must also build and initialize the `ganesh-gl` compositor explicitly.
    pub const fn chromium_linux_ganesh() -> Self {
        let mut configuration = Self::chromium_linux_lcd();
        configuration.backend = RasterBackend::GaneshGl;
        configuration
    }

    pub const fn with_backend(mut self, backend: RasterBackend) -> Self {
        self.backend = backend;
        self
    }

    /// Unit-scale compatibility policy used only to replay the immutable
    /// pre-raster-configuration evidence. The historical environment knobs
    /// selected text properties but left paint on the software Skia backend.
    pub const fn legacy_deterministic_aliased(subpixel_positioning: bool) -> Self {
        let mut configuration = Self::deterministic_aliased(subpixel_positioning);
        configuration.backend = RasterBackend::Skia;
        configuration
    }

    /// Real-font counterpart of [`Self::legacy_deterministic_aliased`].
    pub const fn legacy_chromium_linux_lcd() -> Self {
        let mut configuration = Self::chromium_linux_lcd();
        configuration.backend = RasterBackend::Skia;
        configuration
    }

    pub const fn gamma(self) -> f32 {
        self.gamma_milli as f32 / 1000.0
    }

    pub const fn contrast(self) -> f32 {
        self.contrast_milli as f32 / 1000.0
    }
}

impl Default for RasterConfiguration {
    fn default() -> Self {
        Self {
            backend: RasterBackend::Skia,
            author_text: TextRasterConfiguration {
                edging: TextEdging::AntiAlias,
                hinting: TextHinting::Slight,
                subpixel_positioning: true,
                force_autohint: false,
                lcd_phase_64ths: 0,
            },
            native_text: TextRasterConfiguration::chromium_lcd(),
            embedded_text: TextRasterConfiguration::chromium_lcd(),
            pixel_geometry: RasterPixelGeometry::Unknown,
            gamma_milli: 1000,
            contrast_milli: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalSnap {
    Floor,
    Nearest,
    Ceil,
}

/// One immutable logical-to-physical snapping policy for a frame.
///
/// Layout and hit testing continue to use logical coordinates. Paint code
/// uses this helper only at raster boundaries so every subsystem applies the
/// same device-scale conversion and rounding rules.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RasterSnapping {
    device_scale: f64,
}

impl RasterSnapping {
    pub fn new(device_scale: f64) -> Self {
        debug_assert!(device_scale.is_finite() && device_scale > 0.0);
        Self { device_scale }
    }

    pub fn device_scale(self) -> f64 {
        self.device_scale
    }

    /// Convert a logical coordinate directly to an integral physical pixel.
    pub fn physical_coordinate(self, value: f32, mode: PhysicalSnap) -> i64 {
        let physical = f64::from(value) * self.device_scale;
        match mode {
            PhysicalSnap::Floor => physical.floor() as i64,
            PhysicalSnap::Nearest => physical.round() as i64,
            PhysicalSnap::Ceil => physical.ceil() as i64,
        }
    }

    /// Snap a logical coordinate through the physical pixel grid.
    pub fn logical_coordinate(self, value: f32, mode: PhysicalSnap) -> f32 {
        self.physical_coordinate(value, mode) as f32 / self.device_scale as f32
    }

    /// Snap a non-negative logical length while enforcing a physical-pixel
    /// minimum. This is used for decorations, borders and carets whose CSS
    /// width remains logical but whose raster strike cannot disappear.
    pub fn logical_length(
        self,
        value: f32,
        mode: PhysicalSnap,
        minimum_physical_pixels: u32,
    ) -> f32 {
        let physical = self
            .physical_coordinate(value.max(0.0), mode)
            .max(i64::from(minimum_physical_pixels));
        physical as f32 / self.device_scale as f32
    }

    /// Snap a finite positive logical interval outward into physical pixels.
    pub fn outward_physical_span(self, start: f32, length: f32) -> Option<(i64, i64)> {
        if !start.is_finite() || !length.is_finite() || length <= 0.0 {
            return None;
        }
        let first = self.physical_coordinate(start, PhysicalSnap::Floor);
        let last = self.physical_coordinate(start + length, PhysicalSnap::Ceil);
        (last > first).then_some((first, last))
    }
}

/// Snap a logical coordinate through physical pixels and return logical units.
pub fn snap_logical(value: f32, device_scale: f64, mode: PhysicalSnap) -> f32 {
    RasterSnapping::new(device_scale).logical_coordinate(value, mode)
}

/// Select a font strike/hinting size in physical pixels while layout advances
/// remain expressed in logical CSS pixels.
pub fn physical_font_size(logical_size: f32, device_scale: f64) -> f32 {
    (f64::from(logical_size) * device_scale) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snaps_coordinates_through_fractional_device_scales() {
        let snapping = RasterSnapping::new(1.25);
        assert_eq!(snapping.physical_coordinate(1.0, PhysicalSnap::Floor), 1);
        assert_eq!(snapping.physical_coordinate(1.0, PhysicalSnap::Ceil), 2);
        assert_eq!(snapping.logical_coordinate(1.0, PhysicalSnap::Nearest), 0.8);
    }

    #[test]
    fn snaps_lengths_with_a_physical_minimum() {
        let snapping = RasterSnapping::new(2.0);
        assert_eq!(snapping.logical_length(0.1, PhysicalSnap::Nearest, 1), 0.5);
        assert_eq!(snapping.logical_length(0.9, PhysicalSnap::Nearest, 1), 1.0);
    }

    #[test]
    fn outward_span_contains_fractional_logical_bounds() {
        let snapping = RasterSnapping::new(1.5);
        assert_eq!(snapping.outward_physical_span(0.5, 1.0), Some((0, 3)));
        assert_eq!(snapping.outward_physical_span(0.5, 0.0), None);
    }

    #[test]
    fn legacy_profiles_preserve_text_policy_on_the_software_backend() {
        let aliased = RasterConfiguration::legacy_deterministic_aliased(true);
        assert_eq!(aliased.backend, RasterBackend::Skia);
        assert_eq!(aliased.author_text.edging, TextEdging::Alias);
        assert_eq!(aliased.native_text.edging, TextEdging::SubpixelAntiAlias);
        assert_eq!(aliased.embedded_text.edging, TextEdging::SubpixelAntiAlias);
        assert!(aliased.author_text.subpixel_positioning);

        let lcd = RasterConfiguration::legacy_chromium_linux_lcd();
        assert_eq!(lcd.backend, RasterBackend::Skia);
        assert_eq!(lcd.author_text.edging, TextEdging::SubpixelAntiAlias);
        assert_eq!(lcd.author_text.hinting, TextHinting::Slight);
        assert_eq!(lcd.native_text.hinting, TextHinting::Slight);
    }

    #[test]
    fn ganesh_profile_is_explicit_and_immutable() {
        let configuration = RasterConfiguration::chromium_linux_ganesh();
        assert_eq!(configuration.backend, RasterBackend::GaneshGl);
        assert_eq!(
            configuration.author_text,
            RasterConfiguration::chromium_linux_lcd().author_text
        );
    }
}

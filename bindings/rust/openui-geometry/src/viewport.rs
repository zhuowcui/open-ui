//! Validated logical/physical viewport metrics.

/// Whether logical or physical dimensions were authoritative at construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewportAuthority {
    /// Headless rendering: logical CSS size is exact and physical size is rounded.
    Logical,
    /// Native windows: physical surface size is exact and logical size is derived.
    Physical,
}

/// A complete viewport description shared by layout, scenes, and frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportMetrics {
    logical_width: f64,
    logical_height: f64,
    physical_width: u32,
    physical_height: u32,
    device_scale_factor: f64,
    authority: ViewportAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportMetricsError {
    InvalidLogicalSize,
    InvalidPhysicalSize,
    InvalidScale,
    SurfaceTooLarge,
}

impl std::fmt::Display for ViewportMetricsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLogicalSize => {
                f.write_str("logical viewport dimensions must be finite and positive")
            }
            Self::InvalidPhysicalSize => {
                f.write_str("physical viewport dimensions must be positive")
            }
            Self::InvalidScale => f.write_str("device scale must be finite and positive"),
            Self::SurfaceTooLarge => {
                f.write_str("physical viewport exceeds the Skia surface limit")
            }
        }
    }
}

impl std::error::Error for ViewportMetricsError {}

impl ViewportMetrics {
    /// Construct a headless viewport from authoritative logical CSS pixels.
    pub fn from_logical_size(
        logical_width: f64,
        logical_height: f64,
        device_scale_factor: f64,
    ) -> Result<Self, ViewportMetricsError> {
        validate_logical(logical_width, logical_height)?;
        validate_scale(device_scale_factor)?;
        let physical_width = logical_to_physical(logical_width, device_scale_factor)?;
        let physical_height = logical_to_physical(logical_height, device_scale_factor)?;
        Ok(Self {
            logical_width,
            logical_height,
            physical_width,
            physical_height,
            device_scale_factor,
            authority: ViewportAuthority::Logical,
        })
    }

    /// Construct a native-window viewport from its authoritative surface size.
    pub fn from_physical_size(
        physical_width: u32,
        physical_height: u32,
        device_scale_factor: f64,
    ) -> Result<Self, ViewportMetricsError> {
        validate_physical(physical_width, physical_height)?;
        validate_scale(device_scale_factor)?;
        let logical_width = f64::from(physical_width) / device_scale_factor;
        let logical_height = f64::from(physical_height) / device_scale_factor;
        validate_logical(logical_width, logical_height)?;
        Ok(Self {
            logical_width,
            logical_height,
            physical_width,
            physical_height,
            device_scale_factor,
            authority: ViewportAuthority::Physical,
        })
    }

    pub const fn logical_width(self) -> f64 {
        self.logical_width
    }
    pub const fn logical_height(self) -> f64 {
        self.logical_height
    }
    pub const fn physical_width(self) -> u32 {
        self.physical_width
    }
    pub const fn physical_height(self) -> u32 {
        self.physical_height
    }
    pub const fn device_scale_factor(self) -> f64 {
        self.device_scale_factor
    }
    pub const fn authority(self) -> ViewportAuthority {
        self.authority
    }
    pub const fn logical_size(self) -> (f64, f64) {
        (self.logical_width, self.logical_height)
    }
    pub const fn physical_size(self) -> (u32, u32) {
        (self.physical_width, self.physical_height)
    }
}

fn validate_logical(width: f64, height: f64) -> Result<(), ViewportMetricsError> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err(ViewportMetricsError::InvalidLogicalSize);
    }
    Ok(())
}

fn validate_scale(scale: f64) -> Result<(), ViewportMetricsError> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(ViewportMetricsError::InvalidScale);
    }
    Ok(())
}

fn validate_physical(width: u32, height: u32) -> Result<(), ViewportMetricsError> {
    if width == 0 || height == 0 {
        return Err(ViewportMetricsError::InvalidPhysicalSize);
    }
    if width > i32::MAX as u32 || height > i32::MAX as u32 {
        return Err(ViewportMetricsError::SurfaceTooLarge);
    }
    Ok(())
}

fn logical_to_physical(logical: f64, scale: f64) -> Result<u32, ViewportMetricsError> {
    let physical = logical * scale;
    if !physical.is_finite() || physical > f64::from(i32::MAX) {
        return Err(ViewportMetricsError::SurfaceTooLarge);
    }
    // winit/dpi uses `round()` for positive logical-to-physical conversion.
    let rounded = physical.round();
    if rounded < 1.0 {
        return Err(ViewportMetricsError::InvalidPhysicalSize);
    }
    Ok(rounded as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_authority_uses_winit_rounding() {
        let metrics = ViewportMetrics::from_logical_size(10.5, 20.25, 1.0).unwrap();
        assert_eq!(metrics.physical_size(), (11, 20));
        assert_eq!(metrics.logical_size(), (10.5, 20.25));
        assert_eq!(metrics.authority(), ViewportAuthority::Logical);
    }

    #[test]
    fn physical_authority_preserves_native_surface() {
        let metrics = ViewportMetrics::from_physical_size(1601, 901, 1.25).unwrap();
        assert_eq!(metrics.physical_size(), (1601, 901));
        assert_eq!(metrics.logical_size(), (1280.8, 720.8));
        assert_eq!(metrics.authority(), ViewportAuthority::Physical);
    }

    #[test]
    fn rejects_non_finite_and_unrepresentable_metrics() {
        assert!(ViewportMetrics::from_logical_size(f64::NAN, 1.0, 1.0).is_err());
        assert!(ViewportMetrics::from_logical_size(1.0, 1.0, 0.0).is_err());
        assert!(ViewportMetrics::from_physical_size(0, 1, 1.0).is_err());
        assert!(ViewportMetrics::from_logical_size(i32::MAX as f64, 1.0, 2.0).is_err());
    }

    #[test]
    fn deterministic_fractional_matrix_preserves_authority() {
        // This exercises more than the 1,000 viewport/scale combinations in
        // the renderer contract without relying on a random-number source.
        let scales = [1.0, 1.25, 1.5, 2.0, 3.0];
        let mut checked = 0;
        for width_step in 1..=16 {
            for height_step in 1..=16 {
                for scale in scales {
                    let logical_width = 0.25 + f64::from(width_step) * 73.125;
                    let logical_height = 0.5 + f64::from(height_step) * 41.0625;
                    let logical =
                        ViewportMetrics::from_logical_size(logical_width, logical_height, scale)
                            .unwrap();
                    assert_eq!(
                        logical.physical_width(),
                        (logical_width * scale).round() as u32
                    );
                    assert_eq!(
                        logical.physical_height(),
                        (logical_height * scale).round() as u32
                    );
                    assert_eq!(logical.logical_size(), (logical_width, logical_height));

                    let physical = ViewportMetrics::from_physical_size(
                        logical.physical_width(),
                        logical.physical_height(),
                        scale,
                    )
                    .unwrap();
                    assert_eq!(physical.physical_size(), logical.physical_size());
                    assert_eq!(
                        physical.logical_width(),
                        f64::from(logical.physical_width()) / scale
                    );
                    assert_eq!(
                        physical.logical_height(),
                        f64::from(logical.physical_height()) / scale
                    );
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, 1_280);
    }
}

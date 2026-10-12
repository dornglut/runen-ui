//! Checked winit-window geometry and displayed-frame-relative cursor translation.
//!
//! This module translates immutable native facts only. Hosts own windows,
//! successful display contexts, focus/input lifetimes, runtime pumping, GPU
//! presentation, and recovery. No event-loop or display authority is retained.

use runenui_core::LogicalPoint;
use runenui_runtime::{LogicalSize, RasterScale};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    window::Window,
};

/// Checked physical extent, native scale, and the corresponding RunenUI sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NativeMapping {
    physical_size: PhysicalSize<u32>,
    native_scale_factor: f64,
    logical_size: LogicalSize,
    raster_scale: RasterScale,
}

impl NativeMapping {
    /// Snapshots the current window's physical extent and native scale.
    #[must_use]
    pub fn from_window(window: &Window) -> Option<Self> {
        Self::from_parts(window.inner_size(), window.scale_factor())
    }

    /// Rejects zero extents, invalid scales, and logical/raster overflow.
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        reason = "native f64 geometry is range-checked before conversion into RunenUI's f32 protocol"
    )]
    pub fn from_parts(physical_size: PhysicalSize<u32>, native_scale_factor: f64) -> Option<Self> {
        if physical_size.width == 0
            || physical_size.height == 0
            || !native_scale_factor.is_finite()
            || native_scale_factor <= 0.0
            || native_scale_factor > f64::from(f32::MAX)
        {
            return None;
        }
        let logical_width = f64::from(physical_size.width) / native_scale_factor;
        let logical_height = f64::from(physical_size.height) / native_scale_factor;
        if !logical_width.is_finite()
            || !logical_height.is_finite()
            || logical_width > f64::from(f32::MAX)
            || logical_height > f64::from(f32::MAX)
        {
            return None;
        }
        Some(Self {
            physical_size,
            native_scale_factor,
            logical_size: LogicalSize::try_new(logical_width as f32, logical_height as f32).ok()?,
            raster_scale: RasterScale::new(native_scale_factor as f32).ok()?,
        })
    }

    #[must_use]
    pub const fn physical_size(self) -> PhysicalSize<u32> {
        self.physical_size
    }

    #[must_use]
    pub const fn native_scale_factor(self) -> f64 {
        self.native_scale_factor
    }

    #[must_use]
    pub const fn logical_size(self) -> LogicalSize {
        self.logical_size
    }

    #[must_use]
    pub const fn raster_scale(self) -> RasterScale {
        self.raster_scale
    }

    /// Converts physical pointer position against the exact successfully
    /// displayed mapping, rejecting any change in the current window mapping.
    ///
    /// Only the caller owns the displayed `SurfaceInputContext` and may attach
    /// it to the returned logical point after this check succeeds.
    ///
    /// # Errors
    ///
    /// Fails when the current mapping is missing or differs from the displayed
    /// mapping, or the position is non-finite or outside logical f32 range.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "native physical positions and divided logical values are checked before f32 conversion"
    )]
    pub fn translate_cursor(
        self,
        current_mapping: Option<Self>,
        physical_position: PhysicalPosition<f64>,
    ) -> Result<LogicalPoint, NativePointError> {
        let current_mapping = current_mapping.ok_or(NativePointError::MappingUnavailable)?;
        if current_mapping != self {
            return Err(NativePointError::DisplayedMappingMismatch);
        }
        if !physical_position.x.is_finite() || !physical_position.y.is_finite() {
            return Err(NativePointError::NonFiniteNativePosition);
        }
        let x = physical_position.x / self.native_scale_factor;
        let y = physical_position.y / self.native_scale_factor;
        if !x.is_finite()
            || !y.is_finite()
            || x < f64::from(f32::MIN)
            || x > f64::from(f32::MAX)
            || y < f64::from(f32::MIN)
            || y > f64::from(f32::MAX)
        {
            return Err(NativePointError::LogicalPositionOutOfRange);
        }
        LogicalPoint::new(x as f32, y as f32)
            .map_err(|_| NativePointError::LogicalPositionOutOfRange)
    }
}

/// Exact reason native cursor coordinates cannot be correlated to a displayed mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativePointError {
    MappingUnavailable,
    DisplayedMappingMismatch,
    NonFiniteNativePosition,
    LogicalPositionOutOfRange,
}

#[cfg(test)]
mod tests {
    use super::{NativeMapping, NativePointError};
    use winit::dpi::{PhysicalPosition, PhysicalSize};

    #[test]
    fn checked_mapping_retains_exact_native_extent_and_derived_scale() {
        let mapping = NativeMapping::from_parts(PhysicalSize::new(1200, 800), 2.0)
            .unwrap_or_else(|| unreachable!("finite native geometry is admitted"));
        assert_eq!(mapping.physical_size(), PhysicalSize::new(1200, 800));
        assert_eq!(mapping.native_scale_factor(), 2.0);
        assert_eq!(mapping.logical_size().width(), 600.0);
        assert_eq!(mapping.logical_size().height(), 400.0);
        assert_eq!(mapping.raster_scale().get(), 2.0);
    }

    #[test]
    fn invalid_native_extent_scale_and_logical_overflow_are_rejected() {
        for extent in [PhysicalSize::new(0, 1), PhysicalSize::new(1, 0)] {
            assert!(NativeMapping::from_parts(extent, 1.0).is_none());
        }
        for scale in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX, 1.0e-100] {
            assert!(
                NativeMapping::from_parts(PhysicalSize::new(10, 10), scale).is_none(),
                "invalid native scale {scale:?}"
            );
        }
    }

    #[test]
    fn displayed_mapping_is_required_to_translate_native_pointer() {
        let displayed = NativeMapping::from_parts(PhysicalSize::new(1200, 800), 2.0)
            .unwrap_or_else(|| unreachable!("mapping fixture is valid"));
        let physical = PhysicalPosition::new(240.0, 120.0);
        let point = displayed
            .translate_cursor(Some(displayed), physical)
            .unwrap_or_else(|_| unreachable!("matching displayed scale translates"));
        assert_eq!(point.x(), 120.0);
        assert_eq!(point.y(), 60.0);
        assert_eq!(
            displayed.translate_cursor(None, physical),
            Err(NativePointError::MappingUnavailable)
        );
        let stale = NativeMapping::from_parts(PhysicalSize::new(1200, 800), 1.5)
            .unwrap_or_else(|| unreachable!("other scale is individually valid"));
        assert_eq!(
            displayed.translate_cursor(Some(stale), physical),
            Err(NativePointError::DisplayedMappingMismatch)
        );
        let resized = NativeMapping::from_parts(PhysicalSize::new(1400, 800), 2.0)
            .unwrap_or_else(|| unreachable!("other physical extent is valid"));
        assert_eq!(
            displayed.translate_cursor(Some(resized), physical),
            Err(NativePointError::DisplayedMappingMismatch)
        );
    }

    #[test]
    fn nonfinite_and_out_of_range_pointer_positions_fail_closed() {
        let displayed = NativeMapping::from_parts(PhysicalSize::new(1200, 800), 2.0)
            .unwrap_or_else(|| unreachable!("mapping fixture is valid"));
        for physical in [
            PhysicalPosition::new(f64::NAN, 20.0),
            PhysicalPosition::new(f64::INFINITY, 20.0),
            PhysicalPosition::new(20.0, f64::NEG_INFINITY),
        ] {
            assert_eq!(
                displayed.translate_cursor(Some(displayed), physical),
                Err(NativePointError::NonFiniteNativePosition)
            );
        }
        assert_eq!(
            displayed.translate_cursor(Some(displayed), PhysicalPosition::new(f64::MAX, 20.0)),
            Err(NativePointError::LogicalPositionOutOfRange)
        );
    }
}

//! Host-neutral scroll viewport and bound-control protocol values.

use core::{error::Error, fmt};

use crate::{Axis, LogicalLength};

/// Error returned when a bound control uses a zero small-step extent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScrollControlBindingError;

impl fmt::Display for ScrollControlBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("scroll-control small step must be greater than zero")
    }
}

impl Error for ScrollControlBindingError {}

/// Transient authored binding for one descendant scroll control.
///
/// Runtime resolves the nearest eligible ancestor scroll owner for this axis.
/// The binding never carries a mounted identity or mirrors the current offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollControlBinding {
    axis: Axis,
    small_step: LogicalLength,
}

impl ScrollControlBinding {
    /// Creates one checked axis binding.
    ///
    /// # Errors
    ///
    /// Returns an error when the small step is zero.
    pub const fn new(
        axis: Axis,
        small_step: LogicalLength,
    ) -> Result<Self, ScrollControlBindingError> {
        if small_step.get() == 0.0 {
            Err(ScrollControlBindingError)
        } else {
            Ok(Self { axis, small_step })
        }
    }

    #[must_use]
    pub const fn axis(self) -> Axis {
        self.axis
    }

    #[must_use]
    pub const fn small_step(self) -> LogicalLength {
        self.small_step
    }
}

/// Neutral visibility policy for one authored scrollbar axis.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScrollBarVisibility {
    #[default]
    Automatic,
    Always,
    Hidden,
}

/// Neutral layout placement for one visible scrollbar.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScrollBarPlacement {
    #[default]
    Reserved,
    Overlay,
}

/// Authored geometry and policy for one scrollbar root.
///
/// This is transient layout intent only. Runtime-owned scroll offset and current
/// visibility remain derived from the bound scroll owner's accepted metrics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollBarLayout {
    axis: Axis,
    visibility: ScrollBarVisibility,
    placement: ScrollBarPlacement,
    thickness: LogicalLength,
    minimum_thumb_extent: LogicalLength,
}

impl ScrollBarLayout {
    #[must_use]
    pub const fn new(
        axis: Axis,
        thickness: LogicalLength,
        minimum_thumb_extent: LogicalLength,
    ) -> Self {
        Self {
            axis,
            visibility: ScrollBarVisibility::Automatic,
            placement: ScrollBarPlacement::Reserved,
            thickness,
            minimum_thumb_extent,
        }
    }

    #[must_use]
    pub const fn axis(self) -> Axis {
        self.axis
    }

    #[must_use]
    pub const fn visibility(self) -> ScrollBarVisibility {
        self.visibility
    }

    #[must_use]
    pub const fn placement(self) -> ScrollBarPlacement {
        self.placement
    }

    #[must_use]
    pub const fn thickness(self) -> LogicalLength {
        self.thickness
    }

    #[must_use]
    pub const fn minimum_thumb_extent(self) -> LogicalLength {
        self.minimum_thumb_extent
    }

    #[must_use]
    pub const fn with_visibility(mut self, visibility: ScrollBarVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    #[must_use]
    pub const fn with_placement(mut self, placement: ScrollBarPlacement) -> Self {
        self.placement = placement;
        self
    }
}

/// Structural scroll chrome authored on ordinary public elements.
///
/// Runtime interprets these facts through the same nearest-ancestor scroll
/// ownership used by `ScrollControlBinding`. They carry no mounted identity,
/// current offset, visibility cache, or renderer state.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollChrome {
    Bar(ScrollBarLayout),
    Thumb(Axis),
    Corner,
}

impl ScrollChrome {
    #[must_use]
    pub const fn axis(self) -> Option<Axis> {
        match self {
            Self::Bar(layout) => Some(layout.axis()),
            Self::Thumb(axis) => Some(axis),
            Self::Corner => None,
        }
    }

    #[must_use]
    pub const fn bar_layout(self) -> Option<ScrollBarLayout> {
        match self {
            Self::Bar(layout) => Some(layout),
            Self::Thumb(_) | Self::Corner => None,
        }
    }
}

/// Validation failure for one normalized scroll-control value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScrollNormalizedError {
    NotFinite,
    OutOfRange,
}

impl fmt::Display for ScrollNormalizedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotFinite => "normalized scroll value must be finite",
            Self::OutOfRange => "normalized scroll value must be within [0, 1]",
        })
    }
}

impl Error for ScrollNormalizedError {}

/// Canonical finite normalized scroll-control value in the closed [0, 1] range.
///
/// The stored representation canonicalizes negative zero and supports exact Eq/Hash
/// so it may participate in the existing semantic-command vocabulary.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct ScrollNormalizedValue(u32);

impl ScrollNormalizedValue {
    pub const ZERO: Self = Self(0.0_f32.to_bits());
    pub const ONE: Self = Self(1.0_f32.to_bits());

    /// Validates and canonicalizes a normalized value.
    ///
    /// # Errors
    ///
    /// Returns an error for non-finite values or values outside the closed
    /// [0, 1] interval.
    pub const fn new(value: f32) -> Result<Self, ScrollNormalizedError> {
        if value.is_nan() || value == f32::INFINITY || value == f32::NEG_INFINITY {
            Err(ScrollNormalizedError::NotFinite)
        } else if value < 0.0 || value > 1.0 {
            Err(ScrollNormalizedError::OutOfRange)
        } else if value == 0.0 {
            Ok(Self::ZERO)
        } else {
            Ok(Self(value.to_bits()))
        }
    }

    #[must_use]
    pub const fn get(self) -> f32 {
        f32::from_bits(self.0)
    }
}

impl fmt::Debug for ScrollNormalizedValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ScrollNormalizedValue")
            .field(&self.get())
            .finish()
    }
}

/// Read-only projection for one valid bound descendant scroll control.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollControlSnapshot {
    axis: Axis,
    offset: LogicalLength,
    maximum_offset: LogicalLength,
    viewport_extent: LogicalLength,
    content_extent: LogicalLength,
    normalized_position: ScrollNormalizedValue,
    visible_fraction: ScrollNormalizedValue,
}

impl ScrollControlSnapshot {
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(
        axis: Axis,
        offset: LogicalLength,
        maximum_offset: LogicalLength,
        viewport_extent: LogicalLength,
        content_extent: LogicalLength,
        normalized_position: ScrollNormalizedValue,
        visible_fraction: ScrollNormalizedValue,
    ) -> Self {
        Self {
            axis,
            offset,
            maximum_offset,
            viewport_extent,
            content_extent,
            normalized_position,
            visible_fraction,
        }
    }

    /// Derives one validated runtime snapshot from canonical axis metrics.
    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_from_metrics(
        axis: Axis,
        offset: f32,
        viewport_extent: f32,
        content_extent: f32,
    ) -> Option<Self> {
        if !offset.is_finite()
            || !viewport_extent.is_finite()
            || !content_extent.is_finite()
            || offset < 0.0
            || viewport_extent < 0.0
            || content_extent < 0.0
        {
            return None;
        }
        let maximum = (content_extent - viewport_extent).max(0.0);
        if !maximum.is_finite() {
            return None;
        }
        if offset > maximum {
            return None;
        }
        let normalized_position = if maximum == 0.0 {
            ScrollNormalizedValue::ZERO
        } else {
            ScrollNormalizedValue::new(offset / maximum).ok()?
        };
        let visible_fraction = if content_extent <= viewport_extent || content_extent == 0.0 {
            ScrollNormalizedValue::ONE
        } else {
            ScrollNormalizedValue::new((viewport_extent / content_extent).clamp(0.0, 1.0)).ok()?
        };
        Some(Self::__runtime_new(
            axis,
            LogicalLength::new(offset).ok()?,
            LogicalLength::new(maximum).ok()?,
            LogicalLength::new(viewport_extent).ok()?,
            LogicalLength::new(content_extent).ok()?,
            normalized_position,
            visible_fraction,
        ))
    }

    #[must_use]
    pub const fn axis(self) -> Axis {
        self.axis
    }

    #[must_use]
    pub const fn offset(self) -> LogicalLength {
        self.offset
    }

    #[must_use]
    pub const fn maximum_offset(self) -> LogicalLength {
        self.maximum_offset
    }

    #[must_use]
    pub const fn viewport_extent(self) -> LogicalLength {
        self.viewport_extent
    }

    #[must_use]
    pub const fn content_extent(self) -> LogicalLength {
        self.content_extent
    }

    #[must_use]
    pub const fn normalized_position(self) -> ScrollNormalizedValue {
        self.normalized_position
    }

    #[must_use]
    pub const fn visible_fraction(self) -> ScrollNormalizedValue {
        self.visible_fraction
    }
}

/// Canonical thumb geometry for one scrollbar track.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollBarThumbGeometry {
    thumb_extent: LogicalLength,
    travel: LogicalLength,
    thumb_origin: LogicalLength,
}

impl ScrollBarThumbGeometry {
    #[must_use]
    pub const fn thumb_extent(self) -> LogicalLength {
        self.thumb_extent
    }

    #[must_use]
    pub const fn travel(self) -> LogicalLength {
        self.travel
    }

    #[must_use]
    pub const fn thumb_origin(self) -> LogicalLength {
        self.thumb_origin
    }
}

impl ScrollBarLayout {
    /// Derives checked thumb extent, travel, and origin from the current bound snapshot.
    #[must_use]
    pub fn thumb_geometry(
        self,
        snapshot: ScrollControlSnapshot,
        track_extent: LogicalLength,
    ) -> Option<ScrollBarThumbGeometry> {
        if snapshot.axis() != self.axis() {
            return None;
        }
        let track_extent = track_extent.get();
        let minimum = self.minimum_thumb_extent().get().min(track_extent);
        let thumb_extent = (track_extent * snapshot.visible_fraction().get())
            .max(minimum)
            .min(track_extent);
        let travel = (track_extent - thumb_extent).max(0.0);
        Some(ScrollBarThumbGeometry {
            thumb_extent: LogicalLength::new(thumb_extent).ok()?,
            travel: LogicalLength::new(travel).ok()?,
            thumb_origin: LogicalLength::new(travel * snapshot.normalized_position().get()).ok()?,
        })
    }
}

/// Device-independent request issued by one bound scroll control.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ScrollControlRequest {
    SmallStepBackward,
    SmallStepForward,
    PageBackward,
    PageForward,
    ToStart,
    ToEnd,
    SetNormalized(ScrollNormalizedValue),
}

#[cfg(test)]
mod tests {
    use super::{
        Axis, ScrollBarLayout, ScrollBarPlacement, ScrollBarVisibility, ScrollChrome,
        ScrollControlBinding, ScrollControlRequest, ScrollControlSnapshot, ScrollNormalizedError,
        ScrollNormalizedValue,
    };
    use crate::LogicalLength;

    #[test]
    fn binding_requires_a_positive_small_step() {
        assert!(ScrollControlBinding::new(Axis::Vertical, LogicalLength::ZERO).is_err());
        let binding = ScrollControlBinding::new(Axis::Horizontal, LogicalLength::from(8_u8))
            .unwrap_or_else(|_| unreachable!("positive fixture step is valid"));
        assert_eq!(binding.axis(), Axis::Horizontal);
        assert_eq!(binding.small_step(), LogicalLength::from(8_u8));
    }

    #[test]
    fn scrollbar_chrome_contract_is_typed_and_uses_neutral_defaults() {
        let layout = ScrollBarLayout::new(
            Axis::Vertical,
            LogicalLength::from(12_u8),
            LogicalLength::from(24_u8),
        );
        assert_eq!(layout.axis(), Axis::Vertical);
        assert_eq!(layout.visibility(), ScrollBarVisibility::Automatic);
        assert_eq!(layout.placement(), ScrollBarPlacement::Reserved);
        assert_eq!(layout.thickness(), LogicalLength::from(12_u8));
        assert_eq!(layout.minimum_thumb_extent(), LogicalLength::from(24_u8));

        let overlay = layout
            .with_visibility(ScrollBarVisibility::Always)
            .with_placement(ScrollBarPlacement::Overlay);
        assert_eq!(ScrollChrome::Bar(overlay).axis(), Some(Axis::Vertical));
        assert_eq!(
            ScrollChrome::Thumb(Axis::Horizontal).axis(),
            Some(Axis::Horizontal)
        );
        assert_eq!(ScrollChrome::Corner.axis(), None);
    }

    #[test]
    fn thumb_geometry_is_shared_checked_and_track_relative() {
        let snapshot =
            ScrollControlSnapshot::__runtime_from_metrics(Axis::Vertical, 50.0, 100.0, 200.0)
                .unwrap_or_else(|| unreachable!("fixture scroll metrics are valid"));
        let layout = ScrollBarLayout::new(
            Axis::Vertical,
            LogicalLength::from(10_u8),
            LogicalLength::from(20_u8),
        );
        let geometry = layout
            .thumb_geometry(snapshot, LogicalLength::from(100_u8))
            .unwrap_or_else(|| unreachable!("matching axis geometry is valid"));

        assert_eq!(geometry.thumb_extent().get(), 50.0);
        assert_eq!(geometry.travel().get(), 50.0);
        assert_eq!(geometry.thumb_origin().get(), 25.0);
        assert!(
            ScrollBarLayout::new(
                Axis::Horizontal,
                LogicalLength::from(10_u8),
                LogicalLength::from(20_u8),
            )
            .thumb_geometry(snapshot, LogicalLength::from(100_u8))
            .is_none()
        );
    }

    #[test]
    fn normalized_values_are_closed_finite_and_hashable_command_payloads() {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                ScrollNormalizedValue::new(value),
                Err(ScrollNormalizedError::NotFinite)
            );
        }
        for value in [-0.1, 1.1] {
            assert_eq!(
                ScrollNormalizedValue::new(value),
                Err(ScrollNormalizedError::OutOfRange)
            );
        }
        assert_eq!(
            ScrollNormalizedValue::new(-0.0)
                .unwrap_or_else(|_| unreachable!("negative zero canonicalizes")),
            ScrollNormalizedValue::ZERO
        );
        let request = ScrollControlRequest::SetNormalized(
            ScrollNormalizedValue::new(0.5)
                .unwrap_or_else(|_| unreachable!("half is a normalized value")),
        );
        assert_eq!(
            request,
            ScrollControlRequest::SetNormalized(
                ScrollNormalizedValue::new(0.5)
                    .unwrap_or_else(|_| unreachable!("half is a normalized value")),
            )
        );
        assert!(
            ScrollControlSnapshot::__runtime_from_metrics(Axis::Vertical, 11.0, 10.0, 20.0)
                .is_none(),
            "runtime projection fails closed rather than repairing an offset beyond maximum"
        );
    }
}

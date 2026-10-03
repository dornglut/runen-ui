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
    /// Errors when the small step is zero.
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
        Axis, ScrollControlBinding, ScrollControlRequest, ScrollControlSnapshot,
        ScrollNormalizedError, ScrollNormalizedValue,
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
        assert!(
            matches!(request, ScrollControlRequest::SetNormalized(value) if value.get() == 0.5)
        );
        assert!(
            ScrollControlSnapshot::__runtime_from_metrics(Axis::Vertical, 11.0, 10.0, 20.0)
                .is_none(),
            "runtime projection fails closed rather than repairing an offset beyond maximum"
        );
    }
}

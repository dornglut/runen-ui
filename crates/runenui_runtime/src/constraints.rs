//! Structurally valid layout constraint vocabulary.

use runenui_core::LogicalLength;

use crate::LogicalSize;

/// Maximum extent allowed on one layout axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AxisLimit {
    Finite(LogicalLength),
    Unbounded,
}

impl AxisLimit {
    #[must_use]
    pub const fn finite(value: LogicalLength) -> Self {
        Self::Finite(value)
    }

    #[must_use]
    pub const fn unbounded() -> Self {
        Self::Unbounded
    }

    #[must_use]
    pub const fn as_finite(self) -> Option<LogicalLength> {
        match self {
            Self::Finite(value) => Some(value),
            Self::Unbounded => None,
        }
    }

    #[must_use]
    pub const fn is_unbounded(self) -> bool {
        matches!(self, Self::Unbounded)
    }
}

/// Normalized minimum and maximum constraints for one layout axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisConstraints {
    min: LogicalLength,
    max: AxisLimit,
}

impl AxisConstraints {
    /// Creates constraints and raises an inverted finite maximum to the minimum.
    #[must_use]
    pub fn new(min: LogicalLength, max: AxisLimit) -> Self {
        let max = match max {
            AxisLimit::Finite(value) if value < min => AxisLimit::Finite(min),
            value => value,
        };
        Self { min, max }
    }

    #[must_use]
    pub const fn tight(extent: LogicalLength) -> Self {
        Self {
            min: extent,
            max: AxisLimit::Finite(extent),
        }
    }

    #[must_use]
    pub fn loose(max: LogicalLength) -> Self {
        Self::new(LogicalLength::ZERO, AxisLimit::Finite(max))
    }

    #[must_use]
    pub const fn unbounded() -> Self {
        Self {
            min: LogicalLength::ZERO,
            max: AxisLimit::Unbounded,
        }
    }

    #[must_use]
    pub const fn min(self) -> LogicalLength {
        self.min
    }

    #[must_use]
    pub const fn max(self) -> AxisLimit {
        self.max
    }

    #[must_use]
    pub fn is_tight(self) -> bool {
        matches!(self.max, AxisLimit::Finite(max) if max == self.min)
    }

    #[must_use]
    pub fn constrain(self, candidate: LogicalLength) -> LogicalLength {
        let candidate = if candidate < self.min {
            self.min
        } else {
            candidate
        };
        match self.max {
            AxisLimit::Finite(max) if candidate > max => max,
            _ => candidate,
        }
    }
}

/// Independent horizontal and vertical layout constraints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutConstraints {
    horizontal: AxisConstraints,
    vertical: AxisConstraints,
}

impl LayoutConstraints {
    #[must_use]
    pub const fn new(horizontal: AxisConstraints, vertical: AxisConstraints) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }

    #[must_use]
    pub const fn tight(size: LogicalSize) -> Self {
        Self::new(
            AxisConstraints::tight(size.width_length()),
            AxisConstraints::tight(size.height_length()),
        )
    }

    #[must_use]
    pub fn loose(max: LogicalSize) -> Self {
        Self::new(
            AxisConstraints::loose(max.width_length()),
            AxisConstraints::loose(max.height_length()),
        )
    }

    #[must_use]
    pub const fn unbounded() -> Self {
        Self::new(AxisConstraints::unbounded(), AxisConstraints::unbounded())
    }

    #[must_use]
    pub const fn horizontal(self) -> AxisConstraints {
        self.horizontal
    }
    #[must_use]
    pub const fn vertical(self) -> AxisConstraints {
        self.vertical
    }
    #[must_use]
    pub fn is_tight_width(self) -> bool {
        self.horizontal.is_tight()
    }
    #[must_use]
    pub fn is_tight_height(self) -> bool {
        self.vertical.is_tight()
    }
    #[must_use]
    pub fn constrain_width(self, candidate: LogicalLength) -> LogicalLength {
        self.horizontal.constrain(candidate)
    }
    #[must_use]
    pub fn constrain_height(self, candidate: LogicalLength) -> LogicalLength {
        self.vertical.constrain(candidate)
    }
    #[must_use]
    pub fn constrain(self, candidate: LogicalSize) -> LogicalSize {
        LogicalSize::new(
            self.constrain_width(candidate.width_length()),
            self.constrain_height(candidate.height_length()),
        )
    }
}

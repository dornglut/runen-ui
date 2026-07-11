//! Normalized layout constraint vocabulary.

use crate::LogicalSize;

/// Maximum extent allowed on one layout axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AxisLimit {
    /// A finite maximum extent.
    Finite(f32),
    /// No maximum extent.
    Unbounded,
}

impl AxisLimit {
    /// Creates a normalized finite or unbounded maximum.
    ///
    /// Positive infinity becomes unbounded. Negative, negative-infinite, and
    /// non-number values normalize to a finite zero maximum.
    #[must_use]
    pub fn new(value: f32) -> Self {
        if value == f32::INFINITY {
            Self::Unbounded
        } else {
            Self::Finite(normalize_extent(value))
        }
    }

    /// Creates an explicitly unbounded maximum.
    #[must_use]
    pub const fn unbounded() -> Self {
        Self::Unbounded
    }

    /// Returns the finite maximum, when one exists.
    #[must_use]
    pub const fn finite(&self) -> Option<f32> {
        match self {
            Self::Finite(value) => Some(*value),
            Self::Unbounded => None,
        }
    }

    /// Returns whether this maximum is unbounded.
    #[must_use]
    pub const fn is_unbounded(&self) -> bool {
        matches!(self, Self::Unbounded)
    }
}

/// Normalized minimum and maximum constraints for one layout axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisConstraints {
    min: f32,
    max: AxisLimit,
}

impl AxisConstraints {
    /// Creates normalized constraints for one axis.
    ///
    /// The minimum is finite and non-negative. A finite maximum below the
    /// normalized minimum is raised to the minimum.
    #[must_use]
    pub fn new(min: f32, max: AxisLimit) -> Self {
        let min = normalize_extent(min);
        let max = match max {
            AxisLimit::Finite(value) => AxisLimit::Finite(normalize_extent(value).max(min)),
            AxisLimit::Unbounded => AxisLimit::Unbounded,
        };

        Self { min, max }
    }

    /// Creates a tight constraint with one required extent.
    #[must_use]
    pub fn tight(extent: f32) -> Self {
        let extent = normalize_extent(extent);
        Self {
            min: extent,
            max: AxisLimit::Finite(extent),
        }
    }

    /// Creates a loose constraint from zero through the provided maximum.
    #[must_use]
    pub fn loose(max: f32) -> Self {
        Self::new(0.0, AxisLimit::new(max))
    }

    /// Creates an unconstrained axis.
    #[must_use]
    pub const fn unbounded() -> Self {
        Self {
            min: 0.0,
            max: AxisLimit::Unbounded,
        }
    }

    /// Returns the normalized minimum extent.
    #[must_use]
    pub const fn min(&self) -> f32 {
        self.min
    }

    /// Returns the normalized maximum extent.
    #[must_use]
    pub const fn max(&self) -> AxisLimit {
        self.max
    }

    /// Returns whether this axis has one exact required extent.
    #[must_use]
    pub const fn is_tight(&self) -> bool {
        matches!(self.max, AxisLimit::Finite(max) if max.to_bits() == self.min.to_bits())
    }

    /// Constrains a candidate extent to this normalized range.
    #[must_use]
    pub fn constrain(&self, candidate: f32) -> f32 {
        let candidate = normalize_extent(candidate).max(self.min);
        match self.max {
            AxisLimit::Finite(max) => candidate.min(max),
            AxisLimit::Unbounded => candidate,
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
    /// Creates layout constraints from normalized axis constraints.
    #[must_use]
    pub const fn new(horizontal: AxisConstraints, vertical: AxisConstraints) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }

    /// Creates tight constraints requiring exactly the provided logical size.
    #[must_use]
    pub fn tight(size: LogicalSize) -> Self {
        Self::new(
            AxisConstraints::tight(size.width()),
            AxisConstraints::tight(size.height()),
        )
    }

    /// Creates loose constraints from zero through the provided logical size.
    #[must_use]
    pub fn loose(max: LogicalSize) -> Self {
        Self::new(
            AxisConstraints::loose(max.width()),
            AxisConstraints::loose(max.height()),
        )
    }

    /// Creates constraints that are unbounded on both axes.
    #[must_use]
    pub const fn unbounded() -> Self {
        Self::new(AxisConstraints::unbounded(), AxisConstraints::unbounded())
    }

    /// Returns the horizontal constraints.
    #[must_use]
    pub const fn horizontal(&self) -> AxisConstraints {
        self.horizontal
    }

    /// Returns the vertical constraints.
    #[must_use]
    pub const fn vertical(&self) -> AxisConstraints {
        self.vertical
    }

    /// Returns whether the horizontal axis is tight.
    #[must_use]
    pub const fn is_tight_width(&self) -> bool {
        self.horizontal.is_tight()
    }

    /// Returns whether the vertical axis is tight.
    #[must_use]
    pub const fn is_tight_height(&self) -> bool {
        self.vertical.is_tight()
    }

    /// Constrains a candidate width on the horizontal axis.
    #[must_use]
    pub fn constrain_width(&self, candidate: f32) -> f32 {
        self.horizontal.constrain(candidate)
    }

    /// Constrains a candidate height on the vertical axis.
    #[must_use]
    pub fn constrain_height(&self, candidate: f32) -> f32 {
        self.vertical.constrain(candidate)
    }

    /// Constrains a candidate logical size on both axes.
    #[must_use]
    pub fn constrain(&self, candidate: LogicalSize) -> LogicalSize {
        LogicalSize::new(
            self.constrain_width(candidate.width()),
            self.constrain_height(candidate.height()),
        )
    }
}

fn normalize_extent(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::{AxisConstraints, AxisLimit, LayoutConstraints};
    use crate::LogicalSize;

    fn assert_f32_eq(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= f32::EPSILON,
            "expected {expected}, got {actual}",
        );
    }

    #[test]
    fn finite_and_unbounded_limits_are_explicit() {
        assert_eq!(AxisLimit::new(12.0), AxisLimit::Finite(12.0));
        assert_eq!(AxisLimit::new(0.0), AxisLimit::Finite(0.0));
        assert_eq!(AxisLimit::new(f32::INFINITY), AxisLimit::Unbounded);
        assert_eq!(AxisLimit::new(f32::NEG_INFINITY), AxisLimit::Finite(0.0));
        assert_eq!(AxisLimit::new(f32::NAN), AxisLimit::Finite(0.0));
        assert_eq!(AxisLimit::unbounded().finite(), None);
        assert!(AxisLimit::unbounded().is_unbounded());
    }

    #[test]
    fn axis_constraints_normalize_invalid_ranges() {
        assert_eq!(
            AxisConstraints::new(-4.0, AxisLimit::Finite(-2.0)),
            AxisConstraints::tight(0.0),
        );
        assert_eq!(
            AxisConstraints::new(20.0, AxisLimit::Finite(10.0)),
            AxisConstraints::tight(20.0),
        );
        assert_eq!(
            AxisConstraints::new(f32::NAN, AxisLimit::Unbounded),
            AxisConstraints::unbounded(),
        );
    }

    #[test]
    fn axis_constraints_clamp_candidates() {
        let constraints = AxisConstraints::new(10.0, AxisLimit::Finite(20.0));

        assert_f32_eq(constraints.constrain(5.0), 10.0);
        assert_f32_eq(constraints.constrain(15.0), 15.0);
        assert_f32_eq(constraints.constrain(25.0), 20.0);
        assert_f32_eq(constraints.constrain(f32::NAN), 10.0);
    }

    #[test]
    fn tight_layout_constraints_require_the_requested_size() {
        let constraints = LayoutConstraints::tight(LogicalSize::new(320.0, 200.0));

        assert!(constraints.is_tight_width());
        assert!(constraints.is_tight_height());
        assert_f32_eq(constraints.constrain_width(10.0), 320.0);
        assert_f32_eq(constraints.constrain_height(10.0), 200.0);
        assert_eq!(
            constraints.constrain(LogicalSize::new(10.0, 10.0)),
            LogicalSize::new(320.0, 200.0),
        );
    }

    #[test]
    fn loose_and_unbounded_constraints_preserve_valid_intrinsic_sizes() {
        let loose = LayoutConstraints::loose(LogicalSize::new(100.0, 80.0));
        let unbounded = LayoutConstraints::unbounded();
        let intrinsic = LogicalSize::new(60.0, 40.0);

        assert_eq!(loose.constrain(intrinsic), intrinsic);
        assert_eq!(
            loose.constrain(LogicalSize::new(120.0, 100.0)),
            LogicalSize::new(100.0, 80.0),
        );
        assert_eq!(unbounded.constrain(intrinsic), intrinsic);
    }
}

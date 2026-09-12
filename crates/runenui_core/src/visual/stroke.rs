use core::{error::Error, fmt};

use crate::LogicalLength;

/// Initial neutral stroke cap.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum StrokeCap {
    /// Stop at the endpoint.
    #[default]
    Butt,
    /// Add a semicircular cap.
    Round,
    /// Extend by half stroke width with a square cap.
    Square,
}

/// Initial neutral stroke join.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum StrokeJoin {
    /// Miter, with deterministic bevel fallback above the miter limit.
    #[default]
    Miter,
    /// Bevel join.
    Bevel,
    /// Round join.
    Round,
}

/// Validation failure for one stroke description.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrokeStyleError {
    /// Miter limit was NaN or infinite.
    NonFiniteMiterLimit,
    /// Miter limit was less than the accepted ratio `1`.
    MiterLimitBelowOne,
}

impl fmt::Display for StrokeStyleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NonFiniteMiterLimit => "stroke miter limit must be finite",
            Self::MiterLimitBelowOne => "stroke miter limit must be at least 1",
        })
    }
}

impl Error for StrokeStyleError {}

/// Centered neutral stroke geometry description.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeStyle {
    width: LogicalLength,
    cap: StrokeCap,
    join: StrokeJoin,
    miter_limit: f32,
}

impl StrokeStyle {
    /// Common centered stroke with butt caps, miter joins, and miter limit `4`.
    #[must_use]
    pub const fn new(width: LogicalLength) -> Self {
        Self {
            width,
            cap: StrokeCap::Butt,
            join: StrokeJoin::Miter,
            miter_limit: 4.0,
        }
    }

    /// Replaces the cap.
    #[must_use]
    pub const fn with_cap(mut self, cap: StrokeCap) -> Self {
        self.cap = cap;
        self
    }

    /// Replaces the join.
    #[must_use]
    pub const fn with_join(mut self, join: StrokeJoin) -> Self {
        self.join = join;
        self
    }

    /// Validates and replaces the miter-limit ratio.
    ///
    /// # Errors
    ///
    /// Returns [`StrokeStyleError`] for non-finite or sub-one values.
    pub fn with_miter_limit(mut self, miter_limit: f32) -> Result<Self, StrokeStyleError> {
        if !miter_limit.is_finite() {
            return Err(StrokeStyleError::NonFiniteMiterLimit);
        }
        if miter_limit < 1.0 {
            return Err(StrokeStyleError::MiterLimitBelowOne);
        }
        self.miter_limit = miter_limit;
        Ok(self)
    }

    /// Returns the finite non-negative centered width.
    #[must_use]
    pub const fn width(self) -> LogicalLength {
        self.width
    }

    /// Returns endpoint cap semantics.
    #[must_use]
    pub const fn cap(self) -> StrokeCap {
        self.cap
    }

    /// Returns join semantics.
    #[must_use]
    pub const fn join(self) -> StrokeJoin {
        self.join
    }

    /// Returns the validated miter-limit ratio.
    #[must_use]
    pub const fn miter_limit(self) -> f32 {
        self.miter_limit
    }
}

#[cfg(test)]
mod tests {
    use super::{StrokeCap, StrokeJoin, StrokeStyle, StrokeStyleError};
    use crate::LogicalLength;

    #[test]
    fn stroke_policy_is_explicit_and_zero_width_remains_literal_zero() {
        let zero = StrokeStyle::new(LogicalLength::ZERO)
            .with_cap(StrokeCap::Round)
            .with_join(StrokeJoin::Bevel);
        assert_eq!(zero.width(), LogicalLength::ZERO);
        assert_eq!(zero.cap(), StrokeCap::Round);
        assert_eq!(zero.join(), StrokeJoin::Bevel);
        assert_eq!(
            zero.with_miter_limit(0.5),
            Err(StrokeStyleError::MiterLimitBelowOne)
        );
        assert_eq!(
            zero.with_miter_limit(f32::NAN),
            Err(StrokeStyleError::NonFiniteMiterLimit)
        );
    }
}

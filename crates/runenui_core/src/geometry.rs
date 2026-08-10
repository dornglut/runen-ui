//! Canonical host-neutral logical geometry shared by authoring and runtime products.

use core::{error::Error, fmt};

use crate::{LogicalLength, LogicalLengthError, LogicalPoint, LogicalPointError};

/// Logical size in device-independent UI coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalSize {
    width: LogicalLength,
    height: LogicalLength,
}

impl LogicalSize {
    /// Zero logical size.
    pub const ZERO: Self = Self::new(LogicalLength::ZERO, LogicalLength::ZERO);

    /// Creates a size from already validated logical extents.
    #[must_use]
    pub const fn new(width: LogicalLength, height: LogicalLength) -> Self {
        Self { width, height }
    }

    /// Validates scalar width and height values.
    ///
    /// # Errors
    ///
    /// Returns [`LogicalLengthError`] when either extent is non-finite or negative.
    pub const fn try_new(width: f32, height: f32) -> Result<Self, LogicalLengthError> {
        let width = match LogicalLength::new(width) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        let height = match LogicalLength::new(height) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        Ok(Self::new(width, height))
    }

    /// Returns the horizontal extent as a scalar logical value.
    #[must_use]
    pub const fn width(self) -> f32 {
        self.width.get()
    }

    /// Returns the vertical extent as a scalar logical value.
    #[must_use]
    pub const fn height(self) -> f32 {
        self.height.get()
    }

    /// Returns the validated horizontal extent.
    #[must_use]
    pub const fn width_length(self) -> LogicalLength {
        self.width
    }

    /// Returns the validated vertical extent.
    #[must_use]
    pub const fn height_length(self) -> LogicalLength {
        self.height
    }
}

/// Error returned when scalar logical-rectangle construction is invalid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogicalRectError {
    /// The logical origin contains a non-finite coordinate.
    Origin(LogicalPointError),
    /// Width or height is non-finite or negative.
    Extent(LogicalLengthError),
}

impl fmt::Display for LogicalRectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Origin(error) => write!(formatter, "invalid logical rectangle origin: {error}"),
            Self::Extent(error) => write!(formatter, "invalid logical rectangle extent: {error}"),
        }
    }
}

impl Error for LogicalRectError {}

/// Logical rectangle in device-independent UI coordinates.
///
/// The type carries no surface identity or absolute-authoring authority. A semantic
/// contribution can therefore use one as an owner-local rectangle while runtime
/// remains responsible for translating it into publication coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalRect {
    origin: LogicalPoint,
    size: LogicalSize,
}

impl LogicalRect {
    /// Creates a rectangle from already validated logical values.
    #[must_use]
    pub const fn new(origin: LogicalPoint, size: LogicalSize) -> Self {
        Self { origin, size }
    }

    /// Validates scalar origin and extent values.
    ///
    /// # Errors
    ///
    /// Returns [`LogicalRectError`] for non-finite coordinates or invalid extents.
    pub const fn try_new(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Result<Self, LogicalRectError> {
        let origin = match LogicalPoint::new(x, y) {
            Ok(value) => value,
            Err(error) => return Err(LogicalRectError::Origin(error)),
        };
        let size = match LogicalSize::try_new(width, height) {
            Ok(value) => value,
            Err(error) => return Err(LogicalRectError::Extent(error)),
        };
        Ok(Self::new(origin, size))
    }

    /// Returns the top-left origin.
    #[must_use]
    pub const fn origin(self) -> LogicalPoint {
        self.origin
    }

    /// Returns the rectangle size.
    #[must_use]
    pub const fn size(self) -> LogicalSize {
        self.size
    }

    /// Returns the left edge.
    #[must_use]
    pub const fn x(self) -> f32 {
        self.origin.x()
    }

    /// Returns the top edge.
    #[must_use]
    pub const fn y(self) -> f32 {
        self.origin.y()
    }

    /// Returns the width.
    #[must_use]
    pub const fn width(self) -> f32 {
        self.size.width()
    }

    /// Returns the height.
    #[must_use]
    pub const fn height(self) -> f32 {
        self.size.height()
    }

    /// Returns the right edge, saturating finite arithmetic overflow.
    #[must_use]
    pub fn max_x(self) -> f32 {
        finite_saturating_add(self.x(), self.width())
    }

    /// Returns the bottom edge, saturating finite arithmetic overflow.
    #[must_use]
    pub fn max_y(self) -> f32 {
        finite_saturating_add(self.y(), self.height())
    }

    /// Returns whether a point lies inside this rectangle.
    ///
    /// Containment is left/top inclusive and right/bottom exclusive.
    #[must_use]
    pub fn contains(self, point: LogicalPoint) -> bool {
        (self.x()..self.max_x()).contains(&point.x())
            && (self.y()..self.max_y()).contains(&point.y())
    }
}

fn finite_saturating_add(left: f32, right: f32) -> f32 {
    let sum = left + right;
    if sum.is_finite() {
        sum
    } else if left.is_sign_negative() && right.is_sign_negative() {
        f32::MIN
    } else {
        f32::MAX
    }
}

#[cfg(test)]
mod tests {
    use super::{LogicalRect, LogicalRectError, LogicalSize};
    use crate::{LogicalLengthError, LogicalPoint, LogicalPointError};

    #[test]
    fn size_and_rect_scalar_construction_remains_checked() {
        assert_eq!(
            LogicalSize::try_new(-1.0, 2.0),
            Err(LogicalLengthError::Negative)
        );
        assert_eq!(
            LogicalRect::try_new(f32::NAN, 0.0, 1.0, 1.0),
            Err(LogicalRectError::Origin(LogicalPointError))
        );
        assert_eq!(
            LogicalRect::try_new(0.0, 0.0, f32::INFINITY, 1.0),
            Err(LogicalRectError::Extent(LogicalLengthError::NotFinite))
        );
    }

    #[test]
    fn rectangle_contains_uses_half_open_edges() {
        let rect = LogicalRect::try_new(-2.0, 3.0, 5.0, 7.0)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"));
        let inside = LogicalPoint::new(2.999, 9.999)
            .unwrap_or_else(|_| unreachable!("test point is finite"));
        let right_edge = LogicalPoint::new(3.0, 4.0)
            .unwrap_or_else(|_| unreachable!("test point is finite"));
        assert!(rect.contains(inside));
        assert!(!rect.contains(right_edge));
    }
}

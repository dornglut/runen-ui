//! Validated logical-coordinate values shared by authoring and runtime products.

use core::{error::Error, fmt};

/// Error returned when a logical length is not finite and non-negative.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogicalLengthError {
    /// The supplied value was NaN or positive/negative infinity.
    NotFinite,
    /// The supplied value was finite but negative.
    Negative,
}

impl fmt::Display for LogicalLengthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFinite => formatter.write_str("logical length must be finite"),
            Self::Negative => formatter.write_str("logical length must not be negative"),
        }
    }
}

impl Error for LogicalLengthError {}

/// A finite, non-negative distance in `RunenUI` logical coordinates.
///
/// Logical coordinates are device-independent. A future host scale factor maps
/// them to physical pixels; it does not change this authored value contract.
#[derive(Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct LogicalLength(f32);

impl LogicalLength {
    /// Zero logical units.
    pub const ZERO: Self = Self(0.0);

    /// Largest finite logical length representable by the current scalar.
    pub const MAX: Self = Self(f32::MAX);

    /// Validates a finite, non-negative logical length.
    ///
    /// # Errors
    ///
    /// Returns [`LogicalLengthError`] for NaN, infinity, or negative values.
    pub const fn new(value: f32) -> Result<Self, LogicalLengthError> {
        if value.is_nan() || value == f32::INFINITY || value == f32::NEG_INFINITY {
            Err(LogicalLengthError::NotFinite)
        } else if value < 0.0 {
            Err(LogicalLengthError::Negative)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the scalar logical-coordinate value.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }

    /// Adds two lengths, saturating at the largest finite logical length.
    #[must_use]
    pub fn saturating_add(self, other: Self) -> Self {
        let sum = self.0 + other.0;
        if sum.is_finite() {
            Self(sum)
        } else {
            Self::MAX
        }
    }

    /// Subtracts a length, saturating at zero.
    #[must_use]
    pub fn saturating_sub(self, other: Self) -> Self {
        Self((self.0 - other.0).max(0.0))
    }
}

impl fmt::Debug for LogicalLength {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("LogicalLength")
            .field(&self.0)
            .finish()
    }
}

impl TryFrom<f32> for LogicalLength {
    type Error = LogicalLengthError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<u16> for LogicalLength {
    fn from(value: u16) -> Self {
        Self(f32::from(value))
    }
}

impl From<u8> for LogicalLength {
    fn from(value: u8) -> Self {
        Self(f32::from(value))
    }
}

#[cfg(test)]
mod tests {
    use super::{LogicalLength, LogicalLengthError};

    #[test]
    fn logical_length_validation_is_explicit_and_deterministic() {
        let cases = [
            (f32::NAN, Err(LogicalLengthError::NotFinite)),
            (f32::INFINITY, Err(LogicalLengthError::NotFinite)),
            (f32::NEG_INFINITY, Err(LogicalLengthError::NotFinite)),
            (-1.0, Err(LogicalLengthError::Negative)),
            (-0.0, Ok(LogicalLength::ZERO)),
            (0.0, Ok(LogicalLength::ZERO)),
            (f32::MAX, Ok(LogicalLength::MAX)),
        ];

        for (input, expected) in cases {
            assert_eq!(LogicalLength::new(input), expected);
        }
        assert_eq!(
            LogicalLengthError::NotFinite.to_string(),
            "logical length must be finite"
        );
        assert_eq!(
            LogicalLengthError::Negative.to_string(),
            "logical length must not be negative"
        );
    }

    #[test]
    fn logical_length_arithmetic_saturates_without_invalid_floats() {
        let four = LogicalLength::new(4.0).unwrap_or_default();
        let six = LogicalLength::new(6.0).unwrap_or_default();

        assert_eq!(
            four.saturating_add(six),
            LogicalLength::new(10.0).unwrap_or_default()
        );
        assert_eq!(four.saturating_sub(six), LogicalLength::ZERO);
        assert_eq!(
            LogicalLength::MAX.saturating_add(LogicalLength::MAX),
            LogicalLength::MAX
        );
    }
}

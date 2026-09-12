use core::{error::Error, fmt};

use crate::{Color, LogicalLength};

/// Validation failure for a finite signed logical scalar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NonFiniteVisualScalar;

impl fmt::Display for NonFiniteVisualScalar {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("visual scalar must be finite")
    }
}

impl Error for NonFiniteVisualScalar {}

/// Ordinary renderer-neutral drop shadow description.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropShadow {
    offset_x: f32,
    offset_y: f32,
    sigma: LogicalLength,
    spread: f32,
    color: Color,
}

impl DropShadow {
    /// Creates one finite ordinary drop shadow.
    ///
    /// `sigma` is finite/non-negative through [`LogicalLength`]. Spread is signed
    /// and therefore validated separately.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteVisualScalar`] for non-finite offset/spread values.
    pub fn new(
        offset_x: f32,
        offset_y: f32,
        sigma: LogicalLength,
        spread: f32,
        color: Color,
    ) -> Result<Self, NonFiniteVisualScalar> {
        if ![offset_x, offset_y, spread].into_iter().all(f32::is_finite) {
            return Err(NonFiniteVisualScalar);
        }
        Ok(Self {
            offset_x,
            offset_y,
            sigma,
            spread,
            color,
        })
    }

    /// Returns finite horizontal offset.
    #[must_use]
    pub const fn offset_x(self) -> f32 {
        self.offset_x
    }

    /// Returns finite vertical offset.
    #[must_use]
    pub const fn offset_y(self) -> f32 {
        self.offset_y
    }

    /// Returns finite non-negative blur sigma.
    #[must_use]
    pub const fn sigma(self) -> LogicalLength {
        self.sigma
    }

    /// Returns finite signed Euclidean spread radius.
    #[must_use]
    pub const fn spread(self) -> f32 {
        self.spread
    }

    /// Returns straight-alpha sRGB8 shadow color.
    #[must_use]
    pub const fn color(self) -> Color {
        self.color
    }
}

#[cfg(test)]
mod tests {
    use super::{DropShadow, NonFiniteVisualScalar};
    use crate::{Color, LogicalLength};

    #[test]
    fn signed_spread_is_finite_and_preserved() {
        let shadow = DropShadow::new(1.0, -2.0, LogicalLength::ZERO, -3.0, Color::BLACK)
            .unwrap_or_else(|_| unreachable!("test shadow is finite"));
        assert_eq!(shadow.spread().to_bits(), (-3.0_f32).to_bits());
        assert_eq!(shadow.offset_x().to_bits(), 1.0_f32.to_bits());
        assert_eq!(shadow.offset_y().to_bits(), (-2.0_f32).to_bits());
        assert_eq!(
            DropShadow::new(f32::NAN, 0.0, LogicalLength::ZERO, 0.0, Color::BLACK),
            Err(NonFiniteVisualScalar)
        );
    }
}

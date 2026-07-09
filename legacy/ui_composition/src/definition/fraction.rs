use core::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SplitFractionError(pub u16);

impl fmt::Display for SplitFractionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "split fraction {} is outside 1..=9999 basis points",
            self.0
        )
    }
}

impl std::error::Error for SplitFractionError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct SplitFraction(u16);

impl SplitFraction {
    pub const fn try_new(basis_points: u16) -> Result<Self, SplitFractionError> {
        if basis_points > 0 && basis_points < 10_000 {
            Ok(Self(basis_points))
        } else {
            Err(SplitFractionError(basis_points))
        }
    }

    pub const fn basis_points(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for SplitFraction {
    type Error = SplitFractionError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<SplitFraction> for u16 {
    fn from(value: SplitFraction) -> Self {
        value.0
    }
}

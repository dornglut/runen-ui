//! Core layout intent.

use crate::LogicalLength;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Axis {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutStyle {
    gap: LogicalLength,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            gap: LogicalLength::ZERO,
        }
    }
}

impl LayoutStyle {
    #[must_use]
    pub const fn gap(self) -> LogicalLength {
        self.gap
    }

    #[must_use]
    pub fn with_gap(mut self, gap: impl Into<LogicalLength>) -> Self {
        self.gap = gap.into();
        self
    }
}

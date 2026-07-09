//! Core layout intent.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Px(f32);

impl Px {
    pub const ZERO: Self = Self(0.0);

    #[must_use]
    pub const fn new(value: f32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for Px {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

impl From<u16> for Px {
    fn from(value: u16) -> Self {
        Self::new(f32::from(value))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Axis {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutStyle {
    gap: Px,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self { gap: Px::ZERO }
    }
}

impl LayoutStyle {
    #[must_use]
    pub const fn gap(self) -> Px {
        self.gap
    }

    #[must_use]
    pub fn with_gap(mut self, gap: impl Into<Px>) -> Self {
        self.gap = gap.into();
        self
    }
}

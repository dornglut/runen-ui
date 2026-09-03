//! Style-property downstream effect classification.

use crate::ComputedStyle;

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StyleProperty {
    Foreground,
    Background,
    Padding,
    Radius,
    Typography,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StyleEffects {
    layout: bool,
    paint: bool,
}

impl StyleEffects {
    pub const NONE: Self = Self {
        layout: false,
        paint: false,
    };
    pub const PAINT: Self = Self {
        layout: false,
        paint: true,
    };
    pub const LAYOUT: Self = Self {
        layout: true,
        paint: false,
    };

    #[must_use]
    pub const fn layout(self) -> bool {
        self.layout
    }
    #[must_use]
    pub const fn paint(self) -> bool {
        self.paint
    }
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self {
            layout: self.layout || other.layout,
            paint: self.paint || other.paint,
        }
    }
}

impl StyleProperty {
    #[must_use]
    pub const fn effects(self) -> StyleEffects {
        match self {
            Self::Foreground | Self::Background | Self::Radius => StyleEffects::PAINT,
            Self::Padding | Self::Typography => StyleEffects::LAYOUT,
        }
    }
}

/// Classifies the direct downstream effects of exact computed-style changes.
///
/// Runtime remains responsible for dependency propagation: a layout change also
/// makes paint/hit/semantic geometry stale. This function deliberately reports
/// only the property-owned direct effect.
#[must_use]
pub fn style_effects_between(old: &ComputedStyle, new: &ComputedStyle) -> StyleEffects {
    let mut effects = StyleEffects::NONE;
    if old.foreground() != new.foreground() {
        effects = effects.union(StyleProperty::Foreground.effects());
    }
    if old.background() != new.background() {
        effects = effects.union(StyleProperty::Background.effects());
    }
    if old.padding() != new.padding() {
        effects = effects.union(StyleProperty::Padding.effects());
    }
    if old.radius() != new.radius() {
        effects = effects.union(StyleProperty::Radius.effects());
    }
    if old.typography() != new.typography() {
        effects = effects.union(StyleProperty::Typography.effects());
    }
    effects
}

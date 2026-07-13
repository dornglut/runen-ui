//! Runtime-owned widget lifecycle contexts and invalidation vocabulary.

use core::ops::{BitOr, BitOrAssign};

/// Widget capability invalidation requested from mounted behavior.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct WidgetInvalidation(u8);

impl WidgetInvalidation {
    pub const NONE: Self = Self(0);
    pub const INTERACTION: Self = Self(1 << 0);
    pub const LAYOUT: Self = Self(1 << 1);
    pub const PAINT: Self = Self(1 << 2);
    pub const SEMANTICS: Self = Self(1 << 3);
    pub const DIAGNOSTICS: Self = Self(1 << 4);
    pub const ALL: Self = Self(
        Self::INTERACTION.0
            | Self::LAYOUT.0
            | Self::PAINT.0
            | Self::SEMANTICS.0
            | Self::DIAGNOSTICS.0,
    );

    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for WidgetInvalidation {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

impl BitOrAssign for WidgetInvalidation {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(rhs);
    }
}

macro_rules! invalidating_context {
    ($name:ident) => {
        #[derive(Debug, Default)]
        pub struct $name {
            invalidation: WidgetInvalidation,
        }

        impl $name {
            pub fn invalidate(&mut self, invalidation: WidgetInvalidation) {
                self.invalidation |= invalidation;
            }

            #[doc(hidden)]
            #[must_use]
            pub const fn __runtime_new() -> Self {
                Self {
                    invalidation: WidgetInvalidation::NONE,
                }
            }

            #[doc(hidden)]
            #[must_use]
            pub fn __runtime_take_invalidation(&mut self) -> WidgetInvalidation {
                core::mem::take(&mut self.invalidation)
            }
        }
    };
}

invalidating_context!(WidgetMountContext);
invalidating_context!(WidgetUpdateContext);
invalidating_context!(WidgetActivationContext);

/// Why a mounted widget lifetime is ending.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WidgetUnmountReason {
    Removed,
    Replaced,
    RuntimeShutdown,
}

/// Read-only context supplied to a widget unmount hook.
#[derive(Debug)]
pub struct WidgetUnmountContext {
    reason: WidgetUnmountReason,
}

impl WidgetUnmountContext {
    #[must_use]
    pub const fn reason(&self) -> WidgetUnmountReason {
        self.reason
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(reason: WidgetUnmountReason) -> Self {
        Self { reason }
    }
}

#[cfg(test)]
mod tests {
    use super::WidgetInvalidation;

    #[test]
    fn invalidation_union_and_containment_are_exact() {
        let value = WidgetInvalidation::LAYOUT | WidgetInvalidation::PAINT;
        assert!(value.contains(WidgetInvalidation::LAYOUT));
        assert!(value.contains(WidgetInvalidation::PAINT));
        assert!(!value.contains(WidgetInvalidation::SEMANTICS));
        assert!(WidgetInvalidation::NONE.is_empty());
        assert!(WidgetInvalidation::ALL.contains(value));
    }
}

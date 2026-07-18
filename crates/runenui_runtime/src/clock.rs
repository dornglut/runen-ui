//! Checked monotonic time and deterministic headless clock.

use std::{cell::Cell, error::Error, fmt, rc::Rc, time::Duration};

/// Runtime-relative monotonic nanosecond instant.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MonotonicInstant(u64);

impl MonotonicInstant {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn as_nanos(self) -> u64 {
        self.0
    }

    pub(crate) const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    /// Adds a duration without wrapping.
    ///
    /// # Errors
    ///
    /// Returns [`MonotonicTimeError::Overflow`] when the duration or result
    /// cannot be represented.
    pub fn checked_add(self, duration: Duration) -> Result<Self, MonotonicTimeError> {
        let nanos = u64::try_from(duration.as_nanos()).map_err(|_| MonotonicTimeError::Overflow)?;
        self.0
            .checked_add(nanos)
            .map(Self)
            .ok_or(MonotonicTimeError::Overflow)
    }
}

/// Read-only monotonic clock adapter used only on the logical UI thread.
pub trait MonotonicClock {
    fn now(&self) -> MonotonicInstant;
}

/// Cloneable deterministic clock advanced explicitly by headless callers.
#[derive(Clone, Debug, Default)]
pub struct ManualClock {
    now: Rc<Cell<MonotonicInstant>>,
}

impl ManualClock {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances monotonic time without wrapping.
    ///
    /// # Errors
    ///
    /// Returns [`MonotonicTimeError::Overflow`] on unrepresentable time.
    pub fn advance(&self, duration: Duration) -> Result<MonotonicInstant, MonotonicTimeError> {
        let next = self.now.get().checked_add(duration)?;
        self.now.set(next);
        Ok(next)
    }
}

impl MonotonicClock for ManualClock {
    fn now(&self) -> MonotonicInstant {
        self.now.get()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MonotonicTimeError {
    Overflow,
}

impl fmt::Display for MonotonicTimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("monotonic time overflow")
    }
}

impl Error for MonotonicTimeError {}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{ManualClock, MonotonicClock, MonotonicInstant, MonotonicTimeError};

    #[test]
    fn manual_time_is_checked_and_never_wall_clock_driven() {
        let clock = ManualClock::new();
        assert_eq!(clock.now(), MonotonicInstant::ZERO);
        assert_eq!(
            clock
                .advance(Duration::from_nanos(7))
                .map(MonotonicInstant::as_nanos),
            Ok(7)
        );
        assert_eq!(
            MonotonicInstant::ZERO.checked_add(Duration::MAX),
            Err(MonotonicTimeError::Overflow)
        );
    }
}

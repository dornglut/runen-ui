//! Checked monotonic time and deterministic headless clock.

use std::{cell::Cell, rc::Rc, time::Duration};

pub use runenui_core::{MonotonicInstant, MonotonicTimeError};

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

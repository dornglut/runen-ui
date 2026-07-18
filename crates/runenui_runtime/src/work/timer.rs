//! Deterministic monotonic timer registry.

use std::time::Duration;

use runenui_core::TimerEffect;

use crate::{MonotonicInstant, work::WorkGeneration};

pub(crate) struct Timer<Action> {
    pub(crate) generation: WorkGeneration,
    pub(crate) deadline: MonotonicInstant,
    interval: Option<Duration>,
    action: Box<dyn FnMut() -> Action>,
    promoted: bool,
}

impl<Action> Timer<Action> {
    pub(crate) fn new(
        generation: WorkGeneration,
        now: MonotonicInstant,
        effect: TimerEffect<Action>,
    ) -> Result<Self, TimerStartError> {
        let (_key, delay, interval, action) = effect.__runtime_into_parts();
        if interval.is_some_and(|interval| interval.is_zero()) {
            return Err(TimerStartError::ZeroInterval);
        }
        Ok(Self {
            generation,
            deadline: now
                .checked_add(delay)
                .map_err(|_| TimerStartError::DeadlineOverflow)?,
            interval,
            action,
            promoted: false,
        })
    }

    pub(crate) fn is_due(&self, now: MonotonicInstant) -> bool {
        !self.promoted && self.deadline <= now
    }

    pub(crate) const fn mark_promoted(&mut self) {
        self.promoted = true;
    }

    pub(crate) fn fire(&mut self, now: MonotonicInstant) -> (Action, TimerFireOutcome) {
        let action = (self.action)();
        let Some(interval) = self.interval else {
            return (action, TimerFireOutcome::Completed);
        };
        let Ok(step) = u64::try_from(interval.as_nanos()) else {
            return (action, TimerFireOutcome::RepeatDeadlineOverflow);
        };
        let elapsed = now.as_nanos() - self.deadline.as_nanos();
        let periods = elapsed / step + 1;
        let Some(delta) = step.checked_mul(periods) else {
            return (action, TimerFireOutcome::RepeatDeadlineOverflow);
        };
        let Some(next) = self.deadline.as_nanos().checked_add(delta) else {
            return (action, TimerFireOutcome::RepeatDeadlineOverflow);
        };
        self.deadline = MonotonicInstant::from_nanos(next);
        self.promoted = false;
        (action, TimerFireOutcome::Rescheduled)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TimerStartError {
    ZeroInterval,
    DeadlineOverflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TimerFireOutcome {
    Completed,
    Rescheduled,
    RepeatDeadlineOverflow,
}

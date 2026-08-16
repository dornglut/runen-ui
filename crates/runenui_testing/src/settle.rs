use core::num::NonZeroUsize;

use runenui_runtime::{PumpBudget, PumpOutcome, PumpReport, RuntimeTerminalReason};

/// Explicit finite budget for deterministic settle-to-idle attempts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SettleBudget {
    max_iterations: NonZeroUsize,
    pump_budget: PumpBudget,
}

impl SettleBudget {
    /// Creates an explicitly bounded settle request.
    #[must_use]
    pub const fn new(max_iterations: NonZeroUsize, pump_budget: PumpBudget) -> Self {
        Self {
            max_iterations,
            pump_budget,
        }
    }

    /// Returns the maximum number of complete pump iterations.
    #[must_use]
    pub const fn max_iterations(self) -> NonZeroUsize {
        self.max_iterations
    }

    /// Returns the budget applied to every complete pump iteration.
    #[must_use]
    pub const fn pump_budget(self) -> PumpBudget {
        self.pump_budget
    }
}

/// Why a bounded settle attempt stopped.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettleOutcome {
    /// A complete pump iteration made zero progress and reported quiescence.
    Idle,
    /// The explicit maximum iteration count was reached first.
    IterationLimit,
    /// Runtime closure was observed.
    Closed,
    /// Runtime terminalization was observed.
    Terminal(RuntimeTerminalReason),
}

/// Exact result of one bounded settle attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SettleReport {
    iterations: usize,
    last_pump: PumpReport,
    outcome: SettleOutcome,
}

impl SettleReport {
    pub(crate) const fn new(
        iterations: usize,
        last_pump: PumpReport,
        outcome: SettleOutcome,
    ) -> Self {
        Self {
            iterations,
            last_pump,
            outcome,
        }
    }

    /// Returns the number of complete pump calls performed.
    #[must_use]
    pub const fn iterations(self) -> usize {
        self.iterations
    }

    /// Returns the final exact pump report.
    #[must_use]
    pub const fn last_pump(self) -> PumpReport {
        self.last_pump
    }

    /// Returns why the bounded settle attempt stopped.
    #[must_use]
    pub const fn outcome(self) -> SettleOutcome {
        self.outcome
    }

    /// Returns whether a final zero-progress quiescent iteration proved idle.
    #[must_use]
    pub const fn is_idle(self) -> bool {
        matches!(self.outcome, SettleOutcome::Idle)
    }
}

const fn zero_progress(report: PumpReport) -> bool {
    report.processed_envelopes() == 0
        && report.imported_completions() == 0
        && report.polled_local_work() == 0
        && report.promoted_timers() == 0
}

pub const fn outcome_for(report: PumpReport, at_limit: bool) -> Option<SettleOutcome> {
    match report.outcome() {
        PumpOutcome::Closed => Some(SettleOutcome::Closed),
        PumpOutcome::Terminal(reason) => Some(SettleOutcome::Terminal(reason)),
        PumpOutcome::Quiescent if zero_progress(report) => Some(SettleOutcome::Idle),
        PumpOutcome::Quiescent | PumpOutcome::BudgetExhausted if at_limit => {
            Some(SettleOutcome::IterationLimit)
        }
        PumpOutcome::Quiescent | PumpOutcome::BudgetExhausted => None,
        _ if at_limit => Some(SettleOutcome::IterationLimit),
        _ => None,
    }
}
